// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared fuzzy match cascade for edit, replace, batch, and edit-loop (v0.1.30+).
//!
//! Workload: CPU-bound (string normalize + similarity scoring on one buffer).
//! Parallelism: none here — single-file match cascade; callers fan out multi-file
//! work (`replace` WalkParallel, `edit` pair reads). Coordination cost of
//! parallelising strategies inside one file exceeds typical buffer size.
//!
//! Cascade strategies (Auto/Aggressive): exact, line_trimmed, whitespace_normalized,
//! punctuation_normalized, indent_flexible, escape_normalized, trimmed_boundary,
//! block_anchor, unicode_normalized, context_aware_jw / context_aware.
//!
//! Product policy (v0.1.30): fuzzy is mandatory. Exact-only (`Off`) is rejected at
//! the CLI/config surface. Guards: escape-drift, match uniqueness, indent delta,
//! unicode preserve, always-on best_candidate + multi-candidates, diff_preview.
//!
//! v0.1.33 one-shot hardening:
//! - [`apply_fuzzy_one_pass`] never re-scans inserted replacement text (sed-style).
//! - Default max applies = 1; `replacement.contains(pattern)` forces 1.
//! - Pattern / levenshtein / window caps + cooperative cancel poll via
//!   [`crate::signal::is_global_shutdown`].

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
    /// Number of occurrences replaced (1 unless replace_all).
    pub match_count: u64,
    /// True when `apply_indent_delta*` changed the replacement text.
    pub indent_adjusted: bool,
}

/// Options for [`match_pair_with`].
#[derive(Debug, Clone, Copy)]
pub struct MatchOpts {
    /// Fuzzy cascade mode (Auto or Aggressive).
    pub mode: FuzzyMode,
    /// Optional similarity threshold override.
    pub threshold: Option<f64>,
    /// When true, replace every occurrence; when false, require uniqueness.
    pub replace_all: bool,
}

impl Default for MatchOpts {
    fn default() -> Self {
        Self {
            mode: FuzzyMode::Auto,
            threshold: None,
            replace_all: false,
        }
    }
}

/// Minimum similarity to serialize a best-candidate in error envelopes.
const BEST_CANDIDATE_MIN: f64 = 0.5;
/// Truncate candidate text for NDJSON envelopes.
const BEST_CANDIDATE_TEXT_MAX: usize = 500;
/// Max multi did_you_mean candidates.
const MAX_CANDIDATES: usize = 3;

fn find_str(haystack: &str, needle: &str) -> Option<usize> {
    memchr::memmem::find(haystack.as_bytes(), needle.as_bytes())
}

fn count_occurrences(haystack: &str, needle: &str) -> u64 {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0u64;
    let mut start = 0usize;
    while let Some(pos) = find_str(&haystack[start..], needle) {
        count += 1;
        start += pos + needle.len();
        if start >= haystack.len() {
            break;
        }
    }
    count
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

fn mini_diff(expected: &str, found: &str) -> Option<String> {
    let diff = similar::TextDiff::from_lines(expected, found);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            similar::ChangeTag::Delete => '-',
            similar::ChangeTag::Insert => '+',
            similar::ChangeTag::Equal => ' ',
        };
        out.push(sign);
        out.push_str(change.value());
        if out.len() > 800 {
            out.push_str("…\n");
            break;
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn consider_best(
    best: &mut Option<BestCandidate>,
    text: &str,
    line: u64,
    column: u64,
    similarity: f64,
    strategy: &str,
    old: &str,
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
            strategy: Some(strategy.into()),
            diff_preview: mini_diff(old, text),
        });
    }
}

fn push_candidate(
    cands: &mut Vec<BestCandidate>,
    text: &str,
    line: u64,
    column: u64,
    similarity: f64,
    strategy: &str,
    old: &str,
) {
    if similarity < BEST_CANDIDATE_MIN {
        return;
    }
    cands.push(BestCandidate {
        text: Some(truncate_text(text)),
        line: Some(line),
        column: Some(column),
        similarity: Some(similarity),
        strategy: Some(strategy.into()),
        diff_preview: mini_diff(old, text),
    });
    cands.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    cands.dedup_by(|a, b| a.line == b.line && a.text == b.text);
    if cands.len() > MAX_CANDIDATES {
        cands.truncate(MAX_CANDIDATES);
    }
}

