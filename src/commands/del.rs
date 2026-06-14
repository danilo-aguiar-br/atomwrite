// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand: `del` — remove a key at a dotted path in a
//! structured config file (TOML or JSON) while preserving formatting.

use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::cli::{DelArgs, GlobalArgs};
use crate::output::NdjsonWriter;

#[derive(Debug, Serialize)]
struct DelResult {
    r#type: &'static str,
    path: String,
    key_path: String,
    removed_value: Option<String>,
    existed: bool,
    format: &'static str,
    elapsed_ms: u64,
}

/// Execute the `del` subcommand.
///
/// Removes the key at `key_path` from the target structured config file
/// (TOML or JSON) and writes the result back atomically. Comments and
/// key order are preserved in TOML.
pub fn cmd_del(
    args: &DelArgs,
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

    let (new_content, removed_value, existed, format) =
        match validated.extension().and_then(|s| s.to_str()) {
            Some("toml") => {
                let mut doc: toml_edit::DocumentMut =
                    original.parse().with_context(|| "invalid TOML in source")?;
                let removed = remove_toml_path(&mut doc, &args.key_path);
                let existed = removed.is_some();
                (doc.to_string(), removed, existed, "toml")
            }
            Some("json") => {
                let mut v: serde_json::Value =
                    serde_json::from_str(&original).with_context(|| "invalid JSON in source")?;
                let pointer = format!("/{}", args.key_path.replace('.', "/"));
                let removed = v.pointer(&pointer).map(|x| x.to_string());
                let existed = removed.is_some();
                remove_json_pointer(&mut v, &pointer);
                let new_content = serde_json::to_string_pretty(&v)?;
                (new_content, removed, existed, "json")
            }
            other => bail!(
                "unsupported format for `del` (extension: {:?}); supported: toml, json",
                other
            ),
        };

    if !existed && !args.force_missing {
        bail!(
            "key path '{}' not found in {}",
            args.key_path,
            validated.display()
        );
    }

    if !existed && args.force_missing {
        // Skip the write: nothing to delete.
        let output = DelResult {
            r#type: "del",
            path: validated.display().to_string(),
            key_path: args.key_path.clone(),
            removed_value,
            existed: false,
            format,
            elapsed_ms: start.elapsed().as_millis() as u64,
        };
        writer.write_event(&output)?;
        return Ok(());
    }

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

    let _ = atomic_write(&validated, new_content.as_bytes(), &opts, &workspace)?;

    let output = DelResult {
        r#type: "del",
        path: validated.display().to_string(),
        key_path: args.key_path.clone(),
        removed_value,
        existed: true,
        format,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    writer.write_event(&output)?;
    Ok(())
}

fn remove_json_pointer(root: &mut serde_json::Value, pointer: &str) {
    use serde_json::Value;
    let segments: Vec<&str> = pointer
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if segments.is_empty() {
        return;
    }
    let mut current = root;
    for seg in &segments[..segments.len() - 1] {
        match current {
            Value::Object(map) => match map.get_mut(*seg) {
                Some(v) => current = v,
                None => return,
            },
            Value::Array(arr) => {
                if let Ok(idx) = seg.parse::<usize>() {
                    if idx < arr.len() {
                        current = &mut arr[idx];
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            }
            _ => return,
        }
    }
    let last = segments[segments.len() - 1];
    match current {
        Value::Object(map) => {
            map.remove::<str>(last);
        }
        Value::Array(arr) => {
            if let Ok(idx) = last.parse::<usize>() {
                if idx < arr.len() {
                    arr.remove(idx);
                }
            }
        }
        _ => {}
    }
}

/// Remove a value at a dotted path in a `toml_edit::DocumentMut`.
/// `toml_edit::DocumentMut::remove` treats dotted keys as a single
/// literal name; this helper manually descends through `Table`s.
fn remove_toml_path(doc: &mut toml_edit::DocumentMut, key_path: &str) -> Option<String> {
    let segments: Vec<&str> = key_path.split('.').collect();
    if segments.is_empty() {
        return None;
    }
    let last = *segments.last().unwrap();
    let mut current: &mut toml_edit::Item = doc.as_item_mut();
    for seg in &segments[..segments.len() - 1] {
        match current.as_table_mut() {
            Some(table) => match table.get_mut(seg) {
                Some(item) => current = item,
                None => return None,
            },
            None => return None,
        }
    }
    let table = current.as_table_mut()?;
    let removed = table.get(last).map(|item| item.to_string());
    table.remove(last);
    removed
}
