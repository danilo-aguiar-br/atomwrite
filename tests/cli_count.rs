// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn count_lines_in_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.txt", "line1\nline2\nline3\n");
    common::create_test_file(dir.path(), "b.txt", "one\ntwo\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "count"])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "count");
    assert_eq!(events[0]["mode"], "lines");
    assert!(events[0]["total"]["files"].as_u64().unwrap() >= 2);
    assert!(events[0]["total"]["lines"].as_u64().unwrap() >= 5);
}

#[test]
fn count_by_extension() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.rs", "fn main() {}\n");
    common::create_test_file(dir.path(), "b.rs", "fn test() {}\n");
    common::create_test_file(dir.path(), "c.txt", "hello\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "count",
            "--by-extension",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["mode"], "by_extension");
    assert!(events[0]["by_extension"]["rs"].is_object());
    assert!(events[0]["by_extension"]["txt"].is_object());
}

#[test]
fn count_include_filters() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.rs", "fn main() {\n    42\n}\n");
    common::create_test_file(dir.path(), "b.py", "def hello():\n    pass\n");
    common::create_test_file(dir.path(), "c.txt", "hello\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "count",
            "--include",
            "*.rs",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "count");
    assert_eq!(events[0]["mode"], "lines");
    assert_eq!(events[0]["total"]["files"].as_u64().unwrap(), 1);
}

#[test]
fn count_exclude_filters() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.rs", "fn main() {\n    42\n}\n");
    common::create_test_file(dir.path(), "b.py", "def hello():\n    pass\n");
    common::create_test_file(dir.path(), "c.txt", "hello\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "count",
            "--exclude",
            "*.rs",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "count");
    assert_eq!(events[0]["mode"], "lines");
    assert_eq!(events[0]["total"]["files"].as_u64().unwrap(), 2);
}

// ============================================================================
// v0.1.20 GAP-2026-001: count --by-size
// ============================================================================

/// GAP-001 --by-size returns items sorted by size descending.
#[test]
fn v0_1_20_count_by_size_returns_top_n_sorted_desc() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.txt", &"x".repeat(100));
    common::create_test_file(dir.path(), "b.txt", &"x".repeat(500));
    common::create_test_file(dir.path(), "c.txt", &"x".repeat(1000));

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "count",
            "--by-size",
            "--top",
            "3",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "count");
    assert_eq!(events[0]["mode"], "by_size");
    let items = events[0]["items"].as_array().expect("items array");
    assert_eq!(items.len(), 3);
    let sizes: Vec<u64> = items.iter().map(|i| i["bytes"].as_u64().unwrap()).collect();
    assert_eq!(sizes, vec![1000, 500, 100], "sorted descending");
}

/// GAP-001 --by-size default top is 10.
#[test]
fn v0_1_20_count_by_size_default_top_is_10() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..15 {
        common::create_test_file(dir.path(), &format!("f{i}.txt"), &"x".repeat((i + 1) * 100));
    }

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "count",
            "--by-size",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    let items = events[0]["items"].as_array().expect("items");
    assert_eq!(items.len(), 10);
}

/// GAP-001 --by-size top-1 returns only the largest file.
#[test]
fn v0_1_20_count_by_size_top_1_returns_largest() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "small.txt", "abc");
    common::create_test_file(dir.path(), "huge.txt", &"z".repeat(1_000_000));

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "count",
            "--by-size",
            "--top",
            "1",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    let items = events[0]["items"].as_array().expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["bytes"].as_u64().unwrap(), 1_000_000);
    assert!(items[0]["path"].as_str().unwrap().contains("huge"));
}

/// GAP-2026-007: --by-extension should categorize backup files under "backup" key.
#[test]
fn v0_1_20_count_by_extension_excludes_backup_timestamps() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "data.txt", "content");
    common::create_test_file(dir.path(), "data.txt.bak.20260615_035515", "old");
    common::create_test_file(dir.path(), "script.rs", "fn main() {}");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "count",
            "--by-extension",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    let by_ext = events[0]["by_extension"]
        .as_object()
        .expect("by_extension object");
    assert!(
        by_ext.contains_key("backup"),
        "backup should be its own category"
    );
    assert!(
        !by_ext.contains_key("20260615_035515"),
        "timestamp must not be a category"
    );
    assert!(by_ext.contains_key("txt"));
    assert!(by_ext.contains_key("rs"));
}
