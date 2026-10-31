// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand: `get` — read a value at a dotted path in a
//! structured config file (TOML or JSON).
//!
//! Workload: I/O-bound (single-file read + structured parse).
//! Parallelism: none — one file, parse cost below thread overhead.

use std::io::Write;

use anyhow::Result;
use serde::Serialize;

use crate::cli::{GetArgs, GlobalArgs};
use crate::output::NdjsonWriter;

/// Terminal event of `get`: the value found at a dotted path.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct GetResult {
    r#type: &'static str,
    path: String,
    key_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    found: bool,
    format: &'static str,
    elapsed_ms: u64,
}

/// Execute the `get` subcommand.
///
/// Reads the target structured config file (TOML or JSON) and emits
/// the value at `key_path` as a single NDJSON line.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` when the target file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` when the path escapes the workspace.
/// Returns `AtomwriteError::FileTooLarge` when the file exceeds the effective max filesize.
/// Returns `AtomwriteError::InvalidInput` when the file is not valid UTF-8, is not TOML or JSON,
/// exceeds the JSON nesting depth, or the key is absent.
/// Returns `AtomwriteError::Io` when the file cannot be read.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
#[tracing::instrument(skip_all, fields(command = "get"))]
pub fn cmd_get(
    args: &GetArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = std::time::Instant::now();
    let workspace = global.resolve_workspace()?;
    let validated = crate::path_safety::validate_path(&args.path, &workspace)?;
    if !validated.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: validated }.into());
    }
    let content = crate::file_io::read_file_string(&validated, global.effective_max_filesize())?;

    let (value, found, format) = match validated.extension().and_then(|s| s.to_str()) {
        Some("toml") => {
            let doc = crate::commands::set::parse_toml_doc(&content)?;
            let (val, found) = get_toml_path(&doc, &args.key_path);
            (val, found, "toml")
        }
        Some("json") => {
            let v = crate::commands::set::parse_json_source(&content)?;
            if !crate::output::check_json_depth(&v, crate::constants::MAX_JSON_DEPTH) {
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: format!(
                        "JSON nesting depth exceeds maximum of {}",
                        crate::constants::MAX_JSON_DEPTH
                    ),
                }
                .into());
            }
            let pointer = crate::output::dotted_to_json_pointer(&args.key_path);
            let val = v.pointer(&pointer).map(|x| match x {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            });
            (val, v.pointer(&pointer).is_some(), "json")
        }
        other => {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: format!(
                    "unsupported format for `get` (extension: {other:?}); supported: toml, json"
                ),
            }
            .into());
        }
    };

    if !found {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "key '{}' not found in {}",
                args.key_path,
                validated.display()
            ),
        }
        .into());
    }

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
/// Delegates to [`crate::commands::toml_path`], the single walk shared with
/// `set` and `del`; it descends through standard and inline tables alike.
fn get_toml_path(doc: &toml_edit::DocumentMut, key_path: &str) -> (Option<String>, bool) {
    match crate::commands::toml_path::get(doc, key_path) {
        Some(value) => (Some(value), true),
        None => (None, false),
    }
}
