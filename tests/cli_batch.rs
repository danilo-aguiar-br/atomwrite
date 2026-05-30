// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn batch_write_creates_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("batch_out.txt");

    let manifest = format!(
        r#"{{"op":"write","target":"{}","content":"hello batch"}}"#,
        target.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);

    let op = events
        .iter()
        .find(|e| e["type"] == "batch_op")
        .expect("batch_op event");
    assert_eq!(op["op"], "write");
    assert_eq!(op["status"], "ok");

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["operations"], 1);
    assert_eq!(summary["succeeded"], 1);
    assert_eq!(summary["failed"], 0);

    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "hello batch");
}

#[test]
fn batch_replace_modifies_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("replace_me.txt");
    std::fs::write(&target, "old_value here\n").expect("write");

    let manifest = format!(
        r#"{{"op":"replace","target":"{}","pattern":"old_value","replacement":"new_value"}}"#,
        target.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(output.status.success());

    let content = std::fs::read_to_string(&target).expect("read");
    assert!(content.contains("new_value"));
    assert!(!content.contains("old_value"));
}

#[test]
fn batch_delete_removes_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("to_delete.txt");
    std::fs::write(&target, "delete me\n").expect("write");

    let manifest = format!(r#"{{"op":"delete","target":"{}"}}"#, target.display());

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(!target.exists());
}

#[test]
fn batch_dry_run_does_not_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("keep.txt");
    let original = "keep me\n";
    std::fs::write(&target, original).expect("write");

    let manifest = format!(
        r#"{{"op":"replace","target":"{}","pattern":"keep","replacement":"gone"}}"#,
        target.display()
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "batch",
            "--dry-run",
        ])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, original);
}

#[test]
fn batch_multiple_operations() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_a = dir.path().join("a.txt");
    let file_b = dir.path().join("b.txt");
    std::fs::write(&file_b, "original_b\n").expect("write");

    let manifest = format!(
        r#"{{"op":"write","target":"{}","content":"content_a"}}
{{"op":"replace","target":"{}","pattern":"original_b","replacement":"modified_b"}}"#,
        file_a.display(),
        file_b.display()
    );

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);

    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert_eq!(summary["operations"], 2);
    assert_eq!(summary["succeeded"], 2);

    assert_eq!(std::fs::read_to_string(&file_a).expect("a"), "content_a");
    assert!(
        std::fs::read_to_string(&file_b)
            .expect("b")
            .contains("modified_b")
    );
}

#[test]
fn batch_invalid_op_fails() {
    let dir = tempfile::tempdir().expect("tempdir");

    let manifest = r#"{"op":"nonexistent","target":"foo.txt"}"#;

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin(manifest)
        .output()
        .expect("run");

    assert!(!output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let op = events
        .iter()
        .find(|e| e["type"] == "batch_op")
        .expect("batch_op");
    assert_eq!(op["status"], "failed");
}

#[test]
fn batch_empty_manifest_fails() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .write_stdin("")
        .output()
        .expect("run");

    assert!(!output.status.success());
}
