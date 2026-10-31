// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 documentation guards: the docs must cover every command of the CLI,
//! cite only flags that exist, avoid removed spellings, keep the English and
//! Portuguese editions in step, keep accents, keep links alive and quote
//! current counts.
//!
//! The binary is the source of truth (`agent-surface`, `commands`,
//! `--print-schema`, `--help`); no command name, flag or total is written down
//! here. Every guard lists ALL offenders, never just the first.

mod common;
#[path = "common/doc_scan.rs"]
mod doc_scan;

use assert_cmd::Command;
use doc_scan::{Tok, anchors, command_words, github_slug, heading, inline_code, invocations};
use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const PROC_TIMEOUT: Duration = Duration::from_secs(60);

/// Guide, reference and agent-facing documents: every command must appear in
/// each of them, and every example must use real flags.
const COMMAND_DOCS: &[&str] = &[
    "README.md",
    "README.pt-BR.md",
    "INTEGRATIONS.md",
    "INTEGRATIONS.pt-BR.md",
    "docs/AGENTS.md",
    "docs/AGENTS.pt-BR.md",
    "docs/HOW_TO_USE.md",
    "docs/HOW_TO_USE.pt-BR.md",
    "docs/COOKBOOK.md",
    "docs/COOKBOOK.pt-BR.md",
    "llms.txt",
    "llms.pt-BR.txt",
    "llms-full.txt",
    "llms-full.pt-BR.txt",
    "skills/atomwrite-en/SKILL.md",
    "skills/atomwrite-pt/SKILL.md",
];

/// Further documents that share the removed-spelling and count guards.
const OTHER_DOCS: &[&str] = &[
    "docs/INSTALL.md",
    "docs/INSTALL.pt-BR.md",
    "docs/TESTING.md",
    "docs/TESTING.pt-BR.md",
    "docs/MIGRATION.md",
    "docs/MIGRATION.pt-BR.md",
    "docs/CROSS_PLATFORM.md",
    "docs/CROSS_PLATFORM.pt-BR.md",
    "ARCHITECTURE.md",
    "ARCHITECTURE.pt-BR.md",
    "SECURITY.md",
    "SECURITY.pt-BR.md",
    "PRIVACY.md",
    "PRIVACY.pt-BR.md",
    "CONTRIBUTING.md",
    "CONTRIBUTING.pt-BR.md",
    "CODE_OF_CONDUCT.md",
    "CODE_OF_CONDUCT.pt-BR.md",
];

/// Documents that describe the SIGPIPE design: the command documents plus the
/// ones that discuss architecture, platforms, security, privacy and process.
const SIGPIPE_DOCS: &[&str] = &[
    "ARCHITECTURE.md",
    "ARCHITECTURE.pt-BR.md",
    "docs/CROSS_PLATFORM.md",
    "docs/CROSS_PLATFORM.pt-BR.md",
    "SECURITY.md",
    "SECURITY.pt-BR.md",
    "PRIVACY.md",
    "PRIVACY.pt-BR.md",
    "CONTRIBUTING.md",
    "CONTRIBUTING.pt-BR.md",
];

/// Documents whose subcommand and schema totals must be current. Migration
/// guides are history by nature and the schema index quotes the schema total.
const COUNT_DOCS_EXCLUDED: &[&str] = &["docs/MIGRATION.md", "docs/MIGRATION.pt-BR.md"];
const COUNT_DOCS_EXTRA: &[&str] = &["docs/schemas/README.md"];

/// Spellings that no longer exist in the CLI or that the project rules forbid.
/// `--include-fifo` is a real `search` flag, so only the `write` form is listed.
const REMOVED_SPELLINGS: &[&str] = &[
    "--mode raw",
    "write --include-fifo",
    "read --raw",
    "set --type",
    "verify --checksum",
    "--diff-algorithm",
    "--skip-startup-wal-heal",
    "--budget-ms",
    "--lock-timeout",
    "edit --confirm",
    "transform --lang",
];

