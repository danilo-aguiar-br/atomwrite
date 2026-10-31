// SPDX-License-Identifier: MIT OR Apache-2.0

//! `search` must report what reached stdout, not what the walker scanned.
//!
//! The producer tallied every file it opened, while the consumer discarded the
//! blocks that arrived after `--limit` filled. The summary then published the
//! walker's work as if it were the answer: `--limit 1` printed one `match` and
//! claimed twenty-nine. These tests hold the summary against the records that
//! really left, in every output mode the verb offers.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_atomwrite"))
}

/// A tree of `n` files, every one of them matching `needle` on one line.
fn tree(n: usize) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 1..=n {
        fs::write(
            dir.path().join(format!("f{i:02}.txt")),
            format!("needle {i}\n"),
        )
        .expect("write fixture");
    }
    dir
}

struct Out {
    events: Vec<serde_json::Value>,
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
        .map(|l| serde_json::from_str(l).expect("stdout must be NDJSON only"))
        .collect();
    Out { events }
}

fn count(out: &Out, kind: &str) -> u64 {
    out.events.iter().filter(|e| e["type"] == kind).count() as u64
}

fn summary(out: &Out) -> &serde_json::Value {
    out.events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("search always closes with a summary")
}

#[test]
fn the_summary_matches_the_matches_that_left_under_a_budget() {
    let dir = tree(40);
    for limit in ["1", "2", "3"] {
        let out = run(&["search", "needle", ".", "--limit", limit], dir.path());
        let emitted = count(&out, "match");
        let last = summary(&out);
        assert_eq!(
            emitted,
            limit.parse::<u64>().expect("limit is a number"),
            "--limit {limit} must print exactly that many matches"
        );
        assert_eq!(
            last["total_matches"].as_u64(),
            Some(emitted),
            "--limit {limit}: the summary must count the matches it printed, got {last}"
        );
        assert_eq!(
            last["files_matched"].as_u64(),
            Some(emitted),
            "--limit {limit}: one match per file, so one matched file, got {last}"
        );
        assert_eq!(
            last["walker_stopped_early"], true,
            "--limit {limit} must name the truncation"
        );
    }
}

#[test]
fn the_summary_matches_the_matches_that_left_when_sorted() {
    let dir = tree(12);
    let out = run(
        &["search", "needle", ".", "--sort", "path", "--limit", "2"],
        dir.path(),
    );
    let emitted = count(&out, "match");
    assert_eq!(emitted, 2, "the sorted path emits the same two matches");
    let last = summary(&out);
    assert_eq!(last["total_matches"].as_u64(), Some(2), "got {last}");
    assert_eq!(last["files_matched"].as_u64(), Some(2), "got {last}");
    // `--sort` buffers the whole tree before it emits anything, so the walker
    // reaches the end of the tree and never quits early. The field says "the
    // walker itself quit before the tree ended", and stamping `true` here would
    // be the exact lie the field exists to prevent.
    assert_ne!(
        last["walker_stopped_early"], true,
        "the sorted path buffers the tree, so the walker never stopped early: {last}"
    );
}

#[test]
fn count_mode_reports_the_files_it_printed() {
    // A tree big enough that the walker cannot reach the end before `--limit`
    // cuts it: with twelve tiny files the walker sometimes finished first, the
    // summary honestly omitted `walker_stopped_early`, and the assertion failed
    // under load.
    let dir = tree(400);
    let out = run(
        &["search", "needle", ".", "--count", "--limit", "2"],
        dir.path(),
    );
    let printed = count(&out, "count");
    assert_eq!(printed, 2, "--count --limit 2 prints two count lines");
    let last = summary(&out);
    assert_eq!(last["files_matched"].as_u64(), Some(2), "got {last}");
    assert_eq!(last["total_matches"].as_u64(), Some(2), "got {last}");
    assert_eq!(last["walker_stopped_early"], true);
}

#[test]
fn files_mode_reports_the_files_it_printed() {
    // Same reason as the count test: the walker has to be cut by the limit, not
    // finish the tree on its own.
    let dir = tree(400);
    let out = run(
        &["search", "needle", ".", "--files", "--limit", "3"],
        dir.path(),
    );
    let printed = count(&out, "file");
    assert_eq!(printed, 3, "--files --limit 3 prints three file lines");
    let last = summary(&out);
    assert_eq!(last["files_matched"].as_u64(), Some(3), "got {last}");
    assert_eq!(last["total_matches"].as_u64(), Some(3), "got {last}");
    assert_eq!(last["walker_stopped_early"], true);
}

#[test]
fn an_unbounded_run_reports_the_whole_tree() {
    let dir = tree(12);
    let out = run(&["search", "needle", "."], dir.path());
    let emitted = count(&out, "match");
    assert_eq!(emitted, 12, "every file matches once");
    let last = summary(&out);
    assert_eq!(last["files_matched"].as_u64(), Some(12), "got {last}");
    assert_eq!(last["total_matches"].as_u64(), Some(12), "got {last}");
    assert!(
        last.get("walker_stopped_early").is_none(),
        "a walk that ran to the end must not claim it stopped early"
    );
}

#[test]
fn offset_pages_the_content_results() {
    let dir = tree(12);
    let out = run(&["search", "needle", ".", "--offset", "5"], dir.path());
    let emitted = count(&out, "match");
    assert_eq!(emitted, 7, "--offset 5 skips five of the twelve matches");
    let last = summary(&out);
    assert_eq!(last["total_matches"].as_u64(), Some(7), "got {last}");
    assert_eq!(last["files_matched"].as_u64(), Some(7), "got {last}");
}

#[test]
fn offset_and_limit_page_together() {
    // Same condition as the count and files modes: the walker has to be cut by
    // the limit instead of reaching the end of a tree it can finish first.
    let dir = tree(400);
    let out = run(
        &["search", "needle", ".", "--offset", "5", "--limit", "3"],
        dir.path(),
    );
    let emitted = count(&out, "match");
    assert_eq!(emitted, 3, "page two of size three");
    let last = summary(&out);
    assert_eq!(last["total_matches"].as_u64(), Some(3), "got {last}");
    assert_eq!(last["walker_stopped_early"], true);
}

#[test]
fn an_offset_past_the_end_reports_nothing() {
    let dir = tree(4);
    let out = run(&["search", "needle", ".", "--offset", "10"], dir.path());
    assert_eq!(count(&out, "match"), 0, "the page is past the answer");
    let last = summary(&out);
    assert_eq!(last["total_matches"].as_u64(), Some(0), "got {last}");
    assert_eq!(last["files_matched"].as_u64(), Some(0), "got {last}");
}
