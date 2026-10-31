// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 red tests for Explicit Target Designation in the execution
//! envelope and for fail-closed usage errors (R14, R15, R16).
//!
//! R14: every successful mutating verb reports `target_source:"argv"` and an
//! absolute `target_resolved` on its terminal event.
//! R15: a mutating verb without an explicit target exits 2 and touches nothing.
//! R16: when a token shifts into another role, the verb never acts on the
//! ambient tree: it either acts on the path named in argv or exits 2.

mod common;

use assert_cmd::Command;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Per-process bound so a hung verb fails the test instead of the suite.
const PROC_TIMEOUT: Duration = Duration::from_secs(60);

struct Out {
    code: Option<i32>,
    stdout: String,
    events: Vec<Value>,
}

impl Out {
    fn last(&self) -> Option<&Value> {
        self.events.last()
    }
}

/// Run with `cwd` as the process directory and `ws` as the jail root.
fn run_in(cwd: &Path, ws: &Path, args: &[String], stdin: Option<&str>) -> Out {
    let mut cmd = common::hermetic(Command::cargo_bin("atomwrite").expect("binary"));
    cmd.timeout(PROC_TIMEOUT)
        .current_dir(cwd)
        .args([
            "--workspace",
            ws.to_str().expect("utf8 workspace"),
            "--no-progress",
            "--no-auto-heal",
        ])
        .args(args)
        .write_stdin(stdin.unwrap_or("").as_bytes().to_vec());
    let output = cmd.output().expect("run atomwrite");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let events = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();
    Out {
        code: output.status.code(),
        stdout,
        events,
    }
}

fn sv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

fn p(base: &Path, name: &str) -> String {
    base.join(name).to_str().expect("utf8 path").to_string()
}

