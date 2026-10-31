//! GAP-2026-019…027 agent-first contract tests for atomwrite v0.1.36.
mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn bin() -> Command {
    common::hermetic(Command::cargo_bin("atomwrite").expect("atomwrite bin"))
}

#[test]
fn old_file_outside_workspace_is_126_with_staging_suggestion() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("t.txt");
    fs::write(&target, "hello world\n").unwrap();
    // OS temp path is outside workspace jail
    let outside = std::env::temp_dir().join(format!("atomwrite-v0136-old-{}", std::process::id()));
    fs::write(&outside, "hello").unwrap();
    let outside_new =
        std::env::temp_dir().join(format!("atomwrite-v0136-new-{}", std::process::id()));
    fs::write(&outside_new, "hi").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "edit",
            "--old-file",
            outside.to_str().unwrap(),
            "--new-file",
            outside_new.to_str().unwrap(),
            target.to_str().unwrap(),
        ])
        .assert();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let _ = fs::remove_file(&outside);
    let _ = fs::remove_file(&outside_new);
    assert.failure().code(predicate::eq(126));
    let v: Value = serde_json::from_str(out.lines().next().unwrap_or("{}")).unwrap();
    assert_eq!(v["error"], true);
    assert_eq!(v["ok"], false);
    assert_eq!(v["code"], "WORKSPACE_JAIL");
    let sug = v["suggestion"].as_str().unwrap_or("");
    assert!(
        sug.contains(".atomwrite/scratch") || sug.contains("workspace"),
        "suggestion should guide staging, got: {sug}"
    );
}

#[test]
fn old_file_inside_scratch_applies() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("t.txt");
    fs::write(&target, "hello world\n").unwrap();
    let scratch = ws.join(".atomwrite").join("scratch").join("p");
    fs::create_dir_all(&scratch).unwrap();
    let old = scratch.join("old.txt");
    let new = scratch.join("new.txt");
    fs::write(&old, "hello").unwrap();
    fs::write(&new, "hola").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "edit",
            "--old-file",
            old.to_str().unwrap(),
            "--new-file",
            new.to_str().unwrap(),
            target.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let v: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    assert_eq!(v["match_count"], 1);
    let body = fs::read_to_string(&target).unwrap();
    assert!(body.contains("hola"));
}

#[test]
fn match_failed_envelope_has_ok_false_and_match_count_zero() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("t.txt");
    fs::write(&target, "alpha beta\n").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "edit",
            "--old",
            "does-not-exist-xyz",
            "--new",
            "x",
            "--fuzzy",
            "off",
            target.to_str().unwrap(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let v: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    assert_eq!(v["error"], true);
    assert_eq!(v["ok"], false);
    assert_eq!(v["match_count"], 0);
    assert!(v["code"].as_str().unwrap().contains("MATCH") || v["exit"].as_u64() == Some(65));
}

#[test]
fn match_ambiguous_exposes_best_candidate() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("t.txt");
    fs::write(&target, "aa\nxx\naa\n").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "edit",
            "--old",
            "aa",
            "--new",
            "bb",
            target.to_str().unwrap(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let v: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    assert_eq!(v["error"], true);
    assert_eq!(v["ok"], false);
    assert_eq!(v["code"], "MATCH_AMBIGUOUS");
    assert!(v["match_count"].as_u64().unwrap_or(0) >= 2);
    assert!(
        !v["best_candidate"].is_null(),
        "GAP-2026-025 best_candidate must not be null: {v}"
    );
}

#[test]
fn content_source_jail_suggestion_mentions_staging_role() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("t.txt");
    fs::write(&target, "hello world\n").unwrap();
    let outside = std::env::temp_dir().join(format!("atomwrite-cs-old-{}", std::process::id()));
    fs::write(&outside, "hello").unwrap();
    let outside_new = std::env::temp_dir().join(format!("atomwrite-cs-new-{}", std::process::id()));
    fs::write(&outside_new, "hi").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "edit",
            "--old-file",
            outside.to_str().unwrap(),
            "--new-file",
            outside_new.to_str().unwrap(),
            target.to_str().unwrap(),
        ])
        .assert();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let _ = fs::remove_file(&outside);
    let _ = fs::remove_file(&outside_new);
    assert.failure().code(predicate::eq(126));
    let v: Value = serde_json::from_str(out.lines().next().unwrap_or("{}")).unwrap();
    let sug = v["suggestion"].as_str().unwrap_or("");
    assert!(
        sug.contains("content-source") || sug.contains(".atomwrite/scratch"),
        "content-source jail suggestion expected staging tip, got: {sug}"
    );
}

