// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for the `outline` subcommand (v14 Tier 3, v0.1.12).
//!
//! Covers high-level structure extraction via tree-sitter:
//! functions, classes, structs, enums, traits.

mod common;

fn write_rust(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("lib.rs");
    std::fs::write(
        &p,
        "pub fn hello() -> i32 { 42 }\n\
         pub struct Point { x: i32, y: i32 }\n\
         pub enum Color { Red, Green, Blue }\n\
         pub trait Greet { fn greet(&self); }\n",
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
fn outline_rust_extracts_all_structural_items() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust(dir.path());

    let output = common::atomwrite()
        .args(["--workspace", workspace, "outline", f.to_str().unwrap()])
        .output()
        .expect("outline");

    assert!(
        output.status.success(),
        "outline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("function"), "missing function: {stdout}");
    assert!(stdout.contains("struct"), "missing struct: {stdout}");
    assert!(stdout.contains("enum"), "missing enum: {stdout}");
    assert!(stdout.contains("trait"), "missing trait: {stdout}");
}

#[test]
fn outline_python_extracts_function_and_class() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_python(dir.path());

    let output = common::atomwrite()
        .args(["--workspace", workspace, "outline", f.to_str().unwrap()])
        .output()
        .expect("outline");

    assert!(
        output.status.success(),
        "outline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("function"), "missing function: {stdout}");
    assert!(stdout.contains("class"), "missing class: {stdout}");
}

#[test]
fn outline_kind_filter_narrows_results() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust(dir.path());

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "outline",
            f.to_str().unwrap(),
            "--kind",
            "function_item",
        ])
        .output()
        .expect("outline");

    assert!(
        output.status.success(),
        "outline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("function_item"),
        "missing function_item: {stdout}"
    );
    assert!(
        !stdout.contains("struct_item"),
        "struct_item should be filtered: {stdout}"
    );
}

#[test]
fn outline_missing_file_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = dir.path().join("missing.rs");

    let output = common::atomwrite()
        .args(["--workspace", workspace, "outline", f.to_str().unwrap()])
        .output()
        .expect("outline");

    assert!(
        !output.status.success(),
        "outline on missing file should fail"
    );
}

#[test]
fn outline_emits_ndjson_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust(dir.path());

    let output = common::atomwrite()
        .args(["--workspace", workspace, "outline", f.to_str().unwrap()])
        .output()
        .expect("outline");

    assert!(
        output.status.success(),
        "outline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let item_count = events
        .iter()
        .filter(|e| e.get("type").and_then(|v| v.as_str()) == Some("outline_item"))
        .count();
    assert!(
        item_count >= 4,
        "expected at least 4 outline items, got {item_count}: {events:?}"
    );
    let summary = events
        .iter()
        .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("outline_summary"));
    assert!(summary.is_some(), "expected outline_summary event");
}
