// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.21 GAP-2026-014 v2: property test for backup cleanup idempotence.
//!
//! `delete_backup_quietly` must be idempotent: calling it on a
//! non-existent path is a no-op (returns Ok(())), and calling it
//! multiple times on the same path always returns Ok(()). This
//! property is exercised by running `write --backup` multiple times
//! (which internally calls `delete_backup_quietly` after each
//! successful write) and verifying that no .bak files accumulate.

mod common;

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5))]

    /// Property: running `write --backup` N times (1..=5) must not
    /// accumulate .bak files, because each successful write cleans up
    /// its backup via `delete_backup_quietly`. This holds regardless
    /// of the content written.
    #[test]
    fn delete_backup_quietly_is_idempotent(
        iterations in 1usize..=5,
        content in "\\PC{1,256}"
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("prop_target.txt");
        std::fs::write(&target, "initial\n").expect("seed");

        for i in 0..iterations {
            let payload = format!("{content}\n{i}\n");
            let output = common::atomwrite()
                .args([
                    "--workspace",
                    dir.path().to_str().unwrap(),
                    "write",
                    "--backup",
                    "--retention",
                    "3",
                ])
                .arg(&target)
                .write_stdin(payload.as_bytes())
                .output()
                .expect("write");

            prop_assert!(
                output.status.success(),
                "write iteration {i} should succeed, stderr={}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // After all iterations, no .bak files should remain.
        let bak_entries: Vec<_> = std::fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("prop_target.txt.bak."))
            })
            .collect();

        prop_assert!(
            bak_entries.is_empty(),
            "no .bak.* files should accumulate after {} write iterations, found: {:?}",
            iterations,
            bak_entries.iter().map(|e| e.file_name()).collect::<Vec<_>>()
        );
    }

    /// Property: writing to a path that has no pre-existing .bak
    /// (i.e., a non-existent target) is trivially a no-op for the
    /// backup cleanup path. The write succeeds, no .bak is created,
    /// and `delete_backup_quietly` is never even called.
    #[test]
    fn write_new_file_creates_no_backup(
        filename_suffix in "[a-z]{1,8}",
        content in "\\PC{1,128}"
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join(format!("new_{filename_suffix}.txt"));

        let output = common::atomwrite()
            .args([
                "--workspace",
                dir.path().to_str().unwrap(),
                "write",
                "--backup",
                "--retention",
                "3",
            ])
            .arg(&target)
            .write_stdin(content.as_bytes())
            .output()
            .expect("write");

        prop_assert!(
            output.status.success(),
            "write to new file should succeed, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );

        // No .bak should be created when the target did not pre-exist.
        let bak_entries: Vec<_> = std::fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                name.to_str()
                    .is_some_and(|n| n.starts_with(&format!("new_{filename_suffix}.txt.bak.")))
            })
            .collect();

        prop_assert!(
            bak_entries.is_empty(),
            "no .bak.* should be created for new file, found: {:?}",
            bak_entries.iter().map(|e| e.file_name()).collect::<Vec<_>>()
        );
    }
}

// v0.1.22 — additional property tests gated behind `slow-tests` feature.
// These run 20 cases each and exercise sequential-drift and backup retention
// invariants of the new edit-loop / backup-keep combinations.

/// Property: chaining N `edit` calls with the SAME initial checksum and the
/// `--allow-sequential-drift` flag must always succeed. This is the
/// v0.1.21 sequential-drift property ported to `--allow-sequential-drift`.
#[cfg(feature = "slow-tests")]
#[test]
fn allow_sequential_drift_accepts_any_sequence() {
    use proptest::prelude::*;

    proptest!(ProptestConfig::with_cases(20), |(
        initial in "[a-z]{1,20}",
        n_edits in 1u32..5,
    )| {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("prop_chain.txt");
        std::fs::write(&target, &initial).expect("seed");

        // Capture initial checksum
        let hash_out = common::atomwrite()
            .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
            .arg(&target)
            .output()
            .expect("hash");
        let initial_cs = common::parse_ndjson(&hash_out.stdout)[0]["value"]
            .as_str()
            .expect("value")
            .to_string();

        // Apply N sequential edits reusing the initial checksum
        for i in 0..n_edits {
            let new_val = format!("{initial}_v{i}");
            let output = common::atomwrite()
                .args([
                    "--workspace",
                    dir.path().to_str().unwrap(),
                    "edit",
                    "--allow-sequential-drift",
                    "--old",
                    &initial,
                    "--new",
                    &new_val,
                    "--expect-checksum",
                    &initial_cs,
                ])
                .arg(&target)
                .output()
                .expect("run");
            prop_assert!(
                output.status.success(),
                "edit {i} deve passar com --allow-sequential-drift, stderr={}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    });
}

/// Property: `write --backup --keep-backup` N times (1..=5) MUST preserve
/// exactly N `.bak.*` files. This is the v0.1.21 GAP-014 v2 invariant
/// ("keep_backup preserves across N writes") from ADR-0038.
///
/// Note: backup filenames embed a timestamp with second-resolution
/// (`.bak.YYYYMMDD_HHMMSS`). When two writes fire within the same wall
/// clock second, they collide on the filename and the second write
/// overwrites the first backup. To make the property deterministic we
/// sleep just over one second between writes.
#[cfg(feature = "slow-tests")]
#[test]
fn keep_backup_preserves_across_n_writes() {
    use proptest::prelude::*;
    use std::time::Duration;

    proptest!(ProptestConfig::with_cases(5), |(
        n_writes in 1u32..4,
        content in "[a-z]{1,10}",
    )| {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("file.txt");
        std::fs::write(&target, &content).expect("seed");

        for i in 0..n_writes {
            let payload = format!("{content}\nv{i}\n");
            let output = common::atomwrite()
                .args([
                    "--workspace",
                    dir.path().to_str().unwrap(),
                    "write",
                    "--backup",
                    "--keep-backup",
                ])
                .arg(&target)
                .write_stdin(payload.as_bytes())
                .output()
                .expect("write");
            prop_assert!(
                output.status.success(),
                "write {i} falhou: stderr={}",
                String::from_utf8_lossy(&output.stderr)
            );
            // Sleep >1s to guarantee the next backup filename differs.
            std::thread::sleep(Duration::from_millis(1100));
        }

        let count: usize = std::fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("file.txt.bak."))
            })
            .count();
        let expected = n_writes as usize;
        prop_assert_eq!(
            count,
            expected,
            "esperava {} backups preservados, got {}",
            expected,
            count
        );
    });
}
