// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 discovery verbs: a file that cannot be read is a reported failure.
//!
//! `count`, `list`, `sparse` and `semantic-search` used to skip an unreadable
//! file in silence, so the terminal event said `ok: true` over a total that
//! never saw the file. Each now carries `files_failed` and `errors[]`, and the
//! terminal record reports `ok: false` with the real code.

#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Restores the mode of a path on drop so the temp dir can be removed.
struct RestoreMode(std::path::PathBuf, u32);

impl Drop for RestoreMode {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(self.1));
    }
}

/// True when a file with mode 0 can still be read (running as root).
fn mode_zero_is_readable(path: &Path) -> bool {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let readable = std::fs::read(path).is_ok();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).expect("chmod 644");
    readable
}

struct Run {
    code: Option<i32>,
    events: Vec<serde_json::Value>,
}

fn run(ws: &Path, args: &[&str]) -> Run {
    let output = common::atomwrite()
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(args)
        .output()
        .expect("run atomwrite");
    Run {
        code: output.status.code(),
        events: common::parse_ndjson(&output.stdout),
    }
}

/// The single terminal record: the one that answers `ok`.
fn terminal(run: &Run) -> &serde_json::Value {
    run.events
        .iter()
        .find(|e| e.get("ok").is_some())
        .unwrap_or_else(|| panic!("no terminal record in {:?}", run.events))
}

/// The hole every one of these verbs reports: one failed file, named.
fn assert_hole_counted<'a>(run: &'a Run, what: &str, expected_path: &str) -> &'a serde_json::Value {
    let summary = terminal(run);
    assert_eq!(
        summary["files_failed"], 1,
        "{what}: the unreadable file is a failure: {summary}"
    );
    let errors = summary["errors"]
        .as_array()
        .unwrap_or_else(|| panic!("{what}: errors[] present: {summary}"));
    assert_eq!(errors.len(), 1, "{what}: {summary}");
    assert_eq!(errors[0]["code"], "PERMISSION_DENIED", "{what}: {summary}");
    assert!(
        errors[0]["path"]
            .as_str()
            .is_some_and(|p| p.ends_with(expected_path)),
        "{what}: errors[0] names the file: {summary}"
    );
    summary
}

/// An aggregate verb that could not answer the whole set fails the run.
fn assert_one_failure(run: &Run, what: &str, expected_path: &str) {
    let summary = assert_hole_counted(run, what, expected_path);
    assert_eq!(summary["ok"], false, "{what}: {summary}");
    assert_eq!(run.code, Some(13), "{what}: {:?}", run.events);
}

/// A verb that answered from what it could read keeps its answer and names the
/// hole: the run did its job, so it must not exit with a failure code.
fn assert_hole_in_answer(run: &Run, what: &str, expected_path: &str) {
    let summary = assert_hole_counted(run, what, expected_path);
    assert_eq!(summary["partial"], true, "{what}: {summary}");
    assert_eq!(summary["ok"], true, "{what}: {summary}");
    assert_eq!(run.code, Some(0), "{what}: {:?}", run.events);
}

#[test]
fn count_reports_files_failed() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "good.txt", "hello\n");
    let secret = common::create_test_file(dir.path(), "secret.txt", "hi\n");
    if mode_zero_is_readable(&secret) {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let _restore = RestoreMode(secret, 0o644);

    let out = run(dir.path(), &["count", dir.path().to_str().expect("utf8")]);
    assert_one_failure(&out, "count", "secret.txt");
}

#[test]
fn list_reports_files_failed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).expect("mkdir");
    common::create_test_file(&locked, "secret.txt", "hi\n");
    if mode_zero_is_readable(&locked) {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let _restore = RestoreMode(locked, 0o755);

    let out = run(dir.path(), &["list", dir.path().to_str().expect("utf8")]);
    assert_one_failure(&out, "list", "locked");
}

#[test]
fn sparse_reports_files_failed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let good = common::create_test_file(dir.path(), "good.txt", "hello\n");
    let secret = common::create_test_file(dir.path(), "secret.txt", "hi\n");
    if mode_zero_is_readable(&secret) {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let _restore = RestoreMode(secret.clone(), 0o644);

    let list_file = dir.path().join("paths.txt");
    std::fs::write(
        &list_file,
        format!(
            "{}\n{}\n",
            good.to_str().expect("utf8"),
            secret.to_str().expect("utf8")
        ),
    )
    .expect("write paths file");

    let out = run(
        dir.path(),
        &[
            "sparse",
            "read",
            "--paths-file",
            list_file.to_str().expect("utf8"),
        ],
    );
    assert_hole_in_answer(&out, "sparse", "secret.txt");
}

#[test]
fn semantic_search_reports_files_failed() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "good.txt", "hello world rust\n");
    let secret = common::create_test_file(dir.path(), "secret.txt", "hello world rust\n");
    if mode_zero_is_readable(&secret) {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let _restore = RestoreMode(secret, 0o644);

    let out = run(
        dir.path(),
        &[
            "semantic-search",
            "hello world",
            dir.path().to_str().expect("utf8"),
        ],
    );
    assert_hole_in_answer(&out, "semantic-search", "secret.txt");
}
