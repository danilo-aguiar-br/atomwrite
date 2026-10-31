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

use anyhow::Result;
use serde::Serialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::cli::{GlobalArgs, SetArgs};
use crate::commands::resolve_backup;
use crate::output::NdjsonWriter;

/// Terminal event of `set`: a value written at a dotted path in a structured config file.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct SetResult {
    r#type: &'static str,
    path: String,
    config_path: String,
    key_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
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
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` when the target file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` when the path escapes the workspace.
/// Returns `AtomwriteError::LockTimeout` when the update lock is not acquired in time.
/// Returns `AtomwriteError::FileTooLarge` when the file exceeds the effective max filesize.
/// Returns `AtomwriteError::InvalidInput` when the file is not valid UTF-8, is not TOML or JSON,
/// or exceeds the JSON nesting depth.
/// Returns `AtomwriteError::Io` when reading or writing the file fails.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
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

    // A1: lock BEFORE reading, hold it to the commit.
    let _update_lock = crate::lock::acquire_for_update(&validated)?;
    let original = crate::file_io::read_file_string(&validated, global.effective_max_filesize())?;

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

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "set".into(),
            path: validated.display().to_string(),
            would_modify: new_content != original,
            details: Some(format!("set {} = {}", args.key_path, args.value)),
            // A dotted-key edit is a TOML value, never a payload the content
            // patterns describe, so there is nothing to assess.
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

    atomic_write(&validated, new_content.as_bytes(), &opts, &workspace)?;

    // JSON carries no comments, so only TOML can lose them.
    let comments_preserved = format != "toml" || comments_survive(&original, &new_content);
    let output = SetResult {
        r#type: "set",
        path: validated.display().to_string(),
        config_path: validated.display().to_string(),
        key_path: args.key_path.clone(),
        old_value,
        new_value: args.value.clone(),
        format,
        comments_preserved,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    writer.write_event(&output)?;
    Ok(())
}

/// Parse the TOML text of a `get`/`set`/`del` source.
///
/// The error is `InvalidInput` (exit 65, NDJSON envelope) and names only the
/// line, the column and a short cause. It never quotes the file: a config file
/// may hold secrets, and a parse error is the one place the text would leak.
///
/// # Errors
///
/// Returns [`crate::error::AtomwriteError::InvalidInput`] when `text` is not
/// valid TOML. The message carries the 1-based line and column plus the
/// parser's own cause, truncated to
/// [`crate::constants::PATCH_ERROR_SNIPPET_CHARS`] characters.
pub(crate) fn parse_toml_doc(
    text: &str,
) -> std::result::Result<toml_edit::DocumentMut, crate::error::AtomwriteError> {
    text.parse().map_err(|e: toml_edit::TomlError| {
        let cause: String = e
            .message()
            .chars()
            .take(crate::constants::PATCH_ERROR_SNIPPET_CHARS)
            .collect();
        let place = e.span().map_or_else(String::new, |span| {
            let (line, column) = line_and_column(text, span.start);
            format!(" at line {line} column {column}")
        });
        crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid TOML in source{place}: {cause}"),
        }
    })
}

/// Parse the JSON text of a `get`/`set`/`del` source (same no-leak rule as
/// [`parse_toml_doc`]: line, column and a fixed cause, never file text).
///
/// # Errors
///
/// Returns [`crate::error::AtomwriteError::InvalidInput`] when `text` is not
/// valid JSON. The message carries the 1-based line and column plus one of
/// four fixed causes (end of input, syntax, unexpected data, read error), so
/// the document text itself is never echoed back.
pub(crate) fn parse_json_source(
    text: &str,
) -> std::result::Result<serde_json::Value, crate::error::AtomwriteError> {
    serde_json::from_str(text).map_err(|e| {
        let cause = match e.classify() {
            serde_json::error::Category::Eof => "unexpected end of input",
            serde_json::error::Category::Syntax => "syntax error",
            serde_json::error::Category::Data => "unexpected data",
            serde_json::error::Category::Io => "read error",
        };
        crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "invalid JSON in source at line {} column {}: {cause}",
                e.line(),
                e.column()
            ),
        }
    })
}

