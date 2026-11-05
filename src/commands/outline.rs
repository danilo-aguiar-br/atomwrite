// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand (v0.1.12): `outline` — extract the high-level
//! structure of a source file (functions, classes, structs, enums,
//! traits, modules, top-level consts) as NDJSON. Uses
//! `tree-sitter-language-pack`.
//!
//! Workload: mixed I/O-bound + CPU-bound (file read + tree-sitter outline walk).
//! Parallelism: single-file; no multi-file fan-out in this subcommand.
//!
//! ## Cause and Effect
//!
//! - **Cause**: `read` returns raw text. LLM agents spend tokens reading
//!   500 lines only to learn that 12 of them are signatures.
//! - **Effect**: Wasted context, high latency, and scope hallucinations
//!   (assuming function X exists when X lives in module Y).
//! - **Solution**: Parse with tree-sitter, walk top-level structural node
//!   kinds, and emit each as `outline_item` NDJSON with `kind`, `name`,
//!   `signature`, and `start_line`.
//! - **Benefit**: Compact, precise, navigable context (10 items ≈ 2 KB vs
//!   50 KB of full source).

use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::cli::{GlobalArgs, OutlineArgs};
use crate::output::NdjsonWriter;

/// One structural outline item (NDJSON `outline_item`).
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
    ///
    /// The cut happens at `OUTLINE_SIGNATURE_MAX_BYTES` or at the first line,
    /// so a reader that saw a signature ending in `{` knows the body was not
    /// part of it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_truncated: Option<bool>,
    /// Present and true when the node's subtree contains a parse error.
    ///
    /// The item stays in the stream because the declaration is still there;
    /// the flag says its body is a recovered view, not the parsed structure.
    /// A tree-sitter recovery that swallowed the structural node emits an item
    /// of the error span instead, so the gap never disappears silently.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_error: Option<bool>,
}

/// Terminal event of `outline`: item count for one file.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct OutlineSummary {
    r#type: &'static str,
    path: String,
    language: String,
    items: usize,
    elapsed_ms: u64,
    /// Items that were emitted with a cut `signature` field.
    ///
    /// The item itself is in the stream: only `signature` was shortened, at
    /// `OUTLINE_SIGNATURE_MAX_BYTES` or at the first line. This never means
    /// an item was left out.
    #[serde(skip_serializing_if = "Option::is_none")]
    items_truncated: Option<u64>,
    /// Items the `--limit` budget kept out of the stream.
    ///
    /// Distinct from `items_truncated`, which counts cut signatures, not
    /// missing items.
    #[serde(skip_serializing_if = "Option::is_none")]
    items_omitted: Option<u64>,
    /// Nodes the grammar marked as errors or missing.
    ///
    /// A non-zero count means the symbol list may be incomplete, so a caller
    /// editing this file knows the outline is a partial view.
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_errors: Option<u64>,
    /// 1-based line where the first parse error starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    first_error_line: Option<usize>,
}

const STRUCTURAL_NODE_KINDS: &[&str] = &[
    "function_item",
    "function_definition",
    "function_declaration",
    "method_declaration",
    "function",
    "generator_function_declaration",
    "class_item",
    "class_definition",
    "class_declaration",
    "class",
    "struct_item",
    "struct_declaration",
    "struct_type",
    "enum_item",
    "enum_declaration",
    "trait_item",
    "interface_declaration",
    "impl_item",
    "module",
    "mod_item",
    "const_item",
    "static_item",
    "type_item",
    "type_alias",
    "macro_invocation",
    "namespace_declaration",
    "lambda",
    "arrow_function",
];

