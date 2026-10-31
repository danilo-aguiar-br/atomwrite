// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 write limits: `--max-filesize` bounds the payload AND the target.
//!
//! Before this, `write` only looked at `--max-size` for stdin (which defaults to
//! nothing) and only at `[write].confirm_large_bytes` for the target, so a
//! 5000-byte payload passed a 1000-byte ceiling and a 5000-byte overwrite did
//! the same. Both are now refused with `FILE_TOO_LARGE`, and a verb-local
//! `--max-size` still narrows the payload further.

mod common;

use std::io::Write as _;
use std::process::{Command, Stdio};

/// Run `atomwrite` with `args`, feeding `stdin` to it, and return the output.
fn run_with_stdin(ws: &std::path::Path, args: &[&str], stdin: &[u8]) -> std::process::Output {
    let mut child = common::hermetic(Command::new(assert_cmd::cargo::cargo_bin("atomwrite")))
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn atomwrite");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(stdin)
        .expect("write stdin");
    child.wait_with_output().expect("wait atomwrite")
}

fn error_code(output: &std::process::Output) -> String {
    let events = common::parse_ndjson(&output.stdout);
    let err = events
        .iter()
        .find(|e| e["error"] == true)
        .expect("error event");
    assert_eq!(err["exit"], 65, "{err}");
    err["code"].as_str().unwrap_or_default().to_string()
}

#[test]
fn write_stdin_above_max_filesize_exits_65() {
    let dir = tempfile::tempdir().expect("tempdir");
    let payload = vec![b'a'; 5000];
    let output = run_with_stdin(
        dir.path(),
        &["--max-filesize", "1000", "write", "out.txt"],
        &payload,
    );
    assert_eq!(
        output.status.code(),
        Some(65),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(error_code(&output), "FILE_TOO_LARGE");
    assert!(!dir.path().join("out.txt").exists(), "nothing is written");
}

#[test]
fn write_target_above_max_filesize_exits_65() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("big.txt");
    std::fs::write(&target, "x".repeat(5000)).expect("fixture");
    let output = run_with_stdin(
        dir.path(),
        &["--max-filesize", "1000", "write", "big.txt"],
        b"small\n",
    );
    assert_eq!(
        output.status.code(),
        Some(65),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(error_code(&output), "FILE_TOO_LARGE");
    assert_eq!(
        std::fs::read_to_string(&target).expect("target readable"),
        "x".repeat(5000),
        "the oversized target is left untouched"
    );
}

#[test]
fn write_max_size_still_bounds_stdin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let payload = vec![b'a'; 5000];
    let output = run_with_stdin(
        dir.path(),
        &[
            "--max-filesize",
            "100000",
            "write",
            "--max-size",
            "1000",
            "out.txt",
        ],
        &payload,
    );
    assert_eq!(
        output.status.code(),
        Some(65),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // A verb-local `--max-size` is an argument mistake, not a host size limit.
    let events = common::parse_ndjson(&output.stdout);
    let err = events
        .iter()
        .find(|e| e["error"] == true)
        .expect("error event");
    assert_eq!(err["code"], "INVALID_INPUT", "{err}");
    assert!(!dir.path().join("out.txt").exists(), "nothing is written");
}

#[test]
fn write_append_still_honours_max_filesize() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("log.txt");
    std::fs::write(&target, "x".repeat(5000)).expect("fixture");
    let output = run_with_stdin(
        dir.path(),
        &["--max-filesize", "1000", "write", "--append", "log.txt"],
        b"more\n",
    );
    assert_eq!(
        output.status.code(),
        Some(65),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(error_code(&output), "FILE_TOO_LARGE");
    assert_eq!(
        std::fs::read_to_string(&target).expect("target readable"),
        "x".repeat(5000)
    );
}

#[test]
fn write_under_both_limits_still_writes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = run_with_stdin(
        dir.path(),
        &["--max-filesize", "1000", "write", "out.txt"],
        b"hello\n",
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("out.txt")).expect("written"),
        "hello\n"
    );
}
