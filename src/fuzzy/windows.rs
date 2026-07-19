// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sliding-window and block-anchor fuzzy locate (CPU-bound).

use rayon::prelude::*;

/// Block-anchor locate: first/last line anchors + `TextDiff` ratio.
pub(crate) fn match_block_anchor(
    content: &[&str],
    pattern: &[&str],
    min_ratio: f64,
) -> Option<(usize, usize, f64)> {
    if pattern.len() < 2 {
        return None;
    }
    let first = pattern.first()?.trim();
    let last = pattern.last()?.trim();
    let plen = pattern.len();
    let mut candidates: Vec<(usize, usize, f64)> = Vec::with_capacity(4);

    for (i, line) in content.iter().enumerate() {
        if line.trim() != first {
            continue;
        }
        let search_end = (i + plen * 2).min(content.len());
        for j in (i + 1)..search_end {
            if content[j].trim() != last {
                continue;
            }
            let block = content[i..=j].join("\n");
            let pat = pattern.join("\n");
            let diff = similar::TextDiff::from_lines(&pat, &block);
            let raw_ratio = diff.ratio() as f64;
            let ratio = if raw_ratio.is_finite() {
                raw_ratio
            } else {
                0.0
            };
            if ratio >= min_ratio {
                candidates.push((i, j + 1, ratio));
            }
        }
    }

    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    candidates.retain(|c| c.2 >= crate::constants::FUZZY_THRESHOLD_AUTO);
    candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    candidates.first().copied()
}

/// Context-aware Damerau windows with rayon bound fan-out (G-FZZ-010/078).
pub(crate) fn match_context_aware(
    content: &[&str],
    pattern: &[&str],
    threshold: f64,
) -> Option<(usize, usize, f64)> {
    if pattern.is_empty() || pattern.len() > content.len() {
        return None;
    }
    let pat_joined = pattern.join("\n");
    if pat_joined.chars().count() > crate::constants::FUZZY_MAX_LEVENSHTEIN_CHARS {
        return None;
    }
    let plen = pattern.len();
    let max_windows = crate::constants::FUZZY_MAX_WINDOWS;
    let last = content.len() - plen;
    let indices: Vec<usize> = (0..=last).take(max_windows).collect();
    if crate::signal::is_global_shutdown() {
        return None;
    }
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
    if let Some((i, j, sim)) = best {
        if sim >= threshold
            || sim >= threshold - crate::constants::FUZZY_CONTEXT_SOFT_FLOOR_DELTA
        {
            return Some((i, j, sim));
        }
    }
    None
}
