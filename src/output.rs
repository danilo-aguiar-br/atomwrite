// SPDX-License-Identifier: MIT OR Apache-2.0

//! NDJSON writer utilities for stdout with broken-pipe handling.

use std::io::{self, BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::error::{AtomwriteError, ErrorContext, ErrorJson};

/// Buffered NDJSON writer that flushes after every line.
pub struct NdjsonWriter<W: Write> {
    writer: BufWriter<W>,
}

impl<W: Write> NdjsonWriter<W> {
    /// Create a new NDJSON writer wrapping the given output.
    pub fn new(inner: W) -> Self {
        Self {
            writer: BufWriter::with_capacity(crate::constants::BUF_CAPACITY, inner),
        }
    }

    /// Serialize a value as a single NDJSON line and flush.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if serialization or writing to the underlying writer fails.
    pub fn write_event<T: Serialize>(&mut self, event: &T) -> anyhow::Result<()> {
        match serde_json::to_writer(&mut self.writer, event) {
            Ok(()) => {}
            Err(e) if is_broken_pipe(&e) => {
                return Err(crate::error::AtomwriteError::BrokenPipe.into());
            }
            Err(e) => return Err(e.into()),
        }
        match self.writer.write_all(b"\n") {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                return Err(crate::error::AtomwriteError::BrokenPipe.into());
            }
            Err(e) => return Err(e.into()),
        }
        match self.writer.flush() {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                return Err(crate::error::AtomwriteError::BrokenPipe.into());
            }
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    /// Emit a structured error as a single NDJSON line.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing the error JSON to the underlying writer fails.
    pub fn write_error(&mut self, err: &AtomwriteError, path: Option<&Path>) -> anyhow::Result<()> {
        self.write_error_with_context(err, path, &ErrorContext::default())
    }

    /// Emit a structured error as a single NDJSON line, with diagnostic context.
    ///
    /// Use this overload when the caller knows whether the workspace root was
    /// explicitly provided (e.g. via `--workspace` or `ATOMWRITE_WORKSPACE`).
    /// The context controls the suggestion text for `WorkspaceJail` errors
    /// (GAP 13 fix).
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing the error JSON to the underlying writer fails.
    pub fn write_error_with_context(
        &mut self,
        err: &AtomwriteError,
        path: Option<&Path>,
        ctx: &ErrorContext,
    ) -> anyhow::Result<()> {
        let mut json = ErrorJson::from_error_with_context(err, ctx);
        if json.path.is_none() {
            json.path = path.map(|p| p.display().to_string());
        }
        self.write_event(&json)
    }

    /// Flush the underlying buffer to the output stream.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if flushing the underlying writer fails.
    pub fn flush(&mut self) -> anyhow::Result<()> {
        self.writer.flush().map_err(|e| e.into())
    }
}

/// Write a structured error as NDJSON directly to a raw writer.
///
/// # Errors
///
/// Returns an I/O error if writing the error JSON to the underlying writer fails.
#[cold]
pub fn write_error_json(
    out: &mut impl Write,
    err: &AtomwriteError,
    path: Option<&Path>,
) -> anyhow::Result<()> {
    write_error_json_with_context(out, err, path, &ErrorContext::default())
}

/// Write a structured error as NDJSON directly to a raw writer, with context.
///
/// # Errors
///
/// Returns an I/O error if writing the error JSON to the underlying writer fails.
#[cold]
pub fn write_error_json_with_context(
    out: &mut impl Write,
    err: &AtomwriteError,
    path: Option<&Path>,
    ctx: &ErrorContext,
) -> anyhow::Result<()> {
    let mut json = ErrorJson::from_error_with_context(err, ctx);
    if json.path.is_none() {
        json.path = path.map(|p| p.display().to_string());
    }
    serde_json::to_writer(&mut *out, &json)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// Read a single line from a buffered reader with a **streaming** per-line size limit.
///
/// Reuses the provided `buf` (cleared before each call). Returns the number
/// of bytes read (0 means EOF). Returns an error if the line exceeds
/// `max_bytes` **before** the full line is buffered (DoS-safe — unlike
/// `BufRead::read_line`, which can allocate unbounded data first).
///
/// CRLF is normalized: a trailing `\r` before `\n` is discarded so Windows
/// NDJSON pipelines parse as LF-delimited records.
///
/// A leading UTF-8 BOM (`U+FEFF`) is stripped when present at the start of
/// the line (common on Windows-exported manifests).
pub fn read_limited_line(
    reader: &mut impl std::io::BufRead,
    buf: &mut String,
    max_bytes: usize,
) -> std::io::Result<usize> {
    buf.clear();
    // Accumulate raw bytes first so multi-byte UTF-8 sequences that straddle
    // `BufRead` buffer boundaries are decoded only once, at end-of-line.
    let mut raw: Vec<u8> = Vec::new();
    let mut total = 0usize;
    loop {
        let available = {
            let filled = reader.fill_buf()?;
            if filled.is_empty() {
                if total == 0 {
                    return Ok(0);
                }
                return finalize_line_bytes(&raw, buf, total);
            }
            filled
        };

        if let Some(nl) = memchr::memchr(b'\n', available) {
            let chunk_len = nl + 1; // include '\n'
            if total.saturating_add(chunk_len) > max_bytes {
                reader.consume(chunk_len);
                return Err(std::io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "NDJSON line exceeds maximum size of {max_bytes} bytes ({} bytes read)",
                        total.saturating_add(chunk_len)
                    ),
                ));
            }
            raw.extend_from_slice(&available[..nl]);
            reader.consume(chunk_len);
            total += chunk_len;
            return finalize_line_bytes(&raw, buf, total);
        }

        let n = available.len();
        if total.saturating_add(n) > max_bytes {
            reader.consume(n);
            total = total.saturating_add(n);
            drain_until_newline(reader, &mut total, max_bytes)?;
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "NDJSON line exceeds maximum size of {max_bytes} bytes ({total} bytes read)"
                ),
            ));
        }
        raw.extend_from_slice(available);
        reader.consume(n);
        total += n;
    }
}

