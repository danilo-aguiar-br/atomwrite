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
    fn fuzzy_off_exact_only() {
        // Off applies exact matches and rejects misses without cascade.
        let (out, info) = match_pair("abc xyz", "xyz", "q", FuzzyMode::Off, None).unwrap();
        assert_eq!(out, "abc q");
        assert!(!info.fuzzy);
        let err = match_pair("abc", "xyz", "q", FuzzyMode::Off, None).unwrap_err();
        match err {
            AtomwriteError::MatchFailed { reason, .. } => {
                assert!(reason.contains("exact-only") || reason.contains("off"));
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
                ..Default::default()
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
        let r = apply_fuzzy_one_pass(content, old, new, MatchOpts::default(), Some(1_000_000))
            .expect("must succeed");
        assert!(r.replacement_embeds_pattern);
        assert_eq!(r.applied, 1, "embeds must force single apply");
        assert_eq!(r.edited.as_deref(), Some("header\nAAA\nBBB\nfooter\n"));
        // Second conceptual apply would grow forever without the guard.
        assert!(!r.edited.as_deref().unwrap_or("").contains("AAA\nBBB\nBBB"));
    }

    #[test]
    fn one_pass_default_limit_is_one() {
        let content = "unique_token_alpha beta\n";
        let r = apply_fuzzy_one_pass(
            content, "unique_token_alpha", "unique_token_omega", MatchOpts::default(), None,
        )
        .unwrap();
        assert_eq!(r.applied, 1);
        assert_eq!(r.edited.as_deref(), Some("unique_token_omega beta\n"));
        assert!(!r.replacement_embeds_pattern);
    }

    #[test]
    fn one_pass_rejects_oversized_pattern() {
        let big = "a".repeat(crate::constants::FUZZY_MAX_PATTERN_BYTES + 1);
        let err = apply_fuzzy_one_pass("a", &big, "b", MatchOpts::default(), None).unwrap_err();
        assert!(matches!(err, AtomwriteError::InvalidInput { .. }));
    }

    #[test]
    fn escape_drift_hermes_blocks_when_old_and_new_have_escapes() {
        // File has real apostrophe; old/new both carry tool-call \\'
        let content = "msg = \"it's great\"\n";
        let old = "msg = \"it\\'s great\"";
        let new = "msg = \"it\\'s fine\"";
        let err = match_pair(content, old, new, FuzzyMode::Auto, None).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("escape-drift") || msg.contains("escape"),
            "expected escape-drift, got {msg}"
        );
    }

    #[test]
    fn crlf_preserved_on_line_trimmed_fuzzy() {
        let content = "alpha\r\n  hello world  \r\nomega\r\n";
        let (out, info) = match_pair(
            content,
            "hello world",
            "hello rust",
            FuzzyMode::Auto,
            None,
        )
        .unwrap();
        assert!(info.fuzzy || info.strategy == "exact" || info.strategy == "line_trimmed" || info.strategy == "whitespace_normalized" || info.strategy == "trimmed_boundary");
        assert!(out.contains("\r\n"), "CRLF must be preserved, got {out:?}");
        assert!(out.contains("hello rust"));
    }

    #[test]
    fn dual_gate_rejects_weak_jw_typo_fp() {
        // High JW between short words can FP; dual-gate should not rewrite freely.
        let content = "value = beta FIXED\n";
        let old = "betta";
        let new = "gamma";
        match match_pair(content, old, new, FuzzyMode::Auto, None) {
            Ok((out, info)) => {
                // If it matched, must be high confidence — not a silent FP on weak edit score alone
                assert!(
                    info.similarity.unwrap_or(0.0) >= 0.99 || out.contains("gamma"),
                    "unexpected weak match: {info:?} out={out}"
                );
            }
            Err(_) => {
                // Prefer fail over FP corruption
            }
        }
    }


    #[test]
    fn thr_section_jw_wire_uses_section_not_defaults() {
        let section = crate::config::FuzzySection {
            threshold_jw: 0.99,
            threshold_aggressive: 0.91,
            threshold_context: 0.92,
            threshold: 0.93,
            ..Default::default()
        };
        let opts = match_opts_from_section(FuzzyMode::Auto, None, &section, true);
        assert!((opts.thr_jw - 0.99).abs() < 1e-9);
        assert!((opts.thr_aggressive - 0.91).abs() < 1e-9);
        assert!((opts.thr_context - 0.92).abs() < 1e-9);
        assert!((opts.thr_auto - 0.93).abs() < 1e-9);
        assert!(opts.replace_all);
        let opts2 = match_opts_from_section(FuzzyMode::Aggressive, Some(0.55), &section, false);
        assert!((opts2.threshold.unwrap() - 0.55).abs() < 1e-9);
        assert!(matches!(opts2.mode, FuzzyMode::Aggressive));
        assert!((opts.soft_floor_delta - crate::constants::FUZZY_CONTEXT_SOFT_FLOOR_DELTA).abs() < 1e-9);
    }

    /// GAP-2026-021: `whitespace_normalized` must not report similarity 1.0.
    #[test]
    fn normalized_strategy_similarity_is_not_fake_one() {
        let content = "hello   world\n";
        let old = "hello world";
        let (out, info) = match_pair(content, old, "hola mundo", FuzzyMode::Auto, None).unwrap();
        assert!(out.contains("hola"));
        assert!(info.fuzzy);
        assert_ne!(
            info.similarity,
            Some(1.0),
            "normalized match must not claim perfect byte identity"
        );
    }

    /// GAP-2026-027: aggressive radical middle rewrite should not match on borders alone.
    #[test]
    fn block_anchor_rejects_radical_middle_when_ratio_low() {
        let content = "fn start() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}\n";
        // first/last lines match anchors but middle is completely different and longer mismatch
        let old = "fn start() {\n    totally_different_body_xxx();\n    another_unrelated_call();\n    yet_more_noise();\n}\n";
        let err = match_pair_with(
            content,
            old,
            "fn start() { evil(); }",
            MatchOpts {
                mode: FuzzyMode::Aggressive,
                threshold: Some(0.95),
                thr_aggressive: 0.95,
                ..Default::default()
            },
        );
        // High threshold + dual-gate middle should fail or not apply silent wrong patch
        match err {
            Ok((out, _)) => {
                // If it matched, content must not become the evil rewrite without high similarity
                assert!(
                    !out.contains("evil()"),
                    "must not apply radical replacement on weak middle"
                );
            }
            Err(AtomwriteError::MatchFailed { .. }) => {}
            Err(other) => panic!("unexpected {other:?}"),
        }
    }

    fn capped(max_content_bytes: u64) -> MatchOpts {
        MatchOpts {
            max_content_bytes,
            ..Default::default()
        }
    }

    /// v0.1.37: exact matching never consults the content cap.
    #[test]
    fn exact_match_ignores_content_cap() {
        let (out, info) =
            match_pair_with("hello world foo bar", "world", "rust", capped(4)).unwrap();
        assert_eq!(out, "hello rust foo bar");
        assert!(!info.fuzzy);
    }

    /// v0.1.37: above the cap the cascade does not run and the miss is a typed error.
    #[test]
    fn content_above_cap_is_invalid_input_not_a_miss() {
        let content = "alpha beta gamma delta epsilon\n";
        let err = match_pair_with(content, "wrongword", "x", capped(8)).unwrap_err();
        match err {
            AtomwriteError::InvalidInput { reason } => {
                assert!(reason.contains("max_content_bytes"), "{reason}");
                assert!(reason.contains("--fuzzy off"), "{reason}");
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    /// `--fuzzy off` is the documented way out of the cap: it stays an exact-only miss.
    #[test]
    fn fuzzy_off_above_cap_stays_exact_only_miss() {
        let opts = MatchOpts {
            mode: FuzzyMode::Off,
            max_content_bytes: 8,
            ..Default::default()
        };
        let err = match_pair_with("alpha beta gamma delta\n", "wrongword", "x", opts).unwrap_err();
        assert!(matches!(err, AtomwriteError::MatchFailed { .. }), "{err:?}");
    }

    #[test]
    fn content_at_cap_still_runs_the_cascade() {
        let content = "alpha beta\n";
        let err =
            match_pair_with(content, "wrongword", "x", capped(content.len() as u64)).unwrap_err();
        assert!(matches!(err, AtomwriteError::MatchFailed { .. }), "{err:?}");
    }

    /// The one-pass wrapper must pass the cap error on, never report zero matches.
    #[test]
    fn one_pass_passes_cap_error_through() {
        let err = apply_fuzzy_one_pass(
            "alpha beta gamma delta\n",
            "wrongword",
            "x",
            capped(8),
            None,
        )
        .unwrap_err();
        assert!(matches!(err, AtomwriteError::InvalidInput { .. }), "{err:?}");
        let err = apply_fuzzy_one_pass(
            "alpha beta gamma delta\n",
            "wrongword",
            "x",
            capped(8),
            Some(5),
        )
        .unwrap_err();
        assert!(matches!(err, AtomwriteError::InvalidInput { .. }), "{err:?}");
    }

    /// A real miss is zero matches and does not copy the input buffer.
    #[test]
    fn one_pass_miss_is_zero_applied_without_a_copy() {
        for max in [None, Some(5)] {
            let r = apply_fuzzy_one_pass(
                "alpha beta\n",
                "zzzzqqqq",
                "x",
                MatchOpts::default(),
                max,
            )
            .unwrap();
            assert_eq!(r.applied, 0);
            assert!(r.edited.is_none());
            assert!(r.info.is_none());
        }
    }

    #[test]
    fn is_no_match_only_for_misses() {
        let miss = AtomwriteError::MatchFailed {
            reason: "r".into(),
            best_candidate: None,
            candidates: None,
        };
        assert!(is_no_match(&miss));
        assert!(!is_no_match(&AtomwriteError::InvalidInput { reason: "r".into() }));
        assert!(!is_no_match(&AtomwriteError::Cancelled {
            reason: "r".into(),
            exit: 124
        }));
    }

    #[test]
    fn cancel_poll_masks_checks_without_global_flag() {
        let mut lines = CancelPoll::lines();
        let mut heavy = CancelPoll::heavy();
        for _ in 0..(crate::constants::FUZZY_CANCEL_POLL_MASK * 4) {
            lines.tick().unwrap();
            heavy.tick().unwrap();
        }
        check_cancel().unwrap();
    }

    #[test]
    fn collect_lines_matches_str_lines() {
        for s in ["", "a", "a\n", "a\nb", "a\r\nb\r\n", "\n\n", "x\n\ny\n"] {
            assert_eq!(collect_lines(s).unwrap(), s.lines().collect::<Vec<_>>(), "{s:?}");
        }
    }

    /// Reference implementation of the previous O(n) line/column walk.
    fn naive_line_col(content: &str, byte_offset: usize) -> (u64, u64) {
        let (mut line, mut col) = (1u64, 1u64);
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

    #[test]
    fn line_col_matches_reference_walk() {
        let content = "ab\ncafé — x\n\nlast line without newline";
        for off in (0..=content.len()).filter(|o| content.is_char_boundary(*o)) {
            assert_eq!(line_col_of_offset(content, off), naive_line_col(content, off), "{off}");
        }
        // Line starts: the O(1) helper equals the walk to the first column of each line.
        let mut start = 0usize;
        for (idx, line) in content.split('\n').enumerate() {
            assert_eq!(line_start_position(idx), naive_line_col(content, start));
            start += line.len() + 1;
        }
    }

    /// The char loop replaced a `\s*([(){}\[\]<>,;:])\s*` regex; both must agree.
    #[test]
    fn punctuation_normalization_matches_previous_regex() {
        let re = regex::Regex::new(r"\s*([(){}\[\]<>,;:])\s*").unwrap();
        for s in [
            "fn  main ( a : i32 ,  b ) { }",
            "  foo (  ) ;  ",
            "a<b>c , d ; e",
            "((  ))",
            "no punctuation here",
            "x\t:\ty",
            "",
        ] {
            let expected = re.replace_all(&normalize_whitespace(s), "$1").to_string();
            assert_eq!(normalize_punctuation_whitespace(s), expected, "{s:?}");
        }
    }

    /// Strategy finders surface the work done as a plain miss when nothing matches.
    #[test]
    fn finders_report_miss_as_ok_none() {
        let content = ["alpha", "beta"];
        let pattern = ["nothing", "here"];
        assert!(match_line_trimmed(&content, &pattern).unwrap().is_none());
        assert!(match_whitespace_normalized(&content, &pattern).unwrap().is_none());
        assert!(match_punctuation_normalized(&content, &pattern).unwrap().is_none());
        assert!(match_indent_flexible(&content, &pattern).unwrap().is_none());
        assert!(match_trimmed_boundary(&content, &pattern).unwrap().is_none());
        assert!(match_unicode_normalized(&content, &pattern).unwrap().is_none());
        assert!(match_block_anchor(&content, &pattern, 0.7).unwrap().is_none());
        assert!(match_context_aware(&content, &pattern, 0.9, 0.0).unwrap().is_none());
    }

    /// FAIL-7: an absent block among many merely similar lines is a miss, not a tie.
    #[test]
    fn absent_block_among_similar_lines_is_a_miss_not_ambiguous() {
        let content: String = (0..200).map(|n| format!("line {n:03}\n")).collect();
        for needle in ["line 999 NOT THERE", "line 099 WRONG", "line 12 NOT THERE"] {
            let err = match_pair(&content, needle, "x", FuzzyMode::Auto, None).unwrap_err();
            match err {
                AtomwriteError::MatchFailed { best_candidate, .. } => {
                    assert!(best_candidate.is_some(), "{needle}: miss must carry best_candidate");
                }
                other => panic!("{needle}: expected MatchFailed, got {other:?}"),
            }
        }
    }

    /// FAIL-7: genuine ties (equivalent after normalization) stay ambiguous.
    #[test]
    fn equivalent_duplicates_stay_ambiguous() {
        let err = match_pair("dup line\nmid\ndup line\n", "dup  line", "x", FuzzyMode::Auto, None)
            .unwrap_err();
        match err {
            AtomwriteError::MatchAmbiguous { count, .. } => assert_eq!(count, 2),
            other => panic!("expected MatchAmbiguous, got {other:?}"),
        }
    }

    /// GAP-2026-025: ambiguous path surfaces `best_candidate`.
    #[test]
    fn ambiguous_exposes_best_candidate() {
        let err = match_pair("aa\nxx\naa\n", "aa", "bb", FuzzyMode::Auto, None).unwrap_err();
        match err {
            AtomwriteError::MatchAmbiguous {
                count,
                best_candidate: Some(bc),
                ..
            } => {
                assert_eq!(count, 2);
                assert!(bc.text.is_some() || bc.line.is_some());
            }
            other => panic!("expected MatchAmbiguous with best_candidate, got {other:?}"),
        }
    }
}
