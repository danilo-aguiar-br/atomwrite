// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 — the implicit startup `wal-heal` is scoped.
//!
//! It used to walk the whole workspace on every run (entering `target/` and
//! `.git`), even for a read. It now runs only for a verb that mutates files,
//! only in the directories that verb names, and never enters `target/`,
//! `.git` or `node_modules`.

mod common;

use std::path::{Path, PathBuf};

/// A journal whose last entry is `Committed` long ago, so the heal reaps it.
const STALE_JOURNAL: &str = "{\"phase\":\"started\",\"op_id\":\"abc123\",\"started_at_unix\":1700000000}\n\
     {\"phase\":\"committed\",\"op_id\":\"abc123\",\"committed_at_unix\":1700000001}\n";

/// Seed a stale journal for `name` inside `dir` and return its path.
fn seed_journal(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("mkdir");
    let path = dir.join(format!(".atomwrite.journal.{name}.atomwrite.journal.json"));
    std::fs::write(&path, STALE_JOURNAL).expect("seed journal");
    path
}

#[test]
fn read_does_not_scan_the_workspace() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir");
    std::fs::write(dir.path().join("src/a.txt"), "hi\n").expect("seed");
    let stale = seed_journal(&dir.path().join("src"), "old.txt");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "src/a.txt",
        ])
        .output()
        .expect("read");
    assert!(output.status.success());
    assert!(
        stale.exists(),
        "a read-only verb must not run the startup heal"
    );
}

#[test]
fn directory_target_heals_inside_but_skips_target_dir_and_outside() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).expect("mkdir");
    std::fs::write(src.join("a.txt"), "hi\n").expect("seed");
    let inside = seed_journal(&src, "inside.txt");
    let nested = seed_journal(&src.join("deep"), "nested.txt");
    let in_target = seed_journal(&src.join("target/debug"), "built.txt");
    let in_git = seed_journal(&src.join(".git/objects"), "packed.txt");
    let outside = seed_journal(&dir.path().join("other"), "elsewhere.txt");

    // Zero matches (exit 1) is fine: the heal runs before the scan.
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--no-progress",
            "replace",
            "--no-backup",
            "no-such-text",
            "x",
            src.to_str().unwrap(),
        ])
        .output()
        .expect("replace");
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "unexpected exit: {:?}",
        output.status
    );

    assert!(
        !inside.exists(),
        "stale journal inside the target root is reaped"
    );
    assert!(
        !nested.exists(),
        "stale journal in a subdirectory is reaped"
    );
    assert!(in_target.exists(), "the heal must not enter `target/`");
    assert!(in_git.exists(), "the heal must not enter `.git`");
    assert!(outside.exists(), "the heal must not leave the named root");
}

#[test]
fn file_target_heals_only_its_own_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sub = dir.path().join("a");
    std::fs::create_dir_all(&sub).expect("mkdir");
    let sibling = seed_journal(&sub, "sibling.txt");
    let deeper = seed_journal(&sub.join("sub"), "deeper.txt");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--no-backup",
            sub.join("x.txt").to_str().unwrap(),
        ])
        .write_stdin("payload\n")
        .output()
        .expect("write");
    assert!(output.status.success(), "write failed: {:?}", output.status);

    assert!(
        !sibling.exists(),
        "stale journal beside the target is reaped"
    );
    assert!(deeper.exists(), "a file target heals its directory only");
}

#[test]
fn no_auto_heal_flag_disables_the_pass() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).expect("mkdir");
    let stale = seed_journal(&src, "inside.txt");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--no-auto-heal",
            "--no-progress",
            "replace",
            "--no-backup",
            "no-such-text",
            "x",
            src.to_str().unwrap(),
        ])
        .output()
        .expect("replace");
    assert!(matches!(output.status.code(), Some(0 | 1)));
    assert!(stale.exists(), "--no-auto-heal must skip the pass");
}
