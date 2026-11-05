// SPDX-License-Identifier: MIT OR Apache-2.0
// Join tables and aggregation.
// Included from output_shape.inc.rs.

/// External NDJSON table loaded by `--join`.
#[derive(Debug, Clone, Default)]
pub struct JoinTable {
    /// Grouping key inside the external records.
    pub key: String,
    /// Top-level suffix written on the event envelope.
    pub suffix: String,
    /// Groups indexed by key; each group keeps its records in arrival order.
    pub groups: std::collections::HashMap<String, Vec<Value>>,
}

/// Parse positioned NDJSON stdin into join tables (one table per `--join`).
///
/// # Errors
///
/// Returns an error when a line is not a JSON object or exceeds
/// `constants::env_limits` style bounds enforced by the caller.
pub fn parse_join_stdin(
    raw: &str,
    specs: &[JoinSpec],
) -> Result<Vec<JoinTable>, crate::error::AtomwriteError> {
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let mut records: Vec<Value> = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value =
            serde_json::from_str(line).map_err(|e| crate::error::AtomwriteError::InvalidInput {
                reason: format!(
                    "{} stdin line is not JSON: {e}",
                    crate::constants::JOIN_REASON_MARKER
                ),
            })?;
        if !value.is_object() {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: format!(
                    "{} stdin must contain NDJSON objects",
                    crate::constants::JOIN_REASON_MARKER
                ),
            });
        }
        records.push(value);
    }
    let mut tables = Vec::with_capacity(specs.len());
    for spec in specs {
        let mut groups: std::collections::HashMap<String, Vec<Value>> =
            std::collections::HashMap::new();
        for rec in &records {
            let Some(group) = rec.get(&spec.key) else {
                continue;
            };
            let Some(group) = group.as_str() else {
                continue;
            };
            groups
                .entry(group.to_string())
                .or_default()
                .push(rec.clone());
        }
        if groups.is_empty() {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: format!(
                    "{} table has no record with a string {:?} key",
                    crate::constants::JOIN_REASON_MARKER,
                    spec.key
                ),
            });
        }
        tables.push(JoinTable {
            key: spec.key.clone(),
            suffix: spec.suffix.clone(),
            groups,
        });
    }
    Ok(tables)
}

/// Apply one join table to an event envelope. Returns `true` on a hit.
///
/// A record that already says the run failed (`ok: false`) or closes it
/// (`type: summary`) is left alone: the join annotates data events, never the
/// verdict. An event without the key, or whose key has no row, keeps its shape.
pub fn apply_join(v: &mut Value, table: &JoinTable) -> bool {
    let is_verdict = v.get("ok") == Some(&Value::Bool(false))
        || v.get("type").and_then(Value::as_str) == Some("summary");
    if is_verdict {
        return false;
    }
    let Some(group) = v
        .as_object()
        .and_then(|o| o.get(&table.key))
        .and_then(Value::as_str)
    else {
        return false;
    };
    let Some(matched) = table.groups.get(group) else {
        return false;
    };
    if let Some(obj) = v.as_object_mut() {
        obj.insert(table.suffix.clone(), Value::Array(matched.clone()));
        return true;
    }
    false
}

/// Reduce a group of events into one summary event (`--aggregate`).
pub fn aggregate_group(
    group_key: &str,
    group_value: Value,
    events: &[PositionedValue],
    specs: &[AggSpec],
) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert(group_key.to_string(), group_value);
    obj.insert("count".to_string(), Value::from(events.len() as u64));
    keep_verdict(&mut obj, events);
    for spec in specs {
        if spec.group != group_key {
            continue;
        }
        let (name, value) = match &spec.op {
            AggOp::Count => ("count".to_string(), Value::from(events.len() as u64)),
            AggOp::Sum(field) => (
                format!("sum_{field}"),
                reduce_numeric(events, field, NumericReduction::Sum),
            ),
            AggOp::Min(field) => (
                format!("min_{field}"),
                reduce_numeric(events, field, NumericReduction::Min),
            ),
            AggOp::Max(field) => (
                format!("max_{field}"),
                reduce_numeric(events, field, NumericReduction::Max),
            ),
        };
        obj.insert(name, value);
    }
    Value::Object(obj)
}

/// Carry the run verdict (`ok`, `code`, `exit`) of a group into its aggregate.
///
/// Reducing the terminal event to a count must not hide that the run failed:
/// the first member that says `ok: false` lends its verdict, and a group whose
/// members all say `ok: true` stays `ok: true`. Groups with no verdict (stream
/// records) gain none.
fn keep_verdict(obj: &mut serde_json::Map<String, Value>, events: &[PositionedValue]) {
    let verdict_of = |event: &PositionedValue| {
        event
            .value
            .as_object()
            .and_then(|o| o.get(crate::output::RESULT_KEY))
            .and_then(Value::as_bool)
    };
    let source = events
        .iter()
        .find(|e| verdict_of(e) == Some(false))
        .or_else(|| events.iter().find(|e| verdict_of(e) == Some(true)));
    let Some(source) = source.and_then(|e| e.value.as_object()) else {
        return;
    };
    for key in [crate::output::RESULT_KEY, "code", "exit"] {
        if let Some(value) = source.get(key) {
            obj.entry(key.to_string()).or_insert_with(|| value.clone());
        }
    }
}

enum NumericReduction {
    Sum,
    Min,
    Max,
}

fn reduce_numeric(events: &[PositionedValue], field: &str, mode: NumericReduction) -> Value {
    let values = events.iter().filter_map(|e| {
        e.value
            .as_object()
            .and_then(|o| o.get(field))
            .and_then(Value::as_f64)
    });
    match mode {
        NumericReduction::Sum => {
            let total: f64 = values.sum();
            number_from_f64(total)
        }
        NumericReduction::Min => match values.reduce(f64::min) {
            Some(m) => number_from_f64(m),
            None => Value::Null,
        },
        NumericReduction::Max => match values.reduce(f64::max) {
            Some(m) => number_from_f64(m),
            None => Value::Null,
        },
    }
}

fn number_from_f64(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < crate::constants::JSON_SAFE_INTEGER_LIMIT {
        Value::from(v as i64)
    } else {
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}