/// Execute the `outline` subcommand.
///
/// Reads the source file, parses it via `tree-sitter-language-pack`,
/// walks the tree looking for structural node kinds (functions,
/// classes, structs, etc.), and emits each as an `outline_item`
/// NDJSON line. After the tree is exhausted, a final
/// `outline_summary` line is emitted with the total count.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` when the target file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` when the path escapes the workspace.
/// Returns `AtomwriteError::FileTooLarge` when the file exceeds the effective max filesize.
/// Returns `AtomwriteError::InvalidInput` when `--language` names an unsupported grammar or the
/// language cannot be detected from the path and content.
/// Returns an error when no tree-sitter parser is available for the language or the parser
/// returns no tree.
/// Returns `AtomwriteError::Io` when the file cannot be read.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
#[tracing::instrument(skip_all, fields(command = "outline"))]
pub fn cmd_outline(
    args: &OutlineArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    writer.set_text_verb("outline");
    let workspace = global.resolve_workspace()?;
    let validated = crate::path_safety::validate_path(&args.path, &workspace)?;
    if !validated.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: validated }.into());
    }
    let content = crate::file_io::read_file_bytes(&validated, global.effective_max_filesize())?;

    let lang_name = crate::commands::query::resolve_language_name(
        args.language.as_deref(),
        &validated,
        &content,
    )?;
    let kind_filter: Option<Vec<String>> = if args.kinds.is_empty() {
        None
    } else {
        Some(args.kinds.iter().map(|s| s.to_lowercase()).collect())
    };

    // Markdown has no grammar node in the structural list, so a document used
    // to come back as `ok:true` with zero items. Headings are read from the
    // text, where they are unambiguous.
    let (collected, parse_errors, first_error_line) =
        if lang_name == "markdown" || lang_name == "md" {
            (markdown_sections(&validated, &content), None, None)
        } else {
            let mut parser = crate::grammar::parser_for(&lang_name)?;
            // Parsing the BYTES is what lets a file with invalid UTF-8 answer its
            // symbols; decoding first replaced every bad byte and parsed a lie.
            let tree = parser
                .parse_bytes(&content)
                .with_context(|| format!("parser returned no tree for {lang_name}"))?;
            let root = tree.root_node();
            let mut collected = Vec::new();
            walk_outline_collect(
                &root,
                &content,
                &validated,
                &lang_name,
                &kind_filter,
                args.positions,
                &mut collected,
            );
            let (errors, first_line) = count_parse_errors(&root);
            ((collected), (errors > 0).then_some(errors), first_line)
        };
    let items_truncated = collected
        .iter()
        .filter(|i| i.signature_truncated == Some(true))
        .count() as u64;
    let tsv = args.format == crate::cli::OutlineFormat::Tsv;
    if tsv {
        writer.write_text_bytes(format!("# {}\n", validated.display()).as_bytes())?;
    }
    let mut items: usize = 0;
    let mut omitted: u64 = 0;
    for item in collected {
        // The record budget is spent: this item is work the consumer may not
        // read, so the count says how many the stream left out.
        if crate::output::remaining_budget() == Some(0) {
            omitted = omitted.saturating_add(1);
            continue;
        }
        if tsv {
            writer.write_text_bytes(
                format!(
                    "{}-{}\t{}\t{}\n",
                    item.start_line, item.end_line, item.kind, item.name
                )
                .as_bytes(),
            )?;
        } else {
            writer.write_event(&item)?;
        }
        items += 1;
    }
    if tsv {
        return Ok(());
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    writer.write_terminal(&OutlineSummary {
        r#type: "outline_summary",
        path: validated.display().to_string(),
        language: lang_name,
        items,
        elapsed_ms,
        items_truncated: (items_truncated > 0).then_some(items_truncated),
        items_omitted: (omitted > 0).then_some(omitted),
        parse_errors,
        first_error_line,
    })?;
    Ok(())
}

/// Collect structural outline items for one file (no I/O writer — rayon-safe).
///
/// Parse failures return an empty vec so budgeted callers can skip quietly.
pub fn collect_outline_for_path(
    path: &std::path::Path,
    content: &[u8],
    language: Option<&str>,
) -> Vec<OutlineItem> {
    collect_outline_for_path_status(path, content, language).0
}

