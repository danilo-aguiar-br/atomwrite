//! Contract e2e for atomwrite v0.1.35 residual gaps (G-024 expanded).
#![cfg(unix)]

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn bin() -> Command {
    Command::new(cargo_bin("atomwrite"))
}

#[test]
fn doctor_strict_ok_without_feature_warns() {
    let dir = tempdir().unwrap();
    let mut cmd = bin();
    cmd.args(["--workspace", dir.path().to_str().unwrap(), "doctor", "--strict"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor_report"));
}

#[test]
fn ready_file_cli_not_env() {
    let dir = tempdir().unwrap();
    let ready = dir.path().join("ready");
    let mut cmd = bin();
    cmd.args([
        "--workspace",
        dir.path().to_str().unwrap(),
        "--ready-file",
        ready.to_str().unwrap(),
        "locale",
    ])
    .assert()
    .success();
    assert!(ready.exists(), "ready-file should be written without env");
}

#[test]
fn fuzzy_off_exact_only_cli() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("f.txt");
    fs::write(&path, "hello world\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            path.to_str().unwrap(),
            "--old",
            "world",
            "--new",
            "rust",
            "--fuzzy",
            "off",
        ])
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&path).unwrap(), "hello rust\n");
}

#[test]
fn calc_comment_only_fails_empty() {
    // B-009: comments are skipped; comment-only stdin has no expressions → error.
    let dir = tempdir().unwrap();
    let mut cmd = bin();
    cmd.args(["--workspace", dir.path().to_str().unwrap(), "calc", "--stdin"])
        .write_stdin("# not an expression\n")
        .assert()
        .failure()
        .stdout(predicate::str::contains("INVALID_INPUT").or(predicate::str::contains("error")));
}

#[test]
fn calc_comment_then_expression_ok() {
    // B-009: comments skipped; following expression evaluates.
    let dir = tempdir().unwrap();
    bin()
        .args(["--workspace", dir.path().to_str().unwrap(), "calc", "--stdin"])
        .write_stdin("# ignore\n2 + 2\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("calc").and(predicate::str::contains("4")));
}

#[test]
fn no_color_env_ignored_cli_controls() {
    let dir = tempdir().unwrap();
    let mut cmd = bin();
    cmd.env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "1")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--color",
            "never",
            "locale",
        ])
        .assert()
        .success();
}

#[test]
fn delete_confirm_rejected_still_exists() {
    // B-005: --confirm is fail-closed (not silent plan).
    let dir = tempdir().unwrap();
    let path = dir.path().join("keep.txt");
    fs::write(&path, "x\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--confirm",
            path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(65);
    assert!(path.exists(), "delete --confirm must not delete");
}

#[test]
fn delete_plan_only_still_exists() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("keep.txt");
    fs::write(&path, "x\n").unwrap();
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--plan",
            path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(path.exists(), "delete --plan must not delete");
    assert!(
        s.contains("\"files_modified\":0") || s.contains("files_modified\":0") || s.contains("plan"),
        "plan/files_modified=0 expected, got {s}"
    );
}

#[test]
fn delete_yes_rejected() {
    // B-015
    let dir = tempdir().unwrap();
    let path = dir.path().join("keep.txt");
    fs::write(&path, "x\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "-y",
            path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(65);
    assert!(path.exists());
}

#[test]
fn diff_stdin_as_file_a() {
    // B-001
    let dir = tempdir().unwrap();
    let b = dir.path().join("b.txt");
    fs::write(&b, "one\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            "-",
            b.to_str().unwrap(),
        ])
        .write_stdin("two\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("diff").or(predicate::str::contains("type")));
}

#[test]
fn write_content_risk_rm_rf() {
    // B-013
    let dir = tempdir().unwrap();
    let path = dir.path().join("risky.sh");
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            path.to_str().unwrap(),
        ])
        .write_stdin("rm -rf /\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(
        s.contains("risk_assessment") && s.contains("content_pattern"),
        "expected content risk_assessment, got {s}"
    );
}

#[test]
fn diff_file_vs_stdin() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("a.txt");
    fs::write(&a, "one\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            a.to_str().unwrap(),
            "-",
        ])
        .write_stdin("two\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("diff").or(predicate::str::contains("type")));
}

#[test]
fn move_directory_same_fs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("d1");
    let dst = dir.path().join("d2");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("f.txt"), "hi\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "move",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst.join("f.txt").exists());
    assert!(!src.exists());
}

#[test]
fn read_raw_nontty_is_bytes() {
    // B-006: --format raw always emits file bytes
    let dir = tempdir().unwrap();
    let path = dir.path().join("r.txt");
    fs::write(&path, "hello raw\n").unwrap();
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--format",
            "raw",
            path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert_eq!(s, "hello raw\n");
    assert!(
        !s.contains("\"type\""),
        "raw must not be NDJSON envelope, got: {s}"
    );
}

