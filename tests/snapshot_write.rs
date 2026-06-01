// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn write_ndjson_structure_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("snap.txt");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .write_stdin("snapshot content\n")
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 1);

    let mut event = events[0].clone();
    event["elapsed_ms"] = serde_json::json!("[redacted]");
    event["path"] = serde_json::json!("[redacted]");
    event["checksum"] = serde_json::json!("[redacted]");

    insta::assert_json_snapshot!("write_output_structure", event);
}

#[test]
fn read_ndjson_structure_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "snap_read.txt", "hello world\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let mut event = events[0].clone();
    event["path"] = serde_json::json!("[redacted]");
    event["checksum"] = serde_json::json!("[redacted]");
    event["modified"] = serde_json::json!("[redacted]");
    event["permissions"] = serde_json::json!("[redacted]");

    insta::assert_json_snapshot!("read_output_structure", event);
}

#[test]
fn error_json_structure_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(dir.path().join("nonexistent.txt"))
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(4));
    let events = common::parse_ndjson(&output.stdout);

    let mut event = events[0].clone();
    event["message"] = serde_json::json!("[redacted]");
    event["path"] = serde_json::json!("[redacted]");
    event["suggestion"] = serde_json::json!("[redacted]");

    insta::assert_json_snapshot!("error_not_found_structure", event);
}

#[test]
fn search_match_ndjson_structure_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "search_snap.txt", "hello world\nfoo bar\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "hello",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let match_event = events.iter().find(|e| e["type"] == "match");
    assert!(match_event.is_some(), "expected a match event");

    let mut event = match_event.expect("match").clone();
    event["path"] = serde_json::json!("[redacted]");
    event["byte_offset"] = serde_json::json!("[redacted]");

    insta::assert_json_snapshot!("search_match_structure", event);
}

#[test]
fn replace_result_ndjson_structure_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "replace_snap.txt", "old_value here\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "old_value",
            "new_value",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let replaced = events.iter().find(|e| e["type"] == "replaced");
    assert!(replaced.is_some(), "expected a replaced event");

    let mut event = replaced.expect("replaced").clone();
    event["path"] = serde_json::json!("[redacted]");
    event["checksum_before"] = serde_json::json!("[redacted]");
    event["checksum_after"] = serde_json::json!("[redacted]");
    event["elapsed_ms"] = serde_json::json!("[redacted]");

    insta::assert_json_snapshot!("replace_result_structure", event);
}

#[test]
fn batch_summary_ndjson_structure_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("batch_snap.txt");

    let manifest = format!(
        r#"{{"op":"write","target":"{}","content":"batch snap"}}"#,
        target.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let summary = events.iter().find(|e| e["type"] == "summary");
    assert!(summary.is_some(), "expected a summary event");

    let mut event = summary.expect("summary").clone();
    event["elapsed_ms"] = serde_json::json!("[redacted]");

    insta::assert_json_snapshot!("batch_summary_structure", event);
}

#[test]
fn edit_output_ndjson_structure_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "edit_snap.txt", "line1\nold_text\nline3\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            path.to_str().unwrap(),
            "--old",
            "old_text",
            "--new",
            "new_text",
        ])
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let edit_event = events.iter().find(|e| e["type"] == "edit");
    assert!(edit_event.is_some(), "expected an edit event");

    let mut event = edit_event.expect("edit").clone();
    event["path"] = serde_json::json!("[redacted]");
    event["checksum_before"] = serde_json::json!("[redacted]");
    event["checksum_after"] = serde_json::json!("[redacted]");
    event["elapsed_ms"] = serde_json::json!("[redacted]");

    insta::assert_json_snapshot!("edit_output_structure", event);
}

#[test]
fn error_invalid_input_structure_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin("{\"op\":\"write\"}\n{bad json")
        .output()
        .expect("run");

    assert!(!output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let error_event = events
        .iter()
        .find(|e| e.get("error") == Some(&serde_json::json!(true)));
    assert!(error_event.is_some(), "expected an error event");

    let mut event = error_event.expect("error").clone();
    event["message"] = serde_json::json!("[redacted]");
    event["suggestion"] = serde_json::json!("[redacted]");

    insta::assert_json_snapshot!("error_invalid_input_structure", event);
}

#[test]
fn error_workspace_jail_structure_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "/etc/evil.txt",
        ])
        .write_stdin("payload")
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(126));
    let events = common::parse_ndjson(&output.stdout);

    let error_event = events
        .iter()
        .find(|e| e.get("error") == Some(&serde_json::json!(true)));
    assert!(error_event.is_some(), "expected an error event");

    let mut event = error_event.expect("error").clone();
    event["message"] = serde_json::json!("[redacted]");
    event["path"] = serde_json::json!("[redacted]");
    event["suggestion"] = serde_json::json!("[redacted]");
    event["workspace"] = serde_json::json!("[redacted]");

    insta::assert_json_snapshot!("error_workspace_jail_structure", event);
}
