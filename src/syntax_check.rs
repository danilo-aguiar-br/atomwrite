// SPDX-License-Identifier: MIT OR Apache-2.0

//! G72 — Real syntax check via `tree-sitter-language-pack`.
//!
//! ## Problem
//!
//! The original v0.1.12 heuristic (`syntax_heuristic_check` in
//! `atomic.rs`) only checks for balanced brackets and quote pairing. It
//! cannot detect:
//! - Missing semicolons in C/Go/Java/JavaScript
//! - Unbalanced braces inside `match` arms
//! - Indentation errors in Python
//! - Unclosed string literals containing escape sequences
//! - Reserved-word typos
//!
//! ## Solution
//!
//! Use `tree_sitter_language_pack::get_parser(name)` to parse the
//! content. The returned `Tree` exposes `Node::is_error()` and
//! `Node::is_missing()` flags plus `Node::has_error()` for the subtree.
//! We walk the tree counting error/missing nodes and report the first
//! one (with line and column) as the failure reason.
//!
//! ## Language Detection
//!
//! - File extension → language name (via internal map).
//! - Fallback: shebang detection for `python`, `ruby`, `bash`, `node`.
//! - If no parser is available for the language, the function returns
//!   `None` (no check performed) — this matches the documented behavior
//!   in `AtomicWriteOptions::syntax_check`.
//!
//! ## Causa x Efeito
//!
//! - **Causa**: Brackets balanceados não detectam erros semânticos.
//! - **Efeito**: Usuário escreve código inválido e o atomic_write
//!   completa silenciosamente, depois o build falha minutos depois.
//! - **Solução**: Parse via tree-sitter + scan de `is_error`/`is_missing`.
//! - **Benefício**: Falha rápida, mensagem precisa, zero overhead para
//!   extensões desconhecidas (no-op silencioso).

use std::path::Path;

use anyhow::Result;
use tree_sitter_language_pack::get_parser;

/// Result of a syntax check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxCheckResult {
    /// Tree parsed cleanly (no error or missing nodes).
    Ok,
    /// No parser available for the file's detected language.
    /// Caller should treat as success (silent skip).
    Skipped {
        /// Human-readable explanation of why the check was skipped.
        reason: String,
    },
    /// Tree parsed but grammar reported errors. `count` is the number
    /// of error/missing nodes; `first` describes the first one.
    Errors {
        /// Number of error or missing nodes found in the tree.
        count: usize,
        /// Location and message of the first error node.
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
    /// The kind of the offending node (e.g. `ERROR`, `MISSING ...`).
    pub kind: String,
    /// Human-readable description.
    pub message: String,
}

/// Map a file extension (without the leading dot) to a tree-sitter
/// language name. Returns `None` for unknown extensions.
fn extension_to_language(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "mjs" | "cjs" | "jsx" => Some("javascript"),
        "ts" => Some("typescript"),
        "tsx" => Some("tsx"),
        "go" => Some("go"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some("cpp"),
        "java" => Some("java"),
        "rb" => Some("ruby"),
        "php" => Some("php"),
        "sh" | "bash" | "zsh" => Some("bash"),
        "html" | "htm" => Some("html"),
        "css" => Some("css"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        "md" | "markdown" => Some("markdown"),
        "lua" => Some("lua"),
        "scala" => Some("scala"),
        "swift" => Some("swift"),
        "kt" | "kts" => Some("kotlin"),
        "sql" => Some("sql"),
        _ => None,
    }
}

/// Detect the tree-sitter language name for a given file path.
///
/// Tries the extension first, then falls back to a content-based
/// detection via the first 8 KiB of `content` (for shebang detection).
///
/// Returns `None` if no language can be detected.
pub fn detect_language_name(path: &Path, content: &[u8]) -> Option<String> {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        if let Some(name) = extension_to_language(ext) {
            return Some(name.to_owned());
        }
    }
    // Fallback: try content-based detection (shebang)
    if let Ok(text) = std::str::from_utf8(content) {
        let head = text.get(..text.len().min(8192)).unwrap_or(text);
        let trimmed = head.trim_start();
        if let Some(rest) = trimmed.strip_prefix("#!") {
            let shebang = rest.lines().next().unwrap_or("").to_ascii_lowercase();
            if shebang.contains("python") {
                return Some("python".to_owned());
            } else if shebang.contains("ruby") {
                return Some("ruby".to_owned());
            } else if shebang.contains("bash") || shebang.contains("sh") {
                return Some("bash".to_owned());
            } else if shebang.contains("node") {
                return Some("javascript".to_owned());
            }
        }
    }
    None
}

/// Backward-compat alias: some callers expect a `detect_language` that
/// returns the language NAME (not an enum). Kept for API stability.
pub fn detect_language(path: &Path, content: &[u8]) -> Option<LangRef> {
    detect_language_name(path, content).map(LangRef)
}

/// Newtype wrapper for a string-based language identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangRef(pub String);

