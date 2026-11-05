// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 red tests for safety and robustness gaps (R17, R18, R19, R20, R22,
//! R28, R29). R27 (stdin buffer proportional to `--max-size`) is deliberately
//! absent: the bug does not reproduce, see the report of this phase.

mod common;

use assert_cmd::Command;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

/// Per-process bound so a hung verb fails the test instead of the suite.
const PROC_TIMEOUT: Duration = Duration::from_secs(60);

struct Out {
    code: Option<i32>,
    #[cfg(unix)]
    signal: Option<i32>,
    stdout: String,
    events: Vec<Value>,
}

impl Out {
    fn last(&self) -> Option<&Value> {
        self.events.last()
    }
}

fn run_args<S: AsRef<std::ffi::OsStr>>(ws: &Path, args: &[S], stdin: &str) -> Out {
    let mut cmd = common::hermetic(Command::cargo_bin("atomwrite").expect("binary"));
    cmd.timeout(PROC_TIMEOUT)
        .args([
            "--workspace",
            ws.to_str().expect("utf8 workspace"),
            "--no-progress",
            "--no-auto-heal",
        ])
        .args(args)
        .write_stdin(stdin.as_bytes().to_vec());
    let output = cmd.output().expect("run atomwrite");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let events = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        output.status.signal()
    };
    Out {
        code: output.status.code(),
        #[cfg(unix)]
        signal,
        stdout,
        events,
    }
}

fn path_str(p: &Path) -> String {
    p.to_str().expect("utf8 path").to_string()
}

