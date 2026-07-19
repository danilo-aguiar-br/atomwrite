// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parallel file content search powered by the ripgrep engine.
//!
//! Workload: I/O-bound (file reading + regex matching via ripgrep engine).
//! Parallelism: content search uses `ignore::WalkParallel` + bounded
//! `crossbeam-channel` (`concurrency::EVENT_CHANNEL_CAP`). `--target files`
//! collects via `collect_files_parallel` (honors `--threads`), sorts, then
//! emits stable NDJSON. Bound via `--threads`/`--max-concurrency`.

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

    if args.pcre2 {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "PCRE2 not available; rebuild with --features pcre2".into(),
        }
        .into());
    }

    let workspace = global.resolve_workspace()?;
    let canonical_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;

    use crate::cli_args::SearchTarget;
    if matches!(args.target, SearchTarget::Files | SearchTarget::Both) {
        let pat = if args.case_insensitive || args.smart_case {
            args.pattern.to_ascii_lowercase()
        } else {
            args.pattern.clone()
        };
        // WalkParallel collect + parallel name filter; sort for stable NDJSON.
        let walker = build_walker(args, &canonical_paths, global)?;
        let files = crate::concurrency::collect_files_parallel(&walker);
        let casefold = args.case_insensitive || args.smart_case;
        let mut matches: Vec<(String, String)> =
            if crate::concurrency::should_parallelize(files.len()) {
                use rayon::prelude::*;
                files
                    .par_iter()
                    .filter_map(|path| {
                        if shutdown.is_shutdown() {
                            return None;
                        }
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let hay = if casefold {
                            name.to_ascii_lowercase()
                        } else {
                            name.clone()
                        };
                        if hay.contains(&pat) {
                            Some((path.display().to_string(), name))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                files
                    .iter()
                    .filter_map(|path| {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let hay = if casefold {
                            name.to_ascii_lowercase()
                        } else {
                            name.clone()
                        };
                        if hay.contains(&pat) {
                            Some((path.display().to_string(), name))
                        } else {
                            None
                        }
                    })
                    .collect()
            };
        crate::concurrency::sort_parallel(&mut matches);

        let total_name_hits = matches.len() as u64;
        let start_idx = args.offset.min(total_name_hits) as usize;
        let end_idx = match args.limit {
            Some(lim) => start_idx.saturating_add(lim as usize).min(matches.len()),
            None => matches.len(),
        };
        for (path, name) in &matches[start_idx..end_idx] {
            writer.write_event(&crate::ndjson_types::FileMatchEvent {
                r#type: "file_match",
                path: path.clone(),
                name: name.clone(),
                target: "files",
            })?;
        }
        let matched_emitted = (end_idx - start_idx) as u64;

        if matches!(args.target, SearchTarget::Files) {
            writer.write_event(&Summary {
                r#type: "summary",
                // files_visited = all name hits (including those skipped by offset).
                files_visited: total_name_hits,
                files_matched: matched_emitted,
                files_modified: None,
                files_skipped: None,
                total_matches: Some(matched_emitted),
                total_replacements: None,
                elapsed_ms: start.elapsed().as_millis() as u64,
            })?;
            return Ok(());
        }
        let _ = matched_emitted;
    }

    let matcher = build_matcher(args)?;

    let walker = build_walker(args, &canonical_paths, global)?;

    let (tx, rx) =
        crossbeam_channel::bounded::<SearchEvent>(crate::concurrency::event_channel_cap());

    // Per-run counters (not process-wide statics). Ordering::Relaxed is enough:
    // each atomic is an independent tally; final loads happen after the
    // parallel walker thread joins, so no cross-field data publication is required.
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
    let no_begin_end = args.no_begin_end;
    let multiline = args.multiline;
    let search_binary = args.binary;

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
                .multi_line(multiline)
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
                // that have FIFOs in /tmp or /var (Docker, restricted, system dirs).
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

                // A-008: skip binary files unless --binary (align with replace/ripgrep).
                // Only probe the first BINARY_DETECT_SIZE bytes (do not load whole file).
                if !search_binary {
                    use std::io::Read;
                    if let Ok(mut f) = std::fs::File::open(entry.path()) {
                        let mut head =
                            vec![0u8; crate::constants::BINARY_DETECT_SIZE];
                        if let Ok(n) = f.read(&mut head) {
                            head.truncate(n);
                            if crate::binary_detect::is_binary(&head) {
                                tracing::debug!(
                                    path = %entry.path().display(),
                                    "skipping binary file (A-008); pass --binary to opt in"
                                );
                                return ignore::WalkState::Continue;
                            }
                        }
                    }
                }

                fv.fetch_add(1, Ordering::Relaxed);

                let path: Arc<std::path::Path> = Arc::from(entry.path());
                let _span = tracing::debug_span!("process_file", path = %path.display()).entered();
                let mut file_matches = 0u64;
                let mut file_lines = 0u64;

                // GAP-2026-010: --no-begin-end suppresses begin/end events for
                // files with zero matches. Default (off) preserves the
                // pre-v0.1.20 behaviour of emitting begin/end for every file.
                // CRITICAL: Begin must be sent BEFORE the sink so that
                // Match events emitted by SearchSink have a current open
                // path to attach to in the consumer's BTreeMap.
                if !no_begin_end {
                    let _ = tx.send(SearchEvent::Begin(Arc::clone(&path)));
                }

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

                if !(no_begin_end && file_matches == 0) {
                    let _ = tx.send(SearchEvent::End {
                        path,
                        matches: file_matches,
                        lines_searched: file_lines,
                    });
                }

                ignore::WalkState::Continue
            })
        });
    });

    let mut has_matches = false;
    let sort_active = args.sort.is_some();
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
                if !sort_active {
                    if let Some(events) = buffer.remove(&path_buf) {
                        open_paths.remove(&path_buf);
                        for ev in events {
                            emit_search_event(ev, writer, args, &mut has_matches)?;
                        }
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

    // Flush remaining buffered events. When --sort is active, ALL events are
    // here and BTreeMap emits them in sorted path order.
    for (_, events) in buffer {
        for ev in events {
            emit_search_event(ev, writer, args, &mut has_matches)?;
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

include!("search_helpers.inc.rs");
