// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn search_finds_pattern_in_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(
        dir.path(),
        "hello.rs",
        "fn main() {\n    println!(\"hello\");\n}\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "println",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    let matches: Vec<_> = events.iter().filter(|e| e["type"] == "match").collect();
    assert!(!matches.is_empty(), "should find at least one match");
    assert_eq!(matches[0]["lines"], "    println!(\"hello\");");

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert!(summary["total_matches"].as_u64().unwrap() > 0);
}

#[test]
fn search_no_match_exits_1() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "empty.txt", "nothing here\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "NONEXISTENT_PATTERN_xyz123",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn search_count_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "multi.txt", "foo\nbar\nfoo\nbaz\nfoo\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "--count",
            "foo",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let counts: Vec<_> = events.iter().filter(|e| e["type"] == "count").collect();
    assert!(!counts.is_empty());
    assert_eq!(counts[0]["count"], 3);
}

#[test]
fn search_files_only_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.txt", "target\n");
    common::create_test_file(dir.path(), "b.txt", "no match\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "--files",
            "target",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let files: Vec<_> = events.iter().filter(|e| e["type"] == "file").collect();
    assert_eq!(files.len(), 1);
}

#[test]
fn search_ndjson_is_parseable() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "valid.rs", "fn test() {}\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "search", "fn"])
        .arg(dir.path())
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "invalid NDJSON line: {line}");
    }
}

#[test]
fn search_case_insensitive() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "case.txt", "Hello World\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "--case-insensitive",
            "hello",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let matches: Vec<_> = events.iter().filter(|e| e["type"] == "match").collect();
    assert!(!matches.is_empty());
}

// --- GAP 05: --files deduplicação ---

#[test]
fn search_files_dedup_multi_match() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(
        dir.path(),
        "multi.txt",
        "TODO first\nTODO second\nTODO third\nTODO fourth\n",
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "--files",
            "TODO",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let files: Vec<_> = events.iter().filter(|e| e["type"] == "file").collect();
    assert_eq!(
        files.len(),
        1,
        "file with 4 matches should emit exactly 1 file event, got {}",
        files.len()
    );

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["total_matches"], 4);
}

#[test]
fn search_files_dedup_multi_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(
        dir.path(),
        "a.txt",
        "MATCH here\nMATCH again\nMATCH third\n",
    );
    common::create_test_file(dir.path(), "b.txt", "MATCH one\nMATCH two\n");
    common::create_test_file(dir.path(), "c.txt", "no hits here\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "--files",
            "MATCH",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let files: Vec<_> = events.iter().filter(|e| e["type"] == "file").collect();
    assert_eq!(
        files.len(),
        2,
        "2 files with matches should emit exactly 2 file events, got {}",
        files.len()
    );
}

// ============================================================================
// v0.1.20 GAP-2026-010: --no-begin-end suppression
// ============================================================================

/// GAP-010 default: begin/end events emitted for every visited file.
#[test]
fn v0_1_20_search_default_emits_begin_end() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.rs", "alpha");
    common::create_test_file(dir.path(), "b.rs", "beta");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "NEVER_MATCH_THIS",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let begin_count = stdout.matches("\"type\":\"begin\"").count();
    assert_eq!(begin_count, 2, "default: 2 begin events for 2 files");
}

/// GAP-010 --no-begin-end: zero begin/end events when no matches.
#[test]
fn v0_1_20_search_no_begin_end_suppresses_empty_walks() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.txt", "alpha");
    common::create_test_file(dir.path(), "b.txt", "beta");
    common::create_test_file(dir.path(), "c.txt", "gamma");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "--no-begin-end",
            "NEVER_MATCH",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let begin_count = stdout.matches("\"type\":\"begin\"").count();
    let end_count = stdout.matches("\"type\":\"end\"").count();
    assert_eq!(begin_count, 0, "no begin events with --no-begin-end");
    assert_eq!(end_count, 0, "no end events with --no-begin-end");
}
