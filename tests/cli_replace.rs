// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn replace_modifies_file_atomically() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "rep.txt", "old_api(x)\nold_api(y)\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "old_api",
            "new_api",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());

    let events = common::parse_ndjson(&output.stdout);
    let replaced: Vec<_> = events.iter().filter(|e| e["type"] == "replaced").collect();
    assert!(!replaced.is_empty());
    assert_eq!(replaced[0]["replacements"], 2);
    assert!(replaced[0]["checksum_before"].is_string());
    assert!(replaced[0]["checksum_after"].is_string());

    let content = std::fs::read_to_string(dir.path().join("rep.txt")).expect("read");
    assert!(content.contains("new_api"));
    assert!(!content.contains("old_api"));
}

#[test]
fn replace_dry_run_does_not_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "dry.txt", "original_text\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--dry-run",
            "original_text",
            "new_text",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());

    let content = std::fs::read_to_string(dir.path().join("dry.txt")).expect("read");
    assert_eq!(content, "original_text\n", "file should not be modified");

    let events = common::parse_ndjson(&output.stdout);
    let plans: Vec<_> = events.iter().filter(|e| e["type"] == "plan").collect();
    assert!(!plans.is_empty());
}

#[test]
fn replace_literal_escapes_regex_chars() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "regex.txt", "fn foo() -> Result<(), Error>\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--literal",
            "Result<(), Error>",
            "anyhow::Result<()>",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());

    let content = std::fs::read_to_string(dir.path().join("regex.txt")).expect("read");
    assert!(content.contains("anyhow::Result<()>"));
}

#[test]
fn replace_preview_shows_diff_without_writing() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "prev.txt", "hello world\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--preview",
            "hello",
            "goodbye",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());

    let content = std::fs::read_to_string(dir.path().join("prev.txt")).expect("read");
    assert_eq!(content, "hello world\n", "file should not be modified");

    let events = common::parse_ndjson(&output.stdout);
    let previews: Vec<_> = events.iter().filter(|e| e["type"] == "preview").collect();
    assert!(!previews.is_empty());
    assert!(previews[0]["diff"].is_string());
}

#[test]
fn replace_max_replacements_limits_count() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "max.txt", "aaa\naaa\naaa\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--max-replacements",
            "1",
            "aaa",
            "bbb",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());

    let content = std::fs::read_to_string(dir.path().join("max.txt")).expect("read");
    let bbb_count = content.matches("bbb").count();
    let aaa_count = content.matches("aaa").count();
    assert_eq!(bbb_count, 1);
    assert_eq!(aaa_count, 2);
}

#[test]
fn replace_summary_has_correct_counts() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.txt", "target\n");
    common::create_test_file(dir.path(), "b.txt", "no match\n");
    common::create_test_file(dir.path(), "c.txt", "target target\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "target",
            "replaced",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());

    let events = common::parse_ndjson(&output.stdout);
    let summary = events
        .iter()
        .find(|e| e["type"] == "summary")
        .expect("summary");
    assert!(summary["files_visited"].as_u64().unwrap() >= 3);
    assert!(summary["total_replacements"].as_u64().unwrap() >= 3);
}

/// G118 (path resolution): `replace` with a root path that escapes the
/// workspace jail (absolute `/etc/passwd` or relative `../escape`)
/// must fail with exit 126 BEFORE the WalkBuilder is constructed.
/// Pre-fix: the walker would resolve the path against the CWD and
/// emit one `JailViolation` event per file walked; the user-facing
/// diagnostic was buried under hundreds of error events.
#[test]
fn replace_root_path_outside_workspace_exits_126() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "inside.txt", "x\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "x",
            "y",
        ])
        .arg("/etc/passwd")
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(126),
        "absolute /etc/passwd must abort with exit 126 WORKSPACE_JAIL"
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["error"], true);
    assert_eq!(events[0]["code"], "WORKSPACE_JAIL");
    assert!(
        events[0]["path"]
            .as_str()
            .unwrap_or("")
            .contains("/etc/passwd"),
        "error envelope must surface the offending path, got: {:?}",
        events[0]["path"]
    );
}

/// G118 (path resolution): `replace` with a relative `..` root that
/// escapes the workspace must abort before any walk, even when CWD
/// differs from the workspace. This is the CWE-367-style race the
/// G118 fix is designed to close.
#[test]
fn replace_relative_dotdot_root_outside_workspace_exits_126() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    common::create_test_file(outside.path(), "secret.txt", "should not be touched\n");

    // CWD is `outside`; workspace is a different dir. A `../<workspace>`
    // path from `outside` would land INSIDE the workspace, but the
    // relevant case is `../outside` evaluated against the workspace
    // — which is a relative-traversal that escapes.
    let escape_path = format!(
        "../{}",
        outside.path().file_name().unwrap().to_str().unwrap()
    );

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace.path().to_str().unwrap(),
            "replace",
            "touched",
            "left_alone",
        ])
        .arg(&escape_path)
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(126),
        "`{}` must be rejected as a jail violation",
        escape_path
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "WORKSPACE_JAIL");
}
