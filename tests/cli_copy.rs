// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn copy_file_with_checksum_verification() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "src.txt", "copy me\n");
    let dst = dir.path().join("dst.txt");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "copy"])
        .arg(&src)
        .arg(&dst)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(src.exists(), "source should still exist");
    assert!(dst.exists(), "target should exist");
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "copy me\n");

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "copied");
    assert_eq!(events[0]["verified"], true);
    assert!(events[0]["checksum"].is_string());
}

#[test]
fn copy_dry_run_does_not_create_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "s.txt", "data\n");
    let dst = dir.path().join("d.txt");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "copy",
            "--dry-run",
        ])
        .arg(&src)
        .arg(&dst)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(!dst.exists());
}

#[test]
fn copy_target_exists_without_force_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "a.txt", "a\n");
    let dst = common::create_test_file(dir.path(), "b.txt", "b\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "copy"])
        .arg(&src)
        .arg(&dst)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
}

#[test]
fn copy_not_found_exits_4() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "copy"])
        .arg(dir.path().join("ghost.txt"))
        .arg(dir.path().join("dest.txt"))
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(4));
}
