// SPDX-License-Identifier: MIT OR Apache-2.0

//! The global `--join KEY:SUFFIX` reads an NDJSON table from stdin once and
//! attaches the matching rows to every data event under SUFFIX. A table it
//! cannot use, or a command that needs stdin itself, is `INVALID_INPUT` (65).

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

struct Out {
    code: Option<i32>,
    events: Vec<Value>,
}

/// Run `args` in `ws` with `stdin` piped in and closed afterwards.
fn run(ws: &Path, args: &[&str], stdin: &[u8]) -> Out {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = common::hermetic(Command::new(bin))
        .args([
            "-v",
            "--workspace",
            ws.to_str().expect("utf8"),
            "--no-progress",
            "--no-auto-heal",
        ])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn atomwrite");
    if let Some(mut pipe) = child.stdin.take() {
        // A refused table may close stdin before the whole table is written.
        let _ = pipe.write_all(stdin);
    }
    let output = child.wait_with_output().expect("wait atomwrite");
    Out {
        code: output.status.code(),
        events: common::parse_ndjson(&output.stdout),
    }
}

fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("workspace");
    common::create_test_file(dir.path(), "a.txt", "alpha\n");
    common::create_test_file(dir.path(), "b.txt", "beta\n");
    dir
}

fn entry<'a>(out: &'a Out, path: &str) -> &'a Value {
    out.events
        .iter()
        .find(|e| e["type"] == "entry" && e["path"] == path)
        .unwrap_or_else(|| panic!("no entry for {path}: {:?}", out.events))
}

fn assert_refused(out: &Out, expected_in_message: &str) {
    assert_eq!(out.code, Some(65), "events: {:?}", out.events);
    let failed: Vec<&Value> = out.events.iter().filter(|e| e["ok"] == false).collect();
    assert_eq!(failed.len(), 1, "one failure: {:?}", out.events);
    assert_eq!(failed[0]["code"], "INVALID_INPUT");
    let message = failed[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("--join"),
        "message names the flag: {message}"
    );
    assert!(
        message.contains(expected_in_message),
        "{expected_in_message} not in {message}"
    );
    let suggestion = failed[0]["suggestion"].as_str().unwrap_or_default();
    assert!(
        suggestion.contains("--join"),
        "join-specific hint: {suggestion}"
    );
}

#[test]
fn matching_rows_are_attached_in_arrival_order() {
    let ws = workspace();
    let table = b"{\"path\":\"a.txt\",\"owner\":\"ann\"}\n{\"path\":\"a.txt\",\"owner\":\"bob\"}\n";
    let out = run(ws.path(), &["--join", "path:meta", "list", "."], table);
    assert_eq!(out.code, Some(0), "events: {:?}", out.events);
    let meta = entry(&out, "a.txt")["meta"].as_array().expect("meta array");
    assert_eq!(meta.len(), 2);
    assert_eq!(meta[0]["owner"], "ann");
    assert_eq!(meta[1]["owner"], "bob");
}

#[test]
fn an_event_without_a_row_keeps_its_own_shape() {
    let ws = workspace();
    let table = b"{\"path\":\"a.txt\",\"owner\":\"ann\"}\n";
    let out = run(ws.path(), &["--join", "path:meta", "list", "."], table);
    assert!(entry(&out, "b.txt").get("meta").is_none());
    let summary = out.events.last().expect("summary");
    assert_eq!(summary["type"], "summary");
    assert!(summary.get("meta").is_none(), "the summary is never joined");
}

#[test]
fn several_joins_attach_several_suffixes() {
    let ws = workspace();
    let table = b"{\"path\":\"a.txt\",\"kind\":\"file\",\"team\":\"core\"}\n";
    let out = run(
        ws.path(),
        &[
            "--join",
            "path:by_path",
            "--join",
            "kind:by_kind",
            "list",
            ".",
        ],
        table,
    );
    assert_eq!(out.code, Some(0), "events: {:?}", out.events);
    let a = entry(&out, "a.txt");
    assert_eq!(a["by_path"][0]["team"], "core");
    assert_eq!(a["by_kind"][0]["team"], "core");
    assert_eq!(entry(&out, "b.txt")["by_kind"][0]["team"], "core");
}

#[test]
fn join_runs_before_select_so_the_projection_can_keep_the_suffix() {
    let ws = workspace();
    let table = b"{\"path\":\"a.txt\",\"owner\":\"ann\"}\n";
    let out = run(
        ws.path(),
        &[
            "--select",
            "path,meta",
            "--filter",
            "path=a.txt",
            "--join",
            "path:meta",
            "list",
            ".",
        ],
        table,
    );
    assert_eq!(out.code, Some(0), "events: {:?}", out.events);
    let kept = &out.events[0];
    assert_eq!(kept["path"], "a.txt");
    assert_eq!(kept["meta"][0]["owner"], "ann");
    assert!(kept.get("kind").is_none(), "projected away: {kept}");
}

