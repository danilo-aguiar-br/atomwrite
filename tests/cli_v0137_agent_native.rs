// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 agent-native conformance: schema discovery, universal data
//! operations and the side-effect guards.
//!
//! Every assertion reads the data channel (stdout) and requires each line to
//! parse as one JSON object, so a stray banner, progress bar or ANSI escape on
//! that channel fails the test rather than passing silently.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn bin() -> Command {
    common::hermetic(Command::cargo_bin("atomwrite").expect("atomwrite bin"))
}

/// Parse every non-empty stdout line as a JSON object.
fn ndjson_lines(out: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(out)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<Value>(l)
                .unwrap_or_else(|e| panic!("data channel line is not JSON: {l:?} ({e})"))
        })
        .collect()
}

/// Fixture with a workspace and three files carrying distinct paths.
fn workspace_with_files() -> (TempDir, String) {
    let td = TempDir::new().expect("tempdir");
    fs::write(td.path().join("b.txt"), "beta\n").expect("write b");
    fs::write(td.path().join("a.txt"), "alpha\n").expect("write a");
    fs::write(td.path().join("c.txt"), "gamma\n").expect("write c");
    let ws = td.path().to_str().expect("utf8 workspace").to_string();
    (td, ws)
}

// ---------------------------------------------------------------------------
// E3 — schema discovery
// ---------------------------------------------------------------------------

#[test]
fn print_schema_without_argument_lists_every_schema_name() {
    let assert = bin().args(["--print-schema"]).assert().success();
    let lines = ndjson_lines(&assert.get_output().stdout);
    assert!(
        lines.len() >= 40,
        "schema surface must list the whole map, got {}",
        lines.len()
    );
    let names: Vec<&str> = lines.iter().filter_map(Value::as_str).collect();
    // Ordered alphabetically so the listing is deterministic.
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "listing must be alphabetically ordered");
    for expected in ["read", "edit", "search", "stream_diff", "error"] {
        assert!(
            names.contains(&expected),
            "listing must expose {expected:?}: {names:?}"
        );
    }
    // No dead weight on the data channel.
    assert!(names.iter().all(|n| !n.trim().is_empty()));
}

#[test]
fn print_schema_with_argument_emits_that_schema() {
    let assert = bin().args(["--print-schema", "edit"]).assert().success();
    // The schema is ONE pretty-printed JSON document, like `--json-schema`,
    // so the whole data channel parses at once rather than line by line.
    let text = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let doc: Value = serde_json::from_str(text.trim())
        .expect("the data channel must carry exactly one parseable schema document");
    assert!(
        doc.get("properties").is_some() || doc.get("$ref").is_some(),
        "expected a JSON Schema document, got: {doc}"
    );
    let rendered = doc.to_string();
    assert!(
        rendered.contains("match_count"),
        "edit schema must describe match_count: {rendered}"
    );
}

/// The `=` form must behave exactly like the separated form.
///
/// The help renders the value as optional (`[<NAME>]`) and clap accepts
/// `--flag=value`, so a caller writing `--print-schema=read` wrote the flag
/// correctly. The pre-clap scan matched only the separated spelling and
/// answered the equal form with a parse error for a flag that was right.
#[test]
fn print_schema_accepts_the_equals_form() {
    let separated = bin().args(["--print-schema", "edit"]).assert().success();
    let joined = bin().args(["--print-schema=edit"]).assert().success();
    assert_eq!(
        separated.get_output().stdout,
        joined.get_output().stdout,
        "the two spellings of the same request must emit the same schema"
    );
    let doc: Value = serde_json::from_str(
        String::from_utf8_lossy(&joined.get_output().stdout)
            .trim()
            .to_string()
            .as_str(),
    )
    .expect("the data channel must carry the schema document");
    assert!(
        doc.to_string().contains("match_count"),
        "the equals form must emit the requested schema, got: {doc}"
    );
}

