// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file creation and overwrite from stdin content.
//! Workload: I/O-bound (stdin read + atomic write).

use std::io::{BufReader, Read, Write};
use std::time::Instant;

use anyhow::{Context, Result, bail};

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{GlobalArgs, WriteArgs};
use crate::error::AtomwriteError;
use crate::ndjson_types::WriteOutput;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Create or overwrite a file atomically from stdin content.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if reading stdin fails.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if writing the file fails.
/// Returns `AtomwriteError::StateDrift` if `--checksum` is set and the expected hash does not match.
#[tracing::instrument(skip_all, fields(command = "write"))]
pub fn cmd_write(
    args: &WriteArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let mut content = read_stdin_content(stdin, args.max_size)?;

    if shutdown.is_shutdown() {
        bail!("interrupted before write");
    }

    if args.append || args.prepend {
        content = handle_append_prepend(
            &args.target,
            &content,
            args.append,
            global.effective_max_filesize(),
        )?;
    }

    content = normalize_line_endings(&content, args.line_ending, &args.target);

    if let Some(ref expected) = args.expect_checksum {
        verify_checksum(&args.target, expected, global.effective_max_filesize())?;
    }

    let target_str = args.target.display().to_string();

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "write".into(),
            path: target_str,
            would_modify: true,
            details: Some(format!("{} bytes from stdin", content.len())),
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let opts = AtomicWriteOptions {
        backup: args.backup,
        syntax_check: args.syntax_check,
        retention: args.retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
    };

    let result = atomic_write(&args.target, &content, &opts, &workspace)?;

    let output = WriteOutput {
        r#type: "write",
        status: "success",
        path: target_str,
        bytes_written: result.bytes_written,
        checksum: result.checksum,
        checksum_before: result.checksum_before,
        backup_path: result.backup_path,
        elapsed_ms: start.elapsed().as_millis() as u64,
        platform: result.platform,
    };

    writer.write_event(&output)?;
    Ok(())
}

fn read_stdin_content(stdin: impl Read, max_size: Option<u64>) -> Result<Vec<u8>> {
    let mut reader = BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
    let mut buf = Vec::with_capacity(crate::constants::STDIN_INITIAL_CAPACITY);
    reader
        .read_to_end(&mut buf)
        .context("failed to read stdin")?;

    if let Some(max) = max_size {
        if buf.len() as u64 > max {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "stdin exceeds max size {} bytes (got {} bytes)",
                    max,
                    buf.len()
                ),
            }
            .into());
        }
    }

    Ok(buf)
}

fn handle_append_prepend(
    target: &std::path::Path,
    new_content: &[u8],
    is_append: bool,
    max_size: u64,
) -> Result<Vec<u8>> {
    if !target.exists() {
        return Ok(new_content.to_vec());
    }

    let existing = crate::file_io::read_file_bytes(target, max_size)
        .with_context(|| format!("cannot read {} for append/prepend", target.display()))?;

    let total = existing
        .len()
        .saturating_add(new_content.len())
        .saturating_add(1);
    let mut combined = Vec::new();
    combined
        .try_reserve(total)
        .map_err(|e| crate::error::AtomwriteError::InternalError {
            reason: format!("allocation failed for {total} bytes: {e}"),
        })?;
    if is_append {
        combined.extend_from_slice(&existing);
        if !existing.ends_with(b"\n") && !existing.is_empty() {
            combined.push(b'\n');
        }
        combined.extend_from_slice(new_content);
    } else {
        combined.extend_from_slice(new_content);
        if !new_content.ends_with(b"\n") && !new_content.is_empty() {
            combined.push(b'\n');
        }
        combined.extend_from_slice(&existing);
    }

    Ok(combined)
}

fn normalize_line_endings(
    content: &[u8],
    mode: crate::line_endings::LineEnding,
    target: &std::path::Path,
) -> Vec<u8> {
    use crate::line_endings::{self, LineEnding};
    let target_ending = match mode {
        LineEnding::Auto => {
            if target.exists() {
                if let Ok(existing) = std::fs::read(target) {
                    line_endings::detect(&existing)
                } else {
                    if cfg!(windows) {
                        LineEnding::CrLf
                    } else {
                        LineEnding::Lf
                    }
                }
            } else {
                if cfg!(windows) {
                    LineEnding::CrLf
                } else {
                    LineEnding::Lf
                }
            }
        }
        other => other,
    };
    if matches!(target_ending, LineEnding::Auto) {
        return content.to_vec();
    }
    match std::str::from_utf8(content) {
        Ok(text) => line_endings::normalize(text, target_ending).into_bytes(),
        Err(_) => content.to_vec(),
    }
}

fn verify_checksum(target: &std::path::Path, expected: &str, max_size: u64) -> Result<()> {
    if !target.exists() {
        return Ok(());
    }

    let actual = checksum::hash_file(target, max_size)?;
    if actual != expected {
        return Err(AtomwriteError::StateDrift {
            path: target.to_path_buf(),
            expected: expected.to_owned(),
            actual,
        }
        .into());
    }

    Ok(())
}
