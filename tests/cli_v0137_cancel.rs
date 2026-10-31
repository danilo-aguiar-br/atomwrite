// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 — the global `--timeout-secs` deadline must interrupt the scan
//! INSIDE a file, not only on the file boundary (A1/A2/A3).
//!
//! Before v0.1.36 the `replace` path bridged to
//! `Regex::replace_all` / `Regex::replacen`, a single synchronous call, so the
//! only cooperative check points sat between files. One large file therefore
//! ran to completion and the summary still reported a full pass — measured at
//! `elapsed_ms: 3357` against a 2000 ms budget, with
//! `total_replacements: 14035087` and no mention of the cancel.
//!
//! These tests pin the contracts the fix must keep true:
//! - the process stops near the deadline instead of overrunning it;
//! - a cancelled run emits the `cancelled` envelope and NO partial summary, so
//!   a consumer reading only the last NDJSON line cannot mistake it for a
//!   completed run;
//! - the data channel (stdout) stays 100% parseable NDJSON.
//!
//! The deadline is controlled exclusively by the binary's own `--timeout-secs`.
//! No `sleep`, no wall-clock polling and no signal from the test harness.
//!
//! # Sizing the fixture
//!
//! A fixed fixture size is a race against the machine: the same bytes take 0.6 s
//! on an idle host and 2.2 s on a loaded one, so a size that always outlasts the
//! budget on one host finishes before the deadline on another and the run ends
//! with no cancel to observe. The fixture is therefore sized from the binary's
//! own throughput, measured once per verb on a small probe:
//! `bytes = throughput x budget x FIXTURE_BUDGET_FACTOR`, bounded by a floor and
//! a cap. If a run still completes before its deadline, the fixture is grown and
//! the run repeated, so a pass always means a cancel was actually observed.
//! The time assertions are proportional to the budget plus a measured startup
//! cost, never an absolute wall-clock value.

mod common;

use assert_cmd::cargo::cargo_bin;
use predicates::prelude::*;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Timeout budget in seconds for the `replace` and `search` runs.
const TIMEOUT_SECS: u64 = 2;

/// Timeout budget in seconds for the `count` run, the shortest the integer
/// `--timeout-secs` flag allows.
const COUNT_TIMEOUT_SECS: u64 = 1;

/// Ceiling on how long the signalled run may take to produce its first byte.
///
/// The wait is for the child's own output, not for a clock: the signal is sent
/// once the data channel is live, so the cancel is guaranteed to land while the
/// walk is running. This bound exists only so a run that never starts fails the
/// test instead of hanging it.
const SIGNAL_START_TIMEOUT_SECS: u64 = 30;

/// Fixed slack, in milliseconds, added on top of `budget x CEILING_NUMERATOR /
/// CEILING_DENOMINATOR`.
///
/// The check point is one `Acquire` load per block of matches, so the overshoot
/// is bounded by the time to scan one block plus the cost of joining the walker
/// threads. The measured overshoot is single-digit milliseconds.
const SLACK_MS: u64 = 1000;

/// Largest startup cost, in milliseconds, the ceiling forgives. Without a bound a
/// probe taken under heavy contention would widen the ceiling until a real
/// regression fit under it.
const STARTUP_ALLOWANCE_CAP_MS: u64 = 500;

/// Numerator of the budget multiplier in the time ceiling (1.5x).
const CEILING_NUMERATOR: u64 = 3;

/// Denominator of the budget multiplier in the time ceiling (1.5x).
const CEILING_DENOMINATOR: u64 = 2;

/// How many budgets of work the fixture must hold at the measured throughput.
///
/// The ceiling is 1.5 budgets plus slack, so a regression to "run the whole
/// file first" (3 budgets of work) lands above it while a healthy cancel lands
/// well below.
const FIXTURE_BUDGET_FACTOR: u64 = 3;

/// Smallest fixture, in bytes, whatever the probe says.
const FIXTURE_FLOOR: u64 = 8 * 1024 * 1024;

