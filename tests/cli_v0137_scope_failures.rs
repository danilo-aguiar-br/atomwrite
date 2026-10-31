// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 `scope` failure accounting: a file that cannot be read is a FAILED
//! file, not a skipped one.
//!
//! The verb used to fold every `Err` into `files_skipped`, so a directory whose
//! only file was unreadable ended `ok: true` with `files_failed: 0` — a
//! complete-looking answer over a file it never looked at. `FileTooLarge` stays
//! a skip (the verb was told to leave that file alone); every other read error
//! is a failure and carries its error record. A read failure fails the run only
//! when no file was read at all, the same rule `search` follows.

#![cfg(feature = "ast")]
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

fn terminal(run: &Run) -> &serde_json::Value {
    run.events
        .iter()
        .find(|e| e["type"] == "summary")
        .unwrap_or_else(|| panic!("no summary in {:?}", run.events))
}

#[test]
fn scope_unreadable_file_reports_files_failed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let good = common::create_test_file(dir.path(), "good.rs", "fn a() { let x = 1; }\n");
    let secret = common::create_test_file(dir.path(), "secret.rs", "fn b() { let y = 2; }\n");
    if mode_zero_is_readable(&secret) {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let _restore = RestoreMode(secret.clone(), 0o644);

    let out = run(
        dir.path(),
        &[
            "scope",
            "-l",
            "rust",
            "--query",
            "fn",
            good.to_str().expect("utf8 path"),
            secret.to_str().expect("utf8 path"),
        ],
    );

    let summary = terminal(&out);
    assert_eq!(
        summary["files_failed"], 1,
        "the unreadable file is a failure: {summary}"
    );
    assert_eq!(summary["files_skipped"], 0, "{summary}");
    let errors = summary["errors"]
        .as_array()
        .unwrap_or_else(|| panic!("errors[] present: {summary}"));
    assert_eq!(errors.len(), 1, "{summary}");
    assert_eq!(errors[0]["code"], "PERMISSION_DENIED", "{summary}");
    assert!(
        errors[0]["path"]
            .as_str()
            .is_some_and(|p| p.ends_with("secret.rs")),
        "errors[0] names the file: {summary}"
    );
    assert_eq!(
        out.code,
        Some(0),
        "a run that read another file still answered: {summary}"
    );
}

#[test]
fn scope_unreadable_only_run_exits_with_denied() {
    let dir = tempfile::tempdir().expect("tempdir");
    let secret = common::create_test_file(dir.path(), "secret.rs", "fn b() { let y = 2; }\n");
    if mode_zero_is_readable(&secret) {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let _restore = RestoreMode(secret.clone(), 0o644);

    let out = run(
        dir.path(),
        &[
            "scope",
            "-l",
            "rust",
            "--query",
            "fn",
            secret.to_str().expect("utf8 path"),
        ],
    );

    assert_ne!(
        out.code,
        Some(1),
        "not a NO_MATCHES run over a file that was never read: {:?}",
        out.events
    );
    assert_eq!(out.code, Some(13), "{:?}", out.events);
    let summary = terminal(&out);
    assert_eq!(summary["ok"], false, "{summary}");
    assert_eq!(summary["code"], "PERMISSION_DENIED", "{summary}");
}

#[test]
fn scope_file_too_large_counts_as_skipped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let good = common::create_test_file(dir.path(), "good.rs", "fn a() { let x = 1; }\n");
    let body = format!("fn big() {{\n{}}}\n", "let x = 1;\n".repeat(2000));
    let big = common::create_test_file(dir.path(), "big.rs", &body);

    let out = run(
        dir.path(),
        &[
            "--max-filesize",
            "1024",
            "scope",
            "-l",
            "rust",
            "--query",
            "fn",
            good.to_str().expect("utf8 path"),
            big.to_str().expect("utf8 path"),
        ],
    );

    let summary = terminal(&out);
    assert_eq!(
        summary["files_skipped"], 1,
        "a size cap is a skip, not a failure: {summary}"
    );
    assert!(
        summary.get("files_failed").is_none() || summary["files_failed"].as_u64().unwrap_or(0) == 0,
        "no failure recorded for a capped file: {summary}"
    );
    assert_eq!(out.code, Some(0), "{summary}");
}
