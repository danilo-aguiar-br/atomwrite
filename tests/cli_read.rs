// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn read_returns_content_and_checksum() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "hello.txt", "hello world\n");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["type"], "read");
    assert_eq!(events[0]["content"], "hello world\n");
    assert_eq!(events[0]["bytes"], 12);
    assert_eq!(events[0]["lines"], 1);
    assert!(events[0]["checksum"].is_string());
    assert_eq!(events[0]["binary"], false);
}

#[test]
fn read_stat_omits_content() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "stat.txt", "data\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--stat",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert!(events[0]["content"].is_null());
    assert_eq!(events[0]["bytes"], 5);
}

#[test]
fn read_lines_range() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "lines.txt", "a\nb\nc\nd\ne\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--lines",
            "2:4",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["content"], "b\nc\nd\n");
}

#[test]
fn read_format_raw_emits_bytes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "raw.txt", "raw content\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--format",
            "raw",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "raw content\n");
}

#[test]
fn read_not_found_exits_4() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(dir.path().join("nonexistent.txt"))
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(4));
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "FILE_NOT_FOUND");
}

#[test]
fn read_binary_file_detected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("binary.bin");
    std::fs::write(&path, b"\x00\x01\x02\x03binary").expect("write");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["binary"], true);
    assert!(events[0]["content"].is_null());
}

// --- GAP 01: --json flag aceita como no-op ---

#[test]
fn read_with_json_flag_is_noop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "json_test.txt", "data\n");

    let with_json = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--json",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(with_json.status.success(), "exit: {:?}", with_json.status);

    let without_json = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(without_json.status.success());
    let events_with = common::parse_ndjson(&with_json.stdout);
    let events_without = common::parse_ndjson(&without_json.stdout);
    assert_eq!(events_with[0]["type"], events_without[0]["type"]);
    assert_eq!(events_with[0]["content"], events_without[0]["content"]);
    assert_eq!(events_with[0]["checksum"], events_without[0]["checksum"]);
}

// --- GAP 07: erros Clap emitem JSON estruturado ---

#[test]
fn invalid_arg_emits_json_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "dummy.txt", "x");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--nonexistent-flag",
        ])
        .arg(dir.path().join("dummy.txt"))
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(2));
    let events = common::parse_ndjson(&output.stdout);
    assert!(!events.is_empty(), "stdout should contain JSON error");
    assert_eq!(events[0]["error"], true);
    assert_eq!(events[0]["code"], "ARGUMENT_PARSE_ERROR");
    assert_eq!(events[0]["exit"], 2);
    assert_eq!(events[0]["retryable"], false);
    assert_eq!(events[0]["error_class"], "permanent");
}

#[test]
fn help_flag_still_works() {
    let output = common::atomwrite().args(["--help"]).output().expect("run");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("atomwrite") || combined.contains("Usage"),
        "help should mention atomwrite or Usage"
    );
}

// --- GAP 10: caminhos relativos com workspace ---

#[test]
fn read_relative_path_with_workspace() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "rel_test.txt", "relative content\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "rel_test.txt",
        ])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "relative path should work with absolute workspace, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["content"], "relative content\n");
}

// --- GAP 08: --json-schema sem argumentos obrigatórios ---

