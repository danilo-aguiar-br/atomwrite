// SPDX-License-Identifier: MIT OR Apache-2.0

//! Claim readers of the `doctor` document guards.
//!
//! The guards compare what the live documents say about the tree against what
//! the tree actually contains. Reading the claim lives here so the doctor does
//! not grow a second parser per claim, and so the two readers share the rule
//! that a line without a recognized phrase returns `None` instead of a guess.

/// Test-target count stated by one line, if any.
///
/// Recognizes the phrasings the documents use: `N test targets`,
/// `N targets`, `N alvos de teste` and `N alvos`. A bare number is never a
/// claim, because line ranges and gap identifiers also carry numbers.
pub(crate) fn target_count_in(line: &str) -> Option<usize> {
    for phrase in crate::constants::DOCS_TARGET_PHRASES {
        let Some(at) = line.find(phrase) else {
            continue;
        };
        let before = line.get(..at)?.trim_end();
        let digits: String = before
            .chars()
            .rev()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(n) = digits.parse::<usize>() {
            return Some(n);
        }
    }
    None
}

/// Config-key count stated by one line, if any.
///
/// Recognizes `N keys` and `N chaves`. The word that follows the number is what
/// separates a key claim from an unrelated count on the same document.
pub(crate) fn key_count_in(line: &str) -> Option<usize> {
    for phrase in crate::constants::DOCS_KEY_PHRASES {
        let Some(at) = line.find(phrase) else {
            continue;
        };
        let before = line.get(..at)?.trim_end();
        let digits: String = before
            .chars()
            .rev()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(n) = digits.parse::<usize>() {
            return Some(n);
        }
    }
    None
}
