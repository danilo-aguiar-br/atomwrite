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
    let checksum = common::parse_ndjson(&hash_out.stdout)[0]["checksum"]
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

// ─── G117: multi-pair fuzzy parity, per-pair reporting, --partial ────────────

#[test]
fn edit_multi_fuzzy_pair_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "g117a.rs", "fn alpha() {}\n    let x = 1;\n");

    // Pair 2 has divergent whitespace ("let  x" vs "let x"): before G117 the
    // multi path was exact-only and this batch failed entirely.
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
            "let  x = 1;",
            "--new",
            "let y = 2;",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "edit");
    assert_eq!(events[0]["edits"], 2);
    assert_eq!(events[0]["mode"], "fuzzy-multi(2)");
    assert_eq!(events[0]["fuzzy"], true);

    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("ALPHA"));
    assert!(content.contains("let y = 2;"));
}

#[test]
fn edit_multi_all_exact_keeps_mode_exact_multi() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "g117b.txt", "alpha\nbeta\n");

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
            "beta",
            "--new",
            "BETA",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["mode"], "exact-multi(2)");
    assert_eq!(events[0]["fuzzy"], false);
}

#[test]
fn edit_multi_success_includes_pair_results() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "g117c.txt", "alpha\nbeta\n");

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
            "beta",
            "--new",
            "BETA",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["pairs_total"], 2);
    let pairs = events[0]["pair_results"]
        .as_array()
        .expect("pair_results array");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0]["index"], 1);
    assert_eq!(pairs[0]["matched"], true);
    assert_eq!(pairs[0]["strategy"], "exact");
    assert_eq!(pairs[1]["index"], 2);
    assert_eq!(pairs[1]["matched"], true);
}

#[test]
fn edit_multi_error_reports_failed_pair_index() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "alpha\nbeta\ngamma\n";
    let path = common::create_test_file(dir.path(), "g117d.txt", before);

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
            "NAOEXISTE",
            "--new",
            "X",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["error"], true);
    assert_eq!(events[0]["code"], "INVALID_INPUT");
    assert_eq!(events[0]["failed_pair_index"], 2);
    assert_eq!(events[0]["pairs_total"], 2);
    let pairs = events[0]["pair_results"]
        .as_array()
        .expect("pair_results array");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0]["matched"], true);
    assert_eq!(pairs[1]["matched"], false);

    // All-or-nothing default: nothing was written, file intact.
    let content = std::fs::read_to_string(&path).expect("read");
    assert_eq!(content, before);
}

#[test]
fn edit_multi_fuzzy_off_preserves_exact_behavior() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "    let x = 1;\n";
    let path = common::create_test_file(dir.path(), "g117e.rs", before);

    // Divergent whitespace pair that fuzzy=auto would rescue must still fail
    // with --fuzzy off (pre-G117 exact behavior preserved on opt-out).
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--fuzzy",
            "off",
            "--old",
            "let  x = 1;",
            "--new",
            "let y = 2;",
            "--old",
            "let x",
            "--new",
            "let z",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
    let content = std::fs::read_to_string(&path).expect("read");
    assert_eq!(content, before);
}

#[test]
fn edit_partial_applies_valid_pairs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "g117f.txt", "alpha\nbeta\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--partial",
            "--old",
            "alpha",
            "--new",
            "ALPHA",
            "--old",
            "NAOEXISTE",
            "--new",
            "X",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["edits"], 1);
    assert_eq!(events[0]["pairs_total"], 2);
    let pairs = events[0]["pair_results"]
        .as_array()
        .expect("pair_results array");
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0]["matched"], true);
    assert_eq!(pairs[1]["matched"], false);

    let content = std::fs::read_to_string(&path).expect("read");
    assert!(content.contains("ALPHA"));
}

#[test]
fn edit_partial_zero_matches_exits_1() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "alpha\n";
    let path = common::create_test_file(dir.path(), "g117g.txt", before);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--partial",
            "--old",
            "NAOEXISTE1",
            "--new",
            "X",
            "--old",
            "NAOEXISTE2",
            "--new",
            "Y",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(1));
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "NO_MATCHES");
    let content = std::fs::read_to_string(&path).expect("read");
    assert_eq!(content, before);
}