#[test]
fn global_select_projects_error_fields() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("t.txt");
    fs::write(&target, "alpha\n").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "--select",
            "ok,error,code,match_count",
            "edit",
            "--old",
            "missing-xyz",
            "--new",
            "x",
            "--fuzzy",
            "off",
            target.to_str().unwrap(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let v: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], true);
    assert!(
        v.get("message").is_none(),
        "select must drop unselected fields: {v}"
    );
    assert!(v.get("code").is_some());
}

#[test]
fn global_filter_and_limit_on_success() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("t.txt");
    fs::write(&target, "hello world\n").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "--filter",
            "match_count=1",
            "--limit",
            "1",
            "--select",
            "match_count,strategy",
            "edit",
            "--old",
            "hello",
            "--new",
            "hola",
            target.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "limit 1: {out}");
    let v: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["match_count"], 1);
    assert!(v.get("path").is_none(), "select dropped path: {v}");
}

#[test]
fn filter_ok_true_does_not_silence_error_envelope() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let target = ws.join("t.txt");
    fs::write(&target, "alpha\n").unwrap();

    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "--filter",
            "ok=true",
            "edit",
            "--old",
            "missing-xyz-no-match",
            "--new",
            "x",
            "--fuzzy",
            "off",
            target.to_str().unwrap(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let line = out
        .lines()
        .next()
        .expect("error envelope must still emit on stdout");
    let v: Value = serde_json::from_str(line).unwrap();
    assert_eq!(
        v["ok"], false,
        "always-emit errors despite filter ok=true: {v}"
    );
    assert_eq!(v["error"], true);
}

#[test]
fn extract_field_flag_projects_fields() {
    // Local `--field` (not global `--select`, which collides at the root).
    let assert = bin()
        .args(["extract", "--field", "a", "--field", "c"])
        .write_stdin(r#"{"a":1,"b":2,"c":3}"#)
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let v: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    assert_eq!(v["a"], 1);
    assert_eq!(v["c"], 3);
    assert!(
        v.get("b").is_none(),
        "extract --field must omit unselected: {v}"
    );
}

#[test]
fn extract_field_conflicts_with_positional() {
    let assert = bin()
        .args(["extract", "--field", "a", "b"])
        .write_stdin(r#"{"a":1,"b":2}"#)
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let v: Value = serde_json::from_str(out.lines().next().unwrap_or("{}")).unwrap();
    assert_eq!(v["error"], true);
    assert_eq!(v["ok"], false);
    let msg = format!("{v}");
    assert!(
        msg.contains("positional")
            || msg.contains("--field")
            || v["code"] == "INVALID_INPUT"
            || v["message"].as_str().is_some_and(|s| s.contains("field")),
        "expected conflict diagnostic: {v}"
    );
}

#[test]
fn global_select_after_extract_full_line_is_agent_native() {
    // Symmetry: global --select projects extract output without local --field.
    let assert = bin()
        .args(["--select", "a,c", "extract"])
        .write_stdin(r#"{"a":1,"b":2,"c":3}"#)
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let v: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    assert_eq!(v["a"], 1);
    assert_eq!(v["c"], 3);
    assert!(v.get("b").is_none(), "global --select on extract: {v}");
}

#[test]
fn scratch_init_creates_staging_dir() {
    let td = TempDir::new().unwrap();
    let ws = td.path();
    let assert = bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "scratch",
            "init",
            ".atomwrite/scratch",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let v: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "scratch");
    assert_eq!(v["action"], "init");
    assert_eq!(v["relative"], ".atomwrite/scratch");
    let path = ws.join(".atomwrite").join("scratch");
    assert!(
        path.is_dir(),
        "scratch dir must exist at {}",
        path.display()
    );
    // idempotent
    bin()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "scratch",
            "init",
            ".atomwrite/scratch",
        ])
        .assert()
        .success();
}