/// 1-based line and column (in characters) of byte `offset` in `text`.
fn line_and_column(text: &str, offset: usize) -> (usize, usize) {
    let mut end = offset.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let before = &text[..end];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |tail| tail.chars().count())
        + 1;
    (line, column)
}

/// Set a dotted key path in a TOML document and return the re-serialized text.
///
/// # Errors
///
/// Returns whatever [`parse_toml_doc`] raises when `original` is not valid
/// TOML, and whatever [`crate::commands::toml_path::set`] raises when
/// `key_path` walks through a scalar instead of a table (published as exit 65
/// `INVALID_INPUT`).
pub(crate) fn toml_set(
    original: &str,
    key_path: &str,
    value: &str,
) -> Result<(String, Option<String>, &'static str)> {
    let mut doc = parse_toml_doc(original)?;
    let old_value = get_toml_value(&doc, key_path);
    set_toml_path(&mut doc, key_path, value)?;
    let new_content = doc.to_string();
    Ok((new_content, old_value, "toml"))
}

fn set_toml_path(doc: &mut toml_edit::DocumentMut, key_path: &str, value: &str) -> Result<()> {
    // GAP-102 keeps its message: the shared walk reports a scalar in the middle
    // of the path as a typed `InvalidInput`, which the caller publishes as exit
    // 65 like before.
    Ok(crate::commands::toml_path::set(
        doc,
        key_path,
        parse_toml_value(value),
    )?)
}

/// Byte offset just past the TOML string literal that starts at `start`.
fn skip_toml_string(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let delim: &[u8] = if bytes[start..].starts_with(b"\"\"\"") {
        b"\"\"\""
    } else if bytes[start..].starts_with(b"'''") {
        b"'''"
    } else if bytes[start] == b'"' {
        b"\""
    } else {
        b"'"
    };
    let basic = bytes[start] == b'"';
    let mut i = start + delim.len();
    while i < bytes.len() {
        if bytes[i..].starts_with(delim) {
            return i + delim.len();
        }
        // A backslash escapes the next byte inside basic strings only.
        i += if basic && bytes[i] == b'\\' { 2 } else { 1 };
    }
    bytes.len()
}

/// Collect every `# comment` of a TOML text, ignoring `#` inside strings.
fn toml_comments(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut comments = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'#' => {
                let end = text[i..].find('\n').map_or(text.len(), |n| i + n);
                comments.push(text[i..end].trim_end().to_owned());
                i = end;
            }
            b'"' | b'\'' => i = skip_toml_string(text, i),
            _ => i += 1,
        }
    }
    comments
}

/// True when every comment of `before` is still present in `after`.
fn comments_survive(before: &str, after: &str) -> bool {
    let mut remaining = toml_comments(after);
    toml_comments(before)
        .iter()
        .all(|c| match remaining.iter().position(|r| r == c) {
            Some(i) => {
                remaining.swap_remove(i);
                true
            }
            None => false,
        })
}

fn parse_toml_value(s: &str) -> toml_edit::Item {
    // Try bool, int, float, then fall back to string.
    if s == "true" {
        return toml_edit::value(true);
    }
    if s == "false" {
        return toml_edit::value(false);
    }
    // An array literal stays an array. Without this arm every `Kind::List`
    // value landed in TOML as one quoted string, and the reader then handed
    // the whole list back as a single item.
    if s.starts_with('[') && s.ends_with(']') {
        if let Ok(parsed) = s.parse::<toml_edit::Value>() {
            if parsed.is_array() {
                return toml_edit::Item::Value(parsed);
            }
        }
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
    let mut value_json = parse_json_source(original)?;
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
    crate::commands::toml_path::get(doc, key_path)
}

#[allow(dead_code)]
fn _path_buf_marker() -> PathBuf {
    PathBuf::new()
}
