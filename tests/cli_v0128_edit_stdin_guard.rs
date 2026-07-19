// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.28 GAP-CLI-SURFACE-DRIFT — `edit` stdin-mode tty guard, E2E layer.
//!
//! `edit --after-line/--before-line/--range/--after-match/--before-match/
//! --between/--multi` read new content from stdin. Without a guard, running
//! one of these under an interactive terminal (stdin is a tty) would hang
//! forever waiting for input the user never intended to type. The guard
//! (`read_stdin_text_guarded` in `src/commands/mod.rs`) rejects a tty stdin
//! immediately with `InvalidInput` (exit 65) instead of blocking.
//!
//! `read_stdin_text_guarded` is `pub(crate)`, so its `stdin_is_tty == true`
//! branch is unit-tested directly in `src/commands/mod.rs` (`tty_guard_tests`
//! module) where the crate-private function is reachable. `assert_cmd`
//! spawns a real child process whose stdin cannot be attached to a
//! synthetic pseudo-terminal from this test harness, so the `true` branch
//! is not re-exercised here. This file instead proves the two things that
//! ARE observable from outside the crate: the happy path (piped, non-tty
//! stdin) works end-to-end, and a closed/empty stdin (also non-tty) never
//! hangs — the guard only ever blocks on an actual terminal, never on an
//! ordinary pipe.

mod common;

use std::time::{Duration, Instant};

#[test]
fn after_match_with_piped_stdin_succeeds_end_to_end() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "f.txt", "before\nmarker\nafter\n");

    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "f.txt",
            "--after-match",
            "marker",
        ])
        .write_stdin("inserted\n")
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "edit --after-match with stdin via pipe must succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("inserted"));
}

#[test]
fn between_mode_with_closed_stdin_completes_quickly_without_hanging() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "f.txt", "start\nold\nend\n");

    let start = Instant::now();
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "f.txt",
            "--between",
            "start",
            "end",
        ])
        .write_stdin("")
        .output()
        .expect("run");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "edit with closed stdin (non-tty) must not hang; took {elapsed:?}"
    );
    assert!(
        output.status.success(),
        "closed stdin (immediate EOF) and non-tty: must succeed with empty content, not hang"
    );
}

#[test]
fn range_mode_with_closed_stdin_completes_quickly_without_hanging() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "f.txt", "a\nb\nc\n");

    let start = Instant::now();
    let output = common::atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "edit",
            "f.txt",
            "--range",
            "2:3",
        ])
        .write_stdin("")
        .output()
        .expect("run");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "edit --range with closed stdin must not hang; took {elapsed:?}"
    );
    assert!(output.status.success());
}
