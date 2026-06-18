// SPDX-License-Identifier: MIT OR Apache-2.0
//! v0.1.22 ADR-0040 — `prune-backups` subcommand regression tests.

mod common;

/// Helper: touch a file with a given mtime so we can simulate backups of
/// varying ages without sleeping. The `filetime` crate is in dev-deps.
fn touch_mtime(path: &std::path::Path, t: std::time::SystemTime) {
    let ft = filetime::FileTime::from_system_time(t);
    filetime::set_file_mtime(path, ft).expect("set mtime");
}

fn count_backups(dir: &std::path::Path, prefix: &str) -> usize {
    std::fs::read_dir(dir)
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with(prefix))
        })
        .count()
}

#[test]
fn prune_backups_dry_run_does_not_delete() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "original\n").expect("seed");

    let bak1 = dir.path().join("file.txt.bak.20260101_120000");
    let bak2 = dir.path().join("file.txt.bak.20260201_120000");
    let bak3 = dir.path().join("file.txt.bak.20260301_120000");
    for (i, b) in [&bak1, &bak2, &bak3].iter().enumerate() {
        std::fs::write(b, format!("v{i}\n")).expect("bak");
        let t =
            std::time::SystemTime::now() - std::time::Duration::from_secs(86400 * (i as u64 + 30));
        touch_mtime(b, t);
    }

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "prune-backups",
            "--max-age-secs",
            "86400",
            "--dry-run",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "dry-run should succeed, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // dry-run NÃO deve deletar nada
    let ndjson = common::parse_ndjson(&output.stdout);
    let summary = ndjson
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary line");
    assert_eq!(summary["action"], "dry_run");
    assert_eq!(summary["total"], 3);

    assert_eq!(count_backups(dir.path(), "file.txt.bak."), 3);
    assert!(target.exists());
}

#[test]
fn prune_backups_max_age_removes_only_old() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "now\n").expect("seed");

    let recent = dir.path().join("file.txt.bak.20260617_120000");
    std::fs::write(&recent, "recent\n").expect("recent");
    touch_mtime(&recent, std::time::SystemTime::now());

    let old = dir.path().join("file.txt.bak.20260615_120000");
    std::fs::write(&old, "old\n").expect("old");
    touch_mtime(
        &old,
        std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 86400),
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "prune-backups",
            "--max-age-secs",
            "86400",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(recent.exists(), "backup recente deve permanecer");
    assert!(!old.exists(), "backup antigo deve ter sido removido");
}

#[test]
fn prune_backups_max_count_keeps_newest_n() {
    // Per ADR-0040: `--max-count N` means "keep at most N most-recent
    // backups". With 5 backups and `--max-count 2`, the 2 most recent
    // survive and the 3 oldest are deleted.
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "now\n").expect("seed");

    // 5 backups with mtimes spaced by 1 minute (most recent = i=4)
    for i in 0..5 {
        let bak = dir.path().join(format!("file.txt.bak.2026060{i}_120000"));
        std::fs::write(&bak, format!("v{i}\n")).expect("bak");
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs((5 - i) as u64 * 60);
        touch_mtime(&bak, t);
    }

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "prune-backups",
            "--max-count",
            "2",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Keeps the 2 most recent (i=4 and i=3), deletes the 3 oldest (i=0,1,2).
    assert_eq!(count_backups(dir.path(), "file.txt.bak."), 2);
}

#[test]
fn prune_backups_no_backups_is_noop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "original\n").expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "prune-backups",
            "--max-age-secs",
            "0",
        ])
        .arg(&target)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(target.exists());

    let ndjson = common::parse_ndjson(&output.stdout);
    let summary = ndjson
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["total"], 0);
}

#[test]
fn prune_backups_target_not_found_emits_skipped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let nonexistent = dir.path().join("missing.txt");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "prune-backups",
            "--max-age-secs",
            "86400",
        ])
        .arg(&nonexistent)
        .output()
        .expect("run");

    // O binário deve completar com sucesso e emitir `skipped` para o path
    // ausente (não é um erro fatal — o operador pode passar uma lista mista
    // de paths existentes e ausentes).
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ndjson = common::parse_ndjson(&output.stdout);
    let skipped = ndjson
        .iter()
        .find(|e| e["type"] == "skipped")
        .expect("skipped line");
    assert_eq!(skipped["reason"], "not_found");
}

#[test]
fn prune_backups_no_filter_aborts_with_error() {
    // Per ADR-0040 (VAI-PSIQUE-CHECK), prune-backups refuses to delete
    // anything when neither --max-age-secs nor --max-count is provided.
    // Without this guard, the default `Option<u8>::None` resolution
    // combined with the prune loop would silently delete every backup
    // for the target — a data-loss footgun.
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "now\n").expect("seed");

    for i in 0..3 {
        let bak = dir.path().join(format!("file.txt.bak.2026060{i}_120000"));
        std::fs::write(&bak, format!("v{i}\n")).expect("bak");
    }

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "prune-backups"])
        .arg(&target)
        .output()
        .expect("run");

    // Should fail (exit non-zero) because no filter was specified.
    assert!(
        !output.status.success(),
        "prune-backups sem filtro deve falhar, got exit={:?}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    // All 3 backups must be preserved — the bail happens BEFORE any deletion.
    assert_eq!(
        count_backups(dir.path(), "file.txt.bak."),
        3,
        "bail não deve deletar nenhum backup"
    );
}

#[test]
fn prune_backups_only_targets_correct_file() {
    // Garante que prune-backups para `file_a.txt` não toca `file_b.txt.bak.*`
    let dir = tempfile::tempdir().expect("tempdir");
    let target_a = dir.path().join("file_a.txt");
    let target_b = dir.path().join("file_b.txt");
    std::fs::write(&target_a, "a\n").expect("seed a");
    std::fs::write(&target_b, "b\n").expect("seed b");

    for prefix in ["file_a", "file_b"] {
        for i in 0..2 {
            let bak = dir
                .path()
                .join(format!("{prefix}.txt.bak.2026060{i}_120000"));
            std::fs::write(&bak, format!("{prefix}-{i}\n")).expect("bak");
            let t = std::time::SystemTime::now() - std::time::Duration::from_secs(86400 * 30);
            touch_mtime(&bak, t);
        }
    }

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "prune-backups",
            "--max-age-secs",
            "86400",
        ])
        .arg(&target_a)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert_eq!(count_backups(dir.path(), "file_a.txt.bak."), 0);
    assert_eq!(
        count_backups(dir.path(), "file_b.txt.bak."),
        2,
        "prune-backups não deve afetar backups de outro arquivo"
    );
}
