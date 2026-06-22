// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn scope_readonly_reports_matches() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(
        dir.path(),
        "test.rs",
        "fn hello() {\n    42\n}\n\nfn world() {\n    0\n}\n",
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

    let scoped = events.iter().find(|e| e["type"] == "scoped");
    assert!(
        scoped.is_some(),
        "read-only scope should emit scoped event, got: {:?}",
        events
    );

    let scoped = scoped.unwrap();
    assert!(
        scoped["scopes_matched"].as_u64().unwrap() >= 2,
        "should find at least 2 functions"
    );
    assert_eq!(scoped["action"], "none");

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert!(
        summary["files_matched"].as_u64().unwrap() >= 1,
        "files_matched should be >= 1 in read-only mode"
    );
}

#[test]
fn scope_readonly_pattern_reports_matches() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(
        dir.path(),
        "test.rs",
        "fn main() {\n    let x = foo().unwrap();\n    let y = bar().unwrap();\n}\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "scope",
            "--language",
            "rust",
            "--pattern",
            "$EXPR.unwrap()",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert!(
        summary["files_matched"].as_u64().unwrap() >= 1,
        "pattern $EXPR.unwrap() should match in read-only mode"
    );
}

#[test]
fn scope_readonly_does_not_modify_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let content = "fn hello() {\n    42\n}\n";
    let file = common::create_test_file(dir.path(), "test.rs", content);

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

    assert!(output.status.success());

    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, content, "read-only scope must not modify file");
}

#[test]
fn scope_delete_still_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(
        dir.path(),
        "test.rs",
        "fn hello() {\n    42\n}\n\nfn world() {\n    0\n}\n",
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
            "--delete",
            "--dry-run",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);

    let scoped = events
        .iter()
        .find(|e| e["type"] == "scoped")
        .expect("scoped event");
    assert!(scoped["bytes_after"].as_u64().unwrap() < scoped["bytes_before"].as_u64().unwrap());

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert!(summary["files_matched"].as_u64().unwrap() >= 1);
}
