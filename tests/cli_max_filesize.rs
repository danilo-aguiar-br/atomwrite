// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn read_rejects_file_above_max_size() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("big.txt");
    std::fs::write(&path, "x".repeat(200)).expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--max-filesize",
            "100",
            "read",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(65),
        "should exit 65 for file too large"
    );

    let events = common::parse_ndjson(&output.stdout);
    let err = events
        .iter()
        .find(|e| e["error"] == true)
        .expect("error event");
    assert_eq!(err["code"], "FILE_TOO_LARGE");
    assert_eq!(err["retryable"], false);
}

#[test]
fn read_accepts_file_below_max_size() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("small.txt");
    std::fs::write(&path, "hello world").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--max-filesize",
            "1000",
            "read",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "should succeed for file below max size"
    );
}

#[test]
fn hash_rejects_file_above_max_size() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("big.bin");
    std::fs::write(&path, vec![0u8; 200]).expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--max-filesize",
            "100",
            "hash",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(65),
        "hash should exit 65 for file too large"
    );

    let events = common::parse_ndjson(&output.stdout);
    let err = events
        .iter()
        .find(|e| e["error"] == true)
        .expect("error event");
    assert_eq!(err["code"], "FILE_TOO_LARGE");
}

#[test]
fn edit_rejects_file_above_max_size() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("big_edit.txt");
    std::fs::write(&path, "x".repeat(200)).expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "--max-filesize",
            "100",
            "edit",
            "--old",
            "xxx",
            "--new",
            "yyy",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(65),
        "edit should exit 65 for file too large"
    );
}

#[test]
fn default_max_filesize_allows_normal_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("normal.txt");
    std::fs::write(&path, "hello world\n").expect("write");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "default 1GiB max should allow normal files"
    );
}
