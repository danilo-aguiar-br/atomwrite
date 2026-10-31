// SPDX-License-Identifier: MIT OR Apache-2.0
// Applying a shape to one event.
// Included from output_shape.inc.rs.

/// Apply filters then optional key projection. Returns `None` when filtered out.
///
/// Used for **success** events only. Error envelopes must use
/// [`shape_error_value`] so aggressive `--filter` never silences failures
/// (agent-native always-emit errors, v0.1.36 residual).
pub fn shape_value(v: Value, shape: &OutputShape) -> Option<Value> {
    if !shape.filters.is_empty() {
        let obj = v.as_object()?;
        for pred in &shape.filters {
            let actual = obj.get(&pred.key).cloned().unwrap_or(Value::Null);
            let eq = json_values_equal(&actual, &pred.value);
            if pred.not_equal {
                if eq {
                    return None;
                }
            } else if !eq {
                return None;
            }
        }
    }
    let mut v = project_select(v, shape);
    // An event that `--select` left without a single key says nothing, so it is
    // not written (a bare `{}` per `begin` / `end` line costs tokens and carries
    // no data). Failure envelopes never pass through here: they always emit.
    if shape.select.is_some() && v.as_object().is_some_and(serde_json::Map::is_empty) {
        return None;
    }
    apply_value_ops(&mut v, shape);
    Some(v)
}

/// Shape an **error** envelope for agent-native stdout.
///
/// - **Never** applies `--filter` (errors always emit).
/// - **Never** returns empty / drop semantics for filter mismatch.
/// - **May** apply `--select` / `--fields` projection so agents can thin keys.
/// - **Always** applies `--redact`: an error envelope frequently carries the
///   offending command line back, so the secret rule must reach this path too
///   or the guarantee would hold for successes only.
/// - **Never** applies the byte window or NFC: the envelope needs no size
///   window, and a second transform here would only add a way to lose the
///   diagnostic the agent needs.
/// - Callers must **not** count errors against `--limit` (success-only budget).
pub fn shape_error_value(v: Value, shape: &OutputShape) -> Value {
    let original = v.clone();
    let mut v = project_select(v, shape);
    // A verdict is read off `type` and `ok`. `--select path` on a failure used
    // to publish `{"path":"/tmp/..."}` and nothing else, so the record that
    // says the run failed stopped naming itself or its verdict. Thinning is
    // allowed; losing the identity of the record is not.
    v = keep_terminal_keys(v, &original);
    // The key floor is not gated: an error envelope frequently carries the
    // offending command line, so redacting only on success would leak exactly
    // where the leak matters most. `--redact` adds the content rules below.
    scrub_envelope_keys(&mut v);
    if shape.redact {
        redact_secrets(&mut v);
    }
    v
}

/// Restore the two keys a terminal record is read by, after `--select` thinned it.
///
/// `--select` names the fields the caller wants, and it may legitimately omit
/// both. The record is still the verdict, so the discriminator and the `ok` flag
/// come back from the untouched original. A record that never carried them (a
/// generic success line) gets nothing added.
pub fn keep_terminal_keys(projected: Value, original: &Value) -> Value {
    let (Some(obj), Some(src)) = (projected.as_object(), original.as_object()) else {
        return projected;
    };
    let mut out = obj.clone();
    for key in ["type", crate::output::RESULT_KEY] {
        if !out.contains_key(key)
            && let Some(val) = src.get(key)
        {
            out.insert(key.to_string(), val.clone());
        }
    }
    Value::Object(out)
}

/// Apply every value-level operation to one event, in place.
///
/// Order matters and is fixed. The key floor runs first and unconditionally, so
/// a secret is gone before anything else can truncate or re-encode it into a
/// still-recognizable form. Then the `--redact` content rules run, then the
/// byte window, and NFC last so the emitted bytes are the normalized ones the
/// caller asked for.
fn apply_value_ops(v: &mut Value, shape: &OutputShape) {
    scrub_envelope_keys(v);
    if shape.redact {
        redact_secrets(v);
    }
    if shape.max_output_bytes.is_some() || !shape.truncate_specs.is_empty() {
        if apply_string_limits(v, shape) {
            // A cut the record does not name is data loss the consumer cannot
            // see: `bytes` and `lines` keep the file's totals, so the reader
            // would take the shortened text for the whole one. The verb that
            // computes those totals has no reason to know this flag exists, so
            // the mark is stamped where the cut happens.
            if let Some(obj) = v.as_object_mut() {
                obj.insert("truncated".to_string(), Value::Bool(true));
            }
            note_truncated_record();
        }
    }
    if shape.normalize_encoding {
        normalize_encoding(v);
    }
}

fn project_select(mut v: Value, shape: &OutputShape) -> Value {
    if let Some(keys) = &shape.select {
        let Some(obj) = v.as_object_mut() else {
            return v;
        };
        let mut out = serde_json::Map::new();
        for k in keys {
            if let Some(val) = obj.remove(k) {
                out.insert(k.clone(), val);
            }
        }
        return Value::Object(out);
    }
    v
}

fn json_values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if let (Some(xf), Some(yf)) = (x.as_f64(), y.as_f64()) {
                (xf - yf).abs() < f64::EPSILON
            } else {
                x == y
            }
        }
        _ => a == b,
    }
}