/// An unknown name is a usage error carrying a machine code.
///
/// It used to exit 1 with an empty stdout. Silence is indistinguishable from a
/// schema that happens to be empty, so a consumer could not tell "this build
/// does not serve that name" from "that name describes nothing".
#[test]
fn print_schema_unknown_name_reports_a_typed_error() {
    let assert = bin()
        .args(["-v", "--print-schema", "nope"])
        .assert()
        .failure()
        .code(predicate::eq(65));
    let lines = ndjson_lines(&assert.get_output().stdout);
    assert_eq!(
        lines.len(),
        1,
        "the failure must arrive as exactly one envelope: {lines:?}"
    );
    assert_eq!(
        lines[0]["code"], "INVALID_INPUT",
        "the failure needs a machine code, got: {}",
        lines[0]
    );
    assert_eq!(lines[0]["ok"], false);
    let message = lines[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("nope"),
        "the envelope must name the rejected name, got: {message}"
    );
    assert!(
        message.contains("--print-schema"),
        "the envelope must name the door that lists the valid names, got: {message}"
    );
}

/// A trailing flag with no value must not replace the command with a listing.
///
/// Measured before the fix: `atomwrite read <file> --print-schema` printed the
/// alphabetised schema-name listing and never read the file. The caller asked
/// for one operation and received a different one, with a success status.
#[test]
fn print_schema_after_a_command_never_replaces_the_command() {
    let (td, ws) = workspace_with_files();
    let a = td.path().join("a.txt");
    let assert = bin()
        .args([
            "-v",
            "--workspace",
            &ws,
            "read",
            a.to_str().unwrap(),
            "--print-schema",
        ])
        .assert()
        .failure()
        .code(predicate::eq(65));
    let lines = ndjson_lines(&assert.get_output().stdout);
    assert_eq!(lines.len(), 1, "expected one envelope, got: {lines:?}");
    assert_ne!(
        lines[0]["type"], "read",
        "the command must not report success when it never ran"
    );
    let message = lines[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("must precede the command"),
        "the envelope must explain the placement rule, got: {message}"
    );
}

/// The discovery door must be discoverable from the agent manifest itself.
///
/// Reading `--help` prose to learn that a schema dump exists makes the door
/// depend on wording surviving a release. The manifest is the machine-readable
/// surface, so the door belongs there, with the names the dispatcher serves.
#[test]
fn agent_surface_publishes_the_schema_discovery_door() {
    let td = TempDir::new().expect("tempdir");
    let assert = bin()
        .args(["--workspace", td.path().to_str().unwrap(), "agent-surface"])
        .assert()
        .success();
    let lines = ndjson_lines(&assert.get_output().stdout);
    let manifest = lines
        .iter()
        .find(|v| v["type"] == "agent_surface")
        .unwrap_or_else(|| panic!("no manifest on the data channel: {lines:?}"));

    let discovery = manifest
        .get("discovery")
        .unwrap_or_else(|| panic!("manifest must publish the discovery door: {manifest}"));
    assert_eq!(discovery["flag"], "--print-schema");

    let names: Vec<&str> = discovery["schema_names"]
        .as_array()
        .expect("schema_names must be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        names.contains(&"cancelled"),
        "the manifest must publish the cancel schema: {names:?}"
    );

    // The manifest and the live door must agree on the served names.
    let listed = bin().args(["--print-schema"]).assert().success();
    let live: Vec<String> = ndjson_lines(&listed.get_output().stdout)
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    let from_manifest: Vec<String> = names.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        from_manifest, live,
        "the manifest must list exactly what the door serves"
    );
}

/// The help must keep advertising the door.
#[test]
fn help_advertises_the_schema_discovery_door() {
    for args in [vec!["--help"], vec!["-h"]] {
        let assert = bin().args(&args).assert().success();
        let text = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
        assert!(
            text.contains("--print-schema"),
            "{args:?} must advertise the discovery flag: {text}"
        );
    }
}

// ---------------------------------------------------------------------------
// E4 — universal data operations
// ---------------------------------------------------------------------------

