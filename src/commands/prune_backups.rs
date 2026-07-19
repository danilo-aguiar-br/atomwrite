// SPDX-License-Identifier: MIT OR Apache-2.0

//! Prune timestamped `.bak.YYYYMMDD_HHMMSS` files by age or by count.
//! Workload: I/O-bound (readdir + filter + delete).
//! Parallelism: phase-1 readdir/retention fans out per target (`par_iter`);
//! phase-2 unlinks all selected backups with one `par_iter`. NDJSON emits
//! in path-sorted order after the join. Bound: process-wide rayon pool.
//!
//! ADR-0040 — `prune-backups` is the bulk-complement of `backup`'s
//! per-file retention. Operators invoke it once to enforce a global
//! policy across many target files.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::cli::{GlobalArgs, PruneBackupsArgs};
use crate::concurrency::should_parallelize;
use crate::ndjson_types::{PruneBackupEntry, PruneBackupSummary};
use crate::output::NdjsonWriter;
use crate::path_safety::validate_path;

/// Prune timestamped backups of one or more target files.
///
/// Emits one `prune-backups` event per backup handled (pruned, skipped,
/// or error) and a single `summary` event at the end.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if any path escapes the workspace.
/// Returns an I/O error if the parent directory cannot be listed.
#[tracing::instrument(skip_all, fields(command = "prune-backups"))]
pub fn cmd_prune_backups(
    args: &PruneBackupsArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let dry_run = args.dry_run;
    let mut total_pruned: usize = 0;

    // SAFETY (VAI-PSIQUE-CHECK per ADR-0040): refuse without a retention filter.
    if args.max_age_secs.is_none() && args.max_count.is_none() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "refusing to prune without --max-age-secs or --max-count; \
                     pass at least one to define the retention policy"
                .into(),
        }
        .into());
    }

    let reason = if args.max_age_secs.is_some() {
        "age"
    } else {
        "count"
    };
    let reason_owned = reason.to_string();

    // Phase 1 — per-target retention (policy is per-file) but readdir fans out
    // across independent targets, then flatten for the unlink stage.
    let max_age_secs = args.max_age_secs;
    let max_count = args.max_count;
    let phase1: Vec<Result<(Vec<PathBuf>, Option<PruneBackupEntry>), anyhow::Error>> =
        if should_parallelize(args.paths.len()) {
            args.paths
                .par_iter()
                .map(|raw_path| {
                    collect_backups_for_target(raw_path, &workspace, max_age_secs, max_count)
                })
                .collect()
        } else {
            args.paths
                .iter()
                .map(|raw_path| {
                    collect_backups_for_target(raw_path, &workspace, max_age_secs, max_count)
                })
                .collect()
        };

    let mut to_prune: Vec<PathBuf> = Vec::new();
    for item in phase1 {
        let (backups, skip) = item?;
        if let Some(entry) = skip {
            writer.write_event(&entry)?;
        }
        to_prune.extend(backups);
    }

    crate::concurrency::sort_paths_parallel(&mut to_prune);
    to_prune.dedup();

    // Phase 2 — one fan-out across all selected backups (multi-target).
    let outcomes: Vec<PruneBackupEntry> = if should_parallelize(to_prune.len()) {
        to_prune
            .par_iter()
            .map(|backup| prune_one(backup, dry_run, &reason_owned))
            .collect()
    } else {
        to_prune
            .iter()
            .map(|backup| prune_one(backup, dry_run, &reason_owned))
            .collect()
    };

    // Emit in path order for stable NDJSON (par_iter order is not sorted).
    let mut outcomes = outcomes;
    outcomes.sort_by(|a, b| a.path.cmp(&b.path));

    for entry in outcomes {
        if entry.r#type == "pruned" {
            total_pruned += 1;
        }
        writer.write_event(&entry)?;
    }

    writer.write_event(&PruneBackupSummary {
        r#type: "summary",
        action: if dry_run { "dry_run" } else { "pruned" }.to_string(),
        total: total_pruned,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}

/// Per-target readdir + retention filter (safe to run in parallel across targets).
fn collect_backups_for_target(
    raw_path: &std::path::Path,
    workspace: &std::path::Path,
    max_age_secs: Option<u32>,
    max_count: Option<u8>,
) -> Result<(Vec<PathBuf>, Option<PruneBackupEntry>)> {
    let target = validate_path(raw_path, workspace)?;

    if !target.exists() {
        return Ok((
            Vec::new(),
            Some(PruneBackupEntry {
                r#type: "skipped",
                path: target.display().to_string(),
                reason: "not_found".to_string(),
                error: None,
            }),
        ));
    }

    let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let prefix = format!("{file_name}.bak.");

    let mut backups: Vec<PathBuf> = fs::read_dir(parent)
        .with_context(|| format!("cannot list directory {}", parent.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix))
        })
        .collect();

    if let Some(max_age_secs) = max_age_secs {
        let now = SystemTime::now();
        let cutoff = now - Duration::from_secs(u64::from(max_age_secs));
        backups.retain(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .map(|m| m < cutoff)
                .unwrap_or(false)
        });
    }

    if let Some(max_count) = max_count {
        if max_count > 0 {
            backups.sort();
            let to_delete = backups.len().saturating_sub(usize::from(max_count));
            backups.truncate(to_delete);
        }
    }

    Ok((backups, None))
}

fn prune_one(backup: &PathBuf, dry_run: bool, reason: &str) -> PruneBackupEntry {
    if !dry_run {
        if let Err(e) = fs::remove_file(backup) {
            return PruneBackupEntry {
                r#type: "error",
                path: backup.display().to_string(),
                reason: "remove_failed".to_string(),
                error: Some(e.to_string()),
            };
        }
    }
    PruneBackupEntry {
        r#type: "pruned",
        path: backup.display().to_string(),
        reason: reason.to_string(),
        error: None,
    }
}
