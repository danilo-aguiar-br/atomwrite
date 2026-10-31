// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 R25/R26 — strict config loading and the opt-in log file sink.
//!
//! Every fixture lives in a temporary directory. The log-sink tests redirect
//! the XDG directories of the child process only, so they never touch the real
//! home; they are Linux-only because `directories` honours `XDG_*` there alone.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::{TempDir, tempdir};

/// A workspace holding one readable file, plus the file name.
fn workspace() -> TempDir {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("a.txt"), "hello\n").expect("write fixture");
    dir
}

/// Absolute path of the fixture file (read targets resolve against the CWD).
fn target(ws: &TempDir) -> String {
    ws.path().join("a.txt").to_string_lossy().into_owned()
}

fn atomwrite() -> Command {
    common::hermetic(Command::cargo_bin("atomwrite").expect("binary"))
}

/// Run `read a.txt` in `ws` with an auto-discovered `.atomwrite.toml` body.
fn read_with_local_config(ws: &TempDir, body: &str) -> assert_cmd::assert::Assert {
    std::fs::write(ws.path().join(".atomwrite.toml"), body).expect("write config");
    atomwrite()
        .args([
            "--workspace",
            ws.path().to_str().unwrap(),
            "read",
            target(ws).as_str(),
        ])
        .assert()
}

#[test]
fn unknown_key_in_auto_discovered_config_exits_78() {
    let ws = workspace();
    read_with_local_config(&ws, "[defaults]\nbogus = 1\n")
        .code(78)
        .stdout(predicate::str::contains("CONFIG_INVALID"))
        .stdout(predicate::str::contains(".atomwrite.toml"));
}

#[test]
fn unknown_section_in_auto_discovered_config_exits_78() {
    let ws = workspace();
    read_with_local_config(&ws, "[nope]\nx = 1\n").code(78);
}

#[test]
fn malformed_auto_discovered_config_exits_78() {
    let ws = workspace();
    read_with_local_config(&ws, "not valid { toml").code(78);
}

#[test]
fn future_config_version_exits_78() {
    let ws = workspace();
    read_with_local_config(&ws, "version = 999\n")
        .code(78)
        .stdout(predicate::str::contains("CONFIG_INVALID"));
}

#[test]
fn current_config_version_loads() {
    let ws = workspace();
    read_with_local_config(&ws, "version = 1\n[defaults]\nbackup = false\n").success();
}

#[test]
fn explicit_config_with_unknown_key_exits_78() {
    let ws = workspace();
    let cfg = ws.path().join("custom.toml");
    std::fs::write(&cfg, "[fuzzy]\nthreshhold = 0.9\n").expect("write config");
    atomwrite()
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "--workspace",
            ws.path().to_str().unwrap(),
            "read",
            target(&ws).as_str(),
        ])
        .assert()
        .code(78)
        .stdout(predicate::str::contains("CONFIG_INVALID"));
}

#[cfg(target_os = "linux")]
mod log_sink {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    /// Run `read a.txt` with the XDG directories of the child under `xdg`.
    fn read_with_xdg(ws: &TempDir, xdg: &Path) -> assert_cmd::assert::Assert {
        atomwrite()
            .env("HOME", xdg)
            .env("XDG_CONFIG_HOME", xdg.join("config"))
            .env("XDG_STATE_HOME", xdg.join("state"))
            .args([
                "--workspace",
                ws.path().to_str().unwrap(),
                "read",
                target(ws).as_str(),
            ])
            .assert()
    }

    fn logs_dir(xdg: &Path) -> PathBuf {
        xdg.join("state").join("atomwrite").join("logs")
    }

    fn write_global_config(xdg: &Path, body: &str) {
        let dir = xdg.join("config").join("atomwrite");
        std::fs::create_dir_all(&dir).expect("config dir");
        std::fs::write(dir.join("config.toml"), body).expect("write global config");
    }

    #[test]
    fn no_log_file_is_created_by_default() {
        let ws = workspace();
        let xdg = tempdir().expect("xdg");
        read_with_xdg(&ws, xdg.path()).success();
        assert!(
            !logs_dir(xdg.path()).exists(),
            "the file sink must be off unless [log] file = true"
        );
    }

    #[test]
    fn log_file_is_created_owner_only_when_enabled() {
        let ws = workspace();
        let xdg = tempdir().expect("xdg");
        write_global_config(xdg.path(), "[log]\nfile = true\n");
        read_with_xdg(&ws, xdg.path()).success();

        let dir = logs_dir(xdg.path());
        let dir_mode = std::fs::metadata(&dir)
            .expect("logs dir")
            .permissions()
            .mode();
        assert_eq!(dir_mode & 0o777, 0o700, "logs dir must be owner-only");

        let files: Vec<_> = std::fs::read_dir(&dir)
            .expect("read logs dir")
            .filter_map(Result::ok)
            .collect();
        assert_eq!(files.len(), 1, "exactly one log file expected: {files:?}");
        let name = files[0].file_name().to_string_lossy().into_owned();
        assert!(
            name.starts_with("atomwrite.") && name.ends_with(".jsonl"),
            "dated rotating name expected, got {name}"
        );
        let mode = files[0].metadata().expect("meta").permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "log file must be owner-only");
    }

    #[test]
    fn invalid_log_rotation_exits_78() {
        let ws = workspace();
        let xdg = tempdir().expect("xdg");
        write_global_config(xdg.path(), "[log]\nfile = true\nrotation = \"weekly\"\n");
        read_with_xdg(&ws, xdg.path())
            .code(78)
            .stdout(predicate::str::contains("CONFIG_INVALID"));
        assert!(
            !logs_dir(xdg.path()).exists(),
            "an invalid [log] section must not open the sink"
        );
    }
}
