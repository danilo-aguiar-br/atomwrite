// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn regex_from_date_examples() {
    let output = common::atomwrite()
        .args(["regex", "2024-01-15", "2025-12-31", "2026-05-28"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "regex");
    assert!(events[0]["regex"].is_string());
    assert_eq!(events[0]["examples"], 3);
    assert_eq!(events[0]["anchored"], true);

    let regex_str = events[0]["regex"].as_str().unwrap();
    let re = regex::Regex::new(regex_str).expect("valid regex");
    assert!(re.is_match("2024-01-15"));
    assert!(re.is_match("2026-05-28"));
}

#[test]
fn regex_with_digits_flag() {
    let output = common::atomwrite()
        .args(["regex", "--digits", "abc123", "xyz789"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let regex_str = events[0]["regex"].as_str().unwrap();
    assert!(regex_str.contains("\\d"));
}

#[test]
fn regex_no_anchors() {
    let output = common::atomwrite()
        .args(["regex", "--no-anchors", "hello", "world"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["anchored"], false);
    let regex_str = events[0]["regex"].as_str().unwrap();
    assert!(!regex_str.starts_with('^'));
}

#[test]
fn regex_stdin_mode() {
    let input = "hello\nworld\n";

    let output = common::atomwrite()
        .args(["regex", "--stdin"])
        .write_stdin(input)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["examples"], 2);
}

#[test]
fn regex_no_examples_exits_65() {
    let output = common::atomwrite()
        .args(["regex", "--stdin"])
        .write_stdin("")
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
}
