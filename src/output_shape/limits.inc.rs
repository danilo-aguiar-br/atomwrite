// SPDX-License-Identifier: MIT OR Apache-2.0
// String limits, secret redaction and encoding normalization.
// Included from output_shape.inc.rs.

/// Records whose string content a limit cut in this process.
///
/// The per-record `truncated` mark says *that* one record was cut; this count
/// is the run-wide total a terminal summary publishes without the verb having
/// to know the flag exists.
static TRUNCATED_RECORDS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

/// Records whose string content a limit cut in this process.
#[must_use]
pub fn truncated_records() -> u64 {
    TRUNCATED_RECORDS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Count one record whose content a limit cut.
pub fn note_truncated_record() {
    TRUNCATED_RECORDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Apply `--truncate-content` and `--max-output-bytes` to string values.
///
/// Returns `true` when the event itself changed. The byte budget additionally
/// stops the writer from streaming further events.
pub fn apply_string_limits(v: &mut Value, shape: &OutputShape) -> bool {
    let mut changed = false;
    let Some(obj) = v.as_object_mut() else {
        if let Value::String(s) = v {
            changed |= truncate_string(s, shape.max_output_bytes);
        }
        return changed;
    };
    for (key, len) in &shape.truncate_specs {
        if let Some(Value::String(s)) = obj.get_mut(key) {
            changed |= prefix_chars(s, *len);
        }
    }
    if let Some(max) = shape.max_output_bytes {
        for val in obj.values_mut() {
            if let Value::String(s) = val {
                changed |= truncate_string(s, Some(max));
            }
        }
    }
    changed
}

fn prefix_chars(s: &mut String, len: usize) -> bool {
    if s.chars().count() <= len {
        return false;
    }
    let end = s.char_indices().nth(len).map(|(i, _)| i).unwrap_or(s.len());
    s.truncate(end);
    true
}

fn truncate_string(s: &mut String, max: Option<usize>) -> bool {
    let Some(max) = max else {
        return false;
    };
    if s.len() <= max {
        return false;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
    true
}

/// Redact secret-shaped string values in place (`--redact`).
///
/// Two independent rules: an object key whose suffix marks a secret always
/// has its string value replaced, and any string containing a known
/// credential prefix is replaced whole. Returns the number of values redacted.
pub fn redact_secrets(v: &mut Value) -> usize {
    fn looks_secret(s: &str) -> bool {
        SECRET_NEEDLES
            .iter()
            .any(|n| memmem::find(s.as_bytes(), n.as_bytes()).is_some())
    }
    fn key_marks_secret(key: &str) -> bool {
        let lower = key.to_ascii_lowercase();
        SECRET_KEY_MARKERS
            .iter()
            .any(|m| lower.ends_with(m) || lower.contains(m))
    }
    match v {
        Value::Object(map) => {
            let mut count = 0;
            for (key, val) in map.iter_mut() {
                match val {
                    Value::String(s) => {
                        if key_marks_secret(key) || looks_secret(s) {
                            *s = REDACTED.to_string();
                            count += 1;
                        }
                    }
                    other => count += redact_secrets(other),
                }
            }
            count
        }
        Value::Array(items) => items.iter_mut().map(redact_secrets).sum(),
        Value::String(s) => {
            if looks_secret(s) {
                *s = REDACTED.to_string();
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Normalize every string value to Unicode NFC in place (`--normalize-encoding`).
///
/// Returns `true` when any string changed. The pass cannot fail: it only
/// rewrites strings the value already holds.
pub fn normalize_encoding(v: &mut Value) -> bool {
    use unicode_normalization::UnicodeNormalization;

    let mut changed = false;
    match v {
        Value::Object(map) => {
            for val in map.values_mut() {
                changed |= normalize_encoding(val);
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                changed |= normalize_encoding(item);
            }
        }
        Value::String(s) => {
            if !s.is_ascii() {
                let normalized: String = s.nfc().collect();
                if normalized != *s {
                    changed = true;
                    *s = normalized;
                }
            }
        }
        // Numbers are lossless JSON already; there is no second text form.
        _ => {}
    }
    changed
}
