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

#[test]
fn write_expect_checksum_accepts_correct() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("locked.txt");
    std::fs::write(&target, "original content\n").expect("write");

    let hash_out = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
        .arg(&target)
        .output()
        .expect("hash");
    let hash_events = common::parse_ndjson(&hash_out.stdout);
    let checksum = hash_events[0]["value"].as_str().expect("checksum value");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--expect-checksum",
            checksum,
        ])
        .arg(&target)
        .write_stdin("updated content\n")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "correct checksum should succeed: {:?}",
        output.status
    );
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "updated content\n");
}

#[test]
fn write_expect_checksum_drift_after_external_modify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("race.txt");
    std::fs::write(&target, "version1").expect("write");

    let hash_out = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
        .arg(&target)
        .output()
        .expect("hash");
    let checksum = common::parse_ndjson(&hash_out.stdout)[0]["value"]
        .as_str()
        .expect("value")
        .to_string();

    std::fs::write(&target, "version2-external-change").expect("external modify");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--expect-checksum",
            &checksum,
        ])
        .arg(&target)
        .write_stdin("version3")
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(82),
        "should detect external modification"
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "STATE_DRIFT");
    assert_eq!(events[0]["retryable"], true);

    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(
        content, "version2-external-change",
        "original external change preserved"
    );
}

// --- G118 regression tests (v0.1.15): target resolution must not depend on CWD ---
// Before the fix, append/prepend, line-ending auto-detection, and --expect-checksum
// used the raw CLI path relative to the CWD. With a RELATIVE target and a CWD that
// differs from --workspace, append truncated the file and checksum verification was
// silently skipped (CWE-367 check-then-act on divergent path identities).

#[test]
fn append_with_cwd_outside_workspace_preserves_existing_content() {
    let ws = tempfile::tempdir().expect("ws");
    let cwd = tempfile::tempdir().expect("cwd");
    std::fs::write(ws.path().join("alvo.md"), "linha1\nlinha2\n").expect("seed");

    let output = common::atomwrite()
        .current_dir(cwd.path())
        .args([
            "--workspace",
            ws.path().to_str().unwrap(),
            "write",
            "--append",
            "alvo.md",
        ])
        .write_stdin("linha3\n")
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);
    let content = std::fs::read_to_string(ws.path().join("alvo.md")).expect("read");
    assert_eq!(
        content, "linha1\nlinha2\nlinha3\n",
        "append must preserve existing lines"
    );
}

#[test]
fn prepend_with_cwd_outside_workspace_preserves_existing_content() {
    let ws = tempfile::tempdir().expect("ws");
    let cwd = tempfile::tempdir().expect("cwd");
    std::fs::write(ws.path().join("alvo.md"), "linha2\nlinha3\n").expect("seed");

    let output = common::atomwrite()
        .current_dir(cwd.path())
        .args([
            "--workspace",
            ws.path().to_str().unwrap(),
            "write",
            "--prepend",
            "alvo.md",
        ])
        .write_stdin("linha1\n")
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);
    let content = std::fs::read_to_string(ws.path().join("alvo.md")).expect("read");
    assert_eq!(
        content, "linha1\nlinha2\nlinha3\n",
        "prepend must preserve existing lines"
    );
}

#[test]
fn expect_checksum_mismatch_with_cwd_outside_workspace_exits_82() {
    let ws = tempfile::tempdir().expect("ws");
    let cwd = tempfile::tempdir().expect("cwd");
    std::fs::write(ws.path().join("alvo.md"), "conteudo original\n").expect("seed");
    let zeros = "0".repeat(64);

    let output = common::atomwrite()
        .current_dir(cwd.path())
        .args([
            "--workspace",
            ws.path().to_str().unwrap(),
            "write",
            "--expect-checksum",
            &zeros,
            "alvo.md",
        ])
        .write_stdin("sobrescrita\n")
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(82), "must fail with STATE_DRIFT");
    let content = std::fs::read_to_string(ws.path().join("alvo.md")).expect("read");
    assert_eq!(
        content, "conteudo original\n",
        "file must remain intact on drift"
    );
}