fn finalize_line_bytes(raw: &[u8], buf: &mut String, total: usize) -> std::io::Result<usize> {
    let mut slice = raw;
    // Drop trailing CR from CRLF (already excluded LF).
    if slice.last() == Some(&b'\r') {
        slice = &slice[..slice.len() - 1];
    }
    // Strip UTF-8 BOM bytes if present at the start of the record.
    if slice.starts_with(&[0xEF, 0xBB, 0xBF]) {
        slice = &slice[3..];
    }
    match std::str::from_utf8(slice) {
        Ok(s) => {
            buf.push_str(s);
            Ok(total)
        }
        Err(e) => Err(std::io::Error::new(
            io::ErrorKind::InvalidData,
            format!("NDJSON line is not valid UTF-8: {e}"),
        )),
    }
}

/// Strip a leading UTF-8 BOM character from an NDJSON line (after decode).
///
/// Rules: multiplataforma — BOM no início de arquivos Windows deve ser
/// removido antes do parsing JSON.
#[inline]
pub fn strip_utf8_bom_str(s: &str) -> &str {
    s.strip_prefix('\u{FEFF}').unwrap_or(s)
}

/// Convert a dotted config path (`a.b.c`) into an RFC 6901 JSON Pointer
/// (`/a/b/c`), escaping `~` → `~0` and `/` → `~1` in each segment.
#[inline]
pub fn dotted_to_json_pointer(path: &str) -> String {
    let mut out = String::with_capacity(path.len() + 1);
    for seg in path.split('.') {
        out.push('/');
        // RFC 6901: escape `~` first, then `/`.
        for ch in seg.chars() {
            match ch {
                '~' => out.push_str("~0"),
                '/' => out.push_str("~1"),
                c => out.push(c),
            }
        }
    }
    out
}

/// Check that a JSON value's nesting depth does not exceed `max`.
///
/// Leaf values have depth 1. An object/array that contains only leaves has
/// depth 2. Used to reject pathological nesting on untrusted JSON input
/// (manifests, config files, extract stdin).
pub fn check_json_depth(value: &serde_json::Value, max: usize) -> bool {
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

/// Consume bytes until `\n` or EOF after a size-limit violation so subsequent
/// reads do not resume mid-line.
fn drain_until_newline(
    reader: &mut impl std::io::BufRead,
    total: &mut usize,
    max_bytes: usize,
) -> std::io::Result<()> {
    // Cap drain work: once we are well past the limit we still seek the
    // newline, but stop if the overrun itself is pathological (16× max).
    let hard_cap = max_bytes.saturating_mul(16).max(max_bytes + 1);
    loop {
        let available = {
            let filled = reader.fill_buf()?;
            if filled.is_empty() {
                return Ok(());
            }
            filled
        };
        if let Some(nl) = memchr::memchr(b'\n', available) {
            reader.consume(nl + 1);
            *total = total.saturating_add(nl + 1);
            return Ok(());
        }
        let n = available.len();
        reader.consume(n);
        *total = total.saturating_add(n);
        if *total >= hard_cap {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "NDJSON line exceeds maximum size of {max_bytes} bytes and has no terminator within {hard_cap} bytes"
                ),
            ));
        }
    }
}