/// Largest fixture, in bytes of content the verb has to scan. It is virtual:
/// beyond `FILE_MAX` the content is hard links, so the cap costs directory
/// entries, not disk. `count` reads a warm cached file at several GB/s across
/// all cores, so the cap must be well above what one second of that covers.
const FIXTURE_CAP: u64 = 64 * 1024 * 1024 * 1024;

/// Largest single file written to disk. The temp directory can be RAM-backed and
/// quota-limited, so a fixture beyond this size is the same file hard-linked
/// under more names: the verb scans every link, the disk holds one copy.
const FILE_MAX: u64 = 128 * 1024 * 1024;

/// Times a run that finished before its deadline grows the fixture and retries.
const MAX_ATTEMPTS: usize = 3;

/// Fixture growth per retry.
const GROWTH_FACTOR: u64 = 4;

/// Probe repetitions; the fastest sample is kept, which yields the highest
/// throughput and therefore the largest (safest) fixture.
const PROBE_SAMPLES: usize = 2;

/// Budget for probe runs, far beyond what a probe needs, so a probe never
/// cancels itself.
const PROBE_TIMEOUT_SECS: u64 = 120;

/// Value for the global `--max-filesize`, above any fixture this file writes.
///
/// The fixtures exceed the default per-verb size caps, so without a raised cap
/// the walk visits nothing, finishes immediately and never observes the
/// deadline these tests exist to check.
const MAX_FILESIZE: &str = "4294967296";

/// Fixture size for the signal test.
///
/// The whole file is scanned in memory, so the fixture sets the test's cost:
/// 64 MiB measured over three minutes of scanning and hundreds of thousands of
/// match events, all of which the reader thread drains. The signal lands within
/// the first few thousand events regardless of size, so the smaller fixture
/// proves the same contract without paying for the scan.
const SIGNAL_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// Search pattern the fixture is built from, so a match is guaranteed once a
/// walk reads the file.
const TOKEN: &str = "tok=1";

/// Pattern absent from the fixture, so `--fuzzy auto` runs the whole cascade and
/// ends in a miss (the A1 shape: the cascade, not the exact scan, outlasts the budget).
const MISS_TOKEN: &str = "XXNOPE";

/// File name `write_fixture_of` creates; the single-file `edit` verb targets it.
const FIXTURE_NAME: &str = "oversized.txt";

/// Calibration cache slots: one per distinct scan cost (the two fuzzy-miss verbs share one).
const CALIBRATION_SLOTS: usize = 4;

/// One line of the generated payload: a stable token repeated.
const LINE: &str = "tok=1 tok=1 tok=1 tok=1 tok=1 tok=1 tok=1 tok=1\n";

/// A verb whose scan can be driven past the global deadline.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Verb {
    Replace,
    Search,
    Count,
    /// `replace --dry-run --fuzzy auto` with a pattern that never matches.
    ReplaceFuzzyMiss,
    /// `edit --old` with a pattern that never matches, on one file.
    EditFuzzyMiss,
}

impl Verb {
    /// Position in the per-verb calibration cache.
    fn slot(self) -> usize {
        match self {
            Verb::Replace => 0,
            Verb::Search => 1,
            Verb::Count => 2,
            Verb::ReplaceFuzzyMiss | Verb::EditFuzzyMiss => 3,
        }
    }

    /// Probe fixture size, in bytes. `count` is two orders of magnitude faster
    /// than the other verbs, so a small probe would be lost in the startup cost.
    fn probe_bytes(self) -> u64 {
        match self {
            Verb::Count => 256 * 1024 * 1024,
            Verb::Replace | Verb::Search | Verb::ReplaceFuzzyMiss | Verb::EditFuzzyMiss => {
                16 * 1024 * 1024
            }
        }
    }

    /// Deadline this verb is run with, in seconds.
    fn budget_secs(self) -> u64 {
        match self {
            Verb::Count | Verb::EditFuzzyMiss => COUNT_TIMEOUT_SECS,
            Verb::Replace | Verb::Search | Verb::ReplaceFuzzyMiss => TIMEOUT_SECS,
        }
    }

