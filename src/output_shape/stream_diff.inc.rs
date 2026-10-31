// SPDX-License-Identifier: MIT OR Apache-2.0
// Stream diff against a baseline.
// Included from output_shape.inc.rs.

/// Difference report between the live stream and a baseline NDJSON file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamDiff {
    /// Events present only in the live stream (by NDJSON text).
    pub added: usize,
    /// Events present only in the baseline (by NDJSON text).
    pub removed: usize,
    /// Events present in both streams (by NDJSON text).
    pub unchanged: usize,
}

/// Wire envelope for the single `stream_diff` event emitted by `--diff-against`.
///
/// The counts are absolute, not deltas of a previous run, so an agent can read
/// the line without holding earlier state.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct StreamDiffOutput {
    /// Event discriminator on the data channel.
    #[serde(rename = "type")]
    pub kind: String,
    /// Events present only in the live stream.
    pub added: u64,
    /// Events present only in the baseline.
    pub removed: u64,
    /// Events present in both streams.
    pub unchanged: u64,
}

impl StreamDiffOutput {
    /// Build the wire envelope from a computed [`StreamDiff`].
    pub fn from_diff(diff: &StreamDiff) -> Self {
        Self {
            kind: "stream_diff".to_string(),
            added: diff.added as u64,
            removed: diff.removed as u64,
            unchanged: diff.unchanged as u64,
        }
    }
}

/// Build a multiset identity for one event.
fn stream_identity(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| String::from("\u{0}unserializable"))
}

/// Compare two event multisets, preserving arrival order for `added`/`removed`.
pub fn diff_streams(baseline: &[PositionedValue], live: &[PositionedValue]) -> StreamDiff {
    use std::collections::HashMap;

    let mut counts: HashMap<String, i64> = HashMap::new();
    for item in baseline {
        *counts.entry(stream_identity(&item.value)).or_insert(0) += 1;
    }
    let mut result = StreamDiff::default();
    for item in live {
        let id = stream_identity(&item.value);
        match counts.get_mut(&id) {
            Some(n) if *n > 0 => {
                *n -= 1;
                result.unchanged += 1;
            }
            _ => result.added += 1,
        }
    }
    result.removed = counts.values().filter(|n| **n > 0).map(|n| *n as usize).sum();
    result
}

/// Load a baseline NDJSON file for `--diff-against`.
///
/// # Errors
///
/// Returns an error when the file cannot be read or a line is not JSON.
pub fn load_baseline(
    path: &std::path::Path,
) -> Result<Vec<PositionedValue>, crate::error::AtomwriteError> {
    let raw = std::fs::read_to_string(path).map_err(|e| crate::error::AtomwriteError::Io {
        source: std::io::Error::new(e.kind(), format!("--diff-against {}: {e}", path.display())),
    })?;
    let mut out = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).map_err(|e| {
            crate::error::AtomwriteError::InvalidInput {
                reason: format!("--diff-against line {} is not JSON: {e}", index + 1),
            }
        })?;
        out.push(PositionedValue {
            value,
            line_index: index as u64,
            terminal: false,
            framing: false,
        });
    }
    Ok(out)
}
