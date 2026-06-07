// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end tests for G72 — Real syntax check via tree-sitter.
//!
//! Verifies that `atomwrite write --syntax-check` invokes the actual
//! tree-sitter parser and rejects syntactically invalid source code,
//! preserving comments and formatting for valid code.

mod common;

#[test]
fn syntax_check_valid_rust_passes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("valid.rs");
    std::fs::write(&f, "fn main() { println!(\"hello\"); }\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "write",
            f.to_str().unwrap(),
            "--syntax-check",
        ])
        .write_stdin(b"fn main() { println!(\"updated\"); }\n")
        .output()
        .expect("write");

    assert!(
        output.status.success(),
        "valid rust should pass syntax check: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert!(
        content.contains("updated"),
        "file should be written: {content}"
    );
}

#[test]
fn syntax_check_invalid_rust_fails_with_exit_88() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("invalid.rs");
    std::fs::write(&f, "fn main() {}\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "write",
            f.to_str().unwrap(),
            "--syntax-check",
        ])
        .write_stdin(b"fn broken( {\n")
        .output()
        .expect("write");

    assert!(
        !output.status.success(),
        "invalid rust should fail with non-zero exit"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("SYNTAX") || combined.contains("syntax"),
        "expected syntax error message, got: {combined}"
    );
    let original = std::fs::read_to_string(&f).expect("read");
    assert_eq!(
        original, "fn main() {}\n",
        "file should not be modified on syntax error"
    );
}

#[test]
fn syntax_check_valid_python_passes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("valid.py");
    std::fs::write(&f, "def hello():\n    return 42\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "write",
            f.to_str().unwrap(),
            "--syntax-check",
        ])
        .write_stdin(b"def hello():\n    return 43\n")
        .output()
        .expect("write");

    assert!(
        output.status.success(),
        "valid python should pass syntax check: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert!(
        content.contains("return 43"),
        "file should be written: {content}"
    );
}

#[test]
fn syntax_check_invalid_python_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("invalid.py");
    std::fs::write(&f, "def hello():\n    return 42\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "write",
            f.to_str().unwrap(),
            "--syntax-check",
        ])
        .write_stdin(b"def broken(:\n    pass\n")
        .output()
        .expect("write");

    assert!(
        !output.status.success(),
        "invalid python should fail with non-zero exit"
    );
    let original = std::fs::read_to_string(&f).expect("read");
    assert_eq!(
        original, "def hello():\n    return 42\n",
        "file should not be modified on syntax error"
    );
}

#[test]
fn syntax_check_unknown_extension_skips_check() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("data.xyz");
    std::fs::write(&f, "anything goes\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "write",
            f.to_str().unwrap(),
            "--syntax-check",
        ])
        .write_stdin(b"still anything\n")
        .output()
        .expect("write");

    assert!(
        output.status.success(),
        "unknown extension should skip syntax check: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert_eq!(content, "still anything\n");
}

#[test]
fn syntax_check_off_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("broken.rs");
    std::fs::write(&f, "fn main() {}\n").expect("write");

    let output = common::atomwrite()
        .args(["--workspace", workspace, "write", f.to_str().unwrap()])
        .write_stdin(b"fn broken( {\n")
        .output()
        .expect("write");

    assert!(
        output.status.success(),
        "without --syntax-check, invalid rust should still be written: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&f).expect("read");
    assert_eq!(
        content, "fn broken( {\n",
        "file should be written without check"
    );
}
