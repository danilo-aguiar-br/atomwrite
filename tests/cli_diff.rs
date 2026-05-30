// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn diff_identical_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = common::create_test_file(dir.path(), "a.txt", "same\n");
    let b = common::create_test_file(dir.path(), "b.txt", "same\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            "--stat",
        ])
        .arg(&a)
        .arg(&b)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["identical"], true);
    assert_eq!(events[0]["insertions"], 0);
    assert_eq!(events[0]["deletions"], 0);
}

#[test]
fn diff_different_files_shows_changes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = common::create_test_file(dir.path(), "old.txt", "line1\nline2\nline3\n");
    let b = common::create_test_file(dir.path(), "new.txt", "line1\nchanged\nline3\nline4\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            "--stat",
        ])
        .arg(&a)
        .arg(&b)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["identical"], false);
    assert!(events[0]["insertions"].as_u64().unwrap() > 0);
    assert!(events[0]["deletions"].as_u64().unwrap() > 0);
    assert!(events[0]["similarity_ratio"].as_f64().unwrap() > 0.0);
    assert!(events[0]["similarity_ratio"].as_f64().unwrap() < 1.0);
}

#[test]
fn diff_unified_format() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = common::create_test_file(dir.path(), "u1.txt", "aaa\nbbb\n");
    let b = common::create_test_file(dir.path(), "u2.txt", "aaa\nccc\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            "--unified",
        ])
        .arg(&a)
        .arg(&b)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["format"], "unified");
    let content = events[0]["content"].as_str().expect("content string");
    assert!(content.contains("---"));
    assert!(content.contains("+++"));
    assert!(content.contains("@@"));
}

#[test]
fn diff_default_emits_changes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = common::create_test_file(dir.path(), "d1.txt", "old\n");
    let b = common::create_test_file(dir.path(), "d2.txt", "new\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "diff"])
        .arg(&a)
        .arg(&b)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let changes: Vec<_> = events.iter().filter(|e| e["type"] == "change").collect();
    assert!(!changes.is_empty());
    let tags: Vec<_> = changes.iter().map(|c| c["tag"].as_str().unwrap()).collect();
    assert!(tags.contains(&"insert") || tags.contains(&"delete"));
}
