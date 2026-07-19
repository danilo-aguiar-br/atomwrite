// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by write.rs (SRP split — A-MONO-001).

/// B-013 / R-XDG-013: classify destructive shell-like payloads (local diagnostics, not telemetry).
///
/// Patterns come from XDG / `.atomwrite.toml` `[write].content_risk_patterns` when set;
/// otherwise the built-in defaults in [`crate::constants::WRITE_CONTENT_RISK_PATTERNS`].
fn assess_content_risk(
    content: &[u8],
    original_bytes: u64,
    new_bytes: u64,
    write_cfg: &crate::config::WriteSection,
) -> Option<crate::ndjson_types::WriteRiskAssessment> {
    let text = std::str::from_utf8(content).ok()?;
    let matched = if write_cfg.content_risk_patterns.is_empty() {
        crate::constants::WRITE_CONTENT_RISK_PATTERNS
            .iter()
            .any(|pat| text.contains(pat))
    } else {
        write_cfg
            .content_risk_patterns
            .iter()
            .any(|pat| !pat.is_empty() && text.contains(pat.as_str()))
    };
    if matched {
        return Some(crate::ndjson_types::WriteRiskAssessment {
            original_bytes,
            new_bytes,
            size_delta_pct: 0,
            risk_level: "high",
            guard_triggered: "content_pattern",
        });
    }
    // curl|sh style without requiring both tokens adjacent in the constant list.
    if text.contains("curl")
        && (text.contains("| sh") || text.contains("|sh") || text.contains("| bash"))
    {
        return Some(crate::ndjson_types::WriteRiskAssessment {
            original_bytes,
            new_bytes,
            size_delta_pct: 0,
            risk_level: "high",
            guard_triggered: "content_pattern",
        });
    }
    None
}

/// Read all bytes from stdin, applying optional `max_size` cap and the G120
/// L1 guard for empty input. Returns the buffer plus the actual byte count
/// read so the caller can include `stdin_bytes_read` in the NDJSON envelope
/// (G120 L4 local diagnostics).
///
/// The empty-stdin guard defaults to ON because accepting 0 bytes as a valid
/// payload is a frequent source of silent data loss when the upstream
/// command in a pipe produces no output (`cat missing.txt`, a heredoc that
/// expands to nothing, a failing `find`, etc.). Callers that genuinely
/// intend to write zero bytes (e.g. truncating a file to empty) must pass
/// `--allow-empty-stdin` to make the intent explicit.
/// Read stdin in 64 KiB chunks, optionally honouring cooperative cancel (v0.1.29 P0-4).
pub(crate) fn read_stdin_content_cancellable(
    stdin: impl Read,
    max_size: Option<u64>,
    allow_empty: bool,
    shutdown: Option<&crate::signal::ShutdownSignal>,
) -> Result<(Vec<u8>, u64)> {
    use std::io::Read as _;
    let mut reader = BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
    let mut buf = Vec::with_capacity(crate::constants::STDIN_INITIAL_CAPACITY);
    if let Some(max) = max_size {
        let reserve = usize::try_from(max.min(u64::from(u32::MAX))).unwrap_or(usize::MAX);
        // Soft pre-reserve up to the cap so growth is one-shot when the
        // payload is near max_size; failure is non-fatal (we still grow).
        let _ = buf.try_reserve(reserve.min(crate::constants::STDIN_INITIAL_CAPACITY * 64));
    }
    // A-025: stdin read chunk aligned with BUF_CAPACITY.
    let mut chunk = [0u8; crate::constants::BUF_CAPACITY];
    loop {
        if shutdown.is_some_and(|s| s.is_shutdown()) {
            return Err(crate::signal::cancelled_error("stdin read cancelled by signal").into());
        }
        let n = reader.read(&mut chunk).context("failed to read stdin")?;
        if n == 0 {
            break;
        }
        buf.try_reserve(n).map_err(|e| AtomwriteError::InternalError {
            reason: format!("allocation failed while reading stdin (+{n} bytes): {e}"),
        })?;
        buf.extend_from_slice(&chunk[..n]);
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
    }
    let n = buf.len();

    if !allow_empty && n == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "stdin produced 0 bytes; pass --allow-empty-stdin to confirm an empty write is intentional".into(),
        }
        .into());
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
                // Only the detect window is needed — never load the whole file.
                match read_line_ending_sample(target) {
                    Ok(sample) => line_endings::detect(&sample),
                    Err(_) => return content.to_vec(),
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

/// Read at most `LINE_ENDING_DETECT_SIZE` bytes for Auto line-ending detect.
fn read_line_ending_sample(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; crate::constants::LINE_ENDING_DETECT_SIZE];
    let n = file.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
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
    /// `write_creates_file_with_ndjson_output` failed on Windows redirected-stderr environments
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
