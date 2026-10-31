// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 — per-domain error codes and exit verdicts.
//!
//! Covers the four contract changes:
//! C1 merge conflict gets its own code (84 / `MERGE_CONFLICT`) and carries
//!    the conflict count in the envelope instead of reporting `INVALID_INPUT`.
//! C2 a partial `edit-loop` exits non-zero by default while the summary keeps
//!    the counts.
//! C3 a directory without `--recursive` names the flag in a structured field.
//! C4 an identical `case --subvert` pair is refused as invalid input.
//!
//! The `edit-loop` cases drive stdin through `std::process::Command` with an
//! explicit stdin drop: `assert_cmd::write_stdin` writes to a fresh pipe while
//! the child holds the inherited handle, which would block the read forever
//! (same rationale as `tests/cli_v0121_edit_loop.rs`).

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use assert_cmd::Command as AssertCommand;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn bin() -> AssertCommand {
    common::hermetic(AssertCommand::cargo_bin("atomwrite").expect("atomwrite bin"))
}

/// First NDJSON line of stdout parsed as a JSON object.
fn first_event(bytes: &[u8]) -> Value {
    let text = String::from_utf8_lossy(bytes);
    let line = text
        .lines()
        .find(|l| !l.trim().is_empty())
        .expect("stdout must carry at least one NDJSON event");
    serde_json::from_str(line).expect("first stdout line must be valid NDJSON")
}

/// Run `edit-loop` with explicit stdin EOF (see module docs).
fn run_edit_loop(
    ws: &std::path::Path,
    target: &std::path::Path,
    ndjson: &str,
    extra: &[&str],
) -> std::process::Output {
    let bin_path = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut cmd = common::hermetic(Command::new(&bin_path));
    cmd.args(["--workspace", ws.to_str().unwrap(), "edit-loop"])
        .arg(target)
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("spawn edit-loop");
    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(ndjson.as_bytes());
        let _ = stdin.flush();
    }
    drop(child.stdin.take());
    child.wait_with_output().expect("wait edit-loop")
}

// ─────────────────────────────────────────────────────────────────────────────
// C1 — merge conflict has its own code, exit and envelope count
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn c1_merge_conflict_uses_own_exit_code_and_count() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let base = ws.join("base.txt");
    let ours = ws.join("ours.txt");
    let theirs = ws.join("theirs.txt");
    let out = ws.join("out.txt");
    std::fs::write(&base, "line1\nshared\nline3\n").unwrap();
    std::fs::write(&ours, "line1\nOURS\nline3\n").unwrap();
    std::fs::write(&theirs, "line1\nTHEIRS\nline3\n").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "semantic-merge",
            "--base",
            base.to_str().unwrap(),
            "--ours",
            ours.to_str().unwrap(),
            "--theirs",
            theirs.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--fail-on-conflict",
            "--no-backup",
        ])
        .assert();

    let stdout = assert.get_output().stdout.clone();
    assert
        .failure()
        .code(predicate::eq(84))
        .stdout(predicate::str::contains("semantic_merge"));

    let events: Vec<Value> = common::parse_ndjson(&stdout);
    let merge_event = events
        .iter()
        .find(|e| e["type"] == "semantic_merge")
        .expect("the conflict event must still be emitted before the failure");
    assert_eq!(merge_event["status"], "conflict");
    assert_eq!(
        merge_event["conflicts"].as_array().map(Vec::len),
        Some(1),
        "one conflicting region expected: {merge_event}"
    );

    let envelope = events
        .iter()
        .rfind(|e| e["ok"] == false)
        .expect("an error envelope must be emitted");
    assert_eq!(envelope["code"], "MERGE_CONFLICT");
    assert_eq!(envelope["exit"], 84);
    assert_eq!(envelope["error_class"], "permanent");
    assert_eq!(envelope["retryable"], false);
    assert_eq!(
        envelope["conflict_count"], 1,
        "the count must travel as its own field: {envelope}"
    );
    assert_ne!(
        envelope["code"], "INVALID_INPUT",
        "a merge conflict is a domain result, not malformed input"
    );
}

