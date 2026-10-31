// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 `--version` format: `atomwrite <crate version> (<source>) <target>`,
//! where `<source>` is a short git SHA (optionally `-dirty`) or `no-git`,
//! never `unknown`.

mod common;

use assert_cmd::Command;
use std::time::Duration;

/// Per-process bound so a hung binary fails the test instead of the suite.
const PROC_TIMEOUT: Duration = Duration::from_secs(60);

fn version_line(flag: &str) -> String {
    let output = common::hermetic(Command::cargo_bin("atomwrite").expect("binary"))
        .timeout(PROC_TIMEOUT)
        .arg(flag)
        .output()
        .expect("run atomwrite");
    assert_eq!(output.status.code(), Some(0), "{flag} must exit 0");
    assert!(output.stderr.is_empty(), "{flag} must not write to stderr");
    String::from_utf8(output.stdout)
        .expect("utf8 stdout")
        .trim_end()
        .to_string()
}

/// `true` when `source` is `no-git` or a short hex SHA with an optional `-dirty`.
fn is_valid_source(source: &str) -> bool {
    if source == "no-git" {
        return true;
    }
    let sha = source.strip_suffix("-dirty").unwrap_or(source);
    (4..=40).contains(&sha.len()) && sha.chars().all(|c| c.is_ascii_hexdigit())
}

#[test]
fn version_has_crate_version_source_and_target() {
    let line = version_line("--version");
    let rest = line
        .strip_prefix("atomwrite ")
        .unwrap_or_else(|| panic!("missing binary name prefix: {line:?}"));
    let rest = rest
        .strip_prefix(env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|| panic!("missing crate version: {line:?}"));
    let rest = rest
        .strip_prefix(" (")
        .unwrap_or_else(|| panic!("missing source group: {line:?}"));
    let (source, target) = rest
        .split_once(") ")
        .unwrap_or_else(|| panic!("missing target after source: {line:?}"));
    assert!(
        is_valid_source(source),
        "invalid build source {source:?} in {line:?}"
    );
    assert!(
        !target.is_empty() && !target.contains(' '),
        "invalid target {target:?} in {line:?}"
    );
}

#[test]
fn build_source_is_never_unknown() {
    let line = version_line("--version");
    // Target triples such as `x86_64-unknown-linux-gnu` legitimately contain
    // "unknown"; only the parenthesised build source must never say it.
    assert!(
        !line.contains("(unknown)"),
        "build source must not say unknown: {line:?}"
    );
}

#[test]
fn short_flag_matches_long_flag() {
    assert_eq!(version_line("-V"), version_line("--version"));
}
