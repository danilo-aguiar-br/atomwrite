// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parallel file content search powered by the ripgrep engine.
//!
//! Workload: I/O-bound (file reading + regex matching via ripgrep engine).
//! Parallelism: content search uses `ignore::WalkParallel`; each worker hands
//! the consumer one `FileFeed` per file over a bounded `crossbeam-channel`
//! (`search_stream.inc.rs`). The consumer drains one file at a time, so a file
//! always appears as one contiguous `begin` .. `end` block and memory stays
//! bounded regardless of the match count. `--sort` buffers instead (O(matches)).
//! `--target files` collects via `collect_files_parallel` (honors `--threads`),
//! sorts, then emits stable NDJSON. Bound via `--threads`/`--max-concurrency`.

use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{SearcherBuilder, Sink, SinkContext, SinkMatch};

use crate::cli::{GlobalArgs, SearchArgs};
use crate::cli_args::SortBy;
use crate::ndjson_types::{
    FailureTally, FileStats, SearchBegin, SearchContext, SearchCount, SearchEnd, SearchFile,
    SearchMatch, Submatch, Summary,
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
    // The positional is optional so `-e/--regex` can carry every pattern; a
    // command line with neither is refused here, before any walk starts.
    validate_patterns(args)?;

    let workspace = global.resolve_workspace()?;
    // The text format reproduces `rg -n`, so the combinations it cannot
    // express are refused up front instead of printed as something else.
    validate_text_format(args)?;
    let text_mode = args.format == Some(crate::cli::SearchFormat::Text);
    let canonical_paths = crate::commands::path_resolution::resolve_paths_against_workspace(
        &search_roots(args),
        &workspace,
    )?;
    // Every terminal envelope names the roots this run resolved, so a partial
    // answer is auditable without a second call (ETD).
    stamp_search_target(writer, args, &canonical_paths, &workspace);
    // A root named in the invocation that does not exist is a caller mistake,
    // not a scan failure, so it is reported before the walk and only the roots
    // that exist are walked (Bug 2).
    let (canonical_paths, missing_roots) =
        crate::commands::path_resolution::split_missing_roots(canonical_paths);
    let roots_missing = missing_roots.len() as u64;
    tracing::debug!(
        present = canonical_paths.len(),
        missing = roots_missing,
        "search roots partitioned"
    );

    if canonical_paths.is_empty() {
        // Every named root is missing: there is nothing to walk, and the
        // walker would index an empty root list.
        let base = Summary {
            r#type: "summary",
            files_visited: 0,
            files_matched: 0,
            files_modified: None,
            files_skipped: None,
            total_matches: Some(0),
            total_replacements: None,
            offset_skipped: Some(0),
            elapsed_ms: start.elapsed().as_millis() as u64,
            walker_stopped_early: None,
            ..Summary::default()
        };
        return finish_search_run(
            writer,
            base,
            root_failures(&missing_roots),
            roots_missing,
            false,
            text_mode,
        );
    }

    if search_file_names(
        args,
        global,
        writer,
        shutdown,
        &canonical_paths,
        &missing_roots,
        start,
    )? {
        return Ok(());
    }

    let walker = build_walker(args, &canonical_paths, global)?;
    // An ignore file the walk half-reads is a hole too, and `ignore` never
    // reports it by itself.
    let mut seed_failures = root_failures(&missing_roots);
    crate::concurrency::record_ignore_file_failures_into(&canonical_paths, &mut seed_failures);
    let run = Arc::new(SearchRun::new(
        args,
        build_matcher(args)?,
        shutdown.flag(),
        seed_failures,
    ));
    let (tx, rx) = crossbeam_channel::bounded::<FileFeed>(FILE_FEED_CAP);
    let walker_thread = spawn_search_walker(walker, Arc::clone(&run), tx);

    let mut tally = SearchTally::default();
    let mut text = TextLayout::new(args, &workspace);
    // Text mode always buffers and sorts: `rg` is compared line by line, and
    // the unsorted path orders files by whichever worker finished first.
    let consumed = if args.sort.is_some() || text_mode {
        consume_sorted(&rx, writer, args, shutdown, &mut tally, &mut text)
    } else {
        consume_streaming(&rx, writer, args, shutdown, &mut tally, &mut text)
    };
    // R4: the walker is stopped and joined on the error path too.
    crate::concurrency::join_walker(rx, &run.stop, walker_thread);
    let has_matches = consumed?;

    // A run killed by SIGINT/SIGTERM or the deadline is not an empty search, so
    // it must not become `NO_MATCHES` nor publish a partial summary. `run`
    // writes the single `cancelled` envelope for every scanning verb.
    if shutdown.is_shutdown() {
        return Ok(());
    }

    // Nothing searched is not the same as nothing found. When every file the
    // walk recognised as text was absent and only binary files were skipped,
    // `NO_MATCHES` would book a proof of absence the run never earned.
    let skipped = run.files_skipped.load(Ordering::Relaxed);
    if !has_matches && skipped > 0 && run.files_visited.load(Ordering::Relaxed) == 0 {
        return Err(crate::error::AtomwriteError::AllSkippedBinary { files: skipped }.into());
    }

    // `--limit` is a page, not the end of the answer. When the budget filled,
    // both formats say so with the same sentence: NDJSON carries it in `hint`,
    // and text prints it on stderr because it has no terminal record. Without
    // it, `--format text --limit 1` printed one line and zero bytes of stderr,
    // hiding that the tree held more.
    let limit_hint = crate::output::limit_reached().then(|| match args.limit {
        Some(cap) => format!(
            "the search stopped after {} of the --limit of {cap} matches; more matches were not shown",
            tally.total_matches
        ),
        None => format!(
            "the search stopped after {} matches; more matches were not shown",
            tally.total_matches
        ),
    });
    let files_skipped = run.files_skipped.load(Ordering::Relaxed);
    // The matched counts come from the CONSUMER, not the walker. The producer
    // tallies every file it scanned, including files whose blocks the consumer
    // discarded once `--limit` filled, so the summary used to promise answers
    // that never reached stdout. `files_visited` stays the walker's own count:
    // it measures work done, and `walker_stopped_early` says the walk was cut.
    let base = Summary {
        r#type: "summary",
        files_visited: run.files_visited.load(Ordering::Relaxed),
        files_matched: tally.files_matched,
        files_modified: None,
        // R29: oversize files are skipped; say how many instead of staying silent.
        files_skipped: (files_skipped > 0).then_some(files_skipped),
        total_matches: Some(tally.total_matches),
        total_replacements: None,
        offset_skipped: Some(tally.offset_skipped),
        matches_suppressed: Some(run.matches_suppressed.load(Ordering::Relaxed)).filter(|n| *n > 0),
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        elapsed_ms: start.elapsed().as_millis() as u64,
        walker_stopped_early: run.walk_stopped_early().then_some(true),
        hint: limit_hint,
        ..Summary::default()
    };
    // Files that could not be read are reported, never folded into "no match".
    let failures = run.take_failures();
    finish_search_run(
        writer,
        base,
        failures,
        roots_missing,
        has_matches,
        text_mode,
    )
}

