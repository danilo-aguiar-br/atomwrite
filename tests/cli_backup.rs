// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn backup_creates_timestamped_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "test.txt", "hello");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "backup"])
        .arg(&file)
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    assert!(events.iter().any(|e| e["type"] == "backup"));

    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_str().unwrap_or("").contains(".bak."))
        .collect();
    assert!(!entries.is_empty(), "backup file should exist");
}

#[test]
fn backup_dry_run_does_not_create() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "test.txt", "hello");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "backup",
            "--dry-run",
        ])
        .arg(&file)
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_str().unwrap_or("").contains(".bak."))
        .collect();
    assert!(entries.is_empty(), "dry-run should not create backup");
}

#[test]
fn backup_not_found_exits_4() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "backup"])
        .arg(dir.path().join("nonexistent.txt"))
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(4));
}
