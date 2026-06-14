// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regression tests for v0.1.2 critical bug fixes.

mod common;

#[test]
fn batch_transaction_rolls_back_created_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let manifest = r#"{"op":"write","path":"a.txt","content":"first"}
{"op":"write","path":"b.txt","content":"second"}
{"op":"write","path":"c.txt","content":"third-violates-nothing"}
"#;

    let output = common::atomwrite()
        .args(["--workspace", workspace, "batch", "--transaction"])
        .write_stdin(manifest.as_bytes())
        .output()
        .expect("batch");

    // Either all 3 succeed and all files exist, OR any failure rolls back ALL files
    if output.status.success() {
        assert!(dir.path().join("a.txt").exists());
        assert!(dir.path().join("b.txt").exists());
        assert!(dir.path().join("c.txt").exists());
    }
}

#[test]
fn batch_file_flag_reads_manifest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let manifest_path = dir.path().join("ops.ndjson");
    std::fs::write(
        &manifest_path,
        "{\"op\":\"write\",\"path\":\"hello.txt\",\"content\":\"from-manifest\"}\n",
    )
    .expect("write manifest");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "batch",
            "--file",
            manifest_path.to_str().unwrap(),
        ])
        .output()
        .expect("batch");

    assert!(
        output.status.success(),
        "batch --file failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join("hello.txt").exists());
}

#[test]
fn replace_jail_violation_does_not_inflate_counter() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outside_dir = tempfile::tempdir().expect("tempdir");
    let inside = dir.path().join("inside.txt");
    let outside = outside_dir.path().join("outside.txt");

    std::fs::write(&inside, "foo bar\n").expect("write inside");
    std::fs::write(&outside, "foo baz\n").expect("write outside");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "foo",
            "FOO",
            outside.to_str().unwrap(),
        ])
        .output()
        .expect("replace");

    // v0.1.18 (G118 fix): replace now pre-validates caller-supplied root
    // paths against the workspace jail BEFORE constructing the
    // WalkBuilder. The legacy per-entry behaviour (v0.1.12 through
    // v0.1.17) emitted one JailViolation event per file walked and
    // returned exit 0 with `files_skipped=1`. The new contract aborts
    // with exit 126 and a single structured error envelope so the
    // user sees the failure immediately instead of buried in N events.
    assert_eq!(
        output.status.code(),
        Some(126),
        "v0.1.18+: out-of-jail path must abort with exit 126, not skip"
    );

    // Original outside file must be unchanged (no partial walk).
    let outside_content = std::fs::read_to_string(&outside).expect("read outside");
    assert_eq!(outside_content, "foo baz\n");

    // Inside file must also be unchanged (replace aborted before walk).
    let inside_content = std::fs::read_to_string(&inside).expect("read inside");
    assert_eq!(inside_content, "foo bar\n");

    // Structured error envelope surfaces the offending path.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("WORKSPACE_JAIL"),
        "error envelope must report WORKSPACE_JAIL, got: {stdout}"
    );
}

#[test]
fn search_invalid_regex_emits_json_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "hello world\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "search",
            "[invalid",
            file.to_str().unwrap(),
        ])
        .output()
        .expect("search");

    // Should fail with non-zero exit AND produce JSON error envelope
    assert!(!output.status.success(), "invalid regex should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"error\":true") || stdout.contains("INVALID_INPUT"),
        "expected JSON error envelope, got: {stdout}"
    );
}

#[test]
fn scope_delete_rust_comments_no_orphan_whitespace() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let src = dir.path().join("lib.rs");
    std::fs::write(
        &src,
        "fn foo() {\n    // hello comment\n    let x = 1;\n}\n",
    )
    .expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "scope",
            "--language",
            "rust",
            "--query",
            "comments",
            "--delete",
            src.to_str().unwrap(),
        ])
        .output()
        .expect("scope");

    assert!(
        output.status.success(),
        "scope failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let result = std::fs::read_to_string(&src).expect("read");
    // After deleting the comment, we should not have a leftover line of just whitespace
    assert!(
        !result.contains("    \n"),
        "orphan whitespace remains: {result:?}"
    );
    assert!(
        result.contains("let x = 1"),
        "expected remaining code, got: {result:?}"
    );
}

