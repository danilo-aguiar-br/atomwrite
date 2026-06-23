// SPDX-License-Identifier: MIT OR Apache-2.0

//! Field extraction from NDJSON input or text columns.
//! Workload: I/O-bound (stdin streaming + field selection).

use std::io::{Read, Write};

use anyhow::Result;

use crate::cli::ExtractArgs;
use crate::output::NdjsonWriter;

/// Extract fields from NDJSON stdin or delimited text columns.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if reading stdin fails.
/// Returns `AtomwriteError::InvalidInput` if the field specification is invalid.
#[tracing::instrument(skip_all, fields(command = "extract"))]
pub fn cmd_extract(
    args: &ExtractArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let mut reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
    let mut line_buf = String::new();

    loop {
        let n = crate::output::read_limited_line(
            &mut reader,
            &mut line_buf,
            crate::constants::MAX_NDJSON_LINE_SIZE,
        )?;
        if n == 0 {
            break;
        }
        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('{') {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
                let typ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if matches!(typ, "begin" | "end" | "summary") {
                    continue;
                }
            }
            extract_json(trimmed, &args.fields, writer)?;
        } else {
            extract_text(trimmed, &args.fields, args.delimiter.as_deref(), writer)?;
        }
    }

    Ok(())
}

/// Check that a JSON value's nesting depth does not exceed `max`.
pub fn check_depth(value: &serde_json::Value, max: usize) -> bool {
    fn recurse(v: &serde_json::Value, remaining: usize) -> bool {
        if remaining == 0 {
            return false;
        }
        match v {
            serde_json::Value::Array(arr) => arr.iter().all(|item| recurse(item, remaining - 1)),
            serde_json::Value::Object(map) => map.values().all(|val| recurse(val, remaining - 1)),
            _ => true,
        }
    }
    recurse(value, max)
}

fn extract_json(
    line: &str,
    fields: &[String],
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let mut parsed: serde_json::Value = serde_json::from_str(line)?;

    if !check_depth(&parsed, crate::constants::MAX_JSON_DEPTH) {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "JSON nesting depth exceeds maximum of {}",
                crate::constants::MAX_JSON_DEPTH
            ),
        }
        .into());
    }

    if fields.is_empty() {
        writer.write_event(&parsed)?;
        return Ok(());
    }

    let mut output = serde_json::Map::new();
    if let Some(obj) = parsed.as_object_mut() {
        for field in fields {
            if let Some(val) = obj.remove(field.as_str()) {
                output.insert(field.clone(), val);
            }
        }
    }
    writer.write_event(&serde_json::Value::Object(output))?;
    Ok(())
}

fn extract_text(
    line: &str,
    fields: &[String],
    delimiter: Option<&str>,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let parts: Vec<&str> = match delimiter {
        Some(d) => line.split(d).collect(),
        None => line.split_whitespace().collect(),
    };

    if fields.is_empty() {
        writer.write_event(&crate::ndjson_types::TextFieldsOutput {
            r#type: "text",
            fields: parts,
        })?;
        return Ok(());
    }

    let mut results = Vec::with_capacity(fields.len());
    for field in fields {
        if let Ok(idx) = field.parse::<i64>() {
            let actual_idx = if idx < 0 {
                parts.len().checked_sub(idx.unsigned_abs() as usize)
            } else {
                Some(idx as usize)
            };
            if let Some(i) = actual_idx {
                if let Some(val) = parts.get(i) {
                    results.push(*val);
                }
            }
        }
    }

    writer.write_event(&crate::ndjson_types::TextValuesOutput {
        r#type: "text",
        values: results,
    })?;
    Ok(())
}
