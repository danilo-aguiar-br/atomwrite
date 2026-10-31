// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 hardcode guard (C14).
//!
//! Scans the production code under `src/`, `build.rs` and `xtask/src`, and
//! fails when a literal that must live in `src/constants.rs` (or a
//! `constants*.inc.rs` module) shows up somewhere else, or when the product
//! reads the process environment or spawns an external process.
//!
//! Skipped on purpose: `constants*` files, test files (`*tests*.rs`),
//! `#[cfg(test)]` items and comment lines.
//! Every exception is listed in [`ALLOWED`] with its reason, so a new
//! violation cannot hide behind an old one.

use std::fs;
use std::path::{Path, PathBuf};

/// One rule the guard enforces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Rule {
    /// `Duration::from_{secs,millis,micros,nanos}(<numeric literal>)`.
    DurationLiteral,
    /// The timestamped-backup infix `.bak.`; use `constants::BACKUP_INFIX`.
    BackupInfix,
    /// Sidecar and probe names that `constants` owns.
    SidecarName,
    /// Time-unit magic numbers; use `constants::SECS_PER_*`.
    TimeUnitNumber,
    /// `env::var`, `env::var_os`, `env::vars`: the product never reads the environment.
    EnvRead,
    /// `Command::new`: the product never spawns an external process.
    ProcessSpawn,
}

/// Sidecar and probe names that only `constants` may spell out.
const SIDECAR_NAMES: &[&str] = &[
    ".atomwrite.journal",
    ".atomwrite.lock",
    ".atomwrite_keep_wal",
    ".atomwrite_no_wal",
    ".atomwrite-doctor-probe",
    "tokens.ndjson",
];

/// Time-unit numbers that only `constants` may spell out.
const TIME_UNIT_NUMBERS: &[&str] = &["86400", "86_400", "604800", "604_800", "3600"];

/// Duration constructors whose first argument must not be a numeric literal.
const DURATION_CTORS: &[&str] = &[
    "Duration::from_secs(",
    "Duration::from_millis(",
    "Duration::from_micros(",
    "Duration::from_nanos(",
];

/// Explicit exceptions: (file suffix, rule, text the offending line contains, reason).
///
/// Only the build tooling is excepted: `build.rs` and `xtask` are not the
/// shipped product, and each reason says why the exception is real.
const ALLOWED: &[(&str, Rule, &str, &str)] = &[
    (
        "build.rs",
        Rule::ProcessSpawn,
        "Command::new(\"git\")",
        "the build script embeds the commit hash; git runs at compile time only",
    ),
    (
        "build.rs",
        Rule::EnvRead,
        "env::var_os(\"CARGO_MANIFEST_DIR\")",
        "cargo passes the manifest directory to every build script",
    ),
    (
        "build.rs",
        Rule::EnvRead,
        "env::var(\"TARGET\")",
        "cargo passes the target triple to every build script",
    ),
    (
        "xtask/src/main.rs",
        Rule::ProcessSpawn,
        "Command::new(\"cargo\")",
        "the xtask runner shells out to cargo by design",
    ),
];

fn src_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Files the guard does not inspect: constants modules and test files.
fn is_skipped_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.starts_with("constants") || name.contains("tests")
}

/// Number of leading spaces of a line.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The production lines of a file as `(1-based line number, text)`.
///
/// Drops comment lines and every `#[cfg(test)]` item. The item ends at the
/// first later line that closes at the attribute's own indentation (`}`), or
/// at its first line when that line already ends the item (`;` or `}`).
fn production_lines(text: &str) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed == "#[cfg(test)]" {
            let indent = indent_of(line);
            i += 1;
            // Other attributes and doc lines of the same item.
            while i < lines.len()
                && (lines[i].trim().starts_with("#[") || lines[i].trim().starts_with("//"))
            {
                i += 1;
            }
            let Some(first) = lines.get(i) else { break };
            let first = first.trim_end();
            if first.ends_with(';') || (first.ends_with('}') && !first.ends_with("{}")) {
                i += 1;
                continue;
            }
            i += 1;
            while i < lines.len() {
                let closes =
                    indent_of(lines[i]) == indent && lines[i].trim_start().starts_with('}');
                i += 1;
                if closes {
                    break;
                }
            }
            continue;
        }
        if !trimmed.starts_with("//") && !trimmed.contains("help = \"") {
            out.push((i + 1, line));
        }
        i += 1;
    }
    out
}

/// `true` when `Duration::from_*(` at `text` is followed by a numeric literal.
fn duration_literal_in(text: &str) -> bool {
    DURATION_CTORS.iter().any(|ctor| {
        text.match_indices(ctor).any(|(at, _)| {
            text[at + ctor.len()..]
                .trim_start()
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        })
    })
}

fn violations_in(line: &str) -> Vec<Rule> {
    let mut found = Vec::new();
    if duration_literal_in(line) {
        found.push(Rule::DurationLiteral);
    }
    if line.contains(".bak.") {
        found.push(Rule::BackupInfix);
    }
    if SIDECAR_NAMES.iter().any(|name| line.contains(name)) {
        found.push(Rule::SidecarName);
    }
    if TIME_UNIT_NUMBERS.iter().any(|n| contains_number(line, n)) {
        found.push(Rule::TimeUnitNumber);
    }
    if line.contains("env::var") {
        found.push(Rule::EnvRead);
    }
    if line.contains("Command::new") {
        found.push(Rule::ProcessSpawn);
    }
    found
}

