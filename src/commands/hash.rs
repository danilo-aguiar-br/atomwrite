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
use crate::ndjson_types::{HashOutput, HashVerifyOutput};
use crate::output::{NdjsonWriter, VerdictPublished};

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
        let (hash, len) = checksum::hash_reader_with_len(&mut reader)?;
        writer.write_event(&HashOutput {
            r#type: "hash",
            path: None,
            source: Some("stdin"),
            algorithm: "blake3",
            value: hash,
            bytes: Some(len),
            verified: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
        // A stdin run closes with the same terminal every file run emits, so a
        // consumer reads one shape and learns the stream size in `total_bytes`.
        let failures = std::sync::Arc::new(std::sync::Mutex::new(
            crate::ndjson_types::FailureTally::default(),
        ));
        return emit_hash_summary(writer, 1, len, failures, start);
    }

    // A directory the walk cannot open is a hole in the digest list, so the
    // walk records it instead of dropping the subtree in silence.
    let failures: std::sync::Arc<std::sync::Mutex<crate::ndjson_types::FailureTally>> =
        std::sync::Arc::new(std::sync::Mutex::new(
            crate::ndjson_types::FailureTally::default(),
        ));
    let failures_in_walk = std::sync::Arc::clone(&failures);
    let mut file_paths: Vec<PathBuf> = Vec::new();
    let mut dir_roots: Vec<PathBuf> = Vec::new();
    // Excludes are matched by name below; an invalid glob is refused up front.
    crate::walk_globs::validate(&[], &args.exclude)?;
    // Recipe and agents pass exclude; recursive walks always skip backups (A-027).
    let excludes = if args.exclude.is_empty() && args.recursive {
        crate::commands::backup_exclude_globs()
    } else {
        args.exclude.clone()
    };
    let skip_bak_in_walk = args.recursive
        || excludes
            .iter()
            .any(|e| e.contains(crate::constants::BACKUP_NAME_MARKER));

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
        // Hashing a tree means every file in it, dotfiles included, so this verb
        // takes the always-hidden flavor of the one shared walker.
        // An ignore file the walk half-reads is a hole too, and `ignore` never
        // reports it by itself.
        crate::concurrency::record_ignore_file_failures(&dir_roots, Some(&failures));
        let mut builder = crate::commands::build_scoped_walker_flavored(
            &dir_roots,
            global,
            crate::commands::WalkerFlavor::AlwaysVisitHidden,
            &[],
            &[],
        )?;
        if skip_bak_in_walk {
            builder.filter_entry(|entry| !path_looks_like_bak(entry.path()));
        }
        let walked =
            crate::concurrency::collect_files_parallel_tallied(&builder, Some(failures_in_walk));
        let filtered: Vec<PathBuf> =
            if crate::concurrency::should_parallelize(walked.len()) && !excludes.is_empty() {
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
    // `--limit` bounds the work: hashing stops at the budget instead of reading
    // the whole tree and printing a fraction of it.
    crate::output::cap_collected(&mut file_paths);

    let max_size = global.effective_max_filesize();

    // Parallel path: independent digests, then emit in sorted order.
    // Sequential when `--verify` needs fail-fast, or single file (no fan-out gain).
    let files_visited = file_paths.len() as u64;

    if args.verify.is_none() && crate::concurrency::should_parallelize(file_paths.len()) {
        let total_bytes = emit_hashes_parallel(&file_paths, max_size, start, writer)?;
        return emit_hash_summary(writer, files_visited, total_bytes, failures, start);
    }

    let mut total_bytes = 0u64;
    for path in &file_paths {
        if crate::signal::is_global_shutdown() {
            break;
        }
        total_bytes = total_bytes.saturating_add(emit_one_hash(
            path,
            max_size,
            args.verify.as_deref(),
            start,
            writer,
        )?);
    }

    emit_hash_summary(writer, files_visited, total_bytes, failures, start)
}

/// Close a `hash` run with its terminal record.
///
/// Emitted on EVERY run, truncated or not: without a terminal the consumer
/// cannot tell a complete list from one the budget cut short.
fn emit_hash_summary(
    writer: &mut NdjsonWriter<impl Write>,
    files_visited: u64,
    total_bytes: u64,
    failures: std::sync::Arc<std::sync::Mutex<crate::ndjson_types::FailureTally>>,
    start: Instant,
) -> Result<()> {
    let tally = match std::sync::Arc::try_unwrap(failures) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => std::mem::take(&mut *arc.lock().unwrap_or_else(|e| e.into_inner())),
    };
    let emitted = writer.emitted_count();
    let (summary, first) = tally.into_summary(crate::ndjson_types::Summary {
        r#type: "summary",
        files_visited,
        files_matched: emitted,
        files_modified: None,
        files_skipped: None,
        total_matches: None,
        total_replacements: None,
        total_bytes: Some(total_bytes),
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        elapsed_ms: start.elapsed().as_millis() as u64,
        walker_stopped_early: crate::output::limit_reached().then_some(true),
        ..crate::ndjson_types::Summary::default()
    });
    if let Some(err) = first {
        return Err(writer.publish_failure(&summary, err));
    }
    writer.write_terminal(&summary)?;
    Ok(())
}

