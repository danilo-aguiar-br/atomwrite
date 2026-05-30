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
