// SPDX-License-Identifier: MIT OR Apache-2.0

//! A directory the walk cannot open is a hole, and a hole must be named.
//!
//! Every tree-scanning verb reports `files_failed` and `errors[]` when a
//! subtree is unreadable, so a total below the tree size is never read as the
//! tree size. On Unix the fixture makes the subdirectory mode `000`, which the
//! process cannot enter; the test skips elsewhere because the permission model
//! that produces the failure does not exist there.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_atomwrite"))
}

/// A tree with one readable file and one unreadable subtree.
///
/// The `inner` directory is left at mode `000` for the duration of the test and
/// is restored on drop so `tempfile` can delete the tree.
struct Blocked {
    dir: tempfile::TempDir,
    inner: PathBuf,
}

impl Drop for Blocked {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.inner, fs::Permissions::from_mode(0o755));
    }
}

fn blocked() -> Blocked {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), "hi\n").expect("write file");
    let inner = dir.path().join("inner");
    fs::create_dir(&inner).expect("create inner");
    fs::write(inner.join("b.txt"), "hi\n").expect("write inner file");
    fs::set_permissions(&inner, fs::Permissions::from_mode(0o000)).expect("chmod 000");
    Blocked { dir, inner }
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

fn terminal(out: &Out) -> &serde_json::Value {
    out.events.last().expect("a terminal record")
}

/// The run names the hole and its path, whatever its exit code.
fn assert_reports_the_hole(out: &Out, what: &str) {
    let last = terminal(out);
    assert_eq!(
        last["files_failed"], 1,
        "{what}: the unreadable subtree must be counted, got {last}"
    );
    let errors = last["errors"]
        .as_array()
        .unwrap_or_else(|| panic!("{what}: `errors[]` must be present, got {last}"));
    assert_eq!(errors.len(), 1, "{what}: one entry per failed target");
    let entry = &errors[0];
    assert!(
        entry["path"].as_str().is_some_and(|p| p.contains("inner")),
        "{what}: the entry must name the subtree, got {entry}"
    );
    assert_eq!(
        entry["code"], "PERMISSION_DENIED",
        "{what}: the entry must carry the reason"
    );
}

#[test]
fn count_reports_the_hole() {
    let b = blocked();
    let out = run(&["count", "."], b.dir.path());
    assert_reports_the_hole(&out, "count");
    assert_eq!(out.code, 13, "an incomplete count fails the run");
}

#[test]
fn list_reports_the_hole() {
    let b = blocked();
    let out = run(&["list", "."], b.dir.path());
    assert_reports_the_hole(&out, "list");
    assert_eq!(out.code, 13);
}

#[test]
fn hash_reports_the_hole() {
    let b = blocked();
    let out = run(&["hash", ".", "-r"], b.dir.path());
    assert_reports_the_hole(&out, "hash");
    assert_eq!(out.code, 13);
}

#[test]
fn copy_reports_the_hole() {
    let root = tempfile::tempdir().expect("tempdir");
    let src = root.path().join("src");
    fs::create_dir(&src).expect("create src");
    fs::write(src.join("a.txt"), "hi\n").expect("write file");
    let inner = src.join("inner");
    fs::create_dir(&inner).expect("create inner");
    fs::write(inner.join("b.txt"), "hi\n").expect("write inner");
    fs::set_permissions(&inner, fs::Permissions::from_mode(0o000)).expect("chmod");
    let dst = root.path().join("out");
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
    let _ = fs::set_permissions(&inner, fs::Permissions::from_mode(0o755));
    assert_reports_the_hole(&out, "copy");
    assert_eq!(out.code, 13);
}

#[test]
fn delete_reports_the_hole() {
    let b = blocked();
    let out = run(&["delete", ".", "-r", "--dry-run"], b.dir.path());
    assert_reports_the_hole(&out, "delete");
    assert_eq!(out.code, 13);
}

#[test]
fn case_reports_the_hole() {
    let b = blocked();
    let out = run(
        &["case", "--to", "snake", "--subvert", "aa", "bb", "."],
        b.dir.path(),
    );
    assert_reports_the_hole(&out, "case");
    assert_eq!(out.code, 13);
}

#[test]
fn sparse_reports_the_hole() {
    let b = blocked();
    let out = run(&["sparse", "list", "."], b.dir.path());
    assert_reports_the_hole(&out, "sparse");
    // The entries it could list were published, so the run answered: the hole
    // is `partial`, and exit 0 is what an answer costs.
    assert_eq!(out.code, 0);
    assert_eq!(terminal(&out)["partial"], true);
}

#[test]
fn semantic_search_reports_the_hole() {
    let b = blocked();
    let out = run(&["semantic-search", "hi", "."], b.dir.path());
    assert_reports_the_hole(&out, "semantic-search");
    // Same rule as `sparse`: hits published, hole reported, answer kept.
    assert_eq!(out.code, 0);
    assert_eq!(terminal(&out)["partial"], true);
}

#[test]
#[cfg(feature = "ast")]
fn scope_reports_the_hole() {
    let b = blocked();
    let out = run(
        &[
            "scope",
            "--language",
            "rust",
            "--pattern",
            "fn $A() { $$$B }",
            ".",
        ],
        b.dir.path(),
    );
    let last = terminal(&out);
    assert_eq!(
        last["files_failed"], 1,
        "scope must count the unreadable subtree, got {last}"
    );
}
