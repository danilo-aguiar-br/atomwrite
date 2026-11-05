// SPDX-License-Identifier: MIT OR Apache-2.0
//! Stub when feature `ast` is disabled.
use crate::cli::GlobalArgs;
use crate::cli_args::OutlineArgs;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use anyhow::Result;
use serde::Serialize;
use std::io::Write;

/// One structural outline item, present so the shared callers keep one shape
/// in a build without `ast`. The stub never produces one.
#[derive(Debug, Clone, Serialize)]
pub struct OutlineItem {
    /// Event type discriminator (`outline_item`).
    pub r#type: &'static str,
    /// Source file path.
    pub path: String,
    /// Detected/selected language id.
    pub language: String,
    /// Structural node kind (function, class, …).
    pub kind: String,
    /// Extracted symbol name.
    pub name: String,
    /// Compact signature snippet.
    pub signature: String,
    /// 1-based start line.
    pub start_line: usize,
    /// 1-based end line.
    pub end_line: usize,
    /// Optional byte offset start (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_byte: Option<usize>,
    /// Optional byte offset end (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_byte: Option<usize>,
    /// Optional start column (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_column: Option<usize>,
    /// Optional end column (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
    /// Present and true when `signature` was cut.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_truncated: Option<bool>,
    /// Present and true when the node's subtree contains a parse error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_error: Option<bool>,
}

/// One `--symbol` match: byte span and the line the declaration starts on.
#[derive(Debug, Clone, Copy)]
pub struct SymbolSpan {
    /// First byte of the match.
    pub start: usize,
    /// Byte just past the match.
    pub end: usize,
    /// 1-based line where the match starts.
    pub line: usize,
}

/// No outline without tree-sitter: the stub answers an empty list.
pub fn collect_outline_for_path(
    _path: &std::path::Path,
    _content: &[u8],
    _language: Option<&str>,
) -> Vec<OutlineItem> {
    Vec::new()
}

/// No outline without tree-sitter: the stub answers an empty list and reports
/// that nothing was parsed.
pub fn collect_outline_for_path_status(
    _path: &std::path::Path,
    _content: &[u8],
    _language: Option<&str>,
) -> (Vec<OutlineItem>, bool) {
    (Vec::new(), false)
}

/// No symbol spans without tree-sitter.
pub fn symbol_spans(
    _path: &std::path::Path,
    _content: &[u8],
    _language: Option<&str>,
    _name: &str,
) -> Vec<SymbolSpan> {
    Vec::new()
}

/// The stub never finds a symbol, so the caller reads the same invalid-input
/// answer an `ast` build gives for a name that is absent.
///
/// # Errors
///
/// Always returns `AtomwriteError::InvalidInput` naming the symbol.
pub fn resolve_symbol_span(
    _spans: &[SymbolSpan],
    name: &str,
    path: &std::path::Path,
) -> Result<SymbolSpan, crate::error::AtomwriteError> {
    Err(crate::error::AtomwriteError::InvalidInput {
        reason: format!("symbol '{name}' not found in {}", path.display()),
    })
}

/// No enclosing span without tree-sitter.
pub fn enclosing_span(
    _path: &std::path::Path,
    _content: &[u8],
    _language: Option<&str>,
    _line: usize,
) -> Option<(usize, usize)> {
    None
}

/// A build without `ast` cannot count parse errors, so the syntax guard of
/// `apply` sees zero before and zero after.
pub(crate) fn parse_error_count(_path: &std::path::Path, _content: &[u8]) -> u64 {
    0
}

/// A build without `ast` has no error span to name.
pub(crate) fn first_error_span(_path: &std::path::Path, _content: &[u8]) -> Option<(usize, usize)> {
    None
}

/// Outline requires feature `ast`.
///
/// # Errors
///
/// Always returns `AtomwriteError::ConfigInvalid` because this build was compiled without the
/// `ast` feature.
pub fn cmd_outline(
    _args: &OutlineArgs,
    _global: &GlobalArgs,
    _writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    Err(AtomwriteError::ConfigInvalid {
        reason: rust_i18n::t!("feature.ast-required", command = "outline").to_string(),
    }
    .into())
}
