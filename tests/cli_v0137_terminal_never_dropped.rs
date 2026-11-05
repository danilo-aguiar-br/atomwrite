// SPDX-License-Identifier: MIT OR Apache-2.0

//! The terminal record survives every tail flag.
//!
//! A consumer reads the last line as the verdict. `--limit`, `--filter`,
//! `--sort-by`, `--dedupe`, `--aggregate`, `--diff-against` and `--select` act
//! on the DATA stream; none of them may remove the record that says how the run
//! ended or leave it projected down to `{}`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_atomwrite"))
}

struct Out {
    events: Vec<serde_json::Value>,
    code: i32,
}

fn run(args: &[&str], cwd: &Path) -> Out {
    let output = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn atomwrite");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let events = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("non-NDJSON stdout {l:?}: {e}")))
        .collect();
    Out {
        events,
        code: output.status.code().unwrap_or(-1),
    }
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 1..=12 {
        fs::write(
            dir.path().join(format!("f{i:02}.txt")),
            "needle line one\nneedle line two\n",
        )
        .expect("write fixture");
    }
    dir
}

/// The last line must be a record that names the run's verdict.
fn assert_terminal(out: &Out, what: &str) {
    let last = out
        .events
        .last()
        .unwrap_or_else(|| panic!("{what}: no record at all"));
    assert!(
        last.get("ok").is_some(),
        "{what}: the last record must carry the verdict, got {last}"
    );
    assert!(
        last.get("type").is_some() || last.get("ok") == Some(&serde_json::Value::Bool(false)),
        "{what}: the last record must name what it is, got {last}"
    );
    assert!(
        last.as_object().is_some_and(|o| !o.is_empty()),
        "{what}: the terminal must never collapse to {{}}"
    );
}

#[test]
fn list_terminal_survives_every_tail_flag() {
    let dir = fixture();
    for args in [
        vec!["list", ".", "--limit", "2"],
        vec!["list", ".", "--sort-by", "path", "--limit", "2"],
        vec!["list", ".", "--sort-by", "path", "--filter", "kind=file"],
        vec!["list", ".", "--sort-by", "path", "--dedupe", "path"],
        vec!["list", ".", "--sort-by", "path", "--select", "path"],
        vec!["list", ".", "--aggregate", "kind:count"],
    ] {
        let what = format!("list {}", args[1..].join(" "));
        assert_terminal(&run(&args, dir.path()), &what);
    }
}

#[test]
fn select_keeps_the_verdict_keys() {
    let dir = fixture();
    let out = run(
        &["list", ".", "--sort-by", "path", "--select", "path"],
        dir.path(),
    );
    let last = out.events.last().expect("a terminal record");
    assert_eq!(
        last["type"], "summary",
        "--select cannot hide which record closed the run"
    );
    assert_eq!(last["ok"], true, "--select cannot hide the verdict");
}

#[test]
fn search_terminal_survives_every_tail_flag() {
    let dir = fixture();
    for args in [
        vec!["search", "needle", ".", "--limit", "1"],
        vec!["search", "needle", ".", "--sort-by", "path", "--limit", "1"],
        vec![
            "search",
            "needle",
            ".",
            "--sort-by",
            "path",
            "--select",
            "path",
        ],
        vec!["search", "needle", ".", "--aggregate", "path:count"],
    ] {
        let what = format!("search {}", args[1..].join(" "));
        assert_terminal(&run(&args, dir.path()), &what);
    }
}

#[test]
fn diff_against_keeps_the_terminal() {
    let dir = fixture();
    let baseline = dir.path().join("baseline.ndjson");
    let first = run(&["list", ".", "--sort-by", "path"], dir.path());
    let text = first
        .events
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&baseline, text).expect("write baseline");
    let out = run(
        &[
            "list",
            ".",
            "--sort-by",
            "path",
            "--diff-against",
            baseline.to_str().expect("utf8"),
        ],
        dir.path(),
    );
    assert_terminal(&out, "list --diff-against");
}

#[test]
fn aggregate_does_not_swallow_the_terminal() {
    let dir = fixture();
    let out = run(&["list", ".", "--aggregate", "kind:count"], dir.path());
    let terminals = out.events.iter().filter(|e| e["type"] == "summary").count();
    assert_eq!(
        terminals, 1,
        "aggregation describes the data stream and leaves exactly one terminal"
    );
    assert_terminal(&out, "list --aggregate");
}

#[test]
fn limit_never_eats_the_terminal() {
    let dir = fixture();
    for n in ["1", "2", "5"] {
        let out = run(
            &["list", ".", "--sort-by", "path", "--limit", n],
            dir.path(),
        );
        let last = out.events.last().expect("a terminal record");
        assert_eq!(
            last["type"], "summary",
            "limit {n} must still close the run"
        );
        assert_eq!(last["ok"], true);
    }
}

#[test]
fn replace_and_scope_close_with_a_verdict() {
    let dir = fixture();
    assert_terminal(
        &run(
            &["replace", "needle", "pin", ".", "--limit", "1"],
            dir.path(),
        ),
        "replace --limit 1",
    );
    assert_terminal(
        &run(
            &["list", ".", "--sort-by", "path", "--filter", "kind=dir"],
            dir.path(),
        ),
        "list --filter with no matching data",
    );
}

#[test]
fn a_failing_run_still_ends_in_one_record() {
    let dir = fixture();
    let out = run(&["search", "needle-does-not-exist", "."], dir.path());
    let last = out.events.last().expect("a terminal record");
    assert_eq!(last["ok"], false, "a zero-hit search must say so");
    assert_eq!(
        out.code, 1,
        "a zero-hit search exits `NO_MATCHES`, the documented status for it"
    );
}