impl LangRef {
    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for LangRef {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Run a tree-sitter syntax check on `content` for the language
/// detected from `path`.
///
/// Returns:
/// - `Ok(Ok)` if the tree parsed without error or missing nodes
/// - `Ok(Skipped)` if no parser is available for the language
/// - `Ok(Errors)` if the grammar reported one or more errors
/// - `Err(_)` only for internal failures (e.g. invalid UTF-8 in content
///   when the language requires it)
pub fn syntax_check(path: &Path, content: &[u8]) -> Result<SyntaxCheckResult> {
    let Some(lang_name) = detect_language_name(path, content) else {
        return Ok(SyntaxCheckResult::Skipped {
            reason: format!(
                "no parser for path {} (no extension or unknown language)",
                path.display()
            ),
        });
    };
    let mut parser = match get_parser(&lang_name) {
        Ok(p) => p,
        Err(e) => {
            return Ok(SyntaxCheckResult::Skipped {
                reason: format!("parser init failed for {lang_name}: {e}"),
            });
        }
    };
    // The Parser API takes `&str` (UTF-8) or `&[u8]` via `parse_bytes`.
    // Try str first; fall back to bytes if invalid UTF-8.
    let tree = if let Ok(text) = std::str::from_utf8(content) {
        parser.parse(text)
    } else {
        // Some grammars (binary-ish) accept bytes directly.
        match parser.parse_bytes(content) {
            Some(t) => Some(t),
            None => {
                return Ok(SyntaxCheckResult::Skipped {
                    reason: format!("parser returned no tree for {lang_name}"),
                });
            }
        }
    };
    let Some(tree) = tree else {
        return Ok(SyntaxCheckResult::Skipped {
            reason: format!("parser returned no tree for {lang_name}"),
        });
    };
    let root = tree.root_node();
    let mut count = 0usize;
    let mut first: Option<SyntaxErrorLocation> = None;
    let mut cursor = root.walk();
    scan_errors(&mut cursor, content, &mut count, &mut first);
    if count == 0 {
        Ok(SyntaxCheckResult::Ok)
    } else {
        Ok(SyntaxCheckResult::Errors {
            count,
            first: first.unwrap_or_else(|| SyntaxErrorLocation {
                byte_offset: 0,
                line: 1,
                column: 1,
                kind: "ERROR".to_owned(),
                message: "tree-sitter reported errors but no first location captured".to_owned(),
            }),
        })
    }
}

/// Iterative DFS walk over the tree using an explicit stack of parent
/// nodes. Counts error/missing nodes and captures the first one for
/// diagnostic display.
///
/// We maintain a `Vec<Node>` of ancestors so that after processing a
/// subtree we can return to the parent via `cursor.goto_parent()` and
/// continue with the next sibling. This avoids the deep recursion that
/// previously caused stack overflows on large or pathological parse trees.
fn scan_errors(
    cursor: &mut tree_sitter_language_pack::TreeCursor,
    source: &[u8],
    count: &mut usize,
    first: &mut Option<SyntaxErrorLocation>,
) {
    let mut parent_stack: Vec<tree_sitter_language_pack::Node> = Vec::with_capacity(64);
    loop {
        // Process the current node.
        let node = cursor.node();
        let kind = node.kind();
        let is_error = node.is_error();
        let is_missing = node.is_missing();
        if is_error || is_missing {
            *count += 1;
            if first.is_none() {
                let start = node.start_position();
                let snippet = extract_snippet(source, node.start_byte(), node.end_byte());
                let kind_str = if is_error {
                    "ERROR".to_owned()
                } else {
                    format!("MISSING {}", kind)
                };
                *first = Some(SyntaxErrorLocation {
                    byte_offset: node.start_byte(),
                    line: start.row + 1,
                    column: start.column + 1,
                    kind: kind_str.clone(),
                    message: format_error_message(&kind_str, &snippet),
                });
            }
        }
        // Try to descend into the first child.
        if cursor.goto_first_child() {
            // Remember the node we just left so we can return later.
            if let Some(parent) = node.parent() {
                parent_stack.push(parent);
            } else {
                // Should not happen — we just came from a parent — but be safe.
                parent_stack.push(node);
            }
            continue;
        }
        // No children. Walk siblings; if none, ascend.
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            // No more siblings. Try to ascend to the parent.
            if cursor.goto_parent() {
                parent_stack.pop();
                // After ascending, try to go to the next sibling of
                // this parent.
                continue;
            }
            // Cannot ascend further: traversal done.
            return;
        }
    }
}

/// Extract a short, printable snippet from `source[start..end]`,
/// collapsing non-ASCII bytes and trimming to 80 chars.
fn extract_snippet(source: &[u8], start: usize, end: usize) -> String {
    let end = end.min(source.len()).max(start);
    let raw = source.get(start..end).unwrap_or(&[]);
    let s = String::from_utf8_lossy(raw);
    let trimmed: String = s
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(80)
        .collect();
    if trimmed.is_empty() {
        "<empty>".to_owned()
    } else {
        trimmed
    }
}

/// Format a human-readable message for a tree-sitter error/missing node.
fn format_error_message(kind: &str, snippet: &str) -> String {
    if kind == "ERROR" {
        format!("unexpected token: {}", snippet)
    } else if let Some(rest) = kind.strip_prefix("MISSING ") {
        format!("expected {} before/after: {}", rest, snippet)
    } else {
        format!("{} near: {}", kind, snippet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_language_uses_extension() {
        let p = Path::new("foo.rs");
        assert_eq!(
            detect_language_name(p, b"fn main() {}").as_deref(),
            Some("rust")
        );
    }

    #[test]
    fn detect_language_unknown_ext_returns_none() {
        let p = Path::new("foo.unknownext");
        assert!(detect_language_name(p, b"hello").is_none());
    }

    #[test]
    fn detect_language_shebang_python() {
        let p = Path::new("script");
        let content = b"#!/usr/bin/env python3\nprint('hi')";
        assert_eq!(detect_language_name(p, content).as_deref(), Some("python"));
    }

    #[test]
    fn detect_language_shebang_bash() {
        let p = Path::new("script");
        let content = b"#!/bin/bash\necho hi";
        assert_eq!(detect_language_name(p, content).as_deref(), Some("bash"));
    }

    #[test]
    fn syntax_check_valid_rust_is_ok() {
        let p = Path::new("foo.rs");
        let content = b"fn main() { println!(\"hi\"); }\n";
        let r = syntax_check(p, content).unwrap();
        assert_eq!(r, SyntaxCheckResult::Ok);
    }

    #[test]
    fn syntax_check_invalid_rust_reports_error() {
        let p = Path::new("foo.rs");
        // Unclosed brace: this should be detected by tree-sitter.
        let content = b"fn main() { println!(\"hi\"); \n";
        let r = syntax_check(p, content).unwrap();
        match r {
            SyntaxCheckResult::Errors { count, first } => {
                assert!(count >= 1);
                assert!(!first.message.is_empty());
            }
            other => panic!("expected Errors, got {:?}", other),
        }
    }

    #[test]
    fn syntax_check_valid_python_is_ok() {
        let p = Path::new("foo.py");
        let content = b"def hello():\n    print('hi')\n";
        let r = syntax_check(p, content).unwrap();
        assert_eq!(r, SyntaxCheckResult::Ok);
    }

    #[test]
    fn syntax_check_invalid_python_reports_error() {
        let p = Path::new("foo.py");
        // Unclosed parenthesis is a clear syntax error in any language.
        let content = b"def hello(:\n    print('hi')\n";
        let r = syntax_check(p, content).unwrap();
        assert!(matches!(r, SyntaxCheckResult::Errors { .. }));
    }

    #[test]
    fn syntax_check_unknown_ext_is_skipped() {
        let p = Path::new("foo.xyz_unknown");
        let content = b"<<<not valid anything>>>";
        let r = syntax_check(p, content).unwrap();
        assert!(matches!(r, SyntaxCheckResult::Skipped { .. }));
    }

    #[test]
    fn syntax_check_valid_json_is_ok() {
        let p = Path::new("foo.json");
        let content = br#"{"key": "value", "n": 42}"#;
        let r = syntax_check(p, content).unwrap();
        assert_eq!(r, SyntaxCheckResult::Ok);
    }

    #[test]
    fn syntax_check_invalid_json_reports_error() {
        let p = Path::new("foo.json");
        // Trailing comma is not valid JSON.
        let content = br#"{"key": "value",}"#;
        let r = syntax_check(p, content).unwrap();
        assert!(matches!(r, SyntaxCheckResult::Errors { .. }));
    }

    #[test]
    fn extension_to_language_is_case_insensitive() {
        assert_eq!(extension_to_language("RS"), Some("rust"));
        assert_eq!(extension_to_language("Py"), Some("python"));
        assert_eq!(extension_to_language("TS"), Some("typescript"));
    }

    #[test]
    fn extract_snippet_truncates_long_content() {
        let long = "x".repeat(200);
        let s = extract_snippet(long.as_bytes(), 0, 200);
        assert_eq!(s.len(), 80);
    }

    #[test]
    fn extract_snippet_handles_empty_range() {
        let s = extract_snippet(b"hello", 3, 3);
        assert_eq!(s, "<empty>");
    }

    #[test]
    fn format_error_message_for_error_kind() {
        let m = format_error_message("ERROR", ";");
        assert!(m.contains("unexpected token"));
        assert!(m.contains(";"));
    }

    #[test]
    fn format_error_message_for_missing_kind() {
        let m = format_error_message("MISSING semicolon", "");
        assert!(m.contains("expected semicolon"));
    }

    #[test]
    fn tempdir_can_parse_typical_rust_file() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("lib.rs");
        let content = b"use std::io;\n\npub fn add(a: i32, b: i32) -> i32 { a + b }\n";
        let r = syntax_check(&p, content).unwrap();
        assert_eq!(r, SyntaxCheckResult::Ok);
    }
}
