// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 tests for the retry wait on a contended lock (`src/retry.rs`).
//!
//! The harness holds `flock(LOCK_EX)` on the target's sidecar, then runs the
//! real binary against the same target and measures how long the wait lasts.
//! The lock wait alone is 5 s (`LOCK_ACQUIRE_TIMEOUT_MS`), so a run that ends
//! much sooner proves the wait obeys the global deadline or the signal instead
//! of its own timeout. No test sleeps: every wait is a bounded channel receive.
#![cfg(unix)]

mod common;

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Hard cap on any child: past it the harness kills the process and fails.
const HARNESS_CAP: Duration = Duration::from_secs(30);

/// Lock wait the binary applies on its own before exit 83.
const LOCK_WAIT: Duration = Duration::from_millis(atomwrite::constants::LOCK_ACQUIRE_TIMEOUT_MS);

/// Global deadline given to the binary in the deadline test, in seconds.
const DEADLINE_SECS: u64 = 1;

/// Time the child is left waiting on the lock before the signal test fires.
const WAIT_BEFORE_SIGNAL: Duration = Duration::from_millis(400);

/// Take the advisory lock the way a concurrent writer would.
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

/// Start `atomwrite write target` with the payload already on its stdin.
fn spawn_write(ws: &Path, target: &Path, global: &[&str]) -> Child {
    let mut child = common::hermetic(Command::new(env!("CARGO_BIN_EXE_atomwrite")))
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(["--no-progress", "--no-auto-heal"])
        .args(global)
        .args(["write", "--no-backup"])
        .arg(target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn atomwrite");
    let mut stdin = child.stdin.take().expect("piped stdin");
    stdin.write_all(b"intruder\n").expect("write payload");
    drop(stdin);
    child
}

/// Wait for the child on a helper thread; the receiver is the bounded wait.
fn wait_in_background(child: Child) -> (u32, mpsc::Receiver<std::process::Output>) {
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        if let Ok(out) = child.wait_with_output() {
            tx.send(out).ok();
        }
    });
    (pid, rx)
}

fn kill_and_fail(pid: u32, what: &str) -> ! {
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
    panic!("{what}: child still alive after {HARNESS_CAP:?} (killed by the harness)");
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
fn c12_lock_wait_ends_at_the_global_deadline_with_exit_124() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("alvo.txt");
    std::fs::write(&target, "original\n").expect("seed");
    let holder = hold_lock(&target);

    let started = Instant::now();
    let child = spawn_write(
        ws.path(),
        &target,
        &["--timeout-secs", &DEADLINE_SECS.to_string()],
    );
    let (pid, rx) = wait_in_background(child);
    let out = rx
        .recv_timeout(HARNESS_CAP)
        .unwrap_or_else(|_| kill_and_fail(pid, "deadline"));
    let elapsed = started.elapsed();
    drop(holder);

    assert_eq!(
        out.status.code(),
        Some(124),
        "the deadline ends the wait as a cancel (124), not as a lock timeout (83); stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        elapsed >= Duration::from_secs(DEADLINE_SECS),
        "ended before the deadline: {elapsed:?}"
    );
    assert!(
        elapsed < LOCK_WAIT,
        "waited out the lock timeout ({LOCK_WAIT:?}) instead of the {DEADLINE_SECS}s deadline: {elapsed:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("read target"),
        "original\n",
        "a cancelled write must leave the target untouched"
    );
    assert!(
        leftover_tmp_files(ws.path()).is_empty(),
        "tempfile left behind"
    );
}

