// SPDX-License-Identifier: MIT OR Apache-2.0
// Unit tests for output shaping.
// Included from output_shape.inc.rs.

#[cfg(test)]
mod output_shape_tests {
    use super::*;
    use serde_json::json;

    fn shaped(select: Option<Vec<&str>>, filters: Vec<&str>) -> OutputShape {
        shape_with(|s| {
            s.select = select.map(|v| v.into_iter().map(str::to_string).collect());
            s.filters = filters
                .into_iter()
                .map(|f| parse_filter_expr(f).expect("filter"))
                .collect();
        })
    }

    /// Build a shape through a mutator closure.
    ///
    /// Every test needs a shape with a few non-default fields; building it this
    /// way keeps the construction in one place and avoids the
    /// assign-after-`Default::default()` pattern.
    fn shape_with(apply: impl FnOnce(&mut OutputShape)) -> OutputShape {
        let mut shape = OutputShape::default();
        apply(&mut shape);
        shape
    }

    fn positioned(values: Vec<Value>) -> Vec<PositionedValue> {
        values
            .into_iter()
            .enumerate()
            .map(|(i, value)| PositionedValue {
                value,
                line_index: i as u64,
                terminal: false,
                framing: false,
            })
            .collect()
    }

    #[test]
    fn parse_eq_bool() {
        let p = parse_filter_expr("ok=false").unwrap();
        assert_eq!(p.key, "ok");
        assert_eq!(p.value, json!(false));
        assert!(!p.not_equal);
    }

    #[test]
    fn parse_neq_string() {
        let p = parse_filter_expr("code!=WORKSPACE_JAIL").unwrap();
        assert!(p.not_equal);
        assert_eq!(p.value, json!("WORKSPACE_JAIL"));
    }

    #[test]
    fn project_select_omits_missing() {
        let shape = shaped(Some(vec!["ok", "missing", "code"]), vec![]);
        let out = shape_value(json!({"ok": false, "code": "X", "noise": 1}), &shape).unwrap();
        assert_eq!(out, json!({"ok": false, "code": "X"}));
    }

    #[test]
    fn filter_drops_non_match() {
        let shape = shaped(None, vec!["ok=false"]);
        assert!(shape_value(json!({"ok": true}), &shape).is_none());
        assert!(shape_value(json!({"ok": false}), &shape).is_some());
    }

    #[test]
    fn identity_when_empty() {
        assert!(OutputShape::default().is_identity());
    }

    #[test]
    fn error_shape_ignores_filter_keeps_select() {
        let mut shape = shaped(Some(vec!["ok", "code"]), vec!["ok=true"]);
        shape.limit = Some(0);
        let err = json!({"ok": false, "code": "WORKSPACE_JAIL", "noise": 1});
        // Success path would drop (ok != true).
        assert!(shape_value(err.clone(), &shape).is_none());
        // Error path always emits and may project.
        let out = shape_error_value(err, &shape);
        assert_eq!(out, json!({"ok": false, "code": "WORKSPACE_JAIL"}));
    }

    // --- v0.1.36 universal data operations ---

    #[test]
    fn sort_by_path_line_is_total_and_stable() {
        let shape = shape_with(|s| {
            s.sort_key = Some("path-line".into());
            s.sort = Some(SortKey::PathLine);
        });
        assert!(!shape.is_identity());
        assert!(shape.is_buffering());
        let mut events = positioned(vec![
            json!({"path": "b.rs", "line": 1}),
            json!({"path": "a.rs", "line": 9}),
            json!({"path": "a.rs", "line": 2}),
            json!({"path": "b.rs", "line": 1}),
        ]);
        events.sort_by(|a, b| compare_positioned(a, b, &shape));
        let ordered: Vec<(&str, u64)> = events
            .iter()
            .map(|e| {
                (
                    e.value["path"].as_str().unwrap_or_default(),
                    e.value["line"].as_u64().unwrap_or_default(),
                )
            })
            .collect();
        assert_eq!(ordered, vec![("a.rs", 2), ("a.rs", 9), ("b.rs", 1), ("b.rs", 1)]);
        // Tie on all fields keeps arrival order (stable).
        assert_eq!(events[2].line_index, 0);
        assert_eq!(events[3].line_index, 3);
    }

