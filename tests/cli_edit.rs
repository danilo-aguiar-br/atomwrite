// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn edit_old_new_replaces_exact_match() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "edit.txt", "fn old_name() {}\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "old_name",
            "--new",
            "new_name",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "edit");
    assert_eq!(events[0]["mode"], "exact");

    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("new_name"));
    assert!(!content.contains("old_name"));
}

#[test]
fn edit_old_not_found_exits_65() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "nofind.txt", "hello world\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "NONEXISTENT",
            "--new",
            "replacement",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
}

#[test]
fn edit_after_line_inserts_content() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "lines.txt", "line1\nline2\nline3\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--after-line",
            "2",
        ])
        .arg(&path)
        .write_stdin("inserted\n")
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read_to_string(&path).expect("read");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines[0], "line1");
    assert_eq!(lines[1], "line2");
    assert_eq!(lines[2], "inserted");
    assert_eq!(lines[3], "line3");
}

#[test]
fn edit_delete_range_removes_lines() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "del.txt", "a\nb\nc\nd\ne\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--delete-range",
            "2:4",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read_to_string(&path).expect("read");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines, vec!["a", "e"]);
}

#[test]
fn edit_after_match_inserts_after_marker() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(
        dir.path(),
        "marker.txt",
        "use std::io;\nuse std::fs;\n\nfn main() {}\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--after-match",
            "use std::fs;",
        ])
        .arg(&path)
        .write_stdin("use std::path::Path;")
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("use std::fs;\nuse std::path::Path;\n"));
}