#[test]
fn c1_merge_conflict_suggestion_is_localized_and_actionable() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    for (name, body) in [
        ("base.txt", "l1\nshared\nl3\n"),
        ("ours.txt", "l1\nOURS\nl3\n"),
        ("theirs.txt", "l1\nTHEIRS\nl3\n"),
    ] {
        std::fs::write(ws.join(name), body).unwrap();
    }

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "semantic-merge",
            "--base",
            "base.txt",
            "--ours",
            "ours.txt",
            "--theirs",
            "theirs.txt",
            "--output",
            "out.txt",
            "--fail-on-conflict",
            "--no-backup",
            "--locale",
            "en",
        ])
        .assert()
        .failure()
        .code(predicate::eq(84));

    let stdout = assert.get_output().stdout.clone();
    let events = common::parse_ndjson(&stdout);
    let envelope = events
        .iter()
        .rfind(|e| e["ok"] == false)
        .expect("an error envelope must be emitted");
    let suggestion = envelope["suggestion"]
        .as_str()
        .expect("merge conflict must carry a suggestion");
    assert!(
        suggestion.contains("--fail-on-conflict"),
        "suggestion must name the escape hatch, got: {suggestion}"
    );
}

#[test]
fn c1_marker_merge_without_fail_on_conflict_still_succeeds() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let out = ws.join("out.txt");
    for (name, body) in [
        ("base.txt", "l1\nshared\nl3\n"),
        ("ours.txt", "l1\nOURS\nl3\n"),
        ("theirs.txt", "l1\nTHEIRS\nl3\n"),
    ] {
        std::fs::write(ws.join(name), body).unwrap();
    }

    // The tolerant path must stay untouched: markers on, no --fail-on-conflict.
    bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "semantic-merge",
            "--base",
            "base.txt",
            "--ours",
            "ours.txt",
            "--theirs",
            "theirs.txt",
            "--output",
            out.to_str().unwrap(),
            "--no-backup",
        ])
        .assert()
        .success();

    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.contains("<<<<<<< ours"), "body={body:?}");
}

// ─────────────────────────────────────────────────────────────────────────────
// C2 — `edit-loop --partial` returns its own domain verdict (exit 87)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn c2_partial_edit_loop_exits_87_with_envelope() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("file.txt");
    std::fs::write(&target, "alpha\n").unwrap();

    let ndjson = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n{\"old\":\"missing\",\"new\":\"X\"}\n";
    let output = run_edit_loop(ws, &target, ndjson, &["--partial"]);

    assert!(
        !output.status.success(),
        "a partial batch must not exit 0, got {:?}",
        output.status.code()
    );

    // The matched pair is still applied: the miss is reported, not rolled back.
    let content = std::fs::read_to_string(&target).unwrap();
    assert_eq!(content, "ALPHA\n", "matched pairs must still be written");

    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "result" && e["action"] == "edit_loop")
        .expect("summary must still be emitted");
    assert_eq!(summary["pairs_total"], 2);
    assert_eq!(summary["pairs_applied"], 1);
    assert_eq!(summary["pairs_unmatched"], 1);

    let envelope = events
        .iter()
        .rfind(|e| e["ok"] == false)
        .expect("an error envelope must be emitted");
    assert_eq!(envelope["code"], "EDIT_LOOP_PARTIAL");
    assert_eq!(
        envelope["exit"], 87,
        "a partial batch has its own status (87), not no-match (1) or invalid input (65)"
    );
    assert_eq!(envelope["pairs_unmatched"], 1);
    assert_eq!(envelope["pairs_applied"], 1);
    assert_eq!(envelope["pairs_total"], 2);
    assert_eq!(envelope["error_class"], "permanent");
    assert_eq!(
        output.status.code(),
        Some(87),
        "process exit must match the envelope exit"
    );
    assert_ne!(
        envelope["code"], "INVALID_INPUT",
        "the pairs were valid input that did not match"
    );
}

