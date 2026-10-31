// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 red tests for the `search` streaming contract (R1, R1b, R3, R5, R7).
//!
//! Every test here is expected to FAIL on the current binary for the reason
//! named in its `RED until` comment, and to PASS once the matching fix lands.
//! The deadline fixtures are never a fixed size: each one is sized from a
//! throughput probe of the binary itself, so the full, uncancelled work is
//! guaranteed to be several times longer than the deadline under test.

mod common;

use serde_json::Value;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// One generated line of dense matches (8 matches per line).
const DENSE_LINE: &str = "tok=1 tok=1 tok=1 tok=1 tok=1 tok=1 tok=1 tok=1\n";

/// Pattern the dense fixtures are built from.
const TOKEN: &str = "tok=1";

/// Size of the throughput probe file.
const PROBE_BYTES: usize = 2 * 1024 * 1024;

/// The uncancelled work must be at least this many times the deadline.
const WORK_FACTOR: f64 = 4.0;

/// How much a fixture grows when the host finished the scan inside the deadline.
const GROWTH_FACTOR: u64 = 4;

/// Most growths one deadline may need before the test calls the host too fast.
const MAX_GROWTHS: u32 = 3;

/// Hard ceiling for any generated fixture (1 GiB).
const MAX_FIXTURE_BYTES: u64 = 1024 * 1024 * 1024;

/// Hard cap on how long a child may run before the harness kills it.
const HARD_CAP: Duration = Duration::from_secs(150);

/// How many bytes from the end of stdout are inspected for the last event.
const TAIL_BYTES: u64 = 256 * 1024;

struct Run {
    status: ExitStatus,
    elapsed: Duration,
    stdout_path: PathBuf,
}

fn bin() -> PathBuf {
    assert_cmd::cargo::cargo_bin("atomwrite")
}

fn kill_pid(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
    #[cfg(not(unix))]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
    }
}

/// Run the binary with stdout redirected to `stdout_path`, bounded by `HARD_CAP`.
/// A child that does not finish is killed and the test fails with a message.
fn run_bounded(args: &[OsString], stdout_path: &Path) -> Run {
    let out = File::create(stdout_path).expect("create stdout file");
    let start = Instant::now();
    let mut child = common::hermetic(Command::new(bin()))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn atomwrite");
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait());
    });
    match rx.recv_timeout(HARD_CAP) {
        Ok(Ok(status)) => Run {
            status,
            elapsed: start.elapsed(),
            stdout_path: stdout_path.to_path_buf(),
        },
        Ok(Err(e)) => panic!("wait failed: {e}"),
        Err(_) => {
            kill_pid(pid);
            panic!("child still running after {HARD_CAP:?}; killed by the harness")
        }
    }
}

fn args_of(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(OsString::from).collect()
}

/// Global flags shared by every run, followed by `rest`.
fn search_args(
    ws: &Path,
    timeout_secs: u64,
    global_extra: &[&str],
    rest: &[&str],
) -> Vec<OsString> {
    let mut v = args_of(&[
        "--workspace",
        ws.to_str().expect("utf8 workspace"),
        "--no-progress",
        "--no-auto-heal",
        "--max-filesize",
        "2000000000",
        "--timeout-secs",
        &timeout_secs.to_string(),
    ]);
    v.extend(args_of(global_extra));
    v.extend(args_of(rest));
    v
}

/// Last non-empty line of the file, read from the tail only.
fn last_line(path: &Path) -> String {
    let mut f = File::open(path).expect("open stdout file");
    let len = f.metadata().expect("metadata").len();
    let start = len.saturating_sub(TAIL_BYTES);
    f.seek(SeekFrom::Start(start)).expect("seek");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("read tail");
    let text = String::from_utf8_lossy(&buf);
    text.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .to_string()
}

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

/// Measure the binary's own throughput for this exact command shape on a
/// small probe and return the fixture size whose uncancelled work lasts
/// `WORK_FACTOR` times the deadline. Fails clearly if the ceiling is too low.
fn sized_fixture_bytes(global_extra: &[&str], rest: &[&str], deadline_secs: u64) -> u64 {
    let probe_dir = tempfile::tempdir().expect("probe tempdir");
    write_dense(&probe_dir.path().join("probe.txt"), PROBE_BYTES as u64);
    let out_dir = tempfile::tempdir().expect("probe out");
    let mut rest_v: Vec<&str> = vec!["search"];
    rest_v.extend_from_slice(rest);
    let ws = probe_dir.path().to_str().expect("utf8").to_string();
    rest_v.push(TOKEN);
    rest_v.push(&ws);
    let args = search_args(probe_dir.path(), 600, global_extra, &rest_v);
    let run = run_bounded(&args, &out_dir.path().join("probe.out"));
    assert!(
        run.status.success(),
        "throughput probe failed: {:?}",
        run.status
    );
    let secs = run.elapsed.as_secs_f64().max(0.001);
    let rate = PROBE_BYTES as f64 / secs;
    let need = (rate * deadline_secs as f64 * WORK_FACTOR) as u64;
    let need = need.max(4 * 1024 * 1024);
    // The 2 MiB probe under-reports the streaming rate of `-l`, which walks a
    // 1 GiB file near memory speed. The ceiling is therefore a cap, not a
    // failure: `assert_cancel_contract` re-deadlines against the measured scan
    // of the fixture it actually built, so a capped fixture still exercises
    // cancellation instead of declaring the host too fast.
    need.min(MAX_FIXTURE_BYTES)
}

