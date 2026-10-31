// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 summary schemas: `--print-schema` serves one union per verb, and the
//! terminal branch of each union describes the keys its last event really carries.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

const SUMMARY_SCHEMAS: [&str; 2] = ["count-by-ext", "count-by-size"];

fn run_ok(ws: &Path, args: &[&str]) -> Vec<Value> {
    let output = common::atomwrite()
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(["--no-progress", "--no-auto-heal"])
        .args(args)
        .output()
        .expect("run");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    common::parse_ndjson(&output.stdout)
}

fn schema_keys(ws: &Path, name: &str) -> BTreeSet<String> {
    let output = common::atomwrite()
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(["--print-schema", name])
        .output()
        .expect("run");
    assert!(output.status.success(), "--print-schema {name}");
    let schema: Value = serde_json::from_slice(&output.stdout).expect("schema JSON");
    let mut keys = BTreeSet::new();
    let mut collect = |v: &Value| {
        if let Some(props) = v.get("properties").and_then(Value::as_object) {
            keys.extend(props.keys().cloned());
        }
    };
    collect(&schema);
    for combinator in ["oneOf", "anyOf", "allOf"] {
        if let Some(arr) = schema.get(combinator).and_then(Value::as_array) {
            arr.iter().for_each(&mut collect);
        }
    }
    keys
}

#[test]
fn print_schema_lists_the_summary_names() {
    let output = common::atomwrite()
        .args(["--print-schema"])
        .output()
        .expect("run");
    assert!(output.status.success());
    let names: BTreeSet<String> = common::parse_ndjson(&output.stdout)
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    for name in SUMMARY_SCHEMAS {
        assert!(names.contains(name), "{name} missing from {names:?}");
    }
}

#[test]
fn summary_schemas_describe_the_real_terminal_event_keys() {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    common::create_test_file(d, "src/a.rs", "fn needle() {}\n");
    common::create_test_file(d, "src/b.txt", "needle needle\n");
    let src = d.join("src");
    let src = src.to_str().expect("utf8 path");

    let search = run_ok(d, &["search", "needle", src]);
    let count_total = run_ok(d, &["count", src]);
    let by_ext = run_ok(d, &["count", "--by-extension", src]);
    let by_size = run_ok(d, &["count", "--by-size", "--top", "2", src]);
    let list = run_ok(d, &["list", src]);
    let backup = run_ok(d, &["backup", &format!("{src}/a.rs")]);
    let replace = run_ok(d, &["replace", "--dry-run", "needle", "thread", src]);

    let cases: [(&str, &Vec<Value>); 7] = [
        ("search", &search),
        ("count", &count_total),
        ("count-by-ext", &by_ext),
        ("count-by-size", &by_size),
        ("list", &list),
        ("backup", &backup),
        ("replace", &replace),
    ];
    let mut gaps = Vec::new();
    for (name, events) in cases {
        let terminal = events
            .last()
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("{name}: no terminal event"));
        let keys = schema_keys(d, name);
        let missing: Vec<&String> = terminal.keys().filter(|k| !keys.contains(*k)).collect();
        if !missing.is_empty() {
            gaps.push(format!("{name}: schema lacks {missing:?}"));
        }
    }
    assert!(gaps.is_empty(), "schema gaps:\n{}", gaps.join("\n"));
}

#[test]
fn diff_schema_describes_the_real_output_of_every_mode() {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    let a = common::create_test_file(d, "a.txt", "one\ntwo\nthree\n");
    let b = common::create_test_file(d, "b.txt", "one\n2\nthree\nfour\n");
    let (a, b) = (
        a.to_str().expect("utf8 path"),
        b.to_str().expect("utf8 path"),
    );
    let keys = schema_keys(d, "diff");
    let mut gaps = Vec::new();
    for extra in [&[][..], &["--unified"][..], &["--stat"][..]] {
        let mut args = vec!["diff"];
        args.extend_from_slice(extra);
        args.extend([a, b]);
        let events = run_ok(d, &args);
        assert!(!events.is_empty(), "diff {extra:?} wrote nothing");
        for event in events {
            let object = event.as_object().expect("object");
            let missing: Vec<&String> = object.keys().filter(|k| !keys.contains(*k)).collect();
            if !missing.is_empty() {
                gaps.push(format!("diff {extra:?}: schema lacks {missing:?}"));
            }
        }
    }
    assert!(gaps.is_empty(), "schema gaps:\n{}", gaps.join("\n"));
    for key in ["lines_a", "insertions", "content", "tag"] {
        assert!(keys.contains(key), "diff schema lacks {key}: {keys:?}");
    }
}