/// Refuse the `--format text` combinations that cannot be printed as a line.
///
/// A multi-line match spans lines, and a file-name match is a name and not a
/// numbered line, so neither has a `LINE:TEXT` form. Answering with something
/// else would be the flag silently lying about what it did.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidValue` when the combination is unsupported.
fn validate_text_format(args: &SearchArgs) -> Result<()> {
    use crate::cli_args::SearchTarget;
    if args.format != Some(crate::cli::SearchFormat::Text) {
        return Ok(());
    }
    let refusal = if args.multiline {
        Some((
            "--format text",
            "--multiline",
            "text mode prints one line per match",
        ))
    } else if matches!(args.target, SearchTarget::Files | SearchTarget::Both) {
        Some((
            "--format text",
            "--target files|both",
            "text mode prints numbered lines, not file names",
        ))
    } else {
        None
    };
    match refusal {
        Some((flag, other, reason)) => Err(crate::error::AtomwriteError::UnsupportedCombination {
            flag: flag.to_string(),
            other: other.to_string(),
            reason: reason.to_string(),
        }
        .into()),
        None => Ok(()),
    }
}

/// Inputs and tallies shared by every walker worker of one search run.
///
/// `Ordering::Relaxed` on the tallies: independent counters, read once after
/// the walker thread joins.
struct SearchRun {
    matcher: grep_regex::RegexMatcher,
    context_lines: usize,
    invert: bool,
    max_count: Option<u64>,
    include_fifo: bool,
    max_filesize: u64,
    max_columns: usize,
    no_begin_end: bool,
    multiline: bool,
    search_binary: bool,
    /// `--count` / `--files`: matches are tallied, never sent as events.
    tally_only: bool,
    /// Process-wide cancel flag (signal or `--timeout-secs`).
    shutdown_flag: Arc<AtomicBool>,
    /// Set by the consumer when it stops reading; workers quit at the next file.
    stop: AtomicBool,
    /// Set at the moment a walker thread returns [`ignore::WalkState::Quit`].
    ///
    /// `walker_stopped_early` used to be stamped from `output::limit_reached()`,
    /// which records that the writer dropped a record. Those are different
    /// facts: measured on 4000 files with `--limit 1`, the walk quit after 9
    /// visited files while the writer's budget had been spent on the first one.
    /// Only the walker knows whether the walker quit, so every `Quit` in this
    /// file records it here.
    walk_quit: AtomicBool,
    files_visited: AtomicU64,
    files_skipped: AtomicU64,
    /// Files where `--max-count` cut at least one match.
    ///
    /// Each file contributes one, and each contribution hides at least one more
    /// match, so the count is a floor on what the page left out. The ceiling is
    /// on records, not on reading: `--max-count` asks the searcher for one match
    /// past the ceiling, so a cut file is read up to one match further than the
    /// ceiling allows, and never more than that.
    matches_suppressed: AtomicU64,
    /// Files that could not be read or walked, with the first error kept whole.
    failures: Mutex<FailureTally>,
}

