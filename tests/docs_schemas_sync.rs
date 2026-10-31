// SPDX-License-Identifier: MIT OR Apache-2.0

//! `docs/schemas/*.json` are the published copy of `atomwrite --print-schema`.
//!
//! Every schema name maps to ONE file through the explicit table below, the
//! file holds exactly the JSON the binary prints, and the schemas that are
//! closed (`additionalProperties: false`) accept the real output of the verb
//! they describe. The failure message lists every divergent file at once.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use common::schema_check;

/// `--print-schema NAME` to `docs/schemas/<FILE>.schema.json`.
///
/// The table lives in the crate (`atomwrite::SCHEMA_FILE_NAMES`) because
/// `cargo xtask schemas` writes the published files from it; the test reads the
/// same table, so the generator and the checker cannot drift. `stat` is
/// intentionally absent: it serves the `read` schema.
use atomwrite::SCHEMA_FILE_NAMES as NAME_TO_FILE;

/// Names that serve another name's schema and so have no file of their own.
const ALIAS_NAMES: &[&str] = &["stat"];

/// Files with no `--print-schema` name: nested inside another schema.
///
/// Empty since `write-risk` became a served name; kept as a named constant so
/// the check below keeps testing the orphans it declares.
const ORPHAN_FILES: &[&str] = &[];

/// Run `args` feeding `stdin` in, and return the NDJSON events.
fn events_with_stdin(ws: &Path, args: &[&str], stdin: &str) -> Vec<Value> {
    use std::io::Write;
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let mut child = common::hermetic(Command::new(bin))
        .args([
            "--workspace",
            ws.to_str().expect("utf8"),
            "--no-progress",
            "--no-auto-heal",
        ])
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn atomwrite");
    {
        let mut pipe = child.stdin.take().expect("stdin");
        pipe.write_all(stdin.as_bytes()).expect("feed stdin");
    }
    common::parse_ndjson(&child.wait_with_output().expect("run").stdout)
}

#[test]
fn the_extract_schema_accepts_every_record_it_writes() {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    let schema = printed("extract");
    assert_eq!(schema["title"], "ExtractOutput");
    assert_ne!(
        printed("extract"),
        printed("calc"),
        "each verb serves its own schema"
    );

    // NDJSON narrowed, NDJSON whole, text with fields, text without fields.
    let cases: [(&[&str], &str); 4] = [
        (
            &["extract", "--field", "path"],
            "{\"type\":\"hit\",\"path\":\"a\"}\n",
        ),
        (&["extract"], "{\"type\":\"hit\",\"path\":\"a\"}\n"),
        (&["extract", "--field", "0"], "alfa beta\n"),
        (&["extract"], "alfa beta\n"),
    ];
    for (args, stdin) in cases {
        let out = events_with_stdin(d, args, stdin);
        let first = out.first().unwrap_or_else(|| panic!("{args:?}: no output"));
        let found = gaps(&schema, first);
        assert!(found.is_empty(), "{args:?}: {found:?} in {first}");
    }
}

fn printed(name: &str) -> Value {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = common::hermetic(Command::new(bin))
        .args(["--print-schema", name])
        .output()
        .expect("run --print-schema");
    assert!(output.status.success(), "--print-schema {name} failed");
    serde_json::from_slice(&output.stdout).expect("schema JSON")
}

fn schemas_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/schemas")
}

#[test]
fn every_schema_name_has_a_file_and_every_file_a_name() {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = common::hermetic(Command::new(bin))
        .arg("--print-schema")
        .output()
        .expect("list schemas");
    let names: BTreeSet<String> = common::parse_ndjson(&output.stdout)
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let mapped: BTreeSet<String> = NAME_TO_FILE.iter().map(|(n, _)| (*n).to_string()).collect();
    let aliases: BTreeSet<String> = ALIAS_NAMES.iter().map(|n| (*n).to_string()).collect();
    let known = &mapped | &aliases;
    let unmapped: Vec<&String> = names.difference(&known).collect();
    let stale: Vec<&String> = mapped.difference(&names).collect();
    assert!(unmapped.is_empty(), "names without a file: {unmapped:?}");
    assert!(
        stale.is_empty(),
        "mapped names the binary no longer has: {stale:?}"
    );

    let on_disk: BTreeSet<String> = std::fs::read_dir(schemas_dir())
        .expect("read docs/schemas")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            file.strip_suffix(".schema.json").map(str::to_string)
        })
        .collect();
    let expected: BTreeSet<String> = NAME_TO_FILE
        .iter()
        .map(|(_, f)| (*f).to_string())
        .chain(ORPHAN_FILES.iter().map(|f| (*f).to_string()))
        .collect();
    let orphans: Vec<&String> = on_disk.difference(&expected).collect();
    let missing: Vec<&String> = expected.difference(&on_disk).collect();
    assert!(orphans.is_empty(), "files no name serves: {orphans:?}");
    assert!(missing.is_empty(), "files that do not exist: {missing:?}");
}

