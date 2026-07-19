// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.28 GAP-CLI-SURFACE-DRIFT — backup flag matrix coverage.
//!
//! Every mutating subcommand flattens the shared `BackupOpts` struct
//! (`--backup`, `--no-backup`, `--keep-backup`, `--retention`). This suite
//! verifies, for all 15 subcommands, that: the flags parse; `--backup` and
//! `--no-backup` conflict; the CLI definition has zero declaration errors
//! (`debug_assert`); and there is no id collision between `BackupOpts` and
//! the global flags. It also exercises the semantic effect of a few flags
//! end-to-end (real backup files on disk).

mod common;

use atomwrite::cli::Cli;
use clap::{CommandFactory, Parser};

/// Minimal-but-valid positional arguments for each of the 15 subcommands
/// that flatten `BackupOpts` (v0.1.28). Values are placeholders: these
/// tests only exercise `try_parse_from`, never touching the filesystem.
fn subcommands_with_backup_opts() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("write", vec!["write", "f.txt"]),
        ("edit", vec!["edit", "f.txt"]),
        ("edit-loop", vec!["edit-loop", "f.txt"]),
        ("replace", vec!["replace", "old", "new"]),
        ("transform", vec!["transform"]),
        ("scope", vec!["scope", "-l", "rust"]),
        ("apply", vec!["apply", "f.txt"]),
        ("set", vec!["set", "f.toml", "key.path", "value"]),
        ("del", vec!["del", "f.toml", "key.path"]),
        ("case", vec!["case", "f.rs"]),
        ("batch", vec!["batch"]),
        ("delete", vec!["delete", "f.txt"]),
        ("move", vec!["move", "src.txt", "dst.txt"]),
        ("copy", vec!["copy", "src.txt", "dst.txt"]),
        ("rollback", vec!["rollback", "f.txt"]),
    ]
}

fn parse(base: &[&str], extra: &[&str]) -> Result<Cli, clap::Error> {
    let mut argv = vec!["atomwrite"];
    argv.extend_from_slice(base);
    argv.extend_from_slice(extra);
    Cli::try_parse_from(argv)
}

#[test]
fn all_15_subcommands_accept_backup_flag() {
    for (name, base) in subcommands_with_backup_opts() {
        let result = parse(&base, &["--backup"]);
        assert!(result.is_ok(), "{name} must accept --backup: {result:?}");
    }
}

#[test]
fn all_15_subcommands_accept_no_backup_flag() {
    for (name, base) in subcommands_with_backup_opts() {
        let result = parse(&base, &["--no-backup"]);
        assert!(
            result.is_ok(),
            "{name} must accept --no-backup: {result:?}"
        );
    }
}

#[test]
fn all_15_subcommands_accept_keep_backup_flag() {
    for (name, base) in subcommands_with_backup_opts() {
        let result = parse(&base, &["--keep-backup"]);
        assert!(
            result.is_ok(),
            "{name} must accept --keep-backup: {result:?}"
        );
    }
}

#[test]
fn all_15_subcommands_accept_retention_flag() {
    for (name, base) in subcommands_with_backup_opts() {
        let result = parse(&base, &["--retention", "3"]);
        assert!(
            result.is_ok(),
            "{name} must accept --retention 3: {result:?}"
        );
    }
}

#[test]
fn all_15_subcommands_reject_backup_and_no_backup_together() {
    for (name, base) in subcommands_with_backup_opts() {
        let result = parse(&base, &["--backup", "--no-backup"]);
        assert!(
            result.is_err(),
            "{name} must reject --backup together with --no-backup"
        );
    }
}

#[test]
fn edit_after_match_conflicts_with_new() {
    let result = Cli::try_parse_from([
        "atomwrite",
        "edit",
        "f.txt",
        "--after-match",
        "X",
        "--new",
        "Y",
    ]);
    assert!(
        result.is_err(),
        "--after-match and --new must conflict at parse time"
    );
}

#[test]
fn cli_definition_has_zero_declaration_errors() {
    // clap's own declarative sanity checker: duplicate ids, colliding
    // short/long flags, invalid conflicts_with references, etc.
    Cli::command().debug_assert();
}

#[test]
fn backup_opts_flags_do_not_collide_with_global_flags() {
    let cmd = Cli::command();
    let global_long_flags: Vec<String> = cmd
        .get_arguments()
        .filter(|a| a.is_global_set())
        .filter_map(clap::Arg::get_long)
        .map(str::to_owned)
        .collect();
    let global_short_flags: Vec<char> = cmd
        .get_arguments()
        .filter(|a| a.is_global_set())
        .filter_map(clap::Arg::get_short)
        .collect();

    let backup_opts_long_flags = ["backup", "no-backup", "keep-backup", "retention"];
    for flag in backup_opts_long_flags {
        assert!(
            !global_long_flags.contains(&flag.to_owned()),
            "global flag collides with BackupOpts::{flag}"
        );
    }
    // BackupOpts declares no short flags; guard against future drift.
    assert!(
        !global_short_flags.contains(&'b'),
        "global short flag -b would collide with a future backup short flag"
    );
}

#[test]
fn replace_with_retention_succeeds_on_real_match() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "f.txt", "foo bar foo\n");

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "replace",
            "--retention",
            "2",
            "foo",
            "baz",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(
        output.status.code(),
        Some(0),
        "replace with --retention 2 must succeed when there is a match: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "baz bar baz\n");
}

#[test]
fn delete_no_backup_leaves_no_bak_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "f.txt", "bye\n");

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--no-backup",
            "--yes",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(!path.exists());
    let has_bak = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .any(|e| e.file_name().to_string_lossy().contains(".bak."));
    assert!(
        !has_bak,
        "delete --no-backup must not leave a .bak.* file"
    );
}

#[test]
fn delete_default_backup_true_leaves_bak_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "f.txt", "bye\n");

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--yes",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    assert!(!path.exists());
    let has_bak = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .any(|e| e.file_name().to_string_lossy().contains(".bak."));
    assert!(
        has_bak,
        "delete without flags (default v0.1.28: backup=true) must leave a .bak.* file"
    );
}

#[test]
fn delete_keep_backup_emits_warning_in_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::create_test_file(dir.path(), "f.txt", "bye\n");

    let output = common::atomwrite()
        .env_remove("ATOMWRITE_BACKUP")
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "delete",
            "--keep-backup",
            "--yes",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let deleted = events
        .iter()
        .find(|e| e["type"] == "deleted")
        .expect("deleted event");
    let warnings = deleted["warnings"]
        .as_array()
        .expect("warnings must be an array");
    assert!(
        !warnings.is_empty(),
        "delete --keep-backup must emit warnings in the envelope (flag is a no-op for delete)"
    );
}