impl SearchRun {
    fn new(
        args: &SearchArgs,
        matcher: grep_regex::RegexMatcher,
        shutdown_flag: Arc<AtomicBool>,
        failures: FailureTally,
    ) -> Self {
        Self {
            matcher,
            context_lines: args.context,
            invert: args.invert,
            max_count: args.max_count,
            include_fifo: args.include_fifo,
            max_filesize: args.max_filesize,
            max_columns: args.max_columns,
            no_begin_end: args.no_begin_end,
            multiline: args.multiline,
            search_binary: args.binary,
            tally_only: args.count || args.files,
            shutdown_flag,
            stop: AtomicBool::new(false),
            walk_quit: AtomicBool::new(false),
            files_visited: AtomicU64::new(0),
            files_skipped: AtomicU64::new(0),
            matches_suppressed: AtomicU64::new(0),
            // Seed with the roots that were never reachable, so the summary
            // and the first error already carry them before the walk starts.
            failures: Mutex::new(failures),
        }
    }

    /// Record that `path` could not be processed because of `err`.
    fn record_failure(&self, path: &std::path::Path, err: anyhow::Error) {
        tracing::debug!(path = %path.display(), error = %err, "search failure recorded");
        self.failures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .record(path, err);
    }

    /// Hand over the failures collected so far, leaving the tally empty.
    fn take_failures(&self) -> FailureTally {
        std::mem::take(
            &mut *self
                .failures
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    /// True once the run was cancelled or the consumer stopped reading.
    fn should_quit(&self) -> bool {
        self.shutdown_flag.load(Ordering::Acquire)
            || self.stop.load(Ordering::Acquire)
            || crate::output::limit_reached()
    }

    /// Record that a walker thread asked the walk to stop.
    fn note_walk_quit(&self) {
        self.walk_quit.store(true, Ordering::Release);
    }

    /// Whether the walk was actually cut before the tree ended.
    fn walk_stopped_early(&self) -> bool {
        self.walk_quit.load(Ordering::Acquire)
    }
}

/// Run the parallel walk on a dedicated thread; dropping `tx` ends the stream.
fn spawn_search_walker(
    walker: ignore::WalkBuilder,
    run: Arc<SearchRun>,
    tx: crossbeam_channel::Sender<FileFeed>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let run = Arc::clone(&run);
            let tx = tx.clone();
            // Context lines are only useful when events are emitted.
            let context = if run.tally_only { 0 } else { run.context_lines };
            // The heap limit follows the largest file the gates admit, because
            // the line buffer starts small and grows by doubling until it holds
            // the longest line, and `--multiline` buffers the whole file when
            // there is no mmap. A fixed cap refused a line that sat inside an
            // allowed file, so the limit is derived from `search.max_filesize`.
            let heap_limit = run
                .max_filesize
                .saturating_mul(crate::constants::SEARCH_HEAP_LIMIT_FACTOR)
                .min(usize::MAX as u64) as usize;
            tracing::debug!(
                heap_limit_bytes = heap_limit,
                max_filesize = run.max_filesize,
                "search searcher heap limit"
            );
            let mut searcher = SearcherBuilder::new()
                .line_number(true)
                .multi_line(run.multiline)
                .invert_match(run.invert)
                .before_context(context)
                .after_context(context)
                // One match past the ceiling proves the file had more, and the
                // sink throws that match away. Asking for exactly the ceiling
                // left the sink unable to tell a complete file from a cut one.
                .max_matches(run.max_count.map(|max| max.saturating_add(1)))
                // A line longer than the heap limit is refused per file, which
                // keeps both the allocation and the distance between two cancel
                // checkpoints bounded (A7).
                .heap_limit(Some(heap_limit))
                .build();
            Box::new(move |entry| visit_search_entry(&run, &tx, &mut searcher, entry))
        });
        tracing::debug!(
            raised_ago_ms =
                crate::signal::shutdown_raised_ago().map_or(0, |d| d.as_millis() as u64),
            "search walker finished"
        );
    })
}

