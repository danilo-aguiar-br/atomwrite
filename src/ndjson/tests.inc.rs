#[cfg(test)]
mod tests {
    use super::*;

    fn assert_valid_ndjson_object<T: serde::Serialize>(val: &T) {
        let json = serde_json::to_value(val).expect("serialize to Value");
        assert!(json.is_object(), "expected JSON object, got: {json}");
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("type"), "missing 'type' field");
    }

    fn assert_roundtrip_json<T: serde::Serialize>(val: &T) {
        let json_str = serde_json::to_string(val).expect("serialize to string");
        let reparsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("reparse from string");
        assert!(reparsed.is_object(), "roundtrip produced non-object");
    }

    #[test]
    fn roundtrip_write_output() {
        let val = WriteOutput {
            r#type: "write",
            status: "ok",
            path: "/tmp/test.rs".into(),
            bytes_written: 42,
            checksum: "abc123".into(),
            checksum_before: None,
            backup_path: None,
            elapsed_ms: 5,
            stdin_bytes_read: 42,
            wal_policy: "auto",
            platform: PlatformInfo {
                fsync: "sync_data",
                dir_fsync: "sync_all",
                durability: None,
                rename_method: None,
                backup_method: None,
            },
            mtime_preserved: None,
            risk_assessment: None,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_batch_summary() {
        let val = BatchSummary {
            r#type: "summary",
            operations: 10,
            succeeded: 9,
            failed: 1,
            dry_run: false,
            elapsed_ms: 100,
            transaction: Some(true),
            committed: Some(false),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_diff_stat() {
        let val = DiffStatOutput {
            r#type: "diff",
            identical: false,
            file_a: "a.rs".into(),
            file_b: "b.rs".into(),
            insertions: 10,
            deletions: 5,
            similarity_ratio: 0.85,
            elapsed_ms: 3,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_summary() {
        let val = Summary {
            r#type: "summary",
            files_visited: 100,
            files_matched: 5,
            files_modified: Some(3),
            files_skipped: None,
            total_matches: Some(42),
            total_replacements: None,
            elapsed_ms: 200,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_edit_output() {
        let val = EditOutput {
            r#type: "edit",
            path: "/tmp/edit.rs".into(),
            edits: 1,
            mode: "old_new".into(),
            bytes_before: 100,
            bytes_after: 110,
            checksum_before: "aaa".into(),
            checksum_after: "bbb".into(),
            lines_before: 10,
            lines_after: 11,
            elapsed_ms: 2,
            fuzzy: Some(true),
            strategy: Some("block_anchor".into()),
            strategies_tried: Some(8),
            similarity: Some(0.95),
            diff_preview: None,
            pairs_total: Some(2),
            pair_results: Some(vec![PairResult {
                index: 1,
                matched: true,
                strategy: Some("exact".into()),
                similarity: None,
                source: None,
            }]),
            mtime_preserved: Some(false),
            match_count: Some(1),
            indent_adjusted: Some(false),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_diff_summary() {
        let val = DiffSummaryOutput {
            r#type: "summary",
            identical: true,
            file_a: "x.rs".into(),
            file_b: "y.rs".into(),
            lines_a: 50,
            lines_b: 50,
            similarity_ratio: 1.0,
            elapsed_ms: 1,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn skip_serializing_if_omits_none_fields() {
        let val = WriteOutput {
            r#type: "write",
            status: "ok",
            path: "/tmp/t.rs".into(),
            bytes_written: 10,
            checksum: "x".into(),
            checksum_before: None,
            backup_path: None,
            elapsed_ms: 1,
            stdin_bytes_read: 10,
            wal_policy: "auto",
            platform: PlatformInfo {
                fsync: "sync_data",
                dir_fsync: "best_effort",
                durability: None,
                rename_method: None,
                backup_method: None,
            },
            mtime_preserved: None,
            risk_assessment: None,
        };
        let json = serde_json::to_value(&val).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("checksum_before"));
        assert!(!obj.contains_key("backup_path"));
        assert!(!obj.contains_key("risk_assessment"));
    }

    #[test]
    fn roundtrip_read_output() {
        let val = ReadOutput {
            r#type: "read",
            path: "/tmp/read.rs".into(),
            content: Some("hello".into()),
            lines: 1,
            lines_total: None,
            bytes: 5,
            checksum: "abc".into(),
            permissions: "0644".into(),
            modified: "2026-01-01T00:00:00Z".into(),
            kind: "file".into(),
            binary: false,
            range: None,
            verified: None,
            mode: "full".into(),
            content_b64: None,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_search_match() {
        let val = SearchMatch {
            r#type: "match",
            path: "/tmp/s.rs".into(),
            line_number: 10,
            lines: "fn main()".into(),
            byte_offset: 42,
            submatches: vec![],
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_replace_result() {
        let val = ReplaceResult {
            r#type: "replaced",
            path: "/tmp/r.rs".into(),
            replacements: 3,
            bytes_before: 100,
            bytes_after: 110,
            checksum_before: "aaa".into(),
            checksum_after: "bbb".into(),
            elapsed_ms: 5,
            mtime_preserved: Some(false),
            fuzzy: None,
            strategy: None,
            similarity: None,
            strategies_tried: None,
            word_ignored: None,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_transform_result() {
        let val = TransformResult {
            r#type: "transform",
            path: "/tmp/t.rs".into(),
            language: "rust".into(),
            matches: 2,
            replacements: 2,
            bytes_before: 50,
            bytes_after: 55,
            checksum_before: "aa".into(),
            checksum_after: "bb".into(),
            elapsed_ms: 3,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_scope_result() {
        let val = ScopeResult {
            r#type: "scope",
            path: "/tmp/sc.rs".into(),
            language: "rust".into(),
            query: "comments".into(),
            action: "delete".into(),
            scopes_matched: 5,
            bytes_before: 200,
            bytes_after: 180,
            checksum_before: "x".into(),
            checksum_after: "y".into(),
            elapsed_ms: 10,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_backup_result() {
        let val = BackupResult {
            r#type: "backup",
            path: "/tmp/src.rs".into(),
            backup_path: "/tmp/src.rs.bak".into(),
            checksum: "hash".into(),
            bytes: 500,
            elapsed_ms: 2,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_rollback_result() {
        let val = RollbackResult {
            r#type: "rollback",
            path: "/tmp/rb.rs".into(),
            restored_from: "/tmp/rb.rs.bak".into(),
            checksum_before: Some("old".into()),
            checksum_after: "new".into(),
            verified: Some(true),
            elapsed_ms: 3,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_apply_result() {
        let val = ApplyResult {
            r#type: "apply",
            path: "/tmp/ap.rs".into(),
            format_detected: "unified".into(),
            hunks_applied: 2,
            bytes_before: 100,
            bytes_after: 120,
            checksum_before: "a".into(),
            checksum_after: "b".into(),
            elapsed_ms: 4,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_hash_output() {
        let val = HashOutput {
            r#type: "hash",
            path: Some("/tmp/h.rs".into()),
            source: None,
            algorithm: "blake3",
            value: "blake3hash".into(),
            bytes: Some(1024),
            verified: None,
            elapsed_ms: 1,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_calc_output() {
        let val = CalcOutput {
            r#type: "calc",
            expression: "2+2".into(),
            result: "4".into(),
            elapsed_ms: 1,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_regex_output() {
        let val = RegexOutput {
            r#type: "regex",
            regex: "\\d+".into(),
            examples: 3,
            anchored: false,
            elapsed_ms: 1,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_list_entry() {
        let val = ListEntry {
            r#type: "entry",
            path: "/tmp/le.rs".into(),
            kind: "file".into(),
            size: Some(100),
            modified: None,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_list_summary() {
        let val = ListSummary {
            r#type: "summary",
            files: 10,
            dirs: 3,
            symlinks: 0,
            total_bytes: Some(5000),
            by_extension: None,
            elapsed_ms: 15,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_dry_run_plan() {
        let val = DryRunPlan {
            r#type: "plan",
            operation: "write".into(),
            path: "/tmp/dr.rs".into(),
            would_modify: true,
            details: None,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_copy_output() {
        let val = CopyOutput {
            r#type: "copy",
            source: "/tmp/a.rs".into(),
            target: "/tmp/b.rs".into(),
            bytes: 200,
            checksum: "hash".into(),
            verified: true,
            elapsed_ms: 2,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_move_output() {
        let val = MoveOutput {
            r#type: "move",
            source: "/tmp/old.rs".into(),
            target: "/tmp/new.rs".into(),
            bytes: 300,
            checksum: "mhash".into(),
            cross_device: false,
            atomic: true,
            backup_path: None,
            elapsed_ms: 3,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_delete_output() {
        let val = DeleteOutput {
            r#type: "delete",
            path: "/tmp/del.rs".into(),
            bytes: 150,
            checksum_before: "dhash".into(),
            elapsed_ms: 1,
            warnings: vec![],
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_count_total_output() {
        let val = CountTotalOutput {
            r#type: "count",
            mode: "total",
            total: CountTotals {
                files: 50,
                lines: 2000,
                blank: 300,
                bytes: 50000,
            },
            elapsed_ms: 20,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_search_begin() {
        let val = SearchBegin {
            r#type: "begin",
            path: "/tmp/proj".into(),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_search_end() {
        let val = SearchEnd {
            r#type: "end",
            path: "/tmp/proj".into(),
            stats: FileStats {
                matches: 12,
                lines_searched: 50,
            },
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_transfer_plan() {
        let val = TransferPlan {
            r#type: "plan",
            operation: "copy",
            source: "/tmp/a.rs".into(),
            target: "/tmp/b.rs".into(),
            would_modify: true,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_backup_plan() {
        let val = BackupPlan {
            r#type: "plan",
            operation: "backup",
            path: "/tmp/src.rs".into(),
            bytes: 500,
            checksum: "hash".into(),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_rollback_plan() {
        let val = RollbackPlan {
            r#type: "plan",
            operation: "rollback",
            path: "/tmp/rb.rs".into(),
            restore_from: "/tmp/rb.bak".into(),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_replace_preview() {
        let val = ReplacePreview {
            r#type: "preview",
            path: "/tmp/rp.rs".into(),
            replacements: 3,
            diff: "-old\n+new".into(),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_file_match_event() {
        let val = FileMatchEvent {
            r#type: "file_match",
            path: "/tmp/a.rs".into(),
            name: "a.rs".into(),
            target: "files",
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_watch_and_semantic_events() {
        let watch = WatchEvent {
            r#type: "watch",
            path: "/tmp/w.rs".into(),
            kind: "Modify(Data)".into(),
            checksum: Some("abc".into()),
        };
        assert_valid_ndjson_object(&watch);
        assert_roundtrip_json(&watch);

        let hit = SemanticMatchEvent {
            r#type: "semantic_match",
            rank: 1,
            score: 0.5,
            path: "/tmp/s.rs".into(),
            line: 10,
            snippet: "fn main()".into(),
            backend: "jaccard",
        };
        assert_valid_ndjson_object(&hit);
        assert_roundtrip_json(&hit);

        let summary = SemanticSummaryEvent {
            r#type: "semantic_summary",
            query: "main".into(),
            k: 20,
            results: 1,
            backend: "jaccard",
        };
        assert_valid_ndjson_object(&summary);
        assert_roundtrip_json(&summary);
    }

    #[test]
    fn roundtrip_query_recipe_codemod_sparse_events() {
        let kind = QueryKindEvent {
            r#type: "query_kind",
            path: "/t.rs".into(),
            language: "rust".into(),
            kind: "function_item".into(),
            count: 3,
        };
        assert_valid_ndjson_object(&kind);
        assert_roundtrip_json(&kind);

        let qmatch = QueryMatchEvent {
            r#type: "query_match",
            path: "/t.rs".into(),
            language: "rust".into(),
            kind: "function_item".into(),
            is_named: true,
            text: "fn a() {}".into(),
            capture_name: Some("name".into()),
            start_byte: Some(0),
            end_byte: Some(9),
            start_line: Some(1),
            start_column: Some(1),
            end_line: Some(1),
            end_column: Some(10),
        };
        assert_valid_ndjson_object(&qmatch);
        assert_roundtrip_json(&qmatch);

        let list = RecipeListEvent {
            r#type: "recipe",
            name: "search-replace-verify",
            builtin: true,
        };
        assert_valid_ndjson_object(&list);
        assert_roundtrip_json(&list);

        let step = RecipeStepEvent {
            r#type: "recipe_step",
            step: 1,
            name: "search".into(),
            status: "ok".into(),
            detail: "done".into(),
            checksum: None,
        };
        assert_valid_ndjson_object(&step);
        assert_roundtrip_json(&step);

        let start = CodemodStartEvent {
            r#type: "codemod",
            phase: "start",
            rules: "rules.yml".into(),
            rule_ids: vec!["r1".into()],
            rule_id: "rules".into(),
            dry_run: true,
        };
        assert_valid_ndjson_object(&start);
        assert_roundtrip_json(&start);

        let mut by = std::collections::BTreeMap::new();
        by.insert(
            "r1".into(),
            CodemodRuleStats {
                matches: 2,
                files: 1,
            },
        );
        let csum = CodemodSummaryEvent {
            r#type: "codemod_summary",
            rules: "rules.yml".into(),
            rule_id: "rules".into(),
            dry_run: true,
            by_rule_id: by,
        };
        assert_valid_ndjson_object(&csum);
        assert_roundtrip_json(&csum);

        let begin = TransformRuleBegin {
            r#type: "rule_begin",
            id: Some("r1".into()),
            language: "rust".into(),
        };
        assert_valid_ndjson_object(&begin);
        assert_roundtrip_json(&begin);

        let rerr = TransformRuleError {
            r#type: "rule_error",
            id: None,
            error: "boom".into(),
        };
        assert_valid_ndjson_object(&rerr);
        assert_roundtrip_json(&rerr);

        let sread = SparseReadEvent {
            r#type: "sparse_read",
            path: "/a.rs".into(),
            head: "fn main() {}".into(),
            lines: 50,
        };
        assert_valid_ndjson_object(&sread);
        assert_roundtrip_json(&sread);

        let budget = SparseOutlineBudget {
            r#type: "sparse_outline_budget",
            files_seen: 3,
            items: 10,
            truncated: false,
            max_files: 50,
        };
        assert_valid_ndjson_object(&budget);
        assert_roundtrip_json(&budget);

        let tok = SemanticIndexToken {
            t: "main".into(),
            p: "/a.rs".into(),
            l: 1,
            s: "fn main".into(),
        };
        // Index tokens are not agent stdout envelopes (no `type`); still serialize.
        let s = serde_json::to_string(&tok).expect("ser");
        assert!(s.contains("\"t\":\"main\""));
    }
}