#[test]
fn expect_checksum_match_with_cwd_outside_workspace_succeeds() {
    let ws = tempfile::tempdir().expect("ws");
    let cwd = tempfile::tempdir().expect("cwd");
    let seed = "conteudo original\n";
    std::fs::write(ws.path().join("alvo.md"), seed).expect("seed");
    let checksum = blake3::hash(seed.as_bytes()).to_hex().to_string();

    let output = common::atomwrite()
        .current_dir(cwd.path())
        .args([
            "--workspace",
            ws.path().to_str().unwrap(),
            "write",
            "--expect-checksum",
            &checksum,
            "alvo.md",
        ])
        .write_stdin("novo conteudo\n")
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);
    let content = std::fs::read_to_string(ws.path().join("alvo.md")).expect("read");
    assert_eq!(content, "novo conteudo\n");
}

#[test]
fn line_ending_auto_with_cwd_outside_workspace_detects_existing() {
    let ws = tempfile::tempdir().expect("ws");
    let cwd = tempfile::tempdir().expect("cwd");
    std::fs::write(ws.path().join("alvo.txt"), "primeira\r\n").expect("seed");

    let output = common::atomwrite()
        .current_dir(cwd.path())
        .args([
            "--workspace",
            ws.path().to_str().unwrap(),
            "write",
            "alvo.txt",
        ])
        .write_stdin("nova linha\n")
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);
    let content = std::fs::read(ws.path().join("alvo.txt")).expect("read");
    assert_eq!(
        content, b"nova linha\r\n",
        "auto mode must detect CRLF from the existing file even with divergent CWD"
    );
}

/// Conformance guard (G118): `cmd_write` must operate on the resolved path.
/// `args.target` may appear only to build `resolved` and the display string.
#[test]
fn write_source_uses_resolved_path_in_pre_steps() {
    let source = include_str!("../src/commands/write.rs");
    let raw_uses = source.matches("&args.target").count();
    assert_eq!(
        raw_uses, 1,
        "src/commands/write.rs must reference &args.target exactly once \
         (inside validate_path); pre-steps must use the resolved path (G118)"
    );
}

// ============================================================================
// G120 — Empty stdin guard (L1, L2, L4)
// ============================================================================

/// G120 L1: empty stdin via pipe must exit 65 INVALID_INPUT by default.
#[test]
fn g120_empty_stdin_rejected_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("alvo.txt");
    std::fs::write(&target, "conteudo original\n").expect("seed");

    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .write_stdin("") // explicit empty stdin
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(65),
        "must reject empty stdin with exit 65"
    );

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["error"], true);
    assert_eq!(events[0]["code"], "INVALID_INPUT");
    assert!(
        events[0]["message"].as_str().unwrap().contains("0 bytes"),
        "message must mention 0 bytes, got: {}",
        events[0]["message"]
    );
    assert!(
        events[0]["message"]
            .as_str()
            .unwrap()
            .contains("--allow-empty-stdin"),
        "message must point to the opt-out flag"
    );

    // File must remain untouched (G120 C1 prevention: no silent data loss).
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "conteudo original\n");
}

/// G120 L1 opt-in: `--allow-empty-stdin` truncates the file to 0 bytes and
/// emits `stdin_bytes_read: 0` in the success envelope (L4 telemetry).
#[test]
fn g120_empty_stdin_allowed_with_flag_truncates() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("alvo.txt");
    std::fs::write(&target, "conteudo original\n").expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--allow-empty-stdin",
        ])
        .arg(&target)
        .write_stdin("")
        .output()
        .expect("run");

    assert!(output.status.success(), "exit: {:?}", output.status);

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["type"], "write");
    assert_eq!(events[0]["status"], "success");
    assert_eq!(events[0]["stdin_bytes_read"], 0);
    assert_eq!(events[0]["bytes_written"], 0);

    // File is now empty — caller explicitly opted in.
    let content = std::fs::read(&target).expect("read");
    assert!(
        content.is_empty(),
        "file must be empty after --allow-empty-stdin"
    );
}

