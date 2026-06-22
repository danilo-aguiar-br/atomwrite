// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file creation and overwrite from stdin content.
//! Workload: I/O-bound (stdin read + atomic write).

use std::io::{BufReader, Read, Write};
use std::time::Instant;

use anyhow::{Context, Result, bail};

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{GlobalArgs, WriteArgs};
use crate::commands::resolve_backup;
use crate::error::AtomwriteError;
use crate::ndjson_types::WriteOutput;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Create or overwrite a file atomically from stdin content.
///
/// The target is resolved against the workspace once, up front, so
/// append/prepend, line-ending auto-detection, and `--expect-checksum`
/// operate on the same path identity as the final atomic write. Before
/// v0.1.15 these pre-steps used the raw CLI path relative to the CWD,
/// which truncated appends and skipped checksum verification whenever
/// the CWD differed from the workspace (G118, CWE-367).
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
    let resolved = crate::path_safety::validate_path(&args.target, &workspace)?;
    let effective_backup = resolve_backup(args.backup, args.no_backup);

    let stdin_bytes_read;
    let mut content = {
        let (buf, n) = read_stdin_content(stdin, args.max_size, args.allow_empty_stdin)?;
        stdin_bytes_read = n;
        buf
    };

    if shutdown.is_shutdown() {
        bail!("interrupted before write");
    }

    if args.append || args.prepend {
        content = handle_append_prepend(
            &resolved,
            &content,
            args.append,
            global.effective_max_filesize(),
            args.allow_empty_stdin,
        )?;
    }

    content = normalize_line_endings(&content, args.line_ending, &resolved);

    // G120 L3: cross-validation. When the caller combines
    // `--append` (or `--prepend`) with `--expect-checksum` and the stdin
    // is empty, the situation is ambiguous: the checksum is for the
    // pre-mutation state but the empty append is effectively a no-op.
    // We emit a structured warning on stderr so the agent and operator
    // can audit the decision, but DO NOT abort — the user explicitly
    // opted into empty stdin with `--allow-empty-stdin`.
    if let Some(ref expected) = args.expect_checksum {
        if stdin_bytes_read == 0 && (args.append || args.prepend) {
            if args.no_checksum_when_empty {
                tracing::warn!(
                    path = %resolved.display(),
                    expected = %expected,
                    "G120 L3: --append/--prepend + --expect-checksum with empty stdin; \
                     skipping checksum verification per --no-checksum-when-empty"
                );
            } else {
                // Default: still verify against the pre-mutation state.
                // If the pre-mutation file exists and matches, this
                // passes and the empty append is a no-op. If it does
                // not exist, verify_checksum's `if !target.exists()`
                // short-circuit returns Ok — the same legacy behaviour
                // as pre-v0.1.15, but now explicitly logged.
                tracing::info!(
                    path = %resolved.display(),
                    expected = %expected,
                    "G120 L3: --append/--prepend + --expect-checksum with empty stdin; \
                     verifying pre-mutation state. Pass --no-checksum-when-empty to skip."
                );
                verify_checksum(&resolved, expected, global.effective_max_filesize())?;
            }
        } else {
            verify_checksum(&resolved, expected, global.effective_max_filesize())?;
        }
    }

    // GAP-2026-017: block writes that shrink >50% when --expect-checksum is active
    if args.expect_checksum.is_some() && !args.allow_shrink && resolved.exists() {
        let original_size = std::fs::metadata(&resolved).map(|m| m.len()).unwrap_or(0);
        let new_size = content.len() as u64;
        if original_size > 0 && new_size < original_size / 2 {
            let shrink_pct = 100u64.saturating_sub(new_size.saturating_mul(100) / original_size);
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "stdin is {}% smaller than target ({} -> {} bytes); \
                     pass --allow-shrink to confirm intentional truncation",
                    shrink_pct, original_size, new_size
                ),
            }
            .into());
        }
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

    // GAP-2026-011 L2 — require-backup guard
    // GAP-2026-033: check no_backup explicitly since backup has default_value_t=true
    if args.require_backup && (args.no_backup || !effective_backup) && resolved.exists() {
        return Err(AtomwriteError::InvalidInput {
            reason: "--require-backup is set but --no-backup disables backup; remove --no-backup or remove --require-backup".into(),
        }
        .into());
    }

    // GAP-2026-011 L3 — confirm guard
    if args.confirm && resolved.exists() {
        let size = std::fs::metadata(&resolved).map(|m| m.len()).unwrap_or(0);
        if size > 100 * 1024 {
            use std::io::BufRead;
            eprint!("Overwrite {} ({} bytes)? [y/N] ", resolved.display(), size);
            let stdin_lock = std::io::stdin();
            let mut handle = stdin_lock.lock();
            let mut input = String::new();
            let _ = handle.read_line(&mut input);
            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                return Err(AtomwriteError::InvalidInput {
                    reason: "aborted by user (confirm=no)".into(),
                }
                .into());
            }
        }
    }

    // GAP-2026-011 L5 — auto-rotate guard
    let auto_rotate_active = args.auto_rotate
        && effective_backup
        && resolved.exists()
        && std::fs::metadata(&resolved)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age < std::time::Duration::from_secs(24 * 3600));

    // GAP-2026-011 L1 + L6 — size guard and risk_assessment telemetry
    // GAP-2026-024: skip risk_assessment for append/prepend (never causes data loss)
    let risk_assessment = if resolved.exists() && !args.append && !args.prepend {
        let original = std::fs::metadata(&resolved).map(|m| m.len()).unwrap_or(0);
        let new_bytes = content.len() as u64;
        if original > 0 {
            let delta_pct = ((new_bytes.abs_diff(original)) * 100 / original) as u8;
            if delta_pct >= args.risk_threshold {
                let level = if delta_pct >= 90 {
                    "high"
                } else if delta_pct >= 70 {
                    "medium"
                } else {
                    "low"
                };
                eprintln!(
                    "\x1b[33mwarning:\x1b[0m write risk: {} ({}% delta, {} -> {} bytes)",
                    level, delta_pct, original, new_bytes
                );
                // GAP-2026-017: block shrink when --expect-checksum is active
                if args.expect_checksum.is_some() && !args.allow_shrink && new_bytes < original {
                    return Err(AtomwriteError::InvalidInput {
                        reason: format!(
                            "write risk {} ({}% size delta, {} -> {} bytes) blocked with --expect-checksum; \
                             pass --allow-shrink to override",
                            level, delta_pct, original, new_bytes
                        ),
                    }
                    .into());
                }
                Some(crate::ndjson_types::WriteRiskAssessment {
                    original_bytes: original,
                    new_bytes,
                    size_delta_pct: delta_pct,
                    risk_level: level,
                    guard_triggered: "size",
                })
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let opts = AtomicWriteOptions {
        backup: effective_backup || auto_rotate_active,
        syntax_check: args.syntax_check,
        retention: args.retention,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: args.wal_policy,
        keep_backup: args.keep_backup,
    };

    let result = atomic_write(&resolved, &content, &opts, &workspace)?;

    let output = WriteOutput {
        r#type: "write",
        status: "success",
        path: target_str,
        bytes_written: result.bytes_written,
        checksum: result.checksum,
        checksum_before: result.checksum_before,
        backup_path: result.backup_path,
        elapsed_ms: start.elapsed().as_millis() as u64,
        stdin_bytes_read,
        wal_policy: args.wal_policy.as_str(),
        platform: result.platform,
        risk_assessment,
    };

    writer.write_event(&output)?;
    Ok(())
}

/// Read all bytes from stdin, applying optional `max_size` cap and the G120
/// L1 guard for empty input. Returns the buffer plus the actual byte count
/// read so the caller can include `stdin_bytes_read` in the NDJSON envelope
/// (G120 L4 telemetry).
///
/// The empty-stdin guard defaults to ON because accepting 0 bytes as a valid
/// payload is a frequent source of silent data loss when the upstream
/// command in a pipe produces no output (`cat missing.txt`, a heredoc that
/// expands to nothing, a failing `find`, etc.). Callers that genuinely
/// intend to write zero bytes (e.g. truncating a file to empty) must pass
/// `--allow-empty-stdin` to make the intent explicit.
fn read_stdin_content(
    stdin: impl Read,
    max_size: Option<u64>,
    allow_empty: bool,
) -> Result<(Vec<u8>, u64)> {
    let mut reader = BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
    let mut buf = Vec::with_capacity(crate::constants::STDIN_INITIAL_CAPACITY);
    let n = reader
        .read_to_end(&mut buf)
        .context("failed to read stdin")?;

    if !allow_empty && n == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "stdin produced 0 bytes; pass --allow-empty-stdin to confirm an empty write is intentional".into(),
        }
        .into());
    }

    if let Some(max) = max_size {
        if n as u64 > max {
            return Err(AtomwriteError::InvalidInput {
                reason: format!("stdin exceeds max size {} bytes (got {} bytes)", max, n),
            }
            .into());
        }
    }

    Ok((buf, n as u64))
}