#[test]
fn sort_orders_events_by_path_line() {
    let (td, ws) = workspace_with_files();
    let assert = bin()
        .args([
            "--workspace",
            &ws,
            "--sort-by",
            "path-line",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let lines = ndjson_lines(&assert.get_output().stdout);
    let paths: Vec<String> = lines
        .iter()
        .filter_map(|v| v["path"].as_str().map(str::to_string))
        .collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(
        paths, sorted,
        "search hits must come out path-ordered: {paths:?}"
    );
}

#[test]
fn limit_keeps_the_deterministic_first_n_after_sort() {
    let (td, ws) = workspace_with_files();
    let assert = bin()
        .args([
            "--workspace",
            &ws,
            "--sort-by",
            "path-line",
            "--limit",
            "2",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let lines = ndjson_lines(&assert.get_output().stdout);
    // `--limit 2` bounds the ANSWER records. Two kinds of record ride along
    // without being answers: the terminal `summary` says the answer is
    // complete, and a `begin` / `end` pair brackets a file without being one of
    // its matches. Charging either of them made the budget cut a match the
    // summary had already counted.
    let answers: Vec<_> = lines
        .iter()
        .filter(|v| matches!(v["type"].as_str(), Some("match" | "count" | "file")))
        .collect();
    assert_eq!(
        answers.len(),
        2,
        "--limit 2 must publish exactly two answer records, got {}",
        answers.len()
    );
    assert_eq!(
        lines.last().map(|v| v["type"].as_str()),
        Some(Some("summary")),
        "the terminal summary must survive --limit: {lines:?}"
    );
}

#[test]
fn dedupe_drops_repeated_tuples_and_keeps_the_first() {
    // The same content in three files makes `search` emit an identical `type`
    // on every hit; dedupe by `type` must collapse them to one line.
    let (td, ws) = workspace_with_files();
    // Both runs sort by path, so the reference run and the deduped run see the
    // same arrival order. `search` walks the tree in parallel, so comparing two
    // unsorted runs would compare two different orders rather than measuring
    // dedupe.
    let baseline = bin()
        .args([
            "--workspace",
            &ws,
            "--sort-by",
            "path-line",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let all = ndjson_lines(&baseline.get_output().stdout);
    assert!(all.len() >= 2, "fixture must produce repeats: {all:?}");

    let deduped = bin()
        .args([
            "--workspace",
            &ws,
            "--sort-by",
            "path-line",
            "--dedupe",
            "type",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let kept = ndjson_lines(&deduped.get_output().stdout);
    assert!(
        kept.len() < all.len(),
        "dedupe must drop repeats: before={} after={}",
        all.len(),
        kept.len()
    );
    assert_eq!(
        kept[0], all[0],
        "dedupe keeps the first event of each group"
    );
}

#[test]
fn max_output_bytes_requires_the_truncation_opt_in() {
    let (td, ws) = workspace_with_files();
    // Negative: the window alone is rejected; truncation is never a default.
    bin()
        .args([
            "--workspace",
            &ws,
            "--max-output-bytes",
            "4",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(predicate::eq(2));

    // Negative: the opt-in alone is also rejected.
    bin()
        .args([
            "--workspace",
            &ws,
            "--allow-truncate",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(predicate::eq(2));

    // Positive: the pair is accepted and every line still parses.
    let assert = bin()
        .args([
            "--workspace",
            &ws,
            "--allow-truncate",
            "--max-output-bytes",
            "4",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let lines = ndjson_lines(&assert.get_output().stdout);
    assert!(!lines.is_empty());
}

#[test]
fn aggregate_conflicts_with_diff_against() {
    let (td, ws) = workspace_with_files();
    bin()
        .args([
            "--workspace",
            &ws,
            "--aggregate",
            "type:count",
            "--diff-against",
            "baseline.ndjson",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(predicate::eq(2));
}

#[test]
fn redact_removes_secret_shaped_values_from_search_output() {
    let td = TempDir::new().expect("tempdir");
    let ws = td.path().to_str().unwrap();
    // The token is inside a file the tool reads, so it reaches the envelope.
    fs::write(
        td.path().join("creds.txt"),
        "api_token = AKIAIOSFODNN7EXAMPLE\n",
    )
    .expect("write creds");

    let plain = bin()
        .args(["--workspace", ws, "search", "AKIA", "."])
        .assert()
        .success();
    let plain_lines = ndjson_lines(&plain.get_output().stdout);
    assert!(
        plain_lines.iter().any(|v| v.to_string().contains("AKIA")),
        "fixture must expose the token without --redact"
    );

    let redacted = bin()
        .args(["--workspace", ws, "--redact", "search", "AKIA", "."])
        .assert()
        .success();
    let redacted_lines = ndjson_lines(&redacted.get_output().stdout);
    assert!(
        !redacted_lines.is_empty(),
        "redaction must not drop the matching events"
    );
    for v in &redacted_lines {
        assert!(
            !v.to_string().contains("AKIAIOSFODNN7EXAMPLE"),
            "secret survived --redact: {v}"
        );
    }
}

#[test]
fn normalize_encoding_reaches_the_data_channel_without_corruption() {
    let td = TempDir::new().expect("tempdir");
    let ws = td.path().to_str().unwrap();
    // Decomposed "café" (e + combining acute) in a file name.
    fs::write(td.path().join("cafe\u{301}.txt"), "needle\n").expect("write");

    let assert = bin()
        .args([
            "--workspace",
            ws,
            "--normalize-encoding",
            "search",
            "needle",
            ".",
        ])
        .assert()
        .success();
    let lines = ndjson_lines(&assert.get_output().stdout);
    assert!(!lines.is_empty());
    let joined = lines
        .iter()
        .filter_map(|v| v["path"].as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        joined.contains("caf\u{e9}"),
        "path must be NFC after normalization: {joined}"
    );
    assert!(
        !joined.contains("cafe\u{301}"),
        "decomposed form must not survive normalization: {joined}"
    );
}

#[test]
fn diff_against_reports_added_removed_unchanged() {
    let (td, ws) = workspace_with_files();
    // Baseline holds one hit that the live run still produces, plus one that
    // it will never produce.
    // Sorted, so the live run and the baseline agree on order: the tree walk is
    // parallel and an unsorted run would number the hits differently each time.
    let live = bin()
        .args([
            "--workspace",
            &ws,
            "--sort-by",
            "path-line",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let live_lines: Vec<String> = String::from_utf8_lossy(&live.get_output().stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        // The terminal `summary` is not a data hit, so it does not belong in the
        // baseline nor in the count of what the live run produced.
        .filter(|l| !l.contains("\"type\":\"summary\""))
        .map(str::to_string)
        .collect();
    // The baseline lives OUTSIDE the scanned tree on purpose: written inside it,
    // the file is itself a hit, so the scan that produces the comparison also
    // feeds it and the counts stop matching the fixture.
    let baseline_dir = TempDir::new().expect("baseline tempdir");
    let baseline_path = baseline_dir.path().join("baseline.ndjson");
    let mut baseline = String::new();
    baseline.push_str(live_lines.first().expect("at least one hit"));
    baseline.push('\n');
    baseline.push_str("{\"type\":\"search_match\",\"path\":\"gone.txt\",\"line\":99}\n");
    fs::write(&baseline_path, baseline).expect("write baseline");

    let assert = bin()
        .args([
            "--workspace",
            &ws,
            "--diff-against",
            baseline_path.to_str().unwrap(),
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let lines = ndjson_lines(&assert.get_output().stdout);
    // The diff replaces the data stream; the terminal summary still closes the
    // run, so the consumer keeps a verdict after the comparison.
    assert_eq!(
        lines.len(),
        2,
        "diff mode emits the comparison plus the terminal: {lines:?}"
    );
    let v = &lines[0];
    assert_eq!(v["type"], "stream_diff");
    assert_eq!(lines[1]["type"], "summary");
    assert!(
        v["unchanged"].as_u64().unwrap_or(0) >= 1,
        "the shared hit must count as unchanged: {v}"
    );
    assert!(
        v["removed"].as_u64().unwrap_or(0) >= 1,
        "the baseline-only hit must count as removed: {v}"
    );
    assert_eq!(
        v["added"].as_u64().unwrap_or(u64::MAX),
        live_lines.len() as u64 - 1,
        "every other live hit is an addition: {v}"
    );
}

// ---------------------------------------------------------------------------
// E6 — side-effect guards
// ---------------------------------------------------------------------------

/// The flag's help promises it is honoured only where the command contract asks
/// for it. On a verb that writes nothing, accepting it would report success for
/// an acknowledgement that acknowledged nothing.
#[test]
fn confirm_side_effect_is_rejected_where_the_contract_does_not_ask() {
    let (td, ws) = workspace_with_files();
    let assert = bin()
        .args([
            "-v",
            "--workspace",
            &ws,
            "--confirm-side-effect",
            "read",
            td.path().join("a.txt").to_str().unwrap(),
        ])
        .assert()
        .code(65)
        // `-v` is what brings `message` back into the envelope, and it also
        // turns on info logging on stderr. What must not happen is the failure
        // being reported twice, so the check is that stderr carries no error
        // envelope, not that stderr is silent.
        .stderr(predicate::str::contains("\"error\":true").not());

    let lines = ndjson_lines(&assert.get_output().stdout);
    let v = lines.first().expect("stdout must carry the error envelope");
    assert_eq!(v["code"], "INVALID_INPUT", "{v}");
    assert!(
        v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("writes nothing"),
        "{v}"
    );
}

#[test]
fn non_interactive_is_accepted_globally() {
    let (td, ws) = workspace_with_files();
    bin()
        .args([
            "--workspace",
            &ws,
            "--non-interactive",
            "read",
            td.path().join("a.txt").to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn mutating_command_accepts_the_side_effect_guards() {
    let (td, ws) = workspace_with_files();
    let target = td.path().join("a.txt");
    let assert = bin()
        .args([
            "--workspace",
            &ws,
            "edit",
            "--confirm-side-effect",
            "--non-interactive",
            "--old",
            "alpha",
            "--new",
            "beta",
            target.to_str().unwrap(),
        ])
        .assert()
        .success();
    let lines = ndjson_lines(&assert.get_output().stdout);
    assert_eq!(
        lines[0]["match_count"], 1,
        "guard must not break the edit: {lines:?}"
    );
}

// ---------------------------------------------------------------------------
// E2 — data channel stays parseable
// ---------------------------------------------------------------------------

#[test]
fn every_line_on_the_data_channel_is_one_json_object() {
    let (td, ws) = workspace_with_files();
    let assert = bin()
        .args([
            "--workspace",
            &ws,
            "-v",
            "--select",
            "type,path",
            "search",
            "a",
            td.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let lines = ndjson_lines(&assert.get_output().stdout);
    assert!(!lines.is_empty());
    for v in &lines {
        assert!(
            v.is_object(),
            "only objects belong on the data channel: {v}"
        );
        assert!(
            v.get("path").is_some() || v.get("type").is_some(),
            "projection must keep the requested keys: {v}"
        );
        let text = v.to_string();
        assert!(
            !text.contains('\u{1b}'),
            "ANSI escape leaked into the data channel: {text}"
        );
    }
}

/// The credential-key floor is unconditional: it must hold on the plainest
/// invocation there is, not only when an unrelated flag happens to be set.
///
/// The defect this guards: `--limit 5` used to redact an `api_token` while a
/// bare call passed it through, so the floor depended on a flag that has
/// nothing to do with redaction. Forgetting a flag must fail closed.
#[test]
fn credential_key_floor_holds_without_any_flag() {
    let td = TempDir::new().expect("tempdir");
    let ws = td.path().to_str().unwrap();
    let payload = td.path().join("data.json");
    fs::write(
        &payload,
        "{\"api_token\":\"leaked-secret\",\"path\":\"ok.rs\"}\n",
    )
    .expect("write");

    for extra in [
        vec![],
        vec!["--redact"],
        vec!["--limit", "5"],
        vec!["--select", "api_token"],
        vec!["--sort-by", "key"],
        vec!["--dedupe", "type"],
    ] {
        // `extract` reads the document on stdin; the payload is piped instead of
        // named, so the call shape is the one an agent actually writes.
        let mut args: Vec<&str> = vec!["--workspace", ws];
        args.extend_from_slice(&extra);
        args.extend_from_slice(&["extract"]);
        let assert = bin()
            .args(&args)
            .write_stdin(fs::read(&payload).expect("read payload"))
            .assert()
            .success();
        let text = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
        assert!(
            !text.contains("leaked-secret"),
            "credential must be redacted with args {extra:?}, got: {text}"
        );
        assert!(
            text.contains("api_token"),
            "the floor redacts the value, never drops the record: {text}"
        );
    }
}

/// The floor keys on the NAME only, so legitimate data whose name merely
/// contains a credential word is not corrupted.
#[test]
fn credential_key_floor_leaves_lookalike_names_alone() {
    let td = TempDir::new().expect("tempdir");
    let assert = bin()
        .args(["--workspace", td.path().to_str().unwrap(), "extract"])
        .write_stdin("{\"token_count\":42,\"secret_count\":7}\n")
        .assert()
        .success();
    let text = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(
        text.contains("42") && text.contains("7"),
        "counters are data, not credentials: {text}"
    );
}

/// A merged stream (`2>&1`) must be splittable by reading the record itself.
///
/// Every diagnostic carries `level` and `timestamp`; no payload has ever
/// carried either. That is the discriminator an agent uses after merging the
/// two channels, and it is asserted here rather than assumed, because the
/// alternative — a constant `channel` marker — cannot be emitted by
/// `tracing-subscriber`'s JSON formatter, and a key nothing stamps is a
/// promise a reader cannot rely on.
#[test]
fn diagnostic_records_are_distinguishable_from_payload() {
    let (td, ws) = workspace_with_files();

    // `-vv` is DEBUG and `-vvv` is TRACE: both must stay JSON-only on stderr
    // and both must keep DEBUG records alive in release builds, which a
    // compile-time `release_max_level_*` cap on `tracing` would strip.
    for flag in ["-vv", "-vvv"] {
        // Diagnostics: verbose stderr, one JSON object per line.
        let run = bin()
            .args([
                "--workspace",
                &ws,
                flag,
                "read",
                td.path().join("a.txt").to_str().unwrap(),
            ])
            .output()
            .expect("run");
        let diag = String::from_utf8_lossy(&run.stderr);
        let diag_lines: Vec<Value> = diag
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("diagnostic line is JSON"))
            .collect();
        assert!(!diag_lines.is_empty(), "{flag} run must log");
        for v in &diag_lines {
            assert!(
                v.get("level").is_some() && v.get("timestamp").is_some(),
                "{flag}: a diagnostic record must carry level and timestamp: {v}"
            );
        }
        assert!(
            diag_lines
                .iter()
                .any(|v| v.get("level").and_then(Value::as_str) == Some("DEBUG")),
            "{flag}: at least one record must have level DEBUG: {diag}"
        );

        // Payload: the data channel must carry neither key, or the
        // discriminator would be ambiguous.
        let payload = ndjson_lines(&run.stdout);
        assert!(!payload.is_empty(), "{flag}: read must emit a payload");
        for v in &payload {
            assert!(
                v.get("level").is_none() && v.get("timestamp").is_none(),
                "{flag}: a payload record must carry neither level nor timestamp: {v}"
            );
        }
    }
}
