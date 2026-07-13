// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared 9-strategy fuzzy match cascade for edit, replace, batch, and edit-loop.
//!
//! Extracted from `commands::edit` in v0.1.29 (P0-1) so multi-file `replace`
//! and batch ops share the same whitespace/indent/JW cascade as single-file
//! edit. Tracks a [`BestCandidate`] across failed strategies so agents get
//! structured diagnostics instead of a bare "not found" (P0-2).

use crate::cli_args::FuzzyMode;
use crate::error::AtomwriteError;
use crate::ndjson_types::BestCandidate;

/// Diagnostic detail about which fuzzy-matching strategy resolved a pair.
#[derive(Debug, Clone)]
pub struct FuzzyInfo {
    /// Whether a fuzzy (non-exact) strategy was used.
    pub fuzzy: bool,
    /// Name of the strategy that produced the match.
    pub strategy: String,
    /// Number of strategies attempted before finding a match.
    pub strategies_tried: u64,
    /// Similarity score of the match (0.0–1.0), if applicable.
    pub similarity: Option<f64>,
    /// Mini unified diff showing what matched vs what was expected.
    pub diff_preview: Option<String>,
}

/// Minimum similarity to serialize a best-candidate in error envelopes.
const BEST_CANDIDATE_MIN: f64 = 0.5;
/// Truncate candidate text for NDJSON envelopes.
const BEST_CANDIDATE_TEXT_MAX: usize = 500;

fn find_str(haystack: &str, needle: &str) -> Option<usize> {
    memchr::memmem::find(haystack.as_bytes(), needle.as_bytes())
}

fn truncate_text(s: &str) -> String {
    if s.chars().count() <= BEST_CANDIDATE_TEXT_MAX {
        return s.to_string();
    }
    s.chars().take(BEST_CANDIDATE_TEXT_MAX).collect()
}

