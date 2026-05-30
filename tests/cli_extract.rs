// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn extract_json_fields() {
    let input = r#"{"file":"src/main.rs","line":42,"text":"hello"}
{"file":"src/lib.rs","line":10,"text":"world"}"#;

    let output = common::atomwrite()
        .args(["extract", "file", "line"])
        .write_stdin(input)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["file"], "src/main.rs");
    assert_eq!(events[0]["line"], 42);
    assert!(events[0].get("text").is_none());
}

#[test]
fn extract_text_by_index() {
    let input = "hello world foo\nbar baz qux\n";

    let output = common::atomwrite()
        .args(["extract", "0", "2"])
        .write_stdin(input)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 2);
    let vals = events[0]["values"].as_array().expect("values array");
    assert_eq!(vals[0], "hello");
    assert_eq!(vals[1], "foo");
}

#[test]
fn extract_empty_input_produces_no_output() {
    let output = common::atomwrite()
        .args(["extract", "file"])
        .write_stdin("")
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert!(events.is_empty());
}

#[test]
fn extract_negative_index_selects_last() {
    let input = "a b c d\n";

    let output = common::atomwrite()
        .args(["extract", "--", "-1"])
        .write_stdin(input)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let vals = events[0]["values"].as_array().expect("values");
    assert_eq!(vals[0], "d");
}