    /// The subcommand and its arguments, scanning the workspace directory.
    fn command(self, ws: &str) -> Vec<String> {
        match self {
            Verb::Replace => ["replace", "--dry-run", TOKEN, "tok=2", ws]
                .map(String::from)
                .to_vec(),
            Verb::Search => ["search", TOKEN, ws].map(String::from).to_vec(),
            Verb::Count => ["count", ws].map(String::from).to_vec(),
            Verb::ReplaceFuzzyMiss => [
                "replace",
                "--dry-run",
                "--fuzzy",
                "auto",
                MISS_TOKEN,
                "x",
                ws,
            ]
            .map(String::from)
            .to_vec(),
            Verb::EditFuzzyMiss => vec![
                "edit".into(),
                format!("{ws}/{FIXTURE_NAME}"),
                "--old".into(),
                MISS_TOKEN.into(),
                "--new".into(),
                "x".into(),
            ],
        }
    }
}

impl std::fmt::Display for Verb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Verb::Replace => "replace",
            Verb::Search => "search",
            Verb::Count => "count",
            Verb::ReplaceFuzzyMiss => "replace-fuzzy-miss",
            Verb::EditFuzzyMiss => "edit-fuzzy-miss",
        };
        f.write_str(name)
    }
}

/// What the probe learned about the binary on this machine, for one verb.
#[derive(Clone, Copy)]
struct Calibration {
    /// Scan throughput, in bytes per millisecond, net of process startup.
    bytes_per_ms: u64,
    /// Cost of starting the process and walking a one-line fixture, in ms.
    base_ms: u64,
}

/// One timed run against a fixture that outlasted its budget.
struct Run {
    /// Keeps the fixture directory alive for the life of the run.
    _dir: tempfile::TempDir,
    output: Output,
    elapsed_ms: u64,
    budget_ms: u64,
    base_ms: u64,
    fixture_bytes: u64,
}

impl Run {
    /// Latest acceptable elapsed time: the budget scaled by 1.5, plus fixed
    /// slack, plus twice the measured startup cost (bounded by
    /// `STARTUP_ALLOWANCE_CAP_MS`) so a loaded host that starts processes slowly
    /// is not reported as a missed deadline.
    fn ceiling_ms(&self) -> u64 {
        let startup = self.base_ms.min(STARTUP_ALLOWANCE_CAP_MS);
        self.budget_ms * CEILING_NUMERATOR / CEILING_DENOMINATOR + SLACK_MS + 2 * startup
    }
}

/// Write a fixture of at least `target` bytes of scannable content, streamed so
/// a large fixture does not need that much heap. Content beyond `FILE_MAX` is
/// the same file hard-linked under more names.
fn write_fixture_of(dir: &Path, target: u64) -> PathBuf {
    use std::io::Write as _;

    let path = dir.join(FIXTURE_NAME);
    let links = target.div_ceil(FILE_MAX).max(1) - 1;
    let target = target.min(FILE_MAX);
    let chunk = LINE.repeat((1024 * 1024) / LINE.len());
    let mut writer =
        std::io::BufWriter::new(std::fs::File::create(&path).expect("create oversized fixture"));
    let mut written = 0;
    while written < target {
        writer
            .write_all(chunk.as_bytes())
            .expect("write oversized fixture");
        written += chunk.len() as u64;
    }
    writer.flush().expect("flush oversized fixture");
    drop(writer);
    for n in 1..=links {
        let link = dir.join(format!("oversized-{n}.txt"));
        let _ = std::fs::remove_file(&link);
        std::fs::hard_link(&path, &link).expect("hard-link oversized fixture");
    }
    path
}

/// Parse stdout as NDJSON, returning the events and any unparseable line.
fn parse_strict(stdout: &[u8]) -> (Vec<Value>, Vec<String>) {
    let mut events = Vec::new();
    let mut bad = Vec::new();
    let text = String::from_utf8_lossy(stdout);
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<Value>(line) {
            Ok(v) => events.push(v),
            Err(_) => bad.push(line.to_string()),
        }
    }
    (events, bad)
}

