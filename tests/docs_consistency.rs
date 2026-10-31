// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 red tests tying the documentation and the source tree to the
//! `agent-surface` manifest, the single source of truth (R12, R39).
//!
//! The manifest is read from the real binary, never from a constant, so these
//! tests keep following the surface when it grows.

mod common;

use assert_cmd::Command;
use regex::Regex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

const PROC_TIMEOUT: Duration = Duration::from_secs(60);

/// Files whose subcommand-count claims must match the manifest. Each one must
/// carry at least one current total. `CONTRIBUTING*.md` is left out because it
/// quotes no current total; the migration guides quote it in their 0.1.37 section.
const COUNT_FILES: &[&str] = &[
    "README.md",
    "README.pt-BR.md",
    "llms.txt",
    "llms-full.txt",
    "llms.pt-BR.txt",
    "skills/atomwrite-en/SKILL.md",
    "skills/atomwrite-pt/SKILL.md",
    "INTEGRATIONS.md",
    "INTEGRATIONS.pt-BR.md",
    "ARCHITECTURE.md",
    "ARCHITECTURE.pt-BR.md",
    "docs/AGENTS.md",
    "docs/AGENTS.pt-BR.md",
    "docs/COOKBOOK.md",
    "docs/COOKBOOK.pt-BR.md",
    "docs/CROSS_PLATFORM.md",
    "docs/CROSS_PLATFORM.pt-BR.md",
    "docs/HOW_TO_USE.md",
    "docs/HOW_TO_USE.pt-BR.md",
    "docs/INSTALL.md",
    "docs/INSTALL.pt-BR.md",
    "docs/MIGRATION.md",
    "docs/MIGRATION.pt-BR.md",
    "docs/TESTING.md",
    "docs/TESTING.pt-BR.md",
    "docs/schemas/README.md",
];

/// A claim like "46 subcommands" is a TOTAL only from this size up; smaller
/// numbers describe families ("backup flag on 15 subcommands").
const TOTAL_CLAIM_MIN: u64 = 20;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

fn run_stdout(args: &[&str]) -> String {
    let out = common::hermetic(Command::cargo_bin("atomwrite").expect("binary"))
        .timeout(PROC_TIMEOUT)
        .args(args)
        .output()
        .expect("run atomwrite");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn surface() -> Value {
    let text = run_stdout(&["agent-surface", "--format", "json"]);
    let line = text
        .lines()
        .find(|l| l.contains("\"tools\""))
        .unwrap_or_else(|| panic!("agent-surface emitted no manifest: {text:.200}"));
    serde_json::from_str(line).expect("manifest is JSON")
}

/// `(code, NAME)` pairs of the manifest's exit table.
fn exit_codes(surface: &Value) -> Vec<(u64, String)> {
    surface["exit_codes"]
        .as_array()
        .expect("manifest has exit_codes")
        .iter()
        .map(|e| {
            (
                e["code"].as_u64().expect("exit code number"),
                e["name"].as_str().expect("exit code name").to_string(),
            )
        })
        .collect()
}

/// Does some line of `text` carry both the exit number and the NAME?
fn has_row(text: &str, code: u64, name: &str) -> bool {
    let num = Regex::new(&format!(r"\b{code}\b")).expect("regex");
    let word = Regex::new(&format!(r"\b{}\b", regex::escape(name))).expect("regex");
    text.lines().any(|l| num.is_match(l) && word.is_match(l))
}

/// The version the docs describe, as it appears in release headings.
const CURRENT_RELEASE: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Total-count claims: "47 subcommands", "**47** subcommands", "47 subcomandos".
fn claim_regex() -> Regex {
    Regex::new(r"\b(\d+)\*{0,2}\s+(?i:subcommands?|subcomandos?|tools)\b").expect("regex")
}

/// Level and text of a Markdown heading, or `None` for any other line.
fn heading_level(line: &str) -> Option<usize> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    (1..=6)
        .contains(&hashes)
        .then_some(hashes)
        .filter(|n| line[*n..].starts_with(' '))
}