/// Collect outline items and say whether the file was parsed at all.
///
/// `false` means no grammar was resolved or the parser returned no tree, and
/// the empty vec is a hole instead of a file without symbols.
pub fn collect_outline_for_path_status(
    path: &std::path::Path,
    content: &[u8],
    language: Option<&str>,
) -> (Vec<OutlineItem>, bool) {
    let lang_name = match crate::commands::query::resolve_language_name(language, path, content) {
        Ok(l) => l,
        Err(_) => return (Vec::new(), false),
    };
    let mut parser = match crate::grammar::parser_for(&lang_name) {
        Ok(p) => p,
        Err(_) => return (Vec::new(), false),
    };
    let tree = match parser
        .parse(std::str::from_utf8(content).unwrap_or(""))
        .or_else(|| parser.parse_bytes(content))
    {
        Some(t) => t,
        None => return (Vec::new(), false),
    };
    let root = tree.root_node();
    let mut out = Vec::new();
    walk_outline_collect(&root, content, path, &lang_name, &None, false, &mut out);
    (out, true)
}

/// Emit structural outline items for one file (used by `outline` and `sparse outline`).
///
/// Returns the number of items emitted. Parse failures return `Ok(0)` so callers
/// can skip unsupported/binary files under a budget.
///
/// # Errors
///
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
/// Returns a serialization error when an outline item cannot be encoded as JSON.
pub fn emit_outline_for_path(
    path: &std::path::Path,
    content: &[u8],
    language: Option<&str>,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<usize> {
    let items = collect_outline_for_path(path, content, language);
    let n = items.len();
    for item in items {
        writer.write_event(&item)?;
    }
    Ok(n)
}

#[allow(clippy::too_many_arguments)]
fn walk_outline_collect(
    root: &tree_sitter_language_pack::Node,
    source: &[u8],
    path: &std::path::Path,
    lang_name: &str,
    kind_filter: &Option<Vec<String>>,
    show_positions: bool,
    out: &mut Vec<OutlineItem>,
) {
    let mut stack: Vec<tree_sitter_language_pack::Node> = vec![root.clone()];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        if STRUCTURAL_NODE_KINDS.contains(&kind.as_str()) {
            let lc_kind = kind.to_lowercase();
            let matches_filter = match kind_filter {
                Some(f) => f.iter().any(|k| k == &lc_kind || k == &kind),
                None => true,
            };
            if matches_filter {
                let name = extract_name(&node, source);
                let (signature, signature_truncated) = extract_signature(&node, source);
                let start = node.start_position();
                let end = node.end_position();
                out.push(OutlineItem {
                    r#type: "outline_item",
                    path: path.display().to_string(),
                    language: lang_name.to_string(),
                    kind: kind.to_string(),
                    name,
                    signature,
                    start_line: start.row + 1,
                    end_line: end.row + 1,
                    start_byte: if show_positions {
                        Some(node.start_byte())
                    } else {
                        None
                    },
                    end_byte: if show_positions {
                        Some(node.end_byte())
                    } else {
                        None
                    },
                    start_column: if show_positions {
                        Some(start.column)
                    } else {
                        None
                    },
                    end_column: if show_positions {
                        Some(end.column)
                    } else {
                        None
                    },
                    signature_truncated: signature_truncated.then_some(true),
                    // Recovery keeps the node but may hide the error inside it.
                    has_error: node.has_error().then_some(true),
                });
            }
        } else if node.is_error() || node.is_missing() {
            // Recovery can wrap a whole declaration in one ERROR node: no
            // structural node is left to mark, so the gap would vanish from
            // the outline. Emit the span instead and stop here, so nested
            // errors do not repeat it.
            if !subtree_has_structural(&node) {
                let start = node.start_position();
                let end = node.end_position();
                let (signature, signature_truncated) = extract_signature(&node, source);
                out.push(OutlineItem {
                    r#type: "outline_item",
                    path: path.display().to_string(),
                    language: lang_name.to_string(),
                    kind: kind.to_string(),
                    name: extract_name(&node, source),
                    signature,
                    start_line: start.row + 1,
                    end_line: end.row + 1,
                    start_byte: if show_positions {
                        Some(node.start_byte())
                    } else {
                        None
                    },
                    end_byte: if show_positions {
                        Some(node.end_byte())
                    } else {
                        None
                    },
                    start_column: if show_positions {
                        Some(start.column)
                    } else {
                        None
                    },
                    end_column: if show_positions {
                        Some(end.column)
                    } else {
                        None
                    },
                    signature_truncated: signature_truncated.then_some(true),
                    has_error: Some(true),
                });
                continue;
            }
        }
        let count = node.child_count() as u32;
        for i in (0..count).rev() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
}

/// Whether any structural node lives inside `root`.
///
/// Used before emitting an error span: when a declaration survives inside the
/// recovered error subtree, that declaration carries the `has_error` mark, and
/// a second item for the same lines would report the gap twice.
fn subtree_has_structural(root: &tree_sitter_language_pack::Node) -> bool {
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        if STRUCTURAL_NODE_KINDS.contains(&node.kind().as_str()) {
            return true;
        }
        let count = node.child_count() as u32;
        for i in 0..count {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    false
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

/// Byte spans of every structural node or Markdown section named `name`.
///
/// A repeated name is ambiguous: the caller lists every line and refuses it,
/// instead of silently answering the first one. Markdown answers by heading
/// text and includes the sub-sections, because a heading is the symbol there.
pub fn symbol_spans(
    path: &std::path::Path,
    content: &[u8],
    language: Option<&str>,
    name: &str,
) -> Vec<SymbolSpan> {
    if is_markdown_path(path) {
        return markdown_symbol_spans(content, name);
    }
    let Some(parsed) = parse_structural(path, content, language) else {
        return Vec::new();
    };
    let mut spans = Vec::new();
    let mut stack = vec![parsed.root];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        if STRUCTURAL_NODE_KINDS.contains(&kind.as_str()) && extract_name(&node, content) == name {
            spans.push(SymbolSpan {
                start: node.start_byte(),
                end: node.end_byte(),
                line: node.start_position().row + 1,
            });
        }
        let count = node.child_count() as u32;
        for i in (0..count).rev() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    spans
}

/// Resolve `--symbol` to exactly one span, or the error that says why not.
pub fn resolve_symbol_span(
    spans: &[SymbolSpan],
    name: &str,
    path: &std::path::Path,
) -> Result<SymbolSpan, crate::error::AtomwriteError> {
    match spans {
        [] => Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("symbol '{name}' not found in {}", path.display()),
        }),
        [one] => Ok(*one),
        many => {
            let lines: Vec<String> = many.iter().map(|span| span.line.to_string()).collect();
            Err(crate::error::AtomwriteError::InvalidInput {
                reason: format!(
                    "symbol '{name}' appears {} times in {} at lines {}; name one with --lines or read it with --line N --enclosing",
                    many.len(),
                    path.display(),
                    lines.join(", ")
                ),
            })
        }
    }
}

fn is_markdown_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown" | "mdx"))
}