/// Locate the terminal `summary` event, if the command emitted one.
fn summary_event(events: &[Value]) -> Option<&Value> {
    events.iter().find(|e| e["type"] == "summary")
}

/// Run `verb` over the fixture in `dir` under `timeout_secs`, returning the raw
/// output and the elapsed time in microseconds.
fn run_verb(verb: Verb, dir: &Path, timeout_secs: u64) -> (Output, u64) {
    let ws = dir.to_str().expect("utf8 workspace");
    let started = Instant::now();
    let output = common::hermetic(std::process::Command::new(cargo_bin("atomwrite")))
        .args([
            "--timeout-secs",
            &timeout_secs.to_string(),
            "--no-auto-heal",
            "--no-progress",
            "--max-filesize",
            MAX_FILESIZE,
            "--workspace",
            ws,
        ])
        .args(verb.command(ws))
        .output()
        .expect("run verb");
    let elapsed_us = started.elapsed().as_micros() as u64;
    (output, elapsed_us)
}

/// Fastest of `PROBE_SAMPLES` runs of `verb`, in microseconds.
fn fastest_run_us(verb: Verb, dir: &Path) -> u64 {
    (0..PROBE_SAMPLES)
        .map(|_| run_verb(verb, dir, PROBE_TIMEOUT_SECS).1)
        .min()
        .expect("at least one probe sample")
}

/// Measure the binary's throughput for `verb`: the time of a one-line fixture is
/// the startup cost, and the extra time of the probe fixture is the scan.
fn measure(verb: Verb) -> Calibration {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixture_of(dir.path(), LINE.len() as u64);
    let base_us = fastest_run_us(verb, dir.path());
    write_fixture_of(dir.path(), verb.probe_bytes());
    let probe_us = fastest_run_us(verb, dir.path());
    let scan_us = probe_us.saturating_sub(base_us).max(1);
    Calibration {
        bytes_per_ms: (verb.probe_bytes() * 1000 / scan_us).max(1),
        base_ms: base_us / 1000,
    }
}

/// Per-verb calibration, measured on first use and shared by every test.
fn calibration(verb: Verb) -> Calibration {
    static CACHE: [OnceLock<Calibration>; CALIBRATION_SLOTS] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    *CACHE[verb.slot()].get_or_init(|| measure(verb))
}

/// Run `verb` against a fixture sized to outlast its deadline.
///
/// A run that finished before its deadline is retried with a bigger fixture, so
/// the run returned here is one that either was cancelled or overran its
/// deadline. The overrun is NOT retried: that is the defect these tests exist to
/// catch, and it is returned for the assertions to fail on.
///
/// # Panics
///
/// When even a `FIXTURE_CAP` fixture finishes inside the budget. Passing there
/// would assert on a run that never observed a cancel, so it fails loudly and
/// says to raise the cap.
fn timed_run(verb: Verb) -> Run {
    let cal = calibration(verb);
    let budget_ms = verb.budget_secs() * 1000;
    let mut bytes = cal
        .bytes_per_ms
        .saturating_mul(budget_ms * FIXTURE_BUDGET_FACTOR)
        .clamp(FIXTURE_FLOOR, FIXTURE_CAP);

    for _ in 0..MAX_ATTEMPTS {
        let dir = tempfile::tempdir().expect("tempdir");
        write_fixture_of(dir.path(), bytes);
        let (output, elapsed_us) = run_verb(verb, dir.path(), verb.budget_secs());
        let elapsed_ms = elapsed_us / 1000;
        eprintln!(
            "{verb}: fixture {bytes} bytes, {} bytes/ms probed, startup {} ms, elapsed {elapsed_ms} ms of {budget_ms} ms",
            cal.bytes_per_ms, cal.base_ms
        );

        let (events, _) = parse_strict(&output.stdout);
        let observed_cancel = events.iter().any(|e| e["type"] == "cancelled");
        // A fuzzy miss that beats the deadline exits 1 (replace) or 65 (edit), not 0.
        let finished_on_its_own = matches!(output.status.code(), Some(0 | 1 | 65))
            && !observed_cancel
            && elapsed_ms < budget_ms;
        if !finished_on_its_own {
            return Run {
                _dir: dir,
                output,
                elapsed_ms,
                budget_ms,
                base_ms: cal.base_ms,
                fixture_bytes: bytes,
            };
        }
        assert!(
            bytes < FIXTURE_CAP,
            "{verb}: a {bytes}-byte fixture finished in {elapsed_ms} ms, inside the \
             {budget_ms} ms budget; this host outruns FIXTURE_CAP, so no cancel can be \
             observed. Raise FIXTURE_CAP"
        );
        bytes = bytes.saturating_mul(GROWTH_FACTOR).min(FIXTURE_CAP);
    }
    panic!(
        "{verb}: {MAX_ATTEMPTS} fixtures finished inside the {budget_ms} ms budget, so no \
         cancel was observed"
    );
}

