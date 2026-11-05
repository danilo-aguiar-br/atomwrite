// SPDX-License-Identifier: MIT OR Apache-2.0
//! Stub syntax check when feature `ast` is disabled.

use std::path::Path;

/// Result of a syntax check pass.
#[derive(Debug, Clone)]
pub enum SyntaxCheckResult {
    /// Parse succeeded with no ERROR nodes.
    Ok,
    /// Language has no parser available; check skipped.
    Skipped {
        /// Why the check was skipped.
        reason: String,
    },
    /// Parser found ERROR or MISSING nodes.
    Errors {
        /// Error node count.
        count: usize,
        /// First error location.
        first: SyntaxErrorLocation,
    },
}

/// Location and description of a single syntax error in the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxErrorLocation {
    /// 0-based byte offset into the source.
    pub byte_offset: usize,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
    /// The kind of the offending node.
    pub kind: String,
    /// Human-readable description.
    pub message: String,
}

/// No language detection without tree-sitter, so a caller treats the file as
/// non-source.
pub fn detect_language_name(_path: &Path, _content: &[u8]) -> Option<String> {
    None
}

/// No-op syntax check without tree-sitter.
///
/// # Errors
///
/// Never returns `Err`: the stub always answers `Ok(Skipped)` with reason
/// `syntax-check requires --features ast`, so callers see the same shape as
/// the `ast` build.
pub fn syntax_check(_path: &Path, _content: &[u8]) -> anyhow::Result<SyntaxCheckResult> {
    Ok(SyntaxCheckResult::Skipped {
        reason: "syntax-check requires --features ast".into(),
    })
}
