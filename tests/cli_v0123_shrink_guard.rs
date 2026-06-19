// SPDX-License-Identifier: MIT OR Apache-2.0

//! GAP-2026-017: shrink guard blocks writes that reduce file size by >50%
//! when --expect-checksum is active. Pass --allow-shrink to override.

mod common;

use std::fs;

fn create_large_file(dir: &std::path::Path, name: &str, size: usize) -> std::path::PathBuf {
    let content: String = "x".repeat(size);
    common::create_test_file(dir, name, &content)
}

fn get_checksum(dir: &std::path::Path, target: &std::path::Path) -> String {
    let output = common::atomwrite()
        .args(["--workspace", dir.to_str().unwrap(), "read", "--stat"])
        .arg(target)
        .output()
        .expect("read for checksum");
    assert!(
        output.status.success(),
        "read failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lines = common::parse_ndjson(&output.stdout);
    for line in &lines {
        if let Some(cs) = line.get("checksum").and_then(|c| c.as_str()) {
            return cs.to_string();
        }
    }
    panic!(
        "checksum not found in output: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn write_expect_checksum_blocks_shrink() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = create_large_file(dir.path(), "large.txt", 1000);

    let cs = get_checksum(dir.path(), &target);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--expect-checksum",
            &cs,
        ])
        .arg(&target)
        .write_stdin("y".repeat(400))
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(65),
        "should exit 65 (INVALID_INPUT); stderr: {}; stdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");
    assert!(
        combined.contains("shrink") || combined.contains("smaller"),
        "error should mention shrink: {combined}"
    );
    assert!(
        combined.contains("--allow-shrink"),
        "error should suggest --allow-shrink: {combined}"
    );

    assert_eq!(
        fs::read_to_string(&target).unwrap().len(),
        1000,
        "original file should be unchanged"
    );
}

#[test]
fn write_expect_checksum_allow_shrink_permits() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = create_large_file(dir.path(), "large.txt", 1000);

    let cs = get_checksum(dir.path(), &target);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--expect-checksum",
            &cs,
            "--allow-shrink",
        ])
        .arg(&target)
        .write_stdin("y".repeat(400))
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "should succeed with --allow-shrink; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&target).unwrap().len(), 400);
}

#[test]
fn write_without_expect_checksum_permits_shrink() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = create_large_file(dir.path(), "large.txt", 1000);

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .write_stdin("y".repeat(400))
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "should succeed without --expect-checksum; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&target).unwrap().len(), 400);
}

#[test]
fn write_expect_checksum_permits_growth() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = create_large_file(dir.path(), "small.txt", 100);

    let cs = get_checksum(dir.path(), &target);

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--expect-checksum",
            &cs,
        ])
        .arg(&target)
        .write_stdin("y".repeat(500))
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "growth should not be blocked; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&target).unwrap().len(), 500);
}
