// SPDX-License-Identifier: MIT OR Apache-2.0

//! `read` must not materialize the whole file to answer a question about it.
//!
//! The verb streams now: `--stat` counts lines and hashes through `BufRead`,
//! and `--head 1` stops at the first line. The test builds its own 256 MiB
//! fixture, so the ceiling below is measured against a file four times larger
//! than the ceiling itself.
//!
//! Peak RSS is read from `/proc/self`-style accounting: the test spawns the
//! binary under a poller that samples `VmHWM` from `/proc/<pid>/status` until
//! the process exits. That file is Linux-only, so the test is gated to Linux;
//! on macOS and Windows the streaming paths still run, they are just not
//! measured here.

#![cfg(target_os = "linux")]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_atomwrite"))
}

/// Build a text fixture of at least `bytes` bytes, one short line at a time.
///
/// The file is written line by line so the fixture itself never holds the
/// content in memory either.
fn big_fixture(bytes: u64, dir: &Path) -> PathBuf {
    let path = dir.join("big.txt");
    let mut file = std::io::BufWriter::new(fs::File::create(&path).expect("create fixture"));
    let line = b"aaaa bbbb cccc dddd eeee ffff gggg hhhh\n";
    let mut written = 0u64;
    while written < bytes {
        file.write_all(line).expect("write fixture");
        written += line.len() as u64;
    }
    file.flush().expect("flush fixture");
    path
}

/// Peak RSS in kilobytes, sampled from `/proc/<pid>/status` until exit.
///
/// Sampling every 5 ms is fine-grained enough for a run of a few hundred
/// milliseconds; `VmHWM` is the kernel's own high-water mark, so a missed
/// sample cannot lower the number, only fail to see a later one.
fn peak_rss_kb(args: &[&str], cwd: &Path) -> (u64, i32) {
    let mut child = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn atomwrite");
    let status_path = format!("/proc/{}/status", child.id());
    let started = Instant::now();
    let mut peak = 0u64;
    loop {
        if let Ok(text) = fs::read_to_string(&status_path) {
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("VmHWM:") {
                    let kb: u64 = rest
                        .split_whitespace()
                        .next()
                        .and_then(|n| n.parse().ok())
                        .unwrap_or(0);
                    peak = peak.max(kb);
                }
            }
        }
        match child.try_wait().expect("wait child") {
            Some(status) => {
                // One last read after exit is not possible; `VmHWM` was already
                // captured on the previous pass.
                let _ = started;
                return (peak, status.code().unwrap_or(-1));
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

#[test]
fn stat_does_not_materialize_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = big_fixture(256 * 1024 * 1024, dir.path());
    let name = path.to_str().expect("utf8");
    let (peak, code) = peak_rss_kb(&["read", name, "--stat"], dir.path());
    assert_eq!(code, 0, "read --stat must succeed");
    assert!(
        peak < 64 * 1024,
        "read --stat on a 256 MiB file peaked at {peak} KB; the ceiling is 65536 KB"
    );
    drop(path);
}

/// One line, 256 MiB long, with no newline anywhere in it.
///
/// `--stat` still has to publish `lines_total`, and the line-counting pass used
/// to read one whole line at a time, so a file with no newline made it allocate
/// the file size on top of the checksum pass. The count now runs over fixed
/// chunks, and this fixture is the case that proves it.
fn single_line_fixture(bytes: u64, dir: &Path) -> PathBuf {
    let path = dir.join("oneline.txt");
    let mut file = std::io::BufWriter::new(fs::File::create(&path).expect("create fixture"));
    let chunk = vec![b'a'; 64 * 1024];
    let mut written = 0u64;
    while written < bytes {
        let take = (bytes - written).min(chunk.len() as u64) as usize;
        file.write_all(&chunk[..take]).expect("write fixture");
        written += take as u64;
    }
    file.flush().expect("flush fixture");
    path
}

#[test]
fn stat_on_a_single_line_file_does_not_materialize_the_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = single_line_fixture(256 * 1024 * 1024, dir.path());
    let name = path.to_str().expect("utf8");
    let (peak, code) = peak_rss_kb(&["read", name, "--stat"], dir.path());
    assert_eq!(code, 0, "read --stat must succeed");
    assert!(
        peak < 64 * 1024,
        "read --stat on a 256 MiB single-line file peaked at {peak} KB; the ceiling is 65536 KB"
    );
    drop(path);
}

#[test]
fn head_one_does_not_materialize_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = big_fixture(256 * 1024 * 1024, dir.path());
    let name = path.to_str().expect("utf8");
    let (peak, code) = peak_rss_kb(&["read", name, "--head", "1"], dir.path());
    assert_eq!(code, 0, "read --head 1 must succeed");
    assert!(
        peak < 32 * 1024,
        "read --head 1 on a 256 MiB file peaked at {peak} KB; the ceiling is 32768 KB"
    );
    drop(path);
}