/// Hash many files in parallel; emit NDJSON in the same order as `file_paths`.
fn emit_hashes_parallel(
    file_paths: &[PathBuf],
    max_size: u64,
    start: Instant,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<u64> {
    // Collect digests first so stdout order stays deterministic (sorted paths).
    let results: Vec<Result<(String, u64), anyhow::Error>> = file_paths
        .par_iter()
        .map(|path| {
            if crate::signal::is_global_shutdown() {
                return Err(
                    crate::signal::cancelled_error(crate::signal::current_cancel_reason()).into(),
                );
            }
            checksum::hash_file_with_len(path, max_size)
        })
        .collect();

    if crate::signal::is_global_shutdown() {
        return Ok(0);
    }

    let mut total = 0u64;
    for (path, result) in file_paths.iter().zip(results) {
        let (hash, bytes) = result?;
        total = total.saturating_add(bytes);
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
    Ok(total)
}

fn emit_one_hash(
    path: &Path,
    max_size: u64,
    verify: Option<&str>,
    start: Instant,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<u64> {
    let path_str = path.display().to_string();
    // Single open+stat: digest and length together (no second metadata()).
    let (hash, bytes) = checksum::hash_file_with_len(path, max_size)?;

    if let Some(expected) = verify {
        let verified = hash == expected;
        // B2: one verification publishes exactly ONE event, and the verdict
        // (`ok` together with `verified`) travels inside it. The failure code
        // and the process status are read off the error variant itself, so the
        // event and the exit status cannot disagree.
        let failure = (!verified).then(|| crate::error::AtomwriteError::ChecksumVerifyFailed {
            path: path.to_path_buf(),
            expected: expected.to_string(),
        });
        writer.write_event(&HashVerifyOutput {
            r#type: "hash_verify",
            path: path_str,
            algorithm: "blake3",
            value: hash,
            expected: expected.to_string(),
            verified,
            ok: verified,
            code: failure.as_ref().map(|e| e.error_code()),
            exit: failure.as_ref().map(|e| e.exit_code()),
            bytes,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
        if let Some(err) = failure {
            // The verdict is already on the data channel, in the very event
            // written above. The `Err` carries the same status the event
            // advertises, and the marker tells `main` not to write a second
            // envelope, so one verification stays one data line (B2).
            return Err(anyhow::Error::from(err).context(VerdictPublished));
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
    Ok(bytes)
}

fn path_looks_like_bak(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.contains(crate::constants::BACKUP_INFIX))
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
            if name.contains(crate::constants::BACKUP_INFIX)
                || s.contains(crate::constants::BACKUP_INFIX)
            {
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
