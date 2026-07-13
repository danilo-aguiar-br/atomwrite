// SPDX-License-Identifier: MIT OR Apache-2.0

//! GAP-086: property-based tests for the fuzzy-matching cascade in
//! `atomwrite::commands::edit::match_pair`.

use atomwrite::{FuzzyMode, match_pair};
use proptest::prelude::*;

proptest! {
    // Property 1: a unique exact substring is always resolved with strategy "exact".
    #[test]
    fn exact_substring_always_matches(
        prefix in "[a-zA-Z0-9 ]{0,50}",
        target in "[a-zA-Z0-9]{1,30}",
        suffix in "[a-zA-Z0-9 ]{0,50}",
        replacement in "[a-zA-Z0-9]{1,20}",
    ) {
        // Guard uniqueness (v0.1.30): skip when target appears more than once.
        let content = format!("{prefix}{target}{suffix}");
        if content.matches(&target).count() != 1 {
            return Ok(());
        }
        let result = match_pair(&content, &target, &replacement, FuzzyMode::Auto, None);
        prop_assert!(result.is_ok(), "unique exact substring must always match");
        let (output, info) = result.unwrap();
        prop_assert_eq!(info.strategy, "exact");
        prop_assert!(!info.fuzzy);
        prop_assert!(output.contains(&replacement));
    }

    // Property 2: arbitrary Unicode input never panics.
    #[test]
    fn no_panic_on_arbitrary_input(
        content in "\\PC{0,200}",
        old in "\\PC{0,100}",
        new_text in "\\PC{0,100}",
    ) {
        let _ = match_pair(&content, &old, &new_text, FuzzyMode::Auto, None);
        // Just verifying no panic occurs.
    }

    // Property 3: empty `old` is rejected (v0.1.30) without panic.
    #[test]
    fn empty_old_inserts_at_start(
        content in "[a-zA-Z]{1,50}",
        new_text in "[a-zA-Z]{1,20}",
    ) {
        let result = match_pair(&content, "", &new_text, FuzzyMode::Auto, None);
        prop_assert!(result.is_err(), "empty old must be rejected");
    }

    // Property 4: threshold 1.0 only accepts near-exact fuzzy matches.
    #[test]
    fn threshold_one_requires_exact(
        content in "[a-z]{10,50}",
        old in "[a-z]{5,15}",
        new_text in "[a-z]{1,10}",
    ) {
        let result = match_pair(&content, &old, &new_text, FuzzyMode::Auto, Some(1.0));
        if let Ok((_, info)) = &result {
            // If it matched fuzzily, the similarity must be near 1.0.
            if info.fuzzy {
                prop_assert!(info.similarity.unwrap_or(0.0) >= 0.99,
                    "threshold 1.0 should only match near-exact");
            }
        }
    }

    // Property 5: a successful match always reports positive similarity (when set).
    #[test]
    fn successful_match_has_positive_similarity(
        prefix in "[a-z ]{0,30}",
        target in "[a-z]{3,20}",
        suffix in "[a-z ]{0,30}",
        new_text in "[a-z]{1,10}",
    ) {
        let content = format!("{prefix}{target}{suffix}");
        let result = match_pair(&content, &target, &new_text, FuzzyMode::Auto, None);
        if let Ok((_, info)) = result
            && let Some(sim) = info.similarity
        {
            prop_assert!(sim > 0.0, "successful match must have positive similarity");
        }
    }
}