fn is_broken_pipe(err: &serde_json::Error) -> bool {
    matches!(err.io_error_kind(), Some(io::ErrorKind::BrokenPipe))
}

/// Parse one NDJSON object line and insert a field without `serde_json::json!`.
///
/// Used by recipe/codemod when re-emitting child events with an extra tag
/// (`step`, `rule_id`). Returns `None` if the line is not a JSON object.
pub fn ndjson_insert_field(
    line: &str,
    key: impl Into<String>,
    value: serde_json::Value,
) -> Option<serde_json::Value> {
    let mut v: serde_json::Value = serde_json::from_str(line).ok()?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(key.into(), value);
        Some(v)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_limited_line_basic_lf() {
        let mut r = Cursor::new(b"{\"a\":1}\n{\"b\":2}\n");
        let mut buf = String::new();
        let n = read_limited_line(&mut r, &mut buf, 1024).unwrap();
        assert!(n > 0);
        assert_eq!(buf, "{\"a\":1}");
        let n = read_limited_line(&mut r, &mut buf, 1024).unwrap();
        assert!(n > 0);
        assert_eq!(buf, "{\"b\":2}");
        assert_eq!(read_limited_line(&mut r, &mut buf, 1024).unwrap(), 0);
    }

    #[test]
    fn read_limited_line_crlf_and_bom() {
        let mut r = Cursor::new(b"\xEF\xBB\xBF{\"x\":1}\r\n");
        let mut buf = String::new();
        read_limited_line(&mut r, &mut buf, 1024).unwrap();
        assert_eq!(buf, "{\"x\":1}");
    }

    #[test]
    fn read_limited_line_rejects_oversize_before_full_buffer() {
        // Line larger than max without needing a multi-GB allocation first.
        let mut payload = vec![b'a'; 64];
        payload.push(b'\n');
        let mut r = Cursor::new(payload);
        let mut buf = String::new();
        let err = read_limited_line(&mut r, &mut buf, 32).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        // Stream recovered: next read is EOF (newline was consumed).
        assert_eq!(read_limited_line(&mut r, &mut buf, 32).unwrap(), 0);
    }

    #[test]
    fn check_json_depth_limits_nesting() {
        let shallow: serde_json::Value =
            serde_json::from_str(r#"{"a":{"b":1}}"#).expect("fixture");
        assert!(check_json_depth(&shallow, 3));
        assert!(!check_json_depth(&shallow, 2));
        let leaf = serde_json::Value::from(1);
        assert!(check_json_depth(&leaf, 1));
    }

    #[test]
    fn ndjson_insert_field_tags_object() {
        let tagged = ndjson_insert_field(r#"{"type":"x","path":"/t"}"#, "step", 2.into())
            .expect("object line");
        assert_eq!(tagged["step"], 2);
        assert_eq!(tagged["type"], "x");
        assert!(ndjson_insert_field("[]", "step", 1.into()).is_none());
    }

    #[test]
    fn strip_utf8_bom_str_prefix() {
        assert_eq!(strip_utf8_bom_str("\u{FEFF}{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_utf8_bom_str("{\"a\":1}"), "{\"a\":1}");
    }

    #[test]
    fn dotted_to_json_pointer_escapes_rfc6901() {
        assert_eq!(dotted_to_json_pointer("a.b.c"), "/a/b/c");
        assert_eq!(dotted_to_json_pointer("a/b"), "/a~1b");
        assert_eq!(dotted_to_json_pointer("a~b"), "/a~0b");
        assert_eq!(dotted_to_json_pointer("x.y/z~w"), "/x/y~1z~0w");
    }
}
