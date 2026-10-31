// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 red test for the `search` peak memory (R5).
//!
//! This file holds exactly ONE test on purpose: `RUSAGE_CHILDREN` accumulates
//! the peak of every child the test process has waited for, so any other test
//! in the same binary would pollute the measurement.
#![cfg(target_os = "linux")]

mod common;

use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// One million matching lines of 50 bytes each (about 50 MiB).
const LINES: usize = 1_000_000;
const LINE_LEN: usize = 50;

/// Peak resident set allowed for the child, in KiB (120 MiB).
const MAX_RSS_KIB: i64 = 120 * 1024;

/// Hard cap on how long the child may run before the harness kills it.
const HARD_CAP: Duration = Duration::from_secs(300);

// RED until R5: `search` must stream matches to the writer with bounded memory
// instead of collecting every event first; today the peak is about 377 MiB.
#[test]
fn r5_search_one_million_matches_peak_rss_under_120_mib() {
    let ws = tempfile::tempdir().expect("workspace");
    let path = ws.path().join("million.txt");
    {
        let mut f =
            std::io::BufWriter::with_capacity(1 << 20, File::create(&path).expect("create"));
        for i in 0..LINES {
            let l = format!("{i:07} needle {}\n", "x".repeat(LINE_LEN - 16));
            debug_assert_eq!(l.len(), LINE_LEN);
            f.write_all(l.as_bytes()).expect("write fixture");
        }
        f.flush().expect("flush fixture");
    }
    let wspath = ws.path().to_str().expect("utf8").to_string();
    let mut child = common::hermetic(Command::new(assert_cmd::cargo::cargo_bin("atomwrite")))
        .args([
            "--workspace",
            &wspath,
            "--no-progress",
            "--no-auto-heal",
            "--max-filesize",
            "2000000000",
            "--timeout-secs",
            "600",
            "search",
            "needle",
            &wspath,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn atomwrite");
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait());
    });
    let status = match rx.recv_timeout(HARD_CAP) {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => panic!("wait failed: {e}"),
        Err(_) => {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
            let _ = rx.recv_timeout(Duration::from_secs(10));
            panic!("search still running after {HARD_CAP:?}; killed by the harness");
        }
    };
    assert_eq!(
        status.code(),
        Some(0),
        "search must finish cleanly: {status:?}"
    );

    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage) };
    assert_eq!(rc, 0, "getrusage failed");
    let peak_kib = usage.ru_maxrss;
    assert!(
        peak_kib < MAX_RSS_KIB,
        "search peak RSS {} MiB exceeds the {} MiB budget for {LINES} matches",
        peak_kib / 1024,
        MAX_RSS_KIB / 1024
    );
}
