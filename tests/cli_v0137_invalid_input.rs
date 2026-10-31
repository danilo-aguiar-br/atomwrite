// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37: input the caller got wrong (an invalid regex, glob, rule, query,
//! pattern, filter or flag combination) fails as `INVALID_INPUT` with exit 65,
//! one short message and a hint, never as an `INTERNAL_ERROR` (exit 255).

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

struct Case {
    name: &'static str,
    /// Arguments after the global flags; `{ws}` is replaced by the workspace path.
    args: Vec<&'static str>,
    stdin: &'static str,
}

fn case(name: &'static str, args: &[&'static str]) -> Case {
    Case {
        name,
        args: args.to_vec(),
        stdin: "",
    }
}

fn run(ws: &Path, case: &Case) -> (Option<i32>, Vec<Value>) {
    let ws_text = ws.to_str().expect("utf8 workspace");
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    // Same empty HOME / XDG as the rest of the suite: no parser cache, no config.
    let mut command = common::hermetic(Command::new(bin));
    command
        .args(["--workspace", ws_text, "--no-progress", "--no-auto-heal"])
        .args(case.args.iter().map(|a| a.replace("{ws}", ws_text)))
        .current_dir(ws)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().expect("spawn");
    if let Some(mut pipe) = child.stdin.take() {
        let _ = pipe.write_all(case.stdin.as_bytes());
    }
    let output = child.wait_with_output().expect("wait");
    (output.status.code(), common::parse_ndjson(&output.stdout))
}

fn workspace() -> tempfile::TempDir {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    common::create_test_file(d, "s.rs", "fn main() { let x = 1; }\n");
    common::create_test_file(d, "a.txt", "alpha beta\n");
    common::create_test_file(d, "bad.yaml", "- id: [unclosed\n  language: rust\n");
    ws
}

/// A broken query is reported as a broken query, with the language named or not.
#[cfg(feature = "ast")]
#[test]
fn invalid_sexpression_names_the_query_not_the_language() {
    for args in [
        vec!["query", "-Q", "(((", "--language", "rust", "{ws}/s.rs"],
        vec!["query", "-Q", "(((", "{ws}/s.rs"],
        vec![
            "query",
            "-Q",
            "(function_item",
            "--language",
            "rust",
            "{ws}/s.rs",
        ],
        vec![
            "query",
            "-Q",
            "function_item)",
            "--language",
            "rust",
            "{ws}/s.rs",
        ],
    ] {
        let ws = workspace();
        let (code, events) = run(ws.path(), &case("query", &args));
        let envelope = events.last().expect("failure event");
        assert_eq!(code, Some(65), "{args:?}: {envelope}");
        let message = envelope["message"].as_str().unwrap_or_default();
        assert!(message.contains("S-expression"), "{args:?}: {message}");
        assert!(
            !message.contains("unsupported language"),
            "{args:?}: {message}"
        );
        assert!(
            !message.ends_with(": ^"),
            "{args:?}: the cause is lost: {message}"
        );
    }
}