#[test]
fn c2_full_match_still_exits_zero() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("file.txt");
    std::fs::write(&target, "alpha beta\n").unwrap();

    let ndjson = "{\"old\":\"alpha\",\"new\":\"ALPHA\"}\n{\"old\":\"beta\",\"new\":\"BETA\"}\n";
    let output = run_edit_loop(ws, &target, ndjson, &[]);

    assert!(
        output.status.success(),
        "all pairs matched, exit must stay 0: {:?}",
        output.status.code()
    );
    let events = common::parse_ndjson(&output.stdout);
    assert!(
        !events.iter().any(|e| e["ok"] == false),
        "no error envelope when every pair matched: {events:?}"
    );
}

#[test]
fn c2_zero_match_aborts_without_writing_and_exits_65() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("file.txt");
    std::fs::write(&target, "alpha\n").unwrap();

    let ndjson = "{\"old\":\"missing\",\"new\":\"X\"}\n";
    let output = run_edit_loop(ws, &target, ndjson, &[]);

    assert!(
        !output.status.success(),
        "zero matched pairs must not exit 0, got {:?}",
        output.status.code()
    );
    let envelope = common::parse_ndjson(&output.stdout)
        .into_iter()
        .rfind(|e| e["ok"] == false)
        .expect("envelope");
    // All-or-nothing by default: a lone unmatched pair is the match error itself.
    assert_eq!(envelope["code"], "MATCH_FAILED");
    assert_eq!(envelope["exit"], 65);
    assert_eq!(output.status.code(), Some(65));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "alpha\n");
}

// ─────────────────────────────────────────────────────────────────────────────
// C3 — a missing companion flag is named in its own envelope field
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn c3_directory_without_recursive_names_the_flag() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let dir = ws.join("folder");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("inner.txt"), "x\n").unwrap();

    let assert = bin()
        .args(["--workspace", ws.to_str().unwrap(), "delete", "folder"])
        .assert()
        .failure()
        .code(predicate::eq(65));

    let envelope = first_event(&assert.get_output().stdout);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["code"], "INVALID_INPUT");
    assert_eq!(
        envelope["missing_flag"], "--recursive",
        "the required flag must be a field, not prose: {envelope}"
    );
    assert!(
        dir.join("inner.txt").exists(),
        "the refusal must not touch the directory"
    );
}

#[test]
fn c3_missing_flag_field_is_absent_on_other_errors() {
    let td = TempDir::new().unwrap();
    let ws = td.path();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "delete",
            "does-not-exist.txt",
        ])
        .assert()
        .failure()
        .code(predicate::eq(4));

    let envelope = first_event(&assert.get_output().stdout);
    assert_eq!(envelope["code"], "FILE_NOT_FOUND");
    assert!(
        envelope.get("missing_flag").is_none(),
        "the field is optional and only present when relevant: {envelope}"
    );
}

#[test]
fn c3_recursive_still_allows_directory_delete() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let dir = ws.join("folder");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("inner.txt"), "x\n").unwrap();

    bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "delete",
            "--recursive",
            "--no-backup",
            "folder",
        ])
        .assert()
        .success();

    assert!(!dir.exists(), "recursive delete must remove the directory");
}

// ─────────────────────────────────────────────────────────────────────────────
// C4 — an identical case --subvert pair is refused
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn c4_identical_subvert_pair_is_rejected() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let f = ws.join("a.rs");
    std::fs::write(&f, "let http_request = 1;\n").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "http_request",
            "http_request",
            "--to",
            "snake",
        ])
        .assert()
        .failure()
        .code(predicate::eq(65));

    let envelope = first_event(&assert.get_output().stdout);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["code"], "INVALID_INPUT");
    assert!(
        envelope["message"]
            .as_str()
            .is_some_and(|m| m.contains("identical")),
        "message must name the problem: {envelope}"
    );

    let body = std::fs::read_to_string(&f).unwrap();
    assert_eq!(body, "let http_request = 1;\n", "nothing may be rewritten");
}

