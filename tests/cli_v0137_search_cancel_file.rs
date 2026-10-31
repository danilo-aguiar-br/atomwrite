// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37: `search` honours SIGTERM and `--timeout-secs` inside one large file.
//!
//! Before the cancellable reader, the flag was read between files and between
//! matches, so a single large file without a match was scanned to its end
//! before the run noticed it had been cancelled. The harness first times a
//! full scan, then cancels a second run a quarter of the way in and requires
//! the exit to follow the signal, not the end of the file. The pattern has no
//! literal, which keeps the regex engine scanning byte by byte instead of
//! skipping ahead with `memchr`.
#![cfg(unix)]

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Size of the scanned file in bytes.
const FILE_BYTES: usize = 160 * 1024 * 1024;

/// Pattern that never matches the generated lines and has no literal prefix.
const NEVER_MATCHES: &str = r"(\w+\s+){8}\d{9}";

/// Shortest full scan for which the cancel timing means something.
const MIN_FULL_SCAN: Duration = Duration::from_millis(300);

/// Hard cap on any child.
const HARNESS_CAP: Duration = Duration::from_secs(120);

/// How many times the fixture may grow before the harness gives up.
const MAX_GROWTHS: u32 = 4;

/// Ceiling of the fixture, so a fast machine cannot grow it without bound.
const MAX_FIXTURE_BYTES: usize = 2 * 1024 * 1024 * 1024;

/// Append bytes to the fixture, keeping the file a whole number of lines.
fn write_big_file(path: &Path, extra_bytes: usize) {
    let line = "line 1234 lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod\n";
    let mut out = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open big file"),
    );
    let mut written = 0usize;
    while written < extra_bytes {
        out.write_all(line.as_bytes()).expect("write");
        written += line.len();
    }
    out.flush().expect("flush");
}

/// Grow the fixture until a full scan clears `MIN_FULL_SCAN`, within the cap.
///
/// A fixed size cannot promise a measurable scan: the same 160 MB are scanned
/// in 270 ms on a fast host, and then there is no window to place the signal
/// in. Growing keeps the property true on every machine instead of moving the
/// constant until one host happens to pass.
fn scan_long_enough(ws: &Path) -> Duration {
    let path = ws.join("big.txt");
    write_big_file(&path, FILE_BYTES);
    let mut size = FILE_BYTES;
    for _ in 0..=MAX_GROWTHS {
        let (code, full) = run_timed(search_command(ws, "big.txt", "0"));
        assert_eq!(code, Some(1), "no match is NO_MATCHES (exit 1)");
        if full >= MIN_FULL_SCAN {
            return full;
        }
        let grown = size.saturating_mul(2).min(MAX_FIXTURE_BYTES);
        assert!(
            grown > size,
            "the fixture hit {MAX_FIXTURE_BYTES} bytes and a full scan still takes {full:?}"
        );
        write_big_file(&path, grown - size);
        size = grown;
    }
    let (_, full) = run_timed(search_command(ws, "big.txt", "0"));
    assert!(
        full >= MIN_FULL_SCAN,
        "the scan ({full:?}) is too fast to time a cancel inside it"
    );
    full
}

fn search_command(ws: &Path, file: &str, timeout_secs: &str) -> Command {
    let mut cmd = common::hermetic(Command::new(env!("CARGO_BIN_EXE_atomwrite")));
    cmd.args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args([
            "--no-progress",
            "--no-auto-heal",
            "--timeout-secs",
            timeout_secs,
        ])
        .args([
            "search",
            "--max-filesize",
            "4000000000",
            "-e",
            NEVER_MATCHES,
            file,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
}

/// Run to completion, returning the exit code and how long it took.
fn run_timed(mut cmd: Command) -> (Option<i32>, Duration) {
    let started = Instant::now();
    let mut child = cmd.spawn().expect("spawn atomwrite");
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        tx.send(child.wait()).ok();
    });
    let status = match rx.recv_timeout(HARNESS_CAP) {
        Ok(Ok(status)) => status,
        _ => {
            unsafe { libc::kill(pid as i32, libc::SIGKILL) };
            panic!("search still alive after {HARNESS_CAP:?} (killed by the harness)");
        }
    };
    (status.code(), started.elapsed())
}

#[test]
fn search_in_one_large_file_stops_on_sigterm_and_on_the_deadline() {
    let ws = tempfile::tempdir().expect("workspace");
    let full = scan_long_enough(ws.path());

    // SIGTERM a quarter of the way into the scan.
    let mut child = search_command(ws.path(), "big.txt", "0")
        .spawn()
        .expect("spawn atomwrite");
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        tx.send(child.wait()).ok();
    });
    assert!(
        rx.recv_timeout(full / 4).is_err(),
        "the scan ended before the signal"
    );
    let signalled = Instant::now();
    unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    let status = rx
        .recv_timeout(HARNESS_CAP)
        .unwrap_or_else(|_| {
            unsafe { libc::kill(pid as i32, libc::SIGKILL) };
            panic!("search ignored SIGTERM (killed by the harness)")
        })
        .expect("wait");
    let reaction = signalled.elapsed();
    assert_eq!(status.code(), Some(143), "SIGTERM is a clean cancel");
    assert!(
        reaction < full / 2,
        "SIGTERM took {reaction:?} to be seen; a full scan takes {full:?}"
    );

    // The one-second deadline, only when the scan outlasts it by a margin.
    if full >= Duration::from_millis(1600) {
        let (code, elapsed) = run_timed(search_command(ws.path(), "big.txt", "1"));
        assert_eq!(code, Some(124), "the deadline is a cancel (124)");
        assert!(
            elapsed < Duration::from_millis(1350),
            "deadline of 1 s took {elapsed:?}; a full scan takes {full:?}"
        );
    }
}
