// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end tests for G114 + G119 — WAL sidecar for crash recovery,
//! plus the G119 L2 Drop guard that auto-cleans Committed sidecars.
//!
//! Pre-v0.1.15, these tests asserted the sidecar persisted forever after
//! a successful write. From v0.1.15, the Drop guard removes it on
//! normal scope exit. The tests now assert:
//!   - `wal-stats` reports the workspace state correctly
//!   - a `wal-heal` pass reaps stale terminal journals older than the
//!     threshold (G119 L3)
//!   - `Started` sidecars (orphans) are NEVER auto-removed

mod common;

#[test]
fn wal_journal_drop_guard_cleans_committed_after_set() {
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

    // G119 L2: the Drop guard removes the sidecar on normal scope exit
    // after the `Committed` entry is written. The sidecar must NOT
    // survive a successful set.
    let journal_path = dir
        .path()
        .join(".atomwrite.journal.Cargo.toml.atomwrite.journal.json");
    assert!(
        !journal_path.exists(),
        "successful set must NOT leave a WAL sidecar (G119 L2 Drop guard)"
    );
}

#[test]
fn wal_heal_reaps_stale_committed_journals() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&toml_path, "[package]\nversion = \"0.1.0\"\n").expect("write");

    // Manually create a stale Committed sidecar older than the threshold.
    let journal_path = dir
        .path()
        .join(".atomwrite.journal.Cargo.toml.atomwrite.journal.json");
    std::fs::write(
        &journal_path,
        "{\"phase\":\"started\",\"op_id\":\"abc123\",\"started_at_unix\":1700000000}\n\
         {\"phase\":\"committed\",\"op_id\":\"abc123\",\"committed_at_unix\":1700000001}\n",
    )
    .expect("seed sidecar");

    // Run wal-heal with threshold 0 so the seeded sidecar qualifies.
    // `--no-auto-heal` ensures the L3 startup pass does not reap the
    // seed before `wal-heal` runs (otherwise the explicit subcommand
    // would see 0 sidecars and the assertion below would fail).
    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "--no-auto-heal",
            "wal-heal",
            "--threshold-secs",
            "0",
        ])
        .output()
        .expect("heal");
    assert!(
        output.status.success(),
        "wal-heal failed: {:?}",
        output.status
    );

    let events = common::parse_ndjson(&output.stdout);
    let report = &events[0];
    let removed = report["removed"].as_u64().unwrap();
    assert!(removed >= 1, "expected at least 1 removed, got {removed}");

    // The sidecar must be gone.
    assert!(
        !journal_path.exists(),
        "wal-heal must remove stale Committed sidecars"
    );
}

#[test]
fn wal_heal_preserves_started_orphans() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&toml_path, "[package]\nversion = \"0.1.0\"\n").expect("write");

    // Manually create a Started-only sidecar (simulates a crashed write).
    let journal_path = dir
        .path()
        .join(".atomwrite.journal.Cargo.toml.atomwrite.journal.json");
    std::fs::write(
        &journal_path,
        "{\"phase\":\"started\",\"op_id\":\"orphan1\",\"started_at_unix\":1700000000}\n",
    )
    .expect("seed orphan");

    // Run wal-heal with threshold 0 — even with threshold 0, Started
    // orphans must NEVER be removed automatically.
    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "wal-heal",
            "--threshold-secs",
            "0",
        ])
        .output()
        .expect("heal");
    assert!(output.status.success());

    assert!(
        journal_path.exists(),
        "wal-heal must NEVER remove Started orphans (they need operator inspection)"
    );
}

#[test]
fn wal_stats_counts_committed_orphans_malformed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();

    // Seed one of each kind.
    let started_path = dir
        .path()
        .join(".atomwrite.journal.started.txt.atomwrite.journal.json");
    std::fs::write(
        &started_path,
        "{\"phase\":\"started\",\"op_id\":\"o1\",\"started_at_unix\":1700000000}\n",
    )
    .unwrap();
    let committed_path = dir
        .path()
        .join(".atomwrite.journal.committed.txt.atomwrite.journal.json");
    std::fs::write(
        &committed_path,
        "{\"phase\":\"started\",\"op_id\":\"c1\",\"started_at_unix\":1700000000}\n\
         {\"phase\":\"committed\",\"op_id\":\"c1\",\"committed_at_unix\":1700000001}\n",
    )
    .unwrap();
    let malformed_path = dir
        .path()
        .join(".atomwrite.journal.bad.txt.atomwrite.journal.json");
    std::fs::write(&malformed_path, "this is not json").unwrap();

    let output = common::atomwrite()
        .args(["--workspace", workspace, "--no-auto-heal", "wal-stats"])
        .output()
        .expect("stats");
    assert!(output.status.success());

    let events = common::parse_ndjson(&output.stdout);
    let stats = &events[0];
    assert_eq!(stats["total_journals"], 3);
    assert_eq!(stats["by_state"]["started"], 1);
    assert_eq!(stats["by_state"]["committed"], 1);
    assert_eq!(stats["by_state"]["malformed"], 1);
    assert_eq!(stats["by_state"]["aborted"], 0);
}
