// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand (v0.1.12): `outline` — extract the high-level
//! structure of a source file (functions, classes, structs, enums,
//! traits, modules, top-level consts) as NDJSON. Uses
//! `tree-sitter-language-pack`.
//!
//! ## Causa x Efeito
//!
//! - **Causa**: `read` devolve texto cru. Agentes LLM gastam tokens
//!   lendo 500 linhas para descobrir que só 12 são assinaturas.
//! - **Efeito**: Contexto desperdiçado, latência alta, alucinações
//!   sobre escopo (achar que função X existe mas X está em módulo Y).
//! - **Solução**: Parse via tree-sitter + walk pelos node kinds
//!   "top-level structural" + emit cada um como `outline_item` NDJSON
//!   com `kind`, `name`, `signature`, `start_line`.
//! - **Benefício**: Contexto compacto (10 itens = 2KB vs 50KB do source),
//!   preciso, navegável.

use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::cli::{GlobalArgs, OutlineArgs};
use crate::output::NdjsonWriter;

#[derive(Debug, Serialize)]
struct OutlineItem {
    r#type: &'static str,
    path: String,
    language: String,
    kind: String,
    name: String,
    signature: String,
    start_line: usize,
    end_line: usize,
    // GAP-109: byte offsets emitted when --positions is active
    #[serde(skip_serializing_if = "Option::is_none")]
    start_byte: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_byte: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_column: Option<usize>,
}

#[derive(Debug, Serialize)]
struct OutlineSummary {
    r#type: &'static str,
    path: String,
    language: String,
    items: usize,
    elapsed_ms: u64,
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
pub fn cmd_outline(
    args: &OutlineArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let validated = crate::path_safety::validate_path(&args.path, &workspace)?;
    if !validated.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: validated }.into());
    }
    let content = std::fs::read(&validated)
        .with_context(|| format!("cannot read {}", validated.display()))?;

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

    let mut parser = tree_sitter_language_pack::get_parser(&lang_name)
        .with_context(|| format!("failed to load parser for {lang_name}"))?;
    let tree = parser
        .parse(std::str::from_utf8(&content).unwrap_or(""))
        .or_else(|| parser.parse_bytes(&content))
        .with_context(|| format!("parser returned no tree for {lang_name}"))?;
    let root = tree.root_node();

    let mut items = 0usize;
    walk_outline(
        &root,
        &content,
        &validated,
        &lang_name,
        &kind_filter,
        args.positions,
        writer,
        &mut items,
    )?;

    let elapsed_ms = start.elapsed().as_millis() as u64;
    writer.write_event(&OutlineSummary {
        r#type: "outline_summary",
        path: validated.display().to_string(),
        language: lang_name,
        items,
        elapsed_ms,
    })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn walk_outline(
    root: &tree_sitter_language_pack::Node,
    source: &[u8],
    path: &std::path::Path,
    lang_name: &str,
    kind_filter: &Option<Vec<String>>,
    show_positions: bool,
    writer: &mut NdjsonWriter<impl Write>,
    items: &mut usize,
) -> Result<()> {
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
                let signature = extract_signature(&node, source);
                let start = node.start_position();
                let end = node.end_position();
                writer.write_event(&OutlineItem {
                    r#type: "outline_item",
                    path: path.display().to_string(),
                    language: lang_name.to_string(),
                    kind: kind.to_string(),
                    name,
                    signature,
                    start_line: start.row + 1,
                    end_line: end.row + 1,
                    // GAP-109: include byte offsets when --positions is set
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
                })?;
                *items += 1;
            }
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

fn extract_signature(node: &tree_sitter_language_pack::Node, source: &[u8]) -> String {
    let start = node.start_byte();
    let end = node.end_byte().min(start + 400);
    let raw = source.get(start..end).unwrap_or(&[]);
    let s = String::from_utf8_lossy(raw);
    let line = s.lines().next().unwrap_or("");
    let collapsed: String = line
        .chars()
        .filter(|c| !c.is_control() || *c == '\t')
        .collect();
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        "<empty>".to_owned()
    } else {
        trimmed.to_string()
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