/// G120 L2: --append with empty stdin must exit 65 even when target exists.
#[test]
fn g120_append_empty_stdin_rejected_with_existing_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("alvo.txt");
    std::fs::write(&target, "linha 1\nlinha 2\n").expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--append",
        ])
        .arg(&target)
        .write_stdin("")
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(65),
        "L1 must reject --append + empty stdin"
    );

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["code"], "INVALID_INPUT");
    // L1 (read_stdin_content) fires before L2 (handle_append_prepend) because
    // the empty read is detected first; the L1 message names the opt-out flag
    // and is the action-facing diagnostic. L2 is the second line of defence
    // that runs when --allow-empty-stdin bypasses L1 and --append sees 0 bytes.
    let msg = events[0]["message"].as_str().unwrap();
    assert!(
        msg.contains("--allow-empty-stdin") || msg.contains("--append"),
        "message must mention either the opt-out flag or the operation, got: {msg}"
    );

    // File must be untouched.
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "linha 1\nlinha 2\n");
}

/// G120 L4: happy path with non-empty stdin must report `stdin_bytes_read`
/// matching the actual byte count read from stdin.
#[test]
fn g120_stdin_bytes_read_reflects_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("alvo.txt");

    let payload = "hello, world\n"; // 13 bytes
    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "write"])
        .arg(&target)
        .write_stdin(payload)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["stdin_bytes_read"], 13);
    assert_eq!(events[0]["bytes_written"], 13);
}

/// G120 L3: cross-validation --append + --expect-checksum + empty stdin +
/// --allow-empty-stdin must succeed without modifying the file. The
/// checksum refers to the pre-mutation state; the empty append is a
/// no-op; the L3 logic must verify the pre-mutation file against the
/// expected hash and accept the no-op append.
#[test]
fn g120_l3_append_empty_stdin_with_matching_checksum_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("locked.txt");
    let original = "linha 1\nlinha 2\n";
    std::fs::write(&target, original).expect("seed");

    let hash_out = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
        .arg(&target)
        .output()
        .expect("hash");
    let checksum = common::parse_ndjson(&hash_out.stdout)[0]["value"]
        .as_str()
        .expect("value")
        .to_string();

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--append",
            "--allow-empty-stdin",
            "--expect-checksum",
            &checksum,
        ])
        .arg(&target)
        .write_stdin("")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "L3 cross-validation must pass when pre-mutation hash matches: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "write");
    assert_eq!(events[0]["stdin_bytes_read"], 0);
    // File content is unchanged because empty append is a no-op.
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, original);
}

/// G120 L3: cross-validation --append + --expect-checksum + empty stdin
/// WITHOUT --allow-empty-stdin must fail at L1 (the empty-stdin guard
/// fires before L3's checksum cross-validation has a chance to run).
/// This preserves the G120 C1 invariant: no silent data loss.
#[test]
fn g120_l3_append_empty_stdin_without_opt_in_rejects_at_l1() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("locked.txt");
    std::fs::write(&target, "linha 1\n").expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--append",
            "--expect-checksum",
            "deadbeef",
        ])
        .arg(&target)
        .write_stdin("")
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(65),
        "L1 must fire first; L3 is unreachable without --allow-empty-stdin"
    );
    // File unchanged.
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "linha 1\n");
}

// ============================================================================
// v0.1.20 GAP-2026-011: write intention guards regression tests
// ============================================================================

/// L2 — --require-backup without --backup AND target exists must fail with
/// exit 65 (InvalidInput). This is the hard guard that prevents silent
/// overwrites of important files, motivated by the 2026-06-15 incident
/// (c24-framework34.html lost 127 lines).
#[test]
fn v0_1_20_require_backup_blocks_overwrite_without_backup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("important.html");
    std::fs::write(&target, "<html>existing</html>\n").expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--require-backup",
            "--no-backup",
        ])
        .arg(&target)
        .write_stdin("<html>new</html>\n")
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(65),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // File unchanged.
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "<html>existing</html>\n");
}

