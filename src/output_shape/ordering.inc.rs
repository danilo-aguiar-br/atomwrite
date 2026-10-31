// SPDX-License-Identifier: MIT OR Apache-2.0
// Positioned events, ordering and dedupe identity.
// Included from output_shape.inc.rs.

// ---------------------------------------------------------------------------
// Universal data operations (v0.1.36)
// ---------------------------------------------------------------------------

/// Positioned event used while buffering for sort, dedupe and diff.
#[derive(Debug, Clone)]
pub struct PositionedValue {
    /// NDJSON event emitted on the data channel.
    pub value: Value,
    /// Monotonic arrival index; the stable tie-break for every sort.
    pub line_index: u64,
    /// True for the record that closes the run (`summary`, `end`, or the
    /// failure envelope).
    ///
    /// The tail pass treats it as untouchable: `--filter`, `--dedupe`,
    /// `--aggregate`, `--diff-against` and `--limit` decide how to present an
    /// answer, so none of them may remove the record that says the answer is
    /// complete.
    pub terminal: bool,
    /// True for a framing record (`search` `begin` / `end`).
    ///
    /// Framing brackets a file; it is not an answer. The `--limit` budget
    /// counts answers, so a framing record is written whatever the budget says
    /// and never consumes it. Charging one made `search --sort-by path --limit
    /// 2` publish a single `match` while its summary promised two.
    pub framing: bool,
}

/// Sort two positioned events by `--sort`.
///
/// Every criterion falls back to `line_index`, so the ordering is total and
/// the same input always produces the same output.
pub fn compare_positioned(a: &PositionedValue, b: &PositionedValue, shape: &OutputShape) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match shape.sort {
        Some(SortKey::PathLine) => {
            compare_field(&a.value, &b.value, &["path"])
                .then_with(|| compare_field(&a.value, &b.value, LINE_KEYS))
                .then_with(|| compare_field(&a.value, &b.value, &["type"]))
                .then_with(|| a.line_index.cmp(&b.line_index))
        }
        Some(SortKey::LinePath) => {
            compare_field(&a.value, &b.value, LINE_KEYS)
                .then_with(|| compare_field(&a.value, &b.value, &["path"]))
                .then_with(|| compare_field(&a.value, &b.value, &["type"]))
                .then_with(|| a.line_index.cmp(&b.line_index))
        }
        Some(SortKey::Key | SortKey::KeyStable) => {
            let key = shape.sort_key.as_deref().unwrap_or_default();
            compare_field(&a.value, &b.value, &[key]).then_with(|| a.line_index.cmp(&b.line_index))
        }
        None => match a.line_index.cmp(&b.line_index) {
            Ordering::Equal => Ordering::Equal,
            other => other,
        },
    }
}

/// Keys carrying a line number, most specific first.
///
/// The search family publishes the line as `line_number`, an older shape used
/// `line`, and a stream record carries neither. Reading both spellings under
/// one criterion keeps the ordering a property of the data rather than of
/// whichever key a given subcommand happens to choose.
const LINE_KEYS: &[&str] = &["line_number", "line"];

/// Compare by the first key present on either side.
///
/// `keys` is tried in order and the first spelling that either event carries
/// decides. Without this, a criterion naming a key no envelope uses compares
/// two absent values, which sorts as "equal" on every pair and leaves the whole
/// order to the arrival index — that is, to the thread interleaving, which is
/// not reproducible.
fn compare_field(a: &Value, b: &Value, keys: &[&str]) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    for key in keys {
        let av = a.as_object().and_then(|o| o.get(*key));
        let bv = b.as_object().and_then(|o| o.get(*key));
        if av.is_none() && bv.is_none() {
            continue;
        }
        let ordering = compare_optional(av, bv);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

fn compare_optional(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => compare_value(x, y),
    }
}

fn compare_value(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(xf), Some(yf)) => xf.partial_cmp(&yf).unwrap_or(Ordering::Equal),
            _ => x.to_string().cmp(&y.to_string()),
        },
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::Bool(_), _) => Ordering::Less,
        (_, Value::Bool(_)) => Ordering::Greater,
        (Value::String(_), _) => Ordering::Less,
        (_, Value::String(_)) => Ordering::Greater,
        (Value::Array(x), Value::Array(y)) => x.len().cmp(&y.len()),
        (Value::Object(x), Value::Object(y)) => x.len().cmp(&y.len()),
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, _) => Ordering::Greater,
        (_, Value::Null) => Ordering::Less,
        (_, _) => Ordering::Equal,
    }
}

/// Build the dedupe identity key from the requested top-level keys.
fn dedupe_identity(v: &Value, keys: &[String]) -> String {
    let mut out = String::new();
    for key in keys {
        out.push('\u{1}');
        match v.as_object().and_then(|o| o.get(key)) {
            None => out.push_str("\u{4}missing"),
            Some(val) => out.push_str(&val.to_string()),
        }
    }
    out
}
