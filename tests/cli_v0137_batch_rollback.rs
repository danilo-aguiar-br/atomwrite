// SPDX-License-Identifier: MIT OR Apache-2.0

//! `batch --transaction` reaches exit 80 when the reversal itself fails.
//!
//! The manifest makes the reversal impossible without any injection: the file
//! a successful operation created is replaced by a non-empty directory, and a
//! later operation fails. Removing the created file then hits `EISDIR`, so the
//! tree is left partially reverted and the run must say so with
//! `TRANSACTION_ROLLBACK_FAILED` (exit 80), never with the input error of the
//! operation that triggered the rollback.

mod common;

use std::path::Path;

fn run(ws: &Path, stdin: &str) -> std::process::Output {
    common::atomwrite()
        .args(["--workspace", ws.to_str().expect("utf8 workspace")])
        .args(["batch", "--transaction"])
        .write_stdin(stdin)
        .output()
        .expect("run atomwrite")
}

#[test]
fn a_failed_reversal_reports_exit_80() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    std::fs::create_dir(root.join("d")).expect("mkdir d");
    std::fs::create_dir(root.join("sub")).expect("mkdir sub");
    common::create_test_file(&root.join("d"), "f.txt", "hi\n");

    // 1. create sub/a.txt, 2. remove it, 3. move the directory onto that path,
    // 4. recreate `d` so the reverse move is skipped, 5. fail.
    let manifest = [
        r#"{"op":"write","path":"sub/a.txt","content":"x"}"#,
        r#"{"op":"delete","path":"sub/a.txt"}"#,
        r#"{"op":"move","path":"d","target":"sub/a.txt"}"#,
        r#"{"op":"write","path":"d/x.txt","content":"y"}"#,
        r#"{"op":"replace","path":"missing.txt","old":"a","new":"b"}"#,
    ]
    .join("\n");

    let out = run(root, &manifest);
    let events = common::parse_ndjson(&out.stdout);
    assert_eq!(out.status.code(), Some(80), "{events:?}");

    let failure = events
        .iter()
        .rev()
        .find(|e| e["ok"] == false && e.get("type").is_none())
        .unwrap_or_else(|| panic!("no failure envelope in {events:?}"));
    assert_eq!(failure["code"], "TRANSACTION_ROLLBACK_FAILED", "{failure}");
    assert_eq!(failure["exit"], 80, "{failure}");
    assert!(
        failure["message"]
            .as_str()
            .is_some_and(|m| m.contains("partially reverted")),
        "the message says the tree may be half reverted: {failure}"
    );
}