    #[test]
    fn sort_missing_key_goes_last_not_first() {
        let shape = shape_with(|s| {
            s.sort_key = Some("score".into());
            s.sort = Some(SortKey::Key);
        });
        let mut events = positioned(vec![
            json!({"id": "no-score"}),
            json!({"id": "ten", "score": 10}),
            json!({"id": "one", "score": 1}),
        ]);
        events.sort_by(|a, b| compare_positioned(a, b, &shape));
        assert_eq!(events[0].value["id"], "one");
        assert_eq!(events[1].value["id"], "ten");
        assert_eq!(events[2].value["id"], "no-score");
    }

    #[test]
    fn dedupe_identity_distinguishes_missing_from_null() {
        let keys: Vec<String> = vec!["a".into(), "b".into()];
        let missing = dedupe_identity(&json!({"a": 1}), &keys);
        let explicit_null = dedupe_identity(&json!({"a": 1, "b": null}), &keys);
        // Absent and null must not collide, or dedupe would merge two events
        // the agent can tell apart.
        assert_ne!(missing, explicit_null);
        // Deterministic: the same tuple always yields the same identity.
        assert_eq!(explicit_null, dedupe_identity(&json!({"a": 1, "b": null}), &keys));
        // Key order in the object must not change the identity, since the
        // selector order is what defines it.
        assert_eq!(
            dedupe_identity(&json!({"b": null, "a": 1}), &keys),
            dedupe_identity(&json!({"a": 1, "b": null}), &keys)
        );
    }

    #[test]
    fn truncate_content_counts_characters_not_bytes() {
        let shape = shape_with(|s| s.truncate_specs = vec![("content".into(), 3)]);
        let mut v = json!({"content": "açúcar", "path": "p.rs"});
        let changed = apply_string_limits(&mut v, &shape);
        assert!(changed);
        // Three CHARACTERS, not three bytes.
        assert_eq!(v["content"], "açú");
        assert_eq!(v["content"].as_str().unwrap().chars().count(), 3);
        assert_eq!(v["path"], "p.rs");
        // Negative: shorter than the limit is untouched.
        let mut short = json!({"content": "aç"});
        assert!(!apply_string_limits(&mut short, &shape));
        assert_eq!(short["content"], "aç");
    }

    #[test]
    fn max_output_bytes_truncates_every_string_and_respects_boundaries() {
        let shape = shape_with(|s| s.max_output_bytes = Some(5));
        let mut v = json!({"a": "0123456789", "b": "açúcar", "n": 1234567890});
        assert!(apply_string_limits(&mut v, &shape));
        assert_eq!(v["a"], "01234");
        // "açúcar" bytes: a=1, ç=2, ú=2, c=1 -> byte 5 would split "ú", so the
        // cut falls back to the last boundary, keeping exactly three characters.
        assert_eq!(v["b"], "açú");
        assert!(v["b"].as_str().unwrap().len() <= 5);
        assert_eq!(v["n"], json!(1234567890));
        // Negative: already within budget.
        let mut small = json!({"a": "abc"});
        assert!(!apply_string_limits(&mut small, &shape));
    }

    #[test]
    fn redact_replaces_secret_key_values_and_credential_prefixes() {
        let mut v = json!({
            "ok": true,
            "api_token": "value-that-is-not-shaped",
            "note": "Authorization: Bearer abc.def",
            "path": "src/main.rs",
        });
        assert_eq!(redact_secrets(&mut v), 2);
        assert_eq!(v["api_token"], REDACTED);
        assert_eq!(v["note"], REDACTED);
        assert_eq!(v["path"], "src/main.rs");
        assert_eq!(v["ok"], json!(true));
        // Negative: nothing secret-shaped.
        let mut clean = json!({"ok": true, "path": "src/main.rs"});
        assert_eq!(redact_secrets(&mut clean), 0);
        assert_eq!(clean["path"], "src/main.rs");
    }

