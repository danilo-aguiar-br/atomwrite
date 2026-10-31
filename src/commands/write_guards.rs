// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared size guards of every verb that replaces file content.
//!
// `write` and `apply` both replace the whole content of a target, so both must
// apply the same two barriers: a large-file overwrite is default-deny, and a
// shrink above the configured percent is default-deny. Keeping the policy in
// one place is what stops a new verb from shipping without it.

use std::path::Path;

use crate::config::WriteSection;
use crate::constants::PERCENT_SCALE;
use crate::error::AtomwriteError;
use crate::ndjson_types::WriteRiskAssessment;

/// Classify a payload that is about to become the content of a file.
///
/// Patterns come from XDG `[write].content_risk_patterns` when the owner set
/// them, and from [`crate::constants::WRITE_CONTENT_RISK_PATTERNS`] otherwise.
/// The check is a local diagnostic, never telemetry: it is what lets an agent
/// see that the bytes it just wrote look like a destructive payload.
///
/// Returns `None` when nothing matches, and the caller then omits the whole
/// `risk_assessment` field instead of publishing an empty object.
pub fn assess_content_risk(
    content: &[u8],
    original_bytes: u64,
    new_bytes: u64,
    write_cfg: &WriteSection,
) -> Option<WriteRiskAssessment> {
    let text = std::str::from_utf8(content).ok()?;
    // The delta is a fact about the two sizes, not about the pattern: a caller
    // that published 0 while the file grew by 45% would be stating a number it
    // never measured, and `write` already derives it the same way.
    let size_delta_pct = if original_bytes > 0 {
        (new_bytes.abs_diff(original_bytes) * PERCENT_SCALE / original_bytes) as u32
    } else {
        0
    };
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
    // `curl` piped into a shell is destructive even when no configured pattern
    // names it, and the two tokens never appear adjacent in the constant list.
    let piped_to_shell = text.contains("curl")
        && (text.contains("| sh") || text.contains("|sh") || text.contains("| bash"));
    if matched || piped_to_shell {
        return Some(WriteRiskAssessment {
            original_bytes,
            new_bytes,
            size_delta_pct,
            risk_level: "high",
            guard_triggered: "content_pattern",
        });
    }
    None
}

/// Reject a content replacement that the caller has not explicitly accepted.
///
/// Two barriers apply, both derived from XDG `[write]` and never from a literal
/// in the code: the target above `confirm_large_bytes` needs
/// `--ack-overwrite`, and a result smaller than the retained percent needs
/// `--allow-shrink`.
///
/// `original_size` is the length of the content already read into memory, not
/// a second `stat`, so the check cannot drift from what is about to be
/// replaced and costs no extra syscall inside the update lock.
///
/// # Errors
///
/// Returns [`AtomwriteError::InvalidInput`] (exit 65) when the large-file
/// barrier is hit without `--ack-overwrite`, or when the shrink barrier is hit
/// without `--allow-shrink`.
pub fn check_replacement_size(
    target: &Path,
    original_size: u64,
    new_size: u64,
    write_cfg: &WriteSection,
    ack_overwrite: bool,
    allow_shrink: bool,
) -> Result<(), AtomwriteError> {
    if original_size > write_cfg.confirm_large_bytes && !ack_overwrite {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "overwrite of large file {} ({} bytes) requires --ack-overwrite \
                 (threshold XDG [write].confirm_large_bytes={}; one-shot; no interactive prompt)",
                target.display(),
                original_size,
                write_cfg.confirm_large_bytes
            ),
        });
    }

    if !allow_shrink && original_size > 0 {
        let remain_pct = new_size.saturating_mul(PERCENT_SCALE) / original_size;
        let min_remain = PERCENT_SCALE.saturating_sub(u64::from(write_cfg.shrink_block_percent));
        if remain_pct < min_remain {
            let shrink_pct = PERCENT_SCALE
                .saturating_sub(new_size.saturating_mul(PERCENT_SCALE) / original_size);
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "content is {}% smaller than target ({} -> {} bytes); \
                     pass --allow-shrink to confirm intentional truncation \
                     (threshold XDG [write].shrink_block_percent={})",
                    shrink_pct, original_size, new_size, write_cfg.shrink_block_percent
                ),
            });
        }
    }

    Ok(())
}
