// SPDX-License-Identifier: MIT OR Apache-2.0

//! A JSON Schema validator for the subset the published schemas actually use.
//!
//! The old check compared `required` and unknown keys and stopped there, so a
//! schema could declare `integer` where the binary emits a string and the suite
//! stayed green. This module checks the value as well, with no new dependency:
//! the keywords below are exactly the ones `docs/schemas/*.json` carries, and a
//! schema that starts using another one fails the guard in
//! `tests/docs_schemas_sync.rs` instead of being silently trusted.
//!
//! Supported: `type` (string or list of strings), `properties`, `required`,
//! `additionalProperties` (bool or schema), `items` (schema), `enum`, `const`,
//! `minimum`, `maximum`, `anyOf`, `oneOf`, `if`/`then`, `$defs` and `$ref`.
//! `format` is documentation in these schemas (`uint64`, `double`, `float` are
//! serde widths, not JSON Schema formats), so it is accepted and not enforced.

use serde_json::Value;

/// Every keyword the validator understands, so a schema cannot lean on one it
/// does not implement and still pass.
const SUPPORTED_KEYWORDS: &[&str] = &[
    "$schema",
    "$defs",
    "$ref",
    "additionalProperties",
    "anyOf",
    "const",
    "description",
    "enum",
    "format",
    "if",
    "items",
    "maximum",
    "minimum",
    "oneOf",
    "properties",
    "required",
    "then",
    "title",
    "type",
];

/// Every keyword of `schema` the validator does not implement, `$defs` member
/// names excluded (those are definitions, not keywords).
///
/// A property named like a keyword is not a keyword: the walk only treats a
/// mapping as a schema where a schema is expected.
pub fn unsupported_keywords(schema: &Value) -> Vec<String> {
    let mut found = Vec::new();
    walk_schema(schema, &mut found);
    found
}

fn walk_schema(schema: &Value, found: &mut Vec<String>) {
    let Some(object) = schema.as_object() else {
        return;
    };
    for (key, value) in object {
        if !SUPPORTED_KEYWORDS.contains(&key.as_str()) {
            found.push(key.clone());
            continue;
        }
        match key.as_str() {
            "properties" | "$defs" => {
                if let Some(map) = value.as_object() {
                    for sub in map.values() {
                        walk_schema(sub, found);
                    }
                }
            }
            "items" | "additionalProperties" | "if" | "then" => {
                if value.is_object() {
                    walk_schema(value, found);
                }
            }
            "anyOf" | "oneOf" => {
                if let Some(list) = value.as_array() {
                    for sub in list {
                        walk_schema(sub, found);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Every failure of `instance` against `schema`, as `path: reason` strings.
///
/// An empty vector means the instance conforms. `root` resolves `$ref`, which
/// these schemas write as `#/$defs/Name`.
pub fn check(root: &Value, schema: &Value, instance: &Value) -> Vec<String> {
    let mut failures = Vec::new();
    check_at(root, schema, instance, "", &mut failures);
    failures
}

fn resolve<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let name = reference.strip_prefix("#/$defs/")?;
    root.get("$defs")?.get(name)
}

fn check_at(root: &Value, schema: &Value, instance: &Value, path: &str, out: &mut Vec<String>) {
    let Some(object) = schema.as_object() else {
        return;
    };
    let here = if path.is_empty() { "/" } else { path };

    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        match resolve(root, reference) {
            Some(target) => check_at(root, target, instance, path, out),
            None => out.push(format!("{here}: unresolved $ref {reference}")),
        }
        return;
    }

    if let Some(branches) = object.get("anyOf").and_then(Value::as_array) {
        let ok = branches
            .iter()
            .any(|branch| check(root, branch, instance).is_empty());
        if !ok {
            out.push(format!("{here}: matches no anyOf branch"));
        }
    }

    if let Some(branches) = object.get("oneOf").and_then(Value::as_array) {
        let hits = branches
            .iter()
            .filter(|branch| check(root, branch, instance).is_empty())
            .count();
        if hits != 1 {
            out.push(format!("{here}: matches {hits} oneOf branches, needs 1"));
        }
    }

    if let Some(condition) = object.get("if")
        && let Some(then) = object.get("then")
        && check(root, condition, instance).is_empty()
    {
        check_at(root, then, instance, path, out);
    }

    if let Some(types) = object.get("type") {
        let allowed: Vec<&str> = match types {
            Value::String(one) => vec![one.as_str()],
            Value::Array(list) => list.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        if !allowed.is_empty() && !allowed.iter().any(|t| type_matches(t, instance)) {
            out.push(format!(
                "{here}: expected {}, got {}",
                allowed.join(" or "),
                kind_of(instance)
            ));
            return;
        }
    }

    if let Some(options) = object.get("enum").and_then(Value::as_array)
        && !options.iter().any(|option| option == instance)
    {
        out.push(format!("{here}: {instance} is not one of the enum values"));
    }

    if let Some(expected) = object.get("const")
        && expected != instance
    {
        out.push(format!("{here}: {instance} is not the const {expected}"));
    }

    if let Some(number) = instance.as_f64() {
        if let Some(min) = object.get("minimum").and_then(Value::as_f64)
            && number < min
        {
            out.push(format!("{here}: {number} is below the minimum {min}"));
        }
        if let Some(max) = object.get("maximum").and_then(Value::as_f64)
            && number > max
        {
            out.push(format!("{here}: {number} is above the maximum {max}"));
        }
    }

    if let Some(item_schema) = object.get("items")
        && let Some(list) = instance.as_array()
    {
        for (index, item) in list.iter().enumerate() {
            check_at(root, item_schema, item, &format!("{path}/{index}"), out);
        }
    }

    let Some(record) = instance.as_object() else {
        return;
    };
    let properties = object.get("properties").and_then(Value::as_object);

    if let Some(required) = object.get("required").and_then(Value::as_array) {
        for key in required.iter().filter_map(Value::as_str) {
            if !record.contains_key(key) {
                out.push(format!("{here}: required {key} missing"));
            }
        }
    }

    for (key, value) in record {
        let child = format!("{path}/{key}");
        match properties.and_then(|map| map.get(key)) {
            Some(property) => check_at(root, property, value, &child, out),
            None => match object.get("additionalProperties") {
                Some(Value::Bool(false)) => {
                    out.push(format!("{here}: {key} not declared"));
                }
                Some(extra @ Value::Object(_)) => check_at(root, extra, value, &child, out),
                _ => {}
            },
        }
    }
}

fn type_matches(name: &str, instance: &Value) -> bool {
    match name {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
        "number" => instance.is_number(),
        _ => false,
    }
}

fn kind_of(instance: &Value) -> &'static str {
    match instance {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
