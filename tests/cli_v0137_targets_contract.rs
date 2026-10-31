// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 target and TOML contracts.
//!
//! Four verbs used to answer the wrong thing when the target was not the shape
//! they assumed: `case` walked nothing in a directory and still exited 0,
//! `prune-backups` accepted a directory and pruned nothing, `get`/`set`/`del`
//! could not see inside an inline table, and `transform` rewrote a file with a
//! grammar its extension did not name. Each test measures the answer the verb
//! gives now.

mod common;

use std::path::Path;

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
fn case_expands_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.rs", "let HTTPRequest = 1;\n");
    common::create_test_file(dir.path(), "sub/b.rs", "let HTTPRequest = 2;\n");
    let ws = dir.path().to_str().expect("utf8");

    let out = run(
        dir.path(),
        &[
            "case",
            ws,
            "--subvert",
            "HTTPRequest",
            "http_request",
            "--to",
            "snake",
        ],
    );
    let events = common::parse_ndjson(&out.stdout);
    let summary = terminal(&events);
    assert_eq!(out.status.code(), Some(0), "{events:?}");
    assert_eq!(summary["files_visited"], 2, "both files were rewritten");
    assert_eq!(summary["identifiers_total"], 2, "{summary}");
    for rel in ["a.rs", "sub/b.rs"] {
        let text = std::fs::read_to_string(dir.path().join(rel)).expect("read");
        assert!(text.contains("http_request"), "{rel}: {text}");
    }
}

#[test]
fn case_directory_without_files_exits_65() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join("empty")).expect("mkdir");
    let ws = dir.path().to_str().expect("utf8");

    let out = run(
        dir.path(),
        &["case", ws, "--subvert", "A", "B", "--to", "snake"],
    );
    let events = common::parse_ndjson(&out.stdout);
    assert_eq!(out.status.code(), Some(65), "{events:?}");
    assert_eq!(terminal(&events)["code"], "INVALID_INPUT", "{events:?}");
}

#[test]
fn prune_backups_directory_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().expect("utf8");

    let out = run(dir.path(), &["prune-backups", ws, "--max-age-secs", "10"]);
    let events = common::parse_ndjson(&out.stdout);
    assert_eq!(out.status.code(), Some(65), "{events:?}");
    assert_eq!(terminal(&events)["code"], "INVALID_INPUT", "{events:?}");
}

#[test]
fn inline_table_keys_are_read_written_and_removed() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "a.toml", "pool = { max = 5, name = \"x\" }\n");

    let got = run(dir.path(), &["get", "a.toml", "pool.max"]);
    let got_events = common::parse_ndjson(&got.stdout);
    assert_eq!(got.status.code(), Some(0), "{got_events:?}");
    assert_eq!(got_events[0]["value"], "5", "{got_events:?}");

    let set = run(dir.path(), &["set", "a.toml", "pool.max", "9"]);
    let set_events = common::parse_ndjson(&set.stdout);
    assert_eq!(set.status.code(), Some(0), "{set_events:?}");
    let text = std::fs::read_to_string(dir.path().join("a.toml")).expect("read");
    assert!(text.contains("max = 9"), "{text}");
    assert!(
        text.contains("name = \"x\""),
        "sibling key survives: {text}"
    );

    let del = run(dir.path(), &["del", "a.toml", "pool.max"]);
    let del_events = common::parse_ndjson(&del.stdout);
    assert_eq!(del.status.code(), Some(0), "{del_events:?}");
    let text = std::fs::read_to_string(dir.path().join("a.toml")).expect("read");
    assert!(!text.contains("max"), "{text}");
    assert!(text.contains("name = \"x\""), "{text}");
}

#[cfg(feature = "ast")]
#[test]
fn transform_extension_mismatch_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = common::create_test_file(dir.path(), "a.rb", "puts \"hi\"\n");

    let out = run(
        dir.path(),
        &["transform", "a.rb", "-l", "haskell", "-p", "x", "-r", "y"],
    );
    let events = common::parse_ndjson(&out.stdout);
    assert_eq!(out.status.code(), Some(65), "{events:?}");
    assert_eq!(terminal(&events)["code"], "INVALID_INPUT", "{events:?}");
    let text = std::fs::read_to_string(&file).expect("read");
    assert_eq!(text, "puts \"hi\"\n", "the file is untouched");
}