/// Process one walk entry: gates, then the search of one file.
fn visit_search_entry(
    run: &SearchRun,
    feeds: &crossbeam_channel::Sender<FileFeed>,
    searcher: &mut grep_searcher::Searcher,
    entry: std::result::Result<ignore::DirEntry, ignore::Error>,
) -> ignore::WalkState {
    // The quit is recorded here and only here: this is the point where an entry
    // is dropped unvisited. A quit after a file was processed ends the walk
    // naturally, and stamping it would say the walk was cut when it was not.
    if run.should_quit() {
        run.note_walk_quit();
        return ignore::WalkState::Quit;
    }
    let entry = match entry {
        Ok(entry) => entry,
        Err(err) => {
            record_walk_error(run, &err);
            return ignore::WalkState::Continue;
        }
    };
    if !entry.file_type().is_some_and(|ft| ft.is_file()) || !passes_file_gates(run, &entry) {
        return ignore::WalkState::Continue;
    }

    run.files_visited.fetch_add(1, Ordering::Relaxed);
    let path: Arc<std::path::Path> = Arc::from(entry.path());
    let _span = tracing::debug_span!("process_file", path = %path.display()).entered();
    search_one_file(run, feeds, searcher, path)
}

/// Count a walk error that names a path (an unreadable directory, a link loop).
///
/// Errors that carry no I/O cause (a malformed ignore file) are not a failure
/// to read the tree and stay in the debug log.
fn record_walk_error(run: &SearchRun, err: &ignore::Error) {
    match crate::error::AtomwriteError::from_walk_error(err) {
        Some((path, failure)) => run.record_failure(&path, failure.into()),
        None => tracing::debug!(error = %err, "walk error without an I/O cause"),
    }
}