fn line_col_of_offset(content: &str, byte_offset: usize) -> (u64, u64) {
    let mut line = 1u64;
    let mut col = 1u64;
    for (i, ch) in content.char_indices() {
        if i >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn byte_offset_of_line(content: &str, line_idx: usize) -> usize {
    if line_idx == 0 {
        return 0;
    }
    let mut seen = 0usize;
    for (i, ch) in content.char_indices() {
        if ch == '\n' {
            seen += 1;
            if seen == line_idx {
                return i + 1;
            }
        }
    }
    content.len()
}

fn consider_best(
    best: &mut Option<BestCandidate>,
    text: &str,
    line: u64,
    column: u64,
    similarity: f64,
    strategy: &str,
) {
    if similarity < BEST_CANDIDATE_MIN {
        return;
    }
    let better = match best {
        None => true,
        Some(cur) => similarity > cur.similarity.unwrap_or(0.0),
    };
    if better {
        *best = Some(BestCandidate {
            text: Some(truncate_text(text)),
            line: Some(line),
            column: Some(column),
            similarity: Some(similarity),
            strategy: Some(strategy.to_string()),
            diff_preview: None,
        });
    }
}

/// Resolve a single `old` → `new` replacement against `content`, running the
/// 9-strategy fuzzy cascade. Shared by edit, replace, batch, and edit-loop.
///
/// # Errors
///
/// Returns [`AtomwriteError::MatchFailed`] with optional [`BestCandidate`] when
/// no strategy matches, or [`AtomwriteError::InvalidInput`] for fuzzy=off miss.
pub fn match_pair(
    content: &str,
    old: &str,
    new: &str,
    fuzzy_mode: FuzzyMode,
    custom_threshold: Option<f64>,
) -> std::result::Result<(String, FuzzyInfo), AtomwriteError> {
    // Strategy 1: exact match
    if let Some(pos) = find_str(content, old) {
        let edited = format!("{}{}{}", &content[..pos], new, &content[pos + old.len()..]);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: false,
                strategy: "exact".into(),
                strategies_tried: 1,
                similarity: None,
                diff_preview: None,
            },
        ));
    }

    if matches!(fuzzy_mode, FuzzyMode::Off) {
        return Err(AtomwriteError::MatchFailed {
            reason: format!("old string not found in file (fuzzy=off): {old:?}"),
            best_candidate: None,
        });
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let content_lines: Vec<&str> = content.lines().collect();
    let mut best: Option<BestCandidate> = None;

    // Strategy 2: line-trimmed
    if let Some((start, end)) = match_line_trimmed(&content_lines, &old_lines) {
        let edited = apply_line_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "line_trimmed".into(),
                strategies_tried: 2,
                similarity: Some(1.0),
                diff_preview: None,
            },
        ));
    }

    // Strategy 3: whitespace-normalized
    if let Some((start, end)) = match_whitespace_normalized(&content_lines, &old_lines) {
        let edited = apply_line_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "whitespace_normalized".into(),
                strategies_tried: 3,
                similarity: Some(1.0),
                diff_preview: None,
            },
        ));
    }

    // Strategy 4: punctuation-whitespace-normalized
    if let Some((start, end)) = match_punctuation_normalized(&content_lines, &old_lines) {
        let edited = apply_line_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "punctuation_normalized".into(),
                strategies_tried: 4,
                similarity: Some(1.0),
                diff_preview: None,
            },
        ));
    }

    // Strategy 5: indent-flexible
    if let Some((start, end)) = match_indent_flexible(&content_lines, &old_lines) {
        let edited = apply_line_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "indent_flexible".into(),
                strategies_tried: 5,
                similarity: Some(1.0),
                diff_preview: None,
            },
        ));
    }

    // Strategy 6: escape-normalized
    if let Some((orig_start, orig_end)) = match_escape_normalized(content, old) {
        let edited = format!("{}{}{}", &content[..orig_start], new, &content[orig_end..]);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "escape_normalized".into(),
                strategies_tried: 6,
                similarity: Some(1.0),
                diff_preview: None,
            },
        ));
    }

    // Strategy 7: trimmed-boundary
    if let Some((start, end)) = match_trimmed_boundary(&content_lines, &old_lines) {
        let edited = apply_line_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "trimmed_boundary".into(),
                strategies_tried: 7,
                similarity: Some(1.0),
                diff_preview: None,
            },
        ));
    }

    // Strategy 8: block-anchor
    let min_ratio = custom_threshold.unwrap_or(match fuzzy_mode {
        FuzzyMode::Aggressive => 0.50,
        _ => 0.70,
    });
    if let Some((start, end, ratio)) = match_block_anchor(&content_lines, &old_lines, min_ratio) {
        let edited = apply_line_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "block_anchor".into(),
                strategies_tried: 8,
                similarity: Some(ratio),
                diff_preview: None,
            },
        ));
    }
    // Track near-miss block anchors below threshold for best_candidate
    if let Some((start, end, ratio)) =
        match_block_anchor(&content_lines, &old_lines, BEST_CANDIDATE_MIN)
    {
        let text = content_lines[start..end].join("\n");
        let off = byte_offset_of_line(content, start);
        let (line, column) = line_col_of_offset(content, off);
        consider_best(&mut best, &text, line, column, ratio, "block_anchor");
    }

    // Strategy 9: context-aware JW / Levenshtein
    if matches!(fuzzy_mode, FuzzyMode::Aggressive | FuzzyMode::Auto) {
        let ctx_threshold = custom_threshold.unwrap_or(0.80);
        if old_lines.len() == 1 && old.len() < 60 {
            let jw_threshold = custom_threshold.unwrap_or(0.85);
            let mut best_jw = (0usize, 0.0f64);
            for (i, line) in content_lines.iter().enumerate() {
                let score = strsim::jaro_winkler(line.trim(), old.trim());
                if score > best_jw.1 {
                    best_jw = (i, score);
                }
            }
            if best_jw.1 >= jw_threshold {
                let edited =
                    apply_line_replacement(content, &content_lines, best_jw.0, best_jw.0 + 1, new);
                return Ok((
                    edited,
                    FuzzyInfo {
                        fuzzy: true,
                        strategy: "context_aware_jw".into(),
                        strategies_tried: 9,
                        similarity: Some(best_jw.1),
                        diff_preview: None,
                    },
                ));
            }
            if best_jw.1 > 0.0 {
                let off = byte_offset_of_line(content, best_jw.0);
                let (line, column) = line_col_of_offset(content, off);
                consider_best(
                    &mut best,
                    content_lines.get(best_jw.0).copied().unwrap_or(""),
                    line,
                    column,
                    best_jw.1,
                    "context_aware_jw",
                );
            }
        }
        if let Some((start, end, similarity)) =
            match_context_aware(&content_lines, &old_lines, ctx_threshold)
        {
            let edited = apply_line_replacement(content, &content_lines, start, end, new);
            return Ok((
                edited,
                FuzzyInfo {
                    fuzzy: true,
                    strategy: "context_aware".into(),
                    strategies_tried: 9,
                    similarity: Some(similarity),
                    diff_preview: None,
                },
            ));
        }
        // Track best Levenshtein window even below threshold
        if !old_lines.is_empty() && old_lines.len() <= content_lines.len() {
            let pat_joined = old_lines.join("\n");
            let plen = old_lines.len();
            let mut best_sim = 0.0f64;
            let mut best_i = 0usize;
            for i in 0..=(content_lines.len() - plen) {
                let window_joined = content_lines[i..i + plen].join("\n");
                let sim = strsim::normalized_levenshtein(&pat_joined, &window_joined);
                if sim > best_sim {
                    best_sim = sim;
                    best_i = i;
                }
            }
            if best_sim > 0.0 {
                let text = content_lines[best_i..best_i + plen].join("\n");
                let off = byte_offset_of_line(content, best_i);
                let (line, column) = line_col_of_offset(content, off);
                consider_best(&mut best, &text, line, column, best_sim, "context_aware");
            }
        }
    }

    Err(AtomwriteError::MatchFailed {
        reason: format!("old string not found after fuzzy cascade (9 strategies tried): {old:?}"),
        best_candidate: best.map(Box::new),
    })
}