#[cfg(feature = "ast")]
#[test]
fn docs_schemas_equal_the_printed_ones() {
    let mut divergent = Vec::new();
    for (name, file) in NAME_TO_FILE {
        let path = schemas_dir().join(format!("{file}.schema.json"));
        let body = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let docs: Value = serde_json::from_str(&body).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        if docs != printed(name) {
            divergent.push(format!(
                "{file}.schema.json (atomwrite --print-schema {name})"
            ));
        }
    }
    assert!(
        divergent.is_empty(),
        "regenerate these from the binary:\n{}",
        divergent.join("\n")
    );
}

/// Run the binary in a hermetic workspace and return its NDJSON events.
fn events(ws: &Path, args: &[&str]) -> Vec<Value> {
    let bin = assert_cmd::cargo::cargo_bin("atomwrite");
    let output = common::hermetic(Command::new(bin))
        .args([
            "--workspace",
            ws.to_str().expect("utf8"),
            "--no-progress",
            "--no-auto-heal",
        ])
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run atomwrite");
    common::parse_ndjson(&output.stdout)
}

/// What `event` fails against `schema`.
///
/// The check used to compare `required` and unknown keys only, so a schema
/// declaring `integer` passed with a string in the event. It now runs the full
/// validator in [`common::schema_check`], which knows every keyword these
/// schemas use and refuses a value of the wrong type, outside the enum or below
/// the minimum.
fn gaps(schema: &Value, event: &Value) -> Vec<String> {
    common::schema_check::check(schema, schema, event)
}

/// What the old shape-only check would have accepted.
///
/// Kept because the suite compares the two: a schema declaring `integer` where
/// the binary emits a string must fail here and pass the shape-only version, so
/// the stronger check is proven stronger and not assumed to be.
fn shape_only_gaps(schema: &Value, event: &Value) -> Vec<String> {
    let object = event.as_object().expect("event object");
    if let Some(branches) = schema["anyOf"].as_array() {
        let per_branch: Vec<Vec<String>> =
            branches.iter().map(|b| shape_only_gaps(b, event)).collect();
        return if per_branch.iter().any(Vec::is_empty) {
            Vec::new()
        } else {
            per_branch.into_iter().flatten().collect()
        };
    }
    let mut found = Vec::new();
    for key in schema["required"].as_array().into_iter().flatten() {
        let key = key.as_str().expect("required key");
        if !object.contains_key(key) {
            found.push(format!("required {key} missing"));
        }
    }
    if schema["additionalProperties"] == false {
        let declared = schema["properties"].as_object().expect("properties");
        for key in object.keys() {
            if !declared.contains_key(key) {
                found.push(format!("{key} not declared"));
            }
        }
    }
    found
}

