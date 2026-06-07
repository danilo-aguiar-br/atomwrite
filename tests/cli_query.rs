// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for the `query` subcommand (v14 Tier 3, v0.1.12).
//!
//! Covers tree-sitter REAL AST traversal via `tree-sitter-language-pack`.
//! Tests --kinds, --tree, --positions modes on Rust and Python.

mod common;

fn write_rust(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("sample.rs");
    std::fs::write(
        &p,
        "fn hello() -> i32 { 42 }\n\
         struct Point { x: i32, y: i32 }\n\
         impl Point { fn new() -> Self { Point { x: 0, y: 0 } } }\n",
    )
    .expect("write");
    p
}

fn write_python(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("sample.py");
    std::fs::write(
        &p,
        "def hello():\n    return 42\n\
         \n\
         class Greeter:\n    def __init__(self, name):\n        self.name = name\n",
    )
    .expect("write");
    p
}

#[test]
fn query_kinds_emits_ndjson_with_kind_counts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust(dir.path());

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "query",
            f.to_str().unwrap(),
            "--kinds",
        ])
        .output()
        .expect("query");

    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("function_item"),
        "missing function_item: {stdout}"
    );
    assert!(
        stdout.contains("struct_item"),
        "missing struct_item: {stdout}"
    );
    assert!(stdout.contains("impl_item"), "missing impl_item: {stdout}");
}

#[test]
fn query_tree_emits_named_nodes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust(dir.path());

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "query",
            f.to_str().unwrap(),
            "--tree",
        ])
        .output()
        .expect("query");

    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("function_item"),
        "expected function_item in tree: {stdout}"
    );
}

#[test]
fn query_python_kinds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_python(dir.path());

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "query",
            f.to_str().unwrap(),
            "--kinds",
        ])
        .output()
        .expect("query");

    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("function_definition"),
        "missing function_definition: {stdout}"
    );
    assert!(
        stdout.contains("class_definition"),
        "missing class_definition: {stdout}"
    );
}

#[test]
fn query_positions_adds_offsets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust(dir.path());

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "query",
            f.to_str().unwrap(),
            "--tree",
            "--positions",
        ])
        .output()
        .expect("query");

    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("start_line") || stdout.contains("byte_offset"),
        "expected position info: {stdout}"
    );
}

#[test]
fn query_missing_file_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("missing.rs");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "query",
            f.to_str().unwrap(),
            "--kinds",
        ])
        .output()
        .expect("query");

    assert!(
        !output.status.success(),
        "query on missing file should fail"
    );
}
