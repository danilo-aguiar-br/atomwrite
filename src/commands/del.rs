// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand: `del` — remove a key at a dotted path in a
//! structured config file (TOML or JSON) while preserving formatting.
//!
//! Workload: I/O-bound (single-file read + edit + atomic write).
//! Parallelism: none — one file.

use std::io::Write;
use std::time::Instant;

use anyhow::Result;
use serde::Serialize;

use crate::atomic::AtomicWriteOptions;
use crate::cli::{DelArgs, GlobalArgs};
use crate::commands::resolve_backup;
use crate::output::NdjsonWriter;

/// Terminal event of `del`: the key removed at a dotted path.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct DelResult {
    r#type: &'static str,
    path: String,
    key_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    removed_value: Option<String>,
    existed: bool,
    format: &'static str,
    /// Write strategy the atomic layer used: `rename`, `inplace` or `copyback`.
    ///
    /// Absent when the key was already missing, because that run took no write
    /// path and a verdict would claim a mechanism it never used.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    write_strategy: Option<&'static str>,
    /// Why the requested backup was skipped, when the directory refused it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    backup_skipped: Option<String>,
    elapsed_ms: u64,
}

/// Execute the `del` subcommand.
///
/// Removes the key at `key_path` from the target structured config file
/// (TOML or JSON) and writes the result back atomically. Comments and
/// key order are preserved in TOML.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` when the target file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` when the path escapes the workspace.
/// Returns `AtomwriteError::LockTimeout` when the update lock is not acquired in time.
/// Returns `AtomwriteError::FileTooLarge` when the file exceeds the effective max filesize.
/// Returns `AtomwriteError::InvalidInput` when the file is not valid UTF-8, is not TOML or JSON,
/// exceeds the JSON nesting depth, or the key is absent while `--force-missing` is not set.
/// Returns `AtomwriteError::Io` when reading or writing the file fails.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
#[tracing::instrument(skip_all, fields(command = "del"))]
pub fn cmd_del(
    args: &DelArgs,
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
    // A1: lock BEFORE reading, hold it to the commit.
    let _update_lock = crate::lock::acquire_for_update(&validated)?;
    let original = crate::file_io::read_file_string(&validated, global.effective_max_filesize())?;

    let (new_content, removed_value, existed, format) =
        match validated.extension().and_then(|s| s.to_str()) {
            Some("toml") => {
                let mut doc = crate::commands::set::parse_toml_doc(&original)?;
                let removed = remove_toml_path(&mut doc, &args.key_path);
                let existed = removed.is_some();
                (doc.to_string(), removed, existed, "toml")
            }
            Some("json") => {
                let mut v = crate::commands::set::parse_json_source(&original)?;
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
                let removed = v.pointer(&pointer).map(|x| match x {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                });
                let existed = removed.is_some();
                remove_json_pointer(&mut v, &pointer);
                // Pretty-print is intentional: human-edited config files, not NDJSON stdout.
                let new_content = serde_json::to_string_pretty(&v)?;
                (new_content, removed, existed, "json")
            }
            other => {
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: format!(
                        "unsupported format for `del` (extension: {other:?}); supported: toml, json"
                    ),
                }
                .into());
            }
        };

    if !existed && !args.force_missing {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "key '{}' not found in {}",
                args.key_path,
                validated.display()
            ),
        }
        .into());
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
            write_strategy: None,
            backup_skipped: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        };
        writer.write_event(&output)?;
        return Ok(());
    }

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "del".into(),
            path: validated.display().to_string(),
            would_modify: existed,
            details: Some(format!("del {}", args.key_path)),
            // Removing a dotted key writes no payload the patterns describe.
            risk_assessment: None,
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let opts = AtomicWriteOptions {
        backup: resolved.backup,
        syntax_check: false,
        retention: resolved.retention,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: args.strict.strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup: resolved.keep,
        durability: crate::platform::Durability::Auto,
    };

    // The content was computed from `original`, so a file that changed since
    // that read is refused under the lock instead of being overwritten.
    let base = crate::checksum::hash_bytes(original.as_bytes());
    let result = crate::atomic::atomic_write_based(
        &validated,
        new_content.as_bytes(),
        &opts,
        &workspace,
        Some(&base),
    )?;

    let output = DelResult {
        r#type: "del",
        path: validated.display().to_string(),
        key_path: args.key_path.clone(),
        removed_value,
        existed: true,
        format,
        write_strategy: Some(result.write_strategy),
        backup_skipped: result.backup_skipped,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    writer.write_event(&output)?;
    Ok(())
}

fn remove_json_pointer(root: &mut serde_json::Value, pointer: &str) {
    use serde_json::Value;
    let segments: Vec<String> = pointer
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect();
    if segments.is_empty() {
        return;
    }
    let mut current = root;
    for seg in &segments[..segments.len() - 1] {
        match current {
            Value::Object(map) => match map.get_mut(seg) {
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
    let last = &segments[segments.len() - 1];
    match current {
        Value::Object(map) => {
            map.remove(last);
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
    crate::commands::toml_path::remove(doc, key_path)
}
