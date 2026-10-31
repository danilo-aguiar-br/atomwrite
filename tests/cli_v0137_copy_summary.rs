// SPDX-License-Identifier: MIT OR Apache-2.0

//! `copy` always closes with a summary, and the summary names truncation.
//!
//! A copy whose record stream simply stops leaves the consumer unable to tell a
//! finished tree from an abandoned one. These tests require the closing record
//! and the `walker_stopped_early` flag that names a budget cut.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_atomwrite"))
}

/// One root holding `src` and a not-yet-created `out`, so the workspace jail
/// accepts both sides.
fn fixture(n: usize) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().expect("tempdir");
    let src = root.path().join("src");
    fs::create_dir(&src).expect("create src");
    for i in 1..=n {
        fs::write(src.join(format!("f{i:02}.txt")), format!("content {i}\n"))
            .expect("write fixture");
    }
    let dst = root.path().join("out");
    (root, src, dst)
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
    let code = output.status.code().unwrap_or(-1);
    Out { events, code }
}

fn copied(out: &Out) -> u64 {
    out.events.iter().filter(|e| e["type"] == "copied").count() as u64
}

#[test]
fn a_single_file_ends_with_a_summary() {
    let (root, src, dst_root) = fixture(1);
    let dst = dst_root.join("one.txt");
    let out = run(
        &[
            "copy",
            src.join("f01.txt").to_str().expect("utf8"),
            dst.to_str().expect("utf8"),
            "--workspace",
            root.path().to_str().expect("utf8"),
        ],
        root.path(),
    );
    let last = out.events.last().expect("a terminal record");
    assert_eq!(
        last["type"], "summary",
        "copy must always close with a summary"
    );
    assert_eq!(last["files_modified"], 1);
    assert_eq!(out.code, 0);
}

#[test]
fn a_tree_ends_with_a_summary() {
    let (root, src, dst) = fixture(4);
    let out = run(
        &[
            "copy",
            src.to_str().expect("utf8"),
            dst.to_str().expect("utf8"),
            "-r",
            "--workspace",
            root.path().to_str().expect("utf8"),
        ],
        root.path(),
    );
    let last = out.events.last().expect("a terminal record");
    assert_eq!(last["type"], "summary");
    assert_eq!(copied(&out), 4);
    assert_eq!(last["files_visited"], 4);
    assert!(
        last.get("walker_stopped_early").is_none(),
        "a complete copy must not claim truncation"
    );
}

#[test]
fn a_budgeted_copy_names_the_truncation() {
    let (root, src, dst) = fixture(8);
    let out = run(
        &[
            "copy",
            src.to_str().expect("utf8"),
            dst.to_str().expect("utf8"),
            "-r",
            "--limit",
            "2",
            "--workspace",
            root.path().to_str().expect("utf8"),
        ],
        root.path(),
    );
    let last = out.events.last().expect("a terminal record");
    assert_eq!(last["type"], "summary");
    assert_eq!(copied(&out), 2, "the budget allows two copies");
    assert_eq!(
        last["walker_stopped_early"], true,
        "a truncated copy must say it was truncated"
    );
    let on_disk = fs::read_dir(&dst)
        .expect("read dst")
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .count();
    assert_eq!(on_disk, 2, "the disk must hold exactly two files");
}
