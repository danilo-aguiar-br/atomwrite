// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 — Explicit Target Designation (ETD).
//!
//! A verb with side effects must receive its subject in the invocation line.
//! These tests pin four properties:
//!
//! 1. a relative root resolves against `--workspace`, never the current
//!    working directory, so an ambient tree stays byte-identical;
//! 2. no token changes role when a non-positional flag enters or leaves the
//!    line — arity never carries semantics;
//! 3. a side-effect verb with no target in argv exits with the usage code and
//!    mutates nothing anywhere;
//! 4. an idempotent read verb may still inherit the ambient subject.
//!
//! Every test builds its own `tempfile::TempDir`; the guard removes the
//! directory on drop, so the suite leaves no residue behind.

mod common;

use std::path::{Path, PathBuf};

use atomwrite::cli::{Cli, Commands};
use clap::Parser;

/// Fixture source, before the rewrite.
const SOURCE: &str = "fn main() {\n    dbg!(41);\n}\n";

/// Fixture source the `dbg!` removal must produce.
const REWRITTEN: &str = "fn main() {\n    41;\n}\n";

/// Plant a Rust fixture at `root/name` and return its path.
fn plant(root: &Path, name: &str) -> PathBuf {
    common::create_test_file(root, name, SOURCE)
}

/// Read a fixture back, panicking on I/O failure.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("read fixture")
}

#[cfg(feature = "ast")]
#[test]
fn relative_root_resolves_against_workspace_not_cwd() {
    let workspace = tempfile::tempdir().expect("workspace");
    let elsewhere = tempfile::tempdir().expect("elsewhere");

    // Two fixtures whose text matches the AST pattern: one at the ambient
    // root, one inside the directory named in argv. A fallback to the ambient
    // root would rewrite the first and leave the second untouched.
    let ambient = plant(workspace.path(), "ambient.rs");
    let nested_dir = workspace.path().join("nested");
    std::fs::create_dir(&nested_dir).expect("mkdir nested");
    let named = plant(&nested_dir, "named.rs");

    let output = common::atomwrite()
        .current_dir(elsewhere.path())
        .args([
            "--workspace",
            workspace.path().to_str().expect("utf8 workspace"),
            "transform",
            "--no-backup",
            "-p",
            "dbg!($$$A)",
            "-r",
            "$$$A",
            "-l",
            "rust",
            "nested",
        ])
        .output()
        .expect("run transform");

    assert!(
        output.status.success(),
        "exit: {:?} stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        read(&named),
        REWRITTEN,
        "the subject named in argv must be the one rewritten"
    );
    assert_eq!(
        read(&ambient),
        SOURCE,
        "the ambient root must stay byte-identical when argv names another subject"
    );
}

#[test]
fn no_token_changes_role_when_a_flag_enters_the_line() {
    let bare = [
        "atomwrite",
        "transform",
        "-p",
        "dbg!($$$A)",
        "-r",
        "$$$A",
        "-l",
        "rust",
        "nested",
    ];
    let flagged = [
        "atomwrite",
        "transform",
        "--dry-run",
        "-p",
        "dbg!($$$A)",
        "-r",
        "$$$A",
        "-l",
        "rust",
        "nested",
    ];

    let plain = Cli::try_parse_from(bare).expect("parse without the flag");
    let with_flag = Cli::try_parse_from(flagged).expect("parse with the flag");

    match (plain.command, with_flag.command) {
        (Commands::Transform(a), Commands::Transform(b)) => {
            assert_eq!(a.paths, vec![PathBuf::from("nested")]);
            assert_eq!(
                a.paths, b.paths,
                "the positional must keep its role when --dry-run enters the line"
            );
            assert!(!a.dry_run);
            assert!(b.dry_run, "the flag must land in its named slot");
        }
        other => panic!("expected two transform parses, got {other:?}"),
    }
}

#[test]
fn side_effect_verb_without_target_fails_closed() {
    let workspace = tempfile::tempdir().expect("workspace");
    let ambient = plant(workspace.path(), "ambient.rs");

    let output = common::atomwrite()
        .current_dir(workspace.path())
        .args([
            "--workspace",
            workspace.path().to_str().expect("utf8 workspace"),
            "transform",
            "--no-backup",
            "-p",
            "dbg!($$$A)",
            "-r",
            "$$$A",
            "-l",
            "rust",
        ])
        .output()
        .expect("run transform");

    assert_eq!(
        output.status.code(),
        Some(2),
        "an incomplete invocation must exit with the usage code; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let events = common::parse_ndjson(&output.stdout);
    let envelope = events.first().expect("error envelope on stdout");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["exit"], 2);
    assert_eq!(envelope["code"], "ARGUMENT_PARSE_ERROR");
    assert_eq!(
        read(&ambient),
        SOURCE,
        "a usage failure must not mutate the ambient root"
    );
}

#[test]
fn read_verb_may_inherit_the_ambient_target() {
    let parsed = Cli::try_parse_from(["atomwrite", "search", "needle"])
        .expect("search parses without a path");
    match parsed.command {
        Commands::Search(args) => assert!(
            args.paths.is_empty(),
            "an omitted path stays empty at parse time, so the run reports it as the workspace root"
        ),
        other => panic!("expected a search parse, got {other:?}"),
    }
}

#[test]
fn side_effect_verb_rejects_a_missing_target_at_parse_time() {
    let usage_error =
        Cli::try_parse_from(["atomwrite", "transform", "-p", "x", "-r", "y", "-l", "rust"])
            .expect_err("a side-effect verb must require its target");
    assert_eq!(usage_error.exit_code(), 2);
}

#[test]
fn conflicting_rule_selectors_fail_closed() {
    let from_file = Cli::try_parse_from(["atomwrite", "transform", "--rules", "r.yaml", "."])
        .expect("--rules alone parses");
    match from_file.command {
        Commands::Transform(args) => assert_eq!(args.rules, Some(PathBuf::from("r.yaml"))),
        other => panic!("expected a transform parse, got {other:?}"),
    }

    let both = Cli::try_parse_from([
        "atomwrite",
        "transform",
        "--rules",
        "r.yaml",
        "--inline-rules",
        "- id: a",
        ".",
    ])
    .expect_err("two rule selectors must conflict");
    assert_eq!(
        both.exit_code(),
        2,
        "a conflicting selector pair is a usage error"
    );
}
