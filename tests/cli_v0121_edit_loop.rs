// SPDX-License-Identifier: MIT OR Apache-2.0
//! v0.1.22 ADR-0039 — `edit-loop` subcommand regression tests.
//!
//! NOTE: `edit-loop` locks `std::io::stdin()` (the inherited handle), not a
//! per-call `Stdio::piped()` pipe. This means we cannot use
//! `assert_cmd::Command::write_stdin` because that only writes to a fresh
//! pipe; the child will still hold the parent's stdin open and `read_to_string`
//! will block forever. We must use `std::process::Command` with
//! `Stdio::piped()` AND explicitly drop the stdin handle to signal EOF.

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

fn run_edit_loop(
    dir: &std::path::Path,
    target: &std::path::Path,
    ndjson: &str,
    extra: &[&str],
) -> std::process::Output {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut cmd = Command::new(&bin);
    cmd.args(["--workspace", dir.to_str().unwrap(), "edit-loop"])
        .arg(target)
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("spawn edit-loop");
    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(ndjson.as_bytes());
        let _ = stdin.flush();
    }
    // Drop stdin to send EOF so the child's read_to_string returns.
    drop(child.stdin.take());

    child.wait_with_output().expect("wait edit-loop")
}

#[test]
fn edit_loop_applies_all_pairs_atomically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "alpha beta gamma\n").expect("seed");

    let ndjson = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n{\"old\":\"beta\",\"new\":\"BETA\"}\n{\"old\":\"gamma\",\"new\":\"GAMMA\"}\n";

    let output = run_edit_loop(dir.path(), &target, ndjson, &[]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "ALPHA BETA GAMMA\n");
}

#[test]
fn edit_loop_reports_unmatched_pairs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "alpha\n").expect("seed");

    let ndjson = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n{\"old\":\"missing\",\"new\":\"X\"}\n";

    let output = run_edit_loop(dir.path(), &target, ndjson, &[]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let ndjson_out = common::parse_ndjson(&output.stdout);
    let summary = ndjson_out
        .iter()
        .find(|e| e["type"] == "result")
        .expect("summary");
    assert_eq!(summary["pairs_total"], 2);
    assert_eq!(summary["pairs_applied"], 1);
    assert_eq!(summary["pairs_unmatched"], 1);
    assert_eq!(summary["action"], "edit_loop");
}

#[test]
fn edit_loop_with_backup_keeps_backup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "original\n").expect("seed");

    let ndjson = "{\"old\":\"original\",\"new\":\"modified\"}\n";

    let output = run_edit_loop(dir.path(), &target, ndjson, &["--backup", "--keep-backup"]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let count = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("file.txt.bak."))
        })
        .count();
    assert!(count >= 1, "esperava ≥1 backup preservado, got {count}");

    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "modified\n");
}

#[test]
fn edit_loop_50_pairs_all_applied() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    let mut content = String::new();
    let mut input = String::new();
    for i in 0..50 {
        content.push_str(&format!("line_{i}_old\n"));
        input.push_str(&format!(r#"{{"old":"line_{i}_old","new":"line_{i}_new"}}"#,));
        input.push('\n');
    }
    std::fs::write(&target, &content).expect("seed");

    let output = run_edit_loop(dir.path(), &target, &input, &[]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let final_content = std::fs::read_to_string(&target).expect("read");
    assert!(final_content.contains("line_0_new"));
    assert!(final_content.contains("line_49_new"));
    assert!(!final_content.contains("line_0_old"));
    assert!(!final_content.contains("line_49_old"));

    let ndjson_out = common::parse_ndjson(&output.stdout);
    let summary = ndjson_out
        .iter()
        .find(|e| e["type"] == "result")
        .expect("summary");
    assert_eq!(summary["pairs_total"], 50);
    assert_eq!(summary["pairs_applied"], 50);
}

#[test]
fn edit_loop_empty_file_reports_unmatched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("empty.txt");
    std::fs::write(&target, "").expect("seed");

    let ndjson = "{\"old\":\"foo\",\"new\":\"bar\"}\n";

    let output = run_edit_loop(dir.path(), &target, ndjson, &[]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ndjson_out = common::parse_ndjson(&output.stdout);
    let summary = ndjson_out
        .iter()
        .find(|e| e["type"] == "result")
        .expect("summary");
    assert_eq!(summary["pairs_unmatched"], 1);
}

#[test]
fn edit_loop_malformed_ndjson_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "original\n").expect("seed");

    let bad_input = "this is not JSON\n";

    let output = run_edit_loop(dir.path(), &target, bad_input, &[]);
    assert!(
        !output.status.success(),
        "NDJSON malformado deve falhar, got exit={:?}",
        output.status.code()
    );
}

#[test]
fn edit_loop_target_not_found_exits_nonzero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("nope.txt");

    let ndjson = "{\"old\":\"a\",\"new\":\"b\"}\n";

    let output = run_edit_loop(dir.path(), &target, ndjson, &[]);
    assert!(
        !output.status.success(),
        "alvo ausente deve falhar, got exit={:?}",
        output.status.code()
    );
}

#[test]
fn edit_loop_pair_results_have_matched_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "alpha\n").expect("seed");

    let ndjson = "{\"old\":\"alpha\",\"new\":\"A\"}\n{\"old\":\"missing\",\"new\":\"X\"}\n";

    let output = run_edit_loop(dir.path(), &target, ndjson, &[]);
    assert!(output.status.success());
    let ndjson_out = common::parse_ndjson(&output.stdout);
    let summary = ndjson_out
        .iter()
        .find(|e| e["type"] == "result")
        .expect("summary");
    let results = summary["pair_results"]
        .as_array()
        .expect("pair_results array");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["index"], 1);
    assert_eq!(results[0]["matched"], true);
    assert_eq!(results[1]["index"], 2);
    assert_eq!(results[1]["matched"], false);
}

/// ADR-0034 wiring: --allow-sequential-drift flag must have a regression test.
#[test]
fn edit_loop_allow_sequential_drift_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "alpha beta\n").expect("seed");

    let ndjson = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n";
    let output = run_edit_loop(dir.path(), &target, ndjson, &["--allow-sequential-drift"]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// ADR-0034 wiring: --retention flag must have a regression test.
#[test]
fn edit_loop_retention_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "alpha\n").expect("seed");

    let ndjson = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n";
    let output = run_edit_loop(
        dir.path(),
        &target,
        ndjson,
        &["--backup", "--retention", "3"],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// ADR-0034 wiring: --syntax-check flag must have a regression test.
#[test]
fn edit_loop_syntax_check_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "alpha\n").expect("seed");

    let ndjson = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n";
    let output = run_edit_loop(
        dir.path(),
        &target,
        ndjson,
        &["--syntax-check", "plaintext"],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// ADR-0034 wiring: --line-ending flag must have a regression test.
#[test]
fn edit_loop_line_ending_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("file.txt");
    std::fs::write(&target, "alpha beta\n").expect("seed");

    let ndjson = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n";
    let output = run_edit_loop(dir.path(), &target, ndjson, &["--line-ending", "lf"]);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}
