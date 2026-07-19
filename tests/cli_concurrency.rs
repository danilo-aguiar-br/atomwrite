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

#[test]
fn max_concurrency_alias_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..10 {
        common::create_test_file(dir.path(), &format!("mc_{i}.txt"), "MAXC_MARK\n");
    }
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--max-concurrency",
            "2",
            "search",
            "MAXC_MARK",
        ])
        .arg(dir.path())
        .output()
        .expect("run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["files_matched"].as_u64().unwrap(), 10);
}

#[test]
fn multi_file_hash_parallel_stable() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..8 {
        common::create_test_file(dir.path(), &format!("h_{i}.bin"), &format!("payload-{i}\n"));
    }
    let ws = dir.path().to_str().unwrap();
    let paths: Vec<_> = (0..8)
        .map(|i| dir.path().join(format!("h_{i}.bin")))
        .collect();

    let out1 = common::atomwrite()
        .args(["--workspace", ws, "--threads", "1", "hash"])
        .args(&paths)
        .output()
        .expect("hash t1");
    let outn = common::atomwrite()
        .args(["--workspace", ws, "hash"])
        .args(&paths)
        .output()
        .expect("hash tn");
    assert!(out1.status.success());
    assert!(outn.status.success());
    let e1 = common::parse_ndjson(&out1.stdout);
    let en = common::parse_ndjson(&outn.stdout);
    let hashes1: Vec<_> = e1
        .iter()
        .filter(|e| e["type"] == "hash")
        .map(|e| {
            (
                e["path"].as_str().unwrap().to_string(),
                e["checksum"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let hashesn: Vec<_> = en
        .iter()
        .filter(|e| e["type"] == "hash")
        .map(|e| {
            (
                e["path"].as_str().unwrap().to_string(),
                e["checksum"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        hashes1, hashesn,
        "parallel hash must match sequential digests+order"
    );
}

#[test]
fn multi_path_backup_parallel() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..4 {
        common::create_test_file(dir.path(), &format!("b_{i}.txt"), "backup-me\n");
    }
    let ws = dir.path().to_str().unwrap();
    let mut cmd = common::atomwrite();
    cmd.args(["--workspace", ws, "backup"]);
    for i in 0..4 {
        cmd.arg(dir.path().join(format!("b_{i}.txt")));
    }
    let output = cmd.output().expect("backup");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let backups: Vec<_> = events.iter().filter(|e| e["type"] == "backup").collect();
    assert_eq!(backups.len(), 4);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["files_backed_up"].as_u64().unwrap(), 4);
}

#[test]
fn recursive_delete_parallel_dry_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..12 {
        common::create_test_file(dir.path(), &format!("d_{i}.txt"), "gone\n");
    }
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--recursive",
            "--dry-run",
        ])
        .arg(dir.path())
        .output()
        .expect("delete");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let plans: Vec<_> = events.iter().filter(|e| e["type"] == "plan").collect();
    assert!(
        plans.len() >= 12,
        "expected plan per file, got {}",
        plans.len()
    );
}

#[test]
fn multi_path_case_parallel() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();
    let mut paths = Vec::new();
    for i in 0..6 {
        let p = dir.path().join(format!("c_{i}.rs"));
        std::fs::write(&p, "let FooBar = 1;\n").expect("write");
        paths.push(p);
    }
    let mut cmd = common::atomwrite();
    cmd.args(["--workspace", ws, "--max-concurrency", "3", "case"]);
    for p in &paths {
        cmd.arg(p);
    }
    cmd.args(["--subvert", "FooBar", "foo_bar", "--to", "snake"]);
    let output = cmd.output().expect("case");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for p in &paths {
        let content = std::fs::read_to_string(p).expect("read");
        assert!(content.contains("let foo_bar = 1;"), "{content}");
    }
    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["files_modified"].as_u64().unwrap(), 6);
}

#[test]
fn multi_path_delete_dry_run_parallel() {
    // PAR-015: N single-file paths must fan out (not outer-loop sequential).
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();
    let mut paths = Vec::new();
    for i in 0..8 {
        let p = dir.path().join(format!("solo_{i}.txt"));
        std::fs::write(&p, "x\n").expect("write");
        paths.push(p);
    }
    let mut cmd = common::atomwrite();
    cmd.args([
        "--workspace",
        ws,
        "--max-concurrency",
        "4",
        "delete",
        "--dry-run",
    ]);
    for p in &paths {
        cmd.arg(p);
    }
    let output = cmd.output().expect("delete multi");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let plans: Vec<_> = events.iter().filter(|e| e["type"] == "plan").collect();
    assert_eq!(plans.len(), 8, "one plan per multi-path file");
    for p in &paths {
        assert!(p.exists(), "dry-run must not delete {}", p.display());
    }
}