/// Keys of `schema` that an object is allowed to carry, or `None` when the
/// schema does not declare them (open object).
fn declared_keys(schema: &Value) -> Option<BTreeSet<String>> {
    (schema.get("additionalProperties") == Some(&Value::Bool(false))).then(|| {
        schema
            .get("properties")
            .and_then(Value::as_object)
            .map(|p| p.keys().cloned().collect())
            .unwrap_or_default()
    })
}

/// Why `event` does not satisfy `schema` (`required`, `additionalProperties`
/// and the `if` / `then` failure rule); empty when it does.
fn violations(event: &Value, schema: &Value) -> Vec<String> {
    // A union accepts the record when any branch does; otherwise report the
    // shortest list of complaints, which names the branch closest to it.
    if let Some(branches) = schema.get("anyOf").and_then(Value::as_array) {
        let mut shortest: Option<Vec<String>> = None;
        for branch in branches {
            let found = violations(event, branch);
            if found.is_empty() {
                return Vec::new();
            }
            shortest = Some(match shortest {
                Some(previous) if previous.len() <= found.len() => previous,
                _ => found,
            });
        }
        return shortest.unwrap_or_else(|| vec!["union declares no branch".into()]);
    }
    let mut found = Vec::new();
    let Some(object) = event.as_object() else {
        return vec!["not an object".into()];
    };
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("required must not be null: {schema}"));
    for key in required.iter().filter_map(Value::as_str) {
        if !object.contains_key(key) {
            found.push(format!("missing required `{key}`"));
        }
    }
    if let Some(declared) = declared_keys(schema) {
        for key in object.keys().filter(|k| !declared.contains(*k)) {
            found.push(format!("undeclared key `{key}`"));
        }
    }
    if object.get("ok") == Some(&Value::Bool(false))
        && let Some(then) = schema.get("then").and_then(|t| t.get("required"))
    {
        for key in then
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !object.contains_key(key) {
                found.push(format!("failed event lacks `{key}`"));
            }
        }
    }
    found
}

fn schema_of(ws: &Path, name: &str) -> Value {
    let output = common::atomwrite()
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(["--print-schema", name])
        .output()
        .expect("run");
    assert!(output.status.success(), "--print-schema {name}");
    serde_json::from_slice(&output.stdout).expect("schema JSON")
}

/// Run a verb that may fail and return its exit code with every event.
fn run_any(ws: &Path, args: &[&str]) -> (Option<i32>, Vec<Value>) {
    let output = common::atomwrite()
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(["--no-progress", "--no-auto-heal"])
        .args(args)
        .output()
        .expect("run");
    (output.status.code(), common::parse_ndjson(&output.stdout))
}

#[test]
fn closed_terminal_schemas_have_required_and_reject_undeclared_keys() {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    common::create_test_file(d, "src/a.rs", "fn needle() {}\n");
    common::create_test_file(d, "src/b.txt", "needle needle\n");
    let src = d.join("src");
    let src = src.to_str().expect("utf8 path");
    let a = format!("{src}/a.rs");
    let cases: Vec<(&str, Vec<&str>)> = vec![
        ("search", vec!["search", "needle", src]),
        ("search", vec!["search", "NOPE", src]),
        (
            "replace",
            vec!["replace", "--dry-run", "needle", "thread", src],
        ),
        ("replace", vec!["replace", "--dry-run", "NOPE", "x", src]),
        ("count", vec!["count", src]),
        ("count-by-ext", vec!["count", "--by-extension", src]),
        (
            "count-by-size",
            vec!["count", "--by-size", "--top", "2", src],
        ),
        ("list", vec!["list", src]),
        ("list", vec!["list", "--long", "--count-by-ext", src]),
        ("backup", vec!["backup", &a]),
        ("config", vec!["config", "get", "fuzzy.threshold"]),
        ("config", vec!["config", "list"]),
    ];
    let mut gaps = Vec::new();
    for (name, args) in &cases {
        let schema = schema_of(d, name);
        let (_, events) = run_any(d, args);
        let terminal = events
            .last()
            .unwrap_or_else(|| panic!("{args:?}: no event"));
        for violation in violations(terminal, &schema) {
            gaps.push(format!("{name} {args:?}: {violation}: {terminal}"));
        }
    }
    assert!(gaps.is_empty(), "schema violations:\n{}", gaps.join("\n"));
}

/// The branch of a union that carries `title`.
fn branch_titled<'a>(schema: &'a Value, title: &str) -> &'a Value {
    schema
        .get("anyOf")
        .and_then(Value::as_array)
        .and_then(|branches| {
            branches
                .iter()
                .find(|branch| branch.get("title").and_then(Value::as_str) == Some(title))
        })
        .unwrap_or_else(|| panic!("no branch titled {title}: {schema}"))
}