/// Reject LLM tool-call escape drift: `new` has `\'`/`\"` that the file never had.
pub fn guard_escape_drift(old: &str, new: &str, content: &str) -> std::result::Result<(), AtomwriteError> {
    let new_esc = new.contains("\\'") || new.contains("\\\"");
    if !new_esc {
        return Ok(());
    }
    let content_has = content.contains("\\'") || content.contains("\\\"");
    let old_has = old.contains("\\'") || old.contains("\\\"");
    if !content_has && !old_has {
        return Err(AtomwriteError::InvalidInput {
            reason: "escape-drift blocked: `new` contains escaped quotes (\\' or \\\") that are not present in the file or old; re-emit unescaped quotes".into(),
        });
    }
    Ok(())
}

fn leading_ws(s: &str) -> &str {
    let trimmed = s.trim_start();
    &s[..s.len() - trimmed.len()]
}

/// Realign `new` indent to match the matched block's per-line indentation.
pub fn apply_indent_delta(matched_first_line: &str, new: &str) -> String {
    apply_indent_delta_block(&[matched_first_line], new)
}

/// Realign each line of `new` to the corresponding matched line indent.
pub fn apply_indent_delta_block(matched_lines: &[&str], new: &str) -> String {
    let new_lines: Vec<&str> = new.lines().collect();
    if new_lines.is_empty() || matched_lines.is_empty() {
        return new.to_string();
    }
    let new_base = leading_ws(new_lines[0]);
    let mut out = String::with_capacity(new.len() + 16);
    for (i, line) in new_lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if line.is_empty() {
            continue;
        }
        let file_indent = matched_lines
            .get(i)
            .or_else(|| matched_lines.last())
            .map(|l| leading_ws(l))
            .unwrap_or("");
        let stripped = if !new_base.is_empty() && line.starts_with(new_base) {
            &line[new_base.len()..]
        } else {
            line.trim_start()
        };
        out.push_str(file_indent);
        out.push_str(stripped);
    }
    if new.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn normalize_unicode_for_match(s: &str) -> String {
    s.replace('—', "-")
        .replace('–', "-")
        .replace('\u{201c}', "\"")
        .replace('\u{201d}', "\"")
        .replace('\u{2018}', "'")
        .replace('\u{2019}', "'")
        .replace('…', "...")
}

/// Prefer file unicode when new used ASCII stand-ins for fancy punctuation.
fn preserve_unicode_in_replacement(matched: &str, new: &str) -> String {
    let mut out = new.to_string();
    // If matched has em-dash and new has hyphen in similar places, keep new but
    // map ASCII quotes/dashes back when matched used fancy forms exclusively.
    if matched.contains('—') && out.contains('-') && !out.contains('—') {
        // only replace isolated " - " style when matched uses —
        out = out.replace(" - ", " — ");
    }
    if (matched.contains('\u{201c}') || matched.contains('\u{201d}')) && out.contains('"') {
        // leave double quotes; agents often want ASCII in code
    }
    out
}

/// Rank best near-miss for diagnostics (always-on, including exact residual).
pub fn rank_best_candidate(content: &str, old: &str) -> Option<BestCandidate> {
    let mut best: Option<BestCandidate> = None;
    let mut cands: Vec<BestCandidate> = Vec::new();
    rank_into(content, old, &mut best, &mut cands);
    best
}

/// Rank top-N candidates for did_you_mean.
pub fn rank_candidates(content: &str, old: &str) -> Vec<BestCandidate> {
    let mut best: Option<BestCandidate> = None;
    let mut cands: Vec<BestCandidate> = Vec::new();
    rank_into(content, old, &mut best, &mut cands);
    if cands.is_empty() {
        if let Some(b) = best {
            cands.push(b);
        }
    }
    cands
}

