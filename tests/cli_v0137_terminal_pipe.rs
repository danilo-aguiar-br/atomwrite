// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 broken-pipe contract: a reader that closes early (`| head -1`) ends
//! the run with exit 141 through the normal error path, not by a kill. The
//! SIGPIPE handler raises the cooperative cancel, the write that hit the closed
//! pipe fails with `BrokenPipe`, every `Drop` runs, and no tempfile, lock
//! sidecar or journal stays behind. Each scenario repeats to catch flakiness.

#![cfg(unix)]

mod common;

use std::io::{BufRead, BufReader};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

/// Files created per run; the output has to outgrow the pipe buffer (64 KiB on
/// Linux) so the writer is still producing when the reader closes.
const FILE_COUNT: usize = 2000;

/// Repetitions of each scenario.
const RUNS: usize = 20;

/// Content of every fixture file before `replace`, and after it.
const OLD_CONTENT: &str = "needle\n";
const NEW_CONTENT: &str = "thread\n";

fn file_name(i: usize) -> String {
    format!("file_with_a_long_enough_name_{i:05}.txt")
}

/// Entries a finished run must not leave: tempfiles, lock sidecars, journals.
fn residue(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read_dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(".atomwrite")
                || name.ends_with(".atomwrite.lock")
                || name.ends_with(".tmp")
            {
                found.push(entry.path().display().to_string());
            }
            if entry.file_type().expect("file type").is_dir() {
                pending.push(entry.path());
            }
        }
    }
    found.sort();
    found
}

/// Run `args`, read ONE line of stdout, close the pipe and wait for the exit.
///
/// stderr is captured too: a closed reader is not a defect, so the run must not
/// leave a diagnostic line behind for it.
fn run_closed_after_first_line(ws: &Path, args: &[&str]) -> (ExitStatus, String, String) {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = common::hermetic(Command::new(bin))
        .args([
            "--workspace",
            ws.to_str().expect("utf8 workspace"),
            "--no-progress",
            "--no-auto-heal",
        ])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn atomwrite");
    // stderr is drained on its own thread: a pipe that filled up would block
    // the child before it could exit, and the wait below would never return.
    let stderr = child.stderr.take().expect("piped stderr");
    let drain = std::thread::spawn(move || {
        let mut text = String::new();
        let mut reader = BufReader::new(stderr);
        let _ = std::io::Read::read_to_string(&mut reader, &mut text);
        text
    });
    let stdout = child.stdout.take().expect("piped stdout");
    let mut first = String::new();
    BufReader::new(stdout)
        .read_line(&mut first)
        .expect("first line");
    // The reader is dropped here: the pipe closes with the child still writing.
    let status = child.wait().expect("wait atomwrite");
    let diagnostics = drain.join().expect("stderr drain");
    (status, first, diagnostics)
}

fn populate(dir: &Path, content: &str) {
    for i in 0..FILE_COUNT {
        std::fs::write(dir.join(file_name(i)), content).expect("fixture");
    }
}

/// A graceful end: exit 141 by `exit`, never a kill by signal 13.
fn assert_graceful_141(status: &ExitStatus, label: &str, run: usize) {
    assert_eq!(status.code(), Some(141), "{label} run {run}: {status:?}");
    assert_eq!(
        status.signal(),
        None,
        "{label} run {run}: killed by a signal, destructors skipped"
    );
}

/// A closed reader is not a defect: nothing about the data channel reaches stderr.
fn assert_silent_stderr(diagnostics: &str, label: &str, run: usize) {
    assert!(
        !diagnostics.contains("data channel write failed"),
        "{label} run {run}: the closed reader was reported as a failure: {diagnostics}"
    );
}

#[test]
fn list_closed_after_the_first_line_exits_141_without_residue() {
    let dir = tempfile::tempdir().expect("tempdir");
    populate(dir.path(), "x\n");
    let path = dir.path().to_str().expect("utf8 path");

    for run in 0..RUNS {
        let (status, first, diagnostics) = run_closed_after_first_line(dir.path(), &["list", path]);
        assert!(!first.is_empty(), "list must have written a first line");
        assert_graceful_141(&status, "list", run);
        assert_silent_stderr(&diagnostics, "list", run);
        assert_eq!(residue(dir.path()), Vec::<String>::new(), "list run {run}");
    }
}

#[test]
fn replace_closed_after_the_first_line_exits_141_without_residue() {
    for run in 0..RUNS {
        let dir = tempfile::tempdir().expect("tempdir");
        populate(dir.path(), OLD_CONTENT);

        let (status, first, diagnostics) = run_closed_after_first_line(
            dir.path(),
            &[
                "replace",
                "--no-backup",
                "needle",
                "thread",
                dir.path().to_str().expect("utf8 path"),
            ],
        );

        assert!(!first.is_empty(), "replace must have written a first line");
        assert_graceful_141(&status, "replace", run);
        assert_silent_stderr(&diagnostics, "replace", run);
        assert_eq!(
            residue(dir.path()),
            Vec::<String>::new(),
            "replace run {run}"
        );
        for i in 0..FILE_COUNT {
            let path = dir.path().join(file_name(i));
            let content = std::fs::read_to_string(&path).expect("target readable");
            assert!(
                content == OLD_CONTENT || content == NEW_CONTENT,
                "run {run}: {} holds a partial write {content:?}",
                path.display()
            );
        }
    }
}

#[test]
fn search_closed_after_the_first_line_exits_141_without_residue() {
    let dir = tempfile::tempdir().expect("tempdir");
    populate(dir.path(), OLD_CONTENT);
    let path = dir.path().to_str().expect("utf8 path");

    for run in 0..RUNS {
        let (status, first, diagnostics) =
            run_closed_after_first_line(dir.path(), &["search", "needle", path]);
        assert!(!first.is_empty(), "search must have written a first line");
        assert_graceful_141(&status, "search", run);
        assert_silent_stderr(&diagnostics, "search", run);
        assert_eq!(
            residue(dir.path()),
            Vec::<String>::new(),
            "search run {run}"
        );
    }
}

#[test]
fn read_closed_pipe_exits_141_instead_of_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Far more than a pipe holds, so the write blocks until the reader is gone.
    let body = "line of text for the broken pipe test\n".repeat(20_000);
    let target = common::create_test_file(dir.path(), "big.txt", &body);

    for run in 0..RUNS {
        let (status, first, diagnostics) = run_closed_after_first_line(
            dir.path(),
            &[
                "read",
                "--format",
                "raw",
                // v0.1.38 stops `read` at 24.000 bytes, which fits inside the
                // pipe buffer and would let the run finish before the reader
                // closes. `--all` asks for the whole stream, which is the case
                // this contract is about.
                "--all",
                target.to_str().expect("utf8 path"),
            ],
        );
        assert!(!first.is_empty(), "read must have written a first line");
        assert_graceful_141(&status, "read", run);
        assert_silent_stderr(&diagnostics, "read", run);
    }
}
