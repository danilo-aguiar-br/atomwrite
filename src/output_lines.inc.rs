// SPDX-License-Identifier: MIT OR Apache-2.0
// Bounded NDJSON line reader and small JSON helpers.
// Included from output.rs.

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
///
/// # Errors
///
/// Returns the `std::io::Error` from `fill_buf` when the reader fails. Returns
/// `ErrorKind::InvalidData` when the line grows past `max_bytes`, and when the
/// decoded line is not valid UTF-8.
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
    if slice.starts_with(&crate::constants::UTF8_BOM) {
        slice = &slice[crate::constants::UTF8_BOM.len()..];
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
/// Cross-platform: Windows editors may emit a UTF-8 BOM; strip it before JSON parse.
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
    let hard_cap = max_bytes
        .saturating_mul(crate::constants::OUTPUT_DRAIN_CAP_FACTOR)
        .max(max_bytes + 1);
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