/// A1/A2 — the deadline stops the process close to the budget, not after the
/// whole file. Measures against the budget, not against a fixed wall clock.
#[test]
fn replace_honours_global_deadline_inside_file() {
    let run = timed_run(Verb::Replace);

    // The budget plus slack must bound the whole process, not just the scan.
    let ceiling = run.ceiling_ms();
    assert!(
        run.elapsed_ms < ceiling,
        "process took {} ms, above the {ceiling} ms ceiling (budget {} ms scaled \
         by 1.5 + {SLACK_MS} ms slack + 2 x {} ms startup, fixture {} bytes); the \
         deadline did not interrupt the scan inside the file",
        run.elapsed_ms,
        run.budget_ms,
        run.base_ms,
        run.fixture_bytes
    );

    // Timeout is the documented Unix convention for an elapsed deadline.
    let code = run.output.status.code();
    assert_eq!(
        code,
        Some(124),
        "expected exit 124 for the elapsed deadline, got {code:?}; stderr: {}",
        String::from_utf8_lossy(&run.output.stderr)
    );
}

/// A2 — nothing on the data channel may be unparseable, in success or timeout.
#[test]
fn replace_data_channel_is_pure_ndjson_on_timeout() {
    let run = timed_run(Verb::Replace);
    let (events, bad) = parse_strict(&run.output.stdout);

    assert!(
        bad.is_empty(),
        "data channel carried {} unparseable line(s): {:?}",
        bad.len(),
        bad
    );
    assert!(
        !events.is_empty(),
        "expected at least the terminal envelope on stdout, got none"
    );

    // The advisory text belongs on stderr; the data channel stays payload-only.
    let stderr = String::from_utf8_lossy(&run.output.stderr);
    assert!(
        stderr.contains("global timeout elapsed"),
        "the deadline notice must be emitted on the error channel, stderr was: {stderr}"
    );
}

/// A3 — a cancelled run must NOT look like a completed one.
///
/// The measured defect: a run killed by the deadline reported
/// `total_replacements: 14035087` with no mention of the cancel, so a consumer
/// reading only the last NDJSON line saw an unqualified success. The terminal
/// record for a cancelled run must therefore be the `cancelled` envelope, and
/// no partial `summary` may be emitted at all.
#[test]
fn cancelled_run_emits_cancelled_envelope_and_no_summary() {
    let run = timed_run(Verb::Replace);
    let (events, _) = parse_strict(&run.output.stdout);

    assert!(
        summary_event(&events).is_none(),
        "a cancelled run must not emit a partial summary, stdout was: {events:?}"
    );

    let cancelled = events
        .iter()
        .find(|e| e["type"] == "cancelled")
        .unwrap_or_else(|| {
            panic!("a cancelled run must emit a `cancelled` envelope, stdout was: {events:?}")
        });

    // Fields the published schema requires, read from the live run.
    assert_eq!(
        cancelled["reason"].as_str(),
        Some("global --timeout-secs deadline elapsed"),
        "the reason must name the real cause, envelope was: {cancelled}"
    );
    assert!(
        cancelled.get("signal").is_none_or(Value::is_null),
        "an elapsed deadline is not a signal, envelope was: {cancelled}"
    );
    // The machine-readable verdict, so a consumer never has to parse the prose
    // to learn that the budget ran out rather than a signal arriving.
    assert_eq!(
        cancelled["deadline_exceeded"], true,
        "an elapsed deadline must say so in the field, envelope was: {cancelled}"
    );

    // The terminal record is the cancel, so a last-line reader cannot mistake it.
    let last = events.last().expect("at least one event");
    assert_eq!(
        last["type"], "cancelled",
        "the last data-channel line must be the cancel, stdout was: {events:?}"
    );
}

