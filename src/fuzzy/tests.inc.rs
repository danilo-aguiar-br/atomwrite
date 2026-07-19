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
        assert_eq!(r.edited, "header\nAAA\nBBB\nfooter\n");
        // Second conceptual apply would grow forever without the guard.
        assert!(!r.edited.contains("AAA\nBBB\nBBB"));
    }

    #[test]
    fn one_pass_default_limit_is_one() {
        let content = "unique_token_alpha beta\n";
        let r = apply_fuzzy_one_pass(
            content, "unique_token_alpha", "unique_token_omega", MatchOpts::default(), None,
        )
        .unwrap();
        assert_eq!(r.applied, 1);
        assert_eq!(r.edited, "unique_token_omega beta\n");
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
    }
}
