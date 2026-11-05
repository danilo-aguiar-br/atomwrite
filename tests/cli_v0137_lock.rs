// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 tests for the advisory lock wired into `atomic_write` (R23).
//!
//! The harness plays the concurrent writer: it takes `flock(LOCK_EX)` on the
//! target's sidecar, then runs the real binary against the same target. Every
//! wait is bounded by `recv_timeout`, a child that does not exit is killed and
//! reaped, and no test sleeps.
#![cfg(unix)]

mod common;

use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Lock wait the binary applies before it gives up (exit 83).
const LOCK_WAIT: Duration = Duration::from_millis(atomwrite::constants::LOCK_ACQUIRE_TIMEOUT_MS);

/// Slack on top of the lock wait for process start and shutdown.
const SLACK: Duration = Duration::from_secs(20);

/// Hard cap on any child: past it the harness kills the process and fails.
const HARNESS_CAP: Duration = Duration::from_secs(40);

struct Outcome {
    code: Option<i32>,
    events: Vec<Value>,
    elapsed: Duration,
}

/// Sidecar path the product uses: `.<name>.atomwrite.lock` next to the target.
fn sidecar(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    target.with_file_name(format!(".{name}.atomwrite.lock"))
}

/// Take the advisory lock the way a concurrent writer would; dropping the
/// returned file releases it.
///
/// v0.1.38 and ADR-0079 — the lock lives on the target's own inode, so the
/// harness locks the target file. The sidecar is best-effort compatibility and
/// no longer gates a write.
fn hold_lock(target: &Path) -> File {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(target)
        .expect("open target");
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    assert_eq!(rc, 0, "harness could not take the lock");
    file
}

/// Run `atomwrite write target` with `payload` on stdin (closed after the
/// payload) and wait for it with a bound.
fn run_write(ws: &Path, target: &Path, payload: &[u8]) -> Outcome {
    let started = Instant::now();
    let mut child = common::hermetic(Command::new(env!("CARGO_BIN_EXE_atomwrite")))
        .args([
            "--workspace",
            ws.to_str().expect("utf8 workspace"),
            "--no-progress",
            "--no-auto-heal",
            "--timeout-secs",
            "120",
            "write",
            "--no-backup",
        ])
        .arg(target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn atomwrite");
    let mut stdin = child.stdin.take().expect("piped stdin");
    stdin.write_all(payload).expect("write payload");
    drop(stdin);
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(HARNESS_CAP) {
        Ok(Ok(out)) => Outcome {
            code: out.status.code(),
            events: String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect(),
            elapsed: started.elapsed(),
        },
        Ok(Err(e)) => panic!("wait failed: {e}"),
        Err(_) => {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
            let _ = rx.recv_timeout(Duration::from_secs(10));
            panic!("write still alive after {HARNESS_CAP:?} (killed by the harness)")
        }
    }
}

fn leftover_tmp_files(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .expect("read_dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".atomwrite-") && n.ends_with(".tmp"))
        .collect()
}

#[test]
fn r23_second_writer_exits_83_while_target_is_locked() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("alvo.txt");
    std::fs::write(&target, "original\n").expect("seed");
    let holder = hold_lock(&target);

    let out = run_write(ws.path(), &target, b"intruder\n");

    assert_eq!(
        out.code,
        Some(83),
        "expected exit 83, events: {:?}",
        out.events
    );
    assert!(
        out.elapsed >= LOCK_WAIT,
        "must wait the full lock deadline, waited {:?}",
        out.elapsed
    );
    assert!(
        out.elapsed < LOCK_WAIT + SLACK,
        "must give up near the deadline, took {:?}",
        out.elapsed
    );
    let err = out
        .events
        .iter()
        .find(|e| e["code"] == "LOCK_TIMEOUT")
        .unwrap_or_else(|| panic!("no LOCK_TIMEOUT envelope in {:?}", out.events));
    let path = err["path"].as_str().unwrap_or_default();
    assert!(
        path.ends_with("alvo.txt"),
        "error must carry the real target path: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("read target"),
        "original\n",
        "the locked target must stay intact"
    );
    assert!(
        leftover_tmp_files(ws.path()).is_empty(),
        "tempfile left behind"
    );
    drop(holder);
}

#[test]
fn r23_write_succeeds_after_holder_releases_and_leaves_no_sidecar() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("alvo.txt");
    std::fs::write(&target, "original\n").expect("seed");
    drop(hold_lock(&target));

    let out = run_write(ws.path(), &target, b"updated!!\n");

    assert_eq!(
        out.code,
        Some(0),
        "expected exit 0, events: {:?}",
        out.events
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("read"),
        "updated!!\n"
    );
    assert!(
        !sidecar(&target).exists(),
        "the lock sidecar must not survive a clean write"
    );
}

#[test]
fn r23_plain_write_creates_no_sidecar() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("novo.txt");

    let out = run_write(ws.path(), &target, b"fresh\n");

    assert_eq!(
        out.code,
        Some(0),
        "expected exit 0, events: {:?}",
        out.events
    );
    assert_eq!(std::fs::read_to_string(&target).expect("read"), "fresh\n");
    assert!(
        !sidecar(&target).exists(),
        "the lock sidecar must not survive"
    );
}
