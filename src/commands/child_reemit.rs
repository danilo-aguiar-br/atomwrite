// SPDX-License-Identifier: MIT OR Apache-2.0

//! Forward a captured child run into the parent NDJSON stream.
//!
//! `recipe` and `codemod` run a child command into a buffer and forward the
//! lines. The child charges `--limit` as it writes, so the forward path must
//! not charge again, must keep `begin` and `end` out of the dedupe identity,
//! and must not let the child's terminal pass as the parent's terminal.

use std::io::Write;

use serde_json::Value;

use crate::output::NdjsonWriter;

/// What the child stream carried besides the forwarded records.
pub(crate) struct ChildReemit {
    /// The child's terminal record, removed from the forwarded stream.
    pub(crate) terminal: Option<Value>,
    /// Whether that terminal says the child failed.
    pub(crate) failed: bool,
}

/// Forward a captured child buffer, letting `prepare` tag each record.
///
/// A `begin` or `end` record keeps its framing role, so `--dedupe` never lets
/// the anchor claim a key the answers need. The child's `summary` is not
/// forwarded: it comes back in [`ChildReemit`] so the parent attaches it to its
/// own step record, and one run keeps one terminal.
///
/// # Errors
///
/// Returns an I/O error when the parent writer cannot write a record.
pub(crate) fn reemit_child_buffer<W: Write>(
    writer: &mut NdjsonWriter<W>,
    buf: &[u8],
    mut prepare: impl FnMut(&mut Value),
) -> anyhow::Result<ChildReemit> {
    let mut out = ChildReemit {
        terminal: None,
        failed: false,
    };
    for line in String::from_utf8_lossy(buf).lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(mut value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if kind == "summary" {
            out.failed |= value.get("ok") == Some(&Value::Bool(false));
            out.terminal = Some(value);
            continue;
        }
        prepare(&mut value);
        if kind == "begin" || kind == "end" {
            writer.write_framing_event(&value)?;
        } else {
            writer.write_reemitted_event(&value)?;
        }
    }
    Ok(out)
}
