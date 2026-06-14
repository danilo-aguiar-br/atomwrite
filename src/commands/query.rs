// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand (v0.1.12): `query` — walk a source file's
//! tree-sitter parse tree and emit AST nodes as NDJSON. Uses
//! `tree-sitter-language-pack` for parser provisioning (305 languages;
//! downloads on first use; cache local).
//!
//! ## Modes
//!
//! - `--query <KIND>`: emit all nodes whose `kind()` matches the given
//!   name (e.g. `function_item`, `class_definition`).
//! - `--kinds`: aggregate all distinct node kinds with their counts.
//! - `--tree`: emit every named node in pre-order DFS (debugging).
//! - `--positions`: include byte offsets and start/end positions.
//!
//! ## Causa x Efeito
//!
//! - **Causa**: Source files são apenas strings opacas para `read`/`grep`.
//!   Buscar `fn add` retorna 12 hits (declaração + 11 chamadas), sem
//!   distinção semântica.
//! - **Efeito**: Agentes LLM gastam tokens disambiguando matches textuais
//!   e falham em extrair estrutura (assinaturas, generics, lifetimes).
//! - **Solução**: Parse via tree-sitter, expor AST como `query_match`
//!   NDJSON com `kind`, `text`, `start_line`. DFS iterativo via
//!   `Node::child(i)` em vez de `TreeCursor` recursivo (stack overflow
//!   em arquivos grandes).
//! - **Benefício**: Resposta estruturada, integrável com grafo e
//!   downstream pipelines.

use std::io::Write;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use tree_sitter::{Query as TsQuery, StreamingIterator};
use tree_sitter_language_pack::{Node, get_language, get_parser, has_language};

use crate::cli::{GlobalArgs, QueryArgs};
use crate::output::NdjsonWriter;

/// Result summary for the `query` subcommand.
#[derive(Debug, Serialize)]
struct QuerySummary {
    r#type: &'static str,
    path: String,
    language: String,
    matches: usize,
    total_nodes: usize,
    elapsed_ms: u64,
}

/// Discriminator between legacy kind-filter matching and the new
/// real S-expression matching introduced in v0.1.19 (G122).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryType {
    /// Plain kind name like `function_item` compared to `node.kind()`.
    KindFilter,
    /// S-expression with optional `@capture` bindings compiled via
    /// `tree_sitter::Query::new`.
    SExpression,
}

/// Detect whether a `--query <PATTERN>` argument is a plain kind
/// name or an S-expression. Presence of `(`, `)`, or `@` is the
/// unambiguous signal documented in ADR-0032.
fn classify_pattern(pattern: &str) -> QueryType {
    if pattern.contains('(') || pattern.contains(')') || pattern.contains('@') {
        QueryType::SExpression
    } else {
        QueryType::KindFilter
    }
}

