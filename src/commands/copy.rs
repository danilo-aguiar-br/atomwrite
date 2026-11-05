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
    let overwrite_authorized = args.force || args.backup_opts.backup == Some(true);
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
    // A directory the walk cannot open is a hole in the copied set, so the walk
    // records it instead of dropping the subtree in silence.
    let failures: std::sync::Arc<std::sync::Mutex<crate::ndjson_types::FailureTally>> =
        std::sync::Arc::new(std::sync::Mutex::new(
            crate::ndjson_types::FailureTally::default(),
        ));
    let failures_in_walk = std::sync::Arc::clone(&failures);
    // Sources the run examined, which is what `files_visited` reports: the
    // copies that landed are `files_modified`, and the ones that did not are
    // `files_failed`, so the three counters add up to what the run saw.
    let examined: u64;
    let mut copied_bytes: u64 = 0;
    if source.is_file() {
        examined = 1;
        let out = copy_file_atomic(&source, &target, args, &workspace, max_size, resolved)?;
        copied_bytes = out.bytes as u64;
        emit_copied(writer, &out, start)?;
    } else if source.is_dir() && args.recursive {
        // A tree copy ignores no ignore file, and dotfiles still follow
        // `--hidden`, so the shared walker runs in the copy flavor.
        // An ignore file the walk half-reads is a hole too, and `ignore` never
        // reports it by itself.
        crate::concurrency::record_ignore_file_failures(
            std::slice::from_ref(&source),
            Some(&failures),
        );
        let walker = crate::commands::build_scoped_walker_flavored(
            std::slice::from_ref(&source),
            global,
            crate::commands::WalkerFlavor::CopyTree,
            &[],
            &[],
        )?;
        let files =
            crate::concurrency::collect_files_parallel_tallied(&walker, Some(failures_in_walk));
        let mut pairs: Vec<(PathBuf, PathBuf)> = files
            .into_iter()
            .map(|src| {
                let rel = src.strip_prefix(&source).unwrap_or(&src);
                let dest = target.join(rel);
                (src, dest)
            })
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0)); // small lists; large trees already sorted by collect
        // `--limit` bounds the work, so the copy fan-out starts from a cut list.
        crate::output::cap_collected(&mut pairs);
        examined = pairs.len() as u64;

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

        let results: Vec<Result<CopiedMeta, anyhow::Error>> = if should_parallelize(pairs.len()) {
            pairs
                .par_iter()
                .map(|(src, dest)| {
                    if crate::signal::is_global_shutdown() {
                        return Err(crate::signal::cancelled_error(
                            crate::signal::current_cancel_reason(),
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

        // A copy that failed is a hole in the copied set, not the end of the
        // run: the files already copied stay, and the terminal says how many
        // could not be. Aborting here dropped the summary the consumer reads
        // the verdict off.
        for ((src, _dest), result) in pairs.iter().zip(results) {
            match result {
                Ok(out) => {
                    copied_bytes = copied_bytes.saturating_add(out.bytes as u64);
                    emit_copied(writer, &out, start)?;
                }
                Err(err) => failures
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record(src, err),
            }
        }
    } else {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("{} is a directory, use --recursive", source.display()),
        }
        .into());
    }

    emit_copy_summary(writer, failures, start, examined, copied_bytes)
}

/// Close a `copy` run with its terminal record.
///
/// Unconditional: a copied list without a terminal leaves the consumer unable
/// to tell a complete run from one the budget cut short.
fn emit_copy_summary(
    writer: &mut NdjsonWriter<impl Write>,
    failures: std::sync::Arc<std::sync::Mutex<crate::ndjson_types::FailureTally>>,
    start: Instant,
    examined: u64,
    total_bytes: u64,
) -> Result<()> {
    let tally = match std::sync::Arc::try_unwrap(failures) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => std::mem::take(&mut *arc.lock().unwrap_or_else(|e| e.into_inner())),
    };
    let copied = writer.emitted_count();
    let (summary, first) = tally.into_summary(crate::ndjson_types::Summary {
        r#type: "summary",
        files_visited: examined,
        files_matched: copied,
        files_modified: Some(copied),
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

struct CopiedMeta {
    source: String,
    target: String,
    bytes: usize,
    checksum: String,
    write_strategy: &'static str,
    backup_skipped: Option<String>,
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
        write_strategy: meta.write_strategy,
        backup_skipped: meta.backup_skipped.clone(),
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
        strict_atomic: args.strict.strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        // GAP-104: retain backup on disk when a backup was actually created
        keep_backup: resolved.keep || resolved.backup,
        durability: crate::platform::Durability::Auto,
    };

    let result = atomic_write(target, &content, &opts, workspace)?;

    // `--preserve` is an explicit request: a failure to carry the source
    // permissions or times over must reach the caller, not vanish.
    if args.preserve {
        let src_meta = std::fs::metadata(source).with_context(|| {
            format!(
                "cannot stat {} to preserve its attributes",
                source.display()
            )
        })?;
        // GAP-103: preserve source permissions after atomic write
        #[cfg(unix)]
        std::fs::set_permissions(target, src_meta.permissions())
            .with_context(|| format!("cannot preserve permissions on {}", target.display()))?;
        // GAP-133: preserve source mtime/atime after atomic write
        let mtime = filetime::FileTime::from_last_modification_time(&src_meta);
        let atime = filetime::FileTime::from_last_access_time(&src_meta);
        filetime::set_file_times(target, atime, mtime)
            .with_context(|| format!("cannot preserve timestamps on {}", target.display()))?;
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
        write_strategy: result.write_strategy,
        backup_skipped: result.backup_skipped,
    })
}