#[test]
fn search_target_files_parallel_stable() {
    // PAR-016: --target files uses WalkParallel + sorted emit.
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();
    for name in ["alpha_MARK.txt", "beta_MARK.txt", "gamma_other.txt", "delta_MARK.rs"] {
        common::create_test_file(dir.path(), name, "body\n");
    }
    let output = common::atomwrite()
        .args([
            "--workspace",
            ws,
            "--threads",
            "2",
            "search",
            "--target",
            "files",
            "MARK",
        ])
        .arg(dir.path())
        .output()
        .expect("search files");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let hits: Vec<_> = events
        .iter()
        .filter(|e| e["type"] == "file_match")
        .collect();
    assert_eq!(hits.len(), 3, "three filenames contain MARK");
    let paths: Vec<&str> = hits
        .iter()
        .filter_map(|e| e["path"].as_str())
        .collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "file_match paths must be sorted for stable NDJSON");
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["files_matched"].as_u64().unwrap(), 3);
}

#[test]
fn doctor_reports_concurrency_bound() {
    // PAR-029/031: bound must be observable (equiv. available_permits).
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();
    let output = common::atomwrite()
        .args([
            "--workspace",
            ws,
            "--max-concurrency",
            "2",
            "doctor",
        ])
        .output()
        .expect("doctor");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let report = events
        .iter()
        .find(|e| e["type"] == "doctor_report")
        .expect("doctor_report");
    let checks = report["checks"]
        .as_array()
        .expect("checks array");
    let bound = checks
        .iter()
        .find(|c| c["name"].as_str() == Some("concurrency_bound"))
        .expect("concurrency_bound check");
    let detail = bound["detail"].as_str().unwrap_or("");
    assert!(
        detail.contains("effective_threads") && detail.contains("cpus="),
        "detail should describe bound: {detail}"
    );
}

#[test]
fn list_parallel_discovery_with_threads() {
    // PAR-035/036: list must accept --threads and emit complete tree.
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..30 {
        common::create_test_file(dir.path(), &format!("f{i}.txt"), "x\n");
    }
    let ws = dir.path().to_str().unwrap();
    let output = common::atomwrite()
        .args(["--workspace", ws, "--threads", "4", "list", "--long"])
        .arg(dir.path())
        .output()
        .expect("run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let files = events
        .iter()
        .filter(|e| e["type"] == "entry" && e["kind"] == "file")
        .count();
    assert_eq!(files, 30, "all files listed under parallel walk");
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["files"].as_u64().unwrap(), 30);
}

#[test]
fn sparse_list_parallel_respects_max_files_and_threads() {
    // PAR-045: sparse list uses WalkParallel; --threads accepted; budget clamp.
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..40 {
        common::create_test_file(dir.path(), &format!("s{i}.txt"), "x\n");
    }
    let ws = dir.path().to_str().unwrap();
    let output = common::atomwrite()
        .args([
            "--workspace",
            ws,
            "--threads",
            "4",
            "sparse",
            "list",
            ".",
            "--max-files",
            "10",
        ])
        .output()
        .expect("sparse list");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let entries: Vec<_> = events
        .iter()
        .filter(|e| e["type"] == "sparse_entry")
        .collect();
    assert_eq!(entries.len(), 10, "max_files clamp after parallel discovery");
    let paths: Vec<&str> = entries
        .iter()
        .filter_map(|e| e["path"].as_str())
        .collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "sparse list emits path-sorted NDJSON");
    let summary = events
        .iter()
        .find(|e| e["type"] == "sparse_summary")
        .expect("sparse_summary");
    assert_eq!(summary["emitted"].as_u64().unwrap(), 10);
    assert_eq!(summary["truncated"].as_bool(), Some(true));
}

#[test]
fn wal_stats_multi_journal_parallel_scan() {
    // PAR-046/047: multi-journal discovery + parse under WalkParallel/par_iter.
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path();
    for i in 0..8 {
        let name = format!(".atomwrite.journal.f{i}.atomwrite.journal.json");
        let body = format!(
            "{{\"phase\":\"committed\",\"op_id\":\"{:016x}\",\"committed_at_unix\":1}}\n",
            i
        );
        std::fs::write(ws.join(name), body).expect("journal");
    }
    let output = common::atomwrite()
        .args([
            "--workspace",
            ws.to_str().unwrap(),
            "--no-auto-heal",
            "wal-stats",
        ])
        .output()
        .expect("wal-stats");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let stats = events
        .iter()
        .find(|e| e["type"] == "wal_stats")
        .expect("wal_stats");
    assert_eq!(stats["total_journals"].as_u64().unwrap(), 8);
    assert_eq!(stats["by_state"]["committed"].as_u64().unwrap(), 8);
}

#[test]
fn doctor_checks_sorted_stable() {
    // PAR-052: independent checks fan out; NDJSON order is name-sorted.
    let dir = tempfile::tempdir().expect("tempdir");
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "doctor",
        ])
        .output()
        .expect("doctor");
    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let report = events
        .iter()
        .find(|e| e["type"] == "doctor_report")
        .expect("doctor_report");
    let names: Vec<&str> = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "doctor checks must be name-sorted after par_iter");
    assert!(names.contains(&"concurrency_bound"));
    assert!(names.contains(&"workspace_exists"));
}