#[test]
fn tail_one_does_not_materialize_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = big_fixture(256 * 1024 * 1024, dir.path());
    let name = path.to_str().expect("utf8");
    let (peak, code) = peak_rss_kb(&["read", name, "--tail", "1"], dir.path());
    assert_eq!(code, 0, "read --tail 1 must succeed");
    assert!(
        peak < 32 * 1024,
        "read --tail 1 on a 256 MiB file peaked at {peak} KB; the ceiling is 32768 KB"
    );
    drop(path);
}

/// Build a fixture above the fuzzy content cap, with no needle in it.
///
/// `[fuzzy].max_content_bytes` defaults to 256 MiB, so the file has to clear
/// that by a margin wide enough that rounding never puts it back under.
fn oversize_fixture(dir: &Path) -> PathBuf {
    big_fixture(300 * 1024 * 1024, dir)
}

#[test]
fn edit_missing_needle_above_the_fuzzy_cap_does_not_materialize_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = oversize_fixture(dir.path());
    let name = path.to_str().expect("utf8");
    // `--fuzzy auto` is the default, and above the cap the cascade can never
    // run: the answer is "the needle is not there", which the streaming count
    // already knows. Reading 300 MiB to repeat that cost 300 MB of RSS.
    let (peak, code) = peak_rss_kb(
        &["edit", name, "--old", "XXNOPE", "--new", "Y", "--dry-run"],
        dir.path(),
    );
    assert_eq!(code, 65, "a missing needle still exits 65");
    assert!(
        peak < 64 * 1024,
        "edit on a 300 MiB file with a missing needle peaked at {peak} KB; the ceiling is 65536 KB"
    );
    drop(path);
}

#[test]
fn edit_missing_needle_with_fuzzy_off_above_the_cap_does_not_materialize_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = oversize_fixture(dir.path());
    let name = path.to_str().expect("utf8");
    let (peak, code) = peak_rss_kb(
        &[
            "edit",
            name,
            "--old",
            "XXNOPE",
            "--new",
            "Y",
            "--dry-run",
            "--fuzzy",
            "off",
        ],
        dir.path(),
    );
    assert_eq!(code, 65, "exact-only miss still exits 65");
    assert!(
        peak < 64 * 1024,
        "edit --fuzzy off on a 300 MiB file peaked at {peak} KB; the ceiling is 65536 KB"
    );
    drop(path);
}

#[test]
fn edit_that_finds_the_needle_still_applies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("marked.txt");
    let mut file = fs::File::create(&path).expect("create");
    file.write_all(b"first line\nMARKER_XYZ\n").expect("write");
    for _ in 0..2000 {
        file.write_all(b"padding padding padding\n").expect("write");
    }
    file.flush().expect("flush");
    drop(file);
    let name = path.to_str().expect("utf8");
    let out = Command::new(bin())
        .args(["edit", name, "--old", "MARKER_XYZ", "--new", "REPLACED"])
        .current_dir(dir.path())
        .output()
        .expect("run edit");
    assert!(out.status.success(), "a found needle must apply: {out:?}");
    let after = fs::read_to_string(&path).expect("read back");
    assert!(after.contains("REPLACED"), "the edit must land on disk");
    assert!(!after.contains("MARKER_XYZ"), "the old text must be gone");
}
