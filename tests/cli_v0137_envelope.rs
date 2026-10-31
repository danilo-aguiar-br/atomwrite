// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 envelope contract tests (B2, B3, B5, D3).
//!
//! These tests pin the envelope shapes the agent surface promises:
//! one event per checksum verification, a closing search anchor for every
//! opening anchor, and an exit status list that is derived from the error
//! enum rather than transcribed.

mod common;

use std::fs;

use common::{atomwrite, create_test_file};

/// B2: a MATCHING verification emits exactly one event on the data channel,
/// and that event carries the verdict.
#[test]
fn verify_match_emits_one_event_with_ok_true() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_file(dir.path(), "v.txt", "test\n");
    let expected = blake3::hash(b"test\n").to_hex().to_string();

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--verify",
            &expected,
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = common::parse_ndjson(&output.stdout);
    // Exactly one verification record, closed by the summary `hash` always
    // emits: the terminal says the list ended, and it carries no second verdict
    // on the checksum, so B2 still holds.
    let verify: Vec<&serde_json::Value> = events
        .iter()
        .filter(|e| e["type"] == "hash_verify")
        .collect();
    assert_eq!(
        verify.len(),
        1,
        "one verification must emit exactly one verification event, got {events:?}"
    );
    assert_eq!(
        events.last().map(|e| e["type"].clone()),
        Some("summary".into())
    );
    assert_eq!(verify[0]["ok"], true);
    assert_eq!(verify[0]["verified"], true);
    assert_eq!(verify[0]["checksum"], expected);
    assert_eq!(verify[0]["expected"], expected);
    assert!(
        events[0]["code"].is_null(),
        "a matching verification carries no failure code"
    );
}

/// B2: a FAILING verification emits exactly one event on the data channel.
///
/// The event states the whole verdict by itself: `ok: false`, the computed
/// checksum, the expected checksum, the machine code and the process status.
/// The separate diagnostic envelope is allowed to exist, but the data channel
/// must not carry two events for one verification.
#[test]
fn verify_mismatch_emits_one_data_event_and_exits_81() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_file(dir.path(), "m.txt", "data\n");

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--verify",
            "wrong_hash_value",
        ])
        .arg(&path)
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(81));

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(
        events.len(),
        1,
        "one verification must emit exactly one data event, got {events:?}"
    );
    let event = &events[0];
    assert_eq!(event["ok"], false, "data event must state the failure");
    assert_eq!(event["verified"], false);
    assert_eq!(event["code"], "CHECKSUM_VERIFY_FAILED");
    assert_eq!(event["exit"], 81);
    assert_eq!(event["expected"], "wrong_hash_value");
    assert!(
        event["checksum"].is_string(),
        "the computed checksum must still be published"
    );
    assert_ne!(
        event["checksum"], event["expected"],
        "the two checksums must disagree, which is the whole point of the failure"
    );
}

/// B2: the exit status of the failure agrees with the status the error enum
/// declares, so the data event can never advertise a code the process does
/// not return.
#[test]
fn verify_mismatch_event_exit_matches_process_status() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_file(dir.path(), "d.txt", "payload\n");

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--verify",
            "not-the-hash",
        ])
        .arg(&path)
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    let advertised = events[0]["exit"].as_u64().expect("exit field");
    let actual = output.status.code().expect("process status") as u64;
    assert_eq!(
        advertised, actual,
        "the event must advertise exactly the status the process returns"
    );
}

/// B2: the `verify` subcommand delegates to `hash --verify`, so it must obey
/// the same one-event rule and the same exit status.
#[test]
fn verify_subcommand_emits_one_data_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_file(dir.path(), "s.txt", "sub\n");

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "verify",
            path.to_str().unwrap(),
            "definitely-wrong",
        ])
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(81));
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(
        events.len(),
        1,
        "`verify` must emit exactly one data event, got {events:?}"
    );
    assert_eq!(events[0]["ok"], false);
    assert_eq!(events[0]["code"], "CHECKSUM_VERIFY_FAILED");
}

