// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file move with cross-device fallback to copy-then-delete.
//! Workload: I/O-bound (rename syscall + fsync).

use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::checksum;
use crate::cli::{GlobalArgs, MoveArgs};
use crate::error::AtomwriteError;
use crate::ndjson_types::{MoveOutput, TransferPlan};
use crate::output::NdjsonWriter;
use crate::platform;

/// Move or rename a file atomically with optional backup.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the source file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if either path escapes the workspace.
/// Returns `AtomwriteError::InvalidInput` if source and destination are the same file or the target already exists.
/// Returns `AtomwriteError::CrossDevice` if the move crosses filesystem boundaries (handled internally via copy+delete).
#[tracing::instrument(skip_all, fields(command = "move"))]
pub fn cmd_move(
    args: &MoveArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let source = crate::path_safety::validate_path(&args.source, &workspace)?;
    let target = crate::path_safety::validate_path(&args.target, &workspace)?;
    let source_str = source.display().to_string();
    let target_str = target.display().to_string();

    if !source.exists() {
        return Err(AtomwriteError::NotFound {
            path: source.clone(),
        }
        .into());
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

    if target.exists() && !args.force && !args.backup {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "target {} already exists, use --force or --backup",
                target.display()
            ),
        }
        .into());
    }

    if args.dry_run {
        writer.write_event(&TransferPlan {
            r#type: "plan",
            operation: "move",
            source: source_str.clone(),
            target: target_str.clone(),
            would_modify: true,
        })?;
        return Ok(());
    }

    if args.backup && target.exists() {
        crate::atomic::create_backup(&target, args.retention)?;
    }

    if let Some(parent) = target.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create dirs for {}", target.display()))?;
        }
    }

    let max_size = global.effective_max_filesize();
    let hash = checksum::hash_file(&source, max_size)?;
    let bytes = std::fs::metadata(&source)?.len();

    match std::fs::rename(&source, &target) {
        Ok(()) => {
            if let Some(src_parent) = source.parent() {
                if let Err(e) = platform::fsync_dir(src_parent) {
                    tracing::warn!(
                        path = %src_parent.display(),
                        error = %e,
                        "fsync_dir after move failed"
                    );
                }
            }
            if let Some(tgt_parent) = target.parent() {
                if let Err(e) = platform::fsync_dir(tgt_parent) {
                    tracing::warn!(
                        path = %tgt_parent.display(),
                        error = %e,
                        "fsync_dir after move failed"
                    );
                }
            }

            writer.write_event(&MoveOutput {
                r#type: "moved",
                source: source_str.clone(),
                target: target_str.clone(),
                bytes,
                checksum: hash,
                cross_device: false,
                atomic: true,
                elapsed_ms: start.elapsed().as_millis() as u64,
            })?;
        }
        Err(e) if e.raw_os_error() == Some(18) => {
            // EXDEV = 18 on Linux
            // Cross-device: copy + delete
            let content = crate::file_io::read_file_bytes(&source, max_size)?;
            crate::atomic::atomic_write(
                &target,
                &content,
                &crate::atomic::AtomicWriteOptions::default(),
                &workspace,
            )?;
            let verify = checksum::hash_file(&target, max_size)?;
            if verify != hash {
                return Err(AtomwriteError::InvalidInput {
                    reason: format!(
                        "checksum mismatch after cross-device copy: expected {hash}, got {verify}"
                    ),
                }
                .into());
            }
            std::fs::remove_file(&source)
                .with_context(|| format!("cannot remove source {}", source.display()))?;
            if let Some(parent) = source.parent() {
                if let Err(e) = platform::fsync_dir(parent) {
                    tracing::warn!(
                        path = %parent.display(),
                        error = %e,
                        "fsync_dir after cross-device move failed"
                    );
                }
            }

            writer.write_event(&MoveOutput {
                r#type: "moved",
                source: source_str.clone(),
                target: target_str.clone(),
                bytes,
                checksum: hash,
                cross_device: true,
                atomic: false,
                elapsed_ms: start.elapsed().as_millis() as u64,
            })?;
        }
        Err(e) => {
            return Err(e).with_context(|| {
                format!("cannot move {} to {}", source.display(), target.display())
            });
        }
    }

    Ok(())
}
