// SPDX-License-Identifier: MIT OR Apache-2.0

//! Loading a tree-sitter parser as a typed, actionable error.
//!
//! `outline`, `query` and `syntax_check` all call
//! [`tree_sitter_language_pack::get_parser`]. That call can fail for two very
//! different reasons, and the crate reports both as its own `Error`, which
//! `anyhow` then carried as an untyped string: the process exited 255 with
//! "internal error" and a suggestion telling the user to file a bug report.
//!
//! The two reasons are not the same:
//!
//! - the network is unreachable and the grammar was never downloaded, which is
//!   [`AtomwriteError::GrammarUnavailable`] — exit 69, retry once online;
//! - the grammar name is not one the pack knows, which is
//!   [`AtomwriteError::InvalidInput`] — exit 65, the `--language` value is wrong.
//!
//! A user who gets the first one must not be told the tool is broken. One
//! function decides, and the three verbs share it so they cannot disagree.

use anyhow::Result;
use tree_sitter_language_pack::Error as PackError;

use crate::error::AtomwriteError;

/// Build a parser for `language`, translating the pack's error.
///
/// Returns [`AtomwriteError::GrammarUnavailable`] when the failure is about
/// getting the grammar (download, network, cache), and
/// [`AtomwriteError::InvalidInput`] when the failure is about the name itself.
///
/// # Errors
///
/// Returns one of the two variants described above; the caller adds nothing,
/// because the message already names the language and the reason.
pub fn parser_for(language: &str) -> Result<tree_sitter_language_pack::Parser> {
    tree_sitter_language_pack::get_parser(language)
        .map_err(|error| classify(language, &error).into())
}

/// Load a `Language` value for `language`, translating the pack's error.
///
/// # Errors
///
/// Same two variants as [`parser_for`].
pub fn language_for(language: &str) -> Result<tree_sitter::Language> {
    tree_sitter_language_pack::get_language(language)
        .map_err(|error| classify(language, &error).into())
}

/// Decide which variant a pack error deserves.
///
/// The `LanguageNotFound` variant is about the name, so it is invalid input.
/// Everything else the pack raises while loading (download, manifest, network,
/// dynamic load) is about obtaining the grammar for a name the caller supplied,
/// which is a host precondition and never a defect of this program.
fn classify(language: &str, error: &PackError) -> AtomwriteError {
    if matches!(error, PackError::LanguageNotFound(_)) {
        return AtomwriteError::InvalidInput {
            reason: format!(
                "unknown tree-sitter language '{language}'; the pack does not know that name"
            ),
        };
    }
    AtomwriteError::GrammarUnavailable {
        language: language.to_owned(),
        reason: error.to_string(),
    }
}