/// B1/B2 regression guard: `ok` is the outcome, `verified` is the checksum verdict.
///
/// The two fields answer different questions and must not be conflated.
/// `verified` is meaningful only with `--verify`; a plain hash has nothing to
/// compare against, so `verified` stays absent. `ok` is the operation outcome,
/// which every success envelope carries (B1), so a plain hash that ran to
/// completion reports `ok: true` while publishing no `verified` field.
#[test]
fn plain_hash_keeps_its_own_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_file(dir.path(), "p.txt", "plain\n");

    let output = atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "hash"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    // The digest, then the terminal summary the verb always closes with.
    assert_eq!(events.len(), 2, "got {events:?}");
    assert_eq!(events[0]["type"], "hash");
    assert_eq!(events[1]["type"], "summary");
    assert_eq!(
        events[0]["ok"], true,
        "a plain hash that ran to completion still reports its outcome (B1)"
    );
    assert!(
        events[0]["verified"].is_null(),
        "`verified` is a checksum verdict and requires --verify"
    );
}

/// B5: with `--no-begin-end` the opening anchor is suppressed, so the closing
/// anchor must be suppressed too. Before v0.1.36 the closing anchor was
/// emitted anyway, and a consumer saw a close without a matching open.
#[test]
fn search_no_begin_end_emits_neither_anchor() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..4 {
        create_test_file(dir.path(), &format!("f{i}.txt"), "nothing here\n");
    }
    create_test_file(dir.path(), "hit.txt", "needle\n");

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "--no-begin-end",
            "needle",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    let begins = events.iter().filter(|e| e["type"] == "begin").count();
    let ends = events.iter().filter(|e| e["type"] == "end").count();
    assert_eq!(
        begins, 0,
        "--no-begin-end must suppress every opening anchor"
    );
    assert_eq!(ends, 0, "--no-begin-end must suppress every closing anchor");
    assert_eq!(begins, ends, "openings and closings must stay paired");
}

/// B5: in the default mode every opening anchor is closed, whether or not the
/// file matched. Before v0.1.36 the file with no match emitted an opening
/// anchor and no closing one.
#[test]
fn search_default_closes_every_opened_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..6 {
        create_test_file(dir.path(), &format!("miss{i}.txt"), "nothing here\n");
    }
    create_test_file(dir.path(), "hit.txt", "needle\n");

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "needle",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let begins: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "begin")
        .map(|e| e["path"].as_str().expect("path"))
        .collect();
    let ends: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "end")
        .map(|e| e["path"].as_str().expect("path"))
        .collect();

    assert_eq!(begins.len(), 7, "seven files must open seven anchors");
    assert_eq!(
        ends.len(),
        begins.len(),
        "every opening anchor must have exactly one closing anchor"
    );
    assert_eq!(begins, ends, "the anchors must pair up by path");
}

/// B5: a closed pair for a file with no match still reports the truth.
///
/// The closing anchor of a non-matching file carries `stats.matches: 0`, so
/// the parity fix does not invent matches.
#[test]
fn search_closing_anchor_of_empty_file_reports_zero_matches() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_file(dir.path(), "empty.txt", "nothing here\n");
    create_test_file(dir.path(), "hit.txt", "needle\n");

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "needle",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    let empty_end = events
        .iter()
        .find(|e| {
            e["type"] == "end" && e["path"].as_str().is_some_and(|p| p.ends_with("empty.txt"))
        })
        .expect("the non-matching file must still close its anchor");
    assert_eq!(
        empty_end["stats"]["matches"], 0,
        "a file with no match must report zero matches, not a fabricated count"
    );
}

/// B5: `--count` and `--files` replace the anchors with their own one-line
/// summary, so neither mode may emit an anchor at all.
#[test]
fn search_count_and_files_emit_no_anchors() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_file(dir.path(), "a.txt", "needle\n");
    create_test_file(dir.path(), "b.txt", "nothing\n");

    for mode in ["--count", "--files"] {
        let output = atomwrite()
            .args([
                "--workspace",
                dir.path().to_str().unwrap(),
                "search",
                mode,
                "needle",
            ])
            .arg(dir.path())
            .output()
            .expect("run");

        let events = common::parse_ndjson(&output.stdout);
        let anchors = events
            .iter()
            .filter(|e| e["type"] == "begin" || e["type"] == "end")
            .count();
        assert_eq!(anchors, 0, "{mode} must not emit begin/end anchors");
    }
}

