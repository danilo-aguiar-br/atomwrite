// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standalone BLAKE3 checksum computation for one or more files.
//!
//! Workload: CPU-bound (BLAKE3 hashing) with I/O for file reads.
//! Parallelism: multi-root recursive discovery via one `WalkBuilder` + `.add`
//! + `collect_files_parallel`; multi-file digests via `rayon::par_iter` then
//! NDJSON in sorted path order. Large single files use BLAKE3 mmap-rayon
//! (see `checksum`). Bound: process-wide rayon pool and `WalkBuilder::threads`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use rayon::prelude::*;

use crate::checksum;
use crate::cli::{GlobalArgs, HashArgs};
use crate::ndjson_types::HashOutput;
use crate::output::NdjsonWriter;

/// Compute BLAKE3 checksums for files or stdin.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if a target file does not exist.
/// Returns `AtomwriteError::Io` if reading the file or stdin fails.
#[tracing::instrument(skip_all, fields(command = "hash"))]
pub fn cmd_hash(
    args: &HashArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    if args.stdin {
        let mut reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
        let hash = checksum::hash_reader(&mut reader)?;
        writer.write_event(&HashOutput {
            r#type: "hash",
            path: None,
            source: Some("stdin"),
            algorithm: "blake3",
            value: hash,
            bytes: None,
            verified: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
        return Ok(());
    }

    let mut file_paths: Vec<PathBuf> = Vec::new();
    let mut dir_roots: Vec<PathBuf> = Vec::new();
    // Recipe and agents pass exclude; recursive walks always skip backups (A-027).
    let excludes = if args.exclude.is_empty() && args.recursive {
        crate::commands::backup_exclude_globs()
    } else {
        args.exclude.clone()
    };
    let skip_bak_in_walk = args.recursive || excludes.iter().any(|e| e.contains("bak"));

    for path in &args.paths {
        let path = crate::path_safety::validate_path(path, &workspace)?;

        if !path.exists() {
            return Err(crate::error::AtomwriteError::NotFound { path }.into());
        }

        if path.is_dir() && !args.recursive {
            continue;
        }

        if path.is_dir() && args.recursive {
            dir_roots.push(path);
        } else if path.is_file() {
            // Explicit single-file paths are hashed even if they look like backups,
            // unless an exclude pattern matches.
            if excludes.is_empty() || !is_excluded_path(&path, &excludes) {
                file_paths.push(path);
            }
        }
    }

    // One multi-root WalkParallel (docs.rs: prefer `.add` over N walks).
    if !dir_roots.is_empty() && !crate::signal::is_global_shutdown() {
        let mut builder = ignore::WalkBuilder::new(&dir_roots[0]);
        for d in dir_roots.iter().skip(1) {
            builder.add(d);
        }
        builder.hidden(false);
        builder.git_ignore(true);
        crate::concurrency::apply_walk_threads(&mut builder, global.threads);
        if skip_bak_in_walk {
            builder.filter_entry(|entry| !path_looks_like_bak(entry.path()));
        }
        let walked = crate::concurrency::collect_files_parallel(&builder);
        let filtered: Vec<PathBuf> = if crate::concurrency::should_parallelize(walked.len())
            && !excludes.is_empty()
        {
            use rayon::prelude::*;
            walked
                .into_par_iter()
                .filter(|p| !is_excluded_path(p, &excludes))
                .collect()
        } else if excludes.is_empty() {
            walked
        } else {
            walked
                .into_iter()
                .filter(|p| !is_excluded_path(p, &excludes))
                .collect()
        };
        file_paths.extend(filtered);
    }

    if crate::signal::is_global_shutdown() {
        return Ok(());
    }

    crate::concurrency::sort_paths_parallel(&mut file_paths);

    let max_size = global.effective_max_filesize();

    // Parallel path: independent digests, then emit in sorted order.
    // Sequential when `--verify` needs fail-fast, or single file (no fan-out gain).
    if args.verify.is_none() && crate::concurrency::should_parallelize(file_paths.len()) {
        return emit_hashes_parallel(&file_paths, max_size, start, writer);
    }

    for path in &file_paths {
        if crate::signal::is_global_shutdown() {
            return Ok(());
        }
        emit_one_hash(path, max_size, args.verify.as_deref(), start, writer)?;
    }

    Ok(())
}

/// Hash many files in parallel; emit NDJSON in the same order as `file_paths`.
fn emit_hashes_parallel(
    file_paths: &[PathBuf],
    max_size: u64,
    start: Instant,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    // Collect digests first so stdout order stays deterministic (sorted paths).
    let results: Vec<Result<(String, u64), anyhow::Error>> = file_paths
        .par_iter()
        .map(|path| {
            if crate::signal::is_global_shutdown() {
                return Err(crate::signal::cancelled_error("hash cancelled by signal").into());
            }
            checksum::hash_file_with_len(path, max_size)
        })
        .collect();

    if crate::signal::is_global_shutdown() {
        return Ok(());
    }

    for (path, result) in file_paths.iter().zip(results) {
        let (hash, bytes) = result?;
        writer.write_event(&HashOutput {
            r#type: "hash",
            path: Some(path.display().to_string()),
            source: None,
            algorithm: "blake3",
            value: hash,
            bytes: Some(bytes),
            verified: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
    }
    Ok(())
}

fn emit_one_hash(
    path: &Path,
    max_size: u64,
    verify: Option<&str>,
    start: Instant,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let path_str = path.display().to_string();
    // Single open+stat: digest and length together (no second metadata()).
    let (hash, bytes) = checksum::hash_file_with_len(path, max_size)?;

    if let Some(expected) = verify {
        let verified = hash == expected;
        writer.write_event(&HashOutput {
            r#type: "hash",
            path: Some(path_str),
            source: None,
            algorithm: "blake3",
            value: hash,
            bytes: Some(bytes),
            verified: Some(verified),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
        if !verified {
            return Err(crate::error::AtomwriteError::ChecksumVerifyFailed {
                path: path.to_path_buf(),
                expected: expected.to_string(),
            }
            .into());
        }
    } else {
        writer.write_event(&HashOutput {
            r#type: "hash",
            path: Some(path_str),
            source: None,
            algorithm: "blake3",
            value: hash,
            bytes: Some(bytes),
            verified: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
    }
    Ok(())
}

fn path_looks_like_bak(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.contains(".bak."))
}

fn is_excluded_path(path: &Path, excludes: &[String]) -> bool {
    if excludes.is_empty() {
        return false;
    }
    let s = path.to_string_lossy();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    for pat in excludes {
        if crate::constants::BACKUP_EXCLUDE_GLOBS
            .iter()
            .any(|g| pat == *g)
        {
            if name.contains(".bak.") || s.contains(".bak.") {
                return true;
            }
            continue;
        }
        if let Some(stripped) = pat.strip_prefix("**/") {
            if s.contains(stripped.trim_start_matches('*')) {
                return true;
            }
        }
        if let Some(stripped) = pat.strip_prefix('*') {
            if name.ends_with(stripped) || s.ends_with(stripped) {
                return true;
            }
        }
        if s.contains(pat) || name == pat {
            return true;
        }
    }
    false
}
