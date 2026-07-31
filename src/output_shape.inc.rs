// SPDX-License-Identifier: MIT OR Apache-2.0
// Agent-native NDJSON stdout shaping: --filter / --select / --limit (v0.1.36 residual).
// Included from output.rs. Zero-cost identity path when no shape flags are set.

/// A single top-level filter predicate (`key=value`, `key==value`, or `key!=value`).
#[derive(Debug, Clone, PartialEq)]
pub struct FilterPred {
    /// Top-level object key.
    pub key: String,
    /// Expected JSON-ish value after parse.
    pub value: serde_json::Value,
    /// When true, require inequality.
    pub not_equal: bool,
}

/// Optional projection / filter / limit applied by [`NdjsonWriter`].
#[derive(Debug, Clone, Default)]
pub struct OutputShape {
    /// Top-level keys to keep (order preserved). `None` = no projection.
    pub select: Option<Vec<String>>,
    /// AND-combined equality filters. Empty = no filter.
    pub filters: Vec<FilterPred>,
    /// Max events after filter. `None` = unlimited.
    pub limit: Option<u64>,
}

impl OutputShape {
    /// True when writer may serialize directly without intermediate `Value`.
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.select.is_none() && self.filters.is_empty() && self.limit.is_none()
    }

    /// Build from global CLI flags. Empty select CSV is treated as no select.
    pub fn from_cli(
        select: Option<&str>,
        filters: &[String],
        limit: Option<u64>,
    ) -> Result<Self, crate::error::AtomwriteError> {
        let select = match select.map(str::trim).filter(|s| !s.is_empty()) {
            Some(csv) => {
                let keys: Vec<String> = csv
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                if keys.is_empty() {
                    None
                } else {
                    Some(keys)
                }
            }
            None => None,
        };
        let mut preds = Vec::with_capacity(filters.len());
        for f in filters {
            preds.push(parse_filter_expr(f)?);
        }
        Ok(Self {
            select,
            filters: preds,
            limit,
        })
    }
}

/// Parse `key=value`, `key==value`, or `key!=value` (top-level only).
pub fn parse_filter_expr(raw: &str) -> Result<FilterPred, crate::error::AtomwriteError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "empty --filter expression".into(),
        });
    }
    let (key, val_raw, not_equal) = if let Some(rest) = s.split_once("!=") {
        (rest.0, rest.1, true)
    } else if let Some(rest) = s.split_once("==") {
        (rest.0, rest.1, false)
    } else if let Some(rest) = s.split_once('=') {
        (rest.0, rest.1, false)
    } else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "invalid --filter {raw:?}: expected key=value, key==value, or key!=value"
            ),
        });
    };
    let key = key.trim();
    if key.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid --filter {raw:?}: empty key"),
        });
    }
    let value = parse_filter_value(val_raw.trim());
    Ok(FilterPred {
        key: key.to_string(),
        value,
        not_equal,
    })
}

fn parse_filter_value(raw: &str) -> serde_json::Value {
    match raw {
        "true" => serde_json::Value::Bool(true),
        "false" => serde_json::Value::Bool(false),
        "null" => serde_json::Value::Null,
        s if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 => {
            serde_json::Value::String(s[1..s.len() - 1].to_string())
        }
        s => {
            if let Ok(n) = s.parse::<i64>() {
                serde_json::Value::Number(n.into())
            } else if let Ok(n) = s.parse::<u64>() {
                serde_json::Value::Number(n.into())
            } else if let Ok(n) = s.parse::<f64>() {
                serde_json::Number::from_f64(n)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(s.to_string()))
            } else {
                serde_json::Value::String(s.to_string())
            }
        }
    }
}

/// Apply filters then optional key projection. Returns `None` when filtered out.
///
/// Used for **success** events only. Error envelopes must use
/// [`shape_error_value`] so aggressive `--filter` never silences failures
/// (agent-native always-emit errors, v0.1.36 residual).
pub fn shape_value(
    v: serde_json::Value,
    shape: &OutputShape,
) -> Option<serde_json::Value> {
    if !shape.filters.is_empty() {
        let obj = v.as_object()?;
        for pred in &shape.filters {
            let actual = obj.get(&pred.key).cloned().unwrap_or(serde_json::Value::Null);
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
    Some(project_select(v, shape))
}

/// Shape an **error** envelope for agent-native stdout.
///
/// - **Never** applies `--filter` (errors always emit).
/// - **Never** returns empty / drop semantics for filter mismatch.
/// - **May** apply `--select` / `--fields` projection so agents can thin keys.
/// - Callers must **not** count errors against `--limit` (success-only budget).
pub fn shape_error_value(v: serde_json::Value, shape: &OutputShape) -> serde_json::Value {
    project_select(v, shape)
}

fn project_select(mut v: serde_json::Value, shape: &OutputShape) -> serde_json::Value {
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
        return serde_json::Value::Object(out);
    }
    v
}

fn json_values_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(x), serde_json::Value::Number(y)) => {
            if let (Some(xf), Some(yf)) = (x.as_f64(), y.as_f64()) {
                (xf - yf).abs() < f64::EPSILON
            } else {
                x == y
            }
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod output_shape_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_eq_bool() {
        let p = parse_filter_expr("ok=false").unwrap();
        assert_eq!(p.key, "ok");
        assert_eq!(p.value, json!(false));
        assert!(!p.not_equal);
    }

    #[test]
    fn parse_neq_string() {
        let p = parse_filter_expr("code!=WORKSPACE_JAIL").unwrap();
        assert!(p.not_equal);
        assert_eq!(p.value, json!("WORKSPACE_JAIL"));
    }

    #[test]
    fn project_select_omits_missing() {
        let shape = OutputShape {
            select: Some(vec!["ok".into(), "missing".into(), "code".into()]),
            filters: vec![],
            limit: None,
        };
        let out = shape_value(json!({"ok": false, "code": "X", "noise": 1}), &shape).unwrap();
        assert_eq!(out, json!({"ok": false, "code": "X"}));
    }

    #[test]
    fn filter_drops_non_match() {
        let shape = OutputShape {
            select: None,
            filters: vec![parse_filter_expr("ok=false").unwrap()],
            limit: None,
        };
        assert!(shape_value(json!({"ok": true}), &shape).is_none());
        assert!(shape_value(json!({"ok": false}), &shape).is_some());
    }

    #[test]
    fn identity_when_empty() {
        assert!(OutputShape::default().is_identity());
    }

    #[test]
    fn error_shape_ignores_filter_keeps_select() {
        let shape = OutputShape {
            select: Some(vec!["ok".into(), "code".into()]),
            filters: vec![parse_filter_expr("ok=true").unwrap()],
            limit: Some(0),
        };
        let err = json!({"ok": false, "error": true, "code": "WORKSPACE_JAIL", "noise": 1});
        // Success path would drop (ok != true).
        assert!(shape_value(err.clone(), &shape).is_none());
        // Error path always emits and may project.
        let out = shape_error_value(err, &shape);
        assert_eq!(out, json!({"ok": false, "code": "WORKSPACE_JAIL"}));
    }
}