/// B3: the agent surface publishes the exit status list, with 85 present.
///
/// `FIFO_DETECTED` existed in the error enum but appeared on no surface, so a
/// consumer could not learn that status 85 is reachable.
#[test]
fn agent_surface_publishes_exit_codes_including_85() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "agent-surface"])
        .output()
        .expect("run");
    assert!(out.status.success());

    let events = common::parse_ndjson(&out.stdout);
    assert_eq!(events.len(), 1, "agent-surface emits one manifest");
    let codes = events[0]["exit_codes"]
        .as_array()
        .expect("exit_codes array must exist");

    let fifo = codes
        .iter()
        .find(|e| e["name"] == "FIFO_DETECTED")
        .expect("FIFO_DETECTED must be on the agent surface");
    assert_eq!(fifo["code"], 85);
    assert_eq!(
        fifo["source"], "error-variant",
        "the entry must be derived from the error enum, not transcribed"
    );
}

/// B3: the published list covers the statuses an agent has to handle, and it
/// is unique and ordered so a consumer can binary-search it.
#[test]
fn agent_surface_exit_codes_are_ordered_and_cover_the_known_statuses() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "agent-surface"])
        .output()
        .expect("run");
    let events = common::parse_ndjson(&out.stdout);
    let codes = events[0]["exit_codes"]
        .as_array()
        .expect("exit_codes array must exist");

    let numbers: Vec<u64> = codes
        .iter()
        .map(|e| e["code"].as_u64().expect("numeric code"))
        .collect();
    let mut sorted = numbers.clone();
    sorted.sort_unstable();
    assert_eq!(numbers, sorted, "the list must be ordered by status");

    for required in [
        0_u64, 1, 2, 4, 13, 28, 65, 74, 78, 81, 82, 85, 88, 124, 126, 141, 143,
    ] {
        assert!(
            numbers.contains(&required),
            "status {required} must be published"
        );
    }
}

/// D3: the free-text notes are reduced to prose that does not encode
/// semantics a flag entry can carry. The structured `flag_contract` list
/// takes over the flag meanings.
#[test]
fn agent_surface_flag_contract_is_structured() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "agent-surface"])
        .output()
        .expect("run");
    let events = common::parse_ndjson(&out.stdout);

    let contract = events[0]["flag_contract"]
        .as_array()
        .expect("flag_contract array must exist");
    assert!(
        !contract.is_empty(),
        "the flag contract must carry at least one entry"
    );
    for entry in contract {
        assert!(
            entry["flag"].as_str().is_some_and(|f| !f.is_empty()),
            "every flag contract entry must name its flag"
        );
        assert!(
            entry["semantics"].as_str().is_some_and(|s| !s.is_empty()),
            "every flag contract entry must state its semantics"
        );
    }

    let notes = events[0]["notes"].as_array().expect("notes array");
    let notes_text = notes
        .iter()
        .filter_map(|n| n.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for moved in [
        "flag glossary",
        "anti-patterns",
        "write supports --durability",
        "--plan=delete list-only",
    ] {
        assert!(
            !notes_text.contains(moved),
            "note text {moved:?} must live in the structured contract, not in prose"
        );
    }
}

/// D3: the manifest still parses as a single NDJSON line and keeps the fields
/// it already promised, so the reduction of prose is not a breaking change
/// for a consumer that reads tools or version.
#[test]
fn agent_surface_keeps_its_existing_contract() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "agent-surface"])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.lines().filter(|l| !l.is_empty()).count(),
        1,
        "agent-surface must emit exactly one NDJSON line"
    );
    let events = common::parse_ndjson(&out.stdout);
    assert_eq!(events[0]["type"], "agent_surface");
    assert!(
        events[0]["version"].as_str().is_some_and(|v| !v.is_empty()),
        "the manifest must keep publishing the version"
    );
    assert!(
        events[0]["tools"].as_array().is_some_and(|t| !t.is_empty()),
        "the manifest must keep publishing the tool list"
    );
}

