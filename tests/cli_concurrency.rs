// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn search_threads_1_produces_correct_results() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..50 {
        common::create_test_file(dir.path(), &format!("file_{i}.txt"), "MARKER_LINE\n");
    }

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "--threads",
            "1",
            "MARKER_LINE",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary event");
    assert_eq!(
        summary["files_matched"].as_u64().unwrap(),
        50,
        "all 50 files should match with --threads 1"
    );
    assert_eq!(
        summary["total_matches"].as_u64().unwrap(),
        50,
        "each file has exactly one match"
    );
}

#[test]
fn search_default_threads_matches_threads_1() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..50 {
        common::create_test_file(dir.path(), &format!("det_{i}.txt"), "DETERMINISM_CHECK\n");
    }
    let ws = dir.path().to_str().unwrap();

    let output_seq = common::atomwrite()
        .args([
            "--workspace",
            ws,
            "search",
            "--threads",
            "1",
            "DETERMINISM_CHECK",
        ])
        .arg(dir.path())
        .output()
        .expect("run seq");

    let output_par = common::atomwrite()
        .args(["--workspace", ws, "search", "DETERMINISM_CHECK"])
        .arg(dir.path())
        .output()
        .expect("run par");

    assert!(output_seq.status.success());
    assert!(output_par.status.success());

    let events_seq = common::parse_ndjson(&output_seq.stdout);
    let events_par = common::parse_ndjson(&output_par.stdout);

    let sum_seq = events_seq.iter().find(|e| e["type"] == "summary").unwrap();
    let sum_par = events_par.iter().find(|e| e["type"] == "summary").unwrap();

    assert_eq!(
        sum_seq["files_matched"], sum_par["files_matched"],
        "files_matched must be deterministic across thread counts"
    );
    assert_eq!(
        sum_seq["total_matches"], sum_par["total_matches"],
        "total_matches must be deterministic across thread counts"
    );
}

#[test]
fn replace_threads_1_applies_all() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..20 {
        common::create_test_file(dir.path(), &format!("rep_{i}.txt"), "OLD_TOKEN here\n");
    }

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--threads",
            "1",
            "--dry-run",
            "OLD_TOKEN",
            "NEW_TOKEN",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "exit: {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary event");
    assert!(
        summary["files_matched"].as_u64().unwrap() >= 20,
        "all 20 files should match with --threads 1"
    );
}

#[test]
fn scope_with_shutdown_check_no_regression() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..10 {
        common::create_test_file(
            dir.path(),
            &format!("func_{i}.rs"),
            &format!("fn test_{i}() {{}}\n"),
        );
    }

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "scope",
            "--language",
            "rust",
            "--query",
            "fn",
            "--delete",
            "--dry-run",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "exit: {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary event");
    assert!(
        summary["files_matched"].as_u64().unwrap() >= 1,
        "scope with shutdown check should still find matches"
    );
}
