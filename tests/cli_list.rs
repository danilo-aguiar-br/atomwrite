// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn list_shows_files_and_dirs() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join("sub")).expect("mkdir");
    common::create_test_file(dir.path(), "a.txt", "hello\n");
    common::create_test_file(&dir.path().join("sub"), "b.rs", "fn main() {}\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "list"])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let entries: Vec<_> = events.iter().filter(|e| e["type"] == "entry").collect();
    assert!(entries.len() >= 2);

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert!(summary["files"].as_u64().unwrap() >= 2);
}

#[test]
fn list_with_depth_limit() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("a/b/c")).expect("mkdir");
    common::create_test_file(&dir.path().join("a/b/c"), "deep.txt", "deep\n");
    common::create_test_file(dir.path(), "top.txt", "top\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "list",
            "--depth",
            "1",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let deep_files: Vec<_> = events
        .iter()
        .filter(|e| e["type"] == "entry" && e["path"].as_str().is_some_and(|p| p.contains("deep")))
        .collect();
    assert!(
        deep_files.is_empty(),
        "deep files should not appear with depth 1"
    );
}

#[test]
fn list_count_by_ext() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.rs", "a\n");
    common::create_test_file(dir.path(), "b.rs", "b\n");
    common::create_test_file(dir.path(), "c.txt", "c\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "list",
            "--count-by-ext",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert!(summary["by_extension"]["rs"].as_u64().unwrap() >= 2);
    assert_eq!(summary["by_extension"]["txt"].as_u64().unwrap(), 1);
}

#[test]
fn list_long_shows_size_and_modified() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "sized.txt", "content here\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "list",
            "--long",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let file_entry = events
        .iter()
        .find(|e| e["type"] == "entry" && e["kind"] == "file")
        .expect("file entry");
    assert!(file_entry["size"].is_number());
    assert!(file_entry["modified"].is_string());
}

#[test]
fn list_exclude_filters() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.rs", "x\n");
    common::create_test_file(dir.path(), "b.py", "x\n");
    common::create_test_file(dir.path(), "c.txt", "x\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "list",
            "--exclude",
            "*.rs",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let entries: Vec<_> = events.iter().filter(|e| e["type"] == "entry").collect();
    for entry in &entries {
        let path = entry["path"].as_str().unwrap_or("");
        assert!(!path.ends_with(".rs"), "excluded .rs file appeared: {path}");
    }

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["files"].as_u64().unwrap(), 2);
}

#[test]
fn list_multi_root_covers_all_paths() {
    // PAR-021: list a b must walk both roots (not only paths[0]).
    let dir = tempfile::tempdir().expect("tempdir");
    let a = dir.path().join("root_a");
    let b = dir.path().join("root_b");
    std::fs::create_dir_all(&a).expect("mkdir a");
    std::fs::create_dir_all(&b).expect("mkdir b");
    common::create_test_file(&a, "only_a.txt", "a\n");
    common::create_test_file(&b, "only_b.txt", "b\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "list"])
        .arg(&a)
        .arg(&b)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    let paths: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "entry" && e["kind"] == "file")
        .filter_map(|e| e["path"].as_str())
        .collect();
    assert!(
        paths.iter().any(|p| p.contains("only_a")),
        "root_a missing: {paths:?}"
    );
    assert!(
        paths.iter().any(|p| p.contains("only_b")),
        "root_b missing: {paths:?}"
    );
}

#[test]
fn list_threads_2_matches_threads_1_ordered() {
    // PAR-035/036: parallel discovery must honor --threads and keep path order.
    let dir = tempfile::tempdir().expect("tempdir");
    for name in ["z.txt", "a.txt", "m.txt", "b.txt"] {
        common::create_test_file(dir.path(), name, "x\n");
    }
    std::fs::create_dir(dir.path().join("sub")).expect("mkdir");
    common::create_test_file(&dir.path().join("sub"), "nested.txt", "n\n");

    let ws = dir.path().to_str().unwrap();
    let run = |threads: &str| {
        let output = common::atomwrite()
            .args(["--workspace", ws, "--threads", threads, "list"])
            .arg(dir.path())
            .output()
            .expect("run");
        assert!(
            output.status.success(),
            "threads={threads} stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let events = common::parse_ndjson(&output.stdout);
        events
            .iter()
            .filter(|e| e["type"] == "entry")
            .filter_map(|e| e["path"].as_str().map(str::to_owned))
            .collect::<Vec<_>>()
    };

    let one = run("1");
    let two = run("2");
    assert_eq!(one, two, "list path order must be stable across --threads");
    assert!(one.len() >= 5, "expected files+dirs: {one:?}");
    // Sorted order: a.txt before z.txt
    let files: Vec<_> = one
        .iter()
        .filter(|p| p.ends_with(".txt") && !p.contains('/'))
        .collect();
    if files.len() >= 2 {
        assert!(
            files[0] <= files[files.len() - 1],
            "paths should be sorted: {files:?}"
        );
    }
}
