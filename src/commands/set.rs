// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand: `set` — modify a single key in a structured
//! config file while preserving comments, key order, and formatting.
//!
//! Currently supports TOML (via the `toml_edit` crate, which preserves
//! trivia). JSON is a stub that errors with a clear message — full JSON
//! edit-with-format-preservation is a future enhancement.
//!
//! Workload: I/O-bound (single-file read + edit + atomic write).
//! Parallelism: none — one file; preserve-format edit is sequential.

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::cli::{GlobalArgs, SetArgs};
use crate::commands::resolve_backup;
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
#[tracing::instrument(skip_all, fields(command = "set"))]
pub fn cmd_set(
    args: &SetArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let resolved = resolve_backup(&args.backup_opts, defaults);

    let validated = crate::path_safety::validate_path(&args.path, &workspace)?;
    if !validated.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: validated }.into());
    }

    let original =
        crate::file_io::read_file_string(&validated, global.effective_max_filesize())?;

    let (new_content, old_value, format) = match validated.extension().and_then(|s| s.to_str()) {
        Some("toml") => toml_set(&original, &args.key_path, &args.value)?,
        Some("json") => json_set(&original, &args.key_path, &args.value)?,
        other => {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: format!(
                    "unsupported format for `set` (extension: {other:?}); supported: toml, json"
                ),
            }
            .into());
        }
    };

    let opts = AtomicWriteOptions {
        backup: resolved.backup,
        syntax_check: false,
        retention: resolved.retention,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup: resolved.keep,
        durability: crate::platform::Durability::Auto,
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
        mtime_preserved: None,
        risk_assessment: None,
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
    let old_value = get_toml_value(&doc, key_path);
    set_toml_path(&mut doc, key_path, value)?;
    let new_content = doc.to_string();
    Ok((new_content, old_value, "toml"))
}

fn set_toml_path(doc: &mut toml_edit::DocumentMut, key_path: &str, value: &str) -> Result<()> {
    let segments: Vec<&str> = key_path.split('.').collect();
    if segments.is_empty() {
        return Ok(());
    }
    if segments.len() == 1 {
        doc[segments[0]] = parse_toml_value(value);
        return Ok(());
    }
    let mut current = doc.as_item_mut();
    for (i, seg) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        if is_last {
            if let Some(table) = current.as_table_mut() {
                table[seg] = parse_toml_value(value);
            } else {
                // GAP-102: the parent is a scalar, not a table.
                let parent_path = segments[..i].join(".");
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: format!(
                        "cannot set '{key_path}': '{}' is a scalar value, not a table",
                        parent_path
                    ),
                }
                .into());
            }
            return Ok(());
        }
        let table = match current.as_table_mut() {
            Some(t) => t,
            None => {
                let traversed = segments[..=i].join(".");
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: format!("cannot set '{key_path}': '{}' is not a table", traversed),
                }
                .into());
            }
        };
        // Entry API: one probe — insert intermediate table or reborrow existing.
        use toml_edit::Entry;
        current = match table.entry(seg) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(toml_edit::Item::Table(toml_edit::Table::new())),
        };
    }
    Ok(())
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
    if !crate::output::check_json_depth(&value_json, crate::constants::MAX_JSON_DEPTH) {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "JSON nesting depth exceeds maximum of {}",
                crate::constants::MAX_JSON_DEPTH
            ),
        }
        .into());
    }
    let pointer = crate::output::dotted_to_json_pointer(key_path);
    let old_value = value_json.pointer(&pointer).map(|v| match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    });
    apply_json_pointer(&mut value_json, &pointer, value);
    // Pretty-print is intentional: human-edited config files, not NDJSON stdout.
    let mut new_content = serde_json::to_string_pretty(&value_json)?;
    // G-022: POSIX text files end with newline; keep agent diffs clean.
    if crate::constants::ENSURE_TRAILING_NEWLINE_JSON && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    Ok((new_content, old_value, "json"))
}

fn apply_json_pointer(root: &mut serde_json::Value, pointer: &str, value: &str) {
    use serde_json::Value;
    // Split RFC 6901 pointer and unescape tokens (`~1` → `/`, `~0` → `~`).
    let segments: Vec<String> = pointer
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(unescape_json_pointer_token)
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
                    map.insert(seg.clone(), parse_json_value(value));
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
        // Entry API: single map probe (no contains_key + get_mut double lookup).
        match current {
            Value::Object(map) => {
                current = map
                    .entry(seg.clone())
                    .or_insert_with(|| Value::Object(serde_json::Map::new()));
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

/// RFC 6901 token unescape: `~1` → `/`, then `~0` → `~` (order matters).
fn unescape_json_pointer_token(seg: &str) -> String {
    seg.replace("~1", "/").replace("~0", "~")
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

fn get_toml_value(doc: &toml_edit::DocumentMut, key_path: &str) -> Option<String> {
    let segments: Vec<&str> = key_path.split('.').collect();
    if segments.is_empty() {
        return None;
    }
    let mut current: &toml_edit::Item = doc.as_item();
    for seg in &segments {
        match current.as_table() {
            Some(table) => match table.get(seg) {
                Some(item) => current = item,
                None => return None,
            },
            None => return None,
        }
    }
    if let Some(v) = current.as_value() {
        match v.as_str() {
            Some(s) => Some(s.to_owned()),
            None => Some(v.to_string().trim().to_owned()),
        }
    } else {
        Some(current.to_string().trim().to_owned())
    }
}

#[allow(dead_code)]
fn _path_buf_marker() -> PathBuf {
    PathBuf::new()
}
