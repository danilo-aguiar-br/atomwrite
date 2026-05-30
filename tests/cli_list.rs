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