    #[test]
    fn normalize_encoding_applies_nfc() {
        // "cafe\u{301}" (e + combining acute) -> "café" (precomposed).
        let mut v = json!({"name": "cafe\u{301}"});
        assert!(normalize_encoding(&mut v));
        assert_eq!(v["name"], "café");
        // Negative: already NFC is a no-op.
        let mut already = json!({"name": "café"});
        assert!(!normalize_encoding(&mut already));
    }

    #[test]
    fn join_parses_spec_and_indexes_external_records() {
        let spec = parse_join_expr("owner:user").unwrap();
        assert_eq!(spec.key, "owner");
        assert_eq!(spec.suffix, "user");
        let tables = parse_join_stdin(
            "{\"owner\":\"alice\",\"email\":\"a@x\"}\n{\"owner\":\"bob\",\"email\":\"b@x\"}\n{\"owner\":\"alice\",\"email\":\"a2@x\"}\n",
            std::slice::from_ref(&spec),
        )
        .unwrap();
        assert_eq!(tables.len(), 1);
        let mut hit = json!({"owner": "alice", "path": "p.rs"});
        assert!(apply_join(&mut hit, &tables[0]));
        assert_eq!(hit["user"].as_array().unwrap().len(), 2);
        // Negative: no group for this key.
        let mut miss = json!({"owner": "carol", "path": "p.rs"});
        assert!(!apply_join(&mut miss, &tables[0]));
        assert!(miss.get("user").is_none());
    }

    #[test]
    fn aggregate_spec_parses_ops_and_rejects_bad_input() {
        assert_eq!(
            parse_aggregate_expr("path").unwrap(),
            AggSpec { group: "path".into(), op: AggOp::Count }
        );
        assert_eq!(
            parse_aggregate_expr("path:count").unwrap(),
            AggSpec { group: "path".into(), op: AggOp::Count }
        );
        assert_eq!(
            parse_aggregate_expr("path:sum:match_count").unwrap(),
            AggSpec { group: "path".into(), op: AggOp::Sum("match_count".into()) }
        );
        // Negative cases.
        assert!(parse_aggregate_expr(":count").is_err());
        assert!(parse_aggregate_expr("path:median:x").is_err());
        assert!(parse_aggregate_expr("path:sum").is_err());
    }

    #[test]
    fn aggregate_group_sums_only_matching_specs() {
        let events = positioned(vec![
            json!({"path": "a.rs", "match_count": 2}),
            json!({"path": "a.rs", "match_count": 3}),
            json!({"path": "b.rs", "match_count": 7}),
        ]);
        let specs = vec![AggSpec { group: "path".into(), op: AggOp::Sum("match_count".into()) }];
        let summary = aggregate_group("path", json!("a.rs"), &events[..2], &specs);
        assert_eq!(summary["path"], "a.rs");
        assert_eq!(summary["count"], 2);
        assert_eq!(summary["sum_match_count"], 5);
        // Negative: group with no numeric field yields zero, not a crash.
        let empty = aggregate_group("path", json!("b.rs"), &events[2..], &specs);
        assert_eq!(empty["count"], 1);
        assert_eq!(empty["sum_match_count"], 7);
        let missing = aggregate_group(
            "path",
            json!("c.rs"),
            &positioned(vec![json!({"path": "c.rs"})]),
            &specs,
        );
        assert_eq!(missing["count"], 1);
        assert_eq!(missing["sum_match_count"], 0);
    }

    #[test]
    fn diff_streams_counts_added_removed_unchanged() {
        let baseline = positioned(vec![json!({"id": 1}), json!({"id": 2})]);
        let live = positioned(vec![json!({"id": 2}), json!({"id": 3})]);
        let d = diff_streams(&baseline, &live);
        assert_eq!(d.unchanged, 1);
        assert_eq!(d.added, 1);
        assert_eq!(d.removed, 1);
        // Negative: identical streams produce an empty diff.
        let same = diff_streams(&baseline, &baseline);
        assert_eq!(same, StreamDiff { added: 0, removed: 0, unchanged: 2 });
    }