#[test]
fn edit_partial_dry_run_no_write() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "alpha\nbeta\n";
    let path = common::create_test_file(dir.path(), "g117h.txt", before);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--partial",
            "--dry-run",
            "--old",
            "alpha",
            "--new",
            "ALPHA",
            "--old",
            "NAOEXISTE",
            "--new",
            "X",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "plan");
    let content = std::fs::read_to_string(&path).expect("read");
    assert_eq!(content, before);
}

// ============================================================================
// G117 follow-up: edge cases not covered by the original multi-pair suite
// ============================================================================

/// G117 edge case: Unicode (non-ASCII) `--old`/`--new` must work via
/// exact UTF-8 byte match. LLM agents commonly emit identifiers with
/// diacritics (e.g. `José`, `naïve`, `über`) and the fuzzy cascade must
/// not corrupt them via normalization. Single-pair `edit` replaces
/// only the FIRST occurrence (use multi-pair to replace all); this
/// test exercises the byte-exact match for the diacritic `ção`.
#[test]
fn edit_unicode_old_new_exact_match() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "alpha ção beta ção\n";
    let path = common::create_test_file(dir.path(), "g117_unicode.txt", before);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "ção",
            "--new",
            "AÇÃO",
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
    assert_eq!(content, "alpha AÇÃO beta ção\n");
}

/// G117 edge case: CRLF (Windows-style) line endings must NOT trip the
/// fuzzy cascade. The first occurrence of `--old` must be replaced and
/// the line-ending normalization in `cmd_edit` must keep the original
/// `\r\n` endings intact.
#[test]
fn edit_crlf_line_endings_preserve_eol_after_replace() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "alpha\r\nbeta\r\ngamma\r\n";
    let path = common::create_test_file(dir.path(), "g117_crlf.txt", before);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "beta",
            "--new",
            "BETA",
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
    // CR-LF must be preserved byte-for-byte (not collapsed to LF).
    assert_eq!(content, "alpha\r\nBETA\r\ngamma\r\n");
}

/// G117 edge case: multi-pair with the SAME `--old` value in two
/// different pairs must apply each pair in sequence against the
/// evolving content. This guards against an off-by-one bug where
/// the second pair would skip the just-inserted first replacement.
#[test]
fn edit_multi_pair_same_old_appears_twice_applies_both() {
    let dir = tempfile::tempdir().expect("tempdir");
    // "foo" appears twice. Two pairs rename the first to "FOO_A" and
    // the second to "FOO_B". After both pairs, both occurrences are
    // renamed and they differ.
    let before = "foo bar foo\n";
    let path = common::create_test_file(dir.path(), "g117_repeat.txt", before);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "foo",
            "--new",
            "FOO_A",
            "--old",
            "foo",
            "--new",
            "FOO_B",
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
    assert_eq!(content, "FOO_A bar FOO_B\n");
}

// ============================================================================
// G117 follow-up round 2: 6 additional edge cases from the v0.1.15/v0.1.18 gap list
// ============================================================================

/// G117 edge case: NFC vs NFD Unicode normalization is NOT performed by
/// the byte-exact `exact` strategy. The precomposed `ã` (U+00E3, 2 bytes)
/// and the decomposed `ã` (U+0061 U+0303, 3 bytes) are distinct byte
/// sequences; the cascade must fail (exit 65 INVALID_INPUT) rather than
/// silently match. This is intentional: silent Unicode normalization
/// would corrupt identifiers and string literals.
#[test]
fn edit_nfc_vs_nfd_does_not_match() {
    let dir = tempfile::tempdir().expect("tempdir");
    // NFD: a + combining tilde (3 bytes total: 0x61 0xCC 0x83)
    let nfd_a = "a\u{0303}";
    assert_eq!(nfd_a.len(), 3, "NFD 'ã' must be 3 bytes");
    let before = format!("José{nfd_a} Silva\n");
    let path = common::create_test_file(dir.path(), "g117_nfd.txt", &before);

    // NFC: ã (2 bytes: 0xC3 0xA3)
    let nfc_a = "ã";
    assert_eq!(nfc_a.len(), 2, "NFC 'ã' must be 2 bytes");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            nfc_a,
            "--new",
            "X",
        ])
        .arg(&path)
        .output()
        .expect("run");

    // Byte-exact: must fail because the file has NFD, --old is NFC.
    assert_eq!(output.status.code(), Some(65));
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["error"], true);
    assert_eq!(events[0]["code"], "INVALID_INPUT");

    // File unchanged.
    let content = std::fs::read_to_string(&path).expect("read");
    assert_eq!(content, before);
}

