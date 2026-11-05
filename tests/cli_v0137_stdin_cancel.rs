// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 red tests for stdin readers that must honour SIGTERM and the global
//! deadline while stdin stays open without EOF (R2a..R2d).
//!
//! The harness keeps the child's stdin pipe open (it never closes it), so a
//! reader that blocks in `read()` hangs forever. Every wait is bounded with
//! `recv_timeout`, a child that does not exit is killed and reaped, and the
//! test fails with a message instead of hanging the suite.
#![cfg(unix)]

mod common;

use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// Bytes sent on stdin before the pipe is left open without EOF.
const PARTIAL_BYTES: usize = 1000;

/// Upper bound on how long the child may take to install its handlers.
const READY_CAP: Duration = Duration::from_secs(30);

/// Poll step of the bounded waits (a channel timeout, not a sleep).
const STEP: Duration = Duration::from_millis(5);

/// Live child plus everything needed to wait for it with a bound.
struct Live {
    pid: u32,
    rx: Receiver<std::io::Result<ExitStatus>>,
    /// Kept alive on purpose: dropping it would send EOF.
    stdin: Option<ChildStdin>,
    stdout_path: PathBuf,
    started: Instant,
}

impl Live {
    /// Wait for exit up to `cap`; on timeout kill the child, reap it, and fail.
    fn wait_exit(&mut self, cap: Duration, what: &str) -> (ExitStatus, Duration) {
        match self.rx.recv_timeout(cap) {
            Ok(Ok(status)) => (status, self.started.elapsed()),
            Ok(Err(e)) => panic!("{what}: wait failed: {e}"),
            Err(_) => {
                unsafe {
                    libc::kill(self.pid as i32, libc::SIGKILL);
                }
                let _ = self.rx.recv_timeout(Duration::from_secs(10));
                panic!("{what}: still alive after {cap:?} with stdin open (killed by the harness)")
            }
        }
    }

    fn stdout_events(&self) -> Vec<Value> {
        std::fs::read_to_string(&self.stdout_path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }
}

/// Spawn the binary with a piped, never-closed stdin that already received
/// `payload`, stdout redirected to a file, stderr discarded.
fn spawn_open_stdin(args: &[String], out_dir: &Path, payload: &[u8]) -> Live {
    let stdout_path = out_dir.join("stdout.ndjson");
    let out = File::create(&stdout_path).expect("create stdout file");
    let started = Instant::now();
    let mut child: Child =
        common::hermetic(Command::new(assert_cmd::cargo::cargo_bin("atomwrite")))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(out))
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn atomwrite");
    let mut stdin = child.stdin.take().expect("piped stdin");
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait());
    });
    stdin.write_all(payload).expect("write partial stdin");
    stdin.flush().expect("flush stdin");
    Live {
        pid,
        rx,
        stdin: Some(stdin),
        stdout_path,
        started,
    }
}

/// Block until the ready-file exists (handlers installed) or fail.
fn wait_ready(live: &mut Live, ready: &Path) {
    let start = Instant::now();
    let (_never_tx, never_rx) = mpsc::channel::<()>();
    while !ready.exists() {
        if let Ok(early) = live.rx.try_recv() {
            panic!("child exited before it became ready: {early:?}");
        }
        assert!(
            start.elapsed() < READY_CAP,
            "ready-file never appeared within {READY_CAP:?}"
        );
        let _ = never_rx.recv_timeout(STEP);
    }
}

fn base_args(ws: &Path, timeout_secs: u64) -> Vec<String> {
    vec![
        "--workspace".into(),
        ws.to_str().expect("utf8 workspace").into(),
        "--no-progress".into(),
        "--no-auto-heal".into(),
        "--timeout-secs".into(),
        timeout_secs.to_string(),
    ]
}

