// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

use serde_json::Value;

#[test]
fn diff_relative_paths_resolve_against_workspace() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.txt", "line1\nline2\n");
    common::create_test_file(dir.path(), "b.txt", "line1\nchanged\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            "--stat",
            "a.txt",
            "b.txt",
        ])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "diff with relative paths should succeed, got exit {:?}\nstdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "diff");
    assert_eq!(events[0]["identical"], false);
}

#[test]
fn diff_absolute_paths_inside_workspace_work() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = common::create_test_file(dir.path(), "abs_a.txt", "hello\n");
    let b = common::create_test_file(dir.path(), "abs_b.txt", "world\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            "--stat",
        ])
        .arg(&a)
        .arg(&b)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["identical"], false);
}

#[test]
fn diff_path_escape_returns_workspace_jail() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "ok.txt", "content\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            "--stat",
            "../escape.txt",
            "ok.txt",
        ])
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(126),
        "path escape should return WORKSPACE_JAIL (exit 126)"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(json["code"], "WORKSPACE_JAIL");
}
