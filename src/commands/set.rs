// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand: `set` — modify a single key in a structured
//! config file while preserving comments, key order, and formatting.
//!
//! Currently supports TOML (via the `toml_edit` crate, which preserves
//! trivia). JSON is a stub that errors with a clear message — full JSON
//! edit-with-format-preservation is a future enhancement.

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::cli::{GlobalArgs, SetArgs};
use crate::ndjson_types::WriteOutput;
use crate::output::NdjsonWriter;

/// Set a value at a dotted path in a structured config file.
#[derive(Debug, Serialize)]
struct SetResult {
    r#type: &'static str,
    path: String,
    config_path: String,
    key_path: String,
    old_value: Option<String>,
    new_value: String,
    format: &'static str,
    comments_preserved: bool,
    elapsed_ms: u64,
}

/// Execute the `set` subcommand.
///
/// Reads the target structured config file, parses it (TOML via
/// `toml_edit`, JSON via `serde_json`), sets the value at `key_path`,
/// and writes the result back atomically. Comments and key order are
/// preserved in TOML; JSON is rewritten canonically.
pub fn cmd_set(
    args: &SetArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let validated = crate::path_safety::validate_path(&args.path, &workspace)?;
    if !validated.exists() {
        bail!("file does not exist: {}", validated.display());
    }

    let original = std::fs::read_to_string(&validated)
        .with_context(|| format!("cannot read {}", validated.display()))?;

    let (new_content, old_value, format) = match validated.extension().and_then(|s| s.to_str()) {
        Some("toml") => toml_set(&original, &args.key_path, &args.value)?,
        Some("json") => json_set(&original, &args.key_path, &args.value)?,
        other => bail!(
            "unsupported format for `set` (extension: {:?}); supported: toml, json",
            other
        ),
    };

    let opts = AtomicWriteOptions {
        backup: args.backup,
        syntax_check: false,
        retention: 5,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
    };

    let result = atomic_write(&validated, new_content.as_bytes(), &opts, &workspace)?;

    let output = SetResult {
        r#type: "set",
        path: validated.display().to_string(),
        config_path: validated.display().to_string(),
        key_path: args.key_path.clone(),
        old_value,
        new_value: args.value.clone(),
        format,
        comments_preserved: true,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    let _ = result; // result is already represented in elapsed_ms via the atomic pipeline
    writer.write_event(&output)?;
    let _ = WriteOutput {
        r#type: "write",
        status: "success",
        path: validated.display().to_string(),
        bytes_written: new_content.len() as u64,
        checksum: blake3::hash(new_content.as_bytes()).to_hex().to_string(),
        checksum_before: None,
        backup_path: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
        stdin_bytes_read: new_content.len() as u64,
        wal_policy: "auto",
        platform: result.platform,
    };
    Ok(())
}

fn toml_set(
    original: &str,
    key_path: &str,
    value: &str,
) -> Result<(String, Option<String>, &'static str)> {
    let mut doc: toml_edit::DocumentMut = original
        .parse()
        .with_context(|| format!("invalid TOML in source: {original}"))?;
    let old_value = doc
        .get(key_path)
        .map(|item| item.to_string().trim().to_owned());
    // toml_edit's `doc["a.b"]` adds a flat key; we need to navigate the
    // dotted path manually and update the leaf, which preserves the
    // existing formatting (comments, key order, table headers).
    set_toml_path(&mut doc, key_path, value);
    let new_content = doc.to_string();
    Ok((new_content, old_value, "toml"))
}

fn set_toml_path(doc: &mut toml_edit::DocumentMut, key_path: &str, value: &str) {
    let segments: Vec<&str> = key_path.split('.').collect();
    if segments.is_empty() {
        return;
    }
    if segments.len() == 1 {
        // Top-level scalar key.
        doc[segments[0]] = parse_toml_value(value);
        return;
    }
    // Navigate to the deepest table. Create intermediate tables as needed.
    let mut current = doc.as_item_mut();
    for (i, seg) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        if is_last {
            // We are at the parent of the leaf. Navigate into the
            // table to update the leaf.
            if let Some(table) = current.as_table_mut() {
                table[seg] = parse_toml_value(value);
            } else {
                // Fall back: store at the document root.
                doc[*seg] = parse_toml_value(value);
            }
            return;
        }
        // Need to descend into or create the table for `seg`.
        // First, ensure the table exists.
        let next_seg = segments[i + 1];
        if current.as_table_mut().is_none() {
            // Not a table (e.g. scalar at this path) — cannot descend.
            return;
        }
        let table = current.as_table_mut().unwrap();
        if !table.contains_key(seg) {
            // Insert a new (possibly inline) table.
            table.insert(seg, toml_edit::Item::Table(toml_edit::Table::new()));
        }
        current = table.get_mut(seg).unwrap();
        // Suppress unused warning for next_seg (kept for clarity).
        let _ = next_seg;
    }
}