/// A completed run still emits its normal summary, with the counters intact.
/// Guards against the cancel guard swallowing the summary of a healthy run.
#[test]
fn completed_run_still_emits_its_summary() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("small.txt"), "tok=1 tok=1 tok=1\n").expect("write fixture");

    let (output, _) = run_verb(Verb::Replace, dir.path(), 60);
    assert!(output.status.success(), "small run must succeed");

    let (events, bad) = parse_strict(&output.stdout);
    assert!(
        bad.is_empty(),
        "unparseable lines on the data channel: {bad:?}"
    );

    let summary = summary_event(&events).expect("a completed run must emit its summary");
    assert_eq!(summary["files_matched"], 1, "summary was: {summary}");
    assert!(
        events.iter().all(|e| e["type"] != "cancelled"),
        "a completed run must not emit a cancel envelope, stdout was: {events:?}"
    );
}

/// The predicate crate is a project dev-dependency; keep one assertion on the
/// human-facing stderr so the notice stays reachable for operators.
#[test]
fn timeout_notice_predicate_matches_stderr() {
    let run = timed_run(Verb::Replace);
    let stderr = String::from_utf8_lossy(&run.output.stderr).into_owned();

    assert_eq!(run.output.status.code(), Some(124), "stderr: {stderr}");
    assert!(
        predicate::str::contains("global timeout elapsed").eval(stderr.as_str()),
        "the deadline notice must reach the operator on stderr, stderr was: {stderr}"
    );
}

/// Every scanning verb that can be driven past the deadline reports it.
///
/// Measured before the fix: `replace` emitted the cancel envelope while
/// `search` and `count` recognised the cancel and published nothing at all. A
/// consumer reading the last data-channel line of a cancelled run saw whatever
/// the walk had produced so far and no indication it had been cut short, which
/// is exactly the defect A3 removed from `replace`.
///
/// Only the verbs that read file content are covered, because only those can be
/// made to outlast a budget that still describes the operation. `hash` digests
/// 384 MiB in 61 ms, so a version of this test that gave it a short budget would
/// assert on a run that finished on its own and pass without ever observing a
/// cancel.
#[test]
fn every_content_scanning_verb_reports_the_interruption() {
    for verb in [Verb::Search, Verb::Count] {
        // Each verb gets a fixture sized from its own throughput. The
        // independence matters: a shared fixture sized for the slower verb would
        // let the faster one finish and pass without ever observing a cancel.
        let run = timed_run(verb);
        let output = &run.output;

        let (events, bad) = parse_strict(&output.stdout);
        assert!(
            bad.is_empty(),
            "{verb} put unparseable lines on the data channel: {bad:?}"
        );
        let cancelled = events
            .iter()
            .find(|e| e["type"] == "cancelled")
            .unwrap_or_else(|| {
                panic!(
                    "{verb} must report the interruption, status {:?}, elapsed {} ms, \
                     fixture {} bytes, stdout tail: {:?}",
                    output.status,
                    run.elapsed_ms,
                    run.fixture_bytes,
                    events.last()
                )
            });
        assert_eq!(
            cancelled["deadline_exceeded"], true,
            "{verb} must name the deadline as the cause, envelope was: {cancelled}"
        );
        assert!(
            cancelled.get("signal").is_none_or(Value::is_null),
            "{verb} reported a signal that never arrived, envelope was: {cancelled}"
        );
        let last = events.last().expect("at least one event");
        assert_eq!(
            last["type"], "cancelled",
            "{verb} must end on the cancel record, stdout was: {events:?}"
        );
    }
}

