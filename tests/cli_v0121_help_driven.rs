// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.22 ADR-0034 — help-driven anti-pattern coverage for new sub-commands.
//!
//! For every flag declared in `prune-backups --help` and `edit-loop --help`,
//! there must be a regression test whose name or body mentions that flag.
//! This catches the "flag in help before implementation exists" anti-pattern
//! documented in ADR-0034.

mod common;

/// All flags exposed by `edit-loop --help` MUST have a regression test in
/// `tests/cli_v0121_edit_loop.rs` that references the flag name. This is
/// the v0.1.21+ invariant established by ADR-0034.
#[test]
fn edit_loop_help_flags_all_wired_to_tests() {
    let output = common::atomwrite()
        .args(["edit-loop", "--help"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);

    // Whitelist of flags that edit-loop advertises in --help.
    let flags = [
        "--allow-sequential-drift",
        "--backup",
        "--retention",
        "--keep-backup",
        "--syntax-check",
        "--line-ending",
    ];
    for flag in flags {
        assert!(
            help.contains(flag),
            "flag {flag} deve aparecer em edit-loop --help"
        );
    }

    // Verify each flag has a regression test. Accept either the kebab-case
    // (`--allow-sequential-drift`) or snake_case (`allow_sequential_drift`)
    // form so tests written in either style satisfy the invariant.
    let test_path = std::path::Path::new("tests/cli_v0121_edit_loop.rs");
    let test_content = std::fs::read_to_string(test_path).expect("read test file");
    for flag in flags {
        let kebab = flag.trim_start_matches("--");
        let snake = kebab.replace('-', "_");
        assert!(
            test_content.contains(kebab) || test_content.contains(&snake),
            "teste deve referenciar flag {flag} (procura por {kebab} ou {snake})"
        );
    }
}

/// All flags exposed by `prune-backups --help` MUST have a regression test
/// in `tests/cli_v0121_prune_backups.rs` that references the flag name.
#[test]
fn prune_backups_help_flags_all_wired_to_tests() {
    let output = common::atomwrite()
        .args(["prune-backups", "--help"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);

    let flags = ["--max-age-secs", "--max-count", "--dry-run"];
    for flag in flags {
        assert!(help.contains(flag), "flag {flag} deve aparecer em --help");
    }

    let test_path = std::path::Path::new("tests/cli_v0121_prune_backups.rs");
    let test_content = std::fs::read_to_string(test_path).expect("read test file");
    for flag in flags {
        let kebab = flag.trim_start_matches("--");
        let snake = kebab.replace('-', "_");
        assert!(
            test_content.contains(kebab) || test_content.contains(&snake),
            "teste deve referenciar flag {flag} (procura por {kebab} ou {snake})"
        );
    }
}

/// Both sub-commands must appear in the top-level `atomwrite --help` output.
/// This is the "the flag is wired into the CLI" half of the help-driven check.
#[test]
fn new_subcommands_listed_in_top_help() {
    let output = common::atomwrite().args(["--help"]).output().expect("run");

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        help.contains("edit-loop"),
        "edit-loop deve aparecer em --help"
    );
    assert!(
        help.contains("prune-backups"),
        "prune-backups deve aparecer em --help"
    );
}
