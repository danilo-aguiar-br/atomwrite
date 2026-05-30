// SPDX-License-Identifier: MIT OR Apache-2.0

//! Batch execution of multiple operations from an NDJSON manifest.

use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::GlobalArgs;
use crate::ndjson_types::{BatchOpResult, BatchSummary};
use crate::output::NdjsonWriter;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchOp {
    op: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    replacement: Option<String>,
    #[serde(default)]
    backup: bool,
    #[serde(default)]
    old: Option<String>,
    #[serde(default)]
    new: Option<String>,
}

/// NDJSON event emitted when a transaction is rolled back.
#[derive(Debug, Serialize)]
struct RollbackEvent {
    r#type: &'static str,
    operations_reverted: u64,
}

/// Execute multiple operations from an NDJSON manifest in batch mode.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if reading stdin or writing results fails.
/// Returns `AtomwriteError::InvalidInput` if the manifest contains invalid operations.
pub fn cmd_batch(
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    dry_run: bool,
    transaction: bool,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let reader = std::io::BufReader::new(stdin);
    let mut ops: Vec<BatchOp> = Vec::with_capacity(16);

    for (idx, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("failed to read stdin line {}", idx + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let op: BatchOp = serde_json::from_str(trimmed).map_err(|e| {
            crate::error::AtomwriteError::InvalidInput {
                reason: format!("invalid batch operation at line {}: {e}", idx + 1),
            }
        })?;
        ops.push(op);
    }

    if ops.is_empty() {
        bail!("empty batch manifest: no operations provided");
    }

    // In transaction mode, snapshot all existing files before any mutation.
    let backups: Vec<(PathBuf, PathBuf)> = if transaction && !dry_run {
        let paths = collect_target_paths(&ops, &workspace);
        let mut pairs = Vec::with_capacity(paths.len());
        for path in paths {
            let backup = crate::atomic::create_backup(&path, 5)
                .with_context(|| format!("transaction pre-backup failed for {}", path.display()))?;
            pairs.push((path, backup));
        }
        pairs
    } else {
        Vec::new()
    };

    let mut succeeded: u64 = 0;
    let mut failed: u64 = 0;

    for (idx, op) in ops.iter().enumerate() {
        let op_start = Instant::now();
        let result = execute_op(op, idx, &workspace, global, dry_run);

        match result {
            Ok(details) => {
                succeeded += 1;
                let event = BatchOpResult {
                    r#type: "batch_op",
                    index: idx as u64,
                    op: &op.op,
                    status: "ok",
                    details: Some(details),
                    error: None,
                    elapsed_ms: op_start.elapsed().as_millis() as u64,
                };
                writer.write_event(&event)?;
            }
            Err(e) => {
                failed += 1;
                let event = BatchOpResult {
                    r#type: "batch_op",
                    index: idx as u64,
                    op: &op.op,
                    status: "failed",
                    details: None,
                    error: Some(format!("{e:#}")),
                    elapsed_ms: op_start.elapsed().as_millis() as u64,
                };
                writer.write_event(&event)?;

                if transaction {
                    match rollback_transaction(&backups, &workspace) {
                        Ok(reverted) => {
                            let rollback_event = RollbackEvent {
                                r#type: "rollback",
                                operations_reverted: reverted,
                            };
                            writer.write_event(&rollback_event)?;
                            bail!(
                                "transaction rolled back after failure at operation {idx}: {e:#}"
                            );
                        }
                        Err(rb_err) => {
                            tracing::error!("rollback failed: {rb_err:#}");
                            std::process::exit(crate::constants::EXIT_TRANSACTION_ROLLBACK_FAILED);
                        }
                    }
                }
            }
        }
    }

    let committed = if transaction { Some(failed == 0) } else { None };
    let summary = BatchSummary {
        r#type: "summary",
        operations: ops.len() as u64,
        succeeded,
        failed,
        dry_run,
        elapsed_ms: start.elapsed().as_millis() as u64,
        transaction: if transaction { Some(true) } else { None },
        committed,
    };
    writer.write_event(&summary)?;

    if failed > 0 {
        bail!("{failed} batch operation(s) failed");
    }

    Ok(())
}

/// Collect validated paths of existing files that operations will mutate.
fn collect_target_paths(ops: &[BatchOp], workspace: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for op in ops {
        for candidate in [op.target.as_ref(), op.path.as_ref()].iter().flatten() {
            let p = std::path::Path::new(candidate.as_str());
            if let Ok(validated) = crate::path_safety::validate_path(p, workspace) {
                if validated.is_file() {
                    paths.push(validated);
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Restore all pre-transaction backups to their original paths.
fn rollback_transaction(
    backups: &[(PathBuf, PathBuf)],
    workspace: &std::path::Path,
) -> Result<u64> {
    let mut reverted = 0u64;
    for (original, backup) in backups {
        if backup.exists() {
            let content = std::fs::read(backup)
                .with_context(|| format!("cannot read backup {}", backup.display()))?;
            let opts = AtomicWriteOptions::default();
            atomic_write(original, &content, &opts, workspace)
                .with_context(|| format!("cannot restore {}", original.display()))?;
            reverted += 1;
        }
    }
    Ok(reverted)
}

fn execute_op(
    op: &BatchOp,
    _idx: usize,
    workspace: &std::path::Path,
    _global: &GlobalArgs,
    dry_run: bool,
) -> Result<String> {
    match op.op.as_str() {
        "write" => execute_write(op, workspace, dry_run),
        "replace" => execute_replace(op, workspace, dry_run),
        "delete" => execute_delete(op, workspace, dry_run),
        "edit" => execute_edit(op, workspace, dry_run),
        "hash" => execute_hash(op, workspace),
        "move" => execute_move(op, workspace, dry_run),
        "copy" => execute_copy(op, workspace, dry_run),
        _ => bail!("unsupported batch operation: {}", op.op),
    }
}

fn execute_write(op: &BatchOp, workspace: &std::path::Path, dry_run: bool) -> Result<String> {
    let target = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("write operation requires 'target' field"))?;
    let content = op
        .content
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("write operation requires 'content' field"))?;

    let target_path = std::path::Path::new(target);

    if dry_run {
        return Ok(format!("would write {} bytes to {target}", content.len()));
    }

    let opts = AtomicWriteOptions {
        backup: op.backup,
        ..Default::default()
    };
    let result = atomic_write(target_path, content.as_bytes(), &opts, workspace)?;
    Ok(format!(
        "wrote {} bytes, checksum={}",
        result.bytes_written, result.checksum
    ))
}

fn execute_replace(op: &BatchOp, workspace: &std::path::Path, dry_run: bool) -> Result<String> {
    let path_str =
        op.path.as_deref().or(op.target.as_deref()).ok_or_else(|| {
            anyhow::anyhow!("replace operation requires 'path' or 'target' field")
        })?;
    let pattern =
        op.pattern.as_deref().or(op.old.as_deref()).ok_or_else(|| {
            anyhow::anyhow!("replace operation requires 'pattern' or 'old' field")
        })?;
    let replacement = op
        .replacement
        .as_deref()
        .or(op.new.as_deref())
        .ok_or_else(|| {
            anyhow::anyhow!("replace operation requires 'replacement' or 'new' field")
        })?;

    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    let content = std::fs::read_to_string(&validated)
        .with_context(|| format!("cannot read {}", validated.display()))?;

    let new_content = content.replace(pattern, replacement);
    if new_content == content {
        return Ok(format!("no matches in {path_str}"));
    }

    let count = content.matches(pattern).count();

    if dry_run {
        return Ok(format!("would replace {count} occurrence(s) in {path_str}"));
    }

    let checksum_before = checksum::hash_bytes(content.as_bytes());
    let opts = AtomicWriteOptions {
        backup: op.backup,
        ..Default::default()
    };
    let result = atomic_write(&validated, new_content.as_bytes(), &opts, workspace)?;
    Ok(format!(
        "replaced {count} occurrence(s), checksum_before={checksum_before}, checksum_after={}",
        result.checksum
    ))
}

fn execute_delete(op: &BatchOp, workspace: &std::path::Path, dry_run: bool) -> Result<String> {
    let target = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("delete operation requires 'target' field"))?;

    let path = std::path::Path::new(target);
    let validated = crate::path_safety::validate_path(path, workspace)?;

    if !validated.exists() {
        return Err(crate::error::AtomwriteError::NotFound {
            path: validated.clone(),
        }
        .into());
    }

    if dry_run {
        return Ok(format!("would delete {target}"));
    }

    if op.backup {
        crate::atomic::create_backup(&validated, 5)
            .with_context(|| format!("cannot backup {target}"))?;
    }

    let checksum = checksum::hash_file(&validated)?;
    std::fs::remove_file(&validated).with_context(|| format!("cannot delete {target}"))?;

    if let Some(parent) = validated.parent() {
        if let Err(e) = crate::platform::fsync_dir(parent) {
            tracing::warn!(
                "fsync_dir after batch delete failed for {}: {e}",
                parent.display()
            );
        }
    }

    Ok(format!("deleted {target}, checksum_before={checksum}"))
}

fn execute_edit(op: &BatchOp, workspace: &std::path::Path, dry_run: bool) -> Result<String> {
    let path_str = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("edit operation requires 'target' field"))?;
    let old = op
        .old
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("edit operation requires 'old' field"))?;
    let new = op.new.as_deref().unwrap_or("");

    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    let content = std::fs::read_to_string(&validated)
        .with_context(|| format!("cannot read {}", validated.display()))?;

    if !content.contains(old) {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("old string not found in {path_str}"),
        }
        .into());
    }

    if dry_run {
        return Ok(format!("would edit {path_str}"));
    }

    let edited = content.replacen(old, new, 1);
    let checksum_before = checksum::hash_bytes(content.as_bytes());
    let opts = AtomicWriteOptions {
        backup: op.backup,
        ..Default::default()
    };
    let result = atomic_write(&validated, edited.as_bytes(), &opts, workspace)?;
    Ok(format!(
        "edited {path_str}, checksum_before={checksum_before}, checksum_after={}",
        result.checksum
    ))
}

