// SPDX-License-Identifier: MIT OR Apache-2.0

//! `hash` always closes with a summary, and the summary names truncation.
//!
//! Without a terminal record a truncated list of hashes is indistinguishable
//! from a complete one: the consumer sees a stream that simply stops. These
//! tests require the closing record on every run, bounded or not.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_atomwrite"))
}

fn tree(n: usize) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 1..=n {
        fs::write(
            dir.path().join(format!("f{i:02}.txt")),
            format!("content {i}\n"),
        )
        .expect("write fixture");
    }
    dir
}

struct Out {
    events: Vec<serde_json::Value>,
    code: i32,
}

fn run(args: &[&str], cwd: &Path) -> Out {
    let output = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn atomwrite");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let events = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("stdout must be NDJSON"))
        .collect();
    Out {
        events,
        code: output.status.code().unwrap_or(-1),
    }
}

fn hashes(out: &Out) -> u64 {
    out.events.iter().filter(|e| e["type"] == "hash").count() as u64
}

#[test]
fn a_single_file_ends_with_a_summary() {
    let dir = tree(1);
    let out = run(&["hash", "f01.txt"], dir.path());
    let last = out.events.last().expect("a terminal record");
    assert_eq!(
        last["type"], "summary",
        "hash must always close with a summary"
    );
    assert_eq!(last["files_visited"], 1);
    assert_eq!(last["files_matched"], 1);
    assert_eq!(out.code, 0);
}

#[test]
fn a_tree_ends_with_a_summary() {
    let dir = tree(5);
    let out = run(&["hash", ".", "-r"], dir.path());
    let last = out.events.last().expect("a terminal record");
    assert_eq!(last["type"], "summary");
    assert_eq!(last["files_visited"], 5);
    assert_eq!(
        last["files_matched"], 5,
        "the summary counts what the run hashed"
    );
    assert!(
        last.get("walker_stopped_early").is_none(),
        "a complete walk must not claim truncation"
    );
    assert_eq!(hashes(&out), 5);
}

#[test]
fn a_budgeted_tree_names_the_truncation() {
    let dir = tree(12);
    let out = run(&["hash", ".", "-r", "--limit", "3"], dir.path());
    let last = out.events.last().expect("a terminal record");
    assert_eq!(last["type"], "summary");
    assert_eq!(hashes(&out), 3, "the budget allows three records");
    assert_eq!(last["files_visited"], 3);
    assert_eq!(
        last["walker_stopped_early"], true,
        "a truncated list must say it was truncated"
    );
}

#[test]
fn verify_with_a_wrong_checksum_still_names_the_run() {
    let dir = tree(1);
    let out = run(
        &["hash", "f01.txt", "--verify", &"0".repeat(64)],
        dir.path(),
    );
    let last = out.events.last().expect("a terminal record");
    assert_eq!(
        last["ok"], false,
        "a failed verification must say so on the last line, got {last}"
    );
    assert!(
        last.get("exit").is_some(),
        "the terminal of a failed run carries the exit status"
    );
}
