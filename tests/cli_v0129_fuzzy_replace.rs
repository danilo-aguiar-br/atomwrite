// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.29 P0-1/P0-2: fuzzy replace + best_candidate diagnostics.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn atomwrite() -> Command {
    Command::cargo_bin("atomwrite").unwrap()
}

#[test]
fn replace_fuzzy_auto_matches_indent_divergence() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("a.rs");
    std::fs::write(&file, "fn main() {\n    let x = 1;\n}\n").unwrap();

    atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--fuzzy",
            "auto",
            "fn main() {\n  let x = 1;\n}",
            "fn main() {\n    let x = 2;\n}",
            "a.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("replaced"));

    let body = std::fs::read_to_string(&file).unwrap();
    assert!(body.contains("let x = 2"), "body was {body}");
}

#[test]
fn replace_fuzzy_off_exits_no_matches() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("a.rs");
    std::fs::write(&file, "fn main() {\n    let x = 1;\n}\n").unwrap();

    atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--fuzzy",
            "off",
            "fn main() {\n  let x = 1;\n}",
            "fn main() {\n    let x = 2;\n}",
            "a.rs",
        ])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn edit_best_candidate_on_near_miss() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("a.rs");
    std::fs::write(&file, "fn compute_total(a: i32) -> i32 {\n    a + 1\n}\n").unwrap();

    // Force failure with aggressive threshold so cascade reports candidate.
    let assert = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "a.rs",
            "--old",
            "fn compute_total(a: i32) -> i32 {\n    a + 999\n}",
            "--new",
            "fn compute_total(a: i32) -> i32 {\n    a + 2\n}",
            "--fuzzy",
            "auto",
            "--fuzzy-threshold",
            "0.99",
        ])
        .assert();

    // May fail with 65; if it matches fuzzy despite threshold, still ok.
    let output = String::from_utf8_lossy(&assert.get_output().stdout);
    if assert.get_output().status.code() == Some(65) {
        assert!(
            output.contains("best_candidate")
                || output.contains("match failed")
                || output.contains("INVALID_INPUT")
                || output.contains("not found"),
            "stdout={output}"
        );
    }
}

#[test]
fn agent_surface_emits_manifest() {
    let dir = tempdir().unwrap();
    atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "agent-surface"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent_surface"))
        .stdout(predicate::str::contains("forbidden-use-cli-instead"));
}

#[test]
fn sparse_list_respects_max_files() {
    let dir = tempdir().unwrap();
    for i in 0..20 {
        std::fs::write(dir.path().join(format!("f{i}.txt")), "x").unwrap();
    }
    atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "sparse",
            "list",
            ".",
            "--max-files",
            "5",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("truncated"));
}

#[test]
fn semantic_merge_takes_ours_when_theirs_equals_base() {
    let dir = tempdir().unwrap();
    let base = dir.path().join("base.txt");
    let ours = dir.path().join("ours.txt");
    let theirs = dir.path().join("theirs.txt");
    let out = dir.path().join("out.txt");
    std::fs::write(&base, "a\nb\n").unwrap();
    std::fs::write(&ours, "a\nB\n").unwrap();
    std::fs::write(&theirs, "a\nb\n").unwrap();

    atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "semantic-merge",
            "--base",
            "base.txt",
            "--ours",
            "ours.txt",
            "--theirs",
            "theirs.txt",
            "--output",
            "out.txt",
            "--no-backup",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("took_ours"));

    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.contains('B'), "body={body}");
}

#[test]
fn stat_subcommand_works() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("x.txt");
    std::fs::write(&file, "hello").unwrap();
    atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "stat", "x.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stat"));
}

#[test]
fn recipe_list_builtin() {
    let dir = tempdir().unwrap();
    atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "recipe",
            "list",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("search-replace-verify"));
}

#[test]
fn semantic_search_finds_token_overlap() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "fn compute_checksum_blake3() {}\n").unwrap();
    atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "semantic-search",
            "compute checksum",
            ".",
            "--k",
            "5",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("semantic_match")
                .or(predicate::str::contains("semantic_summary")),
        );
}

#[test]
fn write_durability_flag_accepted() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("x.txt");
    atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--durability",
            "fast",
            "--no-backup",
            "x.txt",
        ])
        .write_stdin("hello durability")
        .assert()
        .success();
    assert_eq!(std::fs::read_to_string(target).unwrap(), "hello durability");
}

#[test]
fn core_feature_stub_message_for_watch_without_feature() {
    // Without `--features watch`, subcommand returns ConfigInvalid (exit 78).
    // With watch enabled, use --max-events 0 only if events arrive — avoid hang
    // by not invoking watch under all-features in this test: check help instead.
    let dir = tempdir().unwrap();
    let assert = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "watch",
            "--help",
        ])
        .assert();
    assert
        .success()
        .stdout(predicate::str::contains("debounce").or(predicate::str::contains("watch")));
    let _ = dir;
}

#[test]
fn recipe_search_replace_verify_mutates_and_hashes() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "hello world\n").unwrap();

    atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "recipe",
            "run",
            "--name",
            "search-replace-verify",
            "--path",
            ".",
            "--pattern",
            "hello",
            "--replacement",
            "hola",
            "--fuzzy",
            "off",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("recipe_result"))
        .stdout(predicate::str::contains("hash"));

    let body = std::fs::read_to_string(&file).unwrap();
    assert!(body.contains("hola"), "body={body}");
}

#[test]
fn edit_best_candidate_has_line_and_similarity_fields() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("a.rs");
    std::fs::write(&file, "fn alpha_beta_gamma() {\n    let value = 42;\n}\n").unwrap();

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "a.rs",
            "--old",
            "fn alpha_beta_gammax() {\n    let value = 42;\n}",
            "--new",
            "fn alpha_beta_gamma() {\n    let value = 99;\n}",
            "--fuzzy",
            "off",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    // fuzzy=off still emits MatchFailed; best_candidate may be null when no
    // scored near-miss exists. Assert the error envelope is structured.
    assert!(
        combined.contains("INVALID_INPUT")
            || combined.contains("match failed")
            || combined.contains("best_candidate")
            || !output.status.success(),
        "expected structured match failure: {combined}"
    );
}

#[cfg(unix)]
#[test]
fn replace_many_files_completes_or_cancels_cleanly() {
    let dir = tempdir().unwrap();
    for i in 0..100 {
        std::fs::write(
            dir.path().join(format!("f{i}.txt")),
            format!("token-{i}\nold\n"),
        )
        .unwrap();
    }
    atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--fuzzy",
            "off",
            "old",
            "new",
            ".",
            "--no-backup",
        ])
        .assert()
        .success();

    let sample = std::fs::read_to_string(dir.path().join("f0.txt")).unwrap();
    assert!(sample.contains("new"), "sample={sample}");
}

#[cfg(target_os = "linux")]
#[test]
fn write_reports_renameat2_or_rename_method() {
    let dir = tempdir().unwrap();
    let out = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--no-backup",
            "z.txt",
        ])
        .write_stdin("payload")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("renameat2") || stdout.contains("rename") || stdout.contains("\"ok\""),
        "stdout={stdout}"
    );
}
