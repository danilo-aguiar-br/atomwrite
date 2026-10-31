// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sliding-window and block-anchor fuzzy locate (CPU-bound).

use rayon::prelude::*;

use super::util::{CancelPoll, check_cancel};
use crate::error::AtomwriteError;

/// Block-anchor locate: first/last line anchors + `TextDiff` ratio.
///
/// GAP-2026-023: when the pattern has a middle (len > 2), also require the
/// middle-only ratio to meet `min_ratio` (dual-gate) so radical mid-block
/// drift cannot ride on matching border lines alone.
///
/// # Errors
///
/// Returns [`AtomwriteError::Cancelled`] when the global shutdown flag is set.
pub(crate) fn match_block_anchor(
    content: &[&str],
    pattern: &[&str],
    min_ratio: f64,
) -> Result<Option<(usize, usize, f64)>, AtomwriteError> {
    if pattern.len() < 2 {
        return Ok(None);
    }
    let (Some(first), Some(last)) = (pattern.first(), pattern.last()) else {
        return Ok(None);
    };
    let (first, last) = (first.trim(), last.trim());
    let plen = pattern.len();
    let pat = pattern.join("\n");
    let mut candidates: Vec<(usize, usize, f64)> =
        Vec::with_capacity(crate::constants::FUZZY_WINDOW_CANDIDATES_CAPACITY);
    let mut line_poll = CancelPoll::lines();
    let mut diff_poll = CancelPoll::heavy();

    for (i, line) in content.iter().enumerate() {
        line_poll.tick()?;
        if line.trim() != first {
            continue;
        }
        let search_end = (i + plen * 2).min(content.len());
        for j in (i + 1)..search_end {
            if content[j].trim() != last {
                continue;
            }
            diff_poll.tick()?;
            let block = content[i..=j].join("\n");
            let diff = similar::TextDiff::from_lines(&pat, &block);
            let raw_ratio = diff.ratio() as f64;
            let ratio = if raw_ratio.is_finite() {
                raw_ratio
            } else {
                0.0
            };
            if ratio < min_ratio {
                continue;
            }
            // Dual-gate middle: border anchors alone are insufficient for FP-prone blocks.
            if pattern.len() > 2 && j > i + 1 {
                let mid_pat = pattern[1..pattern.len() - 1].join("\n");
                let mid_block = content[i + 1..j].join("\n");
                let mid_diff = similar::TextDiff::from_lines(&mid_pat, &mid_block);
                let mid_ratio = mid_diff.ratio() as f64;
                let mid_ratio = if mid_ratio.is_finite() {
                    mid_ratio
                } else {
                    0.0
                };
                if mid_ratio < min_ratio {
                    continue;
                }
            }
            candidates.push((i, j + 1, ratio));
        }
    }

    if candidates.len() == 1 {
        return Ok(Some(candidates[0]));
    }
    candidates.retain(|c| c.2 >= crate::constants::FUZZY_THRESHOLD_AUTO);
    candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    Ok(candidates.first().copied())
}

/// Context-aware Damerau windows with rayon bound fan-out (G-FZZ-010/078).
///
/// `soft_floor_delta` comes from XDG `[fuzzy].soft_floor_delta` (GAP-2026-022).
///
/// # Errors
///
/// Returns [`AtomwriteError::Cancelled`] when the global shutdown flag is set
/// before or while the windows are scored; a cancelled scan never reports a miss.
pub(crate) fn match_context_aware(
    content: &[&str],
    pattern: &[&str],
    threshold: f64,
    soft_floor_delta: f64,
) -> Result<Option<(usize, usize, f64)>, AtomwriteError> {
    if pattern.is_empty() || pattern.len() > content.len() {
        return Ok(None);
    }
    let pat_joined = pattern.join("\n");
    if pat_joined.chars().count() > crate::constants::FUZZY_MAX_LEVENSHTEIN_CHARS {
        return Ok(None);
    }
    let plen = pattern.len();
    let max_windows = crate::constants::FUZZY_MAX_WINDOWS;
    let last = content.len() - plen;
    let indices: Vec<usize> = (0..=last).take(max_windows).collect();
    check_cancel()?;
    let best = indices
        .par_iter()
        .filter_map(|&i| {
            if crate::signal::is_global_shutdown() {
                return None;
            }
            let window_joined = content[i..i + plen].join("\n");
            if window_joined.chars().count() > crate::constants::FUZZY_MAX_LEVENSHTEIN_CHARS {
                return None;
            }
            let sim = strsim::normalized_damerau_levenshtein(&pat_joined, &window_joined);
            Some((i, i + plen, sim))
        })
        .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    // Workers stop early on shutdown and return a partial max: surface it as a cancel.
    check_cancel()?;
    if let Some((i, j, sim)) = best {
        let floor = soft_floor_delta.max(0.0);
        if sim >= threshold || sim >= threshold - floor {
            return Ok(Some((i, j, sim)));
        }
    }
    Ok(None)
}
