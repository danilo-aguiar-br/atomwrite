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

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "x",
            "y",
            "z",
        ])
        .output()
        .expect("case");

    assert!(!output.status.success(), "odd subvert count should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");
    assert!(
        combined.contains("even") || combined.contains("odd"),
        "got: {combined}"
    );
}
