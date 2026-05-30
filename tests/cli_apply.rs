// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn apply_full_file_replacement() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "test.txt", "old content");
    let ws = dir.path().to_str().unwrap();

    let output = common::atomwrite()
        .args(["--workspace", ws, "apply"])
        .arg(&file)
        .write_stdin("new content\n")
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    assert!(events.iter().any(|e| e["type"] == "applied"));

    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(content, "new content\n");
}

#[test]
fn apply_search_replace_blocks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "test.txt", "hello world\nfoo bar\n");
    let ws = dir.path().to_str().unwrap();

    let patch = "<<<<<<< SEARCH\nhello world\n=======\nhello universe\n>>>>>>> REPLACE\n";

    let output = common::atomwrite()
        .args(["--workspace", ws, "apply"])
        .arg(&file)
        .write_stdin(patch)
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("hello universe"));
}

#[test]
fn apply_dry_run_does_not_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "test.txt", "original");
    let ws = dir.path().to_str().unwrap();

    let output = common::atomwrite()
        .args(["--workspace", ws, "apply", "--dry-run"])
        .arg(&file)
        .write_stdin("replacement")
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(content, "original");
}

#[test]
fn apply_unified_not_found_exits_4() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();

    let output = common::atomwrite()
        .args(["--workspace", ws, "apply", "--format", "unified"])
        .arg(dir.path().join("nonexistent.txt"))
        .write_stdin("--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\n+new\n")
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(4));
}