/// Does a heading name releases only older than `current`
/// ("What's New in v0.1.36")? A heading with no release, or one that names the
/// current release, is not a past section.
fn is_past_release_heading(line: &str, current: &str) -> bool {
    let re = Regex::new(r"v0\.1\.\d+").expect("regex");
    let mut seen = false;
    for m in re.find_iter(line) {
        if m.as_str() == current {
            return false;
        }
        seen = true;
    }
    seen
}

/// Total subcommand counts quoted in `text` as `(line number, claimed total)`.
///
/// Lines marked "Historical"/"Histórico" and everything under a heading of an
/// older release are history and are skipped, as are numbers below
/// [`TOTAL_CLAIM_MIN`] (they describe families). Headings inside code fences are
/// shell comments, not headings.
fn total_claims(text: &str, current: &str) -> Vec<(usize, u64)> {
    let re = claim_regex();
    let mut stack: Vec<bool> = Vec::new();
    let mut in_fence = false;
    let mut claims = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if !in_fence && let Some(level) = heading_level(line) {
            stack.truncate(level - 1);
            stack.resize(level - 1, false);
            stack.push(is_past_release_heading(line, current));
        }
        let lower = line.to_lowercase();
        if stack.iter().any(|past| *past)
            || lower.contains("historical")
            || lower.contains("histórico")
        {
            continue;
        }
        for cap in re.captures_iter(line) {
            let n: u64 = cap[1].parse().expect("number");
            if n >= TOTAL_CLAIM_MIN {
                claims.push((i + 1, n));
            }
        }
    }
    claims
}

/// One message per claim in `text` that differs from `total`, plus one when the
/// file quotes no total at all.
fn stale_claims(file: &str, text: &str, total: u64, current: &str) -> Vec<String> {
    let claims = total_claims(text, current);
    let mut bad: Vec<String> = claims
        .iter()
        .filter(|(_, n)| *n != total)
        .map(|(line, n)| format!("{file}:{line}: says {n}, manifest has {total}"))
        .collect();
    if claims.is_empty() {
        bad.push(format!(
            "{file}: no total subcommand count found (expected {total})"
        ));
    }
    bad
}

// Guard (passes while docs and manifest agree): every TOTAL subcommand count
// quoted in the docs equals the manifest's tool count.
#[test]
fn r12_documented_subcommand_totals_match_agent_surface() {
    let total = surface()["tools"].as_array().expect("tools array").len() as u64;
    let bad: Vec<String> = COUNT_FILES
        .iter()
        .flat_map(|file| stale_claims(file, &read(file), total, CURRENT_RELEASE))
        .collect();
    assert!(
        bad.is_empty(),
        "stale subcommand counts:\n{}",
        bad.join("\n")
    );
}

// The reader itself must catch the shapes that slipped through before: a bold
// total such as `**45** subcommands`, the Portuguese forms, and a stale count
// next to a current one.
#[test]
fn r12_total_reader_rejects_stale_bold_counts() {
    let cur = CURRENT_RELEASE;
    let bold = "- Pin agents to `^0.1.36`. **45** subcommands (SSOT).";
    assert_eq!(total_claims(bold, cur), vec![(1, 45)]);
    assert_eq!(total_claims("**46 subcommands**", cur), vec![(1, 46)]);
    assert_eq!(
        total_claims("- Pin. **45** subcomandos (SSOT).", cur),
        vec![(1, 45)]
    );
    assert_eq!(
        total_claims("- **46** subcommand implementations", cur),
        vec![(1, 46)]
    );
    assert_eq!(total_claims("## 46 Subcommands (SSOT)", cur), vec![(1, 46)]);
    let stale = stale_claims("x.md", bold, 47, cur);
    assert_eq!(stale, vec!["x.md:1: says 45, manifest has 47".to_string()]);
    assert!(stale_claims("x.md", "- **47** subcommands (SSOT)", 47, cur).is_empty());
    let none = stale_claims("x.md", "no count here", 47, cur);
    assert_eq!(
        none.len(),
        1,
        "a file with no total must be reported: {none:?}"
    );
}