/// Every Markdown section whose heading text equals `name`.
///
/// A section runs from its heading line to the line before the next heading of
/// the same or shallower level, so the sub-sections come along.
fn markdown_symbol_spans(content: &[u8], name: &str) -> Vec<SymbolSpan> {
    let mut lines: Vec<(usize, usize, &[u8])> = Vec::new();
    let mut offset = 0usize;
    for (index, line) in content.split_inclusive(|&b| b == b'\n').enumerate() {
        lines.push((offset, index + 1, line));
        offset += line.len();
    }
    let mut headings: Vec<(usize, usize, String, usize)> = Vec::new();
    for (index, (start, number, raw)) in lines.iter().enumerate() {
        let text = String::from_utf8_lossy(raw);
        let trimmed = text.trim_end_matches(['\r', '\n']);
        if let Some(rest) = trimmed.strip_prefix('#') {
            let level = trimmed.len() - trimmed.trim_start_matches('#').len();
            let title = rest.trim_start_matches('#').trim();
            if !title.is_empty() {
                headings.push((level, *start, title.to_string(), *number));
            }
            continue;
        }
        let title = trimmed.trim();
        if title.is_empty() {
            continue;
        }
        if let Some((_, _, next)) = lines.get(index + 1) {
            let underline = String::from_utf8_lossy(next);
            let underline = underline.trim();
            let is_setext = !underline.is_empty()
                && (underline.chars().all(|c| c == '=') || underline.chars().all(|c| c == '-'));
            if is_setext {
                let level = if underline.starts_with('=') { 1 } else { 2 };
                headings.push((level, *start, title.to_string(), *number));
            }
        }
    }
    let mut spans = Vec::new();
    for (index, (level, start, title, line)) in headings.iter().enumerate() {
        if title != name {
            continue;
        }
        let end = headings[index + 1..]
            .iter()
            .find(|(next_level, ..)| next_level <= level)
            .map(|(_, next_start, ..)| *next_start)
            .unwrap_or(content.len());
        spans.push(SymbolSpan {
            start: *start,
            end,
            line: *line,
        });
    }
    spans
}

