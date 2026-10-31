// SPDX-License-Identifier: MIT OR Apache-2.0

//! A reader that closed the pipe is exit 141 for EVERY way the product writes
//! to stdout, never a panic (101/134) and never a report file left in the temp
//! directory. The pipe is created here with its read end already closed, so the
//! child's first write fails with `EPIPE` deterministically: no timing involved.

#![cfg(unix)]

mod common;

use std::process::{Command, Stdio};

/// Every output path of the product that writes to stdout by itself.
const OUTPUTS: &[&[&str]] = &[
    &["completions", "bash"],
    &["completions", "zsh"],
    &["completions", "fish"],
    &["completions", "powershell"],
    &["completions", "elvish"],
    &["--print-schema"],
    &["--print-schema", "error"],
    &["--print-schema", "search"],
    &["--json-schema", "search"],
    &["agent-surface"],
    &["commands"],
    &["doctor"],
    &["--help"],
    &["search", "--help"],
    &["completions", "--help"],
    &["--version"],
    &["not-a-command"],
];

/// Run `args` with a stdout whose reader is already gone.
///
/// Returns the exit status, the stderr text and the files left in the child's
/// temp directory.
fn run_with_closed_reader(args: &[&str]) -> (Option<i32>, String, Vec<String>) {
    let temp = tempfile::tempdir().expect("temp dir");
    let (reader, writer) = std::io::pipe().expect("pipe");
    drop(reader);
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = common::hermetic(Command::new(bin))
        .env("TMPDIR", temp.path())
        .args(args)
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .expect("run atomwrite");
    let left: Vec<String> = std::fs::read_dir(temp.path())
        .expect("read temp dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        left,
    )
}

#[test]
fn every_stdout_path_exits_141_without_a_panic_or_a_report_file() {
    let mut failures = Vec::new();
    for args in OUTPUTS {
        let (code, stderr, left) = run_with_closed_reader(args);
        if code != Some(141) {
            failures.push(format!("{args:?}: exit {code:?}, stderr: {stderr}"));
        }
        if stderr.contains("panicked") {
            failures.push(format!("{args:?}: panicked: {stderr}"));
        }
        if !left.is_empty() {
            failures.push(format!("{args:?}: left {left:?} in the temp dir"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_reader_that_stays_open_still_gets_the_full_answer() {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = common::hermetic(Command::new(bin))
        .args(["completions", "bash"])
        .output()
        .expect("run atomwrite");
    assert_eq!(output.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("atomwrite"),
        "the completion script must reach an open reader"
    );
}