/// A1 — a fuzzy cascade that ends in a miss still obeys the global deadline.
///
/// Measured before the fix: with a 2 s budget, `replace --dry-run --fuzzy auto` on a
/// 50 MB file with no match took 6.5 s (100 MB: 13 s, 150 MB: 19.8 s) and `edit`
/// answered `MATCH_FAILED` (exit 65) after the same delay, because the cascade loops
/// never read the shutdown flag and the cascade ended in a miss instead of a cancel.
/// The run must now stop near the budget with exit 124 and say so on the data channel.
fn assert_fuzzy_miss_is_cancelled(verb: Verb) {
    let run = timed_run(verb);

    let ceiling = run.ceiling_ms();
    assert!(
        run.elapsed_ms < ceiling,
        "{verb} took {} ms, above the {ceiling} ms ceiling (budget {} ms, startup {} ms, \
         fixture {} bytes); the cascade did not honour the deadline",
        run.elapsed_ms,
        run.budget_ms,
        run.base_ms,
        run.fixture_bytes
    );

    let stderr = String::from_utf8_lossy(&run.output.stderr);
    assert_eq!(
        run.output.status.code(),
        Some(124),
        "{verb}: an elapsed deadline is exit 124, never MATCH_FAILED (65) or NO_MATCHES (1); \
         stderr: {stderr}"
    );

    let (events, bad) = parse_strict(&run.output.stdout);
    assert!(
        bad.is_empty(),
        "{verb} put unparseable lines on the data channel: {bad:?}"
    );
    let last = events.last().unwrap_or_else(|| {
        panic!("{verb} must end on an NDJSON record, stdout was empty; stderr: {stderr}")
    });
    assert!(
        last["type"] == "cancelled" || last["ok"] == false,
        "{verb} must end on a cancel or error record, last record was: {last}"
    );
    assert!(
        events
            .iter()
            .all(|e| e["code"] != "MATCH_FAILED" && e["code"] != "NO_MATCHES"),
        "{verb} reported a miss instead of the cancel, stdout was: {events:?}"
    );
    assert!(
        summary_event(&events).is_none(),
        "{verb}: a cancelled run must not emit a summary, stdout was: {events:?}"
    );
}

#[test]
fn fuzzy_replace_miss_honours_global_deadline() {
    assert_fuzzy_miss_is_cancelled(Verb::ReplaceFuzzyMiss);
}

#[test]
fn fuzzy_edit_miss_honours_global_deadline() {
    assert_fuzzy_miss_is_cancelled(Verb::EditFuzzyMiss);
}

/// The cascade reports an armed shutdown flag as `Cancelled`, never as a miss.
///
/// Arms the in-process flag, so it lives in this integration binary: every other
/// test here drives the CLI through a child process and never reads this flag, while
/// the library unit tests share one process where an armed flag would leak.
#[test]
fn fuzzy_cascade_reports_cancel_not_a_miss_when_shutdown_is_armed() {
    use atomwrite::error::AtomwriteError;
    use atomwrite::fuzzy::{MatchOpts, apply_fuzzy_one_pass, match_pair_with};
    use std::sync::atomic::Ordering;

    let signal = atomwrite::signal::get_or_install_handlers().expect("install handlers");
    signal.flag().store(true, Ordering::Release);

    let content = "alpha beta gamma\ndelta epsilon zeta\n".repeat(64);

    let err = match_pair_with(&content, MISS_TOKEN, "x", MatchOpts::default())
        .expect_err("a cancelled cascade is an error");
    assert!(
        matches!(err, AtomwriteError::Cancelled { .. }),
        "match_pair_with must return Cancelled, got {err:?}"
    );

    let err = apply_fuzzy_one_pass(&content, MISS_TOKEN, "x", MatchOpts::default(), None)
        .expect_err("a cancelled one-pass is an error, not zero matches");
    assert!(
        matches!(err, AtomwriteError::Cancelled { .. }),
        "apply_fuzzy_one_pass must pass Cancelled through, got {err:?}"
    );

    let err = apply_fuzzy_one_pass(&content, MISS_TOKEN, "x", MatchOpts::default(), Some(3))
        .expect_err("the multi-hit path must pass Cancelled through too");
    assert!(
        matches!(err, AtomwriteError::Cancelled { .. }),
        "multi-hit apply_fuzzy_one_pass must pass Cancelled through, got {err:?}"
    );
}

