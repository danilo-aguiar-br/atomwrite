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
    assert!(events[0]["checksum"].is_string());
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
    assert!(events[0]["checksum"].is_string());
}

#[test]
fn hash_no_include_exclude_flags() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "dummy.txt", "x\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--include",
            "*.rs",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(2),
        "--include should be rejected by Clap with exit code 2"
    );
}

/// Multi-file without `--verify` uses the parallel path; NDJSON order stays
/// sorted by path (stable agent contract).
#[test]
fn hash_multi_file_sorted_parallel() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();
    let p_b = common::create_test_file(dir.path(), "b.txt", "b\n");
    let p_a = common::create_test_file(dir.path(), "a.txt", "a\n");
    let p_c = common::create_test_file(dir.path(), "c.txt", "c\n");

    let output = common::atomwrite()
        .args(["--workspace", ws, "hash"])
        .arg(&p_b)
        .arg(&p_a)
        .arg(&p_c)
        .output()
        .expect("run");

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 3);
    let paths: Vec<&str> = events
        .iter()
        .map(|e| e["path"].as_str().expect("path"))
        .collect();
    // Sorted lexicographically by full path → a, b, c filenames.
    assert!(paths[0].ends_with("a.txt"), "got {paths:?}");
    assert!(paths[1].ends_with("b.txt"), "got {paths:?}");
    assert!(paths[2].ends_with("c.txt"), "got {paths:?}");
    for e in &events {
        assert_eq!(e["type"], "hash");
        assert_eq!(e["algorithm"], "blake3");
        assert!(e["checksum"].is_string());
        assert_eq!(e["bytes"], 2);
    }
}