/// L2 — --require-backup combined with --backup must succeed.
#[test]
fn v0_1_20_require_backup_with_backup_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("important.html");
    std::fs::write(&target, "<html>existing</html>\n").expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--require-backup",
            "--backup",
            "--retention",
            "3",
        ])
        .arg(&target)
        .write_stdin("<html>new</html>\n")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&target).expect("read");
    assert_eq!(content, "<html>new</html>\n");
}

/// L1 — risk_assessment telemetry: size delta of 90% must emit high risk.
#[test]
fn v0_1_20_risk_assessment_high_delta_emits_warning() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("data.bin");
    std::fs::write(&target, "x".repeat(1000)).expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--risk-threshold",
            "50",
        ])
        .arg(&target)
        .write_stdin("y") // 1 byte vs 1000 bytes = 99.9% delta
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let risk = events[0]["risk_assessment"]
        .as_object()
        .expect("risk_assessment present");
    assert_eq!(risk["risk_level"], "high");
    assert_eq!(risk["guard_triggered"], "size");
    assert!(risk["size_delta_pct"].as_u64().unwrap() >= 90);
}

/// L1 — small delta (10%) must NOT emit risk_assessment (silent).
#[test]
fn v0_1_20_risk_assessment_small_delta_silent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("data.bin");
    std::fs::write(&target, "x".repeat(1000)).expect("seed");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--risk-threshold",
            "50",
        ])
        .arg(&target)
        .write_stdin("y".repeat(900)) // 10% delta
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    // risk_assessment should be null/absent for small delta.
    assert!(events[0].get("risk_assessment").is_none() || events[0]["risk_assessment"].is_null());
}

/// GAP-2026-002 — --preserve-timestamps keeps mtime stable.
#[cfg(unix)]
#[test]
fn v0_1_20_write_preserve_timestamps_keeps_mtime() {
    use std::os::unix::fs::MetadataExt;
    use std::time::Duration;

    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("data.txt");
    std::fs::write(&target, "original").expect("seed");
    let original_mtime = std::fs::metadata(&target).unwrap().mtime();
    std::thread::sleep(Duration::from_millis(50));

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--preserve-timestamps",
        ])
        .arg(&target)
        .write_stdin("updated")
        .output()
        .expect("run");

    assert!(output.status.success());
    let new_mtime = std::fs::metadata(&target).unwrap().mtime();
    assert_eq!(original_mtime, new_mtime, "mtime must be preserved");
}

/// GAP-2026-004 — --line-ending crlf without hyphen must work.
#[test]
fn v0_1_20_write_line_ending_crlf_no_hyphen_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("out.txt");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--line-ending",
            "crlf",
        ])
        .arg(&target)
        .write_stdin("line1\nline2\n")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read(&target).expect("read");
    assert_eq!(content, b"line1\r\nline2\r\n");
}

/// GAP-2026-004 — --line-ending cr-lf with hyphen must also work.
#[test]
fn v0_1_20_write_line_ending_cr_lf_with_hyphen_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("out.txt");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--line-ending",
            "cr-lf",
        ])
        .arg(&target)
        .write_stdin("a\nb\n")
        .output()
        .expect("run");

    assert!(output.status.success());
    let content = std::fs::read(&target).expect("read");
    assert_eq!(content, b"a\r\nb\r\n");
}

/// L2 edge case — --require-backup on NEW file (no existing) must succeed
/// even without --backup (target doesn't exist, nothing to guard against).
#[test]
fn v0_1_20_require_backup_new_file_succeeds_without_backup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("brand_new.txt");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "write",
            "--require-backup",
        ])
        .arg(&target)
        .write_stdin("new content")
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(target.exists());
}