/// The hash family keeps resolving to a published schema, so the change to
/// the verification path does not leave a dangling hint behind.
///
/// The repository-wide sweep over every hint lives in the end-to-end suite;
/// this test only pins the hints this change can move.
#[test]
fn hash_family_schema_hints_resolve() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "agent-surface"])
        .output()
        .expect("run");
    let events = common::parse_ndjson(&out.stdout);

    let schemas_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/schemas");
    if !schemas_dir.is_dir() {
        return;
    }
    let tools = events[0]["tools"].as_array().expect("tools array");
    for name in ["hash", "verify"] {
        let tool = tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("tool {name} missing from the surface"));
        let hint = tool["schema_hint"]
            .as_str()
            .unwrap_or_else(|| panic!("tool {name} has no schema_hint"));
        let path = schemas_dir.join(format!("{hint}.schema.json"));
        assert!(
            path.is_file(),
            "schema_hint {hint:?} for tool {name} has no file"
        );
    }
}

/// The generated `hash_verify` event must itself be serializable to one
/// NDJSON object, and the failure shape must be a superset of the success
/// shape so a consumer reads one struct.
#[test]
fn hash_verify_event_serializes_with_the_verdict_inside_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_file(dir.path(), "x.txt", "body\n");
    let expected = blake3::hash(b"body\n").to_hex().to_string();

    let ok_out = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--verify",
            &expected,
        ])
        .arg(&path)
        .output()
        .expect("run");
    let ok_events = common::parse_ndjson(&ok_out.stdout);

    let bad_out = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "hash",
            "--verify",
            "nope",
        ])
        .arg(&path)
        .output()
        .expect("run");
    let bad_events = common::parse_ndjson(&bad_out.stdout);

    let ok_keys: std::collections::BTreeSet<&str> = ok_events[0]
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    let bad_keys: std::collections::BTreeSet<&str> = bad_events[0]
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();

    assert!(
        ok_keys.is_subset(&bad_keys),
        "a passing event must not carry keys the failing event lacks: {ok_keys:?} vs {bad_keys:?}"
    );
    assert_eq!(ok_events[0]["type"], "hash_verify");
    assert_eq!(bad_events[0]["type"], "hash_verify");
}

/// The test directory is removed when the fixture drops; assert the fixture
/// really writes where the test expects, so a silent fixture change cannot
/// make the parity tests vacuous.
#[test]
fn fixture_writes_the_files_the_parity_tests_count() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_file(dir.path(), "one.txt", "a\n");
    create_test_file(dir.path(), "two.txt", "b\n");
    let entries = fs::read_dir(dir.path())
        .expect("read_dir")
        .filter_map(Result::ok)
        .count();
    assert_eq!(entries, 2);
}

/// B1: the success envelope carries the result key, which until now only
/// `doctor` and `scratch` declared, each in its own struct.
///
/// The `read` envelope is the witness: it declares no such field, so the key
/// can only have come from the writer.
#[test]
fn success_envelopes_carry_the_result_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_file(dir.path(), "r.txt", "hello world\n");

    let output = atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "read"])
        .arg(&path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["type"], "read");
    assert_eq!(
        events[0]["ok"], true,
        "a success envelope must state the result key"
    );
}

/// B1: the key arrives on every terminal envelope, not on a chosen few.
///
/// Each entry is one invocation and the command whose terminal envelope it
/// must stamp.
#[test]
fn result_key_reaches_every_terminal_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();
    let file = create_test_file(dir.path(), "t.txt", "alpha\nbeta\n");

    // (label, args, terminal envelope type)
    let cases: Vec<(&str, Vec<&str>, &str)> = vec![
        ("hash", vec!["hash", file.to_str().unwrap()], "hash"),
        ("read", vec!["read", file.to_str().unwrap()], "read"),
        (
            "list",
            vec!["list", dir.path().to_str().unwrap()],
            "summary",
        ),
        (
            "count",
            vec!["count", dir.path().to_str().unwrap()],
            "count",
        ),
        ("calc", vec!["calc", "2+2"], "calc"),
        ("regex", vec!["regex", "abc-12", "xyz-99", "id-7"], "regex"),
        (
            "semantic-search",
            vec!["semantic-search", "alpha", dir.path().to_str().unwrap()],
            "semantic_summary",
        ),
        (
            "search",
            vec!["search", "alpha", dir.path().to_str().unwrap()],
            "summary",
        ),
        ("agent-surface", vec!["agent-surface"], "agent_surface"),
        ("commands", vec!["commands"], "commands_tree"),
        ("locale", vec!["locale"], "locale_report"),
        ("config", vec!["config", "list"], "config"),
        ("wal-stats", vec!["wal-stats"], "wal_stats"),
    ];

    for (label, args, terminal) in cases {
        let output = atomwrite()
            .arg("--workspace")
            .arg(ws)
            .args(&args)
            .output()
            .unwrap_or_else(|e| panic!("{label}: run failed: {e}"));

        let events = common::parse_ndjson(&output.stdout);
        let envelope = events
            .iter()
            .find(|e| e["type"] == terminal)
            .unwrap_or_else(|| panic!("{label}: no {terminal} envelope in {events:?}"));
        assert_eq!(
            envelope["ok"], true,
            "{label}: the {terminal} envelope must state the result key"
        );
    }
}

