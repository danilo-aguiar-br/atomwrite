// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for the `case` subcommand (v14 Tier 3, v0.1.12).
//!
//! Verifies all 5 heck styles produce correct output:
//! snake_case, camelCase, PascalCase, kebab-case, SCREAMING_SNAKE_CASE.

mod common;

fn write_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, content).expect("write");
    p
}

#[test]
fn case_snake_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_file(dir.path(), "a.rs", "let HTTPRequest = 1;\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "HTTPRequest",
            "http_request",
            "--to",
            "snake",
        ])
        .output()
        .expect("case");

    assert!(
        output.status.success(),
        "case failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert!(
        content.contains("let http_request = 1;"),
        "snake_case: {content}"
    );
}

#[test]
fn case_camel_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_file(dir.path(), "a.rs", "let user_id = 1;\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "user_id",
            "userId",
            "--to",
            "camel",
        ])
        .output()
        .expect("case");

    assert!(
        output.status.success(),
        "case failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert!(content.contains("let userId = 1;"), "camelCase: {content}");
}

#[test]
fn case_pascal_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_file(dir.path(), "a.py", "user_id = 1\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "user_id",
            "userID",
            "--to",
            "pascal",
        ])
        .output()
        .expect("case");

    assert!(
        output.status.success(),
        "case failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert!(
        content.contains("UserId = 1") || content.contains("UserID = 1"),
        "PascalCase: {content}"
    );
}

#[test]
fn case_kebab_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_file(dir.path(), "a.txt", "user_id = 1\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "user_id",
            "userID",
            "--to",
            "kebab",
        ])
        .output()
        .expect("case");

    assert!(
        output.status.success(),
        "case failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert!(content.contains("user-id = 1"), "kebab-case: {content}");
}

#[test]
fn case_screaming_snake_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_file(dir.path(), "a.txt", "user_id = 1\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "user_id",
            "userID",
            "--to",
            "screaming-snake",
        ])
        .output()
        .expect("case");

    assert!(
        output.status.success(),
        "case failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert!(
        content.contains("USER_ID = 1") || content.contains("USERID = 1"),
        "SCREAMING_SNAKE: {content}"
    );
}

#[test]
fn case_dry_run_does_not_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_file(dir.path(), "a.rs", "let HTTPRequest = 1;\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "HTTPRequest",
            "http_request",
            "--to",
            "snake",
            "--dry-run",
        ])
        .output()
        .expect("case");

    assert!(
        output.status.success(),
        "case failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert_eq!(
        content, "let HTTPRequest = 1;\n",
        "dry-run should not modify"
    );
}

#[test]
fn case_odd_subvert_count_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_file(dir.path(), "a.rs", "x = 1\n");

    // With num_args=2, clap rejects a single value for --subvert at parse time.
    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "x",
        ])
        .output()
        .expect("case");

    assert!(
        !output.status.success(),
        "single subvert value should fail at parse"
    );
}

#[test]
fn case_multi_file_parallel_rewrites_all() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let mut paths = Vec::new();
    for i in 0..8 {
        paths.push(write_file(
            dir.path(),
            &format!("m_{i}.rs"),
            "let HTTPRequest = 1;\n",
        ));
    }

    let mut cmd = common::atomwrite();
    cmd.args(["--workspace", workspace, "--threads", "4", "case"]);
    for p in &paths {
        cmd.arg(p);
    }
    cmd.args(["--subvert", "HTTPRequest", "http_request", "--to", "snake"]);
    let output = cmd.output().expect("case multi");
    assert!(
        output.status.success(),
        "case multi failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for p in &paths {
        let content = std::fs::read_to_string(p).expect("read");
        assert!(
            content.contains("let http_request = 1;"),
            "expected rewrite in {}: {content}",
            p.display()
        );
    }

    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["files_modified"].as_u64().unwrap(), 8);
    assert_eq!(summary["identifiers_total"].as_u64().unwrap(), 8);
}
