// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for v0.1.19 G122: real S-expression matching in
//! the `query` subcommand.
//!
//! Pre-v0.1.19, `--query <PATTERN>` always compared the entire pattern
//! string against `node.kind()` and never matched S-expressions. These
//! tests exercise the new auto-detected routing: S-expression patterns
//! go through `tree_sitter::Query::new`, kind-filter patterns keep the
//! legacy literal-compare path.

mod common;

fn write_rust_sample(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("sample.rs");
    std::fs::write(
        &p,
        "fn main() -> i32 { 42 }\n\
         struct Point { x: i32, y: i32 }\n\
         impl Point { fn new() -> Self { Point { x: 0, y: 0 } } }\n",
    )
    .expect("write sample.rs");
    p
}

fn read_query_matches(output: &std::process::Output) -> Vec<serde_json::Value> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let v: serde_json::Value = serde_json::from_str(trimmed).ok()?;
            if v.get("type").and_then(|t| t.as_str()) == Some("query_match") {
                Some(v)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn sexpr_function_item_returns_main() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust_sample(dir.path());

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "query",
            f.to_str().unwrap(),
            "--query",
            "(function_item name: (identifier) @name)",
        ])
        .output()
        .expect("query");

    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let matches = read_query_matches(&output);
    assert!(
        !matches.is_empty(),
        "expected at least one query_match for the S-expression pattern; stdout=\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    // The captured name "main" must be present.
    let captured_names: Vec<&str> = matches
        .iter()
        .filter_map(|m| m.get("capture_name").and_then(|c| c.as_str()))
        .collect();
    assert!(
        captured_names.contains(&"name"),
        "expected a capture named 'name' in: {captured_names:?}"
    );
}

#[test]
fn sexpr_with_capture_returns_captured_text() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust_sample(dir.path());

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "query",
            f.to_str().unwrap(),
            "--query",
            "(function_item name: (identifier) @name)",
            "--positions",
        ])
        .output()
        .expect("query");

    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let matches = read_query_matches(&output);
    let captured_texts: Vec<&str> = matches
        .iter()
        .filter(|m| m.get("capture_name").and_then(|c| c.as_str()) == Some("name"))
        .filter_map(|m| m.get("text").and_then(|t| t.as_str()))
        .collect();
    assert!(
        captured_texts.contains(&"main"),
        "expected captured text 'main' in: {captured_texts:?}"
    );
    // The first capture with --positions must include start_byte / end_byte.
    let first = matches
        .iter()
        .find(|m| m.get("capture_name").and_then(|c| c.as_str()) == Some("name"))
        .expect("at least one name capture");
    assert!(
        first.get("start_byte").is_some() && first.get("end_byte").is_some(),
        "expected byte offsets in capture event: {first}"
    );
}

#[test]
fn sexpr_struct_item() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust_sample(dir.path());

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "query",
            f.to_str().unwrap(),
            "--query",
            "(struct_item name: (type_identifier) @name)",
        ])
        .output()
        .expect("query");

    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let matches = read_query_matches(&output);
    let captured_texts: Vec<&str> = matches
        .iter()
        .filter_map(|m| m.get("text").and_then(|t| t.as_str()))
        .collect();
    assert!(
        captured_texts.contains(&"Point"),
        "expected captured struct name 'Point' in: {captured_texts:?}"
    );
}

#[test]
fn kind_filter_still_works() {
    // Plain kind names (no parens, no @) must keep the v0.1.12 path
    // that compares node.kind() as a literal string. The S-expression
    // classifier must NOT route `function_item` to tree_sitter::Query.
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let f = write_rust_sample(dir.path());

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "query",
            f.to_str().unwrap(),
            "--query",
            "function_item",
        ])
        .output()
        .expect("query");

    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let matches = read_query_matches(&output);
    let kinds: Vec<&str> = matches
        .iter()
        .filter_map(|m| m.get("kind").and_then(|k| k.as_str()))
        .collect();
    assert!(
        !kinds.is_empty(),
        "kind filter must return at least one function_item match"
    );
    assert!(
        kinds.iter().all(|k| *k == "function_item"),
        "kind filter must only return function_item kinds, got: {kinds:?}"
    );
    // The legacy kind filter must NOT include a capture_name field.
    assert!(
        matches.iter().all(|m| m.get("capture_name").is_none()),
        "kind filter matches must not include capture_name"
    );
}
