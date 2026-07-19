// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.28 GAP-CLI-SURFACE-DRIFT — `.atomwrite.toml [defaults]` becomes
//! effective for the shared `BackupOpts` (`resolve_backup` in
//! `src/commands/mod.rs`). Precedence implemented there is:
//!
//!   `ATOMWRITE_BACKUP` env \> `--no-backup`/`--backup` \> `[defaults].backup`
//!   \> built-in `true`.
//!   `--retention` \> `[defaults].retention` \> built-in `5`.
//!
//! These tests observe that precedence end-to-end via the presence/absence
//! of `.bak.*` sidecar files, using `delete` as the observable command
//! (unlike `write`/`edit`/etc., whose transactional backup auto-removes on
//! success and leaves nothing to inspect; `delete` always preserves its
//! backup on success, per ADR / GAP-CLI-SURFACE-DRIFT).

mod common;

fn write_config(workspace: &std::path::Path, body: &str) {
    std::fs::write(workspace.join(".atomwrite.toml"), body).expect("write .atomwrite.toml");
}

fn has_bak_file(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .any(|e| e.file_name().to_string_lossy().contains(".bak."))
}

fn count_bak_files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().contains(".bak."))
        .count()
}

#[test]
fn config_defaults_backup_false_is_honored_without_flags() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_config(dir.path(), "[defaults]\nbackup = false\nretention = 2\n");
    let path = common::create_test_file(dir.path(), "f.txt", "bye\n");

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--yes",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(
        !has_bak_file(dir.path()),
        "[defaults].backup = false must be honored when no backup flag is passed"
    );
}

#[test]
fn explicit_backup_flag_overrides_config_defaults_backup_false() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_config(dir.path(), "[defaults]\nbackup = false\nretention = 2\n");
    let path = common::create_test_file(dir.path(), "f.txt", "bye\n");

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--backup",
            "--yes",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(
        has_bak_file(dir.path()),
        "explicit --backup must override [defaults].backup = false"
    );
}

#[test]
fn env_atomwrite_backup_zero_overrides_explicit_backup_flag() {
    // Documented precedence: ATOMWRITE_BACKUP env wins over --backup/--no-backup
    // unconditionally (see resolve_backup in src/commands/mod.rs).
    let dir = tempfile::tempdir().expect("tempdir");
    // No config file: built-in default is backup = true; --backup would be
    // redundant confirmation, so the env override is the only thing that
    // can flip the outcome to "no backup".
    let path = common::create_test_file(dir.path(), "f.txt", "bye\n");

    let output = common::atomwrite()
        .env("ATOMWRITE_BACKUP", "0")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--backup",
            "--yes",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(
        !has_bak_file(dir.path()),
        "ATOMWRITE_BACKUP=0 must override explicit --backup (env has highest precedence)"
    );
}

#[test]
fn env_atomwrite_backup_nonzero_overrides_config_defaults_backup_false() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_config(dir.path(), "[defaults]\nbackup = false\n");
    let path = common::create_test_file(dir.path(), "f.txt", "bye\n");

    let output = common::atomwrite()
        .env("ATOMWRITE_BACKUP", "1")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--yes",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(
        has_bak_file(dir.path()),
        "ATOMWRITE_BACKUP=1 must override [defaults].backup = false even without --backup"
    );
}

#[test]
fn config_defaults_retention_caps_accumulated_backups() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_config(dir.path(), "[defaults]\nbackup = true\nretention = 2\n");

    // Recreate and delete the same filename repeatedly: each delete backs
    // up the current version before removing it, and create_backup_in
    // enforces retention synchronously (src/atomic.rs cleanup_old_backups_in).
    for i in 0..4 {
        common::create_test_file(dir.path(), "f.txt", &format!("version {i}\n"));
        let output = common::atomwrite()
            .env_remove("ATOMWRITE_BACKUP")
            .args([
                "--workspace",
                dir.path().to_str().unwrap(),
                "delete",
                "--yes",
                "f.txt",
            ])
            .output()
            .expect("run");
        assert!(output.status.success(), "delete iteration {i} must succeed");
    }

    let bak_count = count_bak_files(dir.path());
    assert!(
        bak_count <= 2,
        "[defaults].retention = 2 must limit accumulated backups to at most 2, got {bak_count}"
    );
}

#[test]
fn config_present_with_retention_does_not_break_write_happy_path() {
    // v0.1.28 GAP-CLI-SURFACE-DRIFT: `write`'s transactional backup
    // auto-removes on success, so `.bak.*` presence isn't observable here.
    // This test only asserts parse+run succeeds with a config file present
    // (the minimal bar requested for the retention knob on `write`).
    let dir = tempfile::tempdir().expect("tempdir");
    write_config(dir.path(), "[defaults]\nbackup = true\nretention = 2\n");

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg("f.txt")
        .write_stdin("hello\n")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "write with .atomwrite.toml [defaults].retention present must not fail: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}
