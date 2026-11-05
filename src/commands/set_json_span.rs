// SPDX-License-Identifier: MIT OR Apache-2.0

//! Byte-span locator for one value inside a JSON document.
//!
//! `set` on a JSON file used to re-serialize the whole document, which
//! re-indented every untouched key. The caller that only wants one key changed
//! then receives a file that differs everywhere, and the diff it must review
//! costs more than the edit itself.
//!
//! This scanner walks the raw bytes, counts bracket depth and skips strings
//! with their escapes, so it can return the exact `[start, end)` of the value
//! at a dotted path. Splicing that span leaves every other byte untouched.
//!
//! The scanner never decides whether the result is correct: the caller
//! re-parses the spliced text and compares it with the value the canonical
//! rewrite produced. Any disagreement falls back to the canonical rewrite, so a
//! scanner mistake costs formatting and never data.

/// Skip ASCII whitespace starting at `i`, returning the first significant byte.
fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

/// Byte offset just past the JSON string that starts at `start`.
///
/// `start` must point at the opening quote. Returns `None` on an unterminated
/// string, which the caller treats as "no span" and not as a parse verdict.
fn skip_string(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'"') {
        return None;
    }
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            // A backslash escapes the next byte, so the quote after it is data.
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// Byte offset just past the JSON value that starts at `start`.
///
/// Strings, arrays and objects are consumed by their own rules so a `,` or `}`
/// inside a string never ends the value early.
fn skip_value(bytes: &[u8], start: usize) -> Option<usize> {
    match *bytes.get(start)? {
        b'"' => skip_string(bytes, start),
        b'{' | b'[' => {
            let mut i = start;
            let mut depth = 0usize;
            while i < bytes.len() {
                match bytes[i] {
                    b'"' => {
                        i = skip_string(bytes, i)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth = depth.checked_sub(1)?;
                        if depth == 0 {
                            return Some(i + 1);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            None
        }
        // A number, `true`, `false` or `null` ends at the first delimiter.
        _ => {
            let mut i = start;
            while i < bytes.len()
                && !matches!(bytes[i], b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
            {
                i += 1;
            }
            (i > start).then_some(i)
        }
    }
}

/// Byte span of the value reached by `segments` from the document root.
///
/// Each segment is one unescaped pointer token: an object key or an array
/// index. Returns `None` when the path does not exist in `text`, which is the
/// signal for the caller to take the canonical rewrite instead.
pub(crate) fn locate_value_span(text: &str, segments: &[String]) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = skip_ws(bytes, 0);
    for segment in segments {
        i = skip_ws(bytes, i);
        match *bytes.get(i)? {
            b'{' => i = find_object_member(text, bytes, i, segment)?,
            b'[' => i = find_array_element(bytes, i, segment)?,
            // A scalar has no children, so the path cannot go deeper.
            _ => return None,
        }
    }
    let start = i;
    Some((start, skip_value(bytes, start)?))
}

/// Byte offset where the value of `key` starts inside the object at `open`.
fn find_object_member(text: &str, bytes: &[u8], open: usize, key: &str) -> Option<usize> {
    let mut i = skip_ws(bytes, open + 1);
    loop {
        match *bytes.get(i)? {
            b'}' => return None,
            b'"' => {
                let key_end = skip_string(bytes, i)?;
                // The raw slice is a quoted JSON string, so serde decodes the
                // escapes exactly as the parser that accepted the document did.
                let decoded: String = serde_json::from_str(&text[i..key_end]).ok()?;
                let mut j = skip_ws(bytes, key_end);
                if *bytes.get(j)? != b':' {
                    return None;
                }
                j = skip_ws(bytes, j + 1);
                let value_end = skip_value(bytes, j)?;
                if decoded == key {
                    return Some(j);
                }
                j = skip_ws(bytes, value_end);
                match *bytes.get(j)? {
                    b',' => i = skip_ws(bytes, j + 1),
                    _ => return None,
                }
            }
            _ => return None,
        }
    }
}

/// Byte offset where element number `segment` starts inside the array at `open`.
fn find_array_element(bytes: &[u8], open: usize, segment: &str) -> Option<usize> {
    let wanted: usize = segment.parse().ok()?;
    let mut i = skip_ws(bytes, open + 1);
    let mut index = 0usize;
    loop {
        match *bytes.get(i)? {
            b']' => return None,
            _ => {
                let value_end = skip_value(bytes, i)?;
                if index == wanted {
                    return Some(i);
                }
                index += 1;
                let j = skip_ws(bytes, value_end);
                match *bytes.get(j)? {
                    b',' => i = skip_ws(bytes, j + 1),
                    _ => return None,
                }
            }
        }
    }
}