// History is not a claim: a "Historical" line, a section of an older release,
// a family count below the total threshold. A shell comment in a code fence
// must not end a past section, and a current section is still checked.
#[test]
fn r12_total_reader_skips_history_and_families() {
    let cur = CURRENT_RELEASE;
    assert!(total_claims("- Historical (v0.1.36): **45** subcommands", cur).is_empty());
    assert!(total_claims("- Histórico (v0.1.36): **45** subcomandos", cur).is_empty());
    assert!(total_claims("backup flag on 15 subcommands", cur).is_empty());
    let past = "## What's New in v0.1.36\n- **45** subcommands";
    assert!(total_claims(past, cur).is_empty());
    let nested = "## v0.1.35 to v0.1.36\n### Steps\n- **45** subcommands";
    assert!(total_claims(nested, cur).is_empty());
    let fenced = "## v0.1.36\n```bash\n# Install atomwrite (no release)\n```\n- **45** subcommands";
    assert!(total_claims(fenced, cur).is_empty());
    let now = format!("## What's New in {cur}\n- **45** subcommands");
    assert_eq!(total_claims(&now, cur), vec![(2, 45)]);
    let both = format!("## v0.1.36 to {cur} (Current)\n- **45** subcommands");
    assert_eq!(total_claims(&both, cur), vec![(2, 45)]);
    let reset = "## What's New in v0.1.36\n- **45** subcommands\n## 47 Subcommands";
    assert_eq!(total_claims(reset, cur), vec![(3, 47)]);
}

// RED until R39: `atomwrite --help` must carry an exit-code section listing
// every code of the manifest with its NAME.
#[test]
fn r39_help_lists_every_exit_code() {
    let help = run_stdout(&["--help"]);
    let lower = help.to_lowercase();
    assert!(
        lower.contains("exit codes") || lower.contains("exit code"),
        "`atomwrite --help` has no exit-codes section"
    );
    let missing: Vec<String> = exit_codes(&surface())
        .into_iter()
        .filter(|(code, name)| !has_row(&help, *code, name))
        .map(|(code, name)| format!("{code} {name}"))
        .collect();
    assert!(
        missing.is_empty(),
        "{} exit codes missing from --help: {}",
        missing.len(),
        missing.join(", ")
    );
}

