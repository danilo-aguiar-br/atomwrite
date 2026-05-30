// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn move_renames_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "old.txt", "content\n");
    let dst = dir.path().join("new.txt");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "move"])
        .arg(&src)
        .arg(&dst)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(!src.exists(), "source should be gone");
    assert!(dst.exists(), "target should exist");
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "content\n");

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "moved");
    assert_eq!(events[0]["atomic"], true);
    assert!(events[0]["checksum"].is_string());
}

#[test]
fn move_dry_run_preserves_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "stay.txt", "data\n");
    let dst = dir.path().join("moved.txt");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "move",
            "--dry-run",
        ])
        .arg(&src)
        .arg(&dst)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(src.exists(), "source should still exist");
    assert!(!dst.exists(), "target should not exist");
}

#[test]
fn move_target_exists_without_force_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "src.txt", "src\n");
    let dst = common::create_test_file(dir.path(), "dst.txt", "dst\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "move"])
        .arg(&src)
        .arg(&dst)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
}

#[test]
fn move_target_exists_with_force_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = common::create_test_file(dir.path(), "s.txt", "new\n");
    let dst = common::create_test_file(dir.path(), "d.txt", "old\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "move",
            "--force",
        ])
        .arg(&src)
        .arg(&dst)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "new\n");
}

#[test]
fn move_not_found_exits_4() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "move"])
        .arg(dir.path().join("ghost.txt"))
        .arg(dir.path().join("dest.txt"))
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(4));
}