/// `true` when `number` appears in `line` as a whole token, not inside a longer number.
fn contains_number(line: &str, number: &str) -> bool {
    line.match_indices(number).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + number.len()..].chars().next();
        let is_part = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        !is_part(before) && !is_part(after)
    })
}

fn is_allowed(file: &str, rule: Rule, line: &str) -> bool {
    ALLOWED
        .iter()
        .any(|(suffix, r, needle, _)| file.ends_with(suffix) && *r == rule && line.contains(needle))
}

/// Every violation under `src/`, `build.rs` and `xtask/src` that is not in
/// [`ALLOWED`], as `file:line: rule: text`.
fn scan_sources() -> Vec<String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_rs_files(&src_root(), &mut files);
    let build_script = manifest.join("build.rs");
    if build_script.is_file() {
        files.push(build_script);
    }
    collect_rs_files(&manifest.join("xtask").join("src"), &mut files);
    files.sort();
    let mut report = Vec::new();
    for path in files.iter().filter(|p| !is_skipped_file(p)) {
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let rel = path
            .strip_prefix(manifest)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for (number, line) in production_lines(&text) {
            for rule in violations_in(line) {
                if !is_allowed(&rel, rule, line) {
                    report.push(format!("{rel}:{number}: {rule:?}: {}", line.trim()));
                }
            }
        }
    }
    report
}

#[test]
fn production_code_has_no_hardcoded_literals() {
    let report = scan_sources();
    assert!(
        report.is_empty(),
        "move these literals to src/constants.rs (or list a reasoned exception in ALLOWED):\n{}",
        report.join("\n")
    );
}

#[test]
fn every_allowed_exception_still_matches_a_real_line() {
    let root = src_root();
    for (suffix, rule, needle, reason) in ALLOWED {
        assert!(!reason.is_empty(), "exception for {suffix} needs a reason");
        let path = root.parent().unwrap_or(&root).join(suffix);
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let hit = production_lines(&text)
            .into_iter()
            .any(|(_, line)| line.contains(needle) && violations_in(line).contains(rule));
        assert!(
            hit,
            "stale exception: {suffix} no longer has a {rule:?} line containing {needle:?}; remove it"
        );
    }
}

#[test]
fn guard_detects_each_rule_on_a_sample() {
    assert!(violations_in("let d = Duration::from_millis(250);").contains(&Rule::DurationLiteral));
    assert!(
        violations_in("let d = std::time::Duration::from_secs( 5 );")
            .contains(&Rule::DurationLiteral)
    );
    assert!(!violations_in("let d = Duration::from_millis(ms);").contains(&Rule::DurationLiteral));
    assert!(
        !violations_in("let d = Duration::from_millis(C::POLL_MS);")
            .contains(&Rule::DurationLiteral)
    );
    assert!(violations_in("format!(\"{name}.bak.\")").contains(&Rule::BackupInfix));
    assert!(violations_in("dir.join(\".atomwrite.lock\")").contains(&Rule::SidecarName));
    assert!(violations_in("let day = secs / 86400;").contains(&Rule::TimeUnitNumber));
    assert!(!violations_in("let n = 1_086_400_000;").contains(&Rule::TimeUnitNumber));
    assert!(violations_in("let v = std::env::var(\"X\");").contains(&Rule::EnvRead));
    assert!(violations_in("let v = env::var_os(\"X\");").contains(&Rule::EnvRead));
    assert!(violations_in("std::process::Command::new(\"ls\")").contains(&Rule::ProcessSpawn));
    assert!(violations_in("let t = env!(\"TARGET\");").is_empty());
}

#[test]
fn guard_skips_test_items_and_comments() {
    let text = "\
fn prod() {}
// Duration::from_secs(5)
#[cfg(test)]
mod tests {
    fn t() {
        let _ = Duration::from_secs(5);
    }
}
fn after() {}
#[cfg(test)]
fn helper() {
    let _ = \".bak.\";
}
#[cfg(test)]
include!(\"x_tests.inc.rs\");
fn last() {}
";
    let kept: Vec<&str> = production_lines(text).into_iter().map(|(_, l)| l).collect();
    assert_eq!(kept, ["fn prod() {}", "fn after() {}", "fn last() {}"]);
}

#[test]
fn product_ships_no_script_or_ci_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(!root.join(".github").exists(), ".github must not exist");
    let mut files = Vec::new();
    collect_all_files(root, &mut files);
    let scripts: Vec<String> = files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "sh"))
        .map(|p| p.display().to_string())
        .collect();
    assert!(scripts.is_empty(), "no .sh file may exist: {scripts:?}");
}

/// Files below `dir`, skipping build output and dependency folders.
fn collect_all_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name == "target" || name == ".git" || name.starts_with('.') && name != ".github" {
                continue;
            }
            collect_all_files(&path, out);
        } else {
            out.push(path);
        }
    }
}
