// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regression tests for the four destructive defects found by the v0.1.37
//! end-to-end audit. Each one was measured on the shipped binary before the
//! fix and must now fail closed instead of touching the file.

mod common;

use assert_cmd::Command;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

/// Per-process bound so a hung verb fails the test instead of the suite.
const PROC_TIMEOUT: Duration = Duration::from_secs(60);

struct Out {
    code: Option<i32>,
    stdout: String,
    events: Vec<Value>,
}

impl Out {
    fn last(&self) -> Option<&Value> {
        self.events.last()
    }
}

fn run_args<S: AsRef<std::ffi::OsStr>>(ws: &Path, args: &[S], stdin: &str) -> Out {
    let mut cmd = common::hermetic(Command::cargo_bin("atomwrite").expect("binary"));
    cmd.timeout(PROC_TIMEOUT)
        .args([
            "--workspace",
            ws.to_str().expect("utf8 workspace"),
            "--no-progress",
            "--no-auto-heal",
        ])
        .args(args)
        .write_stdin(stdin.as_bytes().to_vec());
    let output = cmd.output().expect("run atomwrite");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let events = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();
    Out {
        code: output.status.code(),
        stdout,
        events,
    }
}

// BUG 1 (destructive): `prune-backups` matched every name that merely STARTED
// with `<file>.bak.`, so a user note named `f.txt.bak.0-user-note` was deleted
// by `--max-count 0` and by `--max-age-secs 0`. The retention path already
// gated on the timestamp shape; the verb did not.
#[test]
fn prune_backups_never_deletes_a_user_file_that_looks_like_a_backup() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("f.txt");
    std::fs::write(&target, "a\nb\n").expect("fixture");
    let note = ws.path().join("f.txt.bak.0-user-note");
    std::fs::write(&note, "not a backup\n").expect("note");

    for extra in [["--max-count", "0"], ["--max-age-secs", "0"]] {
        let out = run_args(
            ws.path(),
            &[
                "prune-backups".to_string(),
                target.display().to_string(),
                extra[0].to_string(),
                extra[1].to_string(),
            ],
            "",
        );
        assert!(
            note.exists(),
            "{} {} deleted a user file; stdout: {:.300}",
            extra[0],
            extra[1],
            out.stdout
        );
        assert_eq!(out.code, Some(0), "stdout: {:.300}", out.stdout);
        assert_eq!(
            out.last().and_then(|v| v["total"].as_u64()),
            Some(0),
            "nothing may be counted as pruned for a non-backup name"
        );
    }
}

// BUG 2 (destructive): `--multi` read `content`, but `new` is also a struct
// field, so `{"op":"replace-range",...,"new":"X"}` fell through to
// `unwrap_or("")` and DELETED the range while answering `ok: true`.
#[test]
fn edit_multi_rejects_a_content_field_the_op_does_not_read() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("m.txt");
    let original = "linha1\nlinha2\nlinha3\n";
    std::fs::write(&target, original).expect("fixture");

    for ops in [
        "{\"op\":\"replace-range\",\"start\":2,\"end\":3,\"new\":\"SUBSTITUIDO\"}\n",
        "{\"op\":\"insert-after\",\"line\":1,\"new\":\"NOVO\"}\n",
        "{\"op\":\"insert-before\",\"line\":1,\"new\":\"NOVO\"}\n",
        "{\"op\":\"delete-range\",\"start\":1,\"end\":1,\"content\":\"X\"}\n",
    ] {
        let out = run_args(
            ws.path(),
            &[
                "edit".to_string(),
                "--multi".to_string(),
                target.display().to_string(),
            ],
            ops,
        );
        assert_eq!(
            out.code,
            Some(65),
            "ops {ops:?} must be INVALID_INPUT, stdout: {:.300}",
            out.stdout
        );
        assert_eq!(
            std::fs::read_to_string(&target).expect("read target"),
            original,
            "ops {ops:?} modified the file"
        );
        assert_eq!(
            out.last().and_then(|v| v["ok"].as_bool()),
            Some(false),
            "last event: {out:?}",
            out = out.stdout
        );
    }
}

// The same shape with the RIGHT field keeps working, so the gate is not a
// blanket refusal of line ops.
#[test]
fn edit_multi_still_applies_a_range_with_content() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("ok.txt");
    std::fs::write(&target, "linha1\nlinha2\nlinha3\n").expect("fixture");

    let out = run_args(
        ws.path(),
        &[
            "edit".to_string(),
            "--multi".to_string(),
            target.display().to_string(),
        ],
        "{\"op\":\"replace-range\",\"start\":2,\"end\":2,\"content\":\"TROCADO\"}\n",
    );
    assert_eq!(out.code, Some(0), "stdout: {:.300}", out.stdout);
    assert_eq!(
        std::fs::read_to_string(&target).expect("read target"),
        "linha1\nTROCADO\nlinha3\n"
    );
}