    #[test]
    fn from_global_args_rejects_malformed_join_and_truncate() {
        assert!(parse_join_expr("owner").is_err());
        assert!(parse_join_expr("owner:").is_err());
        assert!(parse_truncate_expr("content").is_err());
        assert!(parse_truncate_expr("content:0").is_err());
        assert!(parse_truncate_expr("content:abc").is_err());
        assert_eq!(
            parse_truncate_expr("content:400").unwrap(),
            ("content".to_string(), 400)
        );
    }

    #[test]
    fn shape_value_applies_redaction_to_success_events() {
        let shape = shape_with(|s| s.redact = true);
        // Positive: secret-shaped value is replaced.
        let out = shape_value(
            json!({"token": "abc", "path": "p.rs"}),
            &shape,
        )
        .unwrap();
        assert_eq!(out["token"], REDACTED);
        assert_eq!(out["path"], "p.rs");
        // Negative: clean value is byte-identical.
        let clean = shape_value(json!({"path": "p.rs"}), &shape).unwrap();
        assert_eq!(clean, json!({"path": "p.rs"}));
    }

    #[test]
    fn shape_error_value_redacts_but_never_truncates() {
        let shape = shape_with(|s| {
            s.redact = true;
            s.max_output_bytes = Some(4);
            s.verbose = true;
        });
        let err = shape_error_value(
            json!({"ok": false, "api_token": "leaked", "message": "long diagnostic text"}),
            &shape,
        );
        assert_eq!(err["api_token"], REDACTED);
        // The diagnostic survives: an error must stay readable.
        assert_eq!(err["message"], "long diagnostic text");
    }

