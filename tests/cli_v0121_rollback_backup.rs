// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.21 GAP-2026-013 Problema C: rollback --backup regression test.
//!
//! Before v0.1.21, `rollback` had `backup: false` hardcoded in
//! `src/commands/rollback.rs:108`. v0.1.21 exposes `--backup` and
//! `--keep-backup` on the `rollback` subcommand.

mod common;

/// Test 1: `rollback --backup` must create a .bak file of the
/// pre-rollback state. This preserves the current file content
/// before restoring the backup, enabling "undo the undo" workflows.
#[test]
fn rollback_creates_backup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("restore.txt");
    std::fs::write(&target, "version_one\n").expect("seed v1");
    let ws = dir.path().to_str().unwrap();

    // Create a backup of v1.
    let output = common::atomwrite()
        .args(["--workspace", ws, "backup"])
        .arg(&target)
        .output()
        .expect("backup v1");
    assert!(
        output.status.success(),
        "backup of v1 failed: {:?}",
        output.status
    );

    // Modify to v2 (--no-backup to avoid interfering with the explicit backup above).
    let output = common::atomwrite()
        .args(["--workspace", ws, "write", "--no-backup"])
        .arg(&target)
        .write_stdin("version_two\n")
        .output()
        .expect("write v2");
    assert!(output.status.success(), "write v2 failed");

    // Rollback with --backup and --keep-backup to preserve v2 before
    // restoring v1. Without --keep-backup the .bak would be deleted
    // after the rollback succeeds (GAP-014 v2 default cleanup).
    let output = common::atomwrite()
        .args([
            "--workspace",
            ws,
            "rollback",
            "--backup",
            "--keep-backup",
            "--retention",
            "3",
        ])
        .arg(&target)
        .output()
        .expect("rollback");

    assert!(
        output.status.success(),
        "rollback --backup failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // File should be back to v1.
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "version_one\n", "rollback should restore v1");

    // A .bak file of the pre-rollback state (v2) must be preserved
    // because of --keep-backup (GAP-014 v2 default is to delete).
    let bak_entries: Vec<_> = std::fs::read_dir(dir.path())
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("restore.txt.bak."))
        })
        .collect();
    assert!(
        bak_entries.len() >= 1,
        "at least one .bak.* file must be preserved with --keep-backup, found {}",
        bak_entries.len()
    );

    // The most recent .bak file should contain the pre-rollback content (v2).
    let mut bak_names: Vec<_> = bak_entries.iter().map(|e| e.file_name()).collect();
    bak_names.sort();
    let newest_bak = dir.path().join(bak_names.last().unwrap());
    let bak_content = std::fs::read_to_string(&newest_bak).expect("read bak");
    assert_eq!(
        bak_content, "version_two\n",
        "newest backup must contain pre-rollback state"
    );
}
