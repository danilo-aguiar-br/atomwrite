// SPDX-License-Identifier: MIT OR Apache-2.0

//! File restoration from a previous timestamped backup.
//! Workload: I/O-bound (backup read + atomic write).

use std::fs;
use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{GlobalArgs, RollbackArgs};
use crate::error::AtomwriteError;
use crate::ndjson_types::{RollbackPlan, RollbackResult};
use crate::output::NdjsonWriter;

/// Restore a file from a previous backup.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if no matching backup is found.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns an I/O error if restoration fails.
#[tracing::instrument(skip_all, fields(command = "rollback"))]
pub fn cmd_rollback(
    args: &RollbackArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let target = crate::path_safety::validate_path(&args.path, &workspace)?;

    let parent = target.parent().unwrap_or(std::path::Path::new("."));
    let filename = target
        .file_name()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "path has no filename".into(),
        })?
        .to_string_lossy();
    let prefix = format!("{filename}.bak.");

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

    if backups.is_empty() {
        return Err(AtomwriteError::NotFound {
            path: target.clone(),
        }
        .into());
    }

    backups.sort();

    let backup = if let Some(ref ts) = args.timestamp {
        let needle = format!("{prefix}{ts}");
        backups
            .iter()
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n == needle)
            })
            .cloned()
            .ok_or_else(|| AtomwriteError::NotFound {
                path: target.clone(),
            })?
    } else {
        // SAFETY: backups is guaranteed non-empty by the is_empty() check above.
        backups
            .last()
            .cloned()
            .expect("BUG: backups verified non-empty")
    };

    let target_str = target.display().to_string();
    let backup_str = backup.display().to_string();

    if args.dry_run {
        writer.write_event(&RollbackPlan {
            r#type: "plan",
            operation: "rollback",
            path: target_str,
            restore_from: backup_str,
        })?;
        return Ok(());
    }

    let max_size = global.effective_max_filesize();
    let checksum_before = if target.exists() {
        Some(checksum::hash_file(&target, max_size)?)
    } else {
        None
    };

    let content = crate::file_io::read_file_bytes(&backup, max_size)?;
    let opts = AtomicWriteOptions {
        backup: false,
        ..Default::default()
    };
    atomic_write(&target, &content, &opts, &workspace)?;

    let checksum_after = checksum::hash_file(&target, max_size)?;

    let verified = if args.verify {
        let backup_hash = checksum::hash_bytes(&content);
        Some(checksum_after == backup_hash)
    } else {
        None
    };

    writer.write_event(&RollbackResult {
        r#type: "rollback",
        path: target_str,
        restored_from: backup_str,
        checksum_before,
        checksum_after,
        verified,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}