    #[test]
    fn emit_shaped_line_reports_filtered_events() {
        let shape = shaped(None, vec!["ok=true"]);
        let mut out: Vec<u8> = Vec::new();
        assert!(!emit_shaped_line(&mut out, json!({"ok": false}), &shape).unwrap());
        assert!(out.is_empty(), "filtered event must not reach the data channel");
        assert!(emit_shaped_line(&mut out, json!({"ok": true}), &shape).unwrap());
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 1);
        assert_eq!(
            serde_json::from_str::<Value>(text.trim_end()).unwrap(),
            json!({"ok": true})
        );
    }

    #[test]
    fn flush_positioned_sorts_dedupes_and_aggregates_in_order() {
        let shape = shape_with(|s| {
            s.dedupe_keys = vec!["path".into()];
            s.aggregates = vec![AggSpec {
                group: "path".into(),
                op: AggOp::Count,
            }];
        });
        // Two events for b.rs and one for a.rs; unsorted and with a duplicate
        // path that must collapse before the count is taken.
        let events = positioned(vec![
            json!({"path": "b.rs"}),
            json!({"path": "a.rs"}),
            json!({"path": "b.rs"}),
        ]);
        let mut out: Vec<u8> = Vec::new();
        let outcome = flush_positioned(&mut out, events, &shape, &[]).unwrap();
        assert_eq!(outcome.emitted, 2, "two groups survive");
        assert_eq!(outcome.dropped, 1, "the duplicate path is dropped");
        assert_eq!(outcome.aggregated, 2, "dedupe happens before the count");
        let lines: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        // Groups keep first-seen order because no sort was requested.
        assert_eq!(lines[0]["path"], "b.rs");
        assert_eq!(lines[0]["count"], 1);
        assert_eq!(lines[1]["path"], "a.rs");
        assert_eq!(lines[1]["count"], 1);
    }

    #[test]
    fn flush_positioned_join_then_filter_then_limit() {
        let shape = shape_with(|s| {
            s.joins = vec![JoinSpec {
                key: "owner".into(),
                suffix: "user".into(),
            }];
            s.filters = vec![parse_filter_expr("ok=true").unwrap()];
            s.limit = Some(1);
        });
        let events = positioned(vec![
            json!({"owner": "alice", "ok": false}),
            json!({"owner": "alice", "ok": true}),
            json!({"owner": "alice", "ok": true}),
        ]);
        let mut out: Vec<u8> = Vec::new();
        let outcome = flush_positioned(&mut out, events, &shape, &[]).unwrap();
        assert_eq!(outcome.emitted, 1, "--limit caps the emitted events");
        assert_eq!(outcome.dropped, 2);
        assert!(
            outcome.limit_hit,
            "the pass reports the limit so the writer decides what it means"
        );
        let v: Value = serde_json::from_str(String::from_utf8(out).unwrap().trim_end()).unwrap();
        assert_eq!(v["ok"], true);
        // The join table was empty, so the suffix stays absent rather than null.
        assert!(v.get("user").is_none());
    }

    #[test]
    fn flush_positioned_diff_emits_only_the_summary() {
        let dir = std::env::temp_dir().join(format!("atomwrite-diff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let baseline_path = dir.join("baseline.ndjson");
        std::fs::write(&baseline_path, "{\"id\":1}\n{\"id\":2}\n").unwrap();

        let shape = shape_with(|s| s.diff_against = Some(baseline_path.clone()));
        let events = positioned(vec![json!({"id": 2}), json!({"id": 3})]);
        let mut out: Vec<u8> = Vec::new();
        let outcome = flush_positioned(&mut out, events, &shape, &[]).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(outcome.emitted, 1, "diff mode emits the summary only");
        assert_eq!(
            outcome.diff,
            Some(StreamDiff {
                added: 1,
                removed: 1,
                unchanged: 1
            })
        );
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 1);
        let v: Value = serde_json::from_str(text.trim_end()).unwrap();
        assert_eq!(v["type"], "stream_diff");
        assert_eq!(v["added"], 1);
        assert_eq!(v["removed"], 1);
        assert_eq!(v["unchanged"], 1);
    }

    #[test]
    fn flush_positioned_reports_a_missing_baseline_as_an_error() {
        let shape = shape_with(|s| {
            s.diff_against =
                Some(std::path::PathBuf::from("/nonexistent/atomwrite-baseline-absent.ndjson"));
        });
        let mut out: Vec<u8> = Vec::new();
        let err = flush_positioned(&mut out, Vec::new(), &shape, &[]).unwrap_err();
        assert!(
            format!("{err:#}").contains("--diff-against"),
            "the error must name the flag: {err:#}"
        );
        assert!(out.is_empty(), "no partial payload on the data channel");
    }

    #[test]
    fn buffering_predicates_match_the_operation_set() {
        // Value-only ops must NOT force buffering: they run per event.
        let value_only = shape_with(|s| {
            s.redact = true;
            s.normalize_encoding = true;
            s.max_output_bytes = Some(10);
            s.truncate_specs = vec![("content".into(), 4)];
        });
        assert!(!value_only.is_identity());
        assert!(!value_only.is_buffering());
        assert!(!value_only.is_tail_flush());

        // Stream-level ops must buffer and flush at the tail.
        for mutate in [
            |s: &mut OutputShape| s.sort = Some(SortKey::Key),
            |s: &mut OutputShape| s.dedupe_keys = vec!["path".into()],
            |s: &mut OutputShape| {
                s.aggregates = vec![AggSpec {
                    group: "path".into(),
                    op: AggOp::Count,
                }]
            },
            |s: &mut OutputShape| s.diff_against = Some(std::path::PathBuf::from("b.ndjson")),
        ] as [fn(&mut OutputShape); 4]
        {
            let mut shape = OutputShape::default();
            mutate(&mut shape);
            assert!(shape.is_buffering(), "must buffer: {shape:?}");
            assert!(shape.is_tail_flush(), "must flush at the tail: {shape:?}");
        }
    }

    #[test]
    fn envelope_key_floor_redacts_secrets_without_any_flag() {
        let mut v = json!({
            "ok": true,
            "api_token": "leaked-by-forgetting-the-flag",
            "Authorization": "Bearer abc",
            "access-token": "x",
            "path": "src/main.rs",
            "count": 3,
        });
        // No shape, no flag: the floor is unconditional.
        assert_eq!(scrub_envelope_keys(&mut v), 3);
        assert_eq!(v["api_token"], REDACTED);
        assert_eq!(v["Authorization"], REDACTED);
        assert_eq!(v["access-token"], REDACTED);
        assert_eq!(v["path"], "src/main.rs");
        assert_eq!(v["count"], 3);
    }

    #[test]
    fn envelope_key_floor_keeps_ordinary_data_keys() {
        // Equality on the folded name, not substring: these describe data.
        let mut v = json!({
            "token_count": 12,
            "secret_count": 0,
            "password_policy": "strong",
            "keys_scanned": 40,
            "authorization_mode": "none",
        });
        assert_eq!(scrub_envelope_keys(&mut v), 0);
        assert_eq!(v["token_count"], 12);
        assert_eq!(v["secret_count"], 0);
        assert_eq!(v["password_policy"], "strong");
        assert_eq!(v["authorization_mode"], "none");
    }

    #[test]
    fn envelope_key_floor_walks_nested_objects_and_arrays() {
        let mut v = json!({
            "error": {"api_token": "nested-leak", "message": "kept"},
            "attempts": [{"password": "p"}, {"password_policy": "kept"}],
        });
        let redacted = scrub_envelope_keys(&mut v);
        assert_eq!(redacted, 2);
        assert_eq!(v["error"]["api_token"], REDACTED);
        assert_eq!(v["error"]["message"], "kept");
        assert_eq!(v["attempts"][0]["password"], REDACTED);
        assert_eq!(v["attempts"][1]["password_policy"], "kept");
    }

    #[test]
    fn error_envelope_gets_the_floor_even_without_redact_flag() {
        let shape = OutputShape::default();
        let err = shape_error_value(
            json!({"ok": false, "api_key": "leaked", "code": "X"}),
            &shape,
        );
        assert_eq!(err["api_key"], REDACTED);
        assert_eq!(err["code"], "X");
    }

    #[test]
    fn shape_options_carry_each_operation_by_name() {
        let options = ShapeOptions {
            select: Some("ok,path".into()),
            limit: Some(2),
            sort_key: Some("path-line".into()),
            dedupe_keys: vec!["path,line".into()],
            aggregate_exprs: vec!["path:sum:match_count".into()],
            join_exprs: vec!["owner:user".into()],
            truncate_exprs: vec!["content:400".into()],
            redact_secrets: true,
            normalize_encoding: true,
            diff_against: None,
            max_output_bytes: Some(64),
            filters: vec!["ok=true".into()],
            profile: OutputProfile::Full,
            verbose: false,
        };
        let shape = OutputShape::from_options(options).unwrap();
        assert_eq!(shape.select, Some(vec!["ok".into(), "path".into()]));
        assert_eq!(shape.limit, Some(2));
        assert_eq!(shape.sort, Some(SortKey::PathLine));
        assert_eq!(shape.dedupe_keys, vec!["path", "line"]);
        assert_eq!(shape.aggregates.len(), 1);
        assert_eq!(shape.joins.len(), 1);
        assert_eq!(shape.truncate_specs, vec![("content".to_string(), 400)]);
        assert!(shape.redact);
        assert!(shape.normalize_encoding);
        assert_eq!(shape.max_output_bytes, Some(64));
        assert_eq!(shape.filters.len(), 1);
        // Negative: the default description requests nothing.
        assert!(OutputShape::from_options(ShapeOptions::default())
            .unwrap()
            .is_identity());
    }

    #[test]
    fn from_trio_matches_the_legacy_constructor() {
        let from_trio =
            OutputShape::from_options(ShapeOptions::from_trio(Some("ok"), &["ok=false".into()], Some(1)))
                .unwrap();
        let legacy = OutputShape::from_cli(Some("ok"), &["ok=false".into()], Some(1)).unwrap();
        assert_eq!(from_trio.select, legacy.select);
        assert_eq!(from_trio.limit, legacy.limit);
        assert_eq!(from_trio.filters.len(), legacy.filters.len());
    }

    #[test]
    fn stream_diff_output_envelope_is_parseable() {
        let diff = StreamDiff {
            added: 2,
            removed: 1,
            unchanged: 7,
        };
        let line = serde_json::to_string(&StreamDiffOutput::from_diff(&diff)).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["type"], "stream_diff");
        assert_eq!(v["added"], 2);
        assert_eq!(v["removed"], 1);
        assert_eq!(v["unchanged"], 7);
    }
}
