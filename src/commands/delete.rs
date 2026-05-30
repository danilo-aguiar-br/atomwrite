// SPDX-License-Identifier: MIT OR Apache-2.0

//! File deletion with optional backup before removal.

use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::checksum;
use crate::cli::{DeleteArgs, GlobalArgs};
use crate::error::AtomwriteError;
use crate::ndjson_types::Summary;
use crate::output::NdjsonWriter;
use crate::platform;

/// Delete files with optional backup and dry-run support.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the target file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if deleting the file fails.
pub fn cmd_delete(
    args: &DeleteArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let mut deleted = 0u64;
    let mut _bytes_freed = 0u64;

    for path in &args.paths {
        let path = crate::path_safety::validate_path(path, &workspace)?;

        if !path.exists() {
            return Err(AtomwriteError::NotFound { path }.into());
        }

        if path.is_dir() && !args.recursive {
            return Err(AtomwriteError::InvalidInput {
                reason: format!("{} is a directory, use --recursive", path.display()),
            }
            .into());
        }

        if path.is_file() {
            let meta = std::fs::metadata(&path)
                .with_context(|| format!("cannot stat {}", path.display()))?;
            let hash = checksum::hash_file(&path)?;
            let size = meta.len();

            if args.dry_run {
                writer.write_event(&serde_json::json!({
                    "type": "plan",
                    "operation": "delete",
                    "path": path.display().to_string(),
                    "would_modify": true,
                    "details": format!("{} bytes", size),
                }))?;
                continue;
            }

            if args.backup {
                crate::atomic::create_backup(&path, args.retention)?;
            }

            std::fs::remove_file(&path)
                .with_context(|| format!("cannot delete {}", path.display()))?;

            if let Some(parent) = path.parent() {
                if let Err(e) = platform::fsync_dir(parent) {
                    tracing::warn!(
                        "fsync_dir after delete failed for {}: {e}",
                        parent.display()
                    );
                }
            }

            deleted += 1;
            _bytes_freed += size;

            writer.write_event(&serde_json::json!({
                "type": "deleted",
                "path": path.display().to_string(),
                "bytes": size,
                "checksum_before": hash,
                "elapsed_ms": start.elapsed().as_millis() as u64,
            }))?;
        }
    }

    writer.write_event(&Summary {
        r#type: "summary",
        files_visited: args.paths.len() as u64,
        files_matched: deleted,
        files_modified: Some(deleted),
        files_skipped: Some(args.paths.len() as u64 - deleted),
        total_matches: None,
        total_replacements: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}