fn leftover_tmp_files(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .expect("read_dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".atomwrite-") && n.ends_with(".tmp"))
        .collect()
}

// RED until R2: the stdin read must be interruptible so SIGTERM cleans up and
// exits 143 even though stdin never reaches EOF; today the process stays alive.
#[test]
fn r2a_write_with_open_stdin_exits_143_on_sigterm() {
    let ws = tempfile::tempdir().expect("workspace");
    let out_dir = tempfile::tempdir().expect("out dir");
    let ready = out_dir.path().join("ready.pid");
    let target = ws.path().join("alvo.txt");
    let mut args = base_args(ws.path(), 600);
    args.extend([
        "--ready-file".into(),
        ready.to_str().expect("utf8").into(),
        "write".into(),
        target.to_str().expect("utf8").into(),
    ]);
    let mut live = spawn_open_stdin(&args, out_dir.path(), &[b'x'; PARTIAL_BYTES]);
    wait_ready(&mut live, &ready);
    unsafe {
        libc::kill(live.pid as i32, libc::SIGTERM);
    }
    let (status, elapsed) = live.wait_exit(Duration::from_secs(3), "write after SIGTERM");
    assert_eq!(
        status.code(),
        Some(143),
        "expected exit 143 after SIGTERM, got {status:?} in {elapsed:?}"
    );
    assert!(!target.exists(), "target must not be created after SIGTERM");
    let leftovers = leftover_tmp_files(ws.path());
    assert!(leftovers.is_empty(), "tempfile left behind: {leftovers:?}");
    drop(live.stdin.take());
}

/// Run `args` with open stdin and `--timeout-secs 2`; expect exit 124 within
/// 3.5 s of spawn and a `cancelled` envelope with `deadline_exceeded:true`.
fn assert_deadline_with_open_stdin(label: &str, verb_args: &[String], payload: &[u8], ws: &Path) {
    let out_dir = tempfile::tempdir().expect("out dir");
    let mut args = base_args(ws, 2);
    args.extend(verb_args.iter().cloned());
    let mut live = spawn_open_stdin(&args, out_dir.path(), payload);
    let (status, elapsed) = live.wait_exit(Duration::from_millis(3500), label);
    assert_eq!(
        status.code(),
        Some(124),
        "{label}: expected exit 124, got {status:?} after {elapsed:?}"
    );
    let events = live.stdout_events();
    let cancelled = events
        .iter()
        .find(|e| e["type"] == "cancelled")
        .unwrap_or_else(|| panic!("{label}: no cancelled envelope in {events:?}"));
    assert_eq!(
        cancelled["deadline_exceeded"],
        Value::Bool(true),
        "{label}: cancelled envelope must carry deadline_exceeded:true"
    );
    // A cancelled run must not also report a bogus error: swallowing the cancelled
    // read used to surface as a second `INVALID_INPUT` envelope (BUG-1).
    assert!(
        events
            .iter()
            .all(|e| e["code"] != "INVALID_INPUT" && e["ok"] != Value::Bool(false)),
        "{label}: a cancelled run must not emit an error envelope: {events:?}"
    );
    drop(live.stdin.take());
}

fn s(p: &Path) -> String {
    p.to_str().expect("utf8").to_string()
}

// RED until R2: the deadline must interrupt the blocking stdin read of `write`.
#[test]
fn r2b_write_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("alvo.txt");
    assert_deadline_with_open_stdin(
        "write",
        &["write".into(), s(&target)],
        &[b'x'; PARTIAL_BYTES],
        ws.path(),
    );
    assert!(!target.exists(), "target must not exist after the deadline");
    assert!(
        leftover_tmp_files(ws.path()).is_empty(),
        "tempfile left behind"
    );
}

// RED until R2: the deadline must interrupt the blocking stdin read of `edit-loop`.
#[test]
fn r2c_edit_loop_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("t.txt");
    std::fs::write(&target, "a\n").expect("fixture");
    assert_deadline_with_open_stdin(
        "edit-loop",
        &["edit-loop".into(), s(&target)],
        b"[{\"old\":\"a\",",
        ws.path(),
    );
}

// RED until R2: the deadline must interrupt the blocking stdin read of `apply`.
#[test]
fn r2c_apply_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("t.txt");
    std::fs::write(&target, "a\n").expect("fixture");
    assert_deadline_with_open_stdin(
        "apply",
        &["apply".into(), s(&target)],
        b"--- a/t.txt\n",
        ws.path(),
    );
}

// RED until R2: the deadline must interrupt the blocking stdin read of `hash --stdin`.
#[test]
fn r2c_hash_stdin_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    assert_deadline_with_open_stdin(
        "hash --stdin",
        &["hash".into(), "--stdin".into()],
        &[b'x'; PARTIAL_BYTES],
        ws.path(),
    );
}