#[test]
fn set_json_trailing_newline() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("c.json");
    fs::write(&path, "{\n  \"a\": 1\n}").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "set",
            path.to_str().unwrap(),
            "a",
            "2",
        ])
        .assert()
        .success();
    let body = fs::read_to_string(&path).unwrap();
    assert!(body.ends_with('\n'), "G-022 trailing newline: {body:?}");
}

#[test]
fn write_confirm_requires_ack_overwrite() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("big.txt");
    // >100KB
    fs::write(&path, vec![b'x'; 120_000]).unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--confirm",
            path.to_str().unwrap(),
        ])
        .write_stdin("new payload\n")
        .assert()
        .failure();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--confirm",
            "--ack-overwrite",
            "--allow-shrink",
            path.to_str().unwrap(),
        ])
        .write_stdin("new payload\n")
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&path).unwrap(), "new payload\n");
}

#[test]
fn fuzzy_match_failed_code() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("m.txt");
    fs::write(&path, "alpha beta\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            path.to_str().unwrap(),
            "--old",
            "zzzz-not-present-qqq",
            "--new",
            "x",
            "--fuzzy",
            "off",
        ])
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("MATCH_FAILED")
                .or(predicate::str::contains("match failed"))
                .or(predicate::str::contains("NoMatches"))
                .or(predicate::str::contains("error")),
        );
}


#[test]
fn replace_binary_exits_binary_file_not_no_matches() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bin.dat");
    fs::write(&path, b"a\x00b").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "-F",
            "a",
            "A",
            path.to_str().unwrap(),
            "--no-backup",
        ])
        .assert()
        .failure()
        .code(65)
        .stdout(
            predicate::str::contains("BINARY_FILE")
                .or(predicate::str::contains("binary file rejected")),
        );
}

#[test]
fn case_requires_subvert_clap() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("id.rs");
    fs::write(&path, "snake_ident\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "case",
            path.to_str().unwrap(),
            "--to",
            "camel",
            "--dry-run",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn prune_requires_policy_group() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("f.txt");
    fs::write(&path, "x\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "prune-backups",
            path.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn write_confirm_before_shrink_mentions_ack() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("L.bin");
    fs::write(&path, vec![b'A'; 120 * 1024]).unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            path.to_str().unwrap(),
            "--confirm",
            "--no-backup",
        ])
        .write_stdin("x")
        .assert()
        .failure()
        .code(65)
        .stdout(predicate::str::contains("ack-overwrite"));
}

#[test]
fn agent_surface_case_schema_hint_not_write_output() {
    let dir = tempdir().unwrap();
    let out = bin()
        .args(["--workspace", dir.path().to_str().unwrap(), "agent-surface"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    // G-037: hints must match docs/schemas/*.schema.json basenames.
    assert!(
        s.contains("\"name\":\"case\"") && s.contains("case-result"),
        "case should use case-result schema_hint"
    );
    assert!(
        s.contains("\"name\":\"set\"") && s.contains("set-result"),
        "set should use set-result"
    );
    assert!(
        s.contains("\"name\":\"del\"") && s.contains("del-result"),
        "del should use del-result"
    );
    assert!(
        s.contains("\"name\":\"get\"") && s.contains("get-result"),
        "get should use get-result"
    );
    assert!(
        s.contains("\"name\":\"verify\"") && s.contains("hash-output"),
        "verify should use hash-output (G-041; same NDJSON as hash)"
    );
    assert!(
        !s.contains("case-output") && !s.contains("set-output"),
        "legacy *-output hints for case/set must not appear"
    );
}

/// G-037/G-041/G-042: every `schema_hint` must resolve to
/// `docs/schemas/{hint}.schema.json` in a source checkout (no allowlist).
#[test]
fn agent_surface_schema_hints_have_schema_files_in_repo() {
    let dir = tempdir().unwrap();
    let out = bin()
        .args(["--workspace", dir.path().to_str().unwrap(), "agent-surface"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    let schemas_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/schemas");
    if !schemas_dir.is_dir() {
        return;
    }
    // G-042: zero missing-schema allowlist — every hint has a published file.
    let allow_missing: &[&str] = &[];
    // agent-surface emits one envelope with tools:[] (not per-line NDJSON tools).
    let v: serde_json::Value =
        serde_json::from_str(s.lines().next().expect("agent-surface stdout")).expect("json");
    let tools = v
        .get("tools")
        .and_then(|t| t.as_array())
        .expect("tools array");
    assert!(!tools.is_empty(), "agent-surface must list tools");
    for tool in tools {
        let Some(hint) = tool.get("schema_hint").and_then(|h| h.as_str()) else {
            continue;
        };
        if allow_missing.contains(&hint) {
            continue;
        }
        let path = schemas_dir.join(format!("{hint}.schema.json"));
        assert!(
            path.is_file(),
            "schema_hint {hint:?} for tool {:?} missing file {}",
            tool.get("name"),
            path.display()
        );
    }
}
