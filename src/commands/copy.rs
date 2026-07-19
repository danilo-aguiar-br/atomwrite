// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file copy with BLAKE3 checksum verification.
//!
//! Workload: I/O-bound (file read + atomic write).
//! Parallelism: recursive discovery via `collect_files_parallel` (honors
//! `--threads`); unique destination parents via `par_iter` `create_dir_all`;
//! then `rayon::par_iter` over `(src, dest)` pairs. NDJSON emits in
//! path-sorted order after the join. Single-file stays sequential.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{CopyArgs, GlobalArgs};
use crate::commands::{ResolvedBackup, resolve_backup};
use crate::concurrency::should_parallelize;
use crate::error::AtomwriteError;
use crate::ndjson_types::{CopyOutput, TransferPlan};
use crate::output::NdjsonWriter;

/// Copy files with checksum verification and atomic destination write.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the source file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if either path escapes the workspace.
/// Returns `AtomwriteError::InvalidInput` if source and destination are the same file or the target already exists.
/// Returns `AtomwriteError::Io` if reading or writing fails.
#[tracing::instrument(skip_all, fields(command = "copy"))]
pub fn cmd_copy(
    args: &CopyArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let resolved = resolve_backup(&args.backup_opts, defaults);

    let source = crate::path_safety::validate_path(&args.source, &workspace)?;
    let target = crate::path_safety::validate_path(&args.target, &workspace)?;

    if !source.exists() {
        return Err(AtomwriteError::NotFound { path: source }.into());
    }

    if target.exists() {
        if let (Ok(src_h), Ok(dst_h)) = (
            same_file::Handle::from_path(&source),
            same_file::Handle::from_path(&target),
        ) {
            if src_h == dst_h {
                return Err(AtomwriteError::InvalidInput {
                    reason: "source and target are the same file".into(),
                }
                .into());
            }
        }
    }

    // v0.1.28 / G-007: overwrite requires EXPLICIT CLI authorization
    // (--force or --backup). No product env knobs.
    let overwrite_authorized =
        args.force || args.backup_opts.backup == Some(true);
    if target.exists() && !overwrite_authorized {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "target {} already exists, use --force to overwrite",
                target.display()
            ),
        }
        .into());
    }

    if args.dry_run {
        writer.write_event(&TransferPlan {
            r#type: "plan",
            operation: "copy",
            source: source.display().to_string(),
            target: target.display().to_string(),
            would_modify: true,
        })?;
        return Ok(());
    }

    let max_size = global.effective_max_filesize();
    if source.is_file() {
        let out = copy_file_atomic(&source, &target, args, &workspace, max_size, resolved)?;
        emit_copied(writer, &out, start)?;
    } else if source.is_dir() && args.recursive {
        let mut walker = ignore::WalkBuilder::new(&source);
        walker.hidden(true).git_ignore(false);
        crate::concurrency::apply_walk_threads(&mut walker, global.threads);
        let files = crate::concurrency::collect_files_parallel(&walker);
        let mut pairs: Vec<(PathBuf, PathBuf)> = files
            .into_iter()
            .map(|src| {
                let rel = src.strip_prefix(&source).unwrap_or(&src);
                let dest = target.join(rel);
                (src, dest)
            })
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0)); // small lists; large trees already sorted by collect

        // Unique parents, then parallel create_dir_all (independent I/O).
        let mut parents: Vec<PathBuf> = pairs
            .iter()
            .filter_map(|(_, dest)| dest.parent().map(|p| p.to_path_buf()))
            .collect();
        crate::concurrency::sort_paths_parallel(&mut parents);
        parents.dedup();
        if should_parallelize(parents.len()) {
            parents.par_iter().try_for_each(|p| {
                std::fs::create_dir_all(p)
                    .with_context(|| format!("cannot create parent directory {}", p.display()))
            })?;
        } else {
            for p in &parents {
                std::fs::create_dir_all(p)
                    .with_context(|| format!("cannot create parent directory {}", p.display()))?;
            }
        }

        let results: Vec<Result<CopiedMeta, anyhow::Error>> =
            if should_parallelize(pairs.len()) {
                pairs
                    .par_iter()
                    .map(|(src, dest)| {
                        if crate::signal::is_global_shutdown() {
                            return Err(crate::signal::cancelled_error(
                                "copy cancelled by signal",
                            )
                            .into());
                        }
                        copy_file_atomic(src, dest, args, &workspace, max_size, resolved)
                    })
                    .collect()
            } else {
                pairs
                    .iter()
                    .map(|(src, dest)| {
                        copy_file_atomic(src, dest, args, &workspace, max_size, resolved)
                    })
                    .collect()
            };

        if crate::signal::is_global_shutdown() {
            return Ok(());
        }

        for result in results {
            emit_copied(writer, &result?, start)?;
        }
    } else {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("{} is a directory, use --recursive", source.display()),
        }
        .into());
    }

    Ok(())
}

struct CopiedMeta {
    source: String,
    target: String,
    bytes: usize,
    checksum: String,
}

fn emit_copied(
    writer: &mut NdjsonWriter<impl Write>,
    meta: &CopiedMeta,
    start: Instant,
) -> Result<()> {
    writer.write_event(&CopyOutput {
        r#type: "copied",
        source: meta.source.clone(),
        target: meta.target.clone(),
        bytes: meta.bytes,
        checksum: meta.checksum.clone(),
        verified: true,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;
    Ok(())
}

fn copy_file_atomic(
    source: &Path,
    target: &Path,
    args: &CopyArgs,
    workspace: &Path,
    max_size: u64,
    resolved: ResolvedBackup,
) -> Result<CopiedMeta> {
    let content = crate::file_io::read_file_bytes(source, max_size)?;
    let source_hash = checksum::hash_bytes(&content);

    let opts = AtomicWriteOptions {
        backup: resolved.backup,
        syntax_check: false,
        retention: resolved.retention,
        preserve_timestamps: args.preserve,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        // GAP-104: retain backup on disk when a backup was actually created
        keep_backup: resolved.keep || resolved.backup,
        durability: crate::platform::Durability::Auto,
    };

    let result = atomic_write(target, &content, &opts, workspace)?;

    // GAP-103: preserve source permissions after atomic write
    #[cfg(unix)]
    if args.preserve {
        if let Ok(src_meta) = std::fs::metadata(source) {
            let _ = std::fs::set_permissions(target, src_meta.permissions());
        }
    }

    // GAP-133: preserve source mtime/atime after atomic write
    if args.preserve {
        if let Ok(src_meta) = std::fs::metadata(source) {
            let mtime = filetime::FileTime::from_last_modification_time(&src_meta);
            let atime = filetime::FileTime::from_last_access_time(&src_meta);
            let _ = filetime::set_file_times(target, atime, mtime);
        }
    }

    if result.checksum != source_hash {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "checksum mismatch after copy: source={source_hash}, target={}",
                result.checksum
            ),
        }
        .into());
    }

    Ok(CopiedMeta {
        source: source.display().to_string(),
        target: target.display().to_string(),
        bytes: content.len(),
        checksum: source_hash,
    })
}