#[test]
fn c4_distinct_subvert_pair_still_applies() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let f = ws.join("a.rs");
    std::fs::write(&f, "let HTTPRequest = 1;\n").unwrap();

    bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "case",
            f.to_str().unwrap(),
            "--subvert",
            "HTTPRequest",
            "http_request",
            "--to",
            "snake",
            "--no-backup",
        ])
        .assert()
        .success();

    let body = std::fs::read_to_string(&f).unwrap();
    assert!(body.contains("let http_request = 1;"), "body={body}");
}

// ─────────────────────────────────────────────────────────────────────────────
// C5 — a failed transaction rollback has its own status, and it is derivable
//
// The runtime failure is a disk-side condition that cannot be forced
// deterministically from the CLI (the reversal only fails on rename / read /
// atomic-write / unlink errors), so the contract is pinned where it is
// observable: the agent surface, which reads the status off the error enum.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn c5_transaction_rollback_failed_is_derived_at_80() {
    let assert = bin().args(["agent-surface"]).assert().success();

    let events = common::parse_ndjson(&assert.get_output().stdout);
    let manifest = events
        .iter()
        .find(|e| e["type"] == "agent_surface")
        .expect("agent surface manifest");

    let entry = manifest["exit_codes"]
        .as_array()
        .expect("exit_codes array")
        .iter()
        .find(|e| e["code"] == 80 && e["name"] == "TRANSACTION_ROLLBACK_FAILED")
        .expect("exit 80 TRANSACTION_ROLLBACK_FAILED must be on the surface");

    assert_eq!(
        entry["source"], "error-variant",
        "the status must be derived from the enum, not hand-listed as a process code"
    );
    assert_eq!(
        entry["error_class"], "permanent",
        "a half-applied reversal is permanent: a blind retry can destroy the backups"
    );
    assert_eq!(entry["retryable"], false, "must not advertise a retry");
}

#[test]
fn c5_rollback_status_is_no_longer_a_bare_process_code() {
    let assert = bin().args(["agent-surface"]).assert().success();
    let events = common::parse_ndjson(&assert.get_output().stdout);
    let manifest = events
        .iter()
        .find(|e| e["type"] == "agent_surface")
        .expect("agent surface manifest");

    let entries: Vec<&Value> = manifest["exit_codes"]
        .as_array()
        .expect("exit_codes array")
        .iter()
        .filter(|e| e["code"] == 80)
        .collect();

    assert_eq!(entries.len(), 1, "exit 80 must appear once: {entries:?}");
    assert_eq!(
        entries[0]["source"], "error-variant",
        "before v0.1.36 the 80 was a placeholder with no producer"
    );
}