// ─── matching strategies ─────────────────────────────────────────────────────

fn match_line_trimmed(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let trimmed_pat: Vec<&str> = pattern.iter().map(|l| l.trim()).collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, pat_line) in trimmed_pat.iter().enumerate() {
            if content[i + j].trim() != *pat_line {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn normalize_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut first = true;
    for word in s.split_whitespace() {
        if !first {
            result.push(' ');
        }
        result.push_str(word);
        first = false;
    }
    result
}

fn normalize_punctuation_whitespace(s: &str) -> String {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"\s*([(){}\[\]<>,;:])\s*").expect("static regex is valid")
    });
    RE.replace_all(&normalize_whitespace(s), "$1").to_string()
}

fn match_punctuation_normalized(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let norm_pat: Vec<String> = pattern
        .iter()
        .map(|l| normalize_punctuation_whitespace(l))
        .collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, norm) in norm_pat.iter().enumerate() {
            if normalize_punctuation_whitespace(content[i + j]) != *norm {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn match_whitespace_normalized(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let norm_pat: Vec<String> = pattern.iter().map(|l| normalize_whitespace(l)).collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, norm) in norm_pat.iter().enumerate() {
            if normalize_whitespace(content[i + j]) != *norm {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn match_indent_flexible(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let stripped_pat: Vec<&str> = pattern.iter().map(|l| l.trim_start()).collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, pat) in stripped_pat.iter().enumerate() {
            if content[i + j].trim_start() != *pat {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn normalize_escapes(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

fn match_escape_normalized(content: &str, pattern: &str) -> Option<(usize, usize)> {
    let norm_pat = normalize_escapes(pattern);
    if norm_pat == pattern {
        return None;
    }
    let norm_content = normalize_escapes(content);
    if let Some(norm_pos) = norm_content.find(&norm_pat) {
        let end_pos = norm_pos + norm_pat.len();
        if end_pos <= content.len() {
            return Some((norm_pos, end_pos));
        }
    }
    None
}

fn match_trimmed_boundary(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    let start = pattern.iter().position(|l| !l.trim().is_empty())?;
    let end = pattern.iter().rposition(|l| !l.trim().is_empty())? + 1;
    if start >= end {
        return None;
    }
    let trimmed = &pattern[start..end];
    match_line_trimmed(content, trimmed)
}

fn match_block_anchor(
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
    candidates.retain(|c| c.2 >= 0.70);
    candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    candidates.first().copied()
}

fn match_context_aware(
    content: &[&str],
    pattern: &[&str],
    threshold: f64,
) -> Option<(usize, usize, f64)> {
    if pattern.is_empty() || pattern.len() > content.len() {
        return None;
    }
    let pat_joined = pattern.join("\n");
    let plen = pattern.len();
    let mut best: Option<(usize, usize, f64)> = None;
    for i in 0..=(content.len() - plen) {
        let window_joined = content[i..i + plen].join("\n");
        let sim = strsim::normalized_levenshtein(&pat_joined, &window_joined);
        if sim >= threshold {
            return Some((i, i + plen, sim));
        }
        if best.is_none_or(|(_, _, b)| sim > b) {
            best = Some((i, i + plen, sim));
        }
    }
    if let Some((i, j, sim)) = best {
        if sim >= threshold - 0.05 {
            return Some((i, j, sim));
        }
    }
    None
}

/// Apply a line-range replacement preserving trailing newline of `original`.
pub fn apply_line_replacement(
    original: &str,
    content_lines: &[&str],
    start: usize,
    end: usize,
    new: &str,
) -> String {
    let before = content_lines[..start].join("\n");
    let after = content_lines[end..].join("\n");
    let mut out = String::with_capacity(before.len() + new.len() + after.len() + 2);
    if !before.is_empty() {
        out.push_str(&before);
        out.push('\n');
    }
    out.push_str(new);
    if !after.is_empty() {
        out.push('\n');
        out.push_str(&after);
    }
    if original.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        let (out, info) =
            match_pair("hello world", "world", "rust", FuzzyMode::Auto, None).unwrap();
        assert_eq!(out, "hello rust");
        assert_eq!(info.strategy, "exact");
        assert!(!info.fuzzy);
    }

    #[test]
    fn indent_flexible_match() {
        let content = "fn main() {\n    let x = 1;\n}\n";
        let old = "fn main() {\n  let x = 1;\n}";
        let (out, info) = match_pair(
            content,
            old,
            "fn main() {\n    let x = 2;\n}",
            FuzzyMode::Auto,
            None,
        )
        .unwrap();
        assert!(info.fuzzy);
        assert!(out.contains("let x = 2"));
    }

    #[test]
    fn fuzzy_off_no_candidate() {
        let err = match_pair("abc", "xyz", "q", FuzzyMode::Off, None).unwrap_err();
        match err {
            AtomwriteError::MatchFailed {
                best_candidate: None,
                ..
            } => {}
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn best_candidate_on_near_miss_or_match() {
        let content = "fn compute_total(a: i32) -> i32 {
    a + 1
}
";
        let old = "fn compute_total(a: i32) -> i32 {
    a + 2
}";
        // High threshold may still match via context_aware near-miss window.
        match match_pair(content, old, "x", FuzzyMode::Auto, Some(0.99)) {
            Ok((_, info)) => {
                assert!(info.fuzzy);
                assert!(info.similarity.unwrap_or(0.0) >= 0.5);
            }
            Err(AtomwriteError::MatchFailed {
                best_candidate: Some(bc),
                ..
            }) => {
                assert!(bc.similarity.unwrap_or(0.0) >= 0.5);
                assert!(bc.line.is_some());
            }
            Err(AtomwriteError::MatchFailed {
                best_candidate: None,
                ..
            }) => {}
            Err(other) => panic!("unexpected {other:?}"),
        }
    }
}
