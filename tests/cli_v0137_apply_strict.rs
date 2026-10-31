// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 — `apply` never writes what it could not match.
//!
//! A unified hunk used to splice the patch's own context in place of the file's
//! lines without comparing them, so a patch made for another version of the
//! file erased code with exit 0. SEARCH/REPLACE used to drop a block it could
//! not find and still report success. Now an unmatched hunk or block fails the
//! whole patch with `MATCH_FAILED` (exit 65) and the file is untouched.

mod common;

use serde_json::Value;

const ORIGINAL: &str = "alpha\nbeta needle\ngamma\n";

/// Run `apply` on `a.txt` (seeded with [`ORIGINAL`]) and return
/// `(exit code, parsed stdout, file content afterwards)`.
fn apply(patch: &str, extra: &[&str]) -> (Option<i32>, Vec<Value>, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "a.txt", ORIGINAL);
    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "apply"])
        .args(extra)
        .arg("--no-backup")
        .arg(&file)
        .write_stdin(patch)
        .output()
        .expect("run apply");
    let content = std::fs::read_to_string(&file).expect("read back");
    (
        output.status.code(),
        common::parse_ndjson(&output.stdout),
        content,
    )
}

fn assert_match_failed(code: Option<i32>, events: &[Value], content: &str) -> String {
    assert_eq!(code, Some(65), "events: {events:?}");
    let err = events.last().expect("an error envelope");
    assert_eq!(err["ok"], false, "{err}");
    assert_eq!(err["code"], "MATCH_FAILED", "{err}");
    assert_eq!(content, ORIGINAL, "a failed apply must not touch the file");
    err["message"].as_str().unwrap_or_default().to_owned()
}

#[test]
fn removed_line_that_is_not_in_the_file_fails() {
    let (code, events, content) = apply("--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-zzz\n+yyy\n", &[]);
    let message = assert_match_failed(code, &events, &content);
    assert!(message.contains("hunk 1/1"), "{message}");
    assert!(message.contains("zzz"), "{message}");
}

#[test]
fn wrong_context_fails_and_keeps_the_file() {
    let patch = "--- a/a.txt\n+++ b/a.txt\n@@ -1,3 +1,3 @@\n qqq\n-beta needle\n+yyy\n rrr\n";
    let (code, events, content) = apply(patch, &[]);
    assert_match_failed(code, &events, &content);
}

#[test]
fn a_bad_second_hunk_applies_no_hunk_at_all() {
    let patch = "--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-alpha\n+ALPHA\n@@ -3 +3 @@\n-nope\n+x\n";
    let (code, events, content) = apply(patch, &[]);
    let message = assert_match_failed(code, &events, &content);
    assert!(message.contains("hunk 2/2"), "{message}");
}

#[test]
fn dry_run_reports_the_same_mismatch() {
    let (code, events, content) = apply(
        "--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-zzz\n+yyy\n",
        &["--dry-run"],
    );
    assert_match_failed(code, &events, &content);
}

#[test]
fn a_correct_patch_still_applies() {
    let patch = "--- a/a.txt\n+++ b/a.txt\n@@ -1,3 +1,3 @@\n alpha\n-beta needle\n+BETA\n gamma\n";
    let (code, events, content) = apply(patch, &[]);
    assert_eq!(code, Some(0), "events: {events:?}");
    assert_eq!(content, "alpha\nBETA\ngamma\n");
    let applied = events
        .iter()
        .find(|e| e["type"] == "applied")
        .expect("applied");
    assert_eq!(applied["hunks_applied"], 1);
}

#[test]
fn a_header_a_few_lines_off_still_finds_its_place() {
    let patch = "--- a/a.txt\n+++ b/a.txt\n@@ -9 +9 @@\n-beta needle\n+BETA\n";
    let (code, _, content) = apply(patch, &[]);
    assert_eq!(code, Some(0));
    assert_eq!(content, "alpha\nBETA\ngamma\n");
}

#[test]
fn search_replace_with_one_missing_block_fails_and_keeps_the_file() {
    let patch = "<<<<<<< SEARCH\nalpha\n=======\nALPHA\n>>>>>>> REPLACE\n\
                 <<<<<<< SEARCH\nCOMPLETELY ABSENT BLOCK TEXT 12345\n=======\nX\n>>>>>>> REPLACE\n";
    let (code, events, content) = apply(patch, &["--format", "search-replace"]);
    let message = assert_match_failed(code, &events, &content);
    assert!(message.contains("1 of 2"), "{message}");
}

#[test]
fn search_replace_with_every_block_present_applies() {
    let patch = "<<<<<<< SEARCH\nalpha\n=======\nALPHA\n>>>>>>> REPLACE\n\
                 <<<<<<< SEARCH\ngamma\n=======\nGAMMA\n>>>>>>> REPLACE\n";
    let (code, _, content) = apply(patch, &["--format", "search-replace"]);
    assert_eq!(code, Some(0));
    assert_eq!(content, "ALPHA\nbeta needle\nGAMMA\n");
}
