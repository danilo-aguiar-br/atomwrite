// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand: `get` — read a value at a dotted path in a
//! structured config file (TOML or JSON).

use std::io::Write;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::cli::{GetArgs, GlobalArgs};
use crate::output::NdjsonWriter;

#[derive(Debug, Serialize)]
struct GetResult {
    r#type: &'static str,
    path: String,
    key_path: String,
    value: Option<String>,
    found: bool,
    format: &'static str,
    elapsed_ms: u64,
}

/// Execute the `get` subcommand.
///
/// Reads the target structured config file (TOML or JSON) and emits
/// the value at `key_path` as a single NDJSON line.
pub fn cmd_get(
    args: &GetArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = std::time::Instant::now();
    let workspace = global.resolve_workspace()?;
    let validated = crate::path_safety::validate_path(&args.path, &workspace)?;
    if !validated.exists() {
        bail!("file does not exist: {}", validated.display());
    }
    let content = std::fs::read_to_string(&validated)
        .with_context(|| format!("cannot read {}", validated.display()))?;

    let (value, found, format) = match validated.extension().and_then(|s| s.to_str()) {
        Some("toml") => {
            let doc: toml_edit::DocumentMut =
                content.parse().with_context(|| "invalid TOML in source")?;
            let (val, found) = get_toml_path(&doc, &args.key_path);
            (val, found, "toml")
        }
        Some("json") => {
            let v: serde_json::Value =
                serde_json::from_str(&content).with_context(|| "invalid JSON in source")?;
            let pointer = format!("/{}", args.key_path.replace('.', "/"));
            let val = v.pointer(&pointer).map(|x| x.to_string());
            (val, v.pointer(&pointer).is_some(), "json")
        }
        other => bail!(
            "unsupported format for `get` (extension: {:?}); supported: toml, json",
            other
        ),
    };

    let output = GetResult {
        r#type: "get",
        path: validated.display().to_string(),
        key_path: args.key_path.clone(),
        value,
        found,
        format,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    writer.write_event(&output)?;
    Ok(())
}

/// Navigate a dotted path in a `toml_edit::DocumentMut` and return the
/// string representation of the leaf value (if any).
///
/// `toml_edit::DocumentMut::get` treats dotted keys as a single literal
/// name; this helper manually descends through `Table`s.
fn get_toml_path(doc: &toml_edit::DocumentMut, key_path: &str) -> (Option<String>, bool) {
    let segments: Vec<&str> = key_path.split('.').collect();
    if segments.is_empty() {
        return (None, false);
    }
    let mut current: &toml_edit::Item = doc.as_item();
    for (i, seg) in segments.iter().enumerate() {
        match current.as_table() {
            Some(table) => match table.get(seg) {
                Some(item) => current = item,
                None => return (None, false),
            },
            None => return (None, false),
        }
        let _ = i;
    }
    (Some(current.to_string().trim().to_owned()), true)
}
