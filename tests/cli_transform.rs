// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn transform_removes_dbg_macro() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "test.rs", "fn main() { dbg!(42); }\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "transform",
            "-p",
            "dbg!($$$A)",
            "-r",
            "$$$A",
            "-l",
            "rust",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let transformed: Vec<_> = events
        .iter()
        .filter(|e| e["type"] == "transformed")
        .collect();
    assert_eq!(transformed.len(), 1);
    assert_eq!(transformed[0]["replacements"], 1);

    let content = std::fs::read_to_string(dir.path().join("test.rs")).expect("read");
    assert!(!content.contains("dbg!"));
    assert!(content.contains("42"));
}

#[test]
fn transform_dry_run_does_not_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let original = "fn main() { dbg!(42); }\n";
    common::create_test_file(dir.path(), "keep.rs", original);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "transform",
            "-p",
            "dbg!($$$A)",
            "-r",
            "$$$A",
            "-l",
            "rust",
            "--dry-run",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read_to_string(dir.path().join("keep.rs")).expect("read");
    assert_eq!(content, original);
}

#[test]
fn transform_no_match_produces_summary_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "clean.rs", "fn main() { let x = 1; }\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "transform",
            "-p",
            "dbg!($$$A)",
            "-r",
            "$$$A",
            "-l",
            "rust",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    // GAP-145: transform with zero matches now returns exit 1 (NO_MATCHES)
    assert_eq!(
        output.status.code(),
        Some(1),
        "transform with zero matches should return exit 1 (NO_MATCHES): {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);

    let transformed: Vec<_> = events
        .iter()
        .filter(|e| e["type"] == "transformed")
        .collect();
    assert!(transformed.is_empty());

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["files_modified"], 0);
}

#[test]
fn transform_invalid_language_exits_65() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "test.txt", "hello\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "transform",
            "-p",
            "hello",
            "-r",
            "world",
            "-l",
            "nonexistent_language",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
}

#[test]
fn transform_multiple_replacements_in_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(
        dir.path(),
        "multi.rs",
        "fn a() { dbg!(1); dbg!(2); dbg!(3); }\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "transform",
            "-p",
            "dbg!($$$A)",
            "-r",
            "$$$A",
            "-l",
            "rust",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let transformed = events
        .iter()
        .find(|e| e["type"] == "transformed")
        .expect("transformed");
    assert_eq!(transformed["replacements"], 3);

    let content = std::fs::read_to_string(dir.path().join("multi.rs")).expect("read");
    assert!(!content.contains("dbg!"));
}

#[test]
fn transform_language_flag_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "test.rs", "fn main() { dbg!(42); }\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "transform",
            "-p",
            "dbg!($$$A)",
            "-r",
            "$$$A",
            "--language",
            "rust",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let content = std::fs::read_to_string(dir.path().join("test.rs")).expect("read");
    assert!(!content.contains("dbg!"));
}
