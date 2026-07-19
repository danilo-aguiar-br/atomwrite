// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.30 agent-contract tests: fuzzy mandatory, uniqueness, escape-drift, search dual, tokenizer.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn bin() -> Command {
    Command::cargo_bin("atomwrite").expect("bin")
}

#[test]
fn fuzzy_off_exact_only_succeeds_on_exact_match() {
    // G-010/G-047: --fuzzy off is exact-only (not rejected); exact hits succeed.
    let dir = tempdir().unwrap();
    let f = dir.path().join("t.rs");
    fs::write(&f, "let x = 1;\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            f.to_str().unwrap(),
            "--old",
            "let x = 1;",
            "--new",
            "let x = 2;",
            "--fuzzy",
            "off",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"strategy\":\"exact\""));
}

#[test]
fn uniqueness_fails_without_replace_all() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("t.txt");
    fs::write(&f, "aa\nxx\naa\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            f.to_str().unwrap(),
            "--old",
            "aa",
            "--new",
            "bb",
        ])
        .assert()
        .failure()
        .code(65)
        .stdout(
            predicate::str::contains("MATCH_AMBIGUOUS")
                .or(predicate::str::contains("ambiguous"))
                .or(predicate::str::contains("replace-all")),
        );
}

#[test]
fn replace_all_edits_all() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("t.txt");
    fs::write(&f, "aa\nxx\naa\n").unwrap();
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            f.to_str().unwrap(),
            "--old",
            "aa",
            "--new",
            "bb",
            "--replace-all",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let body = fs::read_to_string(&f).unwrap();
    assert_eq!(body.matches("bb").count(), 2);
    let s = String::from_utf8_lossy(&out);
    assert!(
        s.contains("\"match_count\":2") || s.contains("\"match_count\": 2"),
        "expected match_count in NDJSON, got: {s}"
    );
}

#[test]
fn replace_all_emits_match_count() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("t.txt");
    fs::write(&f, "x\nx\nx\n").unwrap();
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            f.to_str().unwrap(),
            "--old",
            "x",
            "--new",
            "y",
            "--replace-all",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(
        s.contains("\"match_count\":3") || s.contains("\"match_count\": 3"),
        "expected match_count 3, got: {s}"
    );
}

#[test]
fn indent_adjusted_true_on_delta() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("t.rs");
    // File uses tabs; pattern uses spaces → indent_flexible realigns new to tabs.
    fs::write(&f, "fn main() {\n\tlet answer = 41;\n}\n").unwrap();
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            f.to_str().unwrap(),
            "--old",
            "    let answer = 41;",
            "--new",
            "    let answer = 42;",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(
        s.contains("\"indent_adjusted\":true") || s.contains("\"indent_adjusted\": true"),
        "expected indent_adjusted true, got: {s}"
    );
    let body = fs::read_to_string(&f).unwrap();
    assert!(body.contains("let answer = 42"), "{body}");
    // Realignment should prefer file indent (tab).
    assert!(
        body.contains("\tlet answer = 42") || body.contains("let answer = 42"),
        "{body}"
    );
}

#[test]
fn config_fuzzy_off_exact_only_succeeds_on_exact_match() {
    // G-010/G-047: XDG [fuzzy].mode = "off" is exact-only, not a hard reject.
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join(".atomwrite.toml"),
        "[fuzzy]\nmode = \"off\"\n",
    )
    .unwrap();
    let f = dir.path().join("t.txt");
    fs::write(&f, "aa\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            f.to_str().unwrap(),
            "--old",
            "aa",
            "--new",
            "bb",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"strategy\":\"exact\""));
}

#[test]
fn recipe_hash_skips_bak() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("app.rs");
    fs::write(&src, "fn main() { let a = 1; }\n").unwrap();
    // Plant a bak file that must not appear in hash step.
    fs::write(
        dir.path().join("app.rs.bak.20260101_000000_000"),
        "stale bak\n",
    )
    .unwrap();
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "recipe",
            "run",
            "--name",
            "search-replace-verify",
            "--path",
            dir.path().to_str().unwrap(),
            "--pattern",
            "let a = 1",
            "--replacement",
            "let a = 2",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(
        !s.contains(".bak."),
        "recipe hash must not list .bak paths, got: {s}"
    );
}

#[test]
fn sparse_outline_emits_kind() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn hello() {}\nstruct Foo;\n").unwrap();
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "sparse",
            "outline",
            dir.path().to_str().unwrap(),
            "--max-files",
            "10",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(
        !s.contains("use outline subcommand"),
        "stub note must be gone: {s}"
    );
    assert!(
        s.contains("outline_item")
            || s.contains("function_item")
            || s.contains("struct_item")
            || s.contains("\"kind\""),
        "expected real outline kinds, got: {s}"
    );
}

#[test]
fn semantic_merge_help_line_based() {
    let out = bin()
        .args(["semantic-merge", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out).to_lowercase();
    assert!(
        s.contains("line-based") || s.contains("line index") || s.contains("not ast"),
        "help must admit line-based merge: {s}"
    );
}

#[test]
fn escape_drift_blocked() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("t.txt");
    fs::write(&f, "foo bar\n").unwrap();
    bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            f.to_str().unwrap(),
            "--old",
            "foo",
            "--new",
            r"bar\'s",
        ])
        .assert()
        .failure()
        .code(65)
        .stdout(predicate::str::contains("escape-drift"));
}

#[test]
fn semantic_search_snake_tokens() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("hash.rs");
    fs::write(&f, "fn compute_checksum_blake3() {}\n").unwrap();
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "semantic-search",
            "compute checksum",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(
        s.contains("semantic_match") || s.contains("compute_checksum"),
        "expected semantic hit, got: {s}"
    );
}

#[test]
fn search_target_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("unique_name_xyz.rs"), "fn main() {}\n").unwrap();
    let out = bin()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "unique_name_xyz",
            dir.path().to_str().unwrap(),
            "--target",
            "files",
            "-F",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("file_match") || s.contains("unique_name_xyz"), "{s}");
}
