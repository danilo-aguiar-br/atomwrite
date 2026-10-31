// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 — a successful write always releases its WAL journal.
//!
//! Before the fix the H3 rate limit was consulted when the journal guard was
//! dropped, so after ten journals in a minute every following one outlived its
//! successful write (110 residual files for a 120-file `replace`). H3 now only
//! throttles journal creation; release after commit is unconditional, and a
//! failed write still keeps its journal for recovery.

mod common;

use std::path::Path;

/// Files seeded for the multi-file replace; well above the H3 window of 10.
const FILE_COUNT: usize = 40;

/// Names of every `.atomwrite.journal.*` entry under `root`.
fn journals_under(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read_dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(".atomwrite.journal."))
            {
                found.push(path.display().to_string());
            }
        }
    }
    found
}

#[test]
fn replace_of_many_files_leaves_no_journal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).expect("mkdir");
    for i in 0..FILE_COUNT {
        std::fs::write(src.join(format!("f{i}.txt")), format!("hello foo {i}\n")).expect("seed");
    }

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--no-progress",
            "replace",
            "--no-backup",
            "foo",
            "bar",
            src.to_str().unwrap(),
        ])
        .output()
        .expect("replace");
    assert!(
        output.status.success(),
        "replace failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for i in 0..FILE_COUNT {
        let text = std::fs::read_to_string(src.join(format!("f{i}.txt"))).expect("read back");
        assert!(text.contains("bar"), "file {i} was not rewritten: {text:?}");
    }
    let left = journals_under(dir.path());
    assert!(
        left.is_empty(),
        "a successful replace must not leave journals, found {}: {left:?}",
        left.len()
    );
}

#[test]
fn failed_write_keeps_its_journal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("broken.json");

    // `--wal-policy always` opens the journal before the syntax check, and the
    // unbalanced bracket makes the write fail after the journal exists.
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--wal-policy",
            "always",
            "--syntax-check",
            "--no-backup",
            target.to_str().unwrap(),
        ])
        .write_stdin("{\"a\": [1, 2")
        .output()
        .expect("write");
    assert!(!output.status.success(), "the syntax check must reject it");

    let journal = dir
        .path()
        .join(".atomwrite.journal.broken.json.atomwrite.journal.json");
    assert!(
        journal.exists(),
        "a failed write must keep its journal for recovery"
    );
}
