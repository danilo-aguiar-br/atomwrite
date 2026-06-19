// SPDX-License-Identifier: MIT OR Apache-2.0

//! GAP-2026-018: --old-file and --new-file flags for edit command.
//! Reads match/replacement content from files instead of CLI arguments,
//! bypassing kernel ARG_MAX limit (~131 KB).

mod common;

use std::fs;

#[test]
fn edit_old_file_new_file_replaces_content() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "hello world");
    common::create_test_file(dir.path(), "old.txt", "hello");
    common::create_test_file(dir.path(), "new.txt", "goodbye");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old-file",
            "old.txt",
            "--new-file",
            "new.txt",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --old-file --new-file failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&target).expect("read");
    assert_eq!(content, "goodbye world");

    let events = common::parse_ndjson(&output.stdout);
    assert!(!events.is_empty());
    if let Some(pr) = events[0].get("pair_results") {
        assert!(pr.is_array());
        if let Some(source) = events[0].get("source") {
            assert_eq!(source, "file");
        }
    }
}

#[test]
fn edit_old_file_rejects_outside_workspace() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "test");
    common::create_test_file(dir.path(), "new.txt", "replacement");

    let outside_name = format!(
        "old_outside_{}.txt",
        dir.path().file_name().unwrap().to_str().unwrap()
    );
    let outside_path = std::path::Path::new("/tmp").join(&outside_name);
    fs::write(&outside_path, "test").expect("write outside file");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old-file",
            outside_path.to_str().unwrap(),
            "--new-file",
            "new.txt",
        ])
        .arg(&target)
        .output()
        .expect("run");

    let _ = fs::remove_file(&outside_path);

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        126,
        "expected exit 126 (WORKSPACE_JAIL), got {exit_code}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn edit_old_file_and_old_conflict() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "text");
    common::create_test_file(dir.path(), "old.txt", "text");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "text",
            "--old-file",
            "old.txt",
            "--new",
            "replacement",
        ])
        .arg(&target)
        .output()
        .expect("run");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        2,
        "expected exit 2 (ARGUMENT_PARSE_ERROR), got {exit_code}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn edit_old_file_large_content() {
    let dir = tempfile::tempdir().expect("tempdir");
    let large = "A".repeat(200_000);
    let large_with_suffix = format!("{large} suffix");

    let target = common::create_test_file(dir.path(), "target.txt", &large_with_suffix);
    common::create_test_file(dir.path(), "old.txt", &large);

    let replacement = "B".repeat(200_000);
    common::create_test_file(dir.path(), "new.txt", &replacement);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old-file",
            "old.txt",
            "--new-file",
            "new.txt",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --old-file large content failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&target).expect("read");
    assert_eq!(content.len(), 200_000 + " suffix".len());
    assert!(content.starts_with(&"B".repeat(100)));
    assert!(content.ends_with(" suffix"));
}

#[test]
fn edit_old_file_new_file_multi_pair() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "alpha beta");
    common::create_test_file(dir.path(), "a.txt", "alpha");
    common::create_test_file(dir.path(), "a2.txt", "ALPHA");
    common::create_test_file(dir.path(), "b.txt", "beta");
    common::create_test_file(dir.path(), "b2.txt", "BETA");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old-file",
            "a.txt",
            "--new-file",
            "a2.txt",
            "--old-file",
            "b.txt",
            "--new-file",
            "b2.txt",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --old-file --new-file multi-pair failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&target).expect("read");
    assert_eq!(content, "ALPHA BETA");

    let events = common::parse_ndjson(&output.stdout);
    assert!(!events.is_empty());
    if let Some(pairs_total) = events[0].get("pairs_total") {
        assert_eq!(pairs_total.as_i64().unwrap(), 2);
    }
}

#[test]
fn edit_old_file_strips_trailing_newline() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "hello world");
    // Write files WITH trailing newlines (simulating echo "hello" > old.txt)
    fs::write(dir.path().join("old.txt"), "hello\n").expect("write old");
    fs::write(dir.path().join("new.txt"), "goodbye\n").expect("write new");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old-file",
            "old.txt",
            "--new-file",
            "new.txt",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --old-file with trailing newline failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&target).expect("read");
    assert_eq!(content, "goodbye world");
}

#[test]
fn edit_rejects_old_argv_with_new_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "hello world");
    common::create_test_file(dir.path(), "new.txt", "goodbye");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "hello",
            "--new-file",
            "new.txt",
        ])
        .arg(&target)
        .output()
        .expect("run");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        65,
        "expected exit 65 (INVALID_INPUT) for mixing --old with --new-file, got {exit_code}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cannot mix"),
        "error should mention 'cannot mix': {stdout}"
    );
}

#[test]
fn edit_rejects_old_file_with_new_argv() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "hello world");
    common::create_test_file(dir.path(), "old.txt", "hello");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old-file",
            "old.txt",
            "--new",
            "goodbye",
        ])
        .arg(&target)
        .output()
        .expect("run");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        65,
        "expected exit 65 (INVALID_INPUT) for mixing --old-file with --new, got {exit_code}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cannot mix"),
        "error should mention 'cannot mix': {stdout}"
    );
}
