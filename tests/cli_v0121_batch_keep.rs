// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.21 GAP-2026-013 Problema C: batch --keep-backup regression test.
//!
//! `batch` processes an NDJSON stream of operations. v0.1.21 adds
//! `--keep-backup` at the batch level. When set, every `write` op
//! in the batch that targets an existing file creates a .bak that
//! persists after the batch completes.

mod common;

/// Test 1: A batch with two `write` operations + `--keep-backup` must
/// create .bak files for any pre-existing targets. The .bak files
/// persist after the batch completes (not deleted by default cleanup).
#[test]
fn batch_keep_backup_propagates() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target_a = dir.path().join("a.txt");
    let target_b = dir.path().join("b.txt");

    // Seed both files so --backup has something to back up.
    std::fs::write(&target_a, "original_a\n").expect("seed a");
    std::fs::write(&target_b, "original_b\n").expect("seed b");

    // Build the NDJSON manifest for the batch.
    let manifest = common::manifest(&[
        serde_json::json!({
            "op": "write",
            "target": target_a.to_string_lossy(),
            "content": "new_a\n",
        }),
        serde_json::json!({
            "op": "write",
            "target": target_b.to_string_lossy(),
            "content": "new_b\n",
        }),
    ]);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "batch",
            "--keep-backup",
        ])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "batch --keep-backup failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Both writes were applied.
    let content_a = std::fs::read_to_string(&target_a).expect("read a");
    let content_b = std::fs::read_to_string(&target_b).expect("read b");
    assert_eq!(content_a, "new_a\n");
    assert_eq!(content_b, "new_b\n");

    // Each target should have a .bak file with pre-batch content.
    let bak_a: Vec<_> = std::fs::read_dir(dir.path())
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("a.txt.bak."))
        })
        .collect();
    let bak_b: Vec<_> = std::fs::read_dir(dir.path())
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("b.txt.bak."))
        })
        .collect();

    assert_eq!(
        bak_a.len(),
        1,
        "exactly one .bak.* for a.txt must be preserved"
    );
    assert_eq!(
        bak_b.len(),
        1,
        "exactly one .bak.* for b.txt must be preserved"
    );

    // Verify .bak content matches pre-batch state.
    let bak_a_content =
        std::fs::read_to_string(dir.path().join(bak_a[0].file_name())).expect("read bak a");
    let bak_b_content =
        std::fs::read_to_string(dir.path().join(bak_b[0].file_name())).expect("read bak b");
    assert_eq!(bak_a_content, "original_a\n");
    assert_eq!(bak_b_content, "original_b\n");
}
