// SPDX-License-Identifier: MIT OR Apache-2.0

//! `doctor --strict` with a failing check is a settled host verdict: exit 1,
//! ONE terminal event (the report itself, stamped `ok: false`), the typed
//! `precondition_failed` class and no "bug" hint. Without `--strict` the same
//! failing check still exits 0.

#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use serde_json::Value;

/// A workspace the process cannot write into, restored to 755 on drop so the
/// tempdir can be deleted.
struct ReadOnlyWorkspace {
    dir: tempfile::TempDir,
}

impl ReadOnlyWorkspace {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555))
            .expect("chmod 555");
        Self { dir }
    }

    /// `None` when the process can write anyway (root), so the check cannot fail.
    fn can_force_failure(&self) -> bool {
        std::fs::File::create(self.dir.path().join(".probe")).is_err()
    }
}

impl Drop for ReadOnlyWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(self.dir.path(), std::fs::Permissions::from_mode(0o755));
    }
}

fn run_doctor(ws: &ReadOnlyWorkspace, strict: bool) -> (Option<i32>, Vec<Value>) {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut cmd = common::hermetic(Command::new(bin));
    cmd.args([
        "-v",
        "--workspace",
        ws.dir.path().to_str().expect("utf8 workspace"),
        "--no-progress",
        "--no-auto-heal",
        "doctor",
    ]);
    if strict {
        cmd.arg("--strict");
    }
    let output = cmd.output().expect("run atomwrite doctor");
    (output.status.code(), common::parse_ndjson(&output.stdout))
}

#[test]
fn strict_failure_exits_one_with_a_single_typed_terminal_event() {
    let ws = ReadOnlyWorkspace::new();
    if !ws.can_force_failure() {
        return;
    }
    let (code, events) = run_doctor(&ws, true);
    assert_eq!(code, Some(1), "events: {events:?}");
    let failed: Vec<&Value> = events.iter().filter(|e| e["ok"] == false).collect();
    assert_eq!(
        failed.len(),
        1,
        "exactly one failed terminal event: {events:?}"
    );
    assert_eq!(events.len(), 1, "no line follows the report: {events:?}");
    let ev = failed[0];
    assert_eq!(ev["type"], "doctor_report");
    assert_eq!(ev["code"], "DOCTOR_CHECKS_FAILED");
    assert_eq!(ev["exit"], 1);
    assert_eq!(ev["error_class"], "precondition_failed");
    assert_eq!(ev["retryable"], false);
    let suggestion = ev["suggestion"].as_str().unwrap_or_default();
    assert!(!suggestion.is_empty(), "suggestion must say what to do");
    assert!(
        !suggestion.to_lowercase().contains("bug"),
        "a host verdict is not a bug: {suggestion}"
    );
    assert!(ev["checks"].as_array().is_some_and(|c| !c.is_empty()));
}

#[test]
fn non_strict_keeps_exit_zero_with_the_failing_check() {
    let ws = ReadOnlyWorkspace::new();
    if !ws.can_force_failure() {
        return;
    }
    let (code, events) = run_doctor(&ws, false);
    assert_eq!(code, Some(0), "events: {events:?}");
    assert_eq!(events.len(), 1);
    let report = &events[0];
    assert_eq!(report["type"], "doctor_report");
    assert_eq!(report["ok"], false);
    assert!(
        report.get("code").is_none(),
        "no failure stamp without --strict"
    );
    let failing = report["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .any(|c| c["status"] == "fail");
    assert!(failing, "the read-only workspace must fail a check");
}

/// The schema the binary prints for `name`.
fn printed_schema(name: &str) -> Value {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = common::hermetic(Command::new(bin))
        .args(["--print-schema", name])
        .output()
        .expect("run --print-schema");
    assert!(output.status.success(), "--print-schema {name} failed");
    serde_json::from_slice(&output.stdout).expect("schema JSON")
}

/// Gaps between `event` and a closed object schema: a `required` key that is
/// missing, or a key the schema does not declare.
fn closed_schema_gaps(schema: &Value, event: &Value) -> Vec<String> {
    let mut gaps = Vec::new();
    let object = event.as_object().expect("event object");
    let declared = schema["properties"].as_object().expect("schema properties");
    assert_eq!(
        schema["additionalProperties"], false,
        "schema must be closed"
    );
    for key in schema["required"].as_array().expect("required") {
        let key = key.as_str().expect("required key");
        if !object.contains_key(key) {
            gaps.push(format!("required key {key} is missing"));
        }
    }
    for key in object.keys() {
        if !declared.contains_key(key) {
            gaps.push(format!("key {key} is not declared"));
        }
    }
    gaps
}

#[test]
fn printed_doctor_schema_describes_the_failed_strict_report_and_the_plain_one() {
    let ws = ReadOnlyWorkspace::new();
    if !ws.can_force_failure() {
        return;
    }
    let schema = printed_schema("doctor");
    for strict in [true, false] {
        let (_, events) = run_doctor(&ws, strict);
        let gaps = closed_schema_gaps(&schema, &events[0]);
        assert!(gaps.is_empty(), "strict={strict}: {gaps:?}");
    }
    // The failure detail is optional: the plain report must stay valid without it.
    let required: Vec<&str> = schema["required"]
        .as_array()
        .expect("required")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for detail in ["code", "exit", "error_class", "retryable", "suggestion"] {
        assert!(!required.contains(&detail), "{detail} must be optional");
        assert!(
            schema["properties"][detail].is_object(),
            "{detail} must be declared"
        );
    }
}

#[test]
fn docs_doctor_schema_matches_the_printed_one() {
    let docs: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/schemas/doctor.schema.json"
        ))
        .expect("read docs schema"),
    )
    .expect("docs schema JSON");
    assert_eq!(docs, printed_schema("doctor"));
}
