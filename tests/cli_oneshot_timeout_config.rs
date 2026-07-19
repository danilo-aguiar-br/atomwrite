// SPDX-License-Identifier: MIT OR Apache-2.0

//! One-shot CLI rules: global --timeout-secs / --config / --no-progress.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn timeout_secs_alias_timeout_accepted() {
    let mut cmd = Command::cargo_bin("atomwrite").unwrap();
    cmd.args(["--timeout", "30", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor_report"));
}

#[test]
fn timeout_secs_flag_accepted() {
    let mut cmd = Command::cargo_bin("atomwrite").unwrap();
    cmd.args(["--timeout-secs", "30", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor_report"));
}

#[test]
fn config_missing_is_hard_error() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("does-not-exist.toml");
    let mut cmd = Command::cargo_bin("atomwrite").unwrap();
    cmd.args([
        "--config",
        missing.to_str().unwrap(),
        "--workspace",
        dir.path().to_str().unwrap(),
        "doctor",
    ])
    .assert()
    .failure()
    .stdout(predicate::str::contains("CONFIG").or(predicate::str::contains("config")));
}

#[test]
fn config_explicit_loads() {
    let dir = tempdir().unwrap();
    let cfg = dir.path().join("custom.toml");
    std::fs::write(&cfg, "[defaults]\nbackup = false\nretention = 2\n").unwrap();
    let mut cmd = Command::cargo_bin("atomwrite").unwrap();
    cmd.args([
        "--config",
        cfg.to_str().unwrap(),
        "--workspace",
        dir.path().to_str().unwrap(),
        "doctor",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("defaults.backup=false"))
    .stdout(predicate::str::contains("retention=2"));
}

#[test]
fn no_progress_flag_accepted() {
    let mut cmd = Command::cargo_bin("atomwrite").unwrap();
    cmd.args(["--no-progress", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor_report"));
}

#[test]
fn help_documents_timeout_and_config() {
    let mut cmd = Command::cargo_bin("atomwrite").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--timeout-secs"))
        .stdout(predicate::str::contains("120"))
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--no-progress"));
}

#[test]
fn arm_global_timeout_sets_flag() {
    // Unit-level via public signal API after early install.
    let _ = atomwrite::signal::install_handlers_early();
    atomwrite::signal::arm_global_timeout(1);
    // Wait slightly over 1s for the watchdog.
    std::thread::sleep(std::time::Duration::from_millis(1200));
    assert!(
        atomwrite::signal::is_global_shutdown(),
        "global timeout must set cooperative cancel flag"
    );
    let sig = atomwrite::signal::get_or_install_handlers().unwrap();
    assert!(sig.is_timeout());
    assert_eq!(sig.exit_code(), 124);
}