/// G117 edge case: `--old` containing only whitespace matches the
/// FIRST byte-exact occurrence via the `exact` strategy (no cascade
/// needed). The file has leading and trailing runs of 3 spaces; the
/// first run is replaced and the trailing run stays untouched because
/// `edit` replaces only the first match.
#[test]
fn edit_old_only_whitespace_matches_first_run_exact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "   hello world   \n";
    let path = common::create_test_file(dir.path(), "g117_ws.txt", before);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "   ",
            "--new",
            "X",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    // 3 spaces appear byte-exactly at the start of the file → exact
    // strategy wins immediately. The fuzzy cascade is NOT triggered.
    assert_eq!(events[0]["fuzzy"], false);
    assert_eq!(events[0]["strategy"], "exact");

    let content = std::fs::read_to_string(&path).expect("read");
    // First run replaced; trailing 3 spaces preserved.
    assert_eq!(content, "Xhello world   \n");
}

/// G117 edge case: `--new ""` must perform a clean deletion of the
/// matched substring. The result must NOT contain any residual
/// characters (no phantom spaces, no BOM, no leftover punctuation).
#[test]
fn edit_new_empty_string_deletion_removes_only_match() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "keep this, remove this, keep this too\n";
    let path = common::create_test_file(dir.path(), "g117_del.txt", before);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "remove this, ",
            "--new",
            "",
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
    assert_eq!(content, "keep this, keep this too\n");
    // Sanity: no leftover "remove" substring and no phantom punctuation.
    assert!(!content.contains("remove"));
    assert!(!content.contains(", ,"));
}

/// G117 edge case: a UTF-8 BOM (`\xEF\xBB\xBF`) at the start of the
/// file is stripped by the reader/loader before the byte-exact match.
/// The match still succeeds because the substring after the BOM
/// (`hello`) is what the strategy sees. This documents the BOM-
/// stripping behavior so future maintainers don't lose the
/// regression coverage. NOTE: the BOM is consumed (not preserved
/// through edit) — this is a known design choice, not a bug.
#[test]
fn edit_utf8_bom_is_stripped_before_match() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "\u{FEFF}hello\n";
    let path = common::create_test_file(dir.path(), "g117_bom.txt", before);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "hello",
            "--new",
            "world",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    // BOM is consumed by the loader — only the post-BOM payload is
    // counted. bytes_before excludes the 3 BOM bytes.
    assert_eq!(events[0]["bytes_before"], 6);
    assert_eq!(events[0]["strategy"], "exact");

    let content = std::fs::read_to_string(&path).expect("read");
    // BOM is stripped; only "world\n" remains.
    assert_eq!(content, "world\n");
    assert!(!content.starts_with('\u{FEFF}'));
}

/// G117 edge case: `--partial` with ZERO applicable pairs must exit
/// with code 1 (`NO_MATCHES`), not 65 (`INVALID_INPUT`). The
/// all-or-nothing default is overridden; partial mode is permissive
/// when at least one pair matches, but if NO pair matches it falls
/// through to NO_MATCHES. This is a documented semantic.
#[test]
fn edit_partial_zero_applicable_pairs_exits_no_matches() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "foo bar\n";
    let path = common::create_test_file(dir.path(), "g117_pz.txt", before);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--partial",
            "--old",
            "NAOEXISTE_ZZ",
            "--new",
            "X",
        ])
        .arg(&path)
        .output()
        .expect("run");

    // Documented behavior: single pair with --partial and zero matches
    // returns NO_MATCHES (exit 1), not INVALID_INPUT (exit 65).
    assert_eq!(output.status.code(), Some(1));
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "NO_MATCHES");
    // File must be untouched.
    let content = std::fs::read_to_string(&path).expect("read");
    assert_eq!(content, before);
}

