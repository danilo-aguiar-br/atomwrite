// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.21 GAP-2026-013 Problema C: apply --keep-backup regression test.
//!
//! `apply` uses a patch-based flow. When --backup is set, a .bak file
//! is created with the pre-patch content. With --keep-backup, the
//! .bak persists after success. Without --keep-backup, the .bak
//! is deleted by the default cleanup.

mod common;

/// Test 1: `apply` with a unified diff patch + --backup + --keep-backup
/// must preserve the .bak file after a successful patch. The backup
/// contains the pre-patch file content.
#[test]
fn apply_keep_backup_preserved() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("patched.txt");
    let original = "line one\nline two\nline three\n";
    std::fs::write(&target, original).expect("seed");
    let ws = dir.path().to_str().unwrap();

    // Unified diff: change "line two" to "LINE TWO".
    let patch = "--- a/patched.txt\n+++ b/patched.txt\n@@ -1,3 +1,3 @@\n line one\n-line two\n+LINE TWO\n line three\n";

    let output = common::atomwrite()
        .args([
            "--workspace",
            ws,
            "apply",
            "--format",
            "unified",
            "--backup",
            "--keep-backup",
            "--retention",
            "3",
        ])
        .arg(&target)
        .write_stdin(patch)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "apply with --keep-backup failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The patch was applied.
    let content = std::fs::read_to_string(&target).expect("read");
    assert!(content.contains("LINE TWO"));
    assert!(!content.contains("line two"));

    // The .bak file must exist with pre-patch content.
    let bak_entries: Vec<_> = std::fs::read_dir(dir.path())
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("patched.txt.bak."))
        })
        .collect();
    assert_eq!(
        bak_entries.len(),
        1,
        "exactly one .bak.* file must be preserved with --keep-backup"
    );

    let bak_path = dir.path().join(bak_entries[0].file_name());
    let bak_content = std::fs::read_to_string(&bak_path).expect("read bak");
    assert_eq!(bak_content, original, "backup must contain pre-patch state");
}