// A JSON array is not NDJSON; the old message blamed the first element
// ("invalid type: map, expected a string") instead of naming the real cause.
#[test]
fn edit_multi_names_the_ndjson_format_when_handed_a_json_array() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("arr.txt");
    std::fs::write(&target, "a\n").expect("fixture");

    let out = run_args(
        ws.path(),
        &[
            "edit".to_string(),
            "--multi".to_string(),
            target.display().to_string(),
        ],
        "[{\"op\":\"exact\",\"old\":\"a\",\"new\":\"b\"}]\n",
    );
    assert_eq!(out.code, Some(65), "stdout: {:.300}", out.stdout);
    let message = out
        .last()
        .and_then(|v| v["message"].as_str())
        .unwrap_or_default()
        .to_string();
    assert!(
        message.contains("one JSON object per line"),
        "message must name the NDJSON contract, got: {message:?}"
    );
    assert_eq!(std::fs::read_to_string(&target).expect("read"), "a\n");
}

// BUG 3 (destructive): a `---`/`+++` pair with no `@@` was classified `full`,
// so the patch TEXT overwrote the target. `patch(1)` calls this malformed.
#[test]
fn apply_refuses_a_diff_header_without_a_hunk() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("a.txt");
    let original = "one\ntwo\n";
    std::fs::write(&target, original).expect("fixture");

    let out = run_args(
        ws.path(),
        &["apply".to_string(), target.display().to_string()],
        "--- a.txt\n+++ a.txt\n",
    );
    assert_eq!(
        out.code,
        Some(65),
        "a hunkless diff must be INVALID_INPUT, stdout: {:.300}",
        out.stdout
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("read target"),
        original,
        "the patch text must never become the file content"
    );
}

// A real unified diff still applies, so the classification did not overshoot.
#[test]
fn apply_still_applies_a_unified_diff() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("u.txt");
    std::fs::write(&target, "a\nb\nc\n").expect("fixture");

    let out = run_args(
        ws.path(),
        &["apply".to_string(), target.display().to_string()],
        "--- u.txt\n+++ u.txt\n@@ -2,1 +2,1 @@\n-b\n+B\n",
    );
    assert_eq!(out.code, Some(0), "stdout: {:.300}", out.stdout);
    assert_eq!(
        out.last().and_then(|v| v["format_detected"].as_str()),
        Some("unified")
    );
    assert_eq!(std::fs::read_to_string(&target).expect("read"), "a\nB\nc\n");
}

// BUG 4: an empty hunk reported `hunks_applied: 1` for a write that changed
// no byte. GNU `patch` answers `malformed patch` and exits 2.
#[test]
fn apply_refuses_a_hunk_that_carries_no_lines() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("h.txt");
    let original = "a\nb\nc\n";
    std::fs::write(&target, original).expect("fixture");

    for header in ["@@ -2,0 +2,0 @@", "@@ -0,0 +0,0 @@", "@@ -1,0 +1,0 @@"] {
        let patch = format!("--- h.txt\n+++ h.txt\n{header}\n");
        let out = run_args(
            ws.path(),
            &["apply".to_string(), target.display().to_string()],
            &patch,
        );
        assert_eq!(
            out.code,
            Some(65),
            "hunk {header:?} must be INVALID_INPUT, stdout: {:.300}",
            out.stdout
        );
        assert_eq!(
            std::fs::read_to_string(&target).expect("read target"),
            original,
            "hunk {header:?} rewrote the file"
        );
    }
}

// A journal in the workspace ROOT used to publish `"path": ""` in
// `by_directory`, which reads as a missing directory.
#[test]
fn wal_stats_names_the_workspace_root_as_dot() {
    let ws = tempfile::tempdir().expect("workspace");
    std::fs::create_dir_all(ws.path().join("sub")).expect("sub");
    std::fs::write(
        ws.path()
            .join(".atomwrite.journal.w.txt.atomwrite.journal.json"),
        "{\"phase\":\"started\",\"path\":\"w.txt\",\"checksum_before\":\"aa\",\"ts\":1}",
    )
    .expect("root journal");
    std::fs::write(
        ws.path()
            .join("sub/.atomwrite.journal.s.txt.atomwrite.journal.json"),
        "{\"phase\":\"started\",\"path\":\"sub/s.txt\",\"checksum_before\":\"bb\",\"ts\":2}",
    )
    .expect("sub journal");

    let out = run_args(ws.path(), &["wal-stats".to_string()], "");
    assert_eq!(out.code, Some(0), "stdout: {:.300}", out.stdout);
    let dirs = out
        .last()
        .and_then(|v| v["by_directory"].as_array())
        .cloned()
        .unwrap_or_default();
    let paths: Vec<String> = dirs
        .iter()
        .filter_map(|d| d["path"].as_str().map(str::to_owned))
        .collect();
    assert!(
        paths.iter().any(|p| p == "."),
        "the root must be published as \".\", got {paths:?}"
    );
    assert!(
        !paths.iter().any(String::is_empty),
        "an empty path reads as a missing directory, got {paths:?}"
    );
}
