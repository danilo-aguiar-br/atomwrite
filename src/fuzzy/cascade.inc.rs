/// Resolve `old` → `new` with default uniqueness (no `replace_all`).
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


/// Resolve pair using full XDG [`FuzzySection`] thresholds (R-THR wire).
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

    // G-010: Off stops after exact — no fuzzy cascade.
    if matches!(opts.mode, FuzzyMode::Off) {
        return Err(AtomwriteError::MatchFailed {
            reason: format!(
                "old string not found (fuzzy mode off = exact-only): {old:?}"
            ),
            best_candidate: None,
            candidates: None,
        });
    }

    // G-048: fuzzy cascade only — cap pattern size (exact already handled above).
    if old.len() > crate::constants::FUZZY_MAX_PATTERN_BYTES {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "fuzzy pattern too large ({} bytes > {} max); shorten the block or use a smaller unique slice for fuzzy match",
                old.len(),
                crate::constants::FUZZY_MAX_PATTERN_BYTES
            ),
        });
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let content_lines: Vec<&str> = content.lines().collect();
    let mut best: Option<BestCandidate> = None;
    let mut cands: Vec<BestCandidate> = Vec::new();

    // Collect all ranges for a strategy
    type LineRangeFinder = fn(&[&str], &[&str]) -> Option<(usize, usize)>;
    let collect_all = |finder: LineRangeFinder| -> Vec<(usize, usize)> {
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
                i += e;
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

    // Strategy 6: escape-normalized (byte offsets) — G-FZZ-120 post-guards.
    if let Some((orig_start, orig_end)) = match_escape_normalized(content, old) {
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
                strategies_tried: 6,
                similarity: Some(1.0),
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
        return apply_hits(vec![(start, end)], "unicode_normalized", 10, Some(1.0));
    }

    // Strategy 9: context-aware
    if matches!(opts.mode, FuzzyMode::Aggressive | FuzzyMode::Auto) {
        let ctx_threshold = adaptive_threshold(
            opts.threshold.unwrap_or(opts.thr_context),
            old.chars().count(),
        );
        if old_lines.len() == 1 && old.len() < 60 {
            let jw_threshold = adaptive_threshold(
                opts.threshold.unwrap_or(opts.thr_jw),
                old.chars().count(),
            );
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
                // G-FZZ-012/121: pick argmax JW, not first hit in scan order.
                let (i, score) = all_jw
                    .iter()
                    .copied()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(all_jw[0]);
                // G-FZZ-145: dual-gate — reject high JW when Damerau/gestalt/line-vote weak.
                let cand = content_lines[i].trim();
                let needle = old.trim();
                let edit_score = strsim::normalized_damerau_levenshtein(cand, needle);
                let gestalt = gestalt_ratio(cand, needle);
                let line_vote = line_vote_ratio(cand, needle);
                let dual_floor = adaptive_threshold(
                    opts.threshold.unwrap_or(opts.thr_auto),
                    old.chars().count(),
                );
                if (edit_score < dual_floor || gestalt < dual_floor || line_vote < dual_floor)
                    && score < crate::constants::FUZZY_DUAL_GATE_NEAR_EXACT
                {
                    let off = byte_offset_of_line(content, i);
                    let (line, column) = line_col_of_offset(content, off);
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
                    return apply_hits(vec![(i, i + 1)], "context_aware_jw", 9, Some(score));
                }
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