/// B1: a stream record is not an outcome, so it must not claim a verdict.
///
/// A `begin` or `end` anchor says "this file was visited", not "the operation
/// succeeded". Stamping a verdict on it would let an agent read success from a
/// record that knows nothing about the run, and would also give the anchors a
/// shape the parity contract does not promise.
#[test]
fn stream_records_do_not_claim_a_verdict() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_file(dir.path(), "a.txt", "needle here\n");

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "search",
            "needle",
        ])
        .arg(dir.path())
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    let stream: Vec<&serde_json::Value> = events
        .iter()
        .filter(|e| {
            matches!(
                e["type"].as_str(),
                Some("begin" | "end" | "match" | "context")
            )
        })
        .collect();
    assert!(!stream.is_empty(), "the search must emit stream records");
    for record in stream {
        assert!(
            record["ok"].is_null(),
            "stream record {:?} must not claim a verdict",
            record["type"]
        );
    }
}

/// B1: a payload the user projected keeps the user's keys.
///
/// `extract` re-emits JSON that came from the user. Stamping an operation
/// verdict into it would both corrupt the user's data and contradict a key the
/// user may have written themselves, so the payload must pass through as-is.
#[test]
fn extract_payload_is_not_stamped_with_a_verdict() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "extract"])
        .write_stdin("{\"a\":1,\"b\":2}\n")
        .output()
        .expect("run");

    assert!(output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        serde_json::json!({"a": 1, "b": 2}),
        "a projected payload must survive the writer unchanged"
    );
}

/// B1: a user payload that already states the result key keeps its own value.
///
/// The writer stamps the key, it does not overrule the payload.
#[test]
fn extract_payload_keeps_its_own_verdict_value() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = atomwrite()
        .args(["--workspace", dir.path().to_str().unwrap(), "extract"])
        .write_stdin("{\"ok\":false,\"a\":1}\n")
        .output()
        .expect("run");

    let events = common::parse_ndjson(&output.stdout);
    assert_eq!(
        events[0]["ok"], false,
        "the writer must not overwrite a value the payload already carries"
    );
}

/// B1: the error envelope keeps its own false verdict, so the writer's
/// injection never turns a failure into a success.
#[test]
fn error_envelope_keeps_its_false_verdict() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = atomwrite()
        .args([
            "--workspace",
            dir.path().to_str().unwrap(),
            "read",
            dir.path().join("missing.txt").to_str().unwrap(),
        ])
        .output()
        .expect("run");

    assert!(!output.status.success());
    let events = common::parse_ndjson(&output.stdout);
    let err = events
        .iter()
        .find(|e| e["error"] == true)
        .expect("an error envelope must be emitted");
    assert_eq!(err["ok"], false);
    assert_eq!(err["code"], "FILE_NOT_FOUND");
}

/// B1: the two subcommands that already declared the key keep the same
/// position and value, so a consumer that reads them is unaffected.
#[test]
fn doctor_and_scratch_keep_their_result_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();

    let doctor = atomwrite()
        .args(["--workspace", ws, "doctor"])
        .output()
        .expect("run");
    let events = common::parse_ndjson(&doctor.stdout);
    assert_eq!(events[0]["type"], "doctor_report");
    assert_eq!(events[0]["ok"], true);

    let scratch = atomwrite()
        .args(["--workspace", ws, "scratch", "init", ".atomwrite/scratch"])
        .output()
        .expect("run");
    let events = common::parse_ndjson(&scratch.stdout);
    assert_eq!(events[0]["ok"], true);
}

