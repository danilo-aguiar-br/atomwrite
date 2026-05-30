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
            "--lang",
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
            "--lang",
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
            "--lang",
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
            "--lang",
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