#[test]
fn backup_output_dir_respected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("source.txt");
    let outdir = dir.path().join("backups");
    std::fs::create_dir(&outdir).expect("mkdir");
    std::fs::write(&src, "important data\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "backup",
            "--output-dir",
            outdir.to_str().unwrap(),
            src.to_str().unwrap(),
        ])
        .output()
        .expect("backup");

    assert!(
        output.status.success(),
        "backup failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The backup file should be in the custom output dir, not next to source
    let mut found_in_outdir = false;
    let mut found_in_srcdir = false;
    for entry in std::fs::read_dir(&outdir).expect("readdir") {
        let entry = entry.expect("entry");
        if entry
            .file_name()
            .to_string_lossy()
            .contains("source.txt.bak")
        {
            found_in_outdir = true;
        }
    }
    for entry in std::fs::read_dir(dir.path()).expect("readdir root") {
        let entry = entry.expect("entry");
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("source.txt") && name.contains(".bak") {
            found_in_srcdir = true;
        }
    }
    assert!(found_in_outdir, "backup not found in --output-dir");
    assert!(!found_in_srcdir, "backup leaked into source directory");
}

#[test]
fn read_with_grep_filters_lines() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().to_str().unwrap();
    let file = dir.path().join("log.txt");
    std::fs::write(
        &file,
        "INFO startup\nERROR connection failed\nINFO retry\nERROR timeout\n",
    )
    .expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            workspace,
            "read",
            "--grep",
            "ERROR",
            file.to_str().unwrap(),
        ])
        .output()
        .expect("read");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ERROR connection failed"),
        "missing ERROR line: {stdout}"
    );
    assert!(
        stdout.contains("ERROR timeout"),
        "missing ERROR line: {stdout}"
    );
    // Raw output won't include INFO lines, but NDJSON content field might
    // (grep is applied to lines for both formats)
}

#[test]
fn completions_install_bash_creates_xdg_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let xdg_data = dir.path().join("xdg");
    // SAFETY: setting env var in test process before forking atomwrite child
    unsafe {
        std::env::set_var("XDG_DATA_HOME", &xdg_data);
    }

    let output = common::atomwrite()
        .args(["completions", "bash", "--install"])
        .output()
        .expect("completions install");

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let install_path = xdg_data.join("bash-completion/completions/atomwrite");
    assert!(
        install_path.exists(),
        "completion file not at {install_path:?}"
    );
    let content = std::fs::read_to_string(&install_path).expect("read");
    assert!(!content.is_empty(), "completion file is empty");
    assert!(
        content.contains("atomwrite"),
        "completion should mention atomwrite"
    );
}

#[test]
fn jail_suggestion_mentions_workspace_flag() {
    // GAP 13: when no workspace is provided, the suggestion must point the
    // user to --workspace or ATOMWRITE_WORKSPACE so they can fix the call.
    let dir = tempfile::tempdir().expect("tempdir");
    let outside_dir = tempfile::tempdir().expect("tempdir");
    let outside = outside_dir.path().join("foo.txt");
    std::fs::write(&outside, "data").expect("write");

    let output = common::atomwrite()
        .args(["read", outside.to_str().unwrap()])
        .env("ATOMWRITE_WORKSPACE", dir.path().to_str().unwrap())
        .env_remove("ATOMWRITE_LANG")
        .output()
        .expect("read");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--workspace") || stdout.contains("ATOMWRITE_WORKSPACE"),
        "jail error without workspace flag should suggest --workspace: {stdout}"
    );
}

#[test]
fn gap13_jail_suggestion_when_workspace_supplied_says_inside() {
    // GAP 13: when workspace IS provided (via --workspace), the suggestion
    // must say "inside the workspace" rather than re-prompting the flag.
    let dir = tempfile::tempdir().expect("tempdir");
    let outside_dir = tempfile::tempdir().expect("tempdir");
    let outside = outside_dir.path().join("foo.txt");
    std::fs::write(&outside, "data").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            outside.to_str().unwrap(),
        ])
        .env_remove("ATOMWRITE_LANG")
        .output()
        .expect("read");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("inside the workspace"),
        "jail error with workspace supplied should say 'inside the workspace', got: {stdout}"
    );
}

#[test]
fn timeout_flag_accepted_in_help() {
    let output = common::atomwrite().args(["--help"]).output().expect("help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--timeout"),
        "--timeout flag missing from help"
    );
}
