// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

use serde_json::Value;

#[test]
fn clap_error_edit_missing_value_has_suggestion() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "target.txt", "hello\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "edit", "--old"])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(2), "expected exit 2");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(json["code"], "ARGUMENT_PARSE_ERROR");
    let suggestion = json["suggestion"].as_str().unwrap_or("");
    assert!(
        suggestion.contains("--old-file"),
        "suggestion should mention --old-file, got: {suggestion}"
    );
}

#[test]
fn clap_error_conflicts_with_old_file_has_suggestion() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "target.txt", "hello\n");
    let old_file = common::create_test_file(dir.path(), "old.txt", "hello");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "hello",
            "--old-file",
        ])
        .arg(&old_file)
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(2), "expected exit 2");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(json["code"], "ARGUMENT_PARSE_ERROR");
    let suggestion = json["suggestion"].as_str().unwrap_or("");
    assert!(
        suggestion.contains("--old-file"),
        "suggestion should mention --old-file, got: {suggestion}"
    );
}

#[test]
fn clap_error_envelope_structure_complete() {
    let output = common::atomwrite()
        .args(["--workspace", "/tmp", "edit"])
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(2), "expected exit 2");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim()).expect("valid JSON");

    assert_eq!(json["error"], true);
    assert_eq!(json["code"], "ARGUMENT_PARSE_ERROR");
    assert_eq!(json["exit"], 2);
    assert!(json["message"].is_string());
    assert_eq!(json["error_class"], "permanent");
    assert_eq!(json["retryable"], false);
}

#[test]
fn clap_error_non_edit_no_old_file_suggestion() {
    let output = common::atomwrite()
        .args(["--workspace", "/tmp", "search"])
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(2), "expected exit 2");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(json["code"], "ARGUMENT_PARSE_ERROR");

    let suggestion = json["suggestion"].as_str().unwrap_or("");
    assert!(
        !suggestion.contains("--old-file"),
        "search error should NOT mention --old-file, got: {suggestion}"
    );
}

#[test]
fn clap_error_unknown_flag_in_non_edit_no_false_positive() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--nonexistent-flag",
        ])
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(2), "expected exit 2");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(json["code"], "ARGUMENT_PARSE_ERROR");

    let suggestion = json["suggestion"].as_str().unwrap_or("");
    assert!(
        !suggestion.contains("--old-file"),
        "hash error with unknown flag should NOT mention --old-file, got: {suggestion}"
    );
}
