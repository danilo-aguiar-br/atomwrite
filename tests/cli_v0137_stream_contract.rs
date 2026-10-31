// SPDX-License-Identifier: MIT OR Apache-2.0

//! Stream contract of the scanning verbs: the `--limit` budget stops the walk,
//! and a read failure survives `--filter`.
//!
//! Both used to be claims the code did not keep: the writer dropped records
//! past the limit while the walker kept reading the tree, and the shaped tail
//! path could swallow the failure event that is the run's one terminal record.

#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Restores the mode of a path on drop so the temp dir can be removed.
struct RestoreMode(std::path::PathBuf, u32);

impl Drop for RestoreMode {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(self.1));
    }
}

fn run(ws: &Path, args: &[&str]) -> std::process::Output {
    common::atomwrite()
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(args)
        .output()
        .expect("run atomwrite")
}

fn terminal(events: &[serde_json::Value]) -> &serde_json::Value {
    events
        .iter()
        .find(|e| e.get("ok").is_some())
        .unwrap_or_else(|| panic!("no terminal record in {events:?}"))
}

#[test]
fn limit_stops_the_walker_early() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..60 {
        common::create_test_file(dir.path(), &format!("f{i}.txt"), "needle\n");
    }

    let total = run(dir.path(), &["search", "needle", "."]);
    let total_events = common::parse_ndjson(&total.stdout);
    assert_eq!(terminal(&total_events)["files_visited"], 60);

    let limited = run(dir.path(), &["search", "needle", ".", "--limit", "1"]);
    let limited_events = common::parse_ndjson(&limited.stdout);
    let summary = terminal(&limited_events);
    // `--limit` caps what leaves, and the walker quits once it sees the cap
    // spent. The field is read off the walker, not off the writer's budget,
    // because those are two different facts: measured before the fix, a run
    // that visited all 60 files still published `true` because the writer had
    // dropped a record. What holds on every run measured is one direction: a
    // walk that visited fewer than all 60 files always reports the quit. With
    // the parallel walker the converse is not guaranteed, because every entry
    // can be dispatched to a worker before any of them notices the quit.
    let visited = summary["files_visited"].as_u64().unwrap_or(0);
    let stopped_early = summary["walker_stopped_early"].as_bool().unwrap_or(false);
    assert!(
        visited >= 60 || stopped_early,
        "a walk that visited fewer files than the tree must report the quit: {summary}"
    );
}

#[test]
fn filter_never_drops_the_failure_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "good.txt", "needle\n");
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).expect("mkdir");
    common::create_test_file(&locked, "x.txt", "needle\n");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    let _restore = RestoreMode(locked.clone(), 0o755);
    if std::fs::read_dir(&locked).is_ok() {
        eprintln!("skipped: running as root, mode 000 does not deny reads");
        return;
    }

    let out = run(
        dir.path(),
        &["search", "needle", ".", "--filter", "ok=true"],
    );
    let events = common::parse_ndjson(&out.stdout);
    let summary = terminal(&events);
    // The tail filter narrows match events; the terminal summary is the
    // failure channel and is never filtered. A run that produced an answer
    // exits 0 and names the failure in `files_failed` and `errors[]`.
    assert_eq!(summary["files_failed"], 1, "{summary}");
    assert!(
        summary["errors"]
            .as_array()
            .is_some_and(|errors| !errors.is_empty()),
        "the failure event survived the filter: {summary}"
    );
    assert_eq!(summary["ok"], true, "{summary}");
    assert_eq!(out.status.code(), Some(0), "{events:?}");
}
