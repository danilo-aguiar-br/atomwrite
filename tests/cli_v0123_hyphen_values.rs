// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn edit_old_new_hyphen_bullet_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "bullet.txt", "- bullet point\nsome text\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "- bullet point",
            "--new",
            "replaced text",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --old '- bullet point' failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("replaced text"));
    assert!(!content.contains("- bullet point"));
}

#[test]
fn edit_old_new_double_dash_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "sep.txt", "-- separator --\ndata\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "-- separator --",
            "--new",
            "=== separator ===",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --old '-- separator --' failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("=== separator ==="));
    assert!(!content.contains("-- separator --"));
}

#[test]
fn edit_between_hyphen_markers_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(
        dir.path(),
        "between.txt",
        "header\n- start\nmiddle content\n- end\nfooter\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--between",
            "- start",
            "- end",
        ])
        .arg(&path)
        .write_stdin("replaced block\n")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --between '- start' '- end' failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("replaced block"));
    assert!(!content.contains("middle content"));
}

#[test]
fn edit_after_match_hyphen_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "after.txt", "before\n- marker\nafter\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--after-match",
            "- marker",
        ])
        .arg(&path)
        .write_stdin("inserted\n")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --after-match '- marker' failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    let marker_pos = content.find("- marker").expect("marker present");
    let inserted_pos = content.find("inserted").expect("inserted present");
    assert!(inserted_pos > marker_pos);
}

#[test]
fn edit_before_match_hyphen_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "before.txt", "before\n- marker\nafter\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--before-match",
            "- marker",
        ])
        .arg(&path)
        .write_stdin("inserted\n")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --before-match '- marker' failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    let marker_pos = content.find("- marker").expect("marker present");
    let inserted_pos = content.find("inserted").expect("inserted present");
    assert!(inserted_pos < marker_pos);
}

#[test]
fn search_hyphen_pattern_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "dep.txt", "-deprecated function\nnormal line\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "-deprecated",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert_ne!(
        output.status.code().unwrap_or(-1),
        2,
        "search '-deprecated' was rejected as arg parse error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "search '-deprecated' failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let matches: Vec<_> = events.iter().filter(|e| e["type"] == "match").collect();
    assert!(!matches.is_empty());
}

#[test]
fn replace_hyphen_pattern_dry_run_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "api.txt", "-old-api call\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--dry-run",
            "-old-api",
            "-new-api",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert_ne!(
        output.status.code().unwrap_or(-1),
        2,
        "replace '-old-api' was rejected as arg parse error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn calc_negative_number_succeeds() {
    let output = common::atomwrite()
        .args(["calc", "-5 + 3"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "calc '-5 + 3' failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    assert!(!events.is_empty());
    let result_str = events[0].to_string();
    assert!(
        result_str.contains("-2") || result_str.contains("\u{2212}2"),
        "expected -2 in result, got: {result_str}"
    );
}

#[test]
fn edit_multi_pair_hyphen_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(
        dir.path(),
        "multi.txt",
        "- first item\ntext\n- second item\nmore\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "- first item",
            "--new",
            "item one",
            "--old",
            "- second item",
            "--new",
            "item two",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit multi-pair with hyphens failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("item one"));
    assert!(content.contains("item two"));
    assert!(!content.contains("- first item"));
    assert!(!content.contains("- second item"));
}

#[test]
fn edit_partial_hyphen_succeeds() {
    use serde_json::Value;

    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "partial.txt", "- found item\nmissing\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--partial",
            "--old",
            "- found item",
            "--new",
            "replaced",
            "--old",
            "- not found",
            "--new",
            "x",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --partial with hyphens failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    assert!(!events.is_empty());
    let result = &events[0];
    if let Some(pair_results) = result.get("pair_results") {
        let pairs: Vec<Value> = serde_json::from_value(pair_results.clone()).expect("pairs");
        assert!(pairs[0]["matched"].as_bool().unwrap_or(false));
        assert!(!pairs[1]["matched"].as_bool().unwrap_or(true));
    }
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("replaced"));
}

#[test]
fn read_grep_hyphen_pattern_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "todo.txt", "-TODO: fix this\nnormal line\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--grep",
            "-TODO",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_ne!(
        output.status.code().unwrap_or(-1),
        2,
        "read --grep '-TODO' was rejected as arg parse error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "read --grep '-TODO' failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("-TODO"),
        "expected grep result with -TODO, got: {stdout}"
    );
}

#[test]
fn transform_hyphen_pattern_dry_run_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "test.rs", "fn test() { -old_call(); }\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "transform",
            "--dry-run",
            "-p",
            "-old_call()",
            "-r",
            "-new_call()",
            "-l",
            "rust",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert_ne!(
        output.status.code().unwrap_or(-1),
        2,
        "transform with hyphen-prefixed pattern was rejected as arg parse error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
