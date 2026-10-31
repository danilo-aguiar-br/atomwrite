// SPDX-License-Identifier: MIT OR Apache-2.0

//! `sparse` must report the entries it actually emitted, not the ones it tried.
//!
//! The counter used to rise before the writer decided, so `--limit 1` over ten
//! files published `emitted: 10`. These tests hold `emitted` against the number
//! of entry lines that really left, under both a budget and no budget.

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
            format!("line one {i}\nline two\nline three\n"),
        )
        .expect("write fixture");
    }
    dir
}

struct Out {
    events: Vec<serde_json::Value>,
    #[allow(dead_code)]
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

fn entries(out: &Out) -> u64 {
    out.events
        .iter()
        .filter(|e| {
            e["type"]
                .as_str()
                .is_some_and(|t| t.starts_with("sparse_") && t != "sparse_summary")
        })
        .count() as u64
}

#[test]
fn emitted_matches_the_lines_that_left() {
    let dir = tree(10);
    let out = run(&["sparse", "list", "."], dir.path());
    let last = out.events.last().expect("a terminal record");
    assert_eq!(last["type"], "sparse_summary");
    assert_eq!(
        last["emitted"],
        entries(&out),
        "`emitted` must count emitted lines, not attempts"
    );
    assert_eq!(entries(&out), 10, "the whole tree is ten files");
}

#[test]
fn emitted_does_not_overcount_under_a_budget() {
    let dir = tree(10);
    let out = run(&["sparse", "list", ".", "--limit", "1"], dir.path());
    let last = out.events.last().expect("a terminal record");
    assert_eq!(
        last["emitted"],
        entries(&out),
        "`--limit 1` cannot report more entries than it published"
    );
    assert_eq!(entries(&out), 1, "the budget allows one entry");
    assert_eq!(
        last["walker_stopped_early"], true,
        "and the summary must name the truncation"
    );
}

#[test]
fn an_empty_tree_reports_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = run(&["sparse", "list", "."], dir.path());
    let last = out.events.last().expect("a terminal record");
    assert_eq!(last["emitted"], 0, "an empty tree emitted nothing");
    assert_eq!(entries(&out), 0);
}