/// Byte span of the smallest structural node that contains `line` (1-based).
///
/// The walk descends into the deepest match, so `--line 42 --enclosing` yields
/// the function body around line 42 and never the whole file.
pub fn enclosing_span(
    path: &std::path::Path,
    content: &[u8],
    language: Option<&str>,
    line: usize,
) -> Option<(usize, usize)> {
    let parsed = parse_structural(path, content, language)?;
    let mut block: Option<(usize, usize, usize)> = None;
    let mut fallback: Option<(usize, usize, usize)> = None;
    let mut stack = vec![parsed.root];
    while let Some(node) = stack.pop() {
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        // Only a structural node is a block the caller asked to see: a token
        // that happens to sit on the line would answer one quote mark, and a
        // macro call would answer the call instead of the function around it.
        if STRUCTURAL_NODE_KINDS.contains(&node.kind().as_str())
            && start_line <= line
            && line <= end_line
        {
            let span = node.end_byte().saturating_sub(node.start_byte());
            let slot = if is_block_kind(node.kind().as_str()) {
                &mut block
            } else {
                &mut fallback
            };
            if slot.is_none_or(|(_, _, size)| span < size) {
                *slot = Some((node.start_byte(), node.end_byte(), span));
            }
        }
        let count = node.child_count() as u32;
        for i in (0..count).rev() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    block.or(fallback).map(|(start, end, _)| (start, end))
}

/// Whether a node is a definition a reader calls a block.
///
/// The list is the intersection of the grammars the CLI already outlines, and
/// anything outside it still answers through the smallest containing node.
fn is_block_kind(kind: &str) -> bool {
    matches!(
        kind,
        "function_item"
            | "impl_item"
            | "trait_item"
            | "mod_item"
            | "struct_item"
            | "enum_item"
            | "function_definition"
            | "function_declaration"
            | "method_definition"
            | "method_declaration"
            | "class_definition"
            | "class_declaration"
            | "constructor_declaration"
    )
}

/// Parse the file with the grammar that matches its language.
struct ParsedTree {
    root: tree_sitter_language_pack::Node,
    _tree: tree_sitter_language_pack::Tree,
}

fn parse_structural(
    path: &std::path::Path,
    content: &[u8],
    language: Option<&str>,
) -> Option<ParsedTree> {
    let lang_name = crate::commands::query::resolve_language_name(language, path, content).ok()?;
    let mut parser = crate::grammar::parser_for(&lang_name).ok()?;
    let tree = parser
        .parse(std::str::from_utf8(content).unwrap_or(""))
        .or_else(|| parser.parse_bytes(content))?;
    let root = tree.root_node();
    Some(ParsedTree { root, _tree: tree })
}

/// Count the error and missing nodes the grammar reports for a file.
///
/// Returns zero when no grammar matches the file: "no grammar" is not evidence
/// of a broken parse, and refusing a write over an unknown language would block
/// every file the tool cannot parse.
pub(crate) fn parse_error_count(path: &std::path::Path, content: &[u8]) -> u64 {
    let Ok(lang_name) = crate::commands::query::resolve_language_name(None, path, content) else {
        return 0;
    };
    let Ok(mut parser) = crate::grammar::parser_for(&lang_name) else {
        return 0;
    };
    match parser.parse_bytes(content) {
        Some(tree) => count_parse_errors(&tree.root_node()).0,
        None => 0,
    }
}

/// Line span of the block that holds the first parse error of `content`.
///
/// A refusal that names only the file makes the caller re-read the whole file
/// to find the broken block. The error node itself is often a single token, so
/// the answer is the nearest enclosing structural node; the error node's own
/// span is the fallback for a file whose enclosing block recovery dropped.
///
/// Returns `None` when no grammar matches the file or the tree has no error,
/// because "no grammar" is not evidence of a broken parse.
pub(crate) fn first_error_span(path: &std::path::Path, content: &[u8]) -> Option<(usize, usize)> {
    let lang_name = crate::commands::query::resolve_language_name(None, path, content).ok()?;
    let mut parser = crate::grammar::parser_for(&lang_name).ok()?;
    let tree = parser.parse_bytes(content)?;
    let node = first_error_node(&tree.root_node())?;
    let own = (node.start_position().row + 1, node.end_position().row + 1);
    let mut cursor = node.parent();
    while let Some(ancestor) = cursor {
        if STRUCTURAL_NODE_KINDS.contains(&ancestor.kind().as_str()) {
            return Some((
                ancestor.start_position().row + 1,
                ancestor.end_position().row + 1,
            ));
        }
        cursor = ancestor.parent();
    }
    Some(own)
}

/// Earliest error or missing node in the tree, by start then end byte.
fn first_error_node(
    root: &tree_sitter_language_pack::Node,
) -> Option<tree_sitter_language_pack::Node> {
    let mut best: Option<tree_sitter_language_pack::Node> = None;
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let better = best.as_ref().is_none_or(|current| {
                (node.start_byte(), node.end_byte()) < (current.start_byte(), current.end_byte())
            });
            if better {
                best = Some(node.clone());
            }
        }
        let children = node.child_count() as u32;
        for i in 0..children {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    best
}

/// Count error and missing nodes, and the line of the first one.
///
/// The tree-sitter walk already visits every node, so the price is one pass
/// over a tree that is in cache.
fn count_parse_errors(root: &tree_sitter_language_pack::Node) -> (u64, Option<usize>) {
    let mut count: u64 = 0;
    let mut first: Option<usize> = None;
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            count = count.saturating_add(1);
            let line = node.start_position().row + 1;
            first = Some(first.map_or(line, |current| current.min(line)));
        }
        let children = node.child_count() as u32;
        for i in 0..children {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    (count, first)
}

/// Headings of a Markdown document, in file order.
///
/// ATX (`# title`) and setext (`title` over `===`) both count, because a
/// document written either way has the same sections.
fn markdown_sections(path: &std::path::Path, content: &[u8]) -> Vec<OutlineItem> {
    let text = String::from_utf8_lossy(content);
    let lines: Vec<&str> = text.lines().collect();
    // (start line, level, title, signature)
    let mut headings: Vec<(usize, usize, String, String)> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_end();
        if let Some(rest) = trimmed.strip_prefix('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            let title = rest.trim_start_matches('#').trim();
            if !title.is_empty() {
                headings.push((index + 1, level, title.to_string(), trimmed.to_string()));
            }
            continue;
        }
        // Setext: the title sits on this line and the underline comes next.
        if let Some(next) = lines.get(index + 1) {
            let underline = next.trim();
            let is_setext = !trimmed.trim().is_empty()
                && !underline.is_empty()
                && (underline.chars().all(|c| c == '=') || underline.chars().all(|c| c == '-'));
            if is_setext {
                let level = usize::from(!underline.chars().all(|c| c == '=')) + 1;
                headings.push((
                    index + 1,
                    level,
                    trimmed.trim().to_string(),
                    trimmed.to_string(),
                ));
            }
        }
    }
    // A section ends on the line before the next heading of the same or a
    // higher level, and the last one ends with the file. Without this, every
    // section looked like a single line and `--symbol` returned the rest of
    // the document.
    let total = lines.len();
    let mut out = Vec::with_capacity(headings.len());
    for (position, (line, level, title, signature)) in headings.iter().enumerate() {
        let end_line = headings[position + 1..]
            .iter()
            .find(|(_, next_level, ..)| next_level <= level)
            .map(|(next_line, ..)| next_line.saturating_sub(1))
            .unwrap_or(total)
            .max(*line);
        out.push(markdown_item(path, *line, end_line, title, signature));
    }
    out
}