fn execute_hash(op: &BatchOp, workspace: &std::path::Path) -> Result<String> {
    let path_str = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("hash operation requires 'target' field"))?;
    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    let hash = checksum::hash_file(&validated)?;
    Ok(format!("hash={hash}"))
}

fn execute_move(op: &BatchOp, workspace: &std::path::Path, dry_run: bool) -> Result<String> {
    let source_str = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("move operation requires 'target' (source path)"))?;
    let dest_str = op
        .path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("move operation requires 'path' (destination path)"))?;

    let source = crate::path_safety::validate_path(std::path::Path::new(source_str), workspace)?;
    let dest = crate::path_safety::validate_path(std::path::Path::new(dest_str), workspace)?;

    if !source.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: source }.into());
    }

    if dry_run {
        return Ok(format!("would move {source_str} to {dest_str}"));
    }

    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create parent dir for {dest_str}"))?;
        }
    }
    std::fs::rename(&source, &dest)
        .with_context(|| format!("cannot move {source_str} to {dest_str}"))?;
    if let Some(parent) = dest.parent() {
        let _ = crate::platform::fsync_dir(parent);
    }
    if let Some(parent) = source.parent() {
        let _ = crate::platform::fsync_dir(parent);
    }
    Ok(format!("moved {source_str} to {dest_str}"))
}

fn execute_copy(op: &BatchOp, workspace: &std::path::Path, dry_run: bool) -> Result<String> {
    let source_str = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("copy operation requires 'target' (source path)"))?;
    let dest_str = op
        .path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("copy operation requires 'path' (destination path)"))?;

    let source = crate::path_safety::validate_path(std::path::Path::new(source_str), workspace)?;
    let dest = crate::path_safety::validate_path(std::path::Path::new(dest_str), workspace)?;

    if !source.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: source }.into());
    }

    if dry_run {
        return Ok(format!("would copy {source_str} to {dest_str}"));
    }

    let content =
        std::fs::read(&source).with_context(|| format!("cannot read {}", source.display()))?;
    let opts = AtomicWriteOptions {
        backup: op.backup,
        ..Default::default()
    };
    let result = atomic_write(&dest, &content, &opts, workspace)?;
    Ok(format!(
        "copied {source_str} to {dest_str}, checksum={}",
        result.checksum
    ))
}