/// A line that mentions `SIG_DFL` is fine when it also says the design is a
/// deliberate departure from it.
const SIGPIPE_DEVIATION_MARKERS: &[&str] = &[
    "not ",
    "neither",
    "não",
    "nem ",
    "deviation",
    "desvio",
    "cooperative",
    "cooperativo",
];

/// ASCII spellings that lost their accent in the Portuguese documents.
const UNACCENTED_WORDS: &[&str] = &[
    "nao",
    "voce",
    "sao",
    "tambem",
    "codigo",
    "versao",
    "execucao",
    "configuracao",
    "decisao",
    "conteudo",
    "Portugues",
    "atencao",
    "usuario",
    "diretorio",
    "minimo",
    "maximo",
];

/// Document pairs (English path) whose Portuguese edition must have the same
/// shape. The Portuguese path inserts `.pt-BR` before the extension.
const PAIRS: &[&str] = &[
    "README.md",
    "INTEGRATIONS.md",
    "ARCHITECTURE.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "PRIVACY.md",
    "CODE_OF_CONDUCT.md",
    "docs/AGENTS.md",
    "docs/HOW_TO_USE.md",
    "docs/COOKBOOK.md",
    "docs/INSTALL.md",
    "docs/TESTING.md",
    "docs/MIGRATION.md",
    "docs/CROSS_PLATFORM.md",
    "llms.txt",
    "llms-full.txt",
];

/// A count below this size describes a family ("backup flag on 15
/// subcommands"), not the total.
const TOTAL_CLAIM_MIN: u64 = 20;

/// The release the documents describe.
const CURRENT_RELEASE: &str = concat!("v", env!("CARGO_PKG_VERSION"));

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load(rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root().join(rel)).map_err(|e| format!("{rel}: cannot read ({e})"))
}

/// Run `check` on every document; unreadable files are offenders too.
fn each_doc(files: &[&str], mut check: impl FnMut(&str, &str) -> Vec<String>) -> Vec<String> {
    files
        .iter()
        .flat_map(|rel| match load(rel) {
            Ok(text) => check(rel, &text),
            Err(e) => vec![e],
        })
        .collect()
}