/// Execute the `query` subcommand.
///
/// Reads the source file, parses it via `tree-sitter-language-pack`,
/// and walks the tree emitting AST nodes as NDJSON lines. Supports
/// three modes: `--kinds` (aggregate counts), `--query <KIND>` (emit
/// nodes matching a single kind name), and `--tree` (emit every
/// named node). A final `query_summary` line is always emitted.
pub fn cmd_query(
    args: &QueryArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let validated = crate::path_safety::validate_path(&args.path, &workspace)?;
    if !validated.exists() {
        bail!("file does not exist: {}", validated.display());
    }
    let content = std::fs::read(&validated)
        .with_context(|| format!("cannot read {}", validated.display()))?;

    let lang_name = resolve_language_name(args.language.as_deref(), &validated, &content)?;

    let mut parser = get_parser(&lang_name)
        .with_context(|| format!("failed to load parser for language {lang_name}"))?;
    let tree = parser
        .parse(std::str::from_utf8(&content).unwrap_or(""))
        .or_else(|| parser.parse_bytes(&content))
        .with_context(|| format!("parser returned no tree for {lang_name}"))?;
    let root = tree.root_node();

    let mut match_count = 0usize;
    let mut node_count = 0usize;
    let show_positions = args.positions;

    if args.kinds {
        let mut kind_counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        walk_kinds(&root, &mut kind_counts, &mut node_count);
        for (kind, count) in &kind_counts {
            writer.write_event(&serde_json::json!({
                "type": "query_kind",
                "path": validated.display().to_string(),
                "language": lang_name,
                "kind": kind,
                "count": count,
            }))?;
        }
        match_count = kind_counts.len();
    } else if let Some(pattern) = args.query.as_deref() {
        match classify_pattern(pattern) {
            QueryType::KindFilter => {
                let wanted: Vec<String> = vec![pattern.to_owned()];
                walk_kind_filter(
                    &root,
                    &content,
                    &validated,
                    &lang_name,
                    &wanted,
                    &show_positions,
                    writer,
                    &mut match_count,
                    &mut node_count,
                )?;
            }
            QueryType::SExpression => {
                let lang = get_language(&lang_name).with_context(|| {
                    format!("failed to load Language for S-expression: {lang_name}")
                })?;
                walk_sexpr(
                    &root,
                    &content,
                    &validated,
                    &lang_name,
                    &lang,
                    pattern,
                    &show_positions,
                    writer,
                    &mut match_count,
                    &mut node_count,
                )?;
            }
        }
    } else if args.tree {
        walk_tree(
            &root,
            &content,
            &validated,
            &lang_name,
            &show_positions,
            writer,
            &mut match_count,
            &mut node_count,
        )?;
    } else {
        bail!("must specify one of --query <KIND>, --tree, or --kinds");
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    writer.write_event(&QuerySummary {
        r#type: "query_summary",
        path: validated.display().to_string(),
        language: lang_name,
        matches: match_count,
        total_nodes: node_count,
        elapsed_ms,
    })?;
    Ok(())
}

/// Resolve the language name from override, extension, or shebang.
pub(crate) fn resolve_language_name(
    override_lang: Option<&str>,
    path: &Path,
    content: &[u8],
) -> Result<String> {
    if let Some(name) = override_lang {
        if !has_language(name) {
            bail!("unsupported language override: {name}");
        }
        return Ok(name.to_owned());
    }
    match crate::syntax_check::detect_language_name(path, content) {
        Some(name) => Ok(name),
        None => bail!(
            "could not detect language for {}; pass --language <LANG>",
            path.display()
        ),
    }
}

fn node_text(source: &[u8], start: usize, end: usize) -> String {
    let end = end.min(source.len());
    let raw = source.get(start..end).unwrap_or(&[]);
    let s = String::from_utf8_lossy(raw);
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(240)
        .collect();
    if cleaned.is_empty() {
        "<empty>".to_owned()
    } else {
        cleaned
    }
}

/// Iterative DFS over `root` using an explicit stack of
/// `(node, child_index)` pairs. Counts each `Node::kind()` in
/// `kind_counts` and the total in `node_count`.
fn walk_kinds(
    root: &Node,
    kind_counts: &mut std::collections::BTreeMap<String, usize>,
    node_count: &mut usize,
) {
    // Stack of (node, next-child-index-to-visit).
    let mut stack: Vec<(Node, u32)> = Vec::with_capacity(64);
    stack.push((root.clone(), 0));
    while let Some((node, idx)) = stack.last() {
        let (node, _idx) = (node.clone(), *idx);
        // Pop before potentially pushing children.
        stack.pop();
        // Process this node.
        *kind_counts.entry(node.kind()).or_insert(0) += 1;
        *node_count += 1;
        // Push children in reverse so we visit them in order.
        let count = node.child_count() as u32;
        if count == 0 {
            continue;
        }
        // We want to visit children in order. The simplest: push the
        // last child first so it pops first, then second-to-last, etc.
        // But we also need to track the "next child to visit" index.
        // The cleanest iterative DFS: push (child[i+1], 0) BEFORE
        // processing child[i], so when we return, we move to next.
        // For simplicity here, push all children with index 0; they
        // will each be processed and their own children will be pushed.
        for i in (0..count).rev() {
            if let Some(child) = node.child(i) {
                stack.push((child, 0));
            }
        }
    }
}

/// Iterative DFS filtered by `wanted` kinds. Emits a `query_match`
/// NDJSON line for each node whose kind matches.
#[allow(clippy::too_many_arguments)]
fn walk_kind_filter(
    root: &Node,
    source: &[u8],
    path: &Path,
    lang_name: &str,
    wanted: &[String],
    show_positions: &bool,
    writer: &mut NdjsonWriter<impl Write>,
    match_count: &mut usize,
    node_count: &mut usize,
) -> Result<()> {
    let mut stack: Vec<Node> = vec![root.clone()];
    while let Some(node) = stack.pop() {
        *node_count += 1;
        let kind = node.kind();
        if wanted.iter().any(|w| w == &kind) {
            let start = node.start_position();
            let end = node.end_position();
            let mut event = serde_json::json!({
                "type": "query_match",
                "path": path.display().to_string(),
                "language": lang_name,
                "kind": kind,
                "is_named": node.is_named(),
                "text": node_text(source, node.start_byte(), node.end_byte()),
            });
            if *show_positions {
                event["start_byte"] = serde_json::json!(node.start_byte());
                event["end_byte"] = serde_json::json!(node.end_byte());
                event["start_line"] = serde_json::json!(start.row + 1);
                event["start_column"] = serde_json::json!(start.column + 1);
                event["end_line"] = serde_json::json!(end.row + 1);
                event["end_column"] = serde_json::json!(end.column + 1);
            }
            writer.write_event(&event)?;
            *match_count += 1;
        }
        let count = node.child_count() as u32;
        for i in (0..count).rev() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    Ok(())
}


/// Run a real tree-sitter S-expression query against `root` and emit
/// one `query_match` NDJSON event per captured node. The captured
/// name (e.g. `name` for `@name`) is exposed via the `capture_name`
/// field. Errors from `tree_sitter::Query::new` (malformed pattern)
/// surface as `anyhow::Error` with the S-expression text in context.
///
/// Re-parses the source via a fresh `tree_sitter::Parser` because
/// `QueryCursor::matches` consumes a `tree_sitter::Node<'_>` and the
/// legacy `root` from the language-pack parser is type-identical but
/// we'd rather make the new path self-contained for clarity.
#[allow(clippy::too_many_arguments)]
fn walk_sexpr(
    root: &Node,
    source: &[u8],
    path: &Path,
    lang_name: &str,
    lang: &tree_sitter::Language,
    pattern: &str,
    show_positions: &bool,
    writer: &mut NdjsonWriter<impl Write>,
    match_count: &mut usize,
    node_count: &mut usize,
) -> Result<()> {
    let _ = root; // Re-parse with a fresh parser to avoid borrow conflicts.
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(lang)
        .with_context(|| format!("failed to set language for S-expression query on {lang_name}"))?;
    let tree = parser
        .parse(source, None)
        .with_context(|| format!("parser returned no tree for S-expression query on {lang_name}"))?;
    let fresh_root = tree.root_node();

    let ts_query = TsQuery::new(lang, pattern)
        .with_context(|| format!("invalid S-expression: {pattern}"))?;
    let mut cursor = tree_sitter::QueryCursor::new();
    let capture_names = ts_query.capture_names();
    let mut matches = cursor.matches(&ts_query, fresh_root, source);

    while let Some(m) = matches.next() {
        for capture in m.captures {
            *node_count += 1;
            let node = capture.node;
            let capture_name = capture_names
                .get(capture.index as usize)
                .copied()
                .unwrap_or("");
            let start = node.start_position();
            let end = node.end_position();
            let mut event = serde_json::json!({
                "type": "query_match",
                "path": path.display().to_string(),
                "language": lang_name,
                "kind": node.kind(),
                "is_named": node.is_named(),
                "text": node_text(source, node.start_byte(), node.end_byte()),
                "capture_name": capture_name,
            });
            if *show_positions {
                event["start_byte"] = serde_json::json!(node.start_byte());
                event["end_byte"] = serde_json::json!(node.end_byte());
                event["start_line"] = serde_json::json!(start.row + 1);
                event["start_column"] = serde_json::json!(start.column + 1);
                event["end_line"] = serde_json::json!(end.row + 1);
                event["end_column"] = serde_json::json!(end.column + 1);
            }
            writer.write_event(&event)?;
            *match_count += 1;
        }
    }
    Ok(())
}

/// Iterative DFS that emits every named node.
#[allow(clippy::too_many_arguments)]
fn walk_tree(
    root: &Node,
    source: &[u8],
    path: &Path,
    lang_name: &str,
    show_positions: &bool,
    writer: &mut NdjsonWriter<impl Write>,
    match_count: &mut usize,
    node_count: &mut usize,
) -> Result<()> {
    let mut stack: Vec<Node> = vec![root.clone()];
    while let Some(node) = stack.pop() {
        *node_count += 1;
        if node.is_named() {
            let start = node.start_position();
            let end = node.end_position();
            let mut event = serde_json::json!({
                "type": "query_match",
                "path": path.display().to_string(),
                "language": lang_name,
                "kind": node.kind(),
                "is_named": true,
                "text": node_text(source, node.start_byte(), node.end_byte()),
            });
            if *show_positions {
                event["start_byte"] = serde_json::json!(node.start_byte());
                event["end_byte"] = serde_json::json!(node.end_byte());
                event["start_line"] = serde_json::json!(start.row + 1);
                event["start_column"] = serde_json::json!(start.column + 1);
                event["end_line"] = serde_json::json!(end.row + 1);
                event["end_column"] = serde_json::json!(end.column + 1);
            }
            writer.write_event(&event)?;
            *match_count += 1;
        }
        let count = node.child_count() as u32;
        for i in (0..count).rev() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_text_truncates_long_input() {
        let s = node_text(b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", 0, 500);
        assert_eq!(s.len(), 240);
    }

    #[test]
    fn node_text_handles_empty() {
        let s = node_text(b"hello", 3, 3);
        assert_eq!(s, "<empty>");
    }
}
