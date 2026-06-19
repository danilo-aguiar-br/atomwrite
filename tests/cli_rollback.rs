// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn rollback_restores_latest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "test.txt", "original content");
    let ws = dir.path().to_str().unwrap();

    // Create backup
    let output = common::atomwrite()
        .args(["--workspace", ws, "backup"])
        .arg(&file)
        .output()
        .expect("run");
    assert!(
        output.status.success(),
        "backup failed: {:?}",
        output.status
    );

    // Modify file via atomwrite (--no-backup to avoid overwriting the explicit backup)
    let output = common::atomwrite()
        .args(["--workspace", ws, "write", "--no-backup"])
        .arg(&file)
        .write_stdin("modified content")
        .output()
        .expect("run");
    assert!(output.status.success(), "write failed: {:?}", output.status);

    // Rollback
    let output = common::atomwrite()
        .args(["--workspace", ws, "rollback"])
        .arg(&file)
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    assert!(events.iter().any(|e| e["type"] == "rollback"));

    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(content, "original content");
}

#[test]
fn rollback_dry_run_does_not_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "test.txt", "original");
    let ws = dir.path().to_str().unwrap();

    let output = common::atomwrite()
        .args(["--workspace", ws, "backup"])
        .arg(&file)
        .output()
        .expect("run");
    assert!(
        output.status.success(),
        "backup failed: {:?}",
        output.status
    );

    let output = common::atomwrite()
        .args(["--workspace", ws, "write", "--no-backup"])
        .arg(&file)
        .write_stdin("changed")
        .output()
        .expect("run");
    assert!(output.status.success(), "write failed: {:?}", output.status);

    let output = common::atomwrite()
        .args(["--workspace", ws, "rollback", "--dry-run"])
        .arg(&file)
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(content, "changed", "dry-run should not modify");
}

#[test]
fn rollback_no_backup_exits_4() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "test.txt", "hello");
    let ws = dir.path().to_str().unwrap();

    let output = common::atomwrite()
        .args(["--workspace", ws, "rollback"])
        .arg(&file)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(4));
}