/// Whether the entry is a named pipe. Always `false` off Unix, which has none.
#[cfg(unix)]
fn entry_is_fifo(entry: &ignore::DirEntry) -> bool {
    use std::os::unix::fs::FileTypeExt;
    entry.file_type().is_some_and(|ft| ft.is_fifo())
}

/// Whether the entry is a named pipe. Always `false` off Unix, which has none.
#[cfg(not(unix))]
fn entry_is_fifo(_entry: &ignore::DirEntry) -> bool {
    false
}

/// FIFO, size and binary gates. Returns `false` when the file is skipped.
fn passes_file_gates(run: &SearchRun, entry: &ignore::DirEntry) -> bool {
    // G56: skip FIFO/named pipe files unless --include-fifo is set.
    // `open()` on a FIFO blocks indefinitely until the other end
    // connects, which can cause atomwrite to hang in environments
    // that have FIFOs in /tmp or /var (Docker, restricted, system dirs).
    if !run.include_fifo && entry_is_fifo(entry) {
        tracing::debug!(
            path = %entry.path().display(),
            "skipping FIFO (G56); pass --include-fifo to opt in"
        );
        return false;
    }

    // G68: skip files larger than --max-filesize, and count them (R29).
    if let Ok(meta) = entry.metadata() {
        if meta.len() > run.max_filesize {
            tracing::debug!(
                path = %entry.path().display(),
                size = meta.len(),
                max = run.max_filesize,
                "skipping oversized file (G68)"
            );
            run.files_skipped.fetch_add(1, Ordering::Relaxed);
            // Same rule as the binary skip: a caller reading only stdout must
            // not conclude the text is absent from a file that was never read.
            crate::best_effort::best_effort(
                writeln!(
                    std::io::stderr().lock(),
                    "atomwrite: skipped oversized {} ({} bytes > --max-filesize {}; raise it to search)",
                    entry.path().display(),
                    meta.len(),
                    run.max_filesize
                ),
                "report oversized skip on stderr",
            );
            return false;
        }
    }

    // A-008: skip binary files unless --binary (align with replace/ripgrep).
    // Only probe the first BINARY_DETECT_SIZE bytes (do not load whole file).
    if !run.search_binary {
        match is_binary_file(entry.path()) {
            Ok(false) => {}
            Ok(true) => {
                tracing::debug!(
                    path = %entry.path().display(),
                    "skipping binary file (A-008); pass --binary to opt in"
                );
                // `files_skipped` is the count of files the run decided not to
                // read, and an agent that asks "why wasn't my file searched"
                // reads exactly this field. A silent skip left it saying zero.
                run.files_skipped.fetch_add(1, Ordering::Relaxed);
                // The skip also goes to stderr, because a caller reading only
                // stdout must not conclude the text is absent from a file that
                // was never opened.
                crate::best_effort::best_effort(
                    writeln!(
                        std::io::stderr().lock(),
                        "atomwrite: skipped binary {} (--binary to search it)",
                        entry.path().display()
                    ),
                    "report binary skip on stderr",
                );
                return false;
            }
            // The probe could not read the file: report it once here, because
            // the search would hit the same error and the file is skipped.
            Err(source) => {
                run.files_visited.fetch_add(1, Ordering::Relaxed);
                let path = entry.path();
                run.record_failure(
                    path,
                    crate::error::AtomwriteError::from_io_at(path, source).into(),
                );
                return false;
            }
        }
    }
    true
}

/// Translate a searcher failure into the crate's own vocabulary.
///
/// `grep-searcher` reports a line bigger than the heap limit as a raw
/// allocation-limit I/O error, which names neither the file nor the reason. The
/// file is searchable only after the line is cut or the tree is split, so the
/// report says exactly that instead of "inspecione o erro de E/S".
fn searcher_failure(
    path: &std::path::Path,
    source: &std::io::Error,
) -> crate::error::AtomwriteError {
    if source
        .to_string()
        .contains(crate::constants::SEARCH_HEAP_LIMIT_MARKER)
    {
        return crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "{} has a single line longer than the searcher's heap limit, which follows `search.max_filesize`; raise that key or split the line and search the file on its own",
                path.display()
            ),
        };
    }
    crate::error::AtomwriteError::from_io_at(
        path,
        std::io::Error::new(source.kind(), source.to_string()),
    )
}

