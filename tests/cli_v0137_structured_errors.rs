// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 — `get`, `set` and `del` on a malformed TOML/JSON file.
//!
//! The failure used to be an untyped error: exit 1 (the `NO_MATCHES` code),
//! nothing on stdout, and, for `set`, the whole file quoted on stderr. It is now
//! `INVALID_INPUT` (exit 65) with an NDJSON envelope naming only line, column
//! and a short cause. A secret inside the file must never reach any channel.

mod common;

const SECRET: &str = "hunter2-top-secret";

/// Run `verb args...` on a malformed file and return `(exit, stdout, stderr)`.
fn run(name: &str, body: &str, verb: &[&str]) -> (Option<i32>, String, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join(name), body).expect("seed");
    let output = common::atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap()])
        .args(verb)
        .output()
        .expect("run");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn assert_clean_invalid_input(result: (Option<i32>, String, String), format: &str) {
    let (code, stdout, stderr) = result;
    assert_eq!(code, Some(65), "stdout: {stdout} stderr: {stderr}");
    let events = common::parse_ndjson(stdout.as_bytes());
    let err = events.last().expect("an error envelope on stdout");
    assert_eq!(err["ok"], false, "{err}");
    assert_eq!(err["code"], "INVALID_INPUT", "{err}");
    let message = err["message"].as_str().unwrap_or_default();
    assert!(message.contains(format), "{message}");
    assert!(message.contains("line"), "{message}");
    assert!(message.contains("column"), "{message}");
    assert!(!stdout.contains(SECRET), "secret reached stdout: {stdout}");
    assert!(!stderr.contains(SECRET), "secret reached stderr: {stderr}");
}

fn toml_body() -> String {
    format!("token = \"{SECRET}\"\n[[[\n")
}

fn json_body() -> String {
    format!("{{\"token\": \"{SECRET}\", \"x\": ")
}

#[test]
fn get_malformed_toml() {
    assert_clean_invalid_input(
        run("bad.toml", &toml_body(), &["get", "bad.toml", "x.y"]),
        "TOML",
    );
}

#[test]
fn set_malformed_toml() {
    assert_clean_invalid_input(
        run("bad.toml", &toml_body(), &["set", "bad.toml", "x", "1"]),
        "TOML",
    );
}

#[test]
fn del_malformed_toml() {
    assert_clean_invalid_input(
        run("bad.toml", &toml_body(), &["del", "bad.toml", "x"]),
        "TOML",
    );
}

#[test]
fn get_malformed_json() {
    assert_clean_invalid_input(
        run("bad.json", &json_body(), &["get", "bad.json", "x"]),
        "JSON",
    );
}

#[test]
fn set_malformed_json() {
    assert_clean_invalid_input(
        run("bad.json", &json_body(), &["set", "bad.json", "x", "1"]),
        "JSON",
    );
}

#[test]
fn del_malformed_json() {
    assert_clean_invalid_input(
        run("bad.json", &json_body(), &["del", "bad.json", "x"]),
        "JSON",
    );
}

#[test]
fn toml_error_points_at_the_right_line() {
    let (_, stdout, _) = run("bad.toml", &toml_body(), &["get", "bad.toml", "x"]);
    let events = common::parse_ndjson(stdout.as_bytes());
    let message = events.last().unwrap()["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(message.contains("line 2"), "{message}");
}
