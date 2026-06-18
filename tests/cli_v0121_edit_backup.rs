// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.21 GAP-2026-013 Problema C: edit --backup regression tests.
//!
//! Before v0.1.21, `edit` had `backup: false` hardcoded in
//! `src/commands/edit.rs:139` and `:393`. v0.1.21 exposes `--backup`,
//! `--retention`, and `--keep-backup` on the `edit` subcommand, matching
//! the API parity of `write` and `replace`.

mod common;

/// Test 1: `edit --backup --old X --new Y` must create a .bak file
/// with the pre-edit content. The backup preserves the original
/// state for forensics and rollback.
#[test]
fn edit_with_backup_creates_bak() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("auditable.rs");
    std::fs::write(&target, "fn original() {}\n").expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--backup",
            "--keep-backup",
            "--retention",
            "3",
            "--old",
            "original",
            "--new",
            "modified",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --backup failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The edit was applied.
    let content = std::fs::read_to_string(&target).expect("read");
    assert!(content.contains("fn modified()"));
    assert!(!content.contains("original"));

    // A .bak file was created with the pre-edit content and
    // preserved because of --keep-backup (GAP-014 v2 default
    // is to delete the backup after success).
    let bak_entries: Vec<_> = std::fs::read_dir(dir.path())
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("auditable.rs.bak."))
        })
        .collect();
    assert_eq!(
        bak_entries.len(),
        1,
        "exactly one .bak.* file must be preserved with --keep-backup"
    );

    // The .bak file must contain the pre-edit content.
    let bak_path = dir.path().join(bak_entries[0].file_name());
    let bak_content = std::fs::read_to_string(&bak_path).expect("read bak");
    assert_eq!(bak_content, "fn original() {}\n");
}

/// Test 2: `edit --old X --new Y` WITHOUT --backup must NOT create
/// any .bak file. This is the default behavior for performance and
/// minimal-overhead edits.
#[test]
fn edit_without_backup_no_bak() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("fast.rs");
    std::fs::write(&target, "fn fast() {}\n").expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "fast",
            "--new",
            "quick",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The edit was applied.
    let content = std::fs::read_to_string(&target).expect("read");
    assert!(content.contains("fn quick()"));
    assert!(!content.contains("fast"));

    // No .bak file was created.
    let bak_entries: Vec<_> = std::fs::read_dir(dir.path())
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("fast.rs.bak."))
        })
        .collect();
    assert!(
        bak_entries.is_empty(),
        "no .bak.* files should be created without --backup, found: {:?}",
        bak_entries
            .iter()
            .map(|e| e.file_name())
            .collect::<Vec<_>>()
    );
}
