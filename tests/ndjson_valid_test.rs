// SPDX-License-Identifier: MIT OR Apache-2.0

//! Validate that all NDJSON output is valid JSON per line using serde_json.

mod common;

fn assert_all_lines_valid_json(stdout: &[u8], cmd_name: &str) {
    let text = String::from_utf8_lossy(stdout);
    let mut valid = 0usize;
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
            panic!(
                "{cmd_name}: line {} is not valid JSON: {e}\nline: {line}",
                i + 1
            )
        });
        assert!(
            parsed.is_object(),
            "{cmd_name}: line {} should be a JSON object, got: {parsed}",
            i + 1
        );
        valid += 1;
    }
    assert!(
        valid > 0,
        "{cmd_name}: expected at least 1 NDJSON line in output"
    );
}

#[test]
fn ndjson_search_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("test.txt"), "needle in haystack\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "needle",
        ])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "search failed: {:?}",
        output.status
    );
    assert_all_lines_valid_json(&output.stdout, "search");
}

#[test]
fn ndjson_hash_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("file.txt");
    std::fs::write(&f, "hello\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
        .arg(&f)
        .output()
        .unwrap();

    assert!(output.status.success(), "hash failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "hash");
}

#[test]
fn ndjson_list_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    std::fs::write(dir.path().join("b.txt"), "b\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "list"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "list failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "list");
}

#[test]
fn ndjson_diff_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    std::fs::write(&a, "line1\nline2\n").unwrap();
    std::fs::write(&b, "line1\nline3\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "diff"])
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();

    assert!(output.status.success(), "diff failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "diff");
}

#[test]
fn ndjson_write_output_valid() {
    use std::io::Write;
    use std::process::Stdio;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("out.txt");

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(b"hello world\n");
        let _ = stdin.flush();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "write failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "write");
}

#[test]
fn ndjson_read_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("read_me.txt");
    std::fs::write(&f, "content\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(&f)
        .output()
        .unwrap();

    assert!(output.status.success(), "read failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "read");
}

#[test]
fn ndjson_edit_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("edit_me.txt");
    std::fs::write(&f, "old_value\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "--old",
            "old_value",
            "--new",
            "new_value",
        ])
        .arg(&f)
        .output()
        .unwrap();

    assert!(output.status.success(), "edit failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "edit");
}

#[test]
fn ndjson_replace_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rep.txt"), "find_me here\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--dry-run",
            "find_me",
            "found_it",
        ])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "replace failed: {:?}",
        output.status
    );
    assert_all_lines_valid_json(&output.stdout, "replace");
}

#[test]
fn ndjson_count_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("c.txt"), "line\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "count"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "count failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "count");
}

#[test]
fn ndjson_backup_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("bak.txt");
    std::fs::write(&f, "backup me\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "backup"])
        .arg(&f)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "backup failed: {:?}",
        output.status
    );
    assert_all_lines_valid_json(&output.stdout, "backup");
}

#[test]
fn ndjson_copy_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    std::fs::write(&src, "copy me\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "copy"])
        .arg(&src)
        .arg(&dst)
        .output()
        .unwrap();

    assert!(output.status.success(), "copy failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "copy");
}

#[test]
fn ndjson_move_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("mv_src.txt");
    let dst = dir.path().join("mv_dst.txt");
    std::fs::write(&src, "move me\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "move"])
        .arg(&src)
        .arg(&dst)
        .output()
        .unwrap();

    assert!(output.status.success(), "move failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "move");
}

#[test]
fn ndjson_delete_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("del.txt");
    std::fs::write(&f, "delete me\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "delete"])
        .arg(&f)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "delete failed: {:?}",
        output.status
    );
    assert_all_lines_valid_json(&output.stdout, "delete");
}

#[test]
fn ndjson_batch_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("batch.txt");
    let manifest = common::manifest(&[serde_json::json!({
        "op": "write",
        "target": target.to_string_lossy(),
        "content": "batch ndjson",
    })]);

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "batch"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        let _ = stdin.write_all(manifest.as_bytes());
        let _ = stdin.flush();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "batch failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "batch");
}

#[test]
fn ndjson_calc_output_valid() {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["calc", "2+2"])
        .output()
        .unwrap();

    assert!(output.status.success(), "calc failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "calc");
}

#[test]
fn ndjson_regex_output_valid() {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args(["regex", "abc", "def", "ghi"])
        .output()
        .unwrap();

    assert!(output.status.success(), "regex failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "regex");
}

#[test]
fn ndjson_rollback_not_found_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("no_backup.txt");
    std::fs::write(&f, "no backup\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = std::process::Command::new(&bin)
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "rollback",
            "--latest",
        ])
        .arg(&f)
        .output()
        .unwrap();

    assert_all_lines_valid_json(&output.stdout, "rollback");
}

#[test]
fn ndjson_apply_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("apply.txt");
    std::fs::write(&f, "line1\nline2\n").unwrap();

    let patch = "<<<< SEARCH\nline2\n==== \nline2_replaced\n>>>> REPLACE";

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = std::process::Command::new(&bin)
        .args(["--workspace", dir.path().to_str().unwrap(), "apply"])
        .arg(&f)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        let _ = stdin.write_all(patch.as_bytes());
        let _ = stdin.flush();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "apply failed: {:?}", output.status);
    assert_all_lines_valid_json(&output.stdout, "apply");
}

#[test]
fn ndjson_extract_output_valid() {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let input = r#"{"path":"a.txt","line":1}
{"path":"b.txt","line":2}"#;

    let mut child = std::process::Command::new(&bin)
        .args(["extract", "path", "line"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        let _ = stdin.write_all(input.as_bytes());
        let _ = stdin.flush();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "extract failed: {:?}",
        output.status
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(!text.is_empty(), "extract should produce output");
}

#[test]
fn ndjson_search_interop_jaq() {
    if std::process::Command::new("jaq")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("SKIP: jaq not in PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("interop.txt"), "interop_test_line\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");

    let jaq2 = std::process::Command::new("sh")
        .args([
            "-c",
            &format!(
                "{} --workspace {} search interop {} | jaq -c .type",
                bin.display(),
                dir.path().display(),
                dir.path().display()
            ),
        ])
        .output()
        .unwrap();

    assert!(jaq2.status.success(), "jaq pipe should succeed");
    let text = String::from_utf8_lossy(&jaq2.stdout);
    assert!(
        text.contains("\"match\"") || text.contains("\"summary\""),
        "jaq should extract .type field from NDJSON: {text}"
    );
}

#[test]
fn ndjson_read_interop_jaq() {
    if std::process::Command::new("jaq")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("SKIP: jaq not in PATH");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("jaq_read.txt");
    std::fs::write(&f, "jaq test\n").unwrap();

    let bin = assert_cmd::cargo::cargo_bin("atomwrite");

    let pipe_out = std::process::Command::new("sh")
        .args([
            "-c",
            &format!(
                "{} --workspace {} read {} | jaq -r .checksum",
                bin.display(),
                dir.path().display(),
                f.display()
            ),
        ])
        .output()
        .unwrap();

    assert!(pipe_out.status.success(), "jaq should parse read output");
    let checksum = String::from_utf8_lossy(&pipe_out.stdout);
    assert!(
        checksum.trim().len() == 64,
        "checksum should be 64 hex chars, got: {checksum}"
    );
}
