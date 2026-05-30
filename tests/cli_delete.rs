// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn delete_removes_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "gone.txt", "bye\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--yes",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(!path.exists(), "file should be deleted");

    let events = common::parse_ndjson(&output.stdout);
    let deleted: Vec<_> = events.iter().filter(|e| e["type"] == "deleted").collect();
    assert_eq!(deleted.len(), 1);
    assert!(deleted[0]["checksum_before"].is_string());
}

#[test]
fn delete_dry_run_preserves_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "keep.txt", "stay\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--dry-run",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(path.exists(), "file should still exist");
}

#[test]
fn delete_with_backup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "bak.txt", "backup me\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--backup",
            "--yes",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(!path.exists());

    let backups: Vec<_> = std::fs::read_dir(dir.path())
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".bak."))
        .collect();
    assert_eq!(backups.len(), 1, "should create one backup");
}

#[test]
fn delete_not_found_exits_4() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--yes",
        ])
        .arg(dir.path().join("nonexistent.txt"))
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(4));
}
