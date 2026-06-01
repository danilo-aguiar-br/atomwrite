// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn scope_rust_find_functions() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(
        dir.path(),
        "test.rs",
        "fn hello() {\n    println!(\"hi\");\n}\n\nfn world() {\n    println!(\"world\");\n}\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "scope",
            "--language",
            "rust",
            "--query",
            "fn",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    assert!(events.iter().any(|e| e["type"] == "summary"));
}

#[test]
fn scope_unknown_language_exits_65() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "test.txt", "content");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "scope",
            "--language",
            "nonexistent_lang",
            "--query",
            "fn",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
}

#[test]
fn scope_custom_pattern() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "test.rs", "fn main() {\n    let x = 42;\n}\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "scope",
            "--language",
            "rust",
            "--pattern",
            "let $NAME = $$$EXPR",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);
}

#[test]
fn scope_dry_run_does_not_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "test.rs", "// comment\nfn main() {}\n");
    let ws = dir.path().to_str().unwrap();

    let output = common::atomwrite()
        .args([
            "--workspace",
            ws,
            "scope",
            "--language",
            "rust",
            "--query",
            "comments",
            "--delete",
            "--dry-run",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("// comment"), "dry-run should not modify");
}

#[test]
fn scope_rust_find_functions_nonzero() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(
        dir.path(),
        "test.rs",
        "fn hello() {\n    42\n}\n\nfn world() {\n    0\n}\n",
    );

    // --delete --dry-run triggers matching without modifying; produces scoped events
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "scope",
            "--language",
            "rust",
            "--query",
            "fn",
            "--delete",
            "--dry-run",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary event");
    assert!(
        summary["files_matched"].as_u64().unwrap() >= 1,
        "expected at least one matched file"
    );

    assert!(
        events.iter().any(|e| e["type"] == "scoped"),
        "expected at least one scoped event"
    );
}

#[test]
fn scope_language_flag_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "test.rs", "fn main() {}\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "scope",
            "--language",
            "rust",
            "--query",
            "fn",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    assert!(
        events.iter().any(|e| e["type"] == "summary"),
        "expected summary event"
    );
}

#[test]
fn scope_filters_by_extension() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.rs", "fn hello() {}\n");
    common::create_test_file(dir.path(), "b.py", "def hello():\n    pass\n");
    common::create_test_file(dir.path(), "c.txt", "just text\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "scope",
            "--language",
            "rust",
            "--query",
            "fn",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary event");
    assert_eq!(
        summary["files_visited"].as_u64().unwrap(),
        1,
        "TypesBuilder should filter to only the .rs file"
    );
}
