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
        if out.len() > crate::constants::FUZZY_MINI_DIFF_MAX_CHARS {
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

/// Hermes-parity escape-drift guard (G-FZZ-001/083/120).
///
/// When `new` (and usually `old`) contain literal `\'` / `\"` sequences that
/// the **matched file region** does not, the transport almost certainly
/// inserted spurious backslashes. Block before writing so agents re-read.
///
/// `matched_regions` should be the concatenation of the file slices that will
/// be replaced. For pre-match / exact-unknown callers, pass `content`.
pub fn guard_escape_drift(
    old: &str,
    new: &str,
    matched_regions: &str,
) -> std::result::Result<(), AtomwriteError> {
    if !new.contains("\\'") && !new.contains("\\\"") {
        return Ok(());
    }
    for suspect in ["\\'", "\\\""] {
        if new.contains(suspect) && old.contains(suspect) && !matched_regions.contains(suspect) {
            let plain = &suspect[1..];
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "escape-drift blocked: old/new contain {suspect:?} but the matched file region does not; re-read the file and pass unescaped {plain:?} (Hermes-parity post-match guard)"
                ),
            });
        }
        // Also block when only `new` carries the drift (old already exact without escapes).
        if new.contains(suspect) && !old.contains(suspect) && !matched_regions.contains(suspect) {
            let plain = &suspect[1..];
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "escape-drift blocked: `new` contains {suspect:?} not present in old or matched region; re-emit unescaped {plain:?}"
                ),
            });
        }
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

// Scoring: see `score` submodule (gestalt + line_vote).

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

/// Rank top-N candidates for `did_you_mean`.
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
                let sim = strsim::normalized_damerau_levenshtein(&pat_joined, &window_joined);
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

