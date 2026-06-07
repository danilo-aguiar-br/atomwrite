// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end tests for G114 — WAL sidecar for crash recovery.
//!
//! Verifies that atomic_write creates a `.atomwrite.journal.<target>.json`
//! file with `Started` and `Committed` entries for crash recovery
//! semantics.

mod common;

#[test]
fn wal_journal_created_on_set() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&toml_path, "[package]\nversion = \"0.1.0\"\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "package.version",
            "1.0.0",
        ])
        .output()
        .expect("set");

    assert!(
        output.status.success(),
        "set failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let journal_path = dir
        .path()
        .join(".atomwrite.journal.Cargo.toml.atomwrite.journal.json");
    assert!(
        journal_path.exists(),
        "expected journal file at {}, but it doesn't exist. Files: {:?}",
        journal_path.display(),
        std::fs::read_dir(dir.path()).map(|d| d
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect::<Vec<_>>())
    );

    let journal = std::fs::read_to_string(&journal_path).expect("read journal");
    assert!(
        journal.contains("\"started\""),
        "expected started event in journal: {journal}"
    );
    assert!(
        journal.contains("\"committed\""),
        "expected committed event in journal: {journal}"
    );
}

#[test]
fn wal_journal_has_op_id_correlation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("config.toml");
    std::fs::write(&toml_path, "name = \"x\"\n").expect("write");

    let _ = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "name",
            "y",
        ])
        .output()
        .expect("set");

    let journal_path = dir
        .path()
        .join(".atomwrite.journal.config.toml.atomwrite.journal.json");
    assert!(journal_path.exists(), "journal file should exist");
    let journal = std::fs::read_to_string(&journal_path).expect("read");
    assert!(journal.contains("op_id"), "expected op_id field: {journal}");
    assert!(
        journal.contains("target"),
        "expected target field: {journal}"
    );
}

#[test]
fn wal_journal_contains_expected_checksum() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("x.toml");
    std::fs::write(&toml_path, "name = \"x\"\n").expect("write");

    let _ = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "name",
            "y",
        ])
        .output()
        .expect("set");

    let journal_path = dir
        .path()
        .join(".atomwrite.journal.x.toml.atomwrite.journal.json");
    let journal = std::fs::read_to_string(&journal_path).expect("read");
    assert!(
        journal.contains("checksum_after"),
        "expected checksum: {journal}"
    );
    assert!(
        journal.contains("started_at_unix"),
        "expected unix timestamp: {journal}"
    );
}

#[test]
fn wal_no_journal_on_validation_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("nonexistent.toml");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "set",
            toml_path.to_str().unwrap(),
            "key",
            "value",
        ])
        .output()
        .expect("set");

    assert!(!output.status.success(), "set on missing file should fail");
    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("journal"))
        .collect();
    assert!(
        entries.is_empty(),
        "no journal should be created on validation failure, but found: {:?}",
        entries.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );
}
