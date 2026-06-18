// SPDX-License-Identifier: MIT OR Apache-2.0

//! Batch execution of multiple operations from an NDJSON manifest.
//! Workload: I/O-bound (NDJSON parse + multi-file atomic writes).

use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::GlobalArgs;
use crate::ndjson_types::{BatchOpResult, BatchSummary};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// A single operation in a batch NDJSON manifest.
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchOp {
    op: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default, alias = "from", alias = "src")]
    source: Option<String>,
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

impl BatchOp {
    fn resolve_file_path(&self) -> anyhow::Result<&str> {
        self.target
            .as_deref()
            .or(self.path.as_deref())
            .ok_or_else(|| anyhow::anyhow!("operation requires 'target' or 'path' field"))
    }
}

/// NDJSON event emitted when a transaction is rolled back.
#[derive(Debug, Serialize)]
struct RollbackEvent {
    r#type: &'static str,
    files_restored: u64,
    files_removed: u64,
    total_reverted: u64,
}

/// Emit the JSON Schema for the batch input manifest format.
pub fn emit_input_schema(writer: &mut NdjsonWriter<impl Write>) -> Result<()> {
    let schema = schemars::schema_for!(BatchOp);
    let schema_value = serde_json::to_value(&schema)?;
    writer.write_event(&schema_value)?;
    Ok(())
}

