// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 red tests for the output contract: terminal `ok:true`, checksum
//! verification envelope, no `null` top-level values, `edit-loop` atomicity,
//! honest `comments_preserved`, served schemas and `scope` failures
//! (R6, R6b, R8, R9, R10, R11, R13, R24).
//!
//! Each test aggregates every failing verb into one assertion message so a
//! single run lists all the gaps instead of stopping at the first one.

mod common;

use assert_cmd::Command;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
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

fn run_with(ws: &Path, global: &[&str], args: &[String], stdin: Option<&str>) -> Out {
    let mut cmd = common::hermetic(Command::cargo_bin("atomwrite").expect("binary"));
    cmd.timeout(PROC_TIMEOUT)
        .args([
            "--workspace",
            ws.to_str().expect("utf8 workspace"),
            "--no-progress",
            "--no-auto-heal",
        ])
        .args(global)
        .args(args);
    cmd.write_stdin(stdin.unwrap_or("").as_bytes().to_vec());
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

fn run(ws: &Path, args: &[String], stdin: Option<&str>) -> Out {
    run_with(ws, &[], args, stdin)
}

fn sv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

/// Absolute path of `name` inside the workspace, as a string argument.
fn p(ws: &Path, name: &str) -> String {
    ws.join(name).to_str().expect("utf8 path").to_string()
}

/// Populate a workspace with the files every case below relies on.
fn fixture() -> tempfile::TempDir {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    let w = |n: &str, c: &str| std::fs::write(d.join(n), c).expect("write fixture");
    w(
        "c.toml",
        "[package]\nname = \"x\" # inline note\nversion = \"1.0.0\"\n\n[feat]\nk = 1\n",
    );
    w("s.rs", "fn a() {}\nfn b() { let x = 1; }\nstruct S;\n");
    w("a.txt", "alpha beta\n");
    w("base.txt", "l1\nl2\nl3\n");
    w("ours.txt", "l1\nl2 ours\nl3\n");
    w("theirs.txt", "l1\nl2\nl3 theirs\n");
    w("conf.txt", "l1\nl2 THEIRS\nl3\n");
    w(
        "rules.yaml",
        "- id: r1\n  language: rust\n  pattern: \"let x = 1;\"\n  rewrite: \"let x = 2;\"\n",
    );
    ws
}

fn merge_args(ws: &Path, theirs: &str, out: &str, extra: &[&str]) -> Vec<String> {
    let mut v = sv(&["semantic-merge", "--base"]);
    v.push(p(ws, "base.txt"));
    v.push("--ours".into());
    v.push(p(ws, "ours.txt"));
    v.push("--theirs".into());
    v.push(p(ws, theirs));
    v.push("--output".into());
    v.push(p(ws, out));
    v.extend(extra.iter().map(|s| s.to_string()));
    v
}

struct Case {
    name: &'static str,
    args: Vec<String>,
    stdin: Option<&'static str>,
    /// Commands run first, in order, to build the state the case needs.
    prep: Vec<Vec<String>>,
}

fn case(name: &'static str, args: Vec<String>) -> Case {
    Case {
        name,
        args,
        stdin: None,
        prep: Vec::new(),
    }
}

fn success_cases(ws: &Path) -> Vec<Case> {
    let s = p(ws, "s.rs");
    let c = p(ws, "c.toml");
    let mut cases = vec![
        case("get", vec!["get".into(), c.clone(), "package.name".into()]),
        case(
            "set",
            vec!["set".into(), c.clone(), "package.name".into(), "y".into()],
        ),
        case(
            "del --force-missing",
            vec![
                "del".into(),
                "--force-missing".into(),
                c.clone(),
                "nope.key".into(),
            ],
        ),
        case("copy", vec!["copy".into(), s.clone(), p(ws, "s_copy.rs")]),
        case("move", vec!["move".into(), s.clone(), p(ws, "s_moved.rs")]),
        case("outline", vec!["outline".into(), s.clone()]),
        case(
            "query --kinds",
            vec!["query".into(), "--kinds".into(), s.clone()],
        ),
        case(
            "sparse list",
            vec![
                "sparse".into(),
                "list".into(),
                p(ws, "."),
                "--max-files".into(),
                "5".into(),
            ],
        ),
        case(
            "sparse outline",
            vec![
                "sparse".into(),
                "outline".into(),
                p(ws, "."),
                "--max-files".into(),
                "5".into(),
            ],
        ),
        case(
            "semantic-merge",
            merge_args(ws, "theirs.txt", "m1.txt", &[]),
        ),
        case(
            "semantic-merge --no-conflict-markers",
            merge_args(ws, "theirs.txt", "m2.txt", &["--no-conflict-markers"]),
        ),
        case("wal-heal", vec!["wal-heal".into(), p(ws, "")]),
        case("backup", vec!["backup".into(), s.clone()]),
        case("delete", vec!["delete".into(), p(ws, "a.txt")]),
        case(
            "read",
            vec!["read".into(), s.clone(), "--head".into(), "1".into()],
        ),
        case("hash", vec!["hash".into(), s.clone()]),
        case(
            "edit",
            vec![
                "edit".into(),
                s.clone(),
                "--old".into(),
                "fn a".into(),
                "--new".into(),
                "fn aa".into(),
            ],
        ),
    ];
    let mut edit_loop = case("edit-loop", vec!["edit-loop".into(), s.clone()]);
    edit_loop.stdin = Some("[{\"old\":\"fn a\",\"new\":\"fn aa\"}]");
    cases.push(edit_loop);
    // `--allow-shrink` is part of the case: a whole-file replacement that
    // shrinks the target past the configured percent is default-deny, exactly
    // as in `write`.
    let mut apply = case(
        "apply",
        vec![
            "apply".into(),
            s.clone(),
            "--format".into(),
            "full".into(),
            "--allow-shrink".into(),
            // v0.1.38 refuses a patch that grows the parse error count. This
            // case intentionally writes the text "x" into a .rs fixture, so it
            // opts out the way a caller would.
            "--no-syntax-check".into(),
        ],
    );
    apply.stdin = Some("x\n");
    cases.push(apply);
    let mut write = case("write", vec!["write".into(), p(ws, "new.txt")]);
    write.stdin = Some("hello\n");
    cases.push(write);
    let mut rollback = case(
        "rollback",
        vec!["rollback".into(), s.clone(), "--latest".into()],
    );
    rollback.prep.push(vec!["backup".into(), s]);
    cases.push(rollback);
    cases
}

// RED until R6: every successful verb must end its stdout with an event that
// carries `ok:true`; today get/del/copy/move/outline/query/sparse/
// semantic-merge/edit-loop/wal-heal end without it.
#[cfg(feature = "ast")]
#[test]
fn r6_terminal_event_carries_ok_true() {
    let mut failures = Vec::new();
    for idx in 0.. {
        let ws = fixture();
        let cases = success_cases(ws.path());
        let Some(case) = cases.get(idx) else { break };
        for prep in &case.prep {
            let prepped = run(ws.path(), prep, None);
            assert_eq!(prepped.code, Some(0), "prep for {} failed", case.name);
        }
        let out = run(ws.path(), &case.args, case.stdin);
        if out.code != Some(0) {
            failures.push(format!(
                "{}: unexpected exit {:?}: {:.200}",
                case.name, out.code, out.stdout
            ));
            continue;
        }
        match out.last() {
            Some(last) if last["ok"] == Value::Bool(true) => {}
            Some(last) => failures.push(format!(
                "{}: terminal event lacks ok:true: {:.160}",
                case.name,
                last.to_string()
            )),
            None => failures.push(format!("{}: empty stdout", case.name)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} verbs violate R6:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// Guard (passes today and must keep passing): stream events never carry `ok`.
#[cfg(feature = "ast")]
#[test]
fn r6_stream_events_do_not_carry_ok() {
    const STREAM_TYPES: [&str; 6] = [
        "begin",
        "end",
        "match",
        "outline_item",
        "query_kind",
        "sparse_entry",
    ];
    let ws = fixture();
    let d = ws.path();
    let s = p(d, "s.rs");
    let runs: Vec<Vec<String>> = vec![
        vec!["search".into(), "fn".into(), p(d, ".")],
        vec!["outline".into(), s.clone()],
        vec!["query".into(), "--kinds".into(), s],
        vec![
            "sparse".into(),
            "list".into(),
            p(d, "."),
            "--max-files".into(),
            "5".into(),
        ],
    ];
    let mut bad = Vec::new();
    let mut seen = BTreeSet::new();
    for args in &runs {
        for e in run(d, args, None).events {
            let t = e["type"].as_str().unwrap_or("").to_string();
            if STREAM_TYPES.contains(&t.as_str()) {
                seen.insert(t.clone());
                if e.get("ok").is_some() {
                    bad.push(format!("{t} event has an `ok` key: {:.120}", e.to_string()));
                }
            }
        }
    }
    assert_eq!(
        seen.len(),
        STREAM_TYPES.len(),
        "fixture must emit every stream type, saw {seen:?}"
    );
    assert!(bad.is_empty(), "stream events with ok:\n{}", bad.join("\n"));
}

// RED until R6b: a conflicting merge with --fail-on-conflict must not report
// ok:true on the `semantic_merge` event; it must say ok:false, exit 84,
// and end with the MERGE_CONFLICT error.
#[test]
fn r6b_semantic_merge_conflict_is_not_ok() {
    let ws = fixture();
    let out = run(
        ws.path(),
        &merge_args(ws.path(), "conf.txt", "mc.txt", &["--fail-on-conflict"]),
        None,
    );
    assert_eq!(out.code, Some(84), "exit code, stdout: {:.300}", out.stdout);
    let merge = out
        .events
        .iter()
        .find(|e| e["type"] == "semantic_merge")
        .unwrap_or_else(|| panic!("no semantic_merge event in {:.300}", out.stdout));
    assert_eq!(
        merge["status"], "conflict",
        "fixture must conflict: {merge}"
    );
    assert_eq!(
        merge["ok"],
        Value::Bool(false),
        "conflict event must carry ok:false: {merge}"
    );
    let err = out.last().expect("a final event");
    assert_eq!(err["code"], "MERGE_CONFLICT", "last event: {err}");
}

// RED until R8: a failed --verify-checksum must exit 81 AND explain itself on
// stdout with a CHECKSUM_VERIFY_FAILED envelope; today stdout is empty.
#[test]
fn r8_read_verify_checksum_mismatch_emits_envelope() {
    let ws = fixture();
    let out = run(
        ws.path(),
        &[
            "read".into(),
            "--verify-checksum".into(),
            "deadbeef".into(),
            p(ws.path(), "s.rs"),
        ],
        None,
    );
    assert_eq!(out.code, Some(81), "exit code, stdout: {:.200}", out.stdout);
    let env = out
        .events
        .iter()
        .find(|e| e["code"] == "CHECKSUM_VERIFY_FAILED")
        .unwrap_or_else(|| {
            panic!(
                "no CHECKSUM_VERIFY_FAILED envelope on stdout: {:?}",
                out.stdout
            )
        });
    assert_eq!(
        env["ok"],
        Value::Bool(false),
        "envelope must carry ok:false: {env}"
    );
}

// RED until R9: no event may carry a top-level `null` (omit the key instead).
#[test]
fn r9_no_null_top_level_values() {
    let ws = fixture();
    let d = ws.path();
    let c = p(d, "c.toml");
    let runs: Vec<(&str, Vec<String>)> = vec![
        ("get", vec!["get".into(), c.clone(), "package.name".into()]),
        (
            "del --force-missing (absent key)",
            vec![
                "del".into(),
                "--force-missing".into(),
                c.clone(),
                "nope.key".into(),
            ],
        ),
        (
            "set (new key)",
            vec!["set".into(), c.clone(), "feat.newkey".into(), "5".into()],
        ),
        (
            "set (existing key)",
            vec!["set".into(), c, "package.name".into(), "y".into()],
        ),
        ("locale", sv(&["locale"])),
        ("config list", sv(&["config", "list"])),
        (
            "config get (key with no value)",
            sv(&["config", "get", "storage.home"]),
        ),
        (
            "config set",
            sv(&[
                "config",
                "set",
                "fuzzy.threshold",
                "0.9",
                "--scope",
                "workspace",
            ]),
        ),
        (
            "config unset",
            sv(&["config", "unset", "fuzzy.threshold", "--scope", "workspace"]),
        ),
        (
            "semantic-merge (clean)",
            merge_args(d, "theirs.txt", "m1.txt", &[]),
        ),
        (
            "semantic-merge (conflict)",
            merge_args(d, "conf.txt", "m2.txt", &["--fail-on-conflict"]),
        ),
    ];
    let mut bad = Vec::new();
    for (name, args) in &runs {
        let out = run(d, args, None);
        assert!(!out.events.is_empty(), "{name}: no stdout events");
        for e in &out.events {
            if let Some(obj) = e.as_object() {
                for (k, v) in obj {
                    if v.is_null() {
                        bad.push(format!("{name}: `{k}` is null in {:.100}", e.to_string()));
                    }
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} null values:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

fn write_pairs_file(ws: &Path, content: &str) -> String {
    let path = p(ws, "target.txt");
    std::fs::write(&path, content).expect("write target");
    path
}

fn find_key(v: &Value, key: &str) -> bool {
    match v {
        Value::Object(m) => m.contains_key(key) || m.values().any(|x| find_key(x, key)),
        Value::Array(a) => a.iter().any(|x| find_key(x, key)),
        _ => false,
    }
}

const PAIRS_ONE_MISS: &str =
    "[{\"old\":\"alpha\",\"new\":\"ALPHA\"},{\"old\":\"zzz_absent\",\"new\":\"y\"}]";

// RED until R13: without --partial a failed pair aborts the whole batch with
// exit 65, leaves the file untouched, and the error carries `best_candidate`.
#[test]
fn r13_edit_loop_unmatched_pair_aborts_without_partial() {
    let ws = fixture();
    let target = write_pairs_file(ws.path(), "alpha beta\n");
    let out = run(
        ws.path(),
        &["edit-loop".into(), target.clone()],
        Some(PAIRS_ONE_MISS),
    );
    assert_eq!(out.code, Some(65), "exit code, stdout: {:.400}", out.stdout);
    assert_eq!(
        std::fs::read_to_string(&target).expect("read target"),
        "alpha beta\n",
        "no pair may be written when one does not match"
    );
    assert!(
        out.events.iter().any(|e| find_key(e, "best_candidate")),
        "the error must expose best_candidate: {:.400}",
        out.stdout
    );
}

// RED until R13: with --partial only the matching pair is written and the
// exit code is 87 / EDIT_LOOP_PARTIAL; no locale may suggest `--allow-partial`.
#[test]
fn r13_edit_loop_partial_writes_matches_and_exits_87() {
    for locale in ["en", "pt-BR"] {
        let ws = fixture();
        let target = write_pairs_file(ws.path(), "alpha beta\n");
        let out = run_with(
            ws.path(),
            &["--locale", locale],
            &["edit-loop".into(), "--partial".into(), target.clone()],
            Some(PAIRS_ONE_MISS),
        );
        assert_eq!(
            out.code,
            Some(87),
            "[{locale}] exit code, stdout: {:.400}",
            out.stdout
        );
        assert_eq!(
            std::fs::read_to_string(&target).expect("read target"),
            "ALPHA beta\n",
            "[{locale}] only the matching pair is written"
        );
        let last = out.last().expect("final event");
        assert_eq!(
            last["code"], "EDIT_LOOP_PARTIAL",
            "[{locale}] last event: {last}"
        );
        assert!(
            !out.stdout.contains("--allow-partial"),
            "[{locale}] the suggestion must not cite the non-existent --allow-partial: {:.400}",
            out.stdout
        );
    }
}

// RED until R13: the failure suggestion without --partial must not cite
// `--allow-partial` either (the flag is `--partial`).
#[test]
fn r13_edit_loop_error_suggestion_never_cites_allow_partial() {
    for locale in ["en", "pt-BR"] {
        let ws = fixture();
        let target = write_pairs_file(ws.path(), "alpha beta\n");
        let out = run_with(
            ws.path(),
            &["--locale", locale],
            &["edit-loop".into(), target],
            Some(PAIRS_ONE_MISS),
        );
        assert!(
            !out.events.is_empty(),
            "[{locale}] expected an error envelope"
        );
        assert!(
            !out.stdout.contains("--allow-partial"),
            "[{locale}] suggestion cites --allow-partial: {:.500}",
            out.stdout
        );
    }
}

// Control (must PASS today): every pair matches, exit 0, file rewritten.
#[test]
fn r13_edit_loop_all_pairs_match_exits_0() {
    let ws = fixture();
    let target = write_pairs_file(ws.path(), "alpha beta\n");
    let out = run(
        ws.path(),
        &["edit-loop".into(), target.clone()],
        Some("[{\"old\":\"alpha\",\"new\":\"ALPHA\"},{\"old\":\"beta\",\"new\":\"BETA\"}]"),
    );
    assert_eq!(out.code, Some(0), "exit code, stdout: {:.400}", out.stdout);
    assert_eq!(
        std::fs::read_to_string(&target).expect("read"),
        "ALPHA BETA\n"
    );
}

// RED until R13: with --partial and NO pair matching the run is a plain
// NO_MATCHES (exit 1), not a partial success.
#[test]
fn r13_edit_loop_partial_with_zero_matches_exits_1() {
    let ws = fixture();
    let target = write_pairs_file(ws.path(), "alpha beta\n");
    let out = run(
        ws.path(),
        &["edit-loop".into(), "--partial".into(), target.clone()],
        Some("[{\"old\":\"zzz1\",\"new\":\"a\"},{\"old\":\"zzz2\",\"new\":\"b\"}]"),
    );
    assert_eq!(out.code, Some(1), "exit code, stdout: {:.400}", out.stdout);
    let last = out.last().expect("final event");
    assert_eq!(last["code"], "NO_MATCHES", "last event: {last}");
    let failed = out.events.iter().filter(|e| e["ok"] == false).count();
    assert_eq!(failed, 1, "one terminal event, got {:?}", out.events);
    assert_eq!(
        std::fs::read_to_string(&target).expect("read"),
        "alpha beta\n"
    );
}

fn comments_of(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.find('#').map(|i| l[i..].trim().to_string()))
        .collect()
}

// RED until R24: setting a key must keep the key's trailing inline comment on
// the same line, and `comments_preserved` must then be true.
#[test]
fn r24_set_keeps_inline_comment_on_the_same_line() {
    let ws = fixture();
    let c = p(ws.path(), "c.toml");
    let out = run(
        ws.path(),
        &["set".into(), c.clone(), "package.name".into(), "y".into()],
        None,
    );
    assert_eq!(out.code, Some(0), "exit code, stdout: {:.300}", out.stdout);
    let text = std::fs::read_to_string(&c).expect("read toml");
    assert!(
        text.lines()
            .any(|l| l.starts_with("name = \"y\"") && l.contains("# inline note")),
        "inline comment lost or moved; file is now:\n{text}"
    );
    assert_eq!(
        out.last().expect("event")["comments_preserved"],
        Value::Bool(true),
        "comments_preserved must be true when the comment survived"
    );
}

// RED until R24: `comments_preserved` must be honest in both directions: true
// only when every comment of the original file is still present afterwards.
#[test]
fn r24_comments_preserved_matches_reality() {
    let ws = fixture();
    let c = p(ws.path(), "c.toml");
    let before = comments_of(&std::fs::read_to_string(&c).expect("read"));
    let out = run(
        ws.path(),
        &["set".into(), c.clone(), "package.name".into(), "y".into()],
        None,
    );
    assert_eq!(out.code, Some(0), "exit code, stdout: {:.300}", out.stdout);
    let after = comments_of(&std::fs::read_to_string(&c).expect("read"));
    let survived = before.iter().all(|b| after.contains(b));
    let reported = out.last().expect("event")["comments_preserved"].as_bool();
    assert_eq!(
        reported,
        Some(survived),
        "comments_preserved reported {reported:?} but survival is {survived} (before {before:?}, after {after:?})"
    );
}

fn schema_property_keys(schema: &Value) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let mut collect = |v: &Value| {
        if let Some(props) = v.get("properties").and_then(Value::as_object) {
            keys.extend(props.keys().cloned());
        }
    };
    collect(schema);
    for combinator in ["oneOf", "anyOf", "allOf"] {
        if let Some(arr) = schema.get(combinator).and_then(Value::as_array) {
            arr.iter().for_each(&mut collect);
        }
    }
    keys
}

struct SchemaCase {
    verb: &'static str,
    args: Vec<String>,
    stdin: Option<&'static str>,
}

fn schema_cases(ws: &Path) -> Vec<SchemaCase> {
    let s = p(ws, "s.rs");
    let c = p(ws, "c.toml");
    let hash = {
        let out = run(ws, &["hash".into(), s.clone()], None);
        // The last record is the terminal summary, which carries no digest.
        out.events
            .iter()
            .find(|e| e["type"] == "hash")
            .expect("hash event")["checksum"]
            .as_str()
            .expect("checksum")
            .to_string()
    };
    let sc = |verb, args: Vec<String>| SchemaCase {
        verb,
        args,
        stdin: None,
    };
    vec![
        sc("get", vec!["get".into(), c.clone(), "package.name".into()]),
        sc(
            "set",
            vec!["set".into(), c.clone(), "package.name".into(), "y".into()],
        ),
        sc(
            "del",
            vec!["del".into(), "--force-missing".into(), c, "nope.key".into()],
        ),
        sc("locale", sv(&["locale"])),
        sc("verify", vec!["verify".into(), s.clone(), hash]),
        sc("stat", vec!["stat".into(), s.clone()]),
        sc("wal-stats", sv(&["wal-stats"])),
        sc("wal-heal", vec!["wal-heal".into(), p(ws, "")]),
        sc(
            "scratch",
            vec!["scratch".into(), "init".into(), p(ws, ".atomwrite/scratch")],
        ),
        sc(
            "case",
            vec![
                "case".into(),
                "--subvert".into(),
                "foo".into(),
                "qux".into(),
                "--dry-run".into(),
                s.clone(),
            ],
        ),
        sc("query", vec!["query".into(), "--kinds".into(), s.clone()]),
        sc("outline", vec!["outline".into(), s.clone()]),
        sc(
            "semantic-merge",
            merge_args(ws, "theirs.txt", "m1.txt", &[]),
        ),
        sc(
            "sparse",
            vec![
                "sparse".into(),
                "list".into(),
                p(ws, "."),
                "--max-files".into(),
                "5".into(),
            ],
        ),
        sc("agent-surface", sv(&["agent-surface", "--format", "json"])),
        sc(
            "codemod",
            vec![
                "codemod".into(),
                "--rules".into(),
                p(ws, "rules.yaml"),
                "--dry-run".into(),
                s.clone(),
            ],
        ),
        sc(
            "semantic-search",
            vec![
                "semantic-search".into(),
                "fn a".into(),
                p(ws, "."),
                "--k".into(),
                "3".into(),
            ],
        ),
        sc("doctor", sv(&["doctor"])),
        sc("commands", sv(&["commands"])),
    ]
}

// RED until R10: the schema served by `--print-schema NAME` must describe the
// verb's real terminal event; today most names serve WriteOutput/ProgressEvent.
// Not covered: `completions` (stdout is a raw shell script, `--install` writes
// outside the sandbox) and `extract` (its output keys are the caller's input
// fields, so there is no fixed key set to compare).
#[cfg(feature = "ast")]
#[test]
fn r10_served_schema_contains_terminal_event_keys() {
    let ws = fixture();
    let listed = run(ws.path(), &sv(&["--print-schema"]), None);
    let names: BTreeSet<String> = listed
        .stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<String>(l).ok())
        .collect();
    assert!(
        names.len() > 10,
        "--print-schema must list the schemas, got {names:?}"
    );
    let mut failures = Vec::new();
    for case in schema_cases(ws.path()) {
        if !names.contains(case.verb) {
            failures.push(format!("{}: not listed by --print-schema", case.verb));
            continue;
        }
        let out = run(ws.path(), &case.args, case.stdin);
        if out.code != Some(0) {
            failures.push(format!(
                "{}: success case exited {:?}: {:.160}",
                case.verb, out.code, out.stdout
            ));
            continue;
        }
        let Some(terminal) = out.last().and_then(Value::as_object) else {
            failures.push(format!("{}: no terminal event", case.verb));
            continue;
        };
        let schema_out = run(ws.path(), &sv(&["--print-schema", case.verb]), None);
        let schema: Value = match serde_json::from_str(&schema_out.stdout) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{}: schema is not JSON ({e})", case.verb));
                continue;
            }
        };
        let props = schema_property_keys(&schema);
        let missing: Vec<&String> = terminal.keys().filter(|k| !props.contains(*k)).collect();
        if !missing.is_empty() {
            failures.push(format!(
                "{}: schema `{}` lacks {missing:?}",
                case.verb,
                schema["title"].as_str().unwrap_or("?")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} schema gaps:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[cfg(unix)]
struct RestorePerms(std::path::PathBuf);

#[cfg(unix)]
impl Drop for RestorePerms {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

// RED until R11: a file that scope could not write must not be counted in
// `files_modified`, and the failure must be reported on stdout, not only on
// stderr.
#[cfg(feature = "ast")]
#[cfg(unix)]
#[test]
fn r11_scope_readonly_failure_is_reported_on_stdout() {
    use std::os::unix::fs::PermissionsExt;
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    std::fs::create_dir(d.join("ok")).expect("mkdir ok");
    std::fs::create_dir(d.join("ro")).expect("mkdir ro");
    std::fs::write(d.join("ok/rw.rs"), "// note\nfn b() {}\n").expect("fixture");
    std::fs::write(d.join("ro/ro.rs"), "// note\nfn a() {}\n").expect("fixture");
    std::fs::set_permissions(d.join("ro/ro.rs"), std::fs::Permissions::from_mode(0o444))
        .expect("chmod file");
    std::fs::set_permissions(d.join("ro"), std::fs::Permissions::from_mode(0o555))
        .expect("chmod dir");
    let _restore = RestorePerms(d.join("ro"));
    let out = run(
        d,
        &sv(&[
            "scope",
            "--lang",
            "rust",
            "--query",
            "comments",
            "--delete",
            d.to_str().expect("utf8"),
        ]),
        None,
    );
    let summary = out
        .events
        .iter()
        .find(|e| e["type"] == "summary")
        .unwrap_or_else(|| panic!("no summary event: {:.300}", out.stdout));
    assert_eq!(
        summary["files_modified"], 1,
        "only the writable file may be counted as modified: {summary}"
    );
    let rewritten = std::fs::read_to_string(d.join("ok/rw.rs")).expect("read rw");
    assert!(
        rewritten.contains("fn b() {}") && !rewritten.contains("// note"),
        "the writable file must be rewritten without its comment: {rewritten:?}"
    );
    let reported = summary["files_failed"].as_u64().unwrap_or(0) >= 1
        || out
            .events
            .iter()
            .any(|e| e["ok"] == false && e.to_string().contains("ro.rs"));
    assert!(
        reported,
        "the write failure is not reported on stdout: {:.400}",
        out.stdout
    );
}