#[cfg(feature = "ast")]
#[test]
fn invalid_user_input_is_invalid_input_exit_65_with_one_short_message() {
    let cases = vec![
        case("replace --regex", &["replace", "--regex", "(", "x", "{ws}"]),
        case(
            "replace include glob",
            &["replace", "-g", "[", "alpha", "x", "{ws}"],
        ),
        case(
            "replace exclude glob",
            &["replace", "--exclude", "[", "alpha", "x", "{ws}"],
        ),
        case("search regex", &["search", "(", "{ws}"]),
        case(
            "search include glob",
            &["search", "alpha", "{ws}", "--include", "["],
        ),
        case(
            "search exclude glob",
            &["search", "alpha", "{ws}", "--exclude", "["],
        ),
        case(
            "transform pattern",
            &[
                "transform",
                "-p",
                "let x = 1; let y = 2;",
                "-r",
                "x",
                "-l",
                "rust",
                "{ws}",
            ],
        ),
        case(
            "transform inline rules",
            &["transform", "--inline-rules", "- id: [unclosed", "{ws}"],
        ),
        case(
            "transform rules file",
            &["transform", "--rules", "{ws}/bad.yaml", "{ws}"],
        ),
        case(
            "transform exclude glob",
            &[
                "transform",
                "-p",
                "let x = 1;",
                "-r",
                "x",
                "-l",
                "rust",
                "--exclude",
                "[",
                "{ws}",
            ],
        ),
        case(
            "query -Q",
            &["query", "-Q", "(((", "--language", "rust", "{ws}/s.rs"],
        ),
        case(
            "scope pattern",
            &[
                "scope",
                "-l",
                "rust",
                "--pattern",
                "let x = 1; let y = 2;",
                "--delete",
                "{ws}",
            ],
        ),
        case(
            "scope exclude glob",
            &[
                "scope",
                "-l",
                "rust",
                "--query",
                "comments",
                "--exclude",
                "[",
                "{ws}",
            ],
        ),
        case(
            "codemod rules",
            &["codemod", "--rules", "{ws}/bad.yaml", "{ws}"],
        ),
        case(
            "case --subvert identical",
            &["case", "--subvert", "same", "same", "--dry-run", "{ws}"],
        ),
        case("list exclude glob", &["list", "--exclude", "[", "{ws}"]),
        case("count exclude glob", &["count", "--exclude", "[", "{ws}"]),
        case(
            "delete exclude glob",
            &["delete", "--plan", "-r", "--exclude", "[", "{ws}"],
        ),
        case("read --grep", &["read", "--grep", "(", "{ws}/a.txt"]),
        // The exact invocations the V1 audit found still failing.
        case("list --include", &["list", "--include", "[", "{ws}"]),
        case("count --include", &["count", "--include", "[", "{ws}"]),
        case(
            "sparse list --include",
            &["sparse", "list", "{ws}", "--include", "["],
        ),
        case(
            "sparse list --exclude",
            &["sparse", "list", "{ws}", "--exclude", "["],
        ),
        case(
            "sparse outline --include",
            &["sparse", "outline", "{ws}", "--include", "["],
        ),
        case(
            "sparse outline --exclude",
            &["sparse", "outline", "{ws}", "--exclude", "["],
        ),
        case(
            "hash -r --exclude",
            &["hash", "-r", "--exclude", "[", "{ws}"],
        ),
        case(
            "delete --include",
            &["delete", "--plan", "-r", "--include", "[", "{ws}"],
        ),
        case(
            "transform --include",
            &[
                "transform",
                "-p",
                "let x = 1;",
                "-r",
                "x",
                "-l",
                "rust",
                "--include",
                "[",
                "{ws}",
            ],
        ),
        case(
            "scope --include",
            &[
                "scope",
                "-l",
                "rust",
                "--query",
                "comments",
                "--include",
                "[",
                "{ws}",
            ],
        ),
        case("replace -g", &["replace", "-g", "[", "alpha", "x", "{ws}"]),
        case("--filter", &["--filter", "nonsense", "list", "{ws}"]),
        case("--aggregate", &["--aggregate", "type:nope", "list", "{ws}"]),
        case("locale --set", &["locale", "--set", "zz-ZZ"]),
        case(
            "locale --set --clear",
            &["locale", "--set", "en", "--clear"],
        ),
        Case {
            name: "edit --multi",
            args: vec!["edit", "--multi", "{ws}/a.txt"],
            stdin: "{not json}\n",
        },
    ];
    let mut gaps = Vec::new();
    for case in &cases {
        let ws = workspace();
        let (code, events) = run(ws.path(), case);
        let failed: Vec<&Value> = events.iter().filter(|e| e["ok"] == false).collect();
        let Some(envelope) = failed.last() else {
            gaps.push(format!(
                "{}: no failure (exit {code:?}): {events:?}",
                case.name
            ));
            continue;
        };
        if code != Some(65) || envelope["code"] != "INVALID_INPUT" {
            gaps.push(format!(
                "{}: exit {code:?} code {} (want 65 INVALID_INPUT): {envelope}",
                case.name, envelope["code"]
            ));
            continue;
        }
        let message = envelope["message"].as_str().unwrap_or_default();
        if message.contains('\n') || message.len() > 240 {
            gaps.push(format!("{}: message is not short: {message:?}", case.name));
        }
        // The message names the real cause, never a stand-in such as an
        // unavailable language when the input itself is what is wrong.
        if message.contains("unsupported language") {
            gaps.push(format!("{}: hides the cause: {message:?}", case.name));
        }
        if envelope["suggestion"].as_str().is_none_or(str::is_empty) {
            gaps.push(format!("{}: no hint: {envelope}", case.name));
        }
        if failed.len() != 1 {
            gaps.push(format!(
                "{}: {} failure events, want 1",
                case.name,
                failed.len()
            ));
        }
    }
    assert!(gaps.is_empty(), "{} gaps:\n{}", gaps.len(), gaps.join("\n"));
}