#[cfg(feature = "ast")]
#[test]
fn the_terminal_event_of_the_main_verbs_fits_its_schema() {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    common::create_test_file(d, "src/a.rs", "// note\nfn needle() {}\n");
    common::create_test_file(d, "src/b.txt", "needle needle\n");
    let src = d.join("src");
    let src = src.to_str().expect("utf8");
    let a_rs = format!("{src}/a.rs");
    let b_txt = format!("{src}/b.txt");

    let cases: Vec<(&str, Vec<&str>)> = vec![
        ("search", vec!["search", "needle", src]),
        (
            "replace",
            vec!["replace", "--dry-run", "needle", "thread", src],
        ),
        ("count", vec!["count", src]),
        ("count-by-ext", vec!["count", "--by-extension", src]),
        (
            "count-by-size",
            vec!["count", "--by-size", "--top", "2", src],
        ),
        ("list", vec!["list", src]),
        ("backup", vec!["backup", &a_rs]),
        ("diff", vec!["diff", &a_rs, &b_txt]),
        ("config", vec!["config", "list"]),
        ("doctor", vec!["doctor"]),
        (
            "scope",
            vec![
                "scope",
                src,
                "--lang",
                "rust",
                "--query",
                "comments",
                "--delete",
                "--dry-run",
            ],
        ),
    ];
    let mut failures = Vec::new();
    for (name, args) in cases {
        let events = events(d, &args);
        let schema = printed(name);
        let Some(terminal) = events.last() else {
            failures.push(format!("{name}: no output"));
            continue;
        };
        let found = gaps(&schema, terminal);
        if !found.is_empty() {
            failures.push(format!("{name}: {found:?} in {terminal}"));
        }
    }
    let batch = {
        let bin = assert_cmd::cargo::cargo_bin("atomwrite");
        let mut child = common::hermetic(Command::new(bin))
            .args([
                "--workspace",
                d.to_str().expect("utf8"),
                "--no-progress",
                "batch",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("spawn batch");
        {
            use std::io::Write;
            let mut pipe = child.stdin.take().expect("stdin");
            pipe.write_all(b"{\"op\":\"write\",\"target\":\"x.txt\",\"content\":\"x\"}\n")
                .expect("manifest");
        }
        common::parse_ndjson(&child.wait_with_output().expect("batch").stdout)
    };
    let found = gaps(&printed("batch"), batch.last().expect("batch summary"));
    if !found.is_empty() {
        failures.push(format!("batch: {found:?}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[cfg(feature = "ast")]
#[test]
fn the_scope_schema_accepts_both_of_its_records() {
    let ws = tempfile::tempdir().expect("workspace");
    let d = ws.path();
    common::create_test_file(d, "a.rs", "// note\nfn keep() {}\n");
    let schema = printed("scope");
    for args in [
        vec![
            "scope",
            ".",
            "--lang",
            "rust",
            "--query",
            "comments",
            "--delete",
            "--dry-run",
        ],
        vec![
            "scope", ".", "--lang", "rust", "--query", "comments", "--delete",
        ],
    ] {
        let out = events(d, &args);
        assert!(out.len() >= 2, "a result and a summary: {out:?}");
        for event in &out {
            let found = gaps(&schema, event);
            assert!(found.is_empty(), "{args:?}: {found:?} in {event}");
        }
    }
}

#[test]
fn the_completions_schema_describes_the_installed_event_not_write() {
    let completions = printed("completions");
    assert_eq!(completions["title"], "CompletionsInstalled");
    assert_ne!(completions, printed("write"));
}

#[test]
fn the_validator_rejects_a_wrong_type_the_shape_only_check_let_through() {
    // A schema whose `hunks_applied` is an integer, against an event that
    // carries it as a string: the exact case the weak check could not see.
    let schema: Value = serde_json::from_str(
        r#"{
            "type": "object",
            "additionalProperties": false,
            "required": ["ok", "hunks_applied"],
            "properties": {
                "ok": {"type": "boolean"},
                "hunks_applied": {"type": "integer", "minimum": 0}
            }
        }"#,
    )
    .expect("schema");
    let wrong: Value =
        serde_json::from_str(r#"{"ok": true, "hunks_applied": "one"}"#).expect("event");

    assert!(
        shape_only_gaps(&schema, &wrong).is_empty(),
        "the old check accepted the wrong type"
    );
    let found = schema_check::check(&schema, &schema, &wrong);
    assert!(
        found.iter().any(|f| f.contains("expected integer")),
        "the validator must refuse the string: {found:?}"
    );

    // Below the minimum, which the shape-only check could never see either.
    let negative: Value =
        serde_json::from_str(r#"{"ok": true, "hunks_applied": -1}"#).expect("event");
    let found = schema_check::check(&schema, &schema, &negative);
    assert!(
        found.iter().any(|f| f.contains("below the minimum")),
        "the validator must refuse the negative count: {found:?}"
    );

    // And the right event still passes, so the check is not a blanket refusal.
    let right: Value = serde_json::from_str(r#"{"ok": true, "hunks_applied": 3}"#).expect("event");
    assert!(
        schema_check::check(&schema, &schema, &right).is_empty(),
        "a conforming event must pass"
    );
}

#[test]
fn every_published_schema_uses_only_keywords_the_validator_implements() {
    let mut unknown = Vec::new();
    for (name, file) in NAME_TO_FILE {
        let path = schemas_dir().join(format!("{file}.schema.json"));
        let body = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let schema: Value = serde_json::from_str(&body).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        for keyword in schema_check::unsupported_keywords(&schema) {
            unknown.push(format!("{name}: {keyword}"));
        }
    }
    assert!(
        unknown.is_empty(),
        "the validator does not implement these keywords, so it cannot vouch for the schema: {unknown:?}"
    );
}
