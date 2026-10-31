// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared language utilities for the AST commands (`scope`, `transform`).
//!
//! The extension table lives in the grammar crate (`ast-grep-language`), and it
//! is the only table: a local copy drifted from it and left languages the
//! grammar knows (`haskell`, `dart`, `php`) with no extension at all, which
//! turned the walker filter into a no-op and rewrote `.rb` files with the
//! Haskell grammar.

use std::path::{Path, PathBuf};

use ast_grep_core::Language as _;
use ast_grep_language::SupportLang;

use crate::error::AtomwriteError;

/// File filter for `lang`, taken from the grammar itself.
///
/// Every [`SupportLang`] carries at least one extension, so the returned filter
/// is never empty and the walker never runs unfiltered.
#[must_use]
pub fn file_types(lang: SupportLang) -> ignore::types::Types {
    lang.file_types()
}

/// Every language name the CLI accepts, in stable order, joined for a message.
///
/// The list comes from the grammar crate's own table, so a language the parser
/// gained is never missing from the hint and a removed one never lingers.
#[must_use]
pub fn accepted_language_names() -> String {
    SupportLang::all_langs()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse `name` as a language, naming the accepted ones when it fails.
///
/// An unknown language is a typo far more often than a missing feature, so the
/// refusal lists what is accepted instead of only echoing the wrong word.
///
/// # Errors
///
/// Returns [`AtomwriteError::InvalidInput`] naming every accepted language.
pub fn parse_language(name: &str) -> Result<SupportLang, AtomwriteError> {
    name.parse::<SupportLang>()
        .map_err(|_| AtomwriteError::InvalidInput {
            reason: format!(
                "unsupported language: {name}; accepted: {}",
                accepted_language_names()
            ),
        })
}

/// Refuse a target whose extension does not belong to `lang`.
///
/// A file named directly on the command line skips the walker's extension
/// filter, so the mismatch has to be refused here: rewriting a Ruby file with
/// the Haskell grammar produces nonsense and reports success.
///
/// # Errors
///
/// Returns [`AtomwriteError::InvalidInput`] when the extension names another
/// language, or when it names no language at all.
pub fn ensure_target_matches(path: &Path, lang: SupportLang) -> Result<(), AtomwriteError> {
    match SupportLang::from_path(path) {
        Some(found) if found == lang => Ok(()),
        Some(found) => Err(AtomwriteError::InvalidInput {
            reason: format!(
                "{} is {found} source, not {lang}; pass -l {found} or name files of the requested language [{}]",
                path.display(),
                crate::constants::LANGUAGE_MISMATCH_REASON_MARKER
            ),
        }),
        None => Err(AtomwriteError::InvalidInput {
            reason: format!(
                "{} has no extension that names a language; name a file whose extension identifies {lang} [{}]",
                path.display(),
                crate::constants::LANGUAGE_MISMATCH_REASON_MARKER
            ),
        }),
    }
}

/// Refuse every named target whose extension is not `lang`.
///
/// Directories are left alone: the walker filters them by
/// [`SupportLang::file_types`], so a tree mixing languages is narrowed instead
/// of refused. Only a file named on the command line is checked, because it
/// reaches the parser with no filter in between.
///
/// # Errors
///
/// Returns the first [`AtomwriteError::InvalidInput`] produced by
/// [`ensure_target_matches`].
pub fn ensure_targets_match(paths: &[PathBuf], lang: SupportLang) -> Result<(), AtomwriteError> {
    for path in paths {
        if path.is_file() {
            ensure_target_matches(path, lang)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_language_own_extension() {
        ensure_target_matches(Path::new("a.rs"), SupportLang::Rust).expect("rust source");
        ensure_target_matches(Path::new("a.rb"), SupportLang::Ruby).expect("ruby source");
    }

    #[test]
    fn refuses_another_language_and_names_it() {
        let err = ensure_target_matches(Path::new("a.rb"), SupportLang::Haskell)
            .expect_err("ruby is not haskell");
        let text = err.to_string();
        assert!(text.contains("Ruby"), "{text}");
        assert!(text.contains("Haskell"), "{text}");
    }

    #[test]
    fn refuses_a_file_without_a_language_extension() {
        ensure_target_matches(Path::new("Makefile"), SupportLang::Rust)
            .expect_err("no extension names a language");
    }

    #[test]
    fn names_every_language_it_accepts() {
        let names = accepted_language_names();
        assert!(names.contains("Rust"), "{names}");
        assert!(names.contains("Haskell"), "{names}");
        assert!(names.split(", ").count() >= 28, "{names}");
    }

    #[test]
    fn a_typo_lists_the_accepted_languages() {
        let err = parse_language("ruust").expect_err("typo");
        let text = err.to_string();
        assert!(text.contains("unsupported language: ruust"), "{text}");
        assert!(text.contains("Rust"), "{text}");
    }

    #[test]
    fn the_filter_matches_the_language_own_suffix() {
        let types = file_types(SupportLang::Haskell);
        assert!(
            !matches!(
                types.matched(Path::new("Main.hs"), false),
                ignore::Match::None
            ),
            "the haskell filter must accept .hs"
        );
        assert!(
            matches!(
                types.matched(Path::new("a.rb"), false),
                ignore::Match::Ignore(_)
            ),
            "the haskell filter must reject .rb"
        );
    }
}