#[test]
fn c5_edit_loop_partial_has_its_own_status() {
    let assert = bin().args(["agent-surface"]).assert().success();
    let events = common::parse_ndjson(&assert.get_output().stdout);
    let manifest = events
        .iter()
        .find(|e| e["type"] == "agent_surface")
        .expect("agent surface manifest");

    let codes = manifest["exit_codes"].as_array().expect("exit_codes");
    let names_at_1: Vec<&str> = codes
        .iter()
        .filter(|e| e["code"] == 1)
        .filter_map(|e| e["name"].as_str())
        .collect();

    // v0.1.37: the partial verdict writes the matching pairs, so it cannot share
    // the "found nothing" number with NO_MATCHES.
    assert!(
        names_at_1.contains(&"NO_MATCHES"),
        "NO_MATCHES must stay at 1: {names_at_1:?}"
    );
    assert!(
        !names_at_1.contains(&"EDIT_LOOP_PARTIAL"),
        "EDIT_LOOP_PARTIAL moved to its own status 87: {names_at_1:?}"
    );
    let names_at_87: Vec<&str> = codes
        .iter()
        .filter(|e| e["code"] == 87)
        .filter_map(|e| e["name"].as_str())
        .collect();
    assert_eq!(names_at_87, vec!["EDIT_LOOP_PARTIAL"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Catalogue — the new keys must exist in BOTH locales
//
// Whole-catalogue key parity is owned by `tests/locale_parity.rs`; this only
// pins the keys this release added, so a missing translation in either
// language fails here instead of silently falling back at runtime.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn new_suggestion_keys_exist_in_both_locales() {
    const NEW_KEYS: [&str; 4] = [
        "suggestion.merge-conflict",
        "suggestion.edit-loop-partial",
        "suggestion.missing-flag",
        "suggestion.transaction-rollback-failed",
    ];

    for locale in ["locales/en.toml", "locales/pt-BR.toml"] {
        let body = std::fs::read_to_string(locale).unwrap_or_else(|e| panic!("read {locale}: {e}"));
        for key in NEW_KEYS {
            assert!(
                body.lines().any(|l| l.starts_with(key) && l.contains('=')),
                "{locale} must define {key}"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Exit 91 — `--strict-atomic` reaches `EXDEV_FALLBACK_DISABLED`
// ─────────────────────────────────────────────────────────────────────────────

/// Two hardlinks to one inode force the in-place strategy, the only write the
/// CLI can do that is not a single atomic step. `--strict-atomic` promises that
/// step, so the write is refused with exit 91 before anything is touched.
#[test]
fn strict_atomic_refuses_the_in_place_write_of_a_hardlinked_target() {
    let td = TempDir::new().expect("tempdir");
    let ws = td.path();
    let target = ws.join("a.txt");
    std::fs::write(&target, "alfa\n").expect("seed");
    std::fs::hard_link(&target, ws.join("b.txt")).expect("hardlink");

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "write",
            "--strict-atomic",
            target.to_str().unwrap(),
        ])
        .write_stdin("beta\n")
        .assert()
        .code(91);

    let v = first_event(&assert.get_output().stdout);
    assert_eq!(v["code"], "EXDEV_FALLBACK_DISABLED", "{v}");
    assert_eq!(v["ok"], false, "{v}");
    assert_eq!(v["retryable"], false, "{v}");
    assert_eq!(
        std::fs::read_to_string(&target).expect("read"),
        "alfa\n",
        "a refused write must not touch the target"
    );
}

/// The same target without the flag keeps the inode and succeeds, so the flag
/// is the only difference between the two runs.
#[test]
fn without_strict_atomic_the_hardlinked_target_is_written_in_place() {
    let td = TempDir::new().expect("tempdir");
    let ws = td.path();
    let target = ws.join("a.txt");
    std::fs::write(&target, "alfa\n").expect("seed");
    std::fs::hard_link(&target, ws.join("b.txt")).expect("hardlink");

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "write",
            target.to_str().unwrap(),
        ])
        .write_stdin("beta\n")
        .assert()
        .success();

    let v = first_event(&assert.get_output().stdout);
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["platform"]["rename_method"], "inplace", "{v}");
    assert_eq!(std::fs::read_to_string(&target).expect("read"), "beta\n");
}

#[test]
fn strict_atomic_and_confirm_side_effect_share_the_same_contract() {
    const NEW_KEYS: [&str; 2] = [
        "suggestion.confirm-side-effect-without-side-effect",
        "suggestion.language-extension-mismatch",
    ];
    for locale in ["locales/en.toml", "locales/pt-BR.toml"] {
        let body = std::fs::read_to_string(locale).unwrap_or_else(|e| panic!("read {locale}: {e}"));
        for key in NEW_KEYS {
            assert!(
                body.lines().any(|l| l.starts_with(key) && l.contains('=')),
                "{locale} must define {key}"
            );
        }
    }
}
