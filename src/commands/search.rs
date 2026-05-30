// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parallel file content search powered by the ripgrep engine.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
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
pub fn cmd_search(
    args: &SearchArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = Instant::now();

    let matcher = build_matcher(args)?;

    let walker = build_walker(args, global)?;

    let (tx, rx) = crossbeam_channel::unbounded::<SearchEvent>();

    let files_visited = Arc::new(AtomicU64::new(0));
    let files_matched = Arc::new(AtomicU64::new(0));
    let total_matches = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let fm = Arc::clone(&files_matched);
    let tm = Arc::clone(&total_matches);

    let context_lines = args.context;
    let invert = args.invert;
    let max_count = args.max_count;

    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let matcher = matcher.clone();
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let fm = Arc::clone(&fm);
            let tm = Arc::clone(&tm);

            let mut searcher = SearcherBuilder::new()
                .line_number(true)
                .invert_match(invert)
                .before_context(context_lines)
                .after_context(context_lines)
                .max_matches(max_count)
                .build();

            Box::new(move |entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };

                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    return ignore::WalkState::Continue;
                }

                fv.fetch_add(1, Ordering::Relaxed);

                let path = entry.path().to_path_buf();
                let mut file_matches = 0u64;
                let mut file_lines = 0u64;

                let _ = tx.send(SearchEvent::Begin(path.clone()));

                let mut sink = SearchSink {
                    matcher: &matcher,
                    path: &path,
                    tx: &tx,
                    file_matches: &mut file_matches,
                    file_lines: &mut file_lines,
                };

                let sink_result = searcher.search_path(&matcher, &path, &mut sink);

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

    for event in rx {
        if shutdown.is_shutdown() {
            break;
        }

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
                has_matches = true;

                if args.count {
                    continue;
                }

                if args.files {
                    writer.write_event(&SearchFile {
                        r#type: "file",
                        path: path.display().to_string(),
                    })?;
                    continue;
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
                if args.count && matches > 0 {
                    writer.write_event(&SearchCount {
                        r#type: "count",
                        path: path.display().to_string(),
                        count: matches,
                    })?;
                }

                if !args.count && !args.files && matches > 0 {
                    writer.write_event(&SearchEnd {
                        r#type: "end",
                        path: path.display().to_string(),
                        stats: FileStats {
                            matches,
                            lines_searched,
                        },
                    })?;
                }
            }
        }
    }

    let _ = walker_thread.join();

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
    Begin(std::path::PathBuf),
    Match {
        path: std::path::PathBuf,
        line_number: u64,
        lines: String,
        byte_offset: u64,
        submatches: Vec<Submatch>,
    },
    Context {
        path: std::path::PathBuf,
        line_number: u64,
        lines: String,
    },
    End {
        path: std::path::PathBuf,
        matches: u64,
        lines_searched: u64,
    },
}

struct SearchSink<'a> {
    matcher: &'a grep_regex::RegexMatcher,
    path: &'a std::path::PathBuf,
    tx: &'a crossbeam_channel::Sender<SearchEvent>,
    file_matches: &'a mut u64,
    file_lines: &'a mut u64,
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

        let line_text = std::str::from_utf8(mat.bytes()).unwrap_or("");
        let subs = extract_submatches(self.matcher, line_text);

        let _ = self.tx.send(SearchEvent::Match {
            path: self.path.clone(),
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
        let line_text = std::str::from_utf8(ctx.bytes()).unwrap_or("");

        let _ = self.tx.send(SearchEvent::Context {
            path: self.path.clone(),
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

    builder
        .build(&args.pattern)
        .with_context(|| format!("invalid search pattern: {}", args.pattern))
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