/// Canonical form that also works for paths that no longer exist.
fn canon(path: &Path) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(path) {
        return c;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => std::fs::canonicalize(parent)
            .map(|c| c.join(name))
            .unwrap_or_else(|_| path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

/// Every file under `root` (relative path to bytes), recursively.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.filter_map(Result::ok) {
            let path = e.path();
            if path.is_dir() {
                out.insert(
                    format!("{}/", path.strip_prefix(base).unwrap().display()),
                    Vec::new(),
                );
                walk(base, &path, out);
            } else {
                let rel = path.strip_prefix(base).unwrap().display().to_string();
                out.insert(rel, std::fs::read(&path).unwrap_or_default());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

const RUST_SRC: &str = "// note\nfn main() {\n    let x = 1;\n    let foo_bar = 2;\n}\n";

/// Workspace laid out for the mutating verbs.
fn fixture() -> tempfile::TempDir {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    let w = |n: &str, c: &str| {
        let path = d.join(n);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, c).expect("write fixture");
    };
    w("c.toml", "[package]\nname = \"x\"\n\n[feat]\nk = 1\n");
    w("s.rs", RUST_SRC);
    w("id.rs", RUST_SRC);
    w("a.txt", "alpha beta\n");
    w("base.txt", "l1\nl2\nl3\n");
    w("ours.txt", "l1\nl2 ours\nl3\n");
    w("theirs.txt", "l1\nl2\nl3 theirs\n");
    w("del.txt", "to delete\n");
    w("d/one.rs", RUST_SRC);
    w("d/two.txt", "alpha two\n");
    w(
        "rules.yaml",
        "- id: r1\n  language: rust\n  pattern: \"let x = 1;\"\n  rewrite: \"let x = 2;\"\n",
    );
    ws
}

struct Case {
    verb: &'static str,
    args: Vec<String>,
    stdin: Option<&'static str>,
    prep: Vec<Vec<String>>,
    /// Paths (relative to the workspace) a correct `target_resolved` may name.
    accepted: Vec<&'static str>,
}

fn case(verb: &'static str, args: Vec<String>, accepted: &[&'static str]) -> Case {
    Case {
        verb,
        args,
        stdin: None,
        prep: Vec::new(),
        accepted: accepted.to_vec(),
    }
}

fn mutating_cases(d: &Path) -> Vec<Case> {
    let s = p(d, "s.rs");
    let dir = p(d, "d");
    let mut edit_loop = case("edit-loop", vec!["edit-loop".into(), s.clone()], &["s.rs"]);
    edit_loop.stdin = Some("[{\"old\":\"fn main\",\"new\":\"fn entry\"}]");
    let mut apply = case(
        "apply",
        vec![
            "apply".into(),
            s.clone(),
            "--format".into(),
            "full".into(),
            "--allow-shrink".into(),
        ],
        &["s.rs"],
    );
    apply.stdin = Some("x\n");
    let mut write = case("write", vec!["write".into(), p(d, "new.txt")], &["new.txt"]);
    write.stdin = Some("hello\n");
    let mut rollback = case(
        "rollback",
        vec!["rollback".into(), s.clone(), "--latest".into()],
        &["s.rs"],
    );
    rollback.prep.push(vec!["backup".into(), s.clone()]);
    let mut prune = case(
        "prune-backups",
        vec![
            "prune-backups".into(),
            "--max-count".into(),
            "1".into(),
            s.clone(),
        ],
        &["s.rs"],
    );
    prune.prep.push(vec!["backup".into(), s.clone()]);
    prune.prep.push(vec!["backup".into(), s.clone()]);
    vec![
        case(
            "edit",
            vec![
                "edit".into(),
                s.clone(),
                "--old".into(),
                "fn main".into(),
                "--new".into(),
                "fn entry".into(),
            ],
            &["s.rs"],
        ),
        write,
        edit_loop,
        case(
            "set",
            vec![
                "set".into(),
                p(d, "c.toml"),
                "package.name".into(),
                "y".into(),
            ],
            &["c.toml"],
        ),
        case(
            "del",
            vec!["del".into(), p(d, "c.toml"), "feat.k".into()],
            &["c.toml"],
        ),
        case(
            "delete",
            vec!["delete".into(), p(d, "del.txt")],
            &["del.txt"],
        ),
        case(
            "copy",
            vec!["copy".into(), s.clone(), p(d, "s_copy.rs")],
            &["s.rs", "s_copy.rs"],
        ),
        case(
            "move",
            vec!["move".into(), s.clone(), p(d, "s_moved.rs")],
            &["s.rs", "s_moved.rs"],
        ),
        apply,
        rollback,
        case(
            "case",
            vec![
                "case".into(),
                "--to".into(),
                "camel".into(),
                "--subvert".into(),
                "foo_bar".into(),
                "fooBar".into(),
                p(d, "id.rs"),
            ],
            &["id.rs"],
        ),
        case(
            "replace",
            vec![
                "replace".into(),
                "alpha".into(),
                "ALPHA".into(),
                dir.clone(),
            ],
            &["d"],
        ),
        case(
            "transform",
            vec![
                "transform".into(),
                "-p".into(),
                "let x = 1;".into(),
                "-r".into(),
                "let x = 2;".into(),
                "-l".into(),
                "rust".into(),
                dir.clone(),
            ],
            &["d"],
        ),
        case(
            "scope",
            vec![
                "scope".into(),
                "--lang".into(),
                "rust".into(),
                "--query".into(),
                "comments".into(),
                "--delete".into(),
                dir.clone(),
            ],
            &["d"],
        ),
        case(
            "codemod",
            vec!["codemod".into(), "--rules".into(), p(d, "rules.yaml"), dir],
            &["d"],
        ),
        case("backup", vec!["backup".into(), p(d, "a.txt")], &["a.txt"]),
        case(
            "semantic-merge",
            vec![
                "semantic-merge".into(),
                "--base".into(),
                p(d, "base.txt"),
                "--ours".into(),
                p(d, "ours.txt"),
                "--theirs".into(),
                p(d, "theirs.txt"),
                "--output".into(),
                p(d, "merged.txt"),
            ],
            &["merged.txt"],
        ),
        prune,
        // The workspace scope keeps the run inside the fixture; the global scope
        // is covered, with HOME redirected, by cli_v0137_config_verb.
        case(
            "config set",
            vec![
                "config".into(),
                "set".into(),
                "fuzzy.threshold".into(),
                "0.9".into(),
                "--scope".into(),
                "workspace".into(),
            ],
            &[".atomwrite.toml"],
        ),
        case(
            "config unset",
            vec![
                "config".into(),
                "unset".into(),
                "fuzzy.threshold".into(),
                "--scope".into(),
                "workspace".into(),
            ],
            &[".atomwrite.toml"],
        ),
    ]
}

// RED until R14: the terminal event of every successful mutating verb carries
// `target_source:"argv"` and the absolute resolved target; today none does.
#[cfg(feature = "ast")]
#[test]
fn r14_mutating_verbs_report_target_source_and_target_resolved() {
    let mut failures = Vec::new();
    for idx in 0.. {
        let ws = fixture();
        let d = ws.path();
        let cases = mutating_cases(d);
        let Some(case) = cases.get(idx) else { break };
        for prep in &case.prep {
            let prepped = run_in(d, d, prep, None);
            assert_eq!(
                prepped.code,
                Some(0),
                "prep for {} failed: {:.200}",
                case.verb,
                prepped.stdout
            );
        }
        let out = run_in(d, d, &case.args, case.stdin);
        if out.code != Some(0) {
            failures.push(format!(
                "{}: success case exited {:?}: {:.200}",
                case.verb, out.code, out.stdout
            ));
            continue;
        }
        let Some(last) = out.last() else {
            failures.push(format!("{}: empty stdout", case.verb));
            continue;
        };
        if last["target_source"] != "argv" {
            failures.push(format!(
                "{}: target_source is {} (want \"argv\") in {:.140}",
                case.verb,
                last["target_source"],
                last.to_string()
            ));
            continue;
        }
        let claimed = last["target_resolved"].as_str().map(PathBuf::from);
        match claimed {
            Some(c) if c.is_absolute() => {
                let ok = case
                    .accepted
                    .iter()
                    .any(|rel| canon(&c) == canon(&d.join(rel)));
                if !ok {
                    failures.push(format!(
                        "{}: target_resolved {} is not one of {:?}",
                        case.verb,
                        c.display(),
                        case.accepted
                    ));
                }
            }
            other => failures.push(format!(
                "{}: target_resolved missing or relative: {other:?}",
                case.verb
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "{} verbs violate R14:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// `locale --set` and `--clear` change one fixed file in the XDG config
/// directory, outside the jail: the terminal event names it as an argv target.
/// A plain `locale` only reads and stamps nothing.
#[test]
fn locale_set_and_clear_stamp_the_preference_file() {
    let ws = fixture();
    let d = ws.path();
    for args in [sv(&["locale", "--set", "en"]), sv(&["locale", "--clear"])] {
        let out = run_in(d, d, &args, None);
        assert_eq!(out.code, Some(0), "{args:?}: {:.300}", out.stdout);
        let last = out.last().expect("terminal event");
        assert_eq!(last["target_source"], "argv", "{args:?}: {last}");
        let target = PathBuf::from(last["target_resolved"].as_str().expect("target_resolved"));
        assert!(target.is_absolute(), "{args:?}: {target:?}");
        assert_eq!(
            target.file_name().and_then(|n| n.to_str()),
            Some(atomwrite::constants::LOCALE_PREFERENCE_FILE),
            "{args:?}: {target:?}"
        );
    }
    let read_only = run_in(d, d, &sv(&["locale"]), None);
    let last = read_only.last().expect("terminal event");
    assert!(last.get("target_resolved").is_none(), "read-only: {last}");
}

/// Ambient tree where every wrongly-targeted mutation would be visible.
fn ambient() -> tempfile::TempDir {
    let ws = tempfile::tempdir().expect("ambient workspace");
    let d = ws.path();
    std::fs::write(d.join("foo.txt"), "foo and foo\n").expect("fixture");
    std::fs::write(d.join("a.rs"), "// foo comment\nfn foo() { let x = 1; }\n").expect("fixture");
    std::fs::write(d.join("keep.bak.20260101_000000"), "old backup\n").expect("fixture");
    std::fs::write(
        d.join("rules.yaml"),
        "- id: r1\n  language: rust\n  pattern: \"let x = 1;\"\n  rewrite: \"let x = 2;\"\n",
    )
    .expect("fixture");
    ws
}

struct UsageCase {
    name: &'static str,
    args: Vec<String>,
    /// Contract assumed from the phase brief (the verb has no target argument today).
    assumed: bool,
}

fn usage_cases() -> Vec<UsageCase> {
    let uc = |name, args: &[&str], assumed| UsageCase {
        name,
        args: sv(args),
        assumed,
    };
    vec![
        uc(
            "replace foo src/ (two args, no path)",
            &["replace", "foo", "src/"],
            false,
        ),
        uc(
            "replace foo bar (no path)",
            &["replace", "foo", "bar"],
            false,
        ),
        uc(
            "scope without path",
            &["scope", "--lang", "rust", "--query", "comments", "--delete"],
            false,
        ),
        uc(
            "codemod without path",
            &["codemod", "--rules", "rules.yaml"],
            false,
        ),
        uc(
            "case --subvert without path",
            &["case", "--subvert", "foo", "qux"],
            false,
        ),
        uc(
            "case --subvert --dry-run without path",
            &["case", "--subvert", "foo", "qux", "--dry-run"],
            false,
        ),
        uc("wal-heal without target", &["wal-heal"], true),
        uc("delete without path", &["delete"], false),
        uc("backup without path", &["backup"], false),
        uc(
            "prune-backups without path",
            &["prune-backups", "--max-count", "1"],
            false,
        ),
        uc("scratch init without target", &["scratch", "init"], true),
        uc(
            "config set without --scope",
            &["config", "set", "fuzzy.threshold", "0.9"],
            false,
        ),
        uc(
            "config unset without --scope",
            &["config", "unset", "fuzzy.threshold"],
            false,
        ),
        uc(
            "recipe run without --path",
            &[
                "recipe",
                "run",
                "--name",
                "search-replace-verify",
                "--pattern",
                "foo",
                "--replacement",
                "bar",
            ],
            false,
        ),
    ]
}

// RED until R15: every mutating verb without an explicit target exits 2 and
// leaves the ambient tree byte-identical. `wal-heal` and `scratch init` have no
// target argument today, so their entries encode the contract of the brief.
#[test]
fn r15_mutating_verbs_without_target_exit_2_and_touch_nothing() {
    let mut failures = Vec::new();
    for case in usage_cases() {
        let ws = ambient();
        let d = ws.path();
        let before = snapshot(d);
        let out = run_in(d, d, &case.args, None);
        let after = snapshot(d);
        let tag = if case.assumed {
            " [assumed contract]"
        } else {
            ""
        };
        if after != before {
            let changed: Vec<&String> = after
                .keys()
                .chain(before.keys())
                .filter(|k| before.get(*k) != after.get(*k))
                .collect();
            failures.push(format!(
                "{}{tag}: touched the ambient tree: {changed:?}",
                case.name
            ));
        }
        if out.code != Some(2) {
            failures.push(format!(
                "{}{tag}: exit {:?}, want 2; stdout {:.120}",
                case.name, out.code, out.stdout
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} usage gaps:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Shifted-token invocations: `Y` is an existing directory named in argv but
/// the line lacks one token, so `Y` lands in a non-target role.
fn shifted_cases(y: &str) -> Vec<(&'static str, Vec<String>)> {
    let l = |parts: Vec<&str>| parts.into_iter().map(str::to_string).collect::<Vec<_>>();
    vec![
        ("replace foo Y", l(vec!["replace", "foo", y])),
        ("replace -F foo Y", l(vec!["replace", "-F", "foo", y])),
        (
            "replace --no-backup foo Y",
            l(vec!["replace", "--no-backup", "foo", y]),
        ),
        (
            "transform -p -r Y (no path)",
            l(vec!["transform", "-p", "let x = 1;", "-r", y, "-l", "rust"]),
        ),
        (
            "scope --replace-with Y (no path)",
            l(vec![
                "scope",
                "--lang",
                "rust",
                "--query",
                "comments",
                "--replace-with",
                y,
            ]),
        ),
        (
            "codemod --rules Y (no path)",
            l(vec!["codemod", "--rules", y]),
        ),
        (
            "case --subvert foo Y",
            l(vec!["case", "--subvert", "foo", y]),
        ),
        (
            "case --subvert foo qux --to Y",
            l(vec!["case", "--subvert", "foo", "qux", "--to", y]),
        ),
        (
            "recipe run --replacement Y (no --path)",
            l(vec![
                "recipe",
                "run",
                "--name",
                "search-replace-verify",
                "--pattern",
                "foo",
                "--replacement",
                y,
            ]),
        ),
        (
            "backup --output-dir Y (no path)",
            l(vec!["backup", "--output-dir", y]),
        ),
        (
            "prune-backups --max-count Y",
            l(vec!["prune-backups", "--max-count", y]),
        ),
        (
            "delete --older-than Y (no path)",
            l(vec!["delete", "--older-than", y]),
        ),
        ("copy Y (no destination)", l(vec!["copy", y])),
        ("move Y (no destination)", l(vec!["move", y])),
        (
            "edit --old foo --new Y (no path)",
            l(vec!["edit", "--old", "foo", "--new", y]),
        ),
        (
            "config set --scope workspace Y (value missing)",
            l(vec!["config", "set", "--scope", "workspace", y]),
        ),
        (
            "config unset --scope Y (scope shifted)",
            l(vec!["config", "unset", "--scope", y]),
        ),
        (
            "config set --scope Y (scope shifted)",
            l(vec![
                "config",
                "set",
                "fuzzy.threshold",
                "0.9",
                "--scope",
                y,
            ]),
        ),
        ("apply (no file)", l(vec!["apply"])),
        ("edit-loop (no file)", l(vec!["edit-loop"])),
        (
            "set --no-backup Y (key/value missing)",
            l(vec!["set", "--no-backup", y]),
        ),
        ("del Y (key missing)", l(vec!["del", y])),
        (
            "rollback --timestamp Y (no path)",
            l(vec!["rollback", "--timestamp", y]),
        ),
    ]
}

// RED until R15/R16: when a token shifts (a path swallowed as a pattern,
// replacement, rules file, ...), the verb must NEVER act on the ambient tree X:
// it either acts on the path Y named in argv (reporting target_source "argv")
// or fails closed with exit 2.
#[test]
fn r16_shifted_tokens_never_act_on_the_ambient_tree() {
    let mut failures = Vec::new();
    for idx in 0.. {
        let root = tempfile::tempdir().expect("root");
        let x = root.path().join("x");
        let y = root.path().join("y");
        std::fs::create_dir_all(&x).expect("mkdir x");
        std::fs::create_dir_all(&y).expect("mkdir y");
        std::fs::write(x.join("foo.txt"), "foo and foo\n").expect("fixture");
        std::fs::write(x.join("a.rs"), "// foo comment\nfn foo() { let x = 1; }\n")
            .expect("fixture");
        std::fs::write(x.join("c.toml"), "[package]\nname = \"foo\"\n").expect("fixture");
        std::fs::write(x.join("keep.bak.20260101_000000"), "old backup\n").expect("fixture");
        std::fs::write(y.join("y.txt"), "foo in y\n").expect("fixture");
        let y_arg = y.to_str().expect("utf8").to_string();
        let cases = shifted_cases(&y_arg);
        let Some((name, args)) = cases.get(idx) else {
            break;
        };
        let before = snapshot(&x);
        // Ambient authority: cwd is X, and the jail root is the common parent.
        let out = run_in(&x, root.path(), args, Some(""));
        let after = snapshot(&x);
        if after != before {
            let changed: Vec<&String> = after
                .keys()
                .chain(before.keys())
                .filter(|k| before.get(*k) != after.get(*k))
                .collect();
            failures.push(format!("{name}: modified the ambient tree X: {changed:?}"));
            continue;
        }
        let acted_on_argv = out.code == Some(0)
            && out.events.iter().any(|e| e["target_source"] == "argv")
            && !out.events.iter().any(|e| e["target_source"] == "workspace");
        if out.code != Some(2) && !acted_on_argv {
            failures.push(format!(
                "{name}: neither exit 2 nor an argv-targeted success: exit {:?}, stdout {:.140}",
                out.code, out.stdout
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} shifted-token gaps:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