/// Write a dense fixture of `bytes` bytes into a fresh workspace.
fn dense_workspace(bytes: u64) -> tempfile::TempDir {
    let ws = tempfile::tempdir().expect("workspace");
    write_dense(&ws.path().join("big.txt"), bytes);
    ws
}

/// Wall clock of one uncancelled scan of `ws`, used to derive a deadline the
/// host can actually beat. A probe of a few MiB is not representative: the
/// same binary streams a 1 GiB file far faster than it scans a 2 MiB one, so
/// the sized fixture alone can under-provision and the deadline never bites.
fn full_scan_time(global_extra: &[&str], local_extra: &[&str], ws: &Path) -> Duration {
    let ws_path = ws.to_str().expect("utf8").to_string();
    let mut rest: Vec<&str> = vec!["search"];
    rest.extend_from_slice(local_extra);
    rest.push(TOKEN);
    rest.push(&ws_path);
    // `--timeout-secs 0` disables the deadline: this run must finish by itself.
    let mut args = args_of(&[
        "--workspace",
        ws.to_str().expect("utf8 workspace"),
        "--no-progress",
        "--no-auto-heal",
        "--max-filesize",
        "2000000000",
        "--timeout-secs",
        "0",
    ]);
    args.extend(args_of(global_extra));
    args.extend(args_of(&rest));
    let out_dir = tempfile::tempdir().expect("out dir");
    run_bounded(&args, &out_dir.path().join("full.out")).elapsed
}

/// Deadline this host can actually miss for this workload: the requested one,
/// capped at an eighth of the uncancelled scan. Never below one second, which
/// is the smallest deadline the CLI accepts.
fn effective_deadline(requested: u64, full: Duration) -> u64 {
    let fitted = (full.as_secs_f64() / WORK_FACTOR).floor() as u64;
    requested.min(fitted.max(1))
}

/// Run `search` on a freshly sized dense fixture under `deadline_secs` and
/// check the cancel contract: exit 124, wall clock near the deadline, and the
/// `cancelled` envelope as the LAST stdout line.
///
/// The contract is "the deadline cuts the scan", not "the host runs at the
/// probed speed". A run that finishes on its own before the deadline proves
/// nothing about cancellation (the host outran the probe, which load swings
/// make routine), so it is not a verdict: the fixture grows by `GROWTH_FACTOR`
/// and the run repeats, up to `MAX_GROWTHS` times and `MAX_FIXTURE_BYTES`. Only
/// a run that is still going at the deadline is judged.
fn assert_cancel_contract(global_extra: &[&str], local_extra: &[&str], deadlines: &[u64]) {
    let longest = *deadlines.iter().max().expect("at least one deadline");
    let mut bytes = sized_fixture_bytes(global_extra, local_extra, longest);
    let mut ws = dense_workspace(bytes);
    let out_dir = tempfile::tempdir().expect("out dir");
    // Size the fixture first, then measure the whole scan of THAT fixture, so
    // the deadline is derived from the real workload instead of a probe rate.
    let full = full_scan_time(global_extra, local_extra, ws.path());
    for &requested in deadlines {
        let mut deadline = effective_deadline(requested, full);
        let mut growths = 0;
        let run = loop {
            let wspath = ws.path().to_str().expect("utf8").to_string();
            let mut rest: Vec<&str> = vec!["search"];
            rest.extend_from_slice(local_extra);
            rest.push(TOKEN);
            rest.push(&wspath);
            let args = search_args(ws.path(), deadline, global_extra, &rest);
            let run = run_bounded(&args, &out_dir.path().join(format!("run{deadline}.out")));
            let finished_on_its_own =
                run.status.success() && run.elapsed < Duration::from_secs(deadline);
            if !finished_on_its_own {
                break run;
            }
            assert!(
                growths < MAX_GROWTHS && bytes < MAX_FIXTURE_BYTES,
                "the whole scan of {bytes} B ends inside the {deadline}s deadline \
                 ({:?}) even after {growths} growths: the host is too fast for the ceiling",
                run.elapsed
            );
            growths += 1;
            bytes = (bytes * GROWTH_FACTOR).min(MAX_FIXTURE_BYTES);
            ws = dense_workspace(bytes);
            // Re-derive from the grown fixture: a fast host shortens the
            // deadline instead of failing, so the cancel path is still tested.
            deadline = effective_deadline(
                requested,
                full_scan_time(global_extra, local_extra, ws.path()),
            );
        };
        let limit = Duration::from_secs_f64(deadline as f64 * 1.25 + 0.5);
        assert!(
            run.elapsed <= limit,
            "search overran the {deadline}s deadline: took {:?}, limit {limit:?} \
             (fixture {bytes} B)",
            run.elapsed
        );
        assert_eq!(
            run.status.code(),
            Some(124),
            "expected exit 124 on deadline {deadline}s, got {:?}",
            run.status
        );
        let last = last_line(&run.stdout_path);
        let v: Value = serde_json::from_str(&last)
            .unwrap_or_else(|e| panic!("last stdout line is not JSON ({e}): {last:.200}"));
        assert_eq!(
            v["type"], "cancelled",
            "the LAST stdout line must be the cancelled event, got: {last:.200}"
        );
    }
}

