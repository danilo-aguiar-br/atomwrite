// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parallel file content search powered by the ripgrep engine.
//! Workload: I/O-bound (file reading + regex matching via ripgrep engine).

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::Result;
use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{SearcherBuilder, Sink, SinkContext, SinkMatch};

use crate::cli::{GlobalArgs, SearchArgs};
use crate::cli_args::SortBy;
use crate::ndjson_types::{
    FileStats, SearchBegin, SearchContext, SearchCount, SearchEnd, SearchFile, SearchMatch,
    Submatch, Summary,
};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Search file contents in parallel using the ripgrep engine.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if reading files fails.
#[tracing::instrument(skip_all, fields(command = "search"))]
pub fn cmd_search(
    args: &SearchArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = Instant::now();

    let matcher = build_matcher(args)?;

    let walker = build_walker(args, global)?;

    let (tx, rx) = crossbeam_channel::bounded::<SearchEvent>(1024);

    let files_visited = Arc::new(AtomicU64::new(0));
    let files_matched = Arc::new(AtomicU64::new(0));
    let total_matches = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let fm = Arc::clone(&files_matched);
    let tm = Arc::clone(&total_matches);

    let context_lines = args.context;
    let invert = args.invert;
    let max_count = args.max_count;
    let include_fifo = args.include_fifo;
    let max_filesize = args.max_filesize;
    let max_columns = args.max_columns;

    let shutdown_flag = shutdown.flag();
    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let matcher = matcher.clone();
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let fm = Arc::clone(&fm);
            let tm = Arc::clone(&tm);
            let shutdown_flag = Arc::clone(&shutdown_flag);

            let mut searcher = SearcherBuilder::new()
                .line_number(true)
                .invert_match(invert)
                .before_context(context_lines)
                .after_context(context_lines)
                .max_matches(max_count)
                .build();

            Box::new(move |entry| {
                if shutdown_flag.load(Ordering::Acquire) {
                    return ignore::WalkState::Quit;
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };

                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    return ignore::WalkState::Continue;
                }

                // G56: skip FIFO/named pipe files unless --include-fifo is set.
                // `open()` on a FIFO blocks indefinitely until the other end
                // connects, which can cause atomwrite to hang in environments
                // that have FIFOs in /tmp or /var (CI, Docker, system dirs).
                if !include_fifo {
                    if let Some(ft) = entry.file_type() {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::FileTypeExt;
                            if ft.is_fifo() {
                                tracing::debug!(
                                    path = %entry.path().display(),
                                    "skipping FIFO (G56); pass --include-fifo to opt in"
                                );
                                return ignore::WalkState::Continue;
                            }
                        }
                        #[cfg(not(unix))]
                        let _ = ft; // no-op on Windows
                    }
                }

                // G68: skip files larger than --max-filesize.
                if let Ok(meta) = entry.metadata() {
                    if meta.len() > max_filesize {
                        tracing::debug!(
                            path = %entry.path().display(),
                            size = meta.len(),
                            max = max_filesize,
                            "skipping oversized file (G68)"
                        );
                        return ignore::WalkState::Continue;
                    }
                }

                fv.fetch_add(1, Ordering::Relaxed);

                let path: Arc<std::path::Path> = Arc::from(entry.path());
                let _span = tracing::debug_span!("process_file", path = %path.display()).entered();
                let mut file_matches = 0u64;
                let mut file_lines = 0u64;

                // Receiver may have dropped during shutdown — send failure is expected
                let _ = tx.send(SearchEvent::Begin(Arc::clone(&path)));

                let mut sink = SearchSink {
                    matcher: &matcher,
                    path: Arc::clone(&path),
                    tx: &tx,
                    file_matches: &mut file_matches,
                    file_lines: &mut file_lines,
                    max_columns,
                };

                let sink_result = searcher.search_path(&matcher, &*path, &mut sink);

                if let Err(e) = sink_result {
                    tracing::warn!(path = %path.display(), error = %e, "search error");
                }

                if file_matches > 0 {
                    fm.fetch_add(1, Ordering::Relaxed);
                    tm.fetch_add(file_matches, Ordering::Relaxed);
                }

                let _ = tx.send(SearchEvent::End {
                    path,
                    matches: file_matches,
                    lines_searched: file_lines,
                });

                ignore::WalkState::Continue
            })
        });
    });

    let mut has_matches = false;
    // Buffer events per path so that parallel walker threads do not interleave
    // Begin/Match/End events for different files in the NDJSON output.
    let mut buffer: std::collections::BTreeMap<std::path::PathBuf, Vec<SearchEvent>> =
        std::collections::BTreeMap::new();
    let mut open_paths: std::collections::BTreeSet<std::path::PathBuf> =
        std::collections::BTreeSet::new();

    for event in rx {
        if shutdown.is_shutdown() {
            break;
        }

        // Track open paths to know when we can flush a complete sequence
        match &event {
            SearchEvent::Begin(p) => {
                open_paths.insert(p.to_path_buf());
                buffer.entry(p.to_path_buf()).or_default().push(event);
            }
            SearchEvent::End { path, .. } => {
                let path_buf = path.to_path_buf();
                buffer.entry(path_buf.clone()).or_default().push(event);
                // Flush all events for this path now that we have End
                if let Some(events) = buffer.remove(&path_buf) {
                    open_paths.remove(&path_buf);
                    for ev in events {
                        emit_search_event(&ev, writer, args, &mut has_matches)?;
                    }
                }
            }
            _ => {
                // For Match/Context, attach to the most recent open path
                if let Some(last) = open_paths.iter().next_back() {
                    buffer.entry(last.clone()).or_default().push(event);
                }
            }
        }
    }

    // Flush any remaining buffered events (e.g. on shutdown)
    for (_, events) in buffer {
        for ev in events {
            emit_search_event(&ev, writer, args, &mut has_matches)?;
        }
    }

    if let Err(panic_payload) = walker_thread.join() {
        std::panic::resume_unwind(panic_payload);
    }

    // If the search was interrupted by SIGINT/SIGTERM, the inner `for event`
    // loop broke out before draining every Begin/End pair, so the buffered
    // Begin events for in-flight files were never flushed and `has_matches`
    // may be `false` even though the parallel walker did find matches
    // (they are still sitting in the `buffer` and may also have been
    // discarded by the break). Returning `Err(NoMatches)` here would be a
    // lie — the search was killed, not empty — and would also cause the
    // main entry point to take the `Err` branch and emit an error JSON
    // envelope instead of the graceful "shutting down" banner that
    // `atomwrite::signal::write_shutdown_message` writes from the main
    // thread. Returning `Ok(())` lets the caller detect the shutdown via
    // `shutdown.is_shutdown()` and emit the banner as designed.
    if shutdown.is_shutdown() {
        return Ok(());
    }

    let summary = Summary {
        r#type: "summary",
        files_visited: files_visited.load(Ordering::Relaxed),
        files_matched: files_matched.load(Ordering::Relaxed),
        files_modified: None,
        files_skipped: None,
        total_matches: Some(total_matches.load(Ordering::Relaxed)),
        total_replacements: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };

    writer.write_event(&summary)?;

    if !has_matches {
        return Err(crate::error::AtomwriteError::NoMatches.into());
    }

    Ok(())
}

