// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.28 GAP-R1..R5 — `batch` backup/retention semantics.
//!
//! T1 fixed 5 residual gaps in `batch`: `--retention` and `--backup` were
//! parsed but silently discarded (G-R1, G-R2); the transactional pre-backup
//! (G-R3) and the `delete` op's backup (G-R4) hardcoded retention=5 instead
//! of honoring `--retention`/`[defaults]`. This suite verifies the SEMANTIC
//! effect (`.bak.*` sidecar files on disk, not just parse-level exit codes).
//!
//! Effective backup per op (see `src/commands/batch.rs`):
//!   `!no_backup && (op.backup || keep_backup || backup_explicit)`
//! where `backup_explicit = backup_opts.backup == Some(true)`.
//!
//! Note: `write`/`edit`/`replace`/`copy` route through `atomic_write`, whose
//! backup auto-removes on success unless `--keep-backup` is also passed
//! (same rule as every other subcommand, see `cli_v0128_backup_matrix.rs`).
//! `delete` and the transaction's pre-backup snapshot call `create_backup`
//! directly and always persist their `.bak.*` on success.

mod common;

fn count_bak_files_for(dir: &std::path::Path, filename: &str) -> usize {
    let prefix = format!("{filename}.bak.");
    std::fs::read_dir(dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with(&prefix))
        })
        .count()
}

/// G-R1: `batch --retention N` must cap accumulated `.bak.*` files instead
/// of being silently ignored (old code always used the hardcoded/default
/// retention regardless of this flag).
#[test]
fn batch_retention_caps_write_backups_within_single_manifest() {
    let dir = tempfile::tempdir().expect("tempdir");

    // `write` backups auto-remove on success unless --keep-backup is also
    // passed (same rule as every other subcommand); pair --keep-backup with
    // --retention here so the accumulated count is actually observable.
    let manifest = common::manifest(&[
        serde_json::json!({"op": "write", "target": "f.txt", "content": "v0\n"}),
        serde_json::json!({"op": "write", "target": "f.txt", "content": "v1\n"}),
        serde_json::json!({"op": "write", "target": "f.txt", "content": "v2\n"}),
        serde_json::json!({"op": "write", "target": "f.txt", "content": "v3\n"}),
    ]);

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "batch",
            "--keep-backup",
            "--retention",
            "2",
        ])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "batch --retention 2 --keep-backup deve suceder: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
        "v3\n"
    );

    let bak_count = count_bak_files_for(dir.path(), "f.txt");
    assert!(
        bak_count <= 2,
        "batch --retention 2 deve limitar backups acumulados de f.txt a no maximo 2, obteve {bak_count}"
    );
    assert!(
        bak_count >= 1,
        "pelo menos 1 backup deveria ter sido criado (3 das 4 escritas encontram o alvo existente)"
    );
}

/// G-R2: `batch --backup` (explicit flag) must force backup=true for every
/// op, even when the op omits its own `"backup"` field (old code silently
/// discarded this flag).
#[test]
fn batch_explicit_backup_flag_forces_backup_for_op_without_own_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "f.txt", "bye\n");

    // `delete` does not route through atomic_write's auto-remove, so its
    // backup persists unconditionally on success — a clean observable for
    // "was a backup created at all".
    let manifest = common::manifest(&[serde_json::json!({
        "op": "delete",
        "target": "f.txt",
    })]);

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "batch",
            "--backup",
        ])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "batch --backup com op sem campo backup deve suceder: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!dir.path().join("f.txt").exists());
    assert_eq!(
        count_bak_files_for(dir.path(), "f.txt"),
        1,
        "batch --backup explicito deve forcar backup mesmo com op.backup ausente/false"
    );
}

/// Regression: `batch --no-backup` must still force zero backups even when
/// the op explicitly sets `"backup":true` — `--no-backup` wins over any
/// per-op request.
#[test]
fn batch_no_backup_overrides_op_level_backup_true() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "f.txt", "bye\n");

    let manifest = common::manifest(&[serde_json::json!({
        "op": "delete",
        "target": "f.txt",
        "backup": true,
    })]);

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "batch",
            "--no-backup",
        ])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "batch --no-backup com op.backup=true deve suceder: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!dir.path().join("f.txt").exists());
    assert_eq!(
        count_bak_files_for(dir.path(), "f.txt"),
        0,
        "batch --no-backup deve vencer op.backup=true (regressao)"
    );
}

/// G-R3: the transaction's pre-mutation snapshot must honor `--retention`
/// instead of the hardcoded `5` — repeated transactional batches against
/// the same file must not accumulate unbounded pre-backups.
#[test]
fn batch_transaction_pre_backup_honors_retention() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "f.txt", "seed\n");

    for i in 0..3 {
        let manifest = common::manifest(&[serde_json::json!({
            "op": "write",
            "target": "f.txt",
            "content": format!("v{i}\n"),
        })]);

        let output = common::atomwrite()
            .env_remove("ATOMWRITE_BACKUP")
            .args([
                "--workspace",
                dir.path().to_str().unwrap(),
                "batch",
                "--transaction",
                "--retention",
                "1",
            ])
            .write_stdin(manifest)
            .output()
            .expect("run");

        assert!(
            output.status.success(),
            "batch --transaction --retention 1 iteracao {i} deve suceder: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let bak_count = count_bak_files_for(dir.path(), "f.txt");
    assert!(
        bak_count <= 1,
        "batch --transaction --retention 1 deve limitar o pre-backup transacional acumulado a no maximo 1, obteve {bak_count}"
    );
}

/// G-R4: the `delete` op's backup must honor `--retention` instead of the
/// hardcoded `5` — repeated write+delete cycles in one manifest must not
/// accumulate unbounded `.bak.*` files.
#[test]
fn batch_delete_op_backup_honors_retention() {
    let dir = tempfile::tempdir().expect("tempdir");

    let mut ops = Vec::new();
    for i in 0..4 {
        ops.push(serde_json::json!({
            "op": "write",
            "target": "f.txt",
            "content": format!("v{i}\n"),
        }));
        ops.push(serde_json::json!({
            "op": "delete",
            "target": "f.txt",
            "backup": true,
        }));
    }
    let manifest = common::manifest(&ops);

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "batch",
            "--retention",
            "2",
        ])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "ciclos write+delete com --retention 2 devem suceder: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!dir.path().join("f.txt").exists());

    let bak_count = count_bak_files_for(dir.path(), "f.txt");
    assert!(
        bak_count <= 2,
        "op delete com backup deve honrar --retention 2 (nao o hardcode antigo de 5), obteve {bak_count}"
    );
    assert!(
        bak_count >= 1,
        "pelo menos 1 backup de delete deveria existir"
    );
}

/// Regression: `--backup` and `--no-backup` remain mutually exclusive on
/// `batch` (clap `conflicts_with`), independent of the T1 fix.
#[test]
fn batch_backup_and_no_backup_together_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "batch",
            "--backup",
            "--no-backup",
        ])
        .write_stdin("")
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(2),
        "batch --backup --no-backup deve ser rejeitado pelo clap com exit 2: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}