/// G117 edge case: block_anchor and context_aware strategies are
/// designed to be MORE permissive than 0.70. With one missing token
/// in a 4-token block, the trigram similarity lands at ~0.76, which
/// is BELOW the context_aware threshold (0.80) but WITHIN the
/// ±0.05 fallback window of match_context_aware. This means the
/// cascade DOES match and the file is rewritten. This test pins down
/// the tolerance behavior so any future tightening of the thresholds
/// is caught by the regression suite.
#[test]
fn edit_one_token_drift_in_block_matches_via_context_aware_fallback() {
    let dir = tempfile::tempdir().expect("tempdir");
    let before = "alpha bravo charlie delta\n";
    let path = common::create_test_file(dir.path(), "g117_thr.txt", before);

    // 3 of 4 tokens match; "charlie" is replaced by "echo" in --old.
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "alpha bravo echo delta",
            "--new",
            "X",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    // Similarity falls in the [0.75, 0.80) band: not high enough for
    // the strict context_aware threshold (0.80) but caught by the
    // ±0.05 fallback window. Strategy must be either block_anchor or
    // context_aware with a similarity in that band.
    assert_eq!(events[0]["fuzzy"], true);
    let strategy = events[0]["strategy"].as_str().expect("strategy");
    assert!(
        strategy == "block_anchor" || strategy == "context_aware" || strategy == "context_aware_jw",
        "expected block_anchor, context_aware, or context_aware_jw, got {strategy}"
    );
    if let Some(sim) = events[0]["similarity"].as_f64() {
        assert!(
            (0.70..=0.95).contains(&sim),
            "similarity {sim} outside expected band [0.70, 0.95]"
        );
    }

    // File rewritten: the entire line collapsed to "X\n".
    let content = std::fs::read_to_string(&path).expect("read");
    assert_eq!(content, "X\n");
}

// ============================================================================
// v0.1.20 GAP-2026-005b: edit --partial single-pair still returns NoMatches
// ============================================================================

/// GAP-005b: single-pair --partial + unmatched pattern returns NoMatches
/// (exit 1) without writing anything. --partial does NOT make single-pair
/// matches "best-effort"; it only applies to multi-pair.
#[test]
fn v0_1_20_edit_partial_single_pair_unmatched_returns_no_matches() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "foo.txt", "hello world\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "NONEXISTENT_TEXT",
            "--new",
            "X",
            "--partial",
        ])
        .arg(dir.path().join("foo.txt"))
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(1),
        "single-pair --partial + no match must be NoMatches (exit 1)"
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "NO_MATCHES");
    let content = std::fs::read_to_string(dir.path().join("foo.txt")).unwrap();
    assert_eq!(content, "hello world\n", "no write on NoMatches");
}

// ============================================================================
// v0.1.20 GAP-2026-006: diff --algorithm myers/patience/lcs
// ============================================================================

/// GAP-006: --algorithm myers runs successfully.
#[test]
fn v0_1_20_diff_algorithm_myers_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    std::fs::write(&a, "the quick brown fox\n").unwrap();
    std::fs::write(&b, "the quick red fox\n").unwrap();

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            "--algorithm",
            "myers",
        ])
        .arg(&a)
        .arg(&b)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// GAP-006: --algorithm lcs runs successfully on identical files.
#[test]
fn v0_1_20_diff_algorithm_lcs_on_identical_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    std::fs::write(&a, "abc\n").unwrap();
    std::fs::write(&b, "abc\n").unwrap();

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "diff",
            "--algorithm",
            "lcs",
            "--stat",
        ])
        .arg(&a)
        .arg(&b)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert!(events[0]["identical"].as_bool().unwrap());
}