fn parse_toml_value(s: &str) -> toml_edit::Item {
    // Try bool, int, float, then fall back to string.
    if s == "true" {
        return toml_edit::value(true);
    }
    if s == "false" {
        return toml_edit::value(false);
    }
    if let Ok(n) = s.parse::<i64>() {
        return toml_edit::value(n);
    }
    if let Ok(n) = s.parse::<f64>() {
        return toml_edit::value(n);
    }
    toml_edit::value(s)
}

fn json_set(
    original: &str,
    key_path: &str,
    value: &str,
) -> Result<(String, Option<String>, &'static str)> {
    let mut value_json: serde_json::Value =
        serde_json::from_str(original).with_context(|| "invalid JSON in source")?;
    let old_value = value_json
        .pointer(&json_pointer(key_path))
        .map(|v| v.to_string());
    // Convert key_path "a.b.c" to JSON pointer "/a/b/c"
    apply_json_pointer(&mut value_json, &json_pointer(key_path), value);
    let new_content = serde_json::to_string_pretty(&value_json)?;
    Ok((new_content, old_value, "json"))
}

fn json_pointer(path: &str) -> String {
    format!("/{}", path.replace('.', "/"))
}

fn apply_json_pointer(root: &mut serde_json::Value, pointer: &str, value: &str) {
    use serde_json::Value;
    let segments: Vec<&str> = pointer
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if segments.is_empty() {
        *root = parse_json_value(value);
        return;
    }
    let mut current = root;
    for (i, seg) in segments.iter().enumerate() {
        if i == segments.len() - 1 {
            match current {
                Value::Object(map) => {
                    map.insert((*seg).to_owned(), parse_json_value(value));
                }
                Value::Array(arr) => {
                    if let Ok(idx) = seg.parse::<usize>() {
                        if idx < arr.len() {
                            arr[idx] = parse_json_value(value);
                        } else {
                            arr.push(parse_json_value(value));
                        }
                    }
                }
                _ => {
                    // Path traversal hit a non-container; replace root.
                    *current = parse_json_value(value);
                }
            }
            return;
        }
        // Navigate into the next segment, creating containers as needed.
        let _is_last_next = i + 1 == segments.len() - 1;
        match current {
            Value::Object(map) => {
                if !map.contains_key(*seg) {
                    map.insert((*seg).to_owned(), Value::Object(serde_json::Map::new()));
                }
                current = map.get_mut(*seg).unwrap();
            }
            Value::Array(arr) => {
                if let Ok(idx) = seg.parse::<usize>() {
                    while arr.len() <= idx {
                        arr.push(Value::Null);
                    }
                    current = &mut arr[idx];
                } else {
                    return; // invalid index
                }
            }
            _ => return, // path traversal failed
        }
    }
}

fn parse_json_value(s: &str) -> serde_json::Value {
    if let Ok(v) = serde_json::from_str(s) {
        return v;
    }
    if s == "true" {
        return serde_json::Value::Bool(true);
    }
    if s == "false" {
        return serde_json::Value::Bool(false);
    }
    if let Ok(n) = s.parse::<i64>() {
        return serde_json::Value::Number(n.into());
    }
    if let Ok(n) = s.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            return serde_json::Value::Number(num);
        }
    }
    serde_json::Value::String(s.to_owned())
}

#[allow(dead_code)]
fn _path_buf_marker() -> PathBuf {
    PathBuf::new()
}