fn rank_into(
    content: &str,
    old: &str,
    best: &mut Option<BestCandidate>,
    cands: &mut Vec<BestCandidate>,
) {
    let old_lines: Vec<&str> = old.lines().collect();
    let content_lines: Vec<&str> = content.lines().collect();
    if old_lines.is_empty() || content_lines.is_empty() {
        return;
    }
    // JW per line for single-line old
    if old_lines.len() == 1 {
        let needle = old_lines[0].trim();
        for (i, line) in content_lines.iter().enumerate() {
            let score = strsim::jaro_winkler(line.trim(), needle);
            if score >= BEST_CANDIDATE_MIN {
                let off = byte_offset_of_line(content, i);
                let (line_n, col) = line_col_of_offset(content, off);
                consider_best(best, line, line_n, col, score, "context_aware_jw", old);
                push_candidate(cands, line, line_n, col, score, "context_aware_jw", old);
            }
        }
    }
    // Levenshtein windows (capped + cancel-polled; v0.1.33)
    if !old_lines.is_empty() && old_lines.len() <= content_lines.len() {
        let pat_joined = old_lines.join("\n");
        if pat_joined.chars().count() <= crate::constants::FUZZY_MAX_LEVENSHTEIN_CHARS {
            let plen = old_lines.len();
            let last = content_lines.len() - plen;
            for (n, i) in (0..=last).enumerate() {
                if n >= crate::constants::FUZZY_MAX_WINDOWS {
                    break;
                }
                if n % 64 == 0 && crate::signal::is_global_shutdown() {
                    break;
                }
                let window_joined = content_lines[i..i + plen].join("\n");
                if window_joined.chars().count() > crate::constants::FUZZY_MAX_LEVENSHTEIN_CHARS {
                    continue;
                }
                let sim = strsim::normalized_levenshtein(&pat_joined, &window_joined);
                if sim >= BEST_CANDIDATE_MIN {
                    let off = byte_offset_of_line(content, i);
                    let (line_n, col) = line_col_of_offset(content, off);
                    consider_best(best, &window_joined, line_n, col, sim, "context_aware", old);
                    push_candidate(cands, &window_joined, line_n, col, sim, "context_aware", old);
                }
            }
        }
    }
}

fn ambiguous_err(_old: &str, count: u64) -> AtomwriteError {
    AtomwriteError::MatchAmbiguous {
        reason: format!(
            "found {count} matches for old string; provide more context to make it unique or pass --replace-all"
        ),
        count,
        best_candidate: None,
    }
}

/// Resolve `old` → `new` with default uniqueness (no replace_all).
pub fn match_pair(
    content: &str,
    old: &str,
    new: &str,
    fuzzy_mode: FuzzyMode,
    custom_threshold: Option<f64>,
) -> std::result::Result<(String, FuzzyInfo), AtomwriteError> {
    match_pair_with(
        content,
        old,
        new,
        MatchOpts {
            mode: fuzzy_mode,
            threshold: custom_threshold,
            replace_all: false,
        },
    )
}