/// A signal is not a deadline, and the field must say which one fired.
///
/// The two causes shared one envelope before, and every fixed-string reason in
/// the scanning verbs said "by signal" — which was wrong exactly when the
/// global deadline was what stopped the run. The signal is delivered process-
/// to-process with `libc::kill`, the pattern `tests/signal_test.rs` already
/// uses, so the run is interrupted with no deadline involved at all.
///
/// The signal is sent only after the data channel has produced its first byte.
/// A run signalled before it starts walking would die without ever observing a
/// cancel, and the test would then pass on evidence that proves nothing. The
/// clock is not consulted for this; the child's own output is.
///
/// The run is stopped by the signal itself, so it never reaches the end of the
/// input and no search bound applies.
#[cfg(unix)]
#[test]
fn a_signal_reports_itself_and_not_a_deadline() {
    use std::io::Read as _;
    use std::process::Stdio;
    use std::sync::mpsc;

    let dir = tempfile::tempdir().expect("tempdir");
    write_fixture_of(dir.path(), SIGNAL_FILE_BYTES);
    let ws = dir.path().to_str().expect("utf8 workspace").to_string();

    let mut child = common::hermetic(std::process::Command::new(assert_cmd::cargo::cargo_bin(
        "atomwrite",
    )))
    .args([
        // A budget far beyond the test's own lifetime: only the signal can
        // be what stops this run.
        "--timeout-secs",
        "600",
        "--no-auto-heal",
        "--no-progress",
        // The fixture is larger than the search default cap, so without this
        // the walk visits nothing and there is no live work to interrupt.
        "--max-filesize",
        "200000000",
        "--workspace",
        &ws,
        "search",
        TOKEN,
        &ws,
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .expect("spawn search");

    let mut stdout = child.stdout.take().expect("stdout pipe");
    let (started_tx, started_rx) = mpsc::channel::<()>();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut notified = false;
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if !notified {
                        notified = true;
                        let _ = started_tx.send(());
                    }
                }
            }
        }
        buf
    });

    let observed_output = started_rx
        .recv_timeout(Duration::from_secs(SIGNAL_START_TIMEOUT_SECS))
        .is_ok();
    if !observed_output {
        let _ = child.kill();
        let _ = child.wait();
        panic!("the search produced no output within the start timeout to interrupt");
    }

    // SAFETY: `child.id()` is a live child of this process; SIGTERM is the
    // documented way to ask it to shut down.
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    // No `kill`: the process must unwind through its own handler, which is the
    // path that emits the record under test. A forced kill would skip it.
    let status = child.wait().expect("wait");
    let all = reader.join().expect("reader thread");

    let (events, _) = parse_strict(&all);
    let cancelled = events.iter().find(|e| e["type"] == "cancelled");
    let Some(cancelled) = cancelled else {
        panic!(
            "a signalled run must report the interruption, status {status:?}, stdout tail: {:?}",
            events.last()
        );
    };
    assert_eq!(
        cancelled["deadline_exceeded"], false,
        "a signal is not an elapsed deadline, envelope was: {cancelled}"
    );
    assert_eq!(
        cancelled["signal"].as_str(),
        Some("SIGTERM"),
        "the envelope must name the signal that arrived, envelope was: {cancelled}"
    );
    assert_eq!(
        cancelled["reason"].as_str(),
        Some("cancelled by signal"),
        "the reason must name the signal, not a deadline that never fired, envelope was: {cancelled}"
    );
}