// RED until R39: both READMEs carry the exit-code table between the markers
// `<!-- exit-codes:begin -->` and `<!-- exit-codes:end -->`, one row per code.
#[test]
fn r39_readmes_list_every_exit_code_between_markers() {
    let codes = exit_codes(&surface());
    let mut problems = Vec::new();
    for file in ["README.md", "README.pt-BR.md"] {
        let text = read(file);
        let begin = text.find("<!-- exit-codes:begin -->");
        let end = text.find("<!-- exit-codes:end -->");
        let (Some(b), Some(e)) = (begin, end) else {
            problems.push(format!("{file}: exit-codes markers are missing"));
            continue;
        };
        if e <= b {
            problems.push(format!(
                "{file}: exit-codes end marker precedes the begin marker"
            ));
            continue;
        }
        let block = &text[b..e];
        let missing: Vec<String> = codes
            .iter()
            .filter(|(code, name)| !has_row(block, *code, name))
            .map(|(code, name)| format!("{code} {name}"))
            .collect();
        if !missing.is_empty() {
            problems.push(format!("{file}: rows missing for {}", missing.join(", ")));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

// RED until R12: the manifest must not mention MCP at all.
#[test]
fn r12_agent_surface_has_no_mcp_key() {
    let s = surface();
    assert!(
        s.get("mcp").is_none(),
        "agent-surface still carries `mcp`: {}",
        s["mcp"]
    );
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let path = e.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

// RED until R12: no tracked Rust source under src/ may cite MCP.
#[test]
fn r12_src_does_not_mention_mcp() {
    let re = Regex::new(r"(?i)\bmcp\b").expect("regex");
    let mut files = Vec::new();
    rust_files(&root().join("src"), &mut files);
    assert!(!files.is_empty(), "no Rust sources found under src/");
    let mut hits = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        for (i, line) in text.lines().enumerate() {
            if re.is_match(line) {
                let rel = f.strip_prefix(root()).unwrap_or(f).display().to_string();
                hits.push(format!("{rel}:{}", i + 1));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "{} MCP mentions in src/: {}",
        hits.len(),
        hits.join(", ")
    );
}

// RED until R12: environment detection must not read process environment
// variables (no product knobs, no ambient authority).
#[test]
fn r12_env_detect_reads_no_environment_variables() {
    let text = read("src/env_detect.rs");
    let offenders: Vec<&str> = ["var_os(", "env::var("]
        .into_iter()
        .filter(|needle| text.contains(needle))
        .collect();
    assert!(
        offenders.is_empty(),
        "src/env_detect.rs still reads the environment through {offenders:?}"
    );
}

/// Documentation files that describe the `doctor` host detection.
const DOCTOR_ENV_DOC_FILES: [&str; 7] = [
    "ARCHITECTURE.md",
    "ARCHITECTURE.pt-BR.md",
    "PRIVACY.md",
    "PRIVACY.pt-BR.md",
    "docs/TESTING.md",
    "docs/TESTING.pt-BR.md",
    "docs/CROSS_PLATFORM.md",
];

/// Phrases that used to claim the `doctor` reads the process environment, or
/// that it still detects CI and `sudo`, which the 0.1.37 detection dropped.
const STALE_DOCTOR_ENV_CLAIMS: [&str; 8] = [
    "may read host env",
    "pode ler env do host",
    "pode observar env do host",
    "A-ENV-001",
    "CI indicators",
    "indicadores de CI",
    "K8s/CI/Termux",
    "Snap/sudo",
];

// The documentation must not contradict `r12_env_detect_reads_no_environment_variables`.
#[test]
fn r12_docs_do_not_claim_doctor_reads_the_environment() {
    let offenders: Vec<String> = DOCTOR_ENV_DOC_FILES
        .iter()
        .flat_map(|file| {
            let text = read(file);
            STALE_DOCTOR_ENV_CLAIMS
                .iter()
                .filter(move |claim| text.contains(**claim))
                .map(move |claim| format!("{file}: {claim}"))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "docs still claim the doctor reads the environment: {offenders:?}"
    );
}

// The project rule is H1, H2 for sections and H3 for subsections, and H4 and
// deeper are forbidden. The two CHANGELOG files carried 123 H4 headings, so the
// rule was published and broken in the same repository.
const CHANGELOG_FILES: [&str; 2] = ["CHANGELOG.md", "CHANGELOG.pt-BR.md"];

// Levels allowed in those files: H1 once per document, H2 per release, H3 per
// kind of change.
const MAX_HEADING_LEVEL: usize = 3;

// Each CHANGELOG keeps its history in one language, so a heading of the other
// language is a translation that stopped half way. These are the words that
// make the mistake visible.
const PT_HEADINGS: [&str; 9] = [
    "Adicionado",
    "Alterado",
    "Corrigido",
    "Removido",
    "Documentação",
    "Validação",
    "Notas",
    "Testes",
    "Migração",
];

const EN_HEADINGS: [&str; 9] = [
    "Added",
    "Changed",
    "Fixed",
    "Removed",
    "Documentation",
    "Validation",
    "Notes",
    "Tests",
    "Migration",
];

#[test]
fn r12_changelogs_use_only_h1_h2_and_h3() {
    let offenders: Vec<String> = CHANGELOG_FILES
        .iter()
        .flat_map(|file| {
            read(file)
                .lines()
                .enumerate()
                .filter_map(move |(i, line)| {
                    let level = heading_level(line)?;
                    (level > MAX_HEADING_LEVEL)
                        .then(|| format!("{file}:{}: H{level} {line}", i + 1))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "H{MAX_HEADING_LEVEL} is the deepest allowed heading: {offenders:?}"
    );
}

#[test]
fn r12_changelogs_keep_their_headings_in_their_own_language() {
    let offenders: Vec<String> = CHANGELOG_FILES
        .iter()
        .flat_map(|file| {
            let foreign: &[&str] = if file.ends_with("pt-BR.md") {
                &EN_HEADINGS
            } else {
                &PT_HEADINGS
            };
            read(file)
                .lines()
                .enumerate()
                .filter_map(move |(i, line)| {
                    let level = heading_level(line)?;
                    if level < 2 {
                        return None;
                    }
                    let title = line.trim_start_matches('#').trim();
                    foreign
                        .iter()
                        .any(|word| title.starts_with(word))
                        .then(|| format!("{file}:{}: {line}", i + 1))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "a CHANGELOG heading must be in the file's language: {offenders:?}"
    );
}
