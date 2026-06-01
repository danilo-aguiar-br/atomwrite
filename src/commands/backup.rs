// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standalone file backup with timestamped copies and BLAKE3 checksums.
//! Workload: I/O-bound (file copy + fsync).

use std::io::Write;
use std::time::Instant;

use anyhow::Result;

use crate::checksum;
use crate::cli::{BackupArgs, GlobalArgs};
use crate::error::AtomwriteError;
use crate::ndjson_types::{BackupPlan, BackupResult, BackupSummary};
use crate::output::NdjsonWriter;

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

    for path in &args.paths {
        let source = crate::path_safety::validate_path(path, &workspace)?;

        if !source.exists() {
            return Err(AtomwriteError::NotFound {
                path: source.clone(),
            }
            .into());
        }

        if !source.is_file() {
            tracing::warn!(path = %source.display(), "skipping non-file path");
            continue;
        }

        let file_start = Instant::now();
        let source_str = source.display().to_string();
        let hash = checksum::hash_file(&source, global.effective_max_filesize())?;
        let bytes = std::fs::metadata(&source)?.len();

        if args.dry_run {
            writer.write_event(&BackupPlan {
                r#type: "plan",
                operation: "backup",
                path: source_str,
                bytes,
                checksum: hash,
            })?;
            continue;
        }

        let backup_path = crate::atomic::create_backup(&source, args.retention)?;

        writer.write_event(&BackupResult {
            r#type: "backup",
            path: source_str,
            backup_path: backup_path.display().to_string(),
            checksum: hash,
            bytes,
            elapsed_ms: file_start.elapsed().as_millis() as u64,
        })?;

        backed_up += 1;
        total_bytes += bytes;
    }

    writer.write_event(&BackupSummary {
        r#type: "summary",
        files_backed_up: backed_up,
        total_bytes,
        dry_run: args.dry_run,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}
