fn emit_search_event(
    event: SearchEvent,
    writer: &mut NdjsonWriter<impl Write>,
    args: &crate::cli::SearchArgs,
    has_matches: &mut bool,
) -> Result<()> {
    match event {
        SearchEvent::Begin(path) => {
            if !args.count && !args.files {
                writer.write_event(&SearchBegin {
                    r#type: "begin",
                    path: path.display().to_string(),
                })?;
            }
        }
        SearchEvent::Match {
            path,
            line_number,
            lines,
            byte_offset,
            submatches,
        } => {
            *has_matches = true;
            if args.count || args.files {
                return Ok(());
            }
            writer.write_event(&SearchMatch {
                r#type: "match",
                path: path.display().to_string(),
                line_number,
                lines,
                byte_offset,
                submatches,
            })?;
        }
        SearchEvent::Context {
            path,
            line_number,
            lines,
        } => {
            if !args.count && !args.files {
                writer.write_event(&SearchContext {
                    r#type: "context",
                    path: path.display().to_string(),
                    line_number,
                    lines,
                })?;
            }
        }
        SearchEvent::End {
            path,
            matches,
            lines_searched,
        } => {
            let path_str = path.display().to_string();
            if args.files && matches > 0 {
                writer.write_event(&SearchFile {
                    r#type: "file",
                    path: path_str.clone(),
                })?;
            }
            if args.count && matches > 0 {
                writer.write_event(&SearchCount {
                    r#type: "count",
                    path: path_str.clone(),
                    count: matches,
                })?;
            }
            if !args.count && !args.files && matches > 0 {
                writer.write_event(&SearchEnd {
                    r#type: "end",
                    path: path_str,
                    stats: FileStats {
                        matches,
                        lines_searched,
                    },
                })?;
            }
        }
    }
    Ok(())
}

struct SearchSink<'a> {
    matcher: &'a grep_regex::RegexMatcher,
    path: Arc<std::path::Path>,
    tx: &'a crossbeam_channel::Sender<SearchEvent>,
    file_matches: &'a mut u64,
    file_lines: &'a mut u64,
    max_columns: usize,
}

impl<'a> Sink for SearchSink<'a> {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        *self.file_lines += 1;
        *self.file_matches += 1;

        let raw_line = std::str::from_utf8(mat.bytes()).unwrap_or("");
        // G68: truncate lines longer than --max-columns to avoid blowing up
        // LLM context windows with minified bundle.js / styles.min.css.
        let line_text = if raw_line.len() > self.max_columns {
            // Find a safe UTF-8 boundary near max_columns.
            let mut cut = self.max_columns;
            while cut > 0 && !raw_line.is_char_boundary(cut) {
                cut -= 1;
            }
            format!("{}...[truncated]", &raw_line[..cut])
        } else {
            raw_line.to_owned()
        };
        let subs = extract_submatches(self.matcher, &line_text);

        // Receiver may have dropped during shutdown — send failure is expected
        let _ = self.tx.send(SearchEvent::Match {
            path: Arc::clone(&self.path),
            line_number: mat.line_number().unwrap_or(0),
            lines: line_text.trim_end_matches('\n').to_owned(),
            byte_offset: mat.absolute_byte_offset(),
            submatches: subs,
        });

        Ok(true)
    }

    fn context(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        let raw_line = std::str::from_utf8(ctx.bytes()).unwrap_or("");
        // G68: also truncate context lines.
        let line_text = if raw_line.len() > self.max_columns {
            let mut cut = self.max_columns;
            while cut > 0 && !raw_line.is_char_boundary(cut) {
                cut -= 1;
            }
            format!("{}...[truncated]", &raw_line[..cut])
        } else {
            raw_line.to_owned()
        };

        let _ = self.tx.send(SearchEvent::Context {
            path: Arc::clone(&self.path),
            line_number: ctx.line_number().unwrap_or(0),
            lines: line_text.trim_end_matches('\n').to_owned(),
        });

        Ok(true)
    }
}

fn build_matcher(args: &SearchArgs) -> Result<grep_regex::RegexMatcher> {
    let mut builder = RegexMatcherBuilder::new();

    if args.case_insensitive {
        builder.case_insensitive(true);
    }

    if args.smart_case {
        builder.case_smart(true);
    }

    if args.word {
        builder.word(true);
    }

    if args.multiline {
        builder.multi_line(true);
    }

    if args.fixed {
        builder.fixed_strings(true);
    }

    builder.build(&args.pattern).map_err(|e| {
        crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid search pattern '{}': {e}", args.pattern),
        }
        .into()
    })
}

fn build_walker(
    args: &SearchArgs,
    canonical_paths: &[std::path::PathBuf],
    global: &GlobalArgs,
) -> Result<ignore::WalkBuilder> {
    let mut builder = ignore::WalkBuilder::new(&canonical_paths[0]);

    for path in canonical_paths.iter().skip(1) {
        builder.add(path);
    }

    builder
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore)
        .follow_links(global.follow_symlinks);

    // Always apply the shared bound (default = all cores, RAM-capped).
    crate::concurrency::apply_walk_threads(&mut builder, global.threads);

    if !args.include.is_empty() || !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&canonical_paths[0]);
        for glob in &args.include {
            overrides.add(glob)?;
        }
        for glob in &args.exclude {
            overrides.add(&format!("!{glob}"))?;
        }
        builder.overrides(overrides.build()?);
    }

    if let Some(SortBy::Path) = &args.sort {
        builder.sort_by_file_path(|a, b| a.cmp(b));
    }

    Ok(builder)
}

fn extract_submatches(matcher: &grep_regex::RegexMatcher, line: &str) -> Vec<Submatch> {
    let mut subs = Vec::with_capacity(4);
    let _ = matcher.find_iter(line.as_bytes(), |m| {
        let matched_text = &line[m.start()..m.end()];
        subs.push(Submatch {
            r#match: matched_text.to_owned(),
            start: m.start(),
            end: m.end(),
        });
        true
    });
    subs
}
