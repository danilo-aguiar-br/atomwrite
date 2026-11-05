/// Resolve `old` → `new` with default uniqueness (no `replace_all`).
///
/// # Examples
///
/// ```
/// use atomwrite::cli::FuzzyMode;
/// use atomwrite::fuzzy::match_pair;
///
/// let content = "fn main() {\n    let x = 1;\n}\n";
/// let (updated, info) = match_pair(content, "let x = 1;", "let x = 2;", FuzzyMode::Off, None)?;
/// assert_eq!(updated, "fn main() {\n    let x = 2;\n}\n");
/// assert!(!info.fuzzy);
///
/// // A near miss fails with `FuzzyMode::Off` and the best candidate is ranked.
/// let miss = match_pair(content, "let x = 10;", "let x = 2;", FuzzyMode::Off, None);
/// assert!(miss.is_err());
/// # Ok::<(), atomwrite::error::AtomwriteError>(())
/// ```
///
/// # Errors
///
/// Returns `AtomwriteError::Cancelled` when the global shutdown flag is set by a signal or `--timeout-secs`.
/// Returns `AtomwriteError::InvalidInput` when `old` is empty, when `new` carries a backslash-escaped quote absent from `old` and the matched region, or when `old` exceeds `FUZZY_MAX_PATTERN_BYTES`.
/// Returns `AtomwriteError::MatchAmbiguous` when `old` matches more than one region, because this wrapper never sets `replace_all`.
/// Returns `AtomwriteError::MatchFailed` when no exact or fuzzy strategy matches, carrying the best candidate when one clears the floor.
/// Returns `AtomwriteError::InternalError` when the line table cannot be allocated or an exact match vanishes between count and replace.
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
            ..Default::default()
        },
    )
}

/// Resolve pair using full XDG [`crate::config::FuzzySection`] thresholds (R-THR wire).
///
/// # Errors
///
/// Returns `AtomwriteError::Cancelled` when the global shutdown flag is set by a signal or `--timeout-secs`.
/// Returns `AtomwriteError::InvalidInput` when `old` is empty, when `new` carries a backslash-escaped quote absent from `old` and the matched region, when `old` exceeds `FUZZY_MAX_PATTERN_BYTES`, or when `content` exceeds `section.max_content_bytes`.
/// Returns `AtomwriteError::MatchAmbiguous` when more than one region matches and `replace_all` is false.
/// Returns `AtomwriteError::MatchFailed` when no strategy matches, carrying the best candidate when one clears the floor.
/// Returns `AtomwriteError::InternalError` when the line table cannot be allocated or an exact match vanishes between count and replace.
pub fn match_pair_cfg(
    content: &str,
    old: &str,
    new: &str,
    mode: FuzzyMode,
    cli_threshold: Option<f64>,
    section: &crate::config::FuzzySection,
    replace_all: bool,
) -> std::result::Result<(String, FuzzyInfo), AtomwriteError> {
    match_pair_with(
        content,
        old,
        new,
        match_opts_from_section(mode, cli_threshold, section, replace_all),
    )
}

