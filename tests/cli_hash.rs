// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn hash_single_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "data.txt", "hello\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "hash");
    assert_eq!(events[0]["algorithm"], "blake3");
    assert!(events[0]["value"].is_string());
    assert_eq!(events[0]["bytes"], 6);
}

#[test]
fn hash_verify_correct() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "v.txt", "test\n");

    let hash = blake3::hash(b"test\n").to_hex().to_string();

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--verify",
            &hash,
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["verified"], true);
}

#[test]
fn hash_verify_mismatch_exits_81() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "m.txt", "data\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--verify",
            "wrong_hash_value",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(81));
}

#[test]
fn hash_stdin_mode() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--stdin",
            "dummy",
        ])
        .write_stdin("hello stdin\n")
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["source"], "stdin");
    assert!(events[0]["value"].is_string());
}