fn is_binary_file(path: &std::path::Path) -> std::io::Result<bool> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut head = vec![0u8; crate::constants::BINARY_DETECT_SIZE];
    let n = file.read(&mut head)?;
    head.truncate(n);
    Ok(crate::binary_detect::is_binary(&head))
}

/// Search one file and stream its `begin` .. `end` block to the consumer.
///
/// A cancelled or abandoned file sends no `end`: its partial block is dropped
/// and the walk quits.
fn search_one_file(
    run: &SearchRun,
    feeds: &crossbeam_channel::Sender<FileFeed>,
    searcher: &mut grep_searcher::Searcher,
    path: Arc<std::path::Path>,
) -> ignore::WalkState {
    let mut out = FileSender::new(feeds);
    // GAP-2026-010: --no-begin-end suppresses begin/end events for files with
    // zero matches. Default (off) emits begin/end for every file.
    if !run.no_begin_end && !out.send(SearchEvent::Begin(Arc::clone(&path))) {
        return ignore::WalkState::Quit;
    }

    let mut sink = SearchSink {
        matcher: &run.matcher,
        path: Arc::clone(&path),
        out: &mut out,
        cancel: crate::signal::CancelCheck::for_matches(Arc::clone(&run.shutdown_flag)),
        max_columns: run.max_columns,
        tally_only: run.tally_only,
        max_count: run.max_count,
        suppressed: false,
        matches: 0,
        lines: 0,
        stopped: false,
    };
    let result =
        search_file_cancellable(searcher, &run.matcher, &path, &run.shutdown_flag, &mut sink);
    let (file_matches, file_lines, stopped) = (sink.matches, sink.lines, sink.stopped);
    if sink.suppressed {
        run.matches_suppressed.fetch_add(1, Ordering::Relaxed);
    }

    if let Err(source) = result {
        let failure = searcher_failure(&path, &source);
        run.record_failure(&path, failure.into());
    }
    if stopped || run.should_quit() {
        // Every `Quit` is the walk being asked to stop, and this one is the
        // common case: the budget filled while this file was being read.
        run.note_walk_quit();
        return ignore::WalkState::Quit;
    }

    let wants_end = !(run.no_begin_end && file_matches == 0);
    let sent = !wants_end
        || out.send(SearchEvent::End {
            path,
            matches: file_matches,
            lines_searched: file_lines,
        });
    if sent && out.finish() {
        ignore::WalkState::Continue
    } else {
        // The consumer closed the channel, which is the other way a walk is
        // cut. Missing it left the flag saying `false` while the walk had
        // stopped after 9 of 4000 files.
        run.note_walk_quit();
        ignore::WalkState::Quit
    }
}

enum SearchEvent {
    Begin(Arc<std::path::Path>),
    Match {
        path: Arc<std::path::Path>,
        line_number: u64,
        lines: String,
        byte_offset: u64,
        submatches: Vec<Submatch>,
        /// Line bytes as they are in the file, terminator excluded.
        ///
        /// `--format text` must reproduce `rg`, which keeps `\r` from CRLF
        /// files and passes invalid UTF-8 through. `lines` cannot: it is the
        /// JSON field, already decoded lossily and cut at `--max-columns`.
        raw: Vec<u8>,
    },
    Context {
        path: Arc<std::path::Path>,
        line_number: u64,
        lines: String,
        /// Context lines of one block, raw, numbered from `line_number`.
        raw_block: Vec<u8>,
    },
    End {
        path: Arc<std::path::Path>,
        matches: u64,
        lines_searched: u64,
    },
}

include!("search_helpers.inc.rs");
include!("search_stream.inc.rs");
include!("search_files.inc.rs");