fn dir_entries(p: &Path) -> Vec<String> {
    std::fs::read_dir(p)
        .expect("read_dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}

// RED until R17: `--index-dir` must go through the workspace jail: a directory
// outside the workspace is refused with exit 126 and nothing is created there.
#[test]
fn r17_semantic_search_index_dir_outside_workspace_is_jailed() {
    let ws = tempfile::tempdir().expect("workspace");
    let outside = tempfile::tempdir().expect("outside dir");
    std::fs::write(ws.path().join("a.txt"), "hello x world\n").expect("fixture");
    let out = run_args(
        ws.path(),
        &[
            "semantic-search".to_string(),
            "hello world".to_string(),
            path_str(ws.path()),
            "--index-dir".to_string(),
            path_str(outside.path()),
        ],
        "",
    );
    let leaked = dir_entries(outside.path());
    assert!(
        leaked.is_empty(),
        "files were created outside the workspace: {leaked:?}"
    );
    assert_eq!(
        out.code,
        Some(126),
        "expected WORKSPACE_JAIL exit 126, stdout: {:.300}",
        out.stdout
    );
    assert_eq!(
        out.last().expect("error envelope")["code"],
        "WORKSPACE_JAIL"
    );
}

// RED until R18: a non-UTF-8 argv entry must produce a structured error, never
// a panic/abort (exit 101 / 134 / killed by signal) with an empty stdout.
#[cfg(unix)]
#[test]
fn r18_non_utf8_argument_is_a_structured_error() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let ws = tempfile::tempdir().expect("workspace");
    let bad = OsStr::from_bytes(b"\xff\xfe.txt");
    let out = run_args(ws.path(), &[OsStr::new("hash"), bad], "");
    assert_eq!(
        out.signal, None,
        "the process died by signal {:?}",
        out.signal
    );
    assert!(
        out.code != Some(134) && out.code != Some(101),
        "the process panicked/aborted with exit {:?}",
        out.code
    );
    let last = out
        .last()
        .unwrap_or_else(|| panic!("empty stdout, exit {:?}", out.code));
    assert!(
        last["ok"] == false,
        "expected an error envelope on stdout, got {last}"
    );
}

#[cfg(target_os = "linux")]
fn nfd_name() -> String {
    "cafe\u{0301}.txt".to_string()
}

// RED until R19: a file whose on-disk name is NFD must be found when the same
// NFD name is passed; today the path is rewritten to NFC and FILE_NOT_FOUND.
#[cfg(target_os = "linux")]
#[test]
fn r19_nfd_filename_is_found_verbatim() {
    let ws = tempfile::tempdir().expect("workspace");
    let name = nfd_name();
    std::fs::write(ws.path().join(&name), "x\n").expect("write NFD file");
    let arg = path_str(&ws.path().join(&name));
    let out = run_args(ws.path(), &["read".to_string(), arg], "");
    assert_eq!(
        out.code,
        Some(0),
        "NFD file must be readable by its own name, stdout: {:.300}",
        out.stdout
    );
}

// Control (must PASS today): an NFC name passed as NFC still works.
#[cfg(target_os = "linux")]
#[test]
fn r19_nfc_filename_is_found_verbatim() {
    let ws = tempfile::tempdir().expect("workspace");
    let name = "caf\u{00e9}.txt";
    std::fs::write(ws.path().join(name), "x\n").expect("write NFC file");
    let arg = path_str(&ws.path().join(name));
    let out = run_args(ws.path(), &["read".to_string(), arg], "");
    assert_eq!(
        out.code,
        Some(0),
        "NFC file must be readable, stdout: {:.300}",
        out.stdout
    );
}

fn multi_edit_case(ops: &str) {
    let ws = tempfile::tempdir().expect("workspace");
    let target = ws.path().join("e.txt");
    let original = "l1\nl2\nl3\nl4\nl5\n";
    std::fs::write(&target, original).expect("fixture");
    let out = run_args(
        ws.path(),
        &["edit".to_string(), "--multi".to_string(), path_str(&target)],
        ops,
    );
    #[cfg(unix)]
    assert_eq!(
        out.signal, None,
        "the process died by signal {:?}",
        out.signal
    );
    assert!(
        out.code != Some(134) && out.code != Some(101),
        "the process panicked/aborted with exit {:?}",
        out.code
    );
    assert_eq!(
        out.code,
        Some(65),
        "overlapping ops must be INVALID_INPUT, stdout: {:.300}",
        out.stdout
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("read target"),
        original,
        "the file must stay intact when the batch is rejected"
    );
    let last = out.last().expect("error envelope");
    assert_eq!(last["ok"], Value::Bool(false), "last event: {last}");
}

// RED until R20: two range operations that partially overlap are rejected
// (exit 65) instead of silently producing a mangled file.
#[test]
fn r20_edit_multi_partially_overlapping_ranges_are_rejected() {
    multi_edit_case(
        "{\"op\":\"replace-range\",\"start\":1,\"end\":3,\"content\":\"X\\n\"}\n\
         {\"op\":\"replace-range\",\"start\":2,\"end\":4,\"content\":\"Y\\n\"}\n",
    );
}

// RED until R20: nested ranges make the current code index out of bounds and
// panic (exit 101, SIGABRT in release); they must be rejected with exit 65.
#[test]
fn r20_edit_multi_nested_ranges_are_rejected_without_abort() {
    multi_edit_case(
        "{\"op\":\"delete-range\",\"start\":1,\"end\":5}\n\
         {\"op\":\"delete-range\",\"start\":2,\"end\":3}\n",
    );
}

// RED until R28: a match on a line with invalid UTF-8 must keep its text
// (lossy), not an empty string.
#[test]
fn r28_search_invalid_utf8_line_keeps_lossy_text() {
    let ws = tempfile::tempdir().expect("workspace");
    std::fs::write(
        ws.path().join("bad.txt"),
        b"good needle line\nabc \xff\xfe needle tail\nlast\n",
    )
    .expect("fixture");
    let out = run_args(
        ws.path(),
        &[
            "search".to_string(),
            "needle".to_string(),
            path_str(ws.path()),
        ],
        "",
    );
    assert_eq!(out.code, Some(0), "stdout: {:.300}", out.stdout);
    let bad = out
        .events
        .iter()
        .find(|e| e["type"] == "match" && e["line_number"] == 2)
        .unwrap_or_else(|| panic!("no match event for line 2: {:.400}", out.stdout));
    let text = bad["lines"].as_str().unwrap_or("");
    assert!(
        text.contains("needle tail") && text.contains("abc"),
        "the invalid-UTF-8 line must be preserved lossily, got {text:?}"
    );
}

// RED until R29: a file above the default 10 MiB search limit must be reported
// as skipped in the structured output (contract assumed: `files_skipped >= 1`
// on the summary), not hidden behind a bare NO_MATCHES.
#[test]
fn r29_search_reports_files_skipped_above_default_limit() {
    use std::io::Write;
    let ws = tempfile::tempdir().expect("workspace");
    {
        let mut f = std::io::BufWriter::new(
            std::fs::File::create(ws.path().join("big.txt")).expect("create"),
        );
        let chunk = vec![b'a'; 1 << 20];
        for _ in 0..11 {
            f.write_all(&chunk).expect("write chunk");
        }
        f.write_all(b"\nneedle here\n").expect("write tail");
        f.flush().expect("flush");
    }
    let out = run_args(
        ws.path(),
        &[
            "search".to_string(),
            "needle".to_string(),
            path_str(ws.path()),
        ],
        "",
    );
    let code = out
        .events
        .iter()
        .find_map(|e| e["code"].as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "ALL_SKIPPED_BINARY",
        "the oversized file must be reported as skipped, stdout: {:.400}",
        out.stdout
    );
    assert_eq!(out.code, Some(89), "stdout: {:.400}", out.stdout);
}

#[cfg(unix)]
mod sighup {
    use serde_json::Value;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::path::{Path, PathBuf};
    use std::process::{Command, ExitStatus, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    const DENSE_LINE: &str = "tok=1 tok=1 tok=1 tok=1 tok=1 tok=1 tok=1 tok=1\n";
    const PROBE_BYTES: usize = 2 * 1024 * 1024;
    const MAX_FIXTURE_BYTES: u64 = 1024 * 1024 * 1024;
    const HARD_CAP: Duration = Duration::from_secs(120);
    const READY_CAP: Duration = Duration::from_secs(30);
    const STEP: Duration = Duration::from_millis(5);

    fn write_dense(path: &Path, bytes: u64) {
        let mut f = std::io::BufWriter::with_capacity(1 << 20, File::create(path).expect("create"));
        let chunk = DENSE_LINE.repeat(20_000);
        let mut written = 0u64;
        while written < bytes {
            f.write_all(chunk.as_bytes()).expect("write fixture");
            written += chunk.len() as u64;
        }
        f.flush().expect("flush fixture");
    }

    fn base_args(ws: &Path) -> Vec<String> {
        vec![
            "--workspace".into(),
            ws.to_str().expect("utf8").into(),
            "--no-progress".into(),
            "--no-auto-heal".into(),
            "--max-filesize".into(),
            "2000000000".into(),
            "--timeout-secs".into(),
            "600".into(),
        ]
    }

    /// Spawn with stdout in `out`, bounded wait through a thread + channel.
    fn spawn(
        args: &[String],
        out: &Path,
    ) -> (u32, mpsc::Receiver<std::io::Result<ExitStatus>>, Instant) {
        let file = File::create(out).expect("stdout file");
        let started = Instant::now();
        let mut child =
            crate::common::hermetic(Command::new(assert_cmd::cargo::cargo_bin("atomwrite")))
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::from(file))
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn atomwrite");
        let pid = child.id();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(child.wait());
        });
        (pid, rx, started)
    }

    fn kill9(pid: u32, rx: &mpsc::Receiver<std::io::Result<ExitStatus>>) {
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
        let _ = rx.recv_timeout(Duration::from_secs(10));
    }

    /// Fixture size whose uncancelled search lasts about four seconds, measured
    /// from the binary's own throughput on a small probe.
    fn sized_bytes() -> u64 {
        let dir = tempfile::tempdir().expect("probe dir");
        write_dense(&dir.path().join("p.txt"), PROBE_BYTES as u64);
        let out = dir.path().join("probe.out");
        let mut args = base_args(dir.path());
        args.extend([
            "search".into(),
            "tok=1".into(),
            dir.path().to_str().expect("utf8").into(),
        ]);
        let (pid, rx, started) = spawn(&args, &out);
        match rx.recv_timeout(HARD_CAP) {
            Ok(Ok(status)) => assert!(status.success(), "probe failed: {status:?}"),
            _ => {
                kill9(pid, &rx);
                panic!("throughput probe did not finish");
            }
        }
        let rate = PROBE_BYTES as f64 / started.elapsed().as_secs_f64().max(0.001);
        let need = ((rate * 4.0) as u64).max(4 * 1024 * 1024);
        assert!(
            need <= MAX_FIXTURE_BYTES,
            "fixture ceiling too low: need {need} B"
        );
        need
    }

    fn last_line(path: &PathBuf) -> String {
        let mut f = File::open(path).expect("open stdout");
        let len = f.metadata().expect("meta").len();
        f.seek(SeekFrom::Start(len.saturating_sub(256 * 1024)))
            .expect("seek");
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).expect("read");
        String::from_utf8_lossy(&buf)
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .to_string()
    }

    // RED until R22: SIGHUP must be handled like SIGTERM: exit 129 (128+1) with
    // a `cancelled` envelope carrying signal "SIGHUP"; today the process is killed
    // by the default action and nothing is emitted.
    #[test]
    fn r22_search_sighup_exits_cleanly_with_cancelled_envelope() {
        use std::os::unix::process::ExitStatusExt;
        let bytes = sized_bytes();
        let ws = tempfile::tempdir().expect("workspace");
        write_dense(&ws.path().join("big.txt"), bytes);
        let out_dir = tempfile::tempdir().expect("out dir");
        let ready = out_dir.path().join("ready.pid");
        let stdout_path = out_dir.path().join("stdout.ndjson");
        let mut args = base_args(ws.path());
        args.extend([
            "--ready-file".into(),
            ready.to_str().expect("utf8").into(),
            "search".into(),
            "tok=1".into(),
            ws.path().to_str().expect("utf8").into(),
        ]);
        let (pid, rx, _started) = spawn(&args, &stdout_path);
        let wait_start = Instant::now();
        let (_never_tx, never_rx) = mpsc::channel::<()>();
        while !ready.exists() {
            if let Ok(early) = rx.try_recv() {
                panic!("child exited before it became ready: {early:?}");
            }
            if wait_start.elapsed() > READY_CAP {
                kill9(pid, &rx);
                panic!("ready-file never appeared within {READY_CAP:?}");
            }
            let _ = never_rx.recv_timeout(STEP);
        }
        unsafe {
            libc::kill(pid as i32, libc::SIGHUP);
        }
        let sent = Instant::now();
        let status = match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(status)) => status,
            _ => {
                kill9(pid, &rx);
                panic!("search still running 10s after SIGHUP (killed by the harness)");
            }
        };
        assert_eq!(
            status.signal(),
            None,
            "killed by signal {:?} instead of a clean exit",
            status.signal()
        );
        assert!(
            matches!(status.code(), Some(129 | 130 | 143)),
            "expected a documented cancel exit code (129 for SIGHUP), got {:?} after {:?}",
            status.code(),
            sent.elapsed()
        );
        let last = last_line(&stdout_path);
        let v: Value = serde_json::from_str(&last)
            .unwrap_or_else(|e| panic!("last stdout line is not JSON ({e}): {last:.200}"));
        assert_eq!(v["type"], "cancelled", "last line: {last:.200}");
        assert_eq!(v["signal"], "SIGHUP", "last line: {last:.200}");
    }
}