/// Full match with `replace_all` / uniqueness / guards (v0.1.30).
///
/// # Errors
///
/// Returns `AtomwriteError::Cancelled` when the global shutdown flag is set by a signal or `--timeout-secs` before or during the cascade.
/// Returns `AtomwriteError::InvalidInput` when `old` is empty, when `new` carries a backslash-escaped quote absent from `old` and the matched region, when `old` exceeds `FUZZY_MAX_PATTERN_BYTES`, or when `content` exceeds `opts.max_content_bytes`.
/// Returns `AtomwriteError::MatchAmbiguous` when more than one region matches and `replace_all` is false.
/// Returns `AtomwriteError::MatchFailed` when no strategy matches, carrying the best candidate when one clears the floor.
/// Returns `AtomwriteError::InternalError` when the line table cannot be allocated or an exact match vanishes between count and replace.
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
    guard_escape_drift(old, new, content)?;

    // Strategy 1: exact first (G-048: no FUZZY_MAX_PATTERN on exact / large --old-file)
    let exact_count = count_occurrences(content, old)?;
    if exact_count > 0 {
        if exact_count > 1 && !opts.replace_all {
            let off = find_str(content, old).unwrap_or(0);
            let (line, column) = line_col_of_offset(content, off);
            let bc = BestCandidate {
                text: Some(truncate_text(old)),
                line: Some(line),
                column: Some(column),
                similarity: Some(1.0),
                strategy: Some("exact".into()),
                diff_preview: None,
            };
            return Err(ambiguous_err(old, exact_count, Some(bc)));
        }
        let edited = if opts.replace_all {
            content.replace(old, new)
        } else {
            let Some(pos) = find_str(content, old) else {
                return Err(AtomwriteError::InternalError {
                    reason: "exact match vanished between count and replace".into(),
                });
            };
            format!("{}{}{}", &content[..pos], new, &content[pos + old.len()..])
        };
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: false,
                strategy: "exact".into(),
                strategies_tried: crate::constants::FUZZY_TRIED_EXACT,
                similarity: None,
                // The preview answers "what changed", so it compares the text
                // that left with the text that arrived. Comparing `old` with
                // itself printed the removed line twice and never showed the
                // replacement, which is the one thing the caller asked for.
                diff_preview: mini_diff(old, new),
                match_count: if opts.replace_all { exact_count } else { 1 },
                indent_adjusted: false,
            },
        ));
    }

    // G-010: Off stops after exact — no fuzzy cascade.
    if matches!(opts.mode, FuzzyMode::Off) {
        return Err(AtomwriteError::MatchFailed {
            reason: format!("old string not found (fuzzy mode off = exact-only): {old:?}"),
            best_candidate: None,
            candidates: None,
        });
    }

    // G-048: fuzzy cascade only — cap pattern size (exact already handled above).
    if old.len() > crate::constants::FUZZY_MAX_PATTERN_BYTES {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "{} ({} bytes > {} max); shorten the block or use a smaller unique slice for fuzzy match",
                crate::constants::FUZZY_PATTERN_CAP_REASON_MARKER,
                old.len(),
                crate::constants::FUZZY_MAX_PATTERN_BYTES
            ),
        });
    }

    // v0.1.37: above the cap the cascade would not finish inside the deadline;
    // fail loudly instead of reporting a miss. Exact matching above is uncapped.
    if content.len() as u64 > opts.max_content_bytes {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "content too large for the fuzzy cascade ({} bytes > {} max); raise {} in config.toml or pass --fuzzy off for exact-only matching",
                content.len(),
                opts.max_content_bytes,
                crate::constants::FUZZY_CONTENT_CAP_REASON_MARKER
            ),
        });
    }
    check_cancel()?;

    let old_lines = collect_lines(old)?;
    let content_lines = collect_lines(content)?;
    let mut best: Option<BestCandidate> = None;
    let mut cands: Vec<BestCandidate> = Vec::new();

    // GAP-2026-026: non-overlapping collect lives in collect.inc.rs (SRP).
    let collect_all =
        |finder: LineRangeFinder| -> std::result::Result<Vec<(usize, usize)>, AtomwriteError> {
            collect_all_ranges(&content_lines, &old_lines, finder)
        };

    let apply_hits = |hits: Vec<(usize, usize)>,
                      name: &str,
                      tried: u64,
                      sim: Option<f64>|
     -> std::result::Result<(String, FuzzyInfo), AtomwriteError> {
        if hits.is_empty() {
            return Err(AtomwriteError::InvalidInput {
                reason: "internal: empty hits".into(),
            });
        }
        if hits.len() > 1 && !opts.replace_all {
            let (start, end) = hits[0];
            let text = content_lines[start..end].join("\n");
            let (line, column) = line_start_position(start);
            let bc = BestCandidate {
                text: Some(truncate_text(&text)),
                line: Some(line),
                column: Some(column),
                similarity: None,
                strategy: Some("ambiguous".into()),
                diff_preview: None,
            };
            return Err(ambiguous_err(old, hits.len() as u64, Some(bc)));
        }
        if hits.len() == 1 || !opts.replace_all {
            let (start, end) = hits[0];
            let matched = content_lines[start..end].join("\n");
            // G-FZZ-001/083: post-match escape-drift against the matched region only.
            guard_escape_drift(old, new, &matched)?;
            let matched_slice = &content_lines[start..end];
            let mut adjusted_new = apply_indent_delta_block(matched_slice, new);
            let indent_adjusted = adjusted_new != new;
            adjusted_new = preserve_unicode_in_replacement(&matched, &adjusted_new);
            adjusted_new = maybe_unescape_new_string(&adjusted_new, &matched);
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
            check_cancel()?;
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
            crate::constants::FUZZY_TRIED_LINE_TRIMMED,
            match_line_trimmed as LineRangeFinder,
        ),
        (
            "whitespace_normalized",
            crate::constants::FUZZY_TRIED_WHITESPACE,
            match_whitespace_normalized,
        ),
        (
            "punctuation_normalized",
            crate::constants::FUZZY_TRIED_PUNCTUATION,
            match_punctuation_normalized,
        ),
        (
            "indent_flexible",
            crate::constants::FUZZY_TRIED_INDENT,
            match_indent_flexible,
        ),
        (
            "trimmed_boundary",
            crate::constants::FUZZY_TRIED_TRIMMED_BOUNDARY,
            match_trimmed_boundary,
        ),
    ] {
        let hits = collect_all(finder)?;
        check_cancel()?;
        if !hits.is_empty() {
            // GAP-2026-021: normalized exact-equivalent strategies are not byte-identical.
            // Emit similarity: null (not a fake 1.0).
            return apply_hits(hits, name, tried, None);
        }
    }

    // Strategy 6: escape-normalized (byte offsets) — G-FZZ-120 post-guards.
    if let Some((orig_start, orig_end)) = match_escape_normalized(content, old)? {
        let matched = &content[orig_start..orig_end];
        guard_escape_drift(old, new, matched)?;
        let mut adjusted_new = apply_indent_delta(matched.lines().next().unwrap_or(""), new);
        let indent_adjusted = adjusted_new != new;
        adjusted_new = preserve_unicode_in_replacement(matched, &adjusted_new);
        adjusted_new = maybe_unescape_new_string(&adjusted_new, matched);
        let edited = format!(
            "{}{}{}",
            &content[..orig_start],
            adjusted_new,
            &content[orig_end..]
        );
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "escape_normalized".into(),
                strategies_tried: crate::constants::FUZZY_TRIED_ESCAPE,
                similarity: None,
                diff_preview: mini_diff(old, matched),
                match_count: 1,
                indent_adjusted,
            },
        ));
    }

    // Strategy 8: block-anchor
    let min_ratio = adaptive_threshold(
        opts.threshold.unwrap_or(match opts.mode {
            FuzzyMode::Aggressive => opts.thr_aggressive,
            FuzzyMode::Auto | FuzzyMode::Off => opts.thr_auto,
        }),
        old.chars().count(),
    );
    if let Some((start, end, ratio)) = match_block_anchor(&content_lines, &old_lines, min_ratio)? {
        return apply_hits(
            vec![(start, end)],
            "block_anchor",
            crate::constants::FUZZY_TRIED_BLOCK_ANCHOR,
            Some(ratio),
        );
    }
    check_cancel()?;
    if let Some((start, end, ratio)) =
        match_block_anchor(&content_lines, &old_lines, BEST_CANDIDATE_MIN)?
    {
        let text = content_lines[start..end].join("\n");
        let (line, column) = line_start_position(start);
        consider_best(&mut best, &text, line, column, ratio, "block_anchor", old);
        push_candidate(&mut cands, &text, line, column, ratio, "block_anchor", old);
    }

    // Strategy 8b: unicode_normalized
    check_cancel()?;
    if let Some((start, end)) = match_unicode_normalized(&content_lines, &old_lines)? {
        return apply_hits(
            vec![(start, end)],
            "unicode_normalized",
            crate::constants::FUZZY_TRIED_UNICODE,
            None,
        );
    }
    check_cancel()?;

    // Strategy 9: context-aware
    if matches!(opts.mode, FuzzyMode::Aggressive | FuzzyMode::Auto) {
        let ctx_threshold = adaptive_threshold(
            opts.threshold.unwrap_or(opts.thr_context),
            old.chars().count(),
        );
        if old_lines.len() == 1 && old.len() < crate::constants::FUZZY_JW_MAX_PATTERN_BYTES {
            let jw_threshold =
                adaptive_threshold(opts.threshold.unwrap_or(opts.thr_jw), old.chars().count());
            let mut best_jw = (0usize, 0.0f64);
            let mut all_jw: Vec<(usize, f64)> = Vec::new();
            let mut jw_poll = CancelPoll::lines();
            for (i, line) in content_lines.iter().enumerate() {
                jw_poll.tick()?;
                let score = strsim::jaro_winkler(line.trim(), old.trim());
                if score >= jw_threshold {
                    all_jw.push((i, score));
                }
                if score > best_jw.1 {
                    best_jw = (i, score);
                }
            }
            // FAIL-7: only candidates that survive the dual gate are real
            // competitors; a pile of merely similar lines is a miss, not a tie.
            let dual_floor = adaptive_threshold(
                opts.threshold.unwrap_or(opts.thr_auto),
                old.chars().count(),
            );
            let needle = old.trim();
            let passes_dual_gate = |i: usize, score: f64| {
                let cand = content_lines[i].trim();
                score >= crate::constants::FUZZY_DUAL_GATE_NEAR_EXACT
                    || (strsim::normalized_damerau_levenshtein(cand, needle) >= dual_floor
                        && gestalt_ratio(cand, needle) >= dual_floor
                        && line_vote_ratio(cand, needle) >= dual_floor)
            };
            let passing_jw: Vec<(usize, f64)> = all_jw
                .iter()
                .copied()
                .filter(|&(i, score)| passes_dual_gate(i, score))
                .collect();
            if !all_jw.is_empty() {
                if passing_jw.len() > 1 && !opts.replace_all {
                    let (i0, score0) = passing_jw[0];
                    let (line, column) = line_start_position(i0);
                    let bc = BestCandidate {
                        text: Some(truncate_text(content_lines.get(i0).copied().unwrap_or(""))),
                        line: Some(line),
                        column: Some(column),
                        similarity: Some(score0),
                        strategy: Some("context_aware_jw".into()),
                        diff_preview: None,
                    };
                    return Err(ambiguous_err(old, passing_jw.len() as u64, Some(bc)));
                }
                // G-FZZ-012/121: pick argmax JW, not first hit in scan order.
                let pool = if passing_jw.is_empty() { &all_jw } else { &passing_jw };
                let (i, score) = pool
                    .iter()
                    .copied()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(pool[0]);
                // G-FZZ-145: dual-gate — reject high JW when Damerau/gestalt/line-vote weak.
                if !passes_dual_gate(i, score) {
                    let (line, column) = line_start_position(i);
                    consider_best(
                        &mut best,
                        content_lines.get(i).copied().unwrap_or(""),
                        line,
                        column,
                        score,
                        "context_aware_jw",
                        old,
                    );
                    // fall through to context_aware / rank rather than FP replace
                } else {
                    return apply_hits(
                        vec![(i, i + 1)],
                        "context_aware_jw",
                        crate::constants::FUZZY_TRIED_CONTEXT_AWARE,
                        Some(score),
                    );
                }
            }
            if best_jw.1 > 0.0 {
                let (line, column) = line_start_position(best_jw.0);
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
        if let Some((start, end, similarity)) = match_context_aware(
            &content_lines,
            &old_lines,
            ctx_threshold,
            opts.soft_floor_delta,
        )? {
            return apply_hits(
                vec![(start, end)],
                "context_aware",
                crate::constants::FUZZY_TRIED_CONTEXT_AWARE,
                Some(similarity),
            );
        }
        check_cancel()?;
        rank_into(&content_lines, &old_lines, old, &mut best, &mut cands)?;
    }

    // A deadline that fired during the last strategy must never read as a miss.
    check_cancel()?;
    let reason = format!("old string not found after fuzzy cascade (strategies tried): {old:?}");
    let best = best.or_else(|| cands.first().cloned()).map(Box::new);
    Err(AtomwriteError::MatchFailed {
        reason,
        best_candidate: best,
        candidates: if cands.len() > 1 { Some(cands) } else { None },
    })
}

// ─── matching strategies ─────────────────────────────────────────────────────
