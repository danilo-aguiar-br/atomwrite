// SPDX-License-Identifier: MIT OR Apache-2.0

//! `--dry-run` on the verbs that only write: `set`, `del`, `semantic-merge`
//! and `config set|unset`.
//!
//! The flag reports the planned change as a `plan` event and leaves the file
//! byte-for-byte as it was. These verbs had no preview, so the only way to know
//! what they would do was to let them do it.

mod common;

use std::path::Path;

fn run(ws: &Path, args: &[&str]) -> std::process::Output {
    common::atomwrite()
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(args)
        .output()
        .expect("run atomwrite")
}

#[test]
fn set_and_del_dry_run_write_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "x.toml", "a = 1\n");

    let set = run(dir.path(), &["set", "x.toml", "a", "2", "--dry-run"]);
    let set_events = common::parse_ndjson(&set.stdout);
    assert_eq!(set.status.code(), Some(0), "{set_events:?}");
    assert_eq!(set_events[0]["type"], "plan", "{set_events:?}");
    assert_eq!(set_events[0]["operation"], "set", "{set_events:?}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("x.toml")).expect("read"),
        "a = 1\n",
        "set --dry-run leaves the file untouched"
    );

    let del = run(dir.path(), &["del", "x.toml", "a", "--dry-run"]);
    let del_events = common::parse_ndjson(&del.stdout);
    assert_eq!(del.status.code(), Some(0), "{del_events:?}");
    assert_eq!(del_events[0]["operation"], "del", "{del_events:?}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("x.toml")).expect("read"),
        "a = 1\n",
        "del --dry-run leaves the file untouched"
    );
}

#[test]
fn config_set_and_unset_dry_run_write_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = dir.path().join(".atomwrite.toml");

    let set = run(
        dir.path(),
        &[
            "config",
            "set",
            "defaults.max_filesize",
            "4096",
            "--scope",
            "workspace",
            "--dry-run",
        ],
    );
    let set_events = common::parse_ndjson(&set.stdout);
    assert_eq!(set.status.code(), Some(0), "{set_events:?}");
    assert_eq!(set_events[0]["dry_run"], true, "{set_events:?}");
    assert!(!config.exists(), "config set --dry-run creates nothing");

    common::create_test_file(
        dir.path(),
        ".atomwrite.toml",
        "[defaults]\nmax_filesize = 4096\n",
    );
    let unset = run(
        dir.path(),
        &[
            "config",
            "unset",
            "defaults.max_filesize",
            "--scope",
            "workspace",
            "--dry-run",
        ],
    );
    let unset_events = common::parse_ndjson(&unset.stdout);
    assert_eq!(unset.status.code(), Some(0), "{unset_events:?}");
    assert_eq!(unset_events[0]["dry_run"], true, "{unset_events:?}");
    let text = std::fs::read_to_string(&config).expect("read");
    assert!(text.contains("max_filesize = 4096"), "{text}");
}

#[test]
fn semantic_merge_dry_run_creates_no_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    common::create_test_file(dir.path(), "base.txt", "a\nb\nc\n");
    common::create_test_file(dir.path(), "ours.txt", "a\nB\nc\n");
    common::create_test_file(dir.path(), "theirs.txt", "a\nb\nC\n");

    let out = run(
        dir.path(),
        &[
            "semantic-merge",
            "--base",
            "base.txt",
            "--ours",
            "ours.txt",
            "--theirs",
            "theirs.txt",
            "--output",
            "out.txt",
            "--dry-run",
        ],
    );
    let events = common::parse_ndjson(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{events:?}");
    assert_eq!(events[0]["type"], "plan", "{events:?}");
    assert!(
        !dir.path().join("out.txt").exists(),
        "the output file is not created"
    );
}
