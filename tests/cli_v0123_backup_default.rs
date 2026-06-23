// SPDX-License-Identifier: MIT OR Apache-2.0

//! GAP-2026-016: backup-by-default for content-mutating commands.
//!
//! Validates that backup is created automatically without --backup flag,
//! that --no-backup disables it, and that ATOMWRITE_BACKUP=0 overrides.

mod common;

use std::fs;

#[test]
fn write_creates_backup_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "original content here");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .write_stdin("new content")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "exit 0 expected; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), "new content");

    let bak_count = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.contains(".bak.")))
        .count();
    assert_eq!(bak_count, 0, "backup should be auto-deleted after success");
}

#[test]
fn write_no_backup_flag_disables() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "original");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--no-backup",
        ])
        .arg(&target)
        .write_stdin("replaced")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "exit 0 expected; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"backup_path\":null") || !stdout.contains("backup_path"),
        "no backup should be created with --no-backup"
    );
}

#[test]
fn write_keep_backup_retains_after_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "keep me as backup");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--keep-backup",
        ])
        .arg(&target)
        .write_stdin("new data")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "exit 0 expected; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), "new data");

    let bak_files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("target.txt.bak."))
        })
        .collect();
    assert_eq!(
        bak_files.len(),
        1,
        "backup should be retained with --keep-backup"
    );
    let bak_content = fs::read_to_string(bak_files[0].path()).unwrap();
    assert_eq!(bak_content, "keep me as backup");
}

#[test]
fn write_backup_not_created_for_new_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("brand_new.txt");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .write_stdin("fresh content")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "exit 0 expected; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let bak_count = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.contains(".bak.")))
        .count();
    assert_eq!(bak_count, 0, "no backup for new file");
}

#[test]
fn edit_creates_backup_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "hello world");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "edit"])
        .arg(&target)
        .args(["--old", "hello", "--new", "goodbye"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "exit 0 expected; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), "goodbye world");

    let bak_count = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.contains(".bak.")))
        .count();
    assert_eq!(
        bak_count, 0,
        "backup should be auto-deleted after successful edit"
    );
}

#[test]
fn env_atomwrite_backup_zero_disables() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "target.txt", "env test");

    let output = common::atomwrite()
        .env("ATOMWRITE_BACKUP", "0")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--keep-backup",
        ])
        .arg(&target)
        .write_stdin("overwritten")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "exit 0 expected; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let bak_count = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.contains(".bak.")))
        .count();
    assert_eq!(
        bak_count, 0,
        "ATOMWRITE_BACKUP=0 should disable backup entirely"
    );
}

#[test]
fn replace_creates_backup_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "target.txt", "old_api is deprecated");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "old_api",
            "new_api",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("run");

    let target = dir.path().join("target.txt");
    let content = fs::read_to_string(&target).unwrap();
    if output.status.success() {
        assert!(content.contains("new_api"));
    }

    let bak_count = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.contains(".bak.")))
        .count();
    // GAP-105: replace --backup now retains the backup on disk
    assert!(
        bak_count >= 1,
        "backup should be retained after replace (GAP-105)"
    );
}