fn assert_clean(guard: &str, offenders: &[String]) {
    assert!(
        offenders.is_empty(),
        "{guard}: {} offender(s)\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

fn run(args: &[&str]) -> (bool, String) {
    let out = common::hermetic(Command::new(env!("CARGO_BIN_EXE_atomwrite")))
        .timeout(PROC_TIMEOUT)
        .args(args)
        .output()
        .expect("run atomwrite");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn json_line(text: &str, needle: &str) -> Value {
    let line = text
        .lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line with {needle}: {text:.200}"));
    serde_json::from_str(line).expect("JSON line")
}

// --- the CLI surface, read from the binary --------------------------------

struct Arg {
    long: Option<String>,
    takes_value: bool,
}

struct Cmd {
    args: Vec<Arg>,
    subs: BTreeMap<String, Cmd>,
}

struct Cli {
    globals: Vec<Arg>,
    commands: BTreeMap<String, Cmd>,
    /// Names of the manifest's `type == "tool"` entries.
    tools: Vec<String>,
    /// Names printed by `--print-schema`.
    schemas: Vec<String>,
}

fn parse_args(value: &Value) -> Vec<Arg> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .map(|a| Arg {
                    long: a["long"].as_str().map(str::to_string),
                    takes_value: a["takes_value"].as_bool().unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_cmds(value: &Value) -> BTreeMap<String, Cmd> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .map(|c| {
                    let name = c["name"].as_str().expect("command name").to_string();
                    let cmd = Cmd {
                        args: parse_args(&c["args"]),
                        subs: parse_cmds(&c["subcommands"]),
                    };
                    (name, cmd)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn cli() -> &'static Cli {
    static CLI: OnceLock<Cli> = OnceLock::new();
    CLI.get_or_init(|| {
        let (_, tree) = run(&["commands", "--include-globals"]);
        let tree = json_line(&tree, "\"commands\"");
        let (_, surface) = run(&["agent-surface", "--format", "json"]);
        let surface = json_line(&surface, "\"tools\"");
        let tools = surface["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .filter(|t| t["type"] == "tool")
            .map(|t| t["name"].as_str().expect("tool name").to_string())
            .collect();
        let (_, schemas) = run(&["--print-schema"]);
        Cli {
            globals: parse_args(&tree["global_args"]),
            commands: parse_cmds(&tree["commands"]),
            tools,
            schemas: schemas
                .lines()
                .map(|l| l.trim().trim_matches('"').to_string())
                .filter(|l| !l.is_empty())
                .collect(),
        }
    })
}

impl Cli {
    fn cmd_at(&self, chain: &[String]) -> Vec<&Cmd> {
        let mut found = Vec::new();
        let mut level = &self.commands;
        for name in chain {
            let Some(cmd) = level.get(name) else { break };
            found.push(cmd);
            level = &cmd.subs;
        }
        found
    }

    /// `Some(takes_value)` when `long` is a flag of the chain or a global.
    fn flag(&self, chain: &[String], long: &str) -> Option<bool> {
        self.cmd_at(chain)
            .iter()
            .rev()
            .flat_map(|c| c.args.iter())
            .chain(self.globals.iter())
            .find(|a| a.long.as_deref() == Some(long))
            .map(|a| a.takes_value)
    }

    /// Long flags and aliases printed by `--help` for the chain and the root:
    /// the tree lists neither visible aliases nor the built-in help flags.
    fn help_flags(&self, chain: &[String]) -> Vec<String> {
        static CACHE: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
        let key = chain.join(" ");
        let cache = CACHE.get_or_init(Mutex::default);
        if let Some(hit) = cache.lock().expect("cache").get(&key) {
            return hit.clone();
        }
        let option = Regex::new(r"(?m)^\s+(?:-\w, )?--([a-z0-9][a-z0-9-]*)").expect("regex");
        let alias = Regex::new(r"\[aliases?: ([^\]]*)\]").expect("regex");
        // Hidden aliases are only named in prose: "accepts --lang as alias".
        let prose_alias =
            Regex::new(r"(?i)(?:accepts|aliases?:?)\s+`?--([a-z0-9][a-z0-9-]*)").expect("regex");
        let mut flags = Vec::new();
        let mut invocations: Vec<Vec<&str>> = vec![vec!["--help"]];
        if !chain.is_empty() {
            let mut args: Vec<&str> = chain.iter().map(String::as_str).collect();
            args.push("--help");
            invocations.push(args);
        }
        for args in invocations {
            let (_, help) = run(&args);
            flags.extend(option.captures_iter(&help).map(|c| c[1].to_string()));
            flags.extend(prose_alias.captures_iter(&help).map(|c| c[1].to_string()));
            for list in alias.captures_iter(&help) {
                flags.extend(
                    list[1]
                        .split(',')
                        .filter_map(|a| a.trim().strip_prefix("--"))
                        .map(str::to_string),
                );
            }
        }
        cache.lock().expect("cache").insert(key, flags.clone());
        flags
    }
}

// --- reading one invocation -------------------------------------------------

/// What an `atomwrite ...` line says: the command path and the long flags.
struct Parsed {
    chain: Vec<String>,
    flags: Vec<(usize, String)>,
    /// A first word that looks like a command name but is not one.
    bad_command: Option<(usize, String)>,
}

fn looks_like_command(word: &str) -> bool {
    Regex::new(r"^[a-z][a-z0-9-]*$")
        .expect("regex")
        .is_match(word)
}

/// Words that follow the program name. Global flags (and the values they take)
/// are skipped to find the subcommand; every long flag is recorded.
fn parse(cli: &Cli, toks: &[Tok]) -> Parsed {
    let mut parsed = Parsed {
        chain: Vec::new(),
        flags: Vec::new(),
        bad_command: None,
    };
    let mut i = 0;
    let mut after_dashes = false;
    let mut next_may_be_sub = false;
    while i < toks.len() {
        let tok = &toks[i];
        i += 1;
        if after_dashes {
            continue;
        }
        let is_flag = !tok.quoted && tok.text.starts_with('-') && tok.text.len() > 1;
        if is_flag && tok.text == "--" {
            after_dashes = true;
        } else if is_flag && tok.text.starts_with("--") {
            next_may_be_sub = false;
            let (name, inline) = match tok.text[2..].split_once('=') {
                Some((n, _)) => (n, true),
                None => (&tok.text[2..], false),
            };
            parsed.flags.push((tok.line, name.to_string()));
            if !inline && cli.flag(&parsed.chain, name) == Some(true) {
                i += 1;
            }
        } else if is_flag {
            next_may_be_sub = false;
            if tok.text == "-j" {
                i += 1;
            }
        } else if parsed.chain.is_empty() {
            if cli.commands.contains_key(&tok.text) {
                parsed.chain.push(tok.text.clone());
                next_may_be_sub = true;
            } else if !tok.quoted
                && tok.text != "help"
                && !cli.schemas.contains(&tok.text)
                && looks_like_command(&tok.text)
            {
                parsed.bad_command = Some((tok.line, tok.text.clone()));
                break;
            }
        } else if next_may_be_sub {
            next_may_be_sub = false;
            let known = cli
                .cmd_at(&parsed.chain)
                .last()
                .is_some_and(|c| c.subs.contains_key(&tok.text));
            if known {
                parsed.chain.push(tok.text.clone());
            }
        }
    }
    parsed
}

/// Every parsed `atomwrite` invocation of the shell blocks of `text`.
fn shell_invocations(cli: &Cli, text: &str) -> Vec<Parsed> {
    doc_scan::scan(text)
        .fences
        .iter()
        .filter(|f| doc_scan::SHELL_INFOS.contains(&f.info.as_str()))
        .flat_map(|f| invocations(&f.body, f.first_line))
        .map(|toks| parse(cli, &toks))
        .collect()
}

/// Commands named by inline code spans: `` `read` `` or `` `atomwrite read ...` ``.
fn span_commands(cli: &Cli, text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (number, line) in doc_scan::scan(text).prose {
        for span in inline_code(&line).0 {
            let rest = span.trim();
            let rest = rest.strip_prefix("atomwrite").map_or(rest, str::trim_start);
            let toks = command_words(rest, 0, number);
            if let Some(name) = parse(cli, &toks).chain.first() {
                found.insert(name.clone());
            }
        }
    }
    found
}

// --- guard 1: every command is documented -----------------------------------

#[test]
fn g1_every_command_appears_in_each_command_document() {
    let cli = cli();
    let bad = each_doc(COMMAND_DOCS, |rel, text| {
        let mut named = span_commands(cli, text);
        named.extend(
            shell_invocations(cli, text)
                .into_iter()
                .filter_map(|p| p.chain.into_iter().next()),
        );
        let missing: Vec<&str> = cli
            .tools
            .iter()
            .map(String::as_str)
            .filter(|t| !named.contains(*t))
            .collect();
        if missing.is_empty() {
            Vec::new()
        } else {
            vec![format!(
                "{rel}: {} of {} commands missing from code spans and command lines: {}",
                missing.len(),
                cli.tools.len(),
                missing.join(", ")
            )]
        }
    });
    assert_clean("commands", &bad);
}

// --- guard 2: every cited flag exists ----------------------------------------

#[cfg(feature = "ast")]
#[test]
fn g2_every_flag_in_a_command_line_exists() {
    let cli = cli();
    let bad = each_doc(COMMAND_DOCS, |rel, text| {
        let mut out = Vec::new();
        for parsed in shell_invocations(cli, text) {
            if let Some((line, word)) = &parsed.bad_command {
                out.push(format!("{rel}:{line}: `{word}` is not a command"));
            }
            for (line, flag) in &parsed.flags {
                let known = cli.flag(&parsed.chain, flag).is_some()
                    || matches!(flag.as_str(), "help" | "version")
                    || cli.help_flags(&parsed.chain).iter().any(|f| f == flag);
                if !known {
                    out.push(format!(
                        "{rel}:{line}: --{flag} is not a flag of `{}`",
                        parsed.chain.join(" ")
                    ));
                }
            }
        }
        out
    });
    assert_clean("flags", &bad);
}

// --- guard 3: removed spellings ------------------------------------------------

/// Is `line` a `SIG_DFL` mention that does not explain the deviation?
fn sigpipe_regression(line: &str) -> bool {
    let lower = line.to_lowercase();
    line.contains("SIG_DFL")
        && !SIGPIPE_DEVIATION_MARKERS
            .iter()
            .any(|m| lower.contains(&m.to_lowercase()))
}

#[test]
fn g3_removed_spellings_are_gone() {
    let mcp = Regex::new(r"(?i)\bmcp\b").expect("regex");
    let ssr = Regex::new(r"(?i)atomwrite(?:\.exe)?\s+(?:-\S+\s+)*ssr\b|`ssr`").expect("regex");
    let exit_total = Regex::new(r"(?i)\b37\s+(?:exit\s+codes?|códigos?)").expect("regex");
    let all: Vec<&str> = COMMAND_DOCS.iter().chain(OTHER_DOCS).copied().collect();
    let mut bad = each_doc(&all, |rel, text| {
        let mut out = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let at = format!("{rel}:{}", i + 1);
            for spelling in REMOVED_SPELLINGS {
                if line.contains(spelling) {
                    out.push(format!("{at}: `{spelling}`"));
                }
            }
            if mcp.is_match(line) {
                out.push(format!("{at}: the word MCP"));
            }
            if ssr.is_match(line) {
                out.push(format!("{at}: `ssr` as a command"));
            }
            if exit_total.is_match(line) {
                out.push(format!("{at}: `37` exit codes"));
            }
        }
        out
    });
    let sigpipe: Vec<&str> = COMMAND_DOCS.iter().chain(SIGPIPE_DOCS).copied().collect();
    bad.extend(each_doc(&sigpipe, |rel, text| {
        text.lines()
            .enumerate()
            .filter(|(_, l)| sigpipe_regression(l))
            .map(|(i, _)| format!("{rel}:{}: SIG_DFL without the deviation explanation", i + 1))
            .collect()
    }));
    assert_clean("removed spellings", &bad);
}

// --- guard 4: English and Portuguese editions keep the same shape ---------------

/// `docs/AGENTS.md` -> `docs/AGENTS.pt-BR.md`, `llms.txt` -> `llms.pt-BR.txt`.
fn portuguese_path(english: &str) -> String {
    match english.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem}.pt-BR.{ext}"),
        None => format!("{english}.pt-BR"),
    }
}

#[test]
fn g4_english_and_portuguese_editions_have_the_same_shape() {
    let mut bad = Vec::new();
    for en in PAIRS {
        let pt = portuguese_path(en);
        let (en_text, pt_text) = match (load(en), load(&pt)) {
            (Ok(a), Ok(b)) => (a, b),
            (a, b) => {
                bad.extend([a.err(), b.err()].into_iter().flatten());
                continue;
            }
        };
        let (a, b) = (doc_scan::shape(&en_text), doc_scan::shape(&pt_text));
        let pairs = [
            ("H1", a.h1, b.h1),
            ("H2", a.h2, b.h2),
            ("H3", a.h3, b.h3),
            ("fenced blocks", a.fences, b.fences),
            ("table rows", a.table_rows, b.table_rows),
        ];
        for (what, x, y) in pairs {
            if x != y {
                bad.push(format!("{en} vs {pt}: {what} {x} != {y}"));
            }
        }
    }
    assert_clean("EN/pt-BR parity", &bad);
}

// --- guard 5: Portuguese keeps its accents ----------------------------------------

#[test]
fn g5_portuguese_documents_keep_their_accents() {
    let words = Regex::new(&format!(r"(?i)\b(?:{})\b", UNACCENTED_WORDS.join("|"))).expect("regex");
    let url = Regex::new(r"\]\([^)]*\)|https?://\S+|<[^>]*>").expect("regex");
    let files: Vec<String> = COMMAND_DOCS
        .iter()
        .chain(OTHER_DOCS)
        .filter(|f| f.contains(".pt-BR"))
        .map(|f| f.to_string())
        .collect();
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let bad = each_doc(&refs, |rel, text| {
        let mut out = Vec::new();
        for (number, line) in doc_scan::scan(text).prose {
            let (_, plain) = inline_code(&line);
            let plain = url.replace_all(&plain, " ");
            for m in words.find_iter(&plain) {
                out.push(format!("{rel}:{number}: `{}` has no accent", m.as_str()));
            }
        }
        out
    });
    assert_clean("accents", &bad);
}

// --- guard 6: links resolve ----------------------------------------------------------

/// Markdown files whose links are checked: the root, `docs/` and the ADR index.
fn link_docs() -> Vec<String> {
    let mut out = Vec::new();
    for dir in ["", "docs"] {
        let mut names: Vec<String> = std::fs::read_dir(root().join(dir))
            .unwrap_or_else(|e| panic!("cannot list {dir}: {e}"))
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".md"))
            .collect();
        names.sort();
        out.extend(names.into_iter().map(|n| {
            if dir.is_empty() {
                n
            } else {
                format!("{dir}/{n}")
            }
        }));
    }
    out.push("docs/decisions/README.md".to_string());
    out
}

#[test]
fn g6_relative_links_and_anchors_resolve() {
    let mut cache: HashMap<String, Vec<String>> = HashMap::new();
    let mut bad = Vec::new();
    let files = link_docs();
    for rel in &files {
        let text = match load(rel) {
            Ok(t) => t,
            Err(e) => {
                bad.push(e);
                continue;
            }
        };
        let dir = std::path::Path::new(rel)
            .parent()
            .map(PathBuf::from)
            .unwrap_or_default();
        for (line, target) in doc_scan::relative_links(&text) {
            let (path_part, anchor) = match target.split_once('#') {
                Some((p, a)) => (p, Some(doc_scan::percent_decode(a))),
                None => (target.as_str(), None),
            };
            let path_part = path_part.split('?').next().unwrap_or("");
            let resolved = if path_part.is_empty() {
                PathBuf::from(rel)
            } else if let Some(abs) = path_part.strip_prefix('/') {
                PathBuf::from(abs)
            } else {
                dir.join(path_part)
            };
            if !root().join(&resolved).exists() {
                bad.push(format!("{rel}:{line}: `{target}` points to a missing file"));
                continue;
            }
            let Some(anchor) = anchor.filter(|a| !a.is_empty()) else {
                continue;
            };
            if resolved.extension().is_none_or(|e| e != "md") || !root().join(&resolved).is_file() {
                continue;
            }
            let key = resolved.to_string_lossy().into_owned();
            let known = cache
                .entry(key.clone())
                .or_insert_with(|| anchors(&load(&key).unwrap_or_default()));
            if !known
                .iter()
                .any(|a| a == &anchor.to_lowercase() || a == &anchor)
            {
                bad.push(format!(
                    "{rel}:{line}: `{target}` has no heading `{anchor}` in {key}"
                ));
            }
        }
    }
    assert_clean("links", &bad);
}

// --- guard 7: current totals -----------------------------------------------------------

fn past_release(line: &str) -> bool {
    let versions = Regex::new(r"v?0\.1\.\d+").expect("regex");
    let mut seen = false;
    for m in versions.find_iter(line) {
        if m.as_str().trim_start_matches('v') == CURRENT_RELEASE.trim_start_matches('v') {
            return false;
        }
        seen = true;
    }
    seen
}

fn history_heading(line: &str) -> bool {
    let lower = line.to_lowercase();
    past_release(line)
        || ["history", "histórico", "changelog"]
            .iter()
            .any(|w| lower.contains(w))
}

/// Counts of at least [`TOTAL_CLAIM_MIN`] quoted by `re` as `(line, count)`,
/// skipping history: lines marked "historical" or naming only older releases,
/// and sections under a history heading.
fn claims(text: &str, re: &Regex) -> Vec<(usize, u64)> {
    claims_where(text, re, |_| true)
}

fn claims_where(text: &str, re: &Regex, keep: impl Fn(&str) -> bool) -> Vec<(usize, u64)> {
    let mut stack: Vec<bool> = Vec::new();
    let mut found = Vec::new();
    let scanned = doc_scan::scan(text);
    for (number, line) in scanned.prose {
        if let Some((level, _)) = heading(&line) {
            stack.resize(level - 1, false);
            stack.push(history_heading(&line));
        }
        let lower = line.to_lowercase();
        if stack.iter().any(|h| *h)
            || lower.contains("historical")
            || lower.contains("histórico")
            || past_release(&line)
        {
            continue;
        }
        if !keep(&line) {
            continue;
        }
        for cap in re.captures_iter(&line) {
            let n: u64 = cap[1].parse().expect("number");
            if n >= TOTAL_CLAIM_MIN {
                found.push((number, n));
            }
        }
    }
    found
}

#[test]
fn g7_quoted_totals_match_the_binary() {
    let cli = cli();
    let commands =
        Regex::new(r"\b(\d+)\*{0,2}\s+(?i:subcommands?|subcomandos?|commands?|comandos?)\b")
            .expect("regex");
    let schemas =
        Regex::new(r"\b(\d+)\*{0,2}\s+(?i:(?:json\s+|nomes\s+de\s+)?(?:schemas?|esquemas?))\b")
            .expect("regex");
    let name_totals = Regex::new(r"\b(\d+)\*{0,2}\s+(?i:names?|nomes?)\b").expect("regex");
    let file_totals = Regex::new(r"\b(\d+)\*{0,2}\s+(?i:files?|arquivos?)\b").expect("regex");
    let files: Vec<&str> = COMMAND_DOCS
        .iter()
        .chain(OTHER_DOCS)
        .chain(COUNT_DOCS_EXTRA)
        .copied()
        .filter(|f| !COUNT_DOCS_EXCLUDED.contains(f))
        .collect();
    let bad = each_doc(&files, |rel, text| {
        let mut out = Vec::new();
        for (re, what, truth) in [
            (&commands, "commands", cli.tools.len() as u64),
            (&schemas, "schemas", cli.schemas.len() as u64),
            (&name_totals, "names", cli.schemas.len() as u64),
            (
                &file_totals,
                "files",
                atomwrite::SCHEMA_FILE_NAMES.len() as u64,
            ),
        ] {
            // `names` and `files` are a schema claim only when the line says so:
            // "4000 files" elsewhere is a different subject, not this total.
            let keep = |l: &str| {
                if what == "commands" || what == "schemas" {
                    return true;
                }
                let lower = l.to_lowercase();
                lower.contains("schema") || lower.contains("esquema")
            };
            for (line, n) in claims_where(text, re, keep) {
                if n != truth {
                    out.push(format!(
                        "{rel}:{line}: says {n} {what}, the binary has {truth}"
                    ));
                }
            }
        }
        out
    });
    assert_clean("totals", &bad);
}

// --- the readers themselves --------------------------------------------------------------

#[test]
fn readers_follow_pipes_quotes_and_continuations() {
    let cli = cli();
    let block = "printf 'x' | atomwrite --workspace \"$WS\" write --dry-run f \\\n  --append # note --bogus\n\
                 CS=\"$(atomwrite hash a.txt | jaq -r .checksum)\"\ncargo install atomwrite --locked\n\
                 atomwrite config set fuzzy.mode auto --scope global && echo '--nope'\n";
    let runs: Vec<Parsed> = invocations(block, 1)
        .iter()
        .map(|t| parse(cli, t))
        .collect();
    let chains: Vec<String> = runs.iter().map(|p| p.chain.join(" ")).collect();
    assert_eq!(chains, ["write", "hash", "config set"]);
    let flags: Vec<&str> = runs[0].flags.iter().map(|(_, f)| f.as_str()).collect();
    assert_eq!(flags, ["workspace", "dry-run", "append"]);
    assert_eq!(runs[0].flags[2].0, 2, "continuation keeps the line number");
    let flags: Vec<&str> = runs[2].flags.iter().map(|(_, f)| f.as_str()).collect();
    assert_eq!(flags, ["scope"]);
}

#[test]
fn readers_flag_unknown_commands_and_skip_flag_values() {
    let cli = cli();
    let toks = command_words(" --old --not-a-flag ssr src/", 0, 1);
    let parsed = parse(cli, &toks);
    assert_eq!(parsed.bad_command, Some((1, "ssr".to_string())));
    let toks = command_words(" --workspace /x --select a,b read f", 0, 1);
    assert_eq!(parse(cli, &toks).chain, ["read"]);
}

#[test]
fn span_reader_counts_code_spans_only() {
    let cli = cli();
    let text = "Prose: write a file, then `read` it, or run `atomwrite --workspace X edit f`.\n";
    let named: Vec<String> = span_commands(cli, text).into_iter().collect();
    assert_eq!(named, ["edit", "read"]);
}

#[test]
fn flag_lookup_uses_the_command_tree() {
    let cli = cli();
    assert_eq!(cli.flag(&["read".to_string()], "head"), Some(true));
    assert_eq!(cli.flag(&["read".to_string()], "stat"), Some(false));
    assert_eq!(cli.flag(&["read".to_string()], "workspace"), Some(true));
    assert_eq!(cli.flag(&["read".to_string()], "no-such-flag"), None);
    assert!(
        cli.flag(&["config".to_string(), "set".to_string()], "scope")
            .is_some()
    );
}

#[test]
fn link_reader_skips_code_and_urls() {
    let text =
        "[a](x.md#top) `[b](code.md)` [c](https://e.org) [d](#here)\n```\n[e](fence.md)\n```\n";
    let found: Vec<String> = doc_scan::relative_links(text)
        .into_iter()
        .map(|(_, t)| t)
        .collect();
    assert_eq!(found, ["x.md#top", "#here"]);
}

#[test]
fn slugs_follow_github() {
    assert_eq!(
        github_slug("`semantic-merge` — merge"),
        "semantic-merge--merge"
    );
    assert_eq!(
        github_slug("Leia em Português (v2)!"),
        "leia-em-português-v2"
    );
    let text = "# A\n## Same\n## Same\n```\n## Not a heading\n```\n";
    assert_eq!(anchors(text), ["a", "same", "same-1"]);
}

#[test]
fn claims_skip_history_and_families() {
    let re = Regex::new(r"\b(\d+)\*{0,2}\s+(?i:subcommands?)\b").expect("regex");
    let cur = CURRENT_RELEASE;
    assert_eq!(claims("- **45** subcommands", &re), [(1, 45)]);
    assert!(claims("- Historical: 45 subcommands", &re).is_empty());
    assert!(claims("- In v0.1.29 there were 41 subcommands", &re).is_empty());
    assert!(claims("backup flag on 15 subcommands", &re).is_empty());
    assert!(claims("## Release History\n- 45 subcommands", &re).is_empty());
    assert_eq!(
        claims(&format!("## {cur}\n- 45 subcommands"), &re),
        [(2, 45)]
    );
}

#[test]
fn sigpipe_marker_accepts_only_the_deviation() {
    assert!(sigpipe_regression("SIGPIPE goes back to `SIG_DFL`"));
    assert!(!sigpipe_regression("SIGPIPE is NOT reset to `SIG_DFL`"));
    assert!(!sigpipe_regression("o SIGPIPE não volta a `SIG_DFL`"));
}

#[test]
fn fence_scan_keeps_shell_comments_out_of_headings() {
    let text = "# T\n```bash\n# comment\n```\n| a | b |\n| - | - |\n";
    let s = doc_scan::shape(text);
    assert_eq!((s.h1, s.fences, s.table_rows), (1, 1, 2));
}