#[test]
fn search_and_replace_summary_schemas_are_distinct_and_state_their_own_keys() {
    let ws = tempfile::tempdir().expect("workspace");
    let search = branch_titled(&schema_of(ws.path(), "search"), "SearchSummary").clone();
    let replace = branch_titled(&schema_of(ws.path(), "replace"), "ReplaceSummary").clone();
    assert_ne!(search["title"], replace["title"]);
    let props = |s: &Value| declared_keys(s).expect("closed schema");
    assert!(!props(&search).contains("total_replacements"));
    assert!(!props(&search).contains("files_modified"));
    assert!(props(&search).contains("total_matches"));
    assert!(!props(&replace).contains("total_matches"));
    assert!(props(&replace).contains("total_replacements"));
    let required = |s: &Value| -> BTreeSet<String> {
        s["required"]
            .as_array()
            .expect("required")
            .iter()
            .filter_map(|k| k.as_str().map(str::to_string))
            .collect()
    };
    for key in [
        "type",
        "ok",
        "elapsed_ms",
        "files_visited",
        "files_matched",
        "total_matches",
    ] {
        assert!(required(&search).contains(key), "search lacks {key}");
    }
    for key in [
        "type",
        "ok",
        "elapsed_ms",
        "files_visited",
        "total_replacements",
    ] {
        assert!(required(&replace).contains(key), "replace lacks {key}");
    }
}

/// The schemas must reject what no verb emits, or the check above proves nothing.
#[test]
fn closed_terminal_schemas_reject_objects_no_verb_emits() {
    let ws = tempfile::tempdir().expect("workspace");
    let schema = branch_titled(&schema_of(ws.path(), "search"), "SearchSummary").clone();
    let bare = violations(&serde_json::json!({"type": "summary"}), &schema);
    assert!(bare.iter().any(|v| v.contains("files_visited")), "{bare:?}");
    assert!(bare.iter().any(|v| v.contains("`ok`")), "{bare:?}");
    let alien = serde_json::json!({
        "type": "summary", "ok": true, "files_visited": 1, "files_matched": 1,
        "total_matches": 1, "elapsed_ms": 1, "total_replacements": 3
    });
    let found = violations(&alien, &schema);
    assert!(
        found.iter().any(|v| v.contains("total_replacements")),
        "{found:?}"
    );
    let failed = serde_json::json!({
        "type": "summary", "ok": false, "files_visited": 1, "files_matched": 0,
        "total_matches": 0, "elapsed_ms": 1
    });
    let found = violations(&failed, &schema);
    assert!(found.iter().any(|v| v.contains("`code`")), "{found:?}");
}

#[test]
fn diff_schema_branches_validate_every_real_record() {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    let a = common::create_test_file(d, "a.txt", "one\ntwo\nthree\n");
    let b = common::create_test_file(d, "b.txt", "one\n2\nthree\nfour\n");
    let (a, b) = (a.to_str().expect("utf8"), b.to_str().expect("utf8"));
    let schema = schema_of(d, "diff");
    assert_eq!(schema["type"], "object", "the root is an object");
    assert!(schema.get("anyOf").is_none(), "the union is a oneOf");
    let branches = schema["oneOf"].as_array().expect("oneOf");
    assert_eq!(branches.len(), 5, "change, summary, unified, stat, error");
    for branch in branches {
        assert!(
            branch["required"].is_array(),
            "branch without required: {branch}"
        );
    }
    for extra in [&[][..], &["--unified"][..], &["--stat"][..]] {
        let mut args = vec!["diff"];
        args.extend_from_slice(extra);
        args.extend([a, b]);
        let (_, events) = run_any(d, &args);
        for event in events {
            let accepting = branches
                .iter()
                .filter(|branch| violations(&event, branch).is_empty())
                .count();
            assert_eq!(
                accepting, 1,
                "diff {extra:?}: oneOf needs exactly one for {event}"
            );
        }
    }
}

#[test]
fn count_serves_the_total_output_schema() {
    let ws = tempfile::tempdir().expect("workspace");
    let keys = schema_keys(ws.path(), "count");
    assert!(keys.contains("total"), "count schema keys: {keys:?}");
    assert!(keys.contains("mode"), "count schema keys: {keys:?}");
}

#[test]
fn failure_keys_are_part_of_the_search_and_replace_summary_schemas() {
    let ws = tempfile::tempdir().expect("workspace");
    for name in ["search", "replace"] {
        let keys = schema_keys(ws.path(), name);
        for key in ["files_failed", "errors", "files_visited", "elapsed_ms"] {
            assert!(keys.contains(key), "{name} lacks {key}: {keys:?}");
        }
    }
}