// RED until R2: the deadline must interrupt the blocking stdin read of `extract --stdin`.
#[test]
fn r2c_extract_stdin_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    assert_deadline_with_open_stdin(
        "extract --stdin",
        &[
            "extract".into(),
            "--stdin".into(),
            "--field".into(),
            "a".into(),
        ],
        b"{\"a\":1",
        ws.path(),
    );
}

// RED until R2: the deadline must interrupt the blocking stdin read of `diff - FILE`
// (`diff - -` is rejected as invalid input, so one side is a real file).
#[test]
fn r2c_diff_stdin_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    let other = ws.path().join("other.txt");
    std::fs::write(&other, "a\n").expect("fixture");
    assert_deadline_with_open_stdin(
        "diff -",
        &["diff".into(), "-".into(), s(&other)],
        &[b'x'; PARTIAL_BYTES],
        ws.path(),
    );
}

// BUG-1: `regex --stdin` used to swallow the cancelled read, report a bogus "no
// example" input error and exit 65 instead of 124.
#[test]
fn r2c_regex_stdin_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    assert_deadline_with_open_stdin(
        "regex --stdin",
        &["regex".into(), "--stdin".into()],
        b"abc\n",
        ws.path(),
    );
}

#[test]
fn r2c_calc_stdin_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    assert_deadline_with_open_stdin(
        "calc --stdin",
        &["calc".into(), "--stdin".into()],
        b"1 + 1\n",
        ws.path(),
    );
}

#[test]
fn r2c_batch_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    assert_deadline_with_open_stdin(
        "batch",
        &["batch".into()],
        b"{\"op\":\"write\",\"target\":\"",
        ws.path(),
    );
}

#[test]
fn r2c_edit_insert_with_open_stdin_hits_deadline_124() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("t.txt");
    std::fs::write(&target, "a\n").expect("fixture");
    assert_deadline_with_open_stdin(
        "edit --after-line",
        &["edit".into(), s(&target), "--after-line".into(), "1".into()],
        b"x",
        ws.path(),
    );
}

// BUG-1: SIGTERM while `regex --stdin` waits for input must exit 143, not 65.
#[test]
fn r2a_regex_stdin_with_open_stdin_exits_143_on_sigterm() {
    let ws = tempfile::tempdir().expect("workspace");
    let out_dir = tempfile::tempdir().expect("out dir");
    let ready = out_dir.path().join("ready.pid");
    let mut args = base_args(ws.path(), 600);
    args.extend([
        "--ready-file".into(),
        ready.to_str().expect("utf8").into(),
        "regex".into(),
        "--stdin".into(),
    ]);
    let mut live = spawn_open_stdin(&args, out_dir.path(), b"abc\n");
    wait_ready(&mut live, &ready);
    unsafe {
        libc::kill(live.pid as i32, libc::SIGTERM);
    }
    let (status, elapsed) = live.wait_exit(Duration::from_secs(3), "regex after SIGTERM");
    assert_eq!(
        status.code(),
        Some(143),
        "expected exit 143 after SIGTERM, got {status:?} in {elapsed:?}"
    );
    let events = live.stdout_events();
    assert!(
        events
            .iter()
            .all(|e| e["code"] != "INVALID_INPUT" && e["ok"] != Value::Bool(false)),
        "a SIGTERM run must not emit an error envelope: {events:?}"
    );
    drop(live.stdin.take());
}

// Control (must PASS today): a write whose stdin ends normally still succeeds
// with exit 0 and a checksum.
#[test]
fn r2d_write_with_eof_stdin_still_succeeds() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("alvo.txt");
    let mut args = base_args(ws.path(), 60);
    args.extend(["write".into(), s(&target)]);
    let mut child = common::hermetic(Command::new(assert_cmd::cargo::cargo_bin("atomwrite")))
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().expect("stdin");
    stdin.write_all(b"hello\n").expect("write stdin");
    drop(stdin);
    let out = child.wait_with_output().expect("wait_with_output");
    assert_eq!(out.status.code(), Some(0), "status {:?}", out.status);
    let text = String::from_utf8_lossy(&out.stdout);
    let last: Value = serde_json::from_str(text.lines().last().expect("a stdout line"))
        .expect("last line is JSON");
    assert!(
        last["checksum"].as_str().is_some_and(|c| !c.is_empty()),
        "missing checksum in {last}"
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("read target"),
        "hello\n"
    );
}