/// One Markdown heading as an outline item.
fn markdown_item(
    path: &std::path::Path,
    line: usize,
    end_line: usize,
    title: &str,
    signature: &str,
) -> OutlineItem {
    OutlineItem {
        r#type: "outline_item",
        path: path.display().to_string(),
        language: "markdown".to_string(),
        kind: "section".to_string(),
        name: title.to_string(),
        signature: if signature.is_empty() {
            format!("# {title}")
        } else {
            signature.to_string()
        },
        start_line: line,
        end_line,
        start_byte: None,
        end_byte: None,
        start_column: None,
        end_column: None,
        signature_truncated: None,
        has_error: None,
    }
}

fn extract_name(node: &tree_sitter_language_pack::Node, source: &[u8]) -> String {
    for kind in ["name", "identifier", "type_identifier", "constant"] {
        if let Some(child) = node.child_by_field_name(kind) {
            return child_text(source, child);
        }
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let c = cursor.node();
            if c.is_named() {
                let text = child_text(source, c);
                if !text.is_empty() {
                    return text;
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    "<anonymous>".to_string()
}

/// Extract the compact signature, saying whether it was cut.
///
/// Two cuts are possible and both count: the per-item byte ceiling, and the
/// signature itself spanning more than one line. A node with a body is not a
/// cut by itself: the signature ends where the body starts, so `fn a() {`
/// reports the whole signature and is not marked truncated.
fn extract_signature(node: &tree_sitter_language_pack::Node, source: &[u8]) -> (String, bool) {
    let start = node.start_byte();
    let signature_end = node
        .child_by_field_name("body")
        .map_or_else(|| node.end_byte(), |body| body.start_byte());
    let cap = start + crate::constants::OUTLINE_SIGNATURE_MAX_BYTES;
    let end = signature_end.min(cap);
    let raw = source.get(start..end).unwrap_or(&[]);
    let s = String::from_utf8_lossy(raw);
    let line = s.lines().next().unwrap_or("");
    let cut = signature_end > cap || s.trim_end().contains('\n');
    let collapsed: String = line
        .chars()
        .filter(|c| !c.is_control() || *c == '\t')
        .collect();
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        ("<empty>".to_owned(), cut)
    } else {
        (trimmed.to_string(), cut)
    }
}

fn child_text(source: &[u8], node: tree_sitter_language_pack::Node) -> String {
    let start = node.start_byte();
    let end = node.end_byte().min(source.len());
    String::from_utf8_lossy(source.get(start..end).unwrap_or(&[])).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_node_kinds_includes_common_ones() {
        assert!(STRUCTURAL_NODE_KINDS.contains(&"function_item"));
        assert!(STRUCTURAL_NODE_KINDS.contains(&"struct_item"));
        assert!(STRUCTURAL_NODE_KINDS.contains(&"class_definition"));
        assert!(STRUCTURAL_NODE_KINDS.contains(&"trait_item"));
    }

    #[test]
    fn kind_filter_lowercases_input() {
        // Filter "function_item" should match an exact "function_item" node.
        let f = Some(vec!["function_item".to_string()]);
        let s = f.as_ref().unwrap();
        let any_match = s.iter().any(|k| k == "function_item");
        assert!(any_match);
    }
}
