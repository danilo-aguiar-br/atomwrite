// SPDX-License-Identifier: MIT OR Apache-2.0

//! `--limit` is a WORK BUDGET, not a print cap.
//!
//! A verb that rewrites, copies or deletes must touch exactly the number of
//! targets the budget allows, and its terminal record must say the walk stopped
//! before the tree ended. These tests measure the disk, not just the stdout: a
//! summary that claims one target while the tree shows forty is the defect this
//! file exists to catch.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_atomwrite"))
}

/// A throwaway tree with `n` regular files, each holding a line of text.
fn tree(n: usize) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 1..=n {
        fs::write(
            dir.path().join(format!("f{i}.txt")),
            format!("needle {i}\n"),
        )
        .expect("write fixture");
    }
    dir
}

struct Out {
    events: Vec<serde_json::Value>,
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
        .map(|l| serde_json::from_str(l).expect("stdout must be NDJSON only"))
        .collect();
    let _ = output.status;
    Out { events }
}

fn terminal(out: &Out) -> &serde_json::Value {
    out.events.last().expect("a run always ends in a record")
}

/// Regular files in `dir`.
fn count_files(dir: &Path) -> usize {
    fs::read_dir(dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .count()
}

#[test]
fn list_cuts_the_walk_at_the_budget() {
    let dir = tree(40);
    let out = run(&["list", ".", "--limit", "1"], dir.path());
    let entries = out.events.iter().filter(|e| e["type"] == "entry").count();
    assert_eq!(entries, 1, "list --limit 1 must print one entry");
    let last = terminal(&out);
    assert_eq!(last["files"], 1, "summary must count what was emitted");
    assert_eq!(
        last["walker_stopped_early"], true,
        "summary must name the truncation"
    );
}

#[test]
fn count_stops_at_the_budget() {
    let dir = tree(40);
    let out = run(&["count", ".", "--limit", "1"], dir.path());
    let last = terminal(&out);
    assert_eq!(
        last["total"]["files"], 1,
        "count --limit 1 must count one file"
    );
    assert_eq!(last["walker_stopped_early"], true);
}

#[test]
fn hash_stops_at_the_budget() {
    let dir = tree(40);
    let out = run(&["hash", ".", "-r", "--limit", "1"], dir.path());
    let hashes = out.events.iter().filter(|e| e["type"] == "hash").count();
    assert_eq!(hashes, 1, "hash --limit 1 must hash one file");
    let last = terminal(&out);
    assert_eq!(last["type"], "summary", "hash always closes with a summary");
    assert_eq!(last["files_visited"], 1);
    assert_eq!(last["walker_stopped_early"], true);
}

#[test]
fn replace_touches_one_file_on_disk() {
    let dir = tree(20);
    let out = run(
        &["replace", "needle", "REPLACED", ".", "--limit", "1"],
        dir.path(),
    );
    let last = terminal(&out);
    assert_eq!(last["files_modified"], 1, "one rewrite, not twenty");
    assert_eq!(last["walker_stopped_early"], true);
    let changed = fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| {
            fs::read_to_string(e.path())
                .map(|s| s.contains("REPLACED"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(changed, 1, "the disk must show exactly one rewritten file");
}

#[test]
#[cfg(feature = "ast")]
fn transform_touches_one_file_on_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 1..=10 {
        fs::write(
            dir.path().join(format!("f{i}.rs")),
            format!("fn old_name_{i}() {{ /*MARKER*/ }}\n"),
        )
        .expect("write fixture");
    }
    let out = run(
        &[
            "transform",
            "--pattern",
            "/*MARKER*/",
            "--rewrite",
            "/*CHANGED*/",
            "--language",
            "rust",
            ".",
            "--limit",
            "1",
        ],
        dir.path(),
    );
    let last = terminal(&out);
    assert_eq!(last["walker_stopped_early"], true, "budget must be named");
    let rewritten = fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| {
            fs::read_to_string(e.path())
                .map(|s| s.contains("CHANGED"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(
        rewritten, 1,
        "only the file the budget allowed is rewritten"
    );
}

#[test]
fn copy_copies_one_file() {
    // One root holds both sides: the workspace jail refuses a target outside it.
    let root = tempfile::tempdir().expect("tempdir");
    let src = root.path().join("src");
    let dst = root.path().join("out");
    fs::create_dir(&src).expect("create src");
    for i in 1..=8 {
        fs::write(src.join(format!("f{i}.txt")), format!("needle {i}\n")).expect("write fixture");
    }
    let ws = root.path().to_str().expect("utf8");
    let out = run(
        &[
            "copy",
            src.to_str().expect("utf8"),
            dst.to_str().expect("utf8"),
            "-r",
            "--limit",
            "1",
            "--workspace",
            ws,
        ],
        root.path(),
    );
    let copied = out.events.iter().filter(|e| e["type"] == "copied").count();
    assert_eq!(
        copied, 1,
        "copy --limit 1 must copy one file; stdout was {:#?}",
        out.events
    );
    let on_disk = count_files(&dst);
    assert_eq!(on_disk, 1, "the destination must hold one file");
    assert_eq!(terminal(&out)["walker_stopped_early"], true);
}

#[test]
fn backup_creates_one_backup() {
    let dir = tree(4);
    let out = run(
        &[
            "backup", "f1.txt", "f2.txt", "f3.txt", "f4.txt", "--limit", "1",
        ],
        dir.path(),
    );
    let backups = fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().contains(".bak."))
        .count();
    assert_eq!(backups, 1, "backup --limit 1 must create one backup");
    let last = terminal(&out);
    assert_eq!(last["files_backed_up"], 1);
    assert_eq!(last["walker_stopped_early"], true);
}

#[test]
fn delete_removes_one_file() {
    let dir = tree(6);
    let out = run(&["delete", ".", "-r", "--limit", "1"], dir.path());
    let last = terminal(&out);
    assert_eq!(
        last["files_modified"], 1,
        "delete --limit 1 removes one file"
    );
    assert_eq!(last["walker_stopped_early"], true);
    // The delete keeps a backup of the one file it removed, so the surviving
    // originals are what the budget promised: five of the six.
    let remaining = fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".txt"))
        .count();
    assert_eq!(remaining, 5, "five files must survive a budget of one");
}

#[test]
fn case_converts_one_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 1..=5 {
        fs::write(
            dir.path().join(format!("f{i}.rs")),
            format!("let myVar{i} = 1;\n"),
        )
        .expect("write fixture");
    }
    let out = run(
        &[
            "case",
            "--to",
            "snake",
            "--subvert",
            "myVar",
            "my_var",
            ".",
            "--limit",
            "1",
        ],
        dir.path(),
    );
    let last = terminal(&out);
    assert_eq!(last["files_visited"], 1);
    assert_eq!(last["walker_stopped_early"], true);
}

#[test]
fn unbounded_runs_report_the_whole_tree() {
    let dir = tree(40);
    let out = run(&["list", "."], dir.path());
    let last = terminal(&out);
    assert_eq!(last["files"], 40, "no budget means the whole tree");
    assert!(
        last.get("walker_stopped_early").is_none(),
        "an untouched walk must not claim it stopped early"
    );
}
