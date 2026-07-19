fn edit_old_new(
    original: &str,
    old: &[String],
    new: &[String],
    opts: crate::fuzzy::MatchOpts,
    partial: bool,
) -> Result<(String, String, FuzzyInfo, Option<MultiReport>)> {
    if old.len() > 1 {
        return edit_old_new_multi(original, old, new, opts, partial);
    }
    let old_str = &old[0];
    let new_str = new.first().map(|s| s.as_str()).unwrap_or("");
    match crate::fuzzy::match_pair_with(original, old_str, new_str, opts) {
        Ok((edited, info)) => {
            let mode = if info.strategy == "exact" {
                "exact".into()
            } else {
                "old_new".into()
            };
            Ok((edited, mode, info, None))
        }
        Err(_) if partial => Err(AtomwriteError::NoMatches.into()),
        Err(err) => Err(err.into()),
    }
}

// Fuzzy cascade lives in `crate::fuzzy` (v0.1.29 P0-1).
/// Re-export for property tests (GAP-086) and historical `commands::edit::match_pair`.
pub use crate::fuzzy::match_pair;

fn edit_old_new_multi(
    original: &str,
    old: &[String],
    new: &[String],
    opts: crate::fuzzy::MatchOpts,
    partial: bool,
) -> Result<(String, String, FuzzyInfo, Option<MultiReport>)> {
    let pairs_total = old.len() as u64;
    let mut content = original.to_string();
    let mut pair_results: Vec<PairResult> = Vec::with_capacity(old.len());
    let mut applied = 0u64;
    let mut any_fuzzy = false;
    let mut max_strategies_tried = 0u64;

    for (i, (old_str, new_str)) in old.iter().zip(new.iter()).enumerate() {
        let index = (i + 1) as u64;
        match crate::fuzzy::match_pair_with(&content, old_str, new_str, opts) {
            Ok((edited, info)) => {
                content = edited;
                applied += 1;
                any_fuzzy |= info.fuzzy;
                max_strategies_tried = max_strategies_tried.max(info.strategies_tried);
                pair_results.push(PairResult {
                    index,
                    matched: true,
                    strategy: Some(info.strategy),
                    similarity: info.similarity,
                    source: None,
                });
            }
            Err(_) if partial => {
                pair_results.push(PairResult {
                    index,
                    matched: false,
                    strategy: None,
                    similarity: None,
                    source: None,
                });
            }
            Err(err) => {
                let (reason, best_candidate) = match err {
                    AtomwriteError::MatchFailed {
                        reason,
                        best_candidate,
                        ..
                    } => (reason, best_candidate),
                    AtomwriteError::MatchAmbiguous {
                        reason,
                        best_candidate,
                        ..
                    } => (reason, best_candidate),
                    AtomwriteError::InvalidInput { reason } => (reason, None),
                    other => (other.to_string(), None),
                };
                pair_results.push(PairResult {
                    index,
                    matched: false,
                    strategy: None,
                    similarity: None,
                    source: None,
                });
                return Err(AtomwriteError::EditPairFailed {
                    index,
                    total: pairs_total,
                    reason,
                    pair_results: Box::new(pair_results),
                    best_candidate,
                }
                .into());
            }
        }
    }

    if applied == 0 {
        return Err(AtomwriteError::NoMatches.into());
    }

    let (mode, strategy) = if any_fuzzy {
        (format!("fuzzy-multi({applied})"), "fuzzy-multi")
    } else {
        (format!("exact-multi({applied})"), "exact-multi")
    };
    Ok((
        content,
        mode,
        FuzzyInfo {
            fuzzy: any_fuzzy,
            strategy: strategy.into(),
            strategies_tried: max_strategies_tried,
            similarity: None,
            diff_preview: None,
            match_count: applied,
            indent_adjusted: false,
        },
        Some(MultiReport {
            pair_results,
            pairs_total,
            applied,
        }),
    ))
}

// ─── line-based and marker-based edits (unchanged) ───────────────────────────

fn edit_by_line(
    lines: &[&str],
    args: &EditArgs,
    stdin: impl Read,
    max_size: u64,
    stdin_is_tty: bool,
) -> Result<(String, String)> {
    let mut result_lines = lines_to_owned(lines);

    if let Some(n) = args.after_line {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "after-line")?;
        let idx = validate_line_num(n, lines.len())?;
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result_lines.insert(idx + i + 1, line);
        }
        return Ok((join_lines(&result_lines), "after_line".into()));
    }

    if let Some(n) = args.before_line {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "before-line")?;
        let idx = validate_line_num(n, lines.len())?;
        let insert_at = if idx == 0 { 0 } else { idx };
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result_lines.insert(insert_at + i, line);
        }
        return Ok((join_lines(&result_lines), "before_line".into()));
    }

    if let Some(ref range_str) = args.range {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "range")?;
        let (start, end) = parse_range(range_str, lines.len())?;
        let new_lines = lines_from_str(&content);
        result_lines.splice(start..end, new_lines);
        return Ok((join_lines(&result_lines), "replace_range".into()));
    }

    if let Some(ref range_str) = args.delete_range {
        let (start, end) = parse_range(range_str, lines.len())?;
        result_lines.drain(start..end);
        return Ok((join_lines(&result_lines), "delete_range".into()));
    }

    Err(crate::error::AtomwriteError::InvalidInput {
        reason: "no line-mode edit operation specified".into(),
    }
    .into())
}

fn edit_by_marker(
    original: &str,
    lines: &[&str],
    args: &EditArgs,
    stdin: impl Read,
    max_size: u64,
    stdin_is_tty: bool,
) -> Result<(String, String)> {
    if let Some(ref marker) = args.after_match {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "after-match")?;
        let idx = find_line_with(lines, marker)?;
        let mut result = lines_to_owned(lines);
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result.insert(idx + 1 + i, line);
        }
        return Ok((join_lines(&result), "after_match".into()));
    }

    if let Some(ref marker) = args.before_match {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "before-match")?;
        let idx = find_line_with(lines, marker)?;
        let mut result = lines_to_owned(lines);
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result.insert(idx + i, line);
        }
        return Ok((join_lines(&result), "before_match".into()));
    }

    if let Some(ref markers) = args.between {
        if markers.len() != 2 {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: "--between requires exactly 2 markers".into(),
            }
            .into());
        }
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "between")?;
        let start_idx = find_line_with(lines, &markers[0])?;
        let end_idx = find_line_with_after(lines, &markers[1], start_idx + 1)?;

        let mut result = lines_to_owned(lines);
        let new_lines = lines_from_str(&content);
        result.splice((start_idx + 1)..end_idx, new_lines);
        return Ok((join_lines(&result), "between".into()));
    }

    let _ = original;
    Err(crate::error::AtomwriteError::InvalidInput {
        reason: "no marker-mode edit operation specified".into(),
    }
    .into())
}