#[test]
fn c12_lock_wait_stops_on_sigterm_within_a_slice() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("alvo.txt");
    std::fs::write(&target, "original\n").expect("seed");
    let holder = hold_lock(&target);

    let child = spawn_write(ws.path(), &target, &["--timeout-secs", "120"]);
    let (pid, rx) = wait_in_background(child);
    // The child must still be waiting on the lock: this receive has to time out.
    assert!(
        rx.recv_timeout(WAIT_BEFORE_SIGNAL).is_err(),
        "the child finished while the lock was held"
    );
    let signalled = Instant::now();
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }
    let out = rx
        .recv_timeout(HARNESS_CAP)
        .unwrap_or_else(|_| kill_and_fail(pid, "sigterm"));
    let reaction = signalled.elapsed();
    drop(holder);

    assert_eq!(out.status.code(), Some(143), "SIGTERM is a clean cancel");
    assert!(
        reaction < Duration::from_secs(2),
        "the wait ignored SIGTERM for {reaction:?} (lock timeout is {LOCK_WAIT:?})"
    );
    assert!(
        leftover_tmp_files(ws.path()).is_empty(),
        "tempfile left behind"
    );
}

/// A non-blocking pipe whose buffer is already full, and its read end.
///
/// Nothing ever reads the pipe, so every write to the returned write end fails
/// with `EAGAIN`, the way a stalled log consumer looks to the process.
fn full_stderr_pipe() -> (std::os::fd::OwnedFd, std::os::fd::OwnedFd) {
    use std::os::fd::{FromRawFd, OwnedFd};
    let mut fds = [0i32; 2];
    let rc = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) };
    assert_eq!(rc, 0, "pipe2 failed");
    let (read_end, write_end) =
        unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    let chunk = [b'x'; 4096];
    loop {
        let n = unsafe {
            libc::write(
                write_end.as_raw_fd(),
                chunk.as_ptr().cast::<libc::c_void>(),
                chunk.len(),
            )
        };
        if n < 0 {
            break;
        }
    }
    (read_end, write_end)
}

#[test]
fn c12_cancel_banner_gives_up_on_a_full_stderr_pipe_instead_of_spinning() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("alvo.txt");
    std::fs::write(&target, "original\n").expect("seed");
    let holder = hold_lock(&target);
    let (_reader, writer) = full_stderr_pipe();

    let mut child = common::hermetic(Command::new(env!("CARGO_BIN_EXE_atomwrite")))
        .args(["--workspace", ws.path().to_str().expect("utf8 workspace")])
        .args(["--no-progress", "--no-auto-heal", "--timeout-secs", "120"])
        .args(["write", "--no-backup"])
        .arg(&target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(writer))
        .spawn()
        .expect("spawn atomwrite");
    let mut stdin = child.stdin.take().expect("piped stdin");
    stdin.write_all(b"intruder\n").expect("write payload");
    drop(stdin);
    let (pid, rx) = wait_in_background(child);
    assert!(
        rx.recv_timeout(WAIT_BEFORE_SIGNAL).is_err(),
        "the child finished while the lock was held"
    );
    let signalled = Instant::now();
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }
    let out = rx
        .recv_timeout(HARNESS_CAP)
        .unwrap_or_else(|_| kill_and_fail(pid, "full stderr"));
    let reaction = signalled.elapsed();
    drop(holder);

    assert_eq!(out.status.code(), Some(143), "SIGTERM is a clean cancel");
    assert!(
        reaction < Duration::from_secs(3),
        "the banner write spun on a full pipe for {reaction:?}"
    );
}

#[test]
fn c12_lock_wait_still_times_out_with_exit_83_when_no_deadline_is_near() {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("alvo.txt");
    std::fs::write(&target, "original\n").expect("seed");
    let holder = hold_lock(&target);

    let started = Instant::now();
    let child = spawn_write(ws.path(), &target, &["--timeout-secs", "120"]);
    let (pid, rx) = wait_in_background(child);
    let out = rx
        .recv_timeout(HARNESS_CAP)
        .unwrap_or_else(|_| kill_and_fail(pid, "lock timeout"));
    let elapsed = started.elapsed();
    drop(holder);

    assert_eq!(out.status.code(), Some(83));
    assert!(
        elapsed >= LOCK_WAIT,
        "the jittered wait stopped before the lock budget: {elapsed:?}"
    );
    assert!(
        elapsed < LOCK_WAIT + Duration::from_secs(5),
        "the last wait overran the lock budget: {elapsed:?}"
    );
}
