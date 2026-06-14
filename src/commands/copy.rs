// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file copy with BLAKE3 checksum verification.
//! Workload: I/O-bound (file read + atomic write).

use std::io::Write;
use std::time::Instant;

use anyhow::Result;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{CopyArgs, GlobalArgs};
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

    if target.exists() && !args.force {
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
            source: source_str.clone(),
            target: target_str.clone(),
            would_modify: true,
        })?;
        return Ok(());
    }

    let max_size = global.effective_max_filesize();
    if source.is_file() {
        copy_file_atomic(&source, &target, args, &workspace, writer, start, max_size)?;
    } else if source.is_dir() && args.recursive {
        for entry in ignore::WalkBuilder::new(&source)
            .hidden(true)
            .git_ignore(false)
            .build()
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }
            let rel = entry.path().strip_prefix(&source).unwrap_or(entry.path());
            let dest = target.join(rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            copy_file_atomic(
                entry.path(),
                &dest,
                args,
                &workspace,
                writer,
                start,
                max_size,
            )?;
        }
    } else {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("{} is a directory, use --recursive", source.display()),
        }
        .into());
    }

    Ok(())
}

fn copy_file_atomic(
    source: &std::path::Path,
    target: &std::path::Path,
    args: &CopyArgs,
    workspace: &std::path::Path,
    writer: &mut NdjsonWriter<impl Write>,
    start: Instant,
    max_size: u64,
) -> Result<()> {
    let content = crate::file_io::read_file_bytes(source, max_size)?;
    let source_hash = checksum::hash_bytes(&content);

    let opts = AtomicWriteOptions {
        backup: args.backup,
        syntax_check: false,
        retention: 5,
        preserve_timestamps: args.preserve,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
    };

    let result = atomic_write(target, &content, &opts, workspace)?;

    if result.checksum != source_hash {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "checksum mismatch after copy: source={source_hash}, target={}",
                result.checksum
            ),
        }
        .into());
    }

    writer.write_event(&CopyOutput {
        r#type: "copied",
        source: source.display().to_string(),
        target: target.display().to_string(),
        bytes: content.len(),
        checksum: source_hash,
        verified: true,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}
