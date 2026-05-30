// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn read_returns_content_and_checksum() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "hello.txt", "hello world\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["type"], "read");
    assert_eq!(events[0]["content"], "hello world\n");
    assert_eq!(events[0]["bytes"], 12);
    assert_eq!(events[0]["lines"], 1);
    assert!(events[0]["checksum"].is_string());
    assert_eq!(events[0]["binary"], false);
}

#[test]
fn read_stat_omits_content() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "stat.txt", "data\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--stat",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert!(events[0]["content"].is_null());
    assert_eq!(events[0]["bytes"], 5);
}

#[test]
fn read_lines_range() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "lines.txt", "a\nb\nc\nd\ne\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--lines",
            "2:4",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["content"], "b\nc\nd\n");
}

#[test]
fn read_format_raw_emits_bytes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "raw.txt", "raw content\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--format",
            "raw",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "raw content\n");
}

#[test]
fn read_not_found_exits_4() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(dir.path().join("nonexistent.txt"))
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(4));
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "FILE_NOT_FOUND");
}

#[test]
fn read_binary_file_detected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("binary.bin");
    std::fs::write(&path, b"\x00\x01\x02\x03binary").expect("write");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["binary"], true);
    assert!(events[0]["content"].is_null());
}