/// Full match with replace_all / uniqueness / guards (v0.1.30).
pub fn match_pair_with(
    content: &str,
    old: &str,
    new: &str,
    opts: MatchOpts,
) -> std::result::Result<(String, FuzzyInfo), AtomwriteError> {
    check_cancel()?;
    if old.is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: "old string must not be empty".into(),
        });
    }
    if old.len() > crate::constants::FUZZY_MAX_PATTERN_BYTES {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "fuzzy pattern too large ({} bytes > {} max); shorten the block or use --old-file with a smaller unique slice",
                old.len(),
                crate::constants::FUZZY_MAX_PATTERN_BYTES
            ),
        });
    }
    guard_escape_drift(old, new, content)?;

    // Reject legacy Off if somehow constructed
    if matches!(opts.mode, FuzzyMode::Off) {
        return Err(AtomwriteError::InvalidInput {
            reason: "fuzzy mode 'off' was removed in v0.1.30; use auto (default) or aggressive".into(),
        });
    }

    // Strategy 1: exact
    let exact_count = count_occurrences(content, old);
    if exact_count > 0 {
        if exact_count > 1 && !opts.replace_all {
            return Err(ambiguous_err(old, exact_count));
        }
        let edited = if opts.replace_all {
            content.replace(old, new)
        } else {
            let pos = find_str(content, old).expect("count > 0");
            format!("{}{}{}", &content[..pos], new, &content[pos + old.len()..])
        };
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: false,
                strategy: "exact".into(),
                strategies_tried: 1,
                similarity: None,
                diff_preview: mini_diff(old, old),
                match_count: if opts.replace_all { exact_count } else { 1 },
                indent_adjusted: false,
            },
        ));
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let content_lines: Vec<&str> = content.lines().collect();
    let mut best: Option<BestCandidate> = None;
    let mut cands: Vec<BestCandidate> = Vec::new();

    // Collect all ranges for a strategy
    let collect_all = |finder: fn(&[&str], &[&str]) -> Option<(usize, usize)>| -> Vec<(usize, usize)> {
        let mut hits = Vec::new();
        let mut offset = 0usize;
        let mut slice = content_lines.as_slice();
        while let Some((s, e)) = finder(slice, &old_lines) {
            hits.push((s + offset, e + offset));
            offset += e;
            if offset >= content_lines.len() {
                break;
            }
            slice = &content_lines[offset..];
            if slice.is_empty() {
                break;
            }
        }
        // Simpler: scan without slicing for non-overlapping
        hits.clear();
        let mut i = 0usize;
        while i < content_lines.len() {
            if let Some((s, e)) = finder(&content_lines[i..], &old_lines) {
                hits.push((i + s, i + e));
                i = i + e;
            } else {
                break;
            }
            // Also try advancing by 1 if finder always searches from 0 of slice
        }
        // Robust scan: check every start
        hits.clear();
        if old_lines.is_empty() {
            return hits;
        }
        let mut start_at = 0usize;
        while start_at < content_lines.len() {
            if let Some((s, e)) = finder(&content_lines[start_at..], &old_lines) {
                let abs_s = start_at + s;
                let abs_e = start_at + e;
                hits.push((abs_s, abs_e));
                start_at = abs_e;
            } else {
                break;
            }
        }
        hits
    };

    let apply_hits = |hits: Vec<(usize, usize)>, name: &str, tried: u64, sim: Option<f64>| -> std::result::Result<(String, FuzzyInfo), AtomwriteError> {
        if hits.is_empty() {
            return Err(AtomwriteError::InvalidInput {
                reason: "internal: empty hits".into(),
            });
        }
        if hits.len() > 1 && !opts.replace_all {
            return Err(ambiguous_err(old, hits.len() as u64));
        }
        if hits.len() == 1 || !opts.replace_all {
            let (start, end) = hits[0];
            let matched = content_lines[start..end].join("\n");
            let matched_slice = &content_lines[start..end];
            let adjusted_new = apply_indent_delta_block(matched_slice, new);
            let indent_adjusted = adjusted_new != new;
            let adjusted_new = preserve_unicode_in_replacement(&matched, &adjusted_new);
            let edited = apply_line_replacement(content, &content_lines, start, end, &adjusted_new);
            return Ok((
                edited,
                FuzzyInfo {
                    fuzzy: true,
                    strategy: name.into(),
                    strategies_tried: tried,
                    similarity: sim,
                    diff_preview: mini_diff(old, &matched),
                    match_count: 1,
                    indent_adjusted,
                },
            ));
        }
        // replace_all: apply from bottom
        let mut lines: Vec<String> = content_lines.iter().map(|s| (*s).to_string()).collect();
        let mut ordered = hits;
        ordered.sort_by(|a, b| b.0.cmp(&a.0));
        let count = ordered.len() as u64;
        let mut indent_adjusted = false;
        for (start, end) in ordered {
            let matched_first = lines.get(start).map(|s| s.as_str()).unwrap_or("");
            let adjusted = apply_indent_delta(matched_first, new);
            indent_adjusted |= adjusted != new;
            let before: Vec<&str> = lines[..start].iter().map(|s| s.as_str()).collect();
            let after: Vec<&str> = lines[end..].iter().map(|s| s.as_str()).collect();
            let mut rebuilt = Vec::new();
            rebuilt.extend(before.iter().map(|s| (*s).to_string()));
            for nl in adjusted.lines() {
                rebuilt.push(nl.to_string());
            }
            if adjusted.ends_with('\n') {
                // lines() drops trailing empty; keep simple
            }
            rebuilt.extend(after.iter().map(|s| (*s).to_string()));
            lines = rebuilt;
        }
        let mut out = lines.join("\n");
        if content.ends_with('\n') && !out.ends_with('\n') {
            out.push('\n');
        }
        Ok((
            out,
            FuzzyInfo {
                fuzzy: true,
                strategy: name.into(),
                strategies_tried: tried,
                similarity: sim,
                diff_preview: None,
                match_count: count,
                indent_adjusted,
            },
        ))
    };

    // Strategy 2–5,7
    for (name, tried, finder) in [
        (
            "line_trimmed",
            2u64,
            match_line_trimmed as fn(&[&str], &[&str]) -> Option<(usize, usize)>,
        ),
        ("whitespace_normalized", 3, match_whitespace_normalized),
        ("punctuation_normalized", 4, match_punctuation_normalized),
        ("indent_flexible", 5, match_indent_flexible),
        ("trimmed_boundary", 7, match_trimmed_boundary),
    ] {
        let hits = collect_all(finder);
        if !hits.is_empty() {
            return apply_hits(hits, name, tried, Some(1.0));
        }
    }

    // Strategy 6: escape-normalized (byte offsets)
    if let Some((orig_start, orig_end)) = match_escape_normalized(content, old) {
        // uniqueness: count via normalized
        let edited = format!(
            "{}{}{}",
            &content[..orig_start],
            new,
            &content[orig_end..]
        );
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "escape_normalized".into(),
                strategies_tried: 6,
                similarity: Some(1.0),
                diff_preview: mini_diff(old, &content[orig_start..orig_end]),
                match_count: 1,
                indent_adjusted: false,
            },
        ));
    }

    // Strategy 8: block-anchor
    let min_ratio = opts.threshold.unwrap_or(match opts.mode {
        FuzzyMode::Aggressive => 0.50,
        FuzzyMode::Auto => 0.70,
        FuzzyMode::Off => 0.70,
    });
    if let Some((start, end, ratio)) = match_block_anchor(&content_lines, &old_lines, min_ratio) {
        return apply_hits(vec![(start, end)], "block_anchor", 8, Some(ratio));
    }
    if let Some((start, end, ratio)) =
        match_block_anchor(&content_lines, &old_lines, BEST_CANDIDATE_MIN)
    {
        let text = content_lines[start..end].join("\n");
        let off = byte_offset_of_line(content, start);
        let (line, column) = line_col_of_offset(content, off);
        consider_best(&mut best, &text, line, column, ratio, "block_anchor", old);
        push_candidate(&mut cands, &text, line, column, ratio, "block_anchor", old);
    }

    // Strategy 8b: unicode_normalized
    if let Some((start, end)) = match_unicode_normalized(&content_lines, &old_lines) {
        return apply_hits(vec![(start, end)], "unicode_normalized", 8, Some(1.0));
    }

    // Strategy 9: context-aware
    if matches!(opts.mode, FuzzyMode::Aggressive | FuzzyMode::Auto) {
        let ctx_threshold = opts.threshold.unwrap_or(0.80);
        if old_lines.len() == 1 && old.len() < 60 {
            let jw_threshold = opts.threshold.unwrap_or(0.85);
            let mut best_jw = (0usize, 0.0f64);
            let mut all_jw: Vec<(usize, f64)> = Vec::new();
            for (i, line) in content_lines.iter().enumerate() {
                let score = strsim::jaro_winkler(line.trim(), old.trim());
                if score >= jw_threshold {
                    all_jw.push((i, score));
                }
                if score > best_jw.1 {
                    best_jw = (i, score);
                }
            }
            if !all_jw.is_empty() {
                if all_jw.len() > 1 && !opts.replace_all {
                    return Err(ambiguous_err(old, all_jw.len() as u64));
                }
                let (i, score) = all_jw[0];
                return apply_hits(vec![(i, i + 1)], "context_aware_jw", 9, Some(score));
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
                    old,
                );
            }
        }
        if let Some((start, end, similarity)) =
            match_context_aware(&content_lines, &old_lines, ctx_threshold)
        {
            return apply_hits(vec![(start, end)], "context_aware", 9, Some(similarity));
        }
        rank_into(content, old, &mut best, &mut cands);
    }

    let reason =
        format!("old string not found after fuzzy cascade (strategies tried): {old:?}");
    let best = best.or_else(|| cands.first().cloned()).map(Box::new);
    Err(AtomwriteError::MatchFailed {
        reason,
        best_candidate: best,
        candidates: if cands.len() > 1 {
            Some(cands)
        } else {
            None
        },
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
    // Function-local `static LazyLock` (not `const`): compiled once, stable
    // address, shared across all call sites. `const` would re-compile on each use.
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

fn match_unicode_normalized(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let norm_pat: Vec<String> = pattern
        .iter()
        .map(|l| normalize_unicode_for_match(l))
        .collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, norm) in norm_pat.iter().enumerate() {
            if normalize_unicode_for_match(content[i + j]) != *norm {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
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
    if pat_joined.chars().count() > crate::constants::FUZZY_MAX_LEVENSHTEIN_CHARS {
        return None;
    }
    let plen = pattern.len();
    let max_windows = crate::constants::FUZZY_MAX_WINDOWS;
    let mut best: Option<(usize, usize, f64)> = None;
    let last = content.len() - plen;
    for (n, i) in (0..=last).enumerate() {
        if n >= max_windows {
            break;
        }
        if n % 64 == 0 && crate::signal::is_global_shutdown() {
            return None;
        }
        let window_joined = content[i..i + plen].join("\n");
        if window_joined.chars().count() > crate::constants::FUZZY_MAX_LEVENSHTEIN_CHARS {
            continue;
        }
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

/// Result of [`apply_fuzzy_one_pass`] (v0.1.33).
#[derive(Debug, Clone)]
pub struct FuzzyOnePassResult {
    /// Edited buffer (or original when `applied == 0`).
    pub edited: String,
    /// Number of successful applies (never re-scans inserted text).
    pub applied: u64,
    /// Last successful match diagnostics.
    pub info: Option<FuzzyInfo>,
    /// True when `replacement` contains `pattern` (forces single apply).
    pub replacement_embeds_pattern: bool,
}

#[inline]
fn check_cancel() -> std::result::Result<(), AtomwriteError> {
    if crate::signal::is_global_shutdown() {
        return Err(crate::signal::cancelled_error(
            "fuzzy match cancelled by signal or --timeout-secs",
        ));
    }
    Ok(())
}

fn max_edited_len(input_len: usize) -> usize {
    let by_factor = input_len.saturating_mul(crate::constants::FUZZY_MAX_BUFFER_GROWTH_FACTOR);
    let by_bytes = input_len.saturating_add(crate::constants::FUZZY_MAX_BUFFER_GROWTH_BYTES);
    by_factor.max(by_bytes)
}

/// One-pass fuzzy apply for multi-file `replace` (v0.1.33 one-shot).
///
/// Never re-scans text that was just inserted (sed / `str::replacen` semantics).
/// Default when `max_replacements` is `None`: **1** apply.
/// When `replacement` contains `pattern`, force **1** apply even if a higher
/// `--max-replacements` was requested (prevents infinite growth).
///
/// Multi-hit (`max_replacements > 1`) advances a cursor on the **original**
/// content only: each subsequent search starts after the previous match end.
pub fn apply_fuzzy_one_pass(
    content: &str,
    pattern: &str,
    replacement: &str,
    fuzzy_mode: FuzzyMode,
    custom_threshold: Option<f64>,
    max_replacements: Option<usize>,
) -> std::result::Result<FuzzyOnePassResult, AtomwriteError> {
    check_cancel()?;
    if pattern.is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: "old string must not be empty".into(),
        });
    }
    if pattern.len() > crate::constants::FUZZY_MAX_PATTERN_BYTES {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "fuzzy pattern too large ({} bytes > {} max); shorten the block",
                pattern.len(),
                crate::constants::FUZZY_MAX_PATTERN_BYTES
            ),
        });
    }

    let embeds = replacement.contains(pattern);
    let mut limit = max_replacements
        .map(|n| n as u64)
        .unwrap_or(crate::constants::FUZZY_DEFAULT_MAX_REPLACEMENTS);
    if embeds {
        limit = 1;
    }
    limit = limit.min(crate::constants::FUZZY_HARD_MAX_REPLACEMENTS);
    if limit == 0 {
        return Ok(FuzzyOnePassResult {
            edited: content.to_string(),
            applied: 0,
            info: None,
            replacement_embeds_pattern: embeds,
        });
    }

    // Fast path: single apply (agent default / embeds).
    if limit == 1 {
        match match_pair(content, pattern, replacement, fuzzy_mode, custom_threshold) {
            Ok((edited, info)) => {
                let cap = max_edited_len(content.len());
                if edited.len() > cap {
                    return Err(AtomwriteError::InvalidInput {
                        reason: format!(
                            "fuzzy edit would grow buffer from {} to {} bytes (cap {cap}); aborting for one-shot safety",
                            content.len(),
                            edited.len()
                        ),
                    });
                }
                Ok(FuzzyOnePassResult {
                    edited,
                    applied: 1,
                    info: Some(info),
                    replacement_embeds_pattern: embeds,
                })
            }
            Err(AtomwriteError::Cancelled { .. }) => Err(crate::signal::cancelled_error(
                "fuzzy match cancelled by signal or --timeout-secs",
            )),
            Err(_) => Ok(FuzzyOnePassResult {
                edited: content.to_string(),
                applied: 0,
                info: None,
                replacement_embeds_pattern: embeds,
            }),
        }
    } else {
        // Multi-hit on ORIGINAL content with advancing cursor (never search inside inserts).
        let mut pos = 0usize;
        let mut out = String::new();
        let cap = max_edited_len(content.len());
        out.try_reserve(content.len().min(cap)).map_err(|e| {
            AtomwriteError::InvalidInput {
                reason: format!("failed to reserve edit buffer: {e}"),
            }
        })?;
        let mut applied = 0u64;
        let mut last_info: Option<FuzzyInfo> = None;

        while applied < limit {
            check_cancel()?;
            if pos >= content.len() {
                break;
            }
            let slice = &content[pos..];
            match match_pair(slice, pattern, replacement, fuzzy_mode, custom_threshold) {
                Ok((edited_slice, info)) => {
                    // Locate the matched span by recovering the preimage:
                    // edited_slice = prefix + replacement_adjusted + suffix of slice.
                    // Prefer exact pattern position in slice; else first line-anchor heuristic.
                    let (rel_start, rel_end, adjusted_new) =
                        locate_applied_span(slice, pattern, replacement, &edited_slice, &info)
                            .unwrap_or((0, slice.len(), edited_slice.clone()));
                    out.push_str(&content[pos..pos + rel_start]);
                    out.push_str(&adjusted_new);
                    if out.len() > cap {
                        return Err(AtomwriteError::InvalidInput {
                            reason: format!(
                                "fuzzy multi-edit exceeded growth cap ({cap} bytes); aborting"
                            ),
                        });
                    }
                    pos += rel_end;
                    applied += 1;
                    last_info = Some(info);
                    // Advance at least one byte on zero-width edge cases.
                    if rel_end == rel_start {
                        pos = pos.saturating_add(1);
                    }
                }
                Err(AtomwriteError::Cancelled { .. }) => {
                    return Err(crate::signal::cancelled_error(
                        "fuzzy match cancelled by signal or --timeout-secs",
                    ));
                }
                Err(_) => break,
            }
        }
        out.push_str(&content[pos..]);
        Ok(FuzzyOnePassResult {
            edited: out,
            applied,
            info: last_info,
            replacement_embeds_pattern: embeds,
        })
    }
}

