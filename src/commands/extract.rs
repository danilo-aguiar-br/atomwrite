// SPDX-License-Identifier: MIT OR Apache-2.0

//! Field extraction from NDJSON input or text columns.

use std::io::{BufRead, Read, Write};

use anyhow::Result;

use crate::cli::ExtractArgs;
use crate::output::NdjsonWriter;

/// Extract fields from NDJSON stdin or delimited text columns.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if reading stdin fails.
/// Returns `AtomwriteError::InvalidInput` if the field specification is invalid.
pub fn cmd_extract(
    args: &ExtractArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let reader = std::io::BufReader::new(stdin);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('{') {
            extract_json(trimmed, &args.fields, writer)?;
        } else {
            extract_text(trimmed, &args.fields, args.delimiter.as_deref(), writer)?;
        }
    }

    Ok(())
}

fn extract_json(
    line: &str,
    fields: &[String],
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let parsed: serde_json::Value = serde_json::from_str(line)?;

    if fields.is_empty() {
        writer.write_event(&parsed)?;
        return Ok(());
    }

    let mut output = serde_json::Map::new();
    for field in fields {
        if let Some(val) = parsed.get(field) {
            output.insert(field.clone(), val.clone());
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
        let output = serde_json::json!({"type": "text", "fields": parts});
        writer.write_event(&output)?;
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

    let output = serde_json::json!({"type": "text", "values": results});
    writer.write_event(&output)?;
    Ok(())
}