#[test]
fn json_schema_write_without_args() {
    let output = common::atomwrite()
        .args(["write", "--json-schema"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "write --json-schema should work without <TARGET>, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON schema");
    assert!(parsed.is_object());
}

#[test]
fn json_schema_transform_without_args() {
    let output = common::atomwrite()
        .args(["transform", "--json-schema"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "transform --json-schema should work without --pattern/--rewrite/--language"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON schema");
    assert!(parsed.is_object());
}

#[test]
fn json_schema_all_subcommands_no_args() {
    let subcommands = [
        "read",
        "write",
        "edit",
        "search",
        "replace",
        "hash",
        "delete",
        "count",
        "diff",
        "move",
        "copy",
        "list",
        "extract",
        "calc",
        "regex",
        "transform",
        "scope",
        "batch",
        "backup",
        "rollback",
        "apply",
        "completions",
    ];

    for cmd in &subcommands {
        let output = common::atomwrite()
            .args([cmd, "--json-schema"])
            .output()
            .unwrap_or_else(|e| panic!("{cmd} --json-schema failed to run: {e}"));

        assert!(
            output.status.success(),
            "{cmd} --json-schema exited with {:?}",
            output.status.code()
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
        assert!(
            parsed.is_ok(),
            "{cmd} --json-schema produced invalid JSON: {stdout}"
        );
    }
}

#[test]
fn locale_flag_does_not_alter_json_output() {
    // ADR-0037 v0.1.20: `--lang` global was renamed to `--locale` to
    // free the namespace for `scope --lang`. Test verifies the new
    // `--locale` flag and that NDJSON output is locale-agnostic.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "lang.txt", "test content\n");
    let ws = dir.path().to_str().unwrap();

    let out_en = common::atomwrite()
        .args(["--locale", "en", "--workspace", ws, "read"])
        .arg(&path)
        .output()
        .expect("en");

    let out_pt = common::atomwrite()
        .args(["--locale", "pt-BR", "--workspace", ws, "read"])
        .arg(&path)
        .output()
        .expect("pt");

    assert!(out_en.status.success());
    assert!(out_pt.status.success());

    let events_en = common::parse_ndjson(&out_en.stdout);
    let events_pt = common::parse_ndjson(&out_pt.stdout);

    assert_eq!(
        events_en[0]["checksum"], events_pt[0]["checksum"],
        "checksum must be identical regardless of --locale"
    );
    assert_eq!(
        events_en[0]["content"], events_pt[0]["content"],
        "content must be identical regardless of --locale"
    );
    assert_eq!(
        events_en[0]["type"], events_pt[0]["type"],
        "type must be identical regardless of --locale"
    );
}

#[test]
fn scope_lang_alias_works() {
    // GAP-2026-003 v0.1.20: `--lang` is now a valid alias for
    // `--language` in `scope` after the global rename to `--locale`.
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "main.rs", "fn main() {}\n");
    let ws = dir.path().to_str().unwrap();

    let out = common::atomwrite()
        .args([
            "--workspace",
            ws,
            "scope",
            "--lang",
            "rust",
            "--query",
            "fn",
        ])
        .arg(dir.path())
        .output()
        .expect("scope --lang");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn scope_language_long_form_still_works() {
    // Regression: explicit --language must keep working post-rename.
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "main.rs", "fn main() {}\n");
    let ws = dir.path().to_str().unwrap();

    let out = common::atomwrite()
        .args([
            "--workspace",
            ws,
            "scope",
            "--language",
            "rust",
            "--query",
            "fn",
        ])
        .arg(dir.path())
        .output()
        .expect("scope --language");
    assert!(out.status.success());
}

#[test]
fn env_atomwrite_lang_still_honored() {
    // ADR-0037: env var name ATOMWRITE_LANG is preserved (backward compat).
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "x.txt", "x");
    let ws = dir.path().to_str().unwrap();

    let out = common::atomwrite()
        .env("ATOMWRITE_LANG", "pt-BR")
        .args(["--workspace", ws, "read"])
        .arg(dir.path().join("x.txt"))
        .output()
        .expect("env ATOMWRITE_LANG");
    assert!(out.status.success());
}

// ============================================================================
// v0.1.20 GAP-2026-008/009: read --mode field + filtered lines
// ============================================================================

/// GAP-009 read --head 2 produces mode: "head" and lines = 2.
#[test]
fn v0_1_20_read_head_emits_mode_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("data.txt");
    std::fs::write(&path, "a\nb\nc\nd\ne\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--head",
            "2",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["mode"], "head");
    assert_eq!(events[0]["lines"].as_u64().unwrap(), 2);
}

/// GAP-008 read --head 2 of 5-line file: lines=2 (filtered), lines_total=5.
#[test]
fn v0_1_20_read_head_reports_filtered_count() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("data.txt");
    std::fs::write(&path, "a\nb\nc\nd\ne\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--head",
            "2",
        ])
        .arg(&path)
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["lines"].as_u64().unwrap(), 2, "filtered count");
    assert_eq!(
        events[0]["lines_total"].as_u64().unwrap(),
        5,
        "total file count"
    );
}

/// GAP-009 read --tail mode discriminator.
#[test]
fn v0_1_20_read_tail_emits_mode_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("data.txt");
    std::fs::write(&path, "a\nb\nc\nd\ne\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--tail",
            "2",
        ])
        .arg(&path)
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["mode"], "tail");
}

/// GAP-009 read --line mode discriminator.
#[test]
fn v0_1_20_read_line_emits_mode_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("data.txt");
    std::fs::write(&path, "a\nb\nc\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--line",
            "2",
        ])
        .arg(&path)
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["mode"], "line");
}

/// GAP-009 read --stat mode discriminator.
#[test]
fn v0_1_20_read_stat_emits_mode_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("data.txt");
    std::fs::write(&path, "hello\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--stat",
        ])
        .arg(&path)
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["mode"], "stat");
}

/// GAP-009 read --grep mode discriminator.
#[test]
fn v0_1_20_read_grep_emits_mode_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("data.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\n").expect("write");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            "--grep",
            "alpha",
        ])
        .arg(&path)
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["mode"], "grep");
}