// RED until R1: the match loop must check the deadline inside the file and the
// writer must stop draining the backlog, so exit 124 + `cancelled` arrive within
// the deadline plus slack instead of after the whole file is emitted.
#[test]
fn r1_search_deadline_stops_near_budget() {
    assert_cancel_contract(&[], &[], &[2, 1]);
}

// RED until R1b: with --sort-by the buffered tail must NOT be flushed after the
// cancel; `cancelled` is the last line and the run stops near the deadline.
#[test]
fn r1b_search_sort_by_cancelled_is_last_line() {
    assert_cancel_contract(&["--sort-by", "path-line"], &[], &[2]);
}

// RED until R1b: the files-only mode must honour the deadline inside the file
// and end with `cancelled`, not with a late summary.
#[test]
fn r1b_search_files_only_cancelled_is_last_line() {
    assert_cancel_contract(&[], &["-l"], &[2]);
}

// RED until R3: parallel workers must emit each file as one atomic
// begin..match..end block; today matches land inside another file's block.
#[test]
fn r3_matches_stay_inside_their_own_begin_end_block() {
    const FILES: usize = 40;
    const MATCHES_PER_FILE: usize = 50;
    let ws = tempfile::tempdir().expect("workspace");
    for i in 0..FILES {
        let body = "needle line\n".repeat(MATCHES_PER_FILE);
        std::fs::write(ws.path().join(format!("f{i:02}.txt")), body).expect("write file");
    }
    let out_dir = tempfile::tempdir().expect("out dir");
    let wspath = ws.path().to_str().expect("utf8").to_string();
    let mut misplaced_total = 0usize;
    let mut first_problem = String::new();
    for round in 0..5 {
        let args = search_args(ws.path(), 120, &[], &["search", "needle", &wspath]);
        let run = run_bounded(&args, &out_dir.path().join(format!("r3_{round}.out")));
        assert_eq!(
            run.status.code(),
            Some(0),
            "round {round}: {:?}",
            run.status
        );
        let reader = BufReader::new(File::open(&run.stdout_path).expect("open"));
        let mut current: Option<String> = None;
        let mut matches = 0usize;
        for line in reader.lines() {
            let line = line.expect("line");
            let v: Value = serde_json::from_str(&line).expect("ndjson");
            match v["type"].as_str() {
                Some("begin") => current = v["path"].as_str().map(str::to_string),
                Some("match") => {
                    matches += 1;
                    let path = v["path"].as_str().map(str::to_string);
                    if path != current {
                        misplaced_total += 1;
                        if first_problem.is_empty() {
                            first_problem = format!(
                                "round {round}: match path {path:?} inside begin block {current:?}"
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        assert_eq!(
            matches,
            FILES * MATCHES_PER_FILE,
            "round {round}: wrong total match count"
        );
    }
    assert_eq!(
        misplaced_total, 0,
        "{misplaced_total} matches outside their own begin/end block; first: {first_problem}"
    );
}

// RED until R5: kept as a structural guard for the streaming rewrite; every file
// must produce exactly begin, match..., end, summary and nothing else.
#[test]
fn r5_search_event_structure_without_cancel() {
    const BYTES: usize = 20 * 1024 * 1024;
    const LINE_LEN: usize = 80;
    let ws = tempfile::tempdir().expect("workspace");
    let path = ws.path().join("many.txt");
    let lines = BYTES / LINE_LEN;
    {
        let mut f = std::io::BufWriter::new(File::create(&path).expect("create"));
        for i in 0..lines {
            let l = format!("{i:08} needle {}\n", "x".repeat(LINE_LEN - 17));
            assert_eq!(l.len(), LINE_LEN);
            f.write_all(l.as_bytes()).expect("write");
        }
        f.flush().expect("flush");
    }
    let out_dir = tempfile::tempdir().expect("out dir");
    let wspath = ws.path().to_str().expect("utf8").to_string();
    let args = search_args(ws.path(), 600, &[], &["search", "needle", &wspath]);
    let run = run_bounded(&args, &out_dir.path().join("r5.out"));
    assert_eq!(
        run.status.code(),
        Some(0),
        "unexpected status {:?}",
        run.status
    );
    let reader = BufReader::new(File::open(&run.stdout_path).expect("open"));
    let (mut begins, mut matches, mut ends, mut summaries, mut others) = (0, 0, 0, 0, 0);
    let mut order: Vec<&'static str> = Vec::new();
    let mut end_matches = 0u64;
    for line in reader.lines() {
        let v: Value = serde_json::from_str(&line.expect("line")).expect("ndjson");
        match v["type"].as_str() {
            Some("begin") => {
                begins += 1;
                order.push("begin");
            }
            Some("match") => {
                matches += 1;
                if order.last() != Some(&"match") {
                    order.push("match");
                }
            }
            Some("end") => {
                ends += 1;
                end_matches = v["stats"]["matches"].as_u64().unwrap_or(0);
                order.push("end");
            }
            Some("summary") => {
                summaries += 1;
                order.push("summary");
            }
            _ => others += 1,
        }
    }
    assert_eq!(
        (begins, ends, summaries, others),
        (1, 1, 1, 0),
        "structure counts (begin,end,summary,other)"
    );
    assert_eq!(matches, lines, "one match event per line");
    assert_eq!(end_matches as usize, lines, "end.stats.matches");
    assert_eq!(order, ["begin", "match", "end", "summary"], "event order");
}

fn event_lines(stdout: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("ndjson line"))
        .collect()
}

/// Run one zero-match command and check R7: exit 1, `NO_MATCHES` last, and no
/// line carrying `ok:true` anywhere before it.
fn assert_zero_match_not_ok(label: &str, verb_args: &[&str]) {
    let ws = tempfile::tempdir().expect("workspace");
    std::fs::write(ws.path().join("a.rs"), "fn main() {}\n").expect("fixture");
    let wspath = ws.path().to_str().expect("utf8").to_string();
    let mut args = args_of(&["--workspace", &wspath, "--no-progress", "--no-auto-heal"]);
    args.extend(args_of(verb_args));
    args.push(OsString::from(&wspath));
    let out = common::hermetic(Command::new(bin()))
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .expect("run");
    assert_eq!(
        out.status.code(),
        Some(1),
        "{label}: exit code, status {:?}",
        out.status
    );
    let events = event_lines(&out.stdout);
    let last = events
        .last()
        .unwrap_or_else(|| panic!("{label}: empty stdout"));
    assert_eq!(
        last["code"], "NO_MATCHES",
        "{label}: last event must be NO_MATCHES"
    );
    for e in &events {
        assert!(
            e["ok"] != Value::Bool(true),
            "{label}: zero-match run emitted ok:true in {e}"
        );
    }
    // One terminal event: the summary states the failure, and no second
    // envelope repeats it.
    let failed = events
        .iter()
        .filter(|e| e["ok"] == Value::Bool(false))
        .count();
    assert_eq!(
        failed, 1,
        "{label}: exactly one terminal ok:false event, got {events:?}"
    );
}

// RED until R7: the zero-match summary must carry ok:false (or be dropped),
// never ok:true followed by a NO_MATCHES error.
#[test]
fn r7_search_zero_match_has_no_ok_true() {
    assert_zero_match_not_ok("search", &["search", "zzzz_absent"]);
}

// RED until R7: same for replace --dry-run.
#[test]
fn r7_replace_dry_run_zero_match_has_no_ok_true() {
    assert_zero_match_not_ok("replace", &["replace", "--dry-run", "zzzz_absent", "yyyy"]);
}

// RED until R7: same for transform --dry-run.
#[cfg(feature = "ast")]
#[test]
fn r7_transform_dry_run_zero_match_has_no_ok_true() {
    assert_zero_match_not_ok(
        "transform",
        &[
            "transform",
            "--dry-run",
            "-p",
            "$E.unwrap()",
            "-r",
            "$E?",
            "-l",
            "rust",
        ],
    );
}

// RED until R7: same for scope --dry-run.
#[cfg(feature = "ast")]
#[test]
fn r7_scope_dry_run_zero_match_has_no_ok_true() {
    assert_zero_match_not_ok(
        "scope",
        &[
            "scope",
            "--dry-run",
            "--lang",
            "rust",
            "--query",
            "comments",
        ],
    );
}