#[test]
fn join_composes_with_a_tail_flag() {
    let ws = workspace();
    let table = b"{\"path\":\"b.txt\",\"owner\":\"bob\"}\n";
    let out = run(
        ws.path(),
        &["--sort-by", "path", "--join", "path:meta", "list", "."],
        table,
    );
    assert_eq!(out.code, Some(0), "events: {:?}", out.events);
    assert_eq!(entry(&out, "b.txt")["meta"][0]["owner"], "bob");
}

#[test]
fn a_failed_run_envelope_is_not_joined() {
    let ws = workspace();
    let table = b"{\"path\":\"missing\",\"owner\":\"ann\"}\n";
    let out = run(
        ws.path(),
        &["--join", "path:meta", "read", "missing"],
        table,
    );
    assert_ne!(out.code, Some(0));
    let failed = out
        .events
        .iter()
        .find(|e| e["ok"] == false)
        .expect("failure");
    assert!(failed.get("meta").is_none(), "{failed}");
}

#[test]
fn an_invalid_spec_is_refused() {
    let ws = workspace();
    let out = run(
        ws.path(),
        &["--join", "path", "list", "."],
        b"{\"path\":\"a\"}\n",
    );
    assert_refused(&out, "key:suffix");
}

#[test]
fn a_line_that_is_not_an_object_is_refused() {
    let ws = workspace();
    let out = run(ws.path(), &["--join", "path:meta", "list", "."], b"[1,2]\n");
    assert_refused(&out, "NDJSON objects");
    let out = run(
        ws.path(),
        &["--join", "path:meta", "list", "."],
        b"not json\n",
    );
    assert_refused(&out, "not JSON");
}

#[test]
fn a_key_absent_from_the_table_is_refused() {
    let ws = workspace();
    let out = run(
        ws.path(),
        &["--join", "path:meta", "list", "."],
        b"{\"other\":\"x\"}\n",
    );
    assert_refused(&out, "\"path\"");
    let out = run(ws.path(), &["--join", "path:meta", "list", "."], b"");
    assert_refused(&out, "\"path\"");
}

#[test]
fn a_command_that_reads_stdin_is_refused_without_reading_it() {
    let ws = workspace();
    let table = b"{\"path\":\"a.txt\"}\n";
    for args in [
        vec!["--join", "path:m", "write", "z.txt"],
        vec!["--join", "path:m", "edit-loop", "a.txt"],
        vec!["--join", "path:m", "apply", "a.txt"],
        vec!["--join", "path:m", "batch"],
        vec!["--join", "path:m", "hash", "--stdin"],
        vec!["--join", "path:m", "diff", "-", "b.txt"],
        vec!["--join", "path:m", "extract", "--field", "path"],
    ] {
        let out = run(ws.path(), &args, table);
        assert_refused(&out, "already reads stdin");
    }
    assert!(!ws.path().join("z.txt").exists(), "write must not have run");
}

#[test]
fn a_command_that_only_reads_stdin_on_demand_still_accepts_a_join() {
    let ws = workspace();
    let table = b"{\"path\":\"a.txt\",\"owner\":\"ann\"}\n";
    let out = run(ws.path(), &["--join", "path:meta", "hash", "a.txt"], table);
    assert_eq!(out.code, Some(0), "events: {:?}", out.events);
}

#[test]
fn a_table_above_the_ceiling_is_refused() {
    let ws = workspace();
    let line = format!("{{\"path\":\"a.txt\",\"pad\":\"{}\"}}\n", "x".repeat(1000));
    let rows = (atomwrite::constants::JOIN_TABLE_MAX_BYTES as usize / line.len()) + 2;
    let table = line.repeat(rows);
    let out = run(
        ws.path(),
        &["--join", "path:meta", "list", "."],
        table.as_bytes(),
    );
    assert_refused(&out, "larger than");
}

#[test]
fn a_terminal_stdin_is_refused() {
    let specs = [atomwrite::output::parse_join_expr("path:meta").expect("spec")];
    let mut empty = std::io::empty();
    let err = atomwrite::output::read_join_tables(&mut empty, true, false, &specs)
        .expect_err("a terminal cannot carry a table");
    let text = format!("{err:#}");
    assert!(
        text.contains("--join") && text.contains("terminal"),
        "{text}"
    );
}