fn handle_append_prepend(
    target: &std::path::Path,
    new_content: &[u8],
    is_append: bool,
    max_size: u64,
    allow_empty: bool,
) -> Result<Vec<u8>> {
    if !target.exists() {
        return Ok(new_content.to_vec());
    }

    let existing = crate::file_io::read_file_bytes(target, max_size)
        .with_context(|| format!("cannot read {} for append/prepend", target.display()))?;

    if new_content.is_empty() && !allow_empty {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "--{} received 0 bytes from stdin; pass --allow-empty-stdin if you want a no-op, or check why the upstream command produced no output",
                if is_append { "append" } else { "prepend" }
            ),
        }
        .into());
    }

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
                    return content.to_vec();
                }
            } else {
                return content.to_vec();
            }
        }
        other => other,
    };
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

#[cfg(test)]
mod tests {
    use super::normalize_line_endings;
    use crate::line_endings::LineEnding;
    use std::path::PathBuf;

    /// `Auto` on a non-existent target must preserve the input bytes verbatim,
    /// regardless of the host OS. This guarantees `bytes_written` round-trips
    /// across Linux, macOS, and Windows for new files (issue: v0.1.13
    /// `write_creates_file_with_ndjson_output` failed on windows-2025-vs2026
    /// because the legacy fallback returned `LineEnding::CrLf` on Windows,
    /// inflating the byte count by 1).
    #[test]
    fn auto_on_new_file_preserves_lf_input() {
        let target = PathBuf::from("does-not-exist-atomwrite-test-12345.txt");
        let input = b"hello world\n";
        let out = normalize_line_endings(input, LineEnding::Auto, &target);
        assert_eq!(
            out,
            input,
            "Auto on new file must be a no-op (got {} bytes, expected {})",
            out.len(),
            input.len()
        );
    }

    #[test]
    fn auto_on_new_file_preserves_crlf_input() {
        let target = PathBuf::from("does-not-exist-atomwrite-test-67890.txt");
        let input = b"hello world\r\n";
        let out = normalize_line_endings(input, LineEnding::Auto, &target);
        assert_eq!(
            out,
            input,
            "Auto on new file must be a no-op (got {:?}, expected {:?})",
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(input)
        );
    }
}