/// Recover (start, end exclusive, adjusted_new) of the first apply inside `slice`.
fn locate_applied_span(
    slice: &str,
    pattern: &str,
    replacement: &str,
    edited_slice: &str,
    _info: &FuzzyInfo,
) -> Option<(usize, usize, String)> {
    if let Some(start) = find_str(slice, pattern) {
        let end = start + pattern.len();
        let prefix = &slice[..start];
        let suffix = &slice[end..];
        if edited_slice.starts_with(prefix)
            && edited_slice.ends_with(suffix)
            && edited_slice.len() >= prefix.len() + suffix.len()
        {
            let adj = edited_slice[prefix.len()..edited_slice.len() - suffix.len()].to_string();
            return Some((start, end, adj));
        }
        return Some((start, end, replacement.to_string()));
    }
    // Fuzzy: derive from longest common prefix/suffix between slice and edited.
    let prefix = common_prefix_len(slice.as_bytes(), edited_slice.as_bytes());
    // Avoid claiming the entire slice when edit failed to shrink/grow sanely.
    if prefix == slice.len() && edited_slice == slice {
        return None;
    }
    let a = &slice.as_bytes()[prefix..];
    let b = if prefix <= edited_slice.len() {
        &edited_slice.as_bytes()[prefix..]
    } else {
        &[]
    };
    let suffix = common_suffix_len(a, b);
    if prefix + suffix > slice.len() {
        return None;
    }
    let end = slice.len() - suffix;
    if end <= prefix {
        return None;
    }
    let adj_end = edited_slice.len().saturating_sub(suffix);
    if adj_end < prefix {
        return None;
    }
    let adjusted = edited_slice[prefix..adj_end].to_string();
    Some((prefix, end, adjusted))
}

fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
}

fn common_suffix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count()
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
    fn fuzzy_off_rejected() {
        let err = match_pair("abc", "xyz", "q", FuzzyMode::Off, None).unwrap_err();
        match err {
            AtomwriteError::InvalidInput { reason } => {
                assert!(reason.contains("0.1.30") || reason.contains("off"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn uniqueness_requires_replace_all() {
        let content = "aa\nxx\naa\n";
        let err = match_pair(content, "aa", "bb", FuzzyMode::Auto, None).unwrap_err();
        match err {
            AtomwriteError::MatchAmbiguous { count, .. } => assert_eq!(count, 2),
            other => panic!("unexpected {other:?}"),
        }
        let (out, info) = match_pair_with(
            content,
            "aa",
            "bb",
            MatchOpts {
                mode: FuzzyMode::Auto,
                threshold: None,
                replace_all: true,
            },
        )
        .unwrap();
        assert_eq!(info.match_count, 2);
        assert_eq!(out.matches("bb").count(), 2);
    }

    #[test]
    fn escape_drift_blocked() {
        let err = guard_escape_drift("foo", "bar\\'s", "foo bar").unwrap_err();
        match err {
            AtomwriteError::InvalidInput { reason } => assert!(reason.contains("escape-drift")),
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
            Err(other) => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn unicode_normalized_matches_emdash() {
        let content = "note — important\n";
        let old = "note - important";
        let (out, info) = match_pair(content, old, "note — done", FuzzyMode::Auto, None).unwrap();
        assert_eq!(info.strategy, "unicode_normalized");
        assert!(out.contains("done"));
    }

    #[test]
    fn indent_delta_realigns_new() {
        // old must not be a raw substring (double-space is subset of 4-space).
        let content = "fn main() {\n    let x = 1;\n}\n";
        let old = "fn main() {\n  let x = 1;\n}";
        let (out, info) = match_pair(
            content,
            old,
            "fn main() {\n  let x = 2;\n}",
            FuzzyMode::Auto,
            None,
        )
        .unwrap();
        assert!(info.fuzzy);
        assert!(out.contains("    let x = 2"));
    }

    #[test]
    fn one_pass_embeds_pattern_applies_once() {
        // Classic agent footgun: NEW contains OLD. Must terminate with 1 apply.
        let content = "header\nAAA\nfooter\n";
        let old = "AAA";
        let new = "AAA\nBBB"; // embeds old
        let r = apply_fuzzy_one_pass(content, old, new, FuzzyMode::Auto, None, Some(1_000_000))
            .expect("must succeed");
        assert!(r.replacement_embeds_pattern);
        assert_eq!(r.applied, 1, "embeds must force single apply");
        assert_eq!(r.edited, "header\nAAA\nBBB\nfooter\n");
        // Second conceptual apply would grow forever without the guard.
        assert!(!r.edited.contains("AAA\nBBB\nBBB"));
    }

    #[test]
    fn one_pass_default_limit_is_one() {
        let content = "unique_token_alpha beta\n";
        let r = apply_fuzzy_one_pass(
            content,
            "unique_token_alpha",
            "unique_token_omega",
            FuzzyMode::Auto,
            None,
            None,
        )
        .unwrap();
        assert_eq!(r.applied, 1);
        assert_eq!(r.edited, "unique_token_omega beta\n");
        assert!(!r.replacement_embeds_pattern);
    }

    #[test]
    fn one_pass_rejects_oversized_pattern() {
        let big = "a".repeat(crate::constants::FUZZY_MAX_PATTERN_BYTES + 1);
        let err = apply_fuzzy_one_pass("a", &big, "b", FuzzyMode::Auto, None, None).unwrap_err();
        assert!(matches!(err, AtomwriteError::InvalidInput { .. }));
    }
}
