// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 (A1): concurrent read-modify-write must not lose an update.
//!
//! Each test starts N real `atomwrite` processes at once against ONE file and
//! changes a different part of it in each. Before the lock covered the read,
//! every process exited 0 with `ok: true` while most of the changes were gone.
//! Now the writers serialise: every process succeeds and every change is there.

use std::path::Path;
use std::process::{Child, Command, Stdio};

use atomwrite::atomic::{AtomicWriteOptions, atomic_write_based};

/// Concurrent writers per test.
const WRITERS: usize = 30;

/// Wall-clock cap handed to every child.
const CHILD_TIMEOUT_SECS: &str = "120";

fn base_command(ws: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_atomwrite"));
    cmd.args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(["--no-progress", "--no-auto-heal"])
        .args(["--timeout-secs", CHILD_TIMEOUT_SECS]);
    cmd
}

fn spawn(mut cmd: Command) -> Child {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn atomwrite")
}

/// Wait for every child; return the exit codes and stdout texts.
fn finish(children: Vec<Child>) -> Vec<(Option<i32>, String)> {
    children
        .into_iter()
        .map(|c| {
            let out = c.wait_with_output().expect("wait");
            (
                out.status.code(),
                String::from_utf8_lossy(&out.stdout).into_owned(),
            )
        })
        .collect()
}

fn assert_all_succeeded(results: &[(Option<i32>, String)]) {
    for (i, (code, out)) in results.iter().enumerate() {
        assert_eq!(*code, Some(0), "writer {i} failed: {out}");
    }
}

fn numbered_lines() -> String {
    (1..=WRITERS).map(|i| format!("line{i}=0\n")).collect()
}

#[test]
fn a1_parallel_edits_of_distinct_lines_all_land() {
    let ws = tempfile::tempdir().expect("workspace");
    let file = ws.path().join("f.txt");
    std::fs::write(&file, numbered_lines()).expect("seed");

    let children: Vec<Child> = (1..=WRITERS)
        .map(|i| {
            let mut cmd = base_command(ws.path());
            cmd.args(["edit", "--no-backup", "f.txt"])
                .args(["--old", &format!("line{i}=0")])
                .args(["--new", &format!("line{i}=DONE")]);
            spawn(cmd)
        })
        .collect();
    assert_all_succeeded(&finish(children));

    let text = std::fs::read_to_string(&file).expect("read");
    assert_eq!(
        text.matches("=DONE").count(),
        WRITERS,
        "updates were lost:\n{text}"
    );
}

#[test]
fn a1_parallel_replaces_of_distinct_tokens_all_land() {
    let ws = tempfile::tempdir().expect("workspace");
    let file = ws.path().join("f.txt");
    std::fs::write(&file, numbered_lines()).expect("seed");

    let children: Vec<Child> = (1..=WRITERS)
        .map(|i| {
            let mut cmd = base_command(ws.path());
            cmd.args(["replace", "--no-backup", "--fuzzy", "off", "-F"])
                .arg(format!("line{i}=0"))
                .arg(format!("line{i}=DONE"))
                .arg("f.txt");
            spawn(cmd)
        })
        .collect();
    assert_all_succeeded(&finish(children));

    let text = std::fs::read_to_string(&file).expect("read");
    assert_eq!(
        text.matches("=DONE").count(),
        WRITERS,
        "updates were lost:\n{text}"
    );
}

#[test]
fn a1_parallel_set_of_distinct_keys_all_land() {
    let ws = tempfile::tempdir().expect("workspace");
    let file = ws.path().join("cfg.toml");
    std::fs::write(&file, "[base]\nkeep = true\n").expect("seed");

    let children: Vec<Child> = (1..=WRITERS)
        .map(|i| {
            let mut cmd = base_command(ws.path());
            cmd.args(["set", "--no-backup", "cfg.toml"])
                .arg(format!("section.key{i}"))
                .arg(i.to_string());
            spawn(cmd)
        })
        .collect();
    assert_all_succeeded(&finish(children));

    let text = std::fs::read_to_string(&file).expect("read");
    for i in 1..=WRITERS {
        assert!(
            text.contains(&format!("key{i} = {i}")),
            "key{i} lost:\n{text}"
        );
    }
    assert!(text.contains("keep = true"), "an existing key was lost");
}

#[test]
fn a1_parallel_config_set_of_distinct_keys_all_land() {
    let ws = tempfile::tempdir().expect("workspace");
    let pairs: &[(&str, &str)] = &[
        ("extract.max_sample_rows", "111"),
        ("extract.max_text_chars", "222222"),
        ("extract.max_sheets", "33"),
        ("extract.max_slides", "44"),
        ("lock.acquire_timeout_ms", "6000"),
        ("retry.base_ms", "60"),
        ("retry.max_ms", "500"),
        ("retry.max_elapsed_ms", "3000"),
        ("write.confirm_large_bytes", "204800"),
        ("fuzzy.max_content_bytes", "5000000"),
        ("search.max_inflight_bytes", "9437184"),
        ("concurrency.max_threads", "7"),
        ("concurrency.ram_fraction_percent", "40"),
        ("fuzzy.threshold", "0.8"),
        ("fuzzy.threshold_aggressive", "0.6"),
        ("fuzzy.threshold_context", "0.75"),
    ];
    let children: Vec<Child> = pairs
        .iter()
        .map(|(key, value)| {
            let mut cmd = base_command(ws.path());
            cmd.args(["config", "set", key, value, "--scope", "workspace"]);
            spawn(cmd)
        })
        .collect();
    assert_all_succeeded(&finish(children));

    let text = std::fs::read_to_string(ws.path().join(".atomwrite.toml")).expect("config file");
    for (key, value) in pairs {
        let field = key.split('.').nth(1).expect("section.field");
        assert!(
            text.lines()
                .any(|l| { l.trim().starts_with(field) && l.contains(&format!("= {value}")) }),
            "{key} = {value} lost:\n{text}"
        );
    }
}

#[test]
fn a1_writer_with_a_stale_base_gets_state_drift_and_writes_nothing() {
    let ws = tempfile::tempdir().expect("workspace");
    let file = ws.path().join("f.txt");
    std::fs::write(&file, "first\n").expect("seed");
    let base = atomwrite::checksum::hash_bytes(b"first\n");
    let opts = AtomicWriteOptions {
        backup: false,
        ..AtomicWriteOptions::default()
    };

    // Another writer got there first: the file no longer matches the base.
    std::fs::write(&file, "second\n").expect("concurrent update");
    let Err(err) = atomic_write_based(&file, b"mine\n", &opts, ws.path(), Some(&base)) else {
        panic!("a stale base must be refused");
    };
    let drift = err
        .downcast_ref::<atomwrite::error::AtomwriteError>()
        .expect("typed error");
    assert!(
        matches!(drift, atomwrite::error::AtomwriteError::StateDrift { .. }),
        "expected StateDrift, got {drift:?}"
    );
    assert!(drift.is_retryable(), "drift is retryable");
    assert_eq!(drift.exit_code(), 82);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "second\n");

    // A fresh base writes.
    let fresh = atomwrite::checksum::hash_bytes(b"second\n");
    atomic_write_based(&file, b"mine\n", &opts, ws.path(), Some(&fresh)).expect("fresh base");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "mine\n");
}