/// Execute multiple operations from an NDJSON manifest in batch mode.
///
/// When `manifest_path` is `Some`, the NDJSON manifest is read from that file
/// instead of from stdin. The path is validated against the workspace jail.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if reading stdin or writing results fails.
/// Returns `AtomwriteError::InvalidInput` if the manifest contains invalid operations.
#[tracing::instrument(skip_all, fields(command = "batch"))]
#[allow(clippy::too_many_arguments)]
pub fn cmd_batch(
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    dry_run: bool,
    transaction: bool,
    manifest_path: Option<&std::path::Path>,
    shutdown: &ShutdownSignal,
    keep_backup: bool,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    // Resolve manifest source: file (validated) or stdin
    let mut reader: Box<dyn Read> = if let Some(manifest_path) = manifest_path {
        let validated_manifest = crate::path_safety::validate_path(manifest_path, &workspace)
            .with_context(|| {
                format!(
                    "manifest path escapes workspace: {}",
                    manifest_path.display()
                )
            })?;
        if !validated_manifest.is_file() {
            return Err(crate::error::AtomwriteError::NotFound {
                path: validated_manifest.clone(),
            }
            .into());
        }
        let file = std::fs::File::open(&validated_manifest)
            .with_context(|| format!("cannot open manifest {}", validated_manifest.display()))?;
        Box::new(file)
    } else {
        Box::new(stdin)
    };

    let mut buf_reader =
        std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, &mut *reader);
    let mut ops: Vec<BatchOp> = Vec::with_capacity(16);
    let mut line_buf = String::new();
    let mut idx = 0usize;

    loop {
        let n = crate::output::read_limited_line(
            &mut buf_reader,
            &mut line_buf,
            crate::constants::MAX_NDJSON_LINE_SIZE,
        )
        .with_context(|| format!("failed to read manifest line {}", idx + 1))?;
        if n == 0 {
            break;
        }
        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            idx += 1;
            continue;
        }
        let jd = &mut serde_json::Deserializer::from_str(trimmed);
        let op: BatchOp = serde_path_to_error::deserialize(jd).map_err(|e| {
            if e.inner().classify() == serde_json::error::Category::Io {
                crate::error::AtomwriteError::Io {
                    source: std::io::Error::other(e.to_string()),
                }
            } else {
                crate::error::AtomwriteError::InvalidInput {
                    reason: format!("invalid batch operation at line {}: {e}", idx + 1),
                }
            }
        })?;
        ops.push(op);
        idx += 1;
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

    // Track files created during the transaction so they can be removed on rollback.
    let mut created_files: Vec<PathBuf> = Vec::new();

    let mut succeeded: u64 = 0;
    let mut failed: u64 = 0;

    for (idx, op) in ops.iter().enumerate() {
        if shutdown.is_shutdown() {
            tracing::info!(
                completed = idx,
                total = ops.len(),
                "batch interrupted by signal"
            );
            break;
        }

        let op_start = Instant::now();
        // Pre-snapshot existence for "write" ops so we know if we created a new file
        let was_new_file = if transaction && !dry_run && op.op == "write" {
            op.resolve_file_path()
                .ok()
                .map(std::path::Path::new)
                .and_then(|p| crate::path_safety::validate_path(p, &workspace).ok())
                .map(|p| !p.exists())
                .unwrap_or(false)
        } else {
            false
        };
        let result = execute_op(op, idx, &workspace, global, dry_run, keep_backup);

        match result {
            Ok(details) => {
                succeeded += 1;
                if transaction && !dry_run && was_new_file {
                    if let Some(target) = op
                        .resolve_file_path()
                        .ok()
                        .map(std::path::Path::new)
                        .and_then(|p| crate::path_safety::validate_path(p, &workspace).ok())
                    {
                        created_files.push(target);
                    }
                }
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
                    match rollback_transaction(&backups, &created_files, &workspace) {
                        Ok((restored, removed)) => {
                            let rollback_event = RollbackEvent {
                                r#type: "rollback",
                                files_restored: restored,
                                files_removed: removed,
                                total_reverted: restored + removed,
                            };
                            writer.write_event(&rollback_event)?;
                            bail!(
                                "transaction rolled back after failure at operation {idx}: {e:#}"
                            );
                        }
                        Err(rb_err) => {
                            tracing::error!(error = %rb_err, "rollback failed");
                            bail!("transaction rollback failed: {rb_err:#}");
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
    let mut paths = Vec::with_capacity(ops.len());
    for op in ops {
        for candidate in [op.target.as_ref(), op.path.as_ref(), op.source.as_ref()]
            .iter()
            .flatten()
        {
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

/// Restore all pre-transaction backups to their original paths and remove
/// any files that were created during the transaction.
///
/// Returns `(restored, removed)` where `restored` is the count of pre-existing
/// files whose content was rolled back, and `removed` is the count of new
/// files that were created and then deleted.
fn rollback_transaction(
    backups: &[(PathBuf, PathBuf)],
    created_files: &[PathBuf],
    workspace: &std::path::Path,
) -> Result<(u64, u64)> {
    let mut restored = 0u64;
    for (original, backup) in backups {
        if backup.exists() {
            let content = std::fs::read(backup)
                .with_context(|| format!("cannot read backup {}", backup.display()))?;
            let opts = AtomicWriteOptions::default();
            atomic_write(original, &content, &opts, workspace)
                .with_context(|| format!("cannot restore {}", original.display()))?;
            restored += 1;
        }
    }

    let mut removed = 0u64;
    for path in created_files {
        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("cannot remove created file {}", path.display()))?;
            removed += 1;
        }
    }

    Ok((restored, removed))
}

fn execute_op(
    op: &BatchOp,
    _idx: usize,
    workspace: &std::path::Path,
    global: &GlobalArgs,
    dry_run: bool,
    keep_backup: bool,
) -> Result<String> {
    let max_size = global.effective_max_filesize();
    match op.op.as_str() {
        "write" => execute_write(op, workspace, dry_run, keep_backup),
        "replace" => execute_replace(op, workspace, dry_run, max_size, keep_backup),
        "delete" => execute_delete(op, workspace, dry_run, max_size),
        "edit" => execute_edit(op, workspace, dry_run, max_size, keep_backup),
        "hash" => execute_hash(op, workspace, max_size),
        "move" => execute_move(op, workspace, dry_run),
        "copy" => execute_copy(op, workspace, dry_run, max_size, keep_backup),
        _ => bail!("unsupported batch operation: {}", op.op),
    }
}

fn execute_write(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    keep_backup: bool,
) -> Result<String> {
    let target = op.resolve_file_path()?;
    let content = op
        .content
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("write operation requires 'content' field"))?;

    let target_path = std::path::Path::new(target);

    if dry_run {
        return Ok(format!("would write {} bytes to {target}", content.len()));
    }

    let opts = AtomicWriteOptions {
        backup: op.backup || keep_backup,
        syntax_check: false,
        retention: 5,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
    };
    let result = atomic_write(target_path, content.as_bytes(), &opts, workspace)?;
    Ok(format!(
        "wrote {} bytes, checksum={}",
        result.bytes_written, result.checksum
    ))
}

fn execute_replace(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    max_size: u64,
    keep_backup: bool,
) -> Result<String> {
    let path_str = op.resolve_file_path()?;
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
    let content = crate::file_io::read_file_string(&validated, max_size)
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
        backup: op.backup || keep_backup,
        syntax_check: false,
        retention: 5,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
    };
    let result = atomic_write(&validated, new_content.as_bytes(), &opts, workspace)?;
    Ok(format!(
        "replaced {count} occurrence(s), checksum_before={checksum_before}, checksum_after={}",
        result.checksum
    ))
}

fn execute_delete(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    max_size: u64,
) -> Result<String> {
    let target = op.resolve_file_path()?;

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

    let checksum = checksum::hash_file(&validated, max_size)?;
    std::fs::remove_file(&validated).with_context(|| format!("cannot delete {target}"))?;

    if let Some(parent) = validated.parent() {
        if let Err(e) = crate::platform::fsync_dir(parent) {
            tracing::warn!(
                path = %parent.display(),
                error = %e,
                "fsync_dir after batch delete failed"
            );
        }
    }

    Ok(format!("deleted {target}, checksum_before={checksum}"))
}

fn execute_edit(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    max_size: u64,
    keep_backup: bool,
) -> Result<String> {
    let path_str = op.resolve_file_path()?;
    let old = op
        .old
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("edit operation requires 'old' field"))?;
    let new = op.new.as_deref().unwrap_or("");

    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    let content = crate::file_io::read_file_string(&validated, max_size)
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
        backup: op.backup || keep_backup,
        syntax_check: false,
        retention: 5,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
    };
    let result = atomic_write(&validated, edited.as_bytes(), &opts, workspace)?;
    Ok(format!(
        "edited {path_str}, checksum_before={checksum_before}, checksum_after={}",
        result.checksum
    ))
}

fn execute_hash(op: &BatchOp, workspace: &std::path::Path, max_size: u64) -> Result<String> {
    let path_str = op.resolve_file_path()?;
    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    let hash = checksum::hash_file(&validated, max_size)?;
    Ok(format!("hash={hash}"))
}

fn execute_move(op: &BatchOp, workspace: &std::path::Path, dry_run: bool) -> Result<String> {
    let source_str = op
        .source
        .as_deref()
        .or(op.path.as_deref())
        .ok_or_else(|| anyhow::anyhow!("move operation requires 'source' field"))?;
    let dest_str = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("move operation requires 'target' (destination) field"))?;

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

fn execute_copy(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    max_size: u64,
    keep_backup: bool,
) -> Result<String> {
    let source_str = op
        .source
        .as_deref()
        .or(op.path.as_deref())
        .ok_or_else(|| anyhow::anyhow!("copy operation requires 'source' field"))?;
    let dest_str = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("copy operation requires 'target' (destination) field"))?;

    let source = crate::path_safety::validate_path(std::path::Path::new(source_str), workspace)?;
    let dest = crate::path_safety::validate_path(std::path::Path::new(dest_str), workspace)?;

    if !source.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: source }.into());
    }

    if dry_run {
        return Ok(format!("would copy {source_str} to {dest_str}"));
    }

    let content = crate::file_io::read_file_bytes(&source, max_size)
        .with_context(|| format!("cannot read {}", source.display()))?;
    let opts = AtomicWriteOptions {
        backup: op.backup || keep_backup,
        syntax_check: false,
        retention: 5,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
    };
    let result = atomic_write(&dest, &content, &opts, workspace)?;
    Ok(format!(
        "copied {source_str} to {dest_str}, checksum={}",
        result.checksum
    ))
}
