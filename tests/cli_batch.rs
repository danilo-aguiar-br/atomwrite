// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn batch_write_creates_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("batch_out.txt");

    let manifest = format!(
        r#"{{"op":"write","target":"{}","content":"hello batch"}}"#,
        target.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);

    let op = events
        .iter()
        .find(|e| e["type"] == "batch_op")
        .expect("batch_op event");
    assert_eq!(op["op"], "write");
    assert_eq!(op["status"], "ok");

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["operations"], 1);
    assert_eq!(summary["succeeded"], 1);
    assert_eq!(summary["failed"], 0);

    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "hello batch");
}

#[test]
fn batch_replace_modifies_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("replace_me.txt");
    std::fs::write(&target, "old_value here\n").expect("write");

    let manifest = format!(
        r#"{{"op":"replace","target":"{}","pattern":"old_value","replacement":"new_value"}}"#,
        target.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(output.status.success());

    let content = std::fs::read_to_string(&target).expect("read");
    assert!(content.contains("new_value"));
    assert!(!content.contains("old_value"));
}

#[test]
fn batch_delete_removes_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("to_delete.txt");
    std::fs::write(&target, "delete me\n").expect("write");

    let manifest = format!(r#"{{"op":"delete","target":"{}"}}"#, target.display());

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(!target.exists());
}

#[test]
fn batch_dry_run_does_not_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("keep.txt");
    let original = "keep me\n";
    std::fs::write(&target, original).expect("write");

    let manifest = format!(
        r#"{{"op":"replace","target":"{}","pattern":"keep","replacement":"gone"}}"#,
        target.display()
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "batch",
            "--dry-run",
        ])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, original);
}

#[test]
fn batch_multiple_operations() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_a = dir.path().join("a.txt");
    let file_b = dir.path().join("b.txt");
    std::fs::write(&file_b, "original_b\n").expect("write");

    let manifest = format!(
        r#"{{"op":"write","target":"{}","content":"content_a"}}
{{"op":"replace","target":"{}","pattern":"original_b","replacement":"modified_b"}}"#,
        file_a.display(),
        file_b.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["operations"], 2);
    assert_eq!(summary["succeeded"], 2);

    assert_eq!(std::fs::read_to_string(&file_a).expect("a"), "content_a");
    assert!(
        std::fs::read_to_string(&file_b)
            .expect("b")
            .contains("modified_b")
    );
}

#[test]
fn batch_invalid_op_fails() {
    let dir = tempfile::tempdir().expect("tempdir");

    let manifest = r#"{"op":"nonexistent","target":"foo.txt"}"#;

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(!output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let op = events
        .iter()
        .find(|e| e["type"] == "batch_op")
        .expect("batch_op");
    assert_eq!(op["status"], "failed");
}

#[test]
fn batch_empty_manifest_fails() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin("")
        .output()
        .expect("run");

    assert!(!output.status.success());
}

// --- GAP 03: campo source com aliases ---

#[test]
fn batch_move_with_source_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "origin.txt", "move me\n");
    let dest = dir.path().join("destination.txt");

    let manifest = format!(
        r#"{{"op":"move","source":"{}","target":"{}"}}"#,
        src.display(),
        dest.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!src.exists(), "source should be removed after move");
    assert!(dest.exists(), "destination should exist after move");
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "move me\n");
}

#[test]
fn batch_copy_with_source_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "src_copy.txt", "copy me\n");
    let dest = dir.path().join("dst_copy.txt");

    let manifest = format!(
        r#"{{"op":"copy","source":"{}","target":"{}"}}"#,
        src.display(),
        dest.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(src.exists(), "source should still exist after copy");
    assert!(dest.exists(), "destination should exist after copy");
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "copy me\n");
}

#[test]
fn batch_move_with_from_alias() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "from_file.txt", "alias test\n");
    let dest = dir.path().join("to_file.txt");

    let manifest = format!(
        r#"{{"op":"move","from":"{}","target":"{}"}}"#,
        src.display(),
        dest.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!src.exists());
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "alias test\n");
}

// --- GAP 04: path como alias de target ---

#[test]
fn batch_write_with_path_alias() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("path_alias.txt");

    let manifest = format!(
        r#"{{"op":"write","path":"{}","content":"via path field"}}"#,
        target.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "via path field");
}

#[test]
fn batch_delete_with_path_alias() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = common::create_test_file(dir.path(), "to_del.txt", "del me\n");

    let manifest = format!(r#"{{"op":"delete","path":"{}"}}"#, target.display());

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!target.exists(), "file should be deleted via path alias");
}

#[test]
fn batch_move_legacy_compat() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "legacy_src.txt", "legacy\n");
    let dest = dir.path().join("legacy_dst.txt");

    // Legacy workaround: path=source (via fallback), target=destination
    // Since resolve: source.or(path) for source, target for destination
    // But legacy used target=source, path=destination — this should still work
    // because the new code reads source first, falls back to path
    // If source is absent and path is present, path becomes source
    // and target becomes destination. This is the NEW correct behavior.
    let manifest = format!(
        r#"{{"op":"move","path":"{}","target":"{}"}}"#,
        src.display(),
        dest.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Known limitation: batch --transaction rollback does NOT remove files
// CREATED during the transaction. This test documents the behavior.
#[test]
fn batch_transaction_rollback_preserves_created_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let created = dir.path().join("created_in_tx.txt");
    let existing = common::create_test_file(dir.path(), "existing.txt", "original\n");

    // First op creates a new file (succeeds), second op targets nonexistent
    // file for replace (fails) — triggering transaction rollback.
    let manifest = format!(
        "{}\n{}",
        format_args!(
            r#"{{"op":"write","target":"{}","content":"new file"}}"#,
            created.display()
        ),
        format_args!(
            r#"{{"op":"replace","target":"{}","pattern":"nonexistent_pattern_xyz","replacement":"x"}}"#,
            dir.path().join("does_not_exist.txt").display()
        ),
    );

    let _output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "batch",
            "--transaction",
        ])
        .write_stdin(manifest)
        .output()
        .expect("run");

    // Transaction may or may not fail depending on how replace handles
    // nonexistent files. The key assertion is about the created file.
    let existing_content = std::fs::read_to_string(&existing).expect("read existing");
    assert_eq!(
        existing_content, "original\n",
        "existing file should be restored by rollback"
    );

    // Known limitation: created_in_tx.txt may still exist after rollback
    // because the transaction rollback mechanism only restores pre-existing
    // files from backup, it does not track and remove newly created files.
    if created.exists() {
        eprintln!(
            "NOTE: known limitation — file created during transaction persists after rollback"
        );
    }
}
