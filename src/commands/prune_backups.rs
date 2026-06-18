// SPDX-License-Identifier: MIT OR Apache-2.0

//! Prune timestamped `.bak.YYYYMMDD_HHMMSS` files by age or by count.
//! Workload: I/O-bound (readdir + filter + delete).
//!
//! ADR-0040 — `prune-backups` is the bulk-complement of `backup`'s
//! per-file retention. Operators invoke it once to enforce a global
//! policy across many target files.

use std::fs;
use std::io::Write;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};

use crate::cli::{GlobalArgs, PruneBackupsArgs};
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

    for raw_path in &args.paths {
        let target = validate_path(raw_path, &workspace)?;

        if !target.exists() {
            writer.write_event(&PruneBackupEntry {
                r#type: "skipped",
                path: target.display().to_string(),
                reason: "not_found".to_string(),
                error: None,
            })?;
            continue;
        }

        // Backup pattern: `<basename>.bak.YYYYMMDD_HHMMSS`
        let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
        let file_name = target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let prefix = format!("{file_name}.bak.");

        let mut backups: Vec<std::path::PathBuf> = fs::read_dir(parent)
            .with_context(|| format!("cannot list directory {}", parent.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&prefix))
            })
            .collect();

        // Age filter: drop backups newer than the cutoff.
        if let Some(max_age_secs) = args.max_age_secs {
            let now = SystemTime::now();
            let cutoff = now - Duration::from_secs(u64::from(max_age_secs));
            backups.retain(|p| {
                p.metadata()
                    .and_then(|m| m.modified())
                    .map(|m| m < cutoff)
                    .unwrap_or(false)
            });
        }

        // Count filter: keep the N most recent backups (delete the rest).
        // Per ADR-0040, `--max-count N` means "keep at most N most-recent",
        // so we delete the (total - N) OLDEST entries. After sorting
        // ascending (oldest first), the slice to delete is the prefix of
        // length `len - N`. The default `N = 0` is documented as
        // "unlimited" (no deletion via the count filter).
        // SAFETY (VAI-PSIQUE-CHECK per ADR-0040): refuse to delete anything
        // unless the operator explicitly chose a filter. Without either
        // `--max-age-secs` or `--max-count`, the command would otherwise
        // delete every backup matching the target — a silent data-loss
        // footgun. The default dry-run does NOT mitigate this because
        // the operator may have explicitly passed `--dry-run false`.
        if args.max_age_secs.is_none() && args.max_count.is_none() {
            anyhow::bail!(
                "refusing to prune without --max-age-secs or --max-count; \
                 pass at least one to define the retention policy"
            );
        }

        if let Some(max_count) = args.max_count {
            if max_count > 0 {
                backups.sort_by_key(|p| {
                    p.metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH)
                });
                let to_delete = backups.len().saturating_sub(usize::from(max_count));
                backups.truncate(to_delete);
            }
        }

        // Tag the reason: "age" takes precedence over "count" because it
        // is the more selective filter and matches the operator's intent
        // when both flags are combined.
        let reason = if args.max_age_secs.is_some() {
            "age"
        } else {
            "count"
        };

        for backup in &backups {
            if !dry_run {
                if let Err(e) = fs::remove_file(backup) {
                    writer.write_event(&PruneBackupEntry {
                        r#type: "error",
                        path: backup.display().to_string(),
                        reason: "remove_failed".to_string(),
                        error: Some(e.to_string()),
                    })?;
                    continue;
                }
            }
            writer.write_event(&PruneBackupEntry {
                r#type: "pruned",
                path: backup.display().to_string(),
                reason: reason.to_string(),
                error: None,
            })?;
            total_pruned += 1;
        }
    }

    writer.write_event(&PruneBackupSummary {
        r#type: "summary",
        action: if dry_run { "dry_run" } else { "pruned" }.to_string(),
        total: total_pruned,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}
