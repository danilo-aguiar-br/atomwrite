// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standalone file backup with timestamped copies and BLAKE3 checksums.
//!
//! Workload: I/O-bound (file copy + fsync).
//! Parallelism: multi-path `backup` hashes and copies in parallel via
//! `rayon::par_iter` when `paths.len() > 1`. NDJSON emits in input order
//! after the join. Bound: process-wide rayon pool (`--threads` /
//! `--max-concurrency`).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use rayon::prelude::*;

use crate::checksum;
use crate::cli::{BackupArgs, GlobalArgs};
use crate::concurrency::should_parallelize;
use crate::error::AtomwriteError;
use crate::ndjson_types::{BackupPlan, BackupResult, BackupSummary};
use crate::output::NdjsonWriter;

enum BackupItem {
    Plan {
        path: String,
        bytes: u64,
        checksum: String,
    },
    Done {
        path: String,
        backup_path: String,
        checksum: String,
        bytes: u64,
        elapsed_ms: u64,
    },
    SkipNonFile,
}

/// Create timestamped backups of one or more files.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if a source file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if a path escapes the workspace.
/// Returns an I/O error if backup creation fails.
#[tracing::instrument(skip_all, fields(command = "backup"))]
pub fn cmd_backup(
    args: &BackupArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let mut backed_up = 0u64;
    let mut total_bytes = 0u64;
    let max_size = global.effective_max_filesize();
    let retention = args.retention;
    let output_dir = args.output_dir.as_deref();
    let dry_run = args.dry_run;

    // Validate all paths first so NotFound fails fast before fan-out.
    let mut sources: Vec<PathBuf> = Vec::with_capacity(args.paths.len());
    for path in &args.paths {
        let source = crate::path_safety::validate_path(path, &workspace)?;
        if !source.exists() {
            return Err(AtomwriteError::NotFound { path: source }.into());
        }
        sources.push(source);
    }

    // `--limit` bounds the work: a backup copies bytes, so the cut has to land
    // before the fan-out, not after it.
    crate::output::cap_collected(&mut sources);

    let items: Vec<Result<BackupItem, anyhow::Error>> = if should_parallelize(sources.len()) {
        sources
            .par_iter()
            .map(|source| process_one_backup(source, max_size, retention, output_dir, dry_run))
            .collect()
    } else {
        sources
            .iter()
            .map(|source| process_one_backup(source, max_size, retention, output_dir, dry_run))
            .collect()
    };

    // One source that cannot be backed up is a hole in the answer, not the end
    // of the run: the copies already on disk stay useful, and the summary names
    // what failed instead of vanishing behind an error envelope.
    let mut failures = crate::ndjson_types::FailureTally::default();
    let mut skipped = 0u64;
    for (source, item) in sources.iter().zip(items) {
        match item {
            Err(err) => failures.record(source, err),
            Ok(BackupItem::SkipNonFile) => skipped += 1,
            Ok(BackupItem::Plan {
                path,
                bytes,
                checksum,
            }) => {
                writer.write_event(&BackupPlan {
                    r#type: "plan",
                    operation: "backup",
                    path,
                    bytes,
                    checksum,
                })?;
            }
            Ok(BackupItem::Done {
                path,
                backup_path,
                checksum,
                bytes,
                elapsed_ms,
            }) => {
                writer.write_event(&BackupResult {
                    r#type: "backup",
                    path,
                    backup_path,
                    checksum,
                    bytes,
                    elapsed_ms,
                })?;
                backed_up += 1;
                total_bytes += bytes;
            }
        }
    }

    let (failed_count, failed_items, first) = failures.into_parts();
    let files_failed = (failed_count > 0).then_some(failed_count);
    let summary = BackupSummary {
        r#type: "summary",
        files_backed_up: backed_up,
        total_bytes,
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        dry_run: args.dry_run,
        elapsed_ms: start.elapsed().as_millis() as u64,
        walker_stopped_early: crate::output::limit_reached().then_some(true),
        files_skipped: (skipped > 0).then_some(skipped),
        files_failed,
        partial: crate::ndjson_types::partial_of(files_failed),
        errors: (!failed_items.is_empty()).then_some(failed_items),
    };
    // A dry run copies nothing, so its verdict is the plan it printed; a real
    // run that copied at least one source answered, and only a run with no copy
    // at all exits with the failure.
    if let Some(err) = first
        && backed_up == 0
        && !dry_run
    {
        return Err(writer.publish_failure(&summary, err));
    }
    writer.write_terminal(&summary)?;

    Ok(())
}

fn process_one_backup(
    source: &Path,
    max_size: u64,
    retention: u8,
    output_dir: Option<&Path>,
    dry_run: bool,
) -> Result<BackupItem> {
    if crate::signal::is_global_shutdown() {
        return Err(crate::signal::cancelled_error(crate::signal::current_cancel_reason()).into());
    }
    if !source.is_file() {
        tracing::warn!(path = %source.display(), "skipping non-file path");
        return Ok(BackupItem::SkipNonFile);
    }

    let file_start = Instant::now();
    let source_str = source.display().to_string();
    let hash = checksum::hash_file(source, max_size)?;
    let bytes = std::fs::metadata(source)?.len();

    if dry_run {
        return Ok(BackupItem::Plan {
            path: source_str,
            bytes,
            checksum: hash,
        });
    }

    let backup_path = crate::atomic::create_backup_in(source, retention, output_dir)?;

    Ok(BackupItem::Done {
        path: source_str,
        backup_path: backup_path.display().to_string(),
        checksum: hash,
        bytes,
        elapsed_ms: file_start.elapsed().as_millis() as u64,
    })
}
