#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_limited_line_basic_lf() {
        let mut r = Cursor::new(b"{\"a\":1}\n{\"b\":2}\n");
        let mut buf = String::new();
        let n = read_limited_line(&mut r, &mut buf, 1024).unwrap();
        assert!(n > 0);
        assert_eq!(buf, "{\"a\":1}");
        let n = read_limited_line(&mut r, &mut buf, 1024).unwrap();
        assert!(n > 0);
        assert_eq!(buf, "{\"b\":2}");
        assert_eq!(read_limited_line(&mut r, &mut buf, 1024).unwrap(), 0);
    }

    #[test]
    fn read_limited_line_crlf_and_bom() {
        let mut r = Cursor::new(b"\xEF\xBB\xBF{\"x\":1}\r\n");
        let mut buf = String::new();
        read_limited_line(&mut r, &mut buf, 1024).unwrap();
        assert_eq!(buf, "{\"x\":1}");
    }

    #[test]
    fn read_limited_line_rejects_oversize_before_full_buffer() {
        // Line larger than max without needing a multi-GB allocation first.
        let mut payload = vec![b'a'; 64];
        payload.push(b'\n');
        let mut r = Cursor::new(payload);
        let mut buf = String::new();
        let err = read_limited_line(&mut r, &mut buf, 32).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        // Stream recovered: next read is EOF (newline was consumed).
        assert_eq!(read_limited_line(&mut r, &mut buf, 32).unwrap(), 0);
    }

    #[test]
    fn check_json_depth_limits_nesting() {
        let shallow: serde_json::Value = serde_json::from_str(r#"{"a":{"b":1}}"#).expect("fixture");
        assert!(check_json_depth(&shallow, 3));
        assert!(!check_json_depth(&shallow, 2));
        let leaf = serde_json::Value::from(1);
        assert!(check_json_depth(&leaf, 1));
    }

    #[test]
    fn ndjson_insert_field_tags_object() {
        let tagged = ndjson_insert_field(r#"{"type":"x","path":"/t"}"#, "step", 2.into())
            .expect("object line");
        assert_eq!(tagged["step"], 2);
        assert_eq!(tagged["type"], "x");
        assert!(ndjson_insert_field("[]", "step", 1.into()).is_none());
    }

    #[test]
    fn strip_utf8_bom_str_prefix() {
        assert_eq!(strip_utf8_bom_str("\u{FEFF}{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_utf8_bom_str("{\"a\":1}"), "{\"a\":1}");
    }

    fn first_line(buf: &[u8]) -> serde_json::Value {
        let line = buf.split(|b| *b == b'\n').next().expect("one line");
        serde_json::from_slice(line).expect("NDJSON line")
    }

    #[test]
    fn published_failure_carries_verdict_detail_and_marker() {
        let mut buf = Vec::<u8>::new();
        let returned = {
            let mut writer = NdjsonWriter::new(&mut buf);
            let summary = serde_json::json!({"type": "summary", "total_matches": 0});
            writer.publish_failure(&summary, AtomwriteError::NoMatches.into())
        };
        assert!(returned.downcast_ref::<VerdictPublished>().is_some());
        assert!(matches!(
            returned.downcast_ref::<AtomwriteError>(),
            Some(AtomwriteError::NoMatches)
        ));
        assert_eq!(
            error_message(&returned),
            AtomwriteError::NoMatches.to_string()
        );
        let line = first_line(&buf);
        assert_eq!(line["type"], "summary");
        assert_eq!(line["total_matches"], 0);
        assert_eq!(line["ok"], false);
        assert_eq!(line["code"], "NO_MATCHES");
        assert_eq!(line["exit"], 1);
        assert_eq!(line["error_class"], "permanent");
        assert_eq!(line["retryable"], false);
        assert_eq!(line["match_count"], 0);
        assert!(line["suggestion_lang"].is_string());
        assert!(
            line.get("error").is_none(),
            "not a standalone error envelope"
        );
        assert_eq!(buf.iter().filter(|b| **b == b'\n').count(), 1);
    }

    #[test]
    fn published_failure_keeps_conflict_count_and_event_keys() {
        let mut buf = Vec::<u8>::new();
        {
            let mut writer = NdjsonWriter::new(&mut buf);
            let event = serde_json::json!({"type": "semantic_merge", "path": "own"});
            let err = AtomwriteError::MergeConflict {
                count: 3,
                path: "/out".into(),
            };
            let _ = writer.publish_failure(&event, err.into());
        }
        let line = first_line(&buf);
        assert_eq!(line["code"], "MERGE_CONFLICT");
        assert_eq!(line["conflict_count"], 3);
        assert_eq!(
            line["path"], "own",
            "the event's own key is not overwritten"
        );
    }

    #[test]
    fn published_failure_ignores_limit_and_honors_select() {
        let mut buf = Vec::<u8>::new();
        {
            let shape = OutputShape {
                limit: Some(0),
                select: Some(vec!["ok".into(), "code".into()]),
                ..OutputShape::default()
            };
            let mut writer = NdjsonWriter::with_shape(&mut buf, shape);
            let summary = serde_json::json!({"type": "summary", "total_matches": 0});
            let _ = writer.publish_failure(&summary, AtomwriteError::NoMatches.into());
        }
        let line = first_line(&buf);
        // `--select` thins the envelope but never below what the consumer reads
        // the verdict off: the discriminator and the `ok` flag come back from
        // the untouched record. Without them the failure line stopped naming
        // itself, which is the defect this assertion now guards.
        assert_eq!(
            line,
            serde_json::json!({"type": "summary", "ok": false, "code": "NO_MATCHES"})
        );
    }

    #[test]
    fn published_failure_on_a_closed_pipe_returns_the_write_error() {
        struct ClosedPipe;
        impl Write for ClosedPipe {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
        }
        let mut writer = NdjsonWriter::new(ClosedPipe);
        let summary = serde_json::json!({"type": "summary"});
        let returned = writer.publish_failure(&summary, AtomwriteError::NoMatches.into());
        assert!(returned.downcast_ref::<VerdictPublished>().is_none());
        assert!(matches!(
            returned.downcast_ref::<AtomwriteError>(),
            Some(AtomwriteError::BrokenPipe)
        ));
    }

    #[test]
    fn terminal_event_names_the_target_only_when_it_was_set() {
        let mut buf = Vec::<u8>::new();
        {
            let mut writer = NdjsonWriter::new(&mut buf);
            writer
                .write_event(&serde_json::json!({"type": "summary"}))
                .expect("write");
            writer.set_target("/ws/a.txt".into(), "argv");
            writer
                .write_event(&serde_json::json!({"type": "summary"}))
                .expect("write");
            writer
                .write_event(&serde_json::json!({"type": "match"}))
                .expect("write");
        }
        let lines: Vec<serde_json::Value> = buf
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_slice(l).expect("NDJSON line"))
            .collect();
        assert!(lines[0].get("target_resolved").is_none());
        assert_eq!(lines[1]["target_resolved"], "/ws/a.txt");
        assert_eq!(lines[1]["target_source"], "argv");
        assert!(lines[2].get("target_resolved").is_none());
    }

    #[test]
    fn dotted_to_json_pointer_escapes_rfc6901() {
        assert_eq!(dotted_to_json_pointer("a.b.c"), "/a/b/c");
        assert_eq!(dotted_to_json_pointer("a/b"), "/a~1b");
        assert_eq!(dotted_to_json_pointer("a~b"), "/a~0b");
        assert_eq!(dotted_to_json_pointer("x.y/z~w"), "/x/y~1z~0w");
    }
}
