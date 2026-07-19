// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.33: one-shot fuzzy replace — no hang when replacement embeds pattern.

use assert_cmd::Command;
use predicates::prelude::*;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn atomwrite() -> Command {
    Command::cargo_bin("atomwrite").unwrap()
}

/// Agent footgun: NEW contains OLD. Must finish in wall-clock seconds, apply once.
#[test]
fn replace_fuzzy_pattern_subset_of_replacement_terminates() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(
        &file,
        "## Section\n- old bullet alpha\n- old bullet beta\n",
    )
    .unwrap();

    let old = "## Section\n- old bullet alpha\n- old bullet beta";
    // NEW embeds OLD (expanded section) — previously infinite-looped.
    let new = "## Section\n- old bullet alpha\n- old bullet beta\n- new bullet gamma";

    let start = Instant::now();
    let output = atomwrite()
        .args([
            "--timeout-secs",
            "10",
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--fuzzy",
            "auto",
            "-F",
            "--no-backup",
            old,
            new,
            file.to_str().unwrap(),
        ])
        .output()
        .expect("run");
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "must be one-shot (<2s), took {elapsed:?}; status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("new bullet gamma"));
    assert_eq!(
        content.matches("new bullet gamma").count(),
        1,
        "must apply once, got:\n{content}"
    );
}

#[test]
fn replace_fuzzy_pattern_subset_high_max_replacements_still_one() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "TOKEN\n").unwrap();

    let start = Instant::now();
    let output = atomwrite()
        .args([
            "--timeout-secs",
            "10",
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--fuzzy",
            "auto",
            "-F",
            "--no-backup",
            "--max-replacements",
            "1000000",
            "TOKEN",
            "TOKEN_EXTRA",
            file.to_str().unwrap(),
        ])
        .output()
        .expect("run");
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(output.status.success());
    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(content, "TOKEN_EXTRA\n");
}

#[test]
fn help_default_timeout_documents_120() {
    atomwrite()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("120"));
}

#[test]
fn replace_exact_pattern_subset_one_pass() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "xxAxx\n").unwrap();
    let start = Instant::now();
    let output = atomwrite()
        .args([
            "--timeout-secs",
            "10",
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "-F",
            "--no-backup",
            "A",
            "BA",
            file.to_str().unwrap(),
        ])
        .output()
        .expect("run");
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(output.status.success());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "xxBAxx\n");
}