enum SearchEvent {
    Begin(Arc<std::path::Path>),
    Match {
        path: Arc<std::path::Path>,
        line_number: u64,
        lines: String,
        byte_offset: u64,
        submatches: Vec<Submatch>,
    },
    Context {
        path: Arc<std::path::Path>,
        line_number: u64,
        lines: String,
    },
    End {
        path: Arc<std::path::Path>,
        matches: u64,
        lines_searched: u64,
    },
}

/// Emit a single search event to the NDJSON writer.
///
/// Centralizes the dispatch logic so the main consumer loop can buffer events
/// per path and emit them contiguously regardless of parallel walker order.
fn emit_search_event(
    event: &SearchEvent,
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
            path: _path,
            line_number,
            lines,
            byte_offset,
            submatches,
        } => {
            *has_matches = true;
            if args.count || args.files {
                return Ok(());
            }
            let path_str = _path.display().to_string();
            writer.write_event(&SearchMatch {
                r#type: "match",
                path: path_str,
                line_number: *line_number,
                lines: lines.clone(),
                byte_offset: *byte_offset,
                submatches: submatches.clone(),
            })?;
        }
        SearchEvent::Context {
            path: _path,
            line_number,
            lines,
        } => {
            if !args.count && !args.files {
                writer.write_event(&SearchContext {
                    r#type: "context",
                    path: _path.display().to_string(),
                    line_number: *line_number,
                    lines: lines.clone(),
                })?;
            }
        }
        SearchEvent::End {
            path,
            matches,
            lines_searched,
        } => {
            let path_str = path.display().to_string();
            if args.files && *matches > 0 {
                writer.write_event(&SearchFile {
                    r#type: "file",
                    path: path_str.clone(),
                })?;
            }
            if args.count && *matches > 0 {
                writer.write_event(&SearchCount {
                    r#type: "count",
                    path: path_str.clone(),
                    count: *matches,
                })?;
            }
            if !args.count && !args.files && *matches > 0 {
                writer.write_event(&SearchEnd {
                    r#type: "end",
                    path: path_str,
                    stats: FileStats {
                        matches: *matches,
                        lines_searched: *lines_searched,
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

fn build_walker(args: &SearchArgs, global: &GlobalArgs) -> Result<ignore::WalkBuilder> {
    let mut builder = ignore::WalkBuilder::new(&args.paths[0]);

    for path in args.paths.iter().skip(1) {
        builder.add(path);
    }

    builder
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore)
        .follow_links(global.follow_symlinks);

    if let Some(threads) = global.threads {
        builder.threads(if threads == 0 { num_cpus() } else { threads });
    }

    if !args.include.is_empty() || !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&args.paths[0]);
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

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
