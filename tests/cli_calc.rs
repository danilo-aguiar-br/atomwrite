// SPDX-License-Identifier: MIT OR Apache-2.0

mod common;

#[test]
fn calc_arithmetic() {
    let output = common::atomwrite()
        .args(["calc", "2 + 3 * 4"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["type"], "calc");
    assert_eq!(events[0]["result"], "14");
}

#[test]
fn calc_unit_conversion() {
    let output = common::atomwrite()
        .args(["calc", "2 hours + 30 minutes to seconds"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events[0]["result"], "9000 seconds");
}

#[test]
fn calc_data_sizes() {
    let output = common::atomwrite()
        .args(["calc", "1 GB to MB"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert!(events[0]["result"].as_str().unwrap().contains("1000"));
}

#[test]
fn calc_stdin_mode() {
    let input = "2+3\n10*5\n";

    let output = common::atomwrite()
        .args(["calc", "--stdin"])
        .write_stdin(input)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["result"], "5");
    assert_eq!(events[1]["result"], "50");
}

#[test]
fn calc_invalid_expression_exits_65() {
    let output = common::atomwrite()
        .args(["calc", "???invalid!!!"])
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(65));
}
