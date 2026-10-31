// SPDX-License-Identifier: MIT OR Apache-2.0

//! The exit-code table of both READMEs matches the `agent-surface` manifest.
//!
//! The manifest is derived from the error enum, so a status added to the code
//! but missing from the README fails here instead of drifting silently (R12).
//! The table sits between the markers `<!-- exit-codes:begin -->` and
//! `<!-- exit-codes:end -->`, one bullet per status.

mod common;

use regex::Regex;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

const BEGIN: &str = "<!-- exit-codes:begin -->";
const END: &str = "<!-- exit-codes:end -->";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `(code, NAME)` pairs of the manifest's exit table, sorted.
fn manifest_codes() -> Vec<(u64, String)> {
    let out = common::hermetic(Command::new(env!("CARGO_BIN_EXE_atomwrite")))
        .args(["agent-surface", "--format", "json"])
        .output()
        .expect("run atomwrite agent-surface");
    assert!(
        out.status.success(),
        "agent-surface failed: {:?}",
        out.status
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text
        .lines()
        .find(|l| l.contains("\"exit_codes\""))
        .unwrap_or_else(|| panic!("agent-surface emitted no manifest: {text:.200}"));
    let manifest: Value = serde_json::from_str(line).expect("manifest is JSON");
    let mut codes: Vec<(u64, String)> = manifest["exit_codes"]
        .as_array()
        .expect("manifest has exit_codes")
        .iter()
        .map(|e| {
            (
                e["code"].as_u64().expect("exit code number"),
                e["name"].as_str().expect("exit code name").to_string(),
            )
        })
        .collect();
    codes.sort();
    codes
}

/// Rows of the marked table in `file`, in document order.
fn readme_rows(file: &str) -> Vec<(u64, String)> {
    // Tolerate CRLF checkouts: the markers and the rows are matched per line.
    let text = std::fs::read_to_string(root().join(file))
        .unwrap_or_else(|e| panic!("cannot read {file}: {e}"))
        .replace("\r\n", "\n");
    let begin = text
        .find(BEGIN)
        .unwrap_or_else(|| panic!("{file}: missing {BEGIN}"));
    let end = text
        .find(END)
        .unwrap_or_else(|| panic!("{file}: missing {END}"));
    assert!(begin < end, "{file}: end marker precedes the begin marker");
    let row = Regex::new(r"^- `(\d+)` ([A-Z][A-Z0-9_]*)\b").expect("regex");
    text[begin + BEGIN.len()..end]
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            let cap = row
                .captures(l)
                .unwrap_or_else(|| panic!("{file}: not a table row: {l:?}"));
            (cap[1].parse().expect("code"), cap[2].to_string())
        })
        .collect()
}

#[test]
fn readme_exit_table_matches_agent_surface() {
    let expected = manifest_codes();
    for file in ["README.md", "README.pt-BR.md"] {
        let rows = readme_rows(file);
        let mut sorted = rows.clone();
        sorted.sort();
        assert_eq!(
            sorted, expected,
            "{file}: the exit-code table differs from agent-surface"
        );
    }
}

#[test]
fn readme_exit_table_is_in_ascending_order() {
    for file in ["README.md", "README.pt-BR.md"] {
        let codes: Vec<u64> = readme_rows(file).into_iter().map(|(c, _)| c).collect();
        let mut ascending = codes.clone();
        ascending.sort_unstable();
        assert_eq!(codes, ascending, "{file}: exit codes are out of order");
    }
}

#[test]
fn readme_tables_list_the_same_statuses() {
    let mut english = readme_rows("README.md");
    let mut portuguese = readme_rows("README.pt-BR.md");
    english.sort();
    portuguese.sort();
    assert_eq!(english, portuguese, "the two READMEs disagree");
}