/// B4: `message` comes from the error variant (English by contract) while
/// `suggestion` comes from the i18n catalogue, so one envelope can carry the
/// two in different languages. The envelope must say which language the
/// suggestion is in, otherwise a consumer cannot tell what it is reading.
#[test]
fn error_envelope_declares_the_suggestion_language() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();
    // Missing file inside the workspace: the catalogue carries a suggestion
    // per error code, so the assertion names the text of THIS code.
    let outside = dir.path().join("nope.txt");

    let en = atomwrite()
        .args([
            "--locale",
            "en",
            "--workspace",
            ws,
            "read",
            outside.to_str().unwrap(),
        ])
        .output()
        .expect("run en");
    let en_events = common::parse_ndjson(&en.stdout);
    assert_eq!(en_events[0]["suggestion_lang"], "en");
    let en_suggestion = en_events[0]["suggestion"].as_str().unwrap_or("");
    assert!(
        en_suggestion.contains("verify the file path exists"),
        "the declared language must match the text: {en_events:?}"
    );

    let pt = atomwrite()
        .args([
            "--locale",
            "pt-BR",
            "--workspace",
            ws,
            "read",
            outside.to_str().unwrap(),
        ])
        .output()
        .expect("run pt-BR");
    let pt_events = common::parse_ndjson(&pt.stdout);
    assert_eq!(pt_events[0]["suggestion_lang"], "pt-BR");
    let pt_suggestion = pt_events[0]["suggestion"].as_str().unwrap_or("");
    assert!(
        pt_suggestion.contains("verifique se o caminho do arquivo existe"),
        "the declared language must match the text: {pt_events:?}"
    );
}
/// Content redaction must reach the ERROR channel too.
///
/// The success channel redacted a credential in a text field while the error
/// channel published the same credential inside `path` and `message`, so the
/// flag promised protection the error path did not deliver. Both envelope
/// channels run the same shape now.
#[test]
fn redact_reaches_the_error_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();
    let secret_path = dir.path().join("AKIAIOSFODNN7EXAMPLE.txt");

    // Both error branches: the one without a projection and the one with it.
    for extra in [vec![], vec!["--select", "code,path"]] {
        let mut args: Vec<&str> = vec!["--workspace", ws, "--redact"];
        args.extend_from_slice(&extra);
        args.extend_from_slice(&["read", secret_path.to_str().unwrap()]);
        let out = atomwrite().args(&args).output().expect("run");
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        assert!(
            !text.contains("AKIAIOSFODNN7EXAMPLE"),
            "a credential in the error envelope must be redacted with args {extra:?}: {text}"
        );
    }

    // Without the flag an ordinary path survives: content redaction is opt-in,
    // and only the credential-key floor is unconditional.
    let plain = atomwrite()
        .args([
            "--workspace",
            ws,
            "read",
            dir.path().join("naoexiste.txt").to_str().unwrap(),
        ])
        .output()
        .expect("run");
    let text = String::from_utf8_lossy(&plain.stdout).to_string();
    assert!(
        text.contains("naoexiste.txt"),
        "an ordinary path must survive without --redact: {text}"
    );
}

/// The error envelope keeps the key order its struct declares.
///
/// The envelope is serialized through `serde_json::Value` so the credential
/// floor and `--redact` can run on it. Without `preserve_order` that path
/// emits keys alphabetically, which silently changed the byte shape of every
/// error line: a consumer comparing raw lines, or a baseline captured before
/// the change, stopped matching even though no key or value moved. The feature
/// pins the declared order, so this asserts the FIRST key rather than any key
/// set, which is the part that regressed.
#[test]
fn error_envelope_keeps_declared_key_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path().to_str().unwrap();
    let missing = dir.path().join("nope.txt");

    let out = atomwrite()
        .args(["--workspace", ws, "read", missing.to_str().unwrap()])
        .output()
        .expect("run");
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().expect("one error line");

    // `ErrorJson` declares `error` first, then `ok`; alphabetical order would
    // put `code` or `error_class` at the front instead.
    assert!(
        line.starts_with("{\"error\":true,\"ok\":false,\"code\":"),
        "error envelope must keep its declared key order, got: {line}"
    );
}
