// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn edit_old_new_replaces_exact_match() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "edit.txt", "fn old_name() {}\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "old_name",
            "--new",
            "new_name",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "edit");
    assert_eq!(events[0]["mode"], "exact");

    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("new_name"));
    assert!(!content.contains("old_name"));
}

#[test]
fn edit_old_not_found_exits_65() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "nofind.txt", "hello world\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "NONEXISTENT",
            "--new",
            "replacement",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
}

#[test]
fn edit_after_line_inserts_content() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "lines.txt", "line1\nline2\nline3\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--after-line",
            "2",
        ])
        .arg(&path)
        .write_stdin("inserted\n")
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read_to_string(&path).expect("read");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines[0], "line1");
    assert_eq!(lines[1], "line2");
    assert_eq!(lines[2], "inserted");
    assert_eq!(lines[3], "line3");
}

#[test]
fn edit_delete_range_removes_lines() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "del.txt", "a\nb\nc\nd\ne\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--delete-range",
            "2:4",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read_to_string(&path).expect("read");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines, vec!["a", "e"]);
}

#[test]
fn edit_after_match_inserts_after_marker() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(
        dir.path(),
        "marker.txt",
        "use std::io;\nuse std::fs;\n\nfn main() {}\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--after-match",
            "use std::fs;",
        ])
        .arg(&path)
        .write_stdin("use std::path::Path;")
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("use std::fs;\nuse std::path::Path;\n"));
}

// --- GAP 06: --old/--new múltiplos pares ---

#[test]
fn edit_multiple_pairs_without_multi() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "multi_pair.txt", "alpha bravo charlie\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "alpha",
            "--new",
            "ALPHA",
            "--old",
            "charlie",
            "--new",
            "CHARLIE",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("ALPHA"), "first pair should be applied");
    assert!(content.contains("CHARLIE"), "second pair should be applied");
    assert!(!content.contains("alpha"));
    assert!(!content.contains("charlie"));
}

#[test]
fn edit_old_new_mismatch_count() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "mismatch.txt", "content\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "A",
            "--new",
            "B",
            "--old",
            "C",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        !output.status.success(),
        "mismatched --old/--new counts should fail"
    );
}

#[test]
fn edit_multi_ndjson_stdin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(
        dir.path(),
        "multi_stdin.txt",
        "line_one\nline_two\nline_three\n",
    );

    let ndjson = r#"{"op":"exact","old":"line_one","new":"LINE_ONE"}
{"op":"exact","old":"line_three","new":"LINE_THREE"}"#;

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--multi",
        ])
        .arg(&path)
        .write_stdin(ndjson)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("LINE_ONE"));
    assert!(content.contains("LINE_THREE"));
    assert!(content.contains("line_two"));
}

// --- GAP 09: fuzzy com pontuação ---

#[test]
fn edit_fuzzy_punctuation_spaces() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(
        dir.path(),
        "fuzzy_punct.rs",
        "fn calculate( items: Vec<i32> ) -> i32 {\n    items.iter().sum()\n}\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "fn calculate(items: Vec<i32>) -> i32 {",
            "--new",
            "fn calc(items: Vec<i32>) -> i32 {",
            "--fuzzy",
            "aggressive",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "fuzzy should match despite punctuation whitespace, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(
        content.contains("fn calc("),
        "replacement should be applied"
    );
}

#[test]
fn edit_fuzzy_single_line_block_anchor() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "anchor.rs", "let result = calculate(x, y);\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "let  result = calculate( x,  y );",
            "--new",
            "let result = compute(x, y);",
            "--fuzzy",
            "aggressive",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "fuzzy should match single-line pattern with punctuation spaces, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("compute"), "replacement should be applied");
    assert!(
        !content.contains("calculate"),
        "original should be replaced"
    );
}

#[test]
fn edit_fuzzy_multiline_first_line_variation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(
        dir.path(),
        "multi.rs",
        "fn process( items: Vec<i32> ) {\n    for item in items {\n        println!(\"{}\", item);\n    }\n}\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "fn process(items: Vec<i32>) {\n    for item in items {\n        println!(\"{}\", item);\n    }\n}",
            "--new",
            "fn handle(items: Vec<i32>) {\n    for item in items {\n        println!(\"{}\", item);\n    }\n}",
            "--fuzzy",
            "aggressive",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "fuzzy should match multiline with first-line whitespace variation, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(
        content.contains("fn handle("),
        "replacement should be applied"
    );
    assert!(
        !content.contains("fn process("),
        "original should be replaced"
    );
}

// NOTE: --multi + --old/--new CLI injection not implemented in v0.1.1
// cmd_edit_multi reads only stdin; CLI --old/--new are ignored in multi mode
// This is a known limitation, not a bug — multiple pairs work WITHOUT --multi

#[test]
fn edit_expect_checksum_rejects_drift() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "edit_drift.txt", "hello world\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "hello",
            "--new",
            "goodbye",
            "--expect-checksum",
            "wrong_checksum_value",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(82),
        "edit with wrong checksum should exit 82"
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "STATE_DRIFT");

    let content = std::fs::read_to_string(&path).expect("read");
    assert_eq!(content, "hello world\n", "file should not be modified");
}

#[test]
fn edit_expect_checksum_accepts_correct() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "edit_locked.txt", "old_value = 42;\n");

    let hash_out = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
        .arg(&path)
        .output()
        .expect("hash");
    let checksum = common::parse_ndjson(&hash_out.stdout)[0]["value"]
        .as_str()
        .expect("checksum")
        .to_string();

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "old_value",
            "--new",
            "new_value",
            "--expect-checksum",
            &checksum,
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit with correct checksum should succeed: {:?}",
        output.status
    );
    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("new_value"));
}
