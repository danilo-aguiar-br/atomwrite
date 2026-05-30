// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn write_creates_file_with_ndjson_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("test.txt");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .write_stdin("hello world\n")
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["type"], "write");
    assert_eq!(events[0]["status"], "success");
    assert_eq!(events[0]["bytes_written"], 12);
    assert!(events[0]["checksum"].is_string());

    let content = std::fs::read_to_string(&target).expect("read file");
    assert_eq!(content, "hello world\n");
}

#[test]
fn write_atomic_preserves_permissions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("perms.txt");
    std::fs::write(&target, "original").expect("write");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).expect("chmod");
    }

    common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .write_stdin("new content")
        .assert()
        .success();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&target)
            .expect("stat")
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(mode, 0o644, "permissions should be preserved");
    }
}

#[test]
fn write_dry_run_does_not_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("dry.txt");
    std::fs::write(&target, "original").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--dry-run",
        ])
        .arg(&target)
        .write_stdin("new content")
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "plan");

    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "original", "file should not be modified");
}

#[test]
fn write_append_adds_to_end() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("append.txt");
    std::fs::write(&target, "line1\n").expect("write");

    common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--append",
        ])
        .arg(&target)
        .write_stdin("line2\n")
        .assert()
        .success();

    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "line1\nline2\n");
}

#[test]
fn write_workspace_jail_rejects_outside_path() {
    let dir = tempfile::tempdir().expect("tempdir");

    common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "/etc/passwd",
        ])
        .write_stdin("hacked")
        .assert()
        .code(126);
}

#[test]
fn write_expect_checksum_rejects_drift() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("drift.txt");
    std::fs::write(&target, "original").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--expect-checksum",
            "wrong_checksum",
        ])
        .arg(&target)
        .write_stdin("new")
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(82));
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "STATE_DRIFT");
}
