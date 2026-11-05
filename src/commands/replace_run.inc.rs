// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by replace.rs (A-MONO-001).
//
// Running the replace: setup, the parallel per-file worker and the join.
// The event consumer and the terminal record live in replace_finish.inc.rs, and
// the per-file read and replace steps in replace_file.inc.rs.

include!("replace_file.inc.rs");

/// Read-only inputs every walker worker needs, resolved once before the walk.
struct ReplaceJob {
    pattern: Regex,
    pattern_text: Arc<str>,
    replacement: Arc<str>,
    max_replacements: Option<usize>,
    preserve_case: bool,
    use_regex: bool,
    word: bool,
    fuzzy_opts: fuzzy::MatchOpts,
    dry_run: bool,
    preview: bool,
    backup: bool,
    keep_backup: bool,
    retention: u8,
    preserve_timestamps: bool,
    strict_atomic: bool,
    expect_checksum: Option<Arc<str>>,
    max_size: u64,
    workspace: Arc<std::path::Path>,
    progress_every: u64,
    quiet: u8,
    no_progress: bool,
    /// The `[write]` policy, shared by every worker for risk assessment.
    write_cfg: Arc<crate::config::WriteSection>,
    /// R4: set by the consumer on exit; workers stop and never write after it.
    stop: std::sync::atomic::AtomicBool,
}

/// Outcome of applying the replacement to the content of one file.
struct FileReplacement<'a> {
    replaced: Cow<'a, str>,
    count: u64,
    /// Matches the file had beyond `--max-replacements`, so the summary can
    /// say how much of the file the limit left untouched.
    suppressed: u64,
    /// `(fuzzy, strategy, similarity, strategies_tried)` when the fuzzy cascade made the edit.
    fuzzy_meta: Option<(bool, String, Option<f64>, u64)>,
    word_ignored: bool,
    /// The deadline or a signal fired inside the file.
    cancelled: bool,
    /// The cascade refused the file (for example above the fuzzy content cap);
    /// reported as a failed file, never as "no match".
    failure: Option<anyhow::Error>,
}

fn run_replace(
    args: &ReplaceArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    // G121 (CWD-relative path resolution for replace): centralize via
    // path_resolution::resolve_paths_against_workspace. This runs
    // validate_path on every caller-supplied root, COLLECTS the canonical
    // absolute PathBuf, and returns them so the WalkBuilder receives a
    // workspace-anchored root instead of the original (CWD-relative) path.
    let canonical_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;

    // A root the caller named and the run never reached is a caller mistake,
    // not a scan failure: it is counted before the walk and only the roots
    // that exist are walked.
    let (canonical_paths, missing_roots) =
        crate::commands::path_resolution::split_missing_roots(canonical_paths);
    tracing::debug!(
        present = canonical_paths.len(),
        missing = missing_roots.len(),
        "replace roots partitioned"
    );

    let pattern = compile_pattern(args)?;

    if canonical_paths.is_empty() {
        let mut failures = crate::ndjson_types::FailureTally::default();
        failures.record_missing_roots(&missing_roots);
        let first_binary_path = std::sync::Mutex::new(None);
        return finish_replace(
            ReplaceTotals {
                total_replacements: 0,
                replacements_suppressed: 0,
                files_matched: 0,
                files_visited: 0,
                files_skipped: 0,
                binary_rejects: 0,
                files_total: 0,
                failures,
                missing_roots,
                walk_quit: false,
            },
            &first_binary_path,
            args.dry_run,
            args.preview,
            shutdown,
            writer,
            start,
        );
    }

    let walker = build_walker(args, &canonical_paths, global)?;

    // v0.1.30: pre-count eligible files so progress has total/rate/eta.
    // Skip entirely when progress is off (avoids a full double-walk).
    // When needed, use WalkParallel so --threads applies (build() ignores threads).
    let progress_wanted = args.progress_every > 0 && !global.no_progress && global.quiet == 0;
    let precount = if progress_wanted {
        let paths = crate::concurrency::collect_files_parallel(&walker);
        paths.len() as u64
    } else {
        0
    };
    let walker = build_walker(args, &canonical_paths, global)?;

    let (tx, rx) =
        crossbeam_channel::bounded::<ReplaceEvent>(crate::concurrency::event_channel_cap());

    let counters = Arc::new(ReplaceCounters::new(precount));
    let job = Arc::new(build_replace_job(
        args, global, &workspace, defaults, fuzzy_cfg, write_cfg, pattern,
    )?);
    let walker_thread = spawn_replace_walker(
        walker,
        Arc::clone(&job),
        Arc::clone(&counters),
        tx,
        shutdown.flag(),
    );

    let drained = drain_replace_events(&rx, writer, shutdown, start);
    crate::concurrency::join_walker(rx, &job.stop, walker_thread);
    let mut failures = drained?;
    // An ignore file the walk half-reads is a hole too, and `ignore` never
    // reports it by itself.
    crate::concurrency::record_ignore_file_failures_into(&canonical_paths, &mut failures);
    failures.record_missing_roots(&missing_roots);

    finish_replace(
        ReplaceTotals {
            total_replacements: counters.total_replacements.load(Ordering::Relaxed),
            replacements_suppressed: counters.replacements_suppressed.load(Ordering::Relaxed),
            files_matched: counters.files_modified.load(Ordering::Relaxed),
            files_visited: counters.files_visited.load(Ordering::Relaxed),
            files_skipped: counters.files_skipped.load(Ordering::Relaxed),
            binary_rejects: counters.binary_rejects.load(Ordering::Relaxed),
            files_total: counters.files_total.load(Ordering::Relaxed),
            failures,
            missing_roots,
            walk_quit: counters
                .walk_quit
                .load(std::sync::atomic::Ordering::Acquire),
        },
        &counters.first_binary_path,
        args.dry_run,
        args.preview,
        shutdown,
        writer,
        start,
    )
}

/// Resolve every per-worker input once, before the walk starts.
fn build_replace_job(
    args: &ReplaceArgs,
    global: &GlobalArgs,
    workspace: &std::path::Path,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    write_cfg: &crate::config::WriteSection,
    pattern: Regex,
) -> Result<ReplaceJob> {
    let resolved = resolve_backup(&args.backup_opts, defaults);
    let (fuzzy_mode, fuzzy_threshold) =
        crate::config::resolve_fuzzy(args.fuzzy, args.fuzzy_threshold, fuzzy_cfg)?;
    Ok(ReplaceJob {
        pattern,
        pattern_text: args.pattern.clone().into(),
        replacement: args.replacement.clone().into(),
        max_replacements: args.max_replacements,
        preserve_case: args.preserve_case,
        use_regex: args.regex,
        word: args.word,
        fuzzy_opts: fuzzy::match_opts_from_section(fuzzy_mode, fuzzy_threshold, fuzzy_cfg, false),
        dry_run: args.dry_run,
        preview: args.preview,
        backup: resolved.backup,
        keep_backup: resolved.keep,
        retention: resolved.retention,
        preserve_timestamps: args.preserve_timestamps,
        strict_atomic: args.strict.strict_atomic,
        expect_checksum: args.expect_checksum.clone().map(Into::into),
        max_size: global.effective_max_filesize(),
        workspace: Arc::from(workspace),
        progress_every: args.progress_every,
        write_cfg: Arc::new(write_cfg.clone()),
        quiet: global.quiet,
        no_progress: global.no_progress,
        stop: std::sync::atomic::AtomicBool::new(false),
    })
}

/// Run the parallel walk on a dedicated thread; dropping `tx` ends the event stream.
fn spawn_replace_walker(
    walker: ignore::WalkBuilder,
    job: Arc<ReplaceJob>,
    counters: Arc<ReplaceCounters>,
    tx: crossbeam_channel::Sender<ReplaceEvent>,
    shutdown_flag: Arc<std::sync::atomic::AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let job = Arc::clone(&job);
            let counters = Arc::clone(&counters);
            let tx = tx.clone();
            let shutdown_flag = Arc::clone(&shutdown_flag);
            Box::new(move |entry| visit_entry(&job, &counters, &tx, &shutdown_flag, entry))
        });
    })
}

/// Process one walk entry: gates, replacement, then delivery of the result.
fn visit_entry(
    job: &ReplaceJob,
    counters: &ReplaceCounters,
    tx: &crossbeam_channel::Sender<ReplaceEvent>,
    shutdown_flag: &Arc<std::sync::atomic::AtomicBool>,
    entry: std::result::Result<ignore::DirEntry, ignore::Error>,
) -> ignore::WalkState {
    if shutdown_flag.load(Ordering::Acquire)
        || job.stop.load(Ordering::Acquire)
        || crate::output::limit_reached()
    {
        counters
            .walk_quit
            .store(true, std::sync::atomic::Ordering::Release);
        return ignore::WalkState::Quit;
    }

    let entry = match entry {
        Ok(e) => e,
        Err(walk_error) => {
            if let Some((path, error)) = crate::error::AtomwriteError::from_walk_error(&walk_error)
            {
                crate::best_effort::send_to_consumer(
                    tx,
                    ReplaceEvent::Error {
                        path,
                        kind: ReplaceErrorKind::Failed(error.into()),
                    },
                );
            }
            return ignore::WalkState::Continue;
        }
    };

    if !entry.file_type().is_some_and(|ft| ft.is_file()) {
        return ignore::WalkState::Continue;
    }

    let visited = counters.files_visited.fetch_add(1, Ordering::Relaxed) + 1;
    if job.progress_every > 0
        && visited.is_multiple_of(job.progress_every)
        && job.quiet == 0
        && !job.no_progress
    {
        crate::best_effort::send_to_consumer(
            tx,
            ReplaceEvent::Progress {
                done: visited,
                total: counters.files_total.load(Ordering::Relaxed),
            },
        );
    }

    let path = entry.path().to_path_buf();
    let _span = tracing::debug_span!("process_file", path = %path.display()).entered();

    let Some(mut content) = read_candidate(job, counters, tx, &path) else {
        return ignore::WalkState::Continue;
    };

    // A1: the first read is lock free, because most files of a tree do not
    // match. A file that does match and is about to be written is locked,
    // checked against what was read, and redone on fresh content when another
    // writer got there first. `update_lock` keeps that lock across the redo.
    let mut update_lock = None;
    loop {
        // v0.1.36: the deadline must fire INSIDE the file, not only at
        // the next file boundary. One check point per file polls the
        // same flag the outer loop already watches, but per work block
        // of matches, so an 800 MB file no longer overruns the budget.
        let mut cancel = crate::signal::CancelCheck::for_matches(Arc::clone(shutdown_flag));
        let mut outcome = replace_in_content(job, &content, &mut cancel);

        if let Some(error) = outcome.failure.take() {
            counters.files_skipped.fetch_add(1, Ordering::Relaxed);
            crate::best_effort::send_to_consumer(
                tx,
                ReplaceEvent::Error {
                    path,
                    kind: ReplaceErrorKind::Failed(error),
                },
            );
            return ignore::WalkState::Continue;
        }

        if outcome.cancelled {
            // Publish the cooperative stop exactly once and re-raise the
            // outer flag so `Quit` propagates to the other walker workers.
            shutdown_flag.store(true, Ordering::Release);
            counters
                .walk_quit
                .store(true, std::sync::atomic::Ordering::Release);
            return ignore::WalkState::Quit;
        }

        if outcome.count == 0 {
            counters.files_skipped.fetch_add(1, Ordering::Relaxed);
            return ignore::WalkState::Continue;
        }

        // `--limit` is a work budget: the claim happens BEFORE the counters move
        // and before any write, so a run with many walk workers rewrites exactly
        // the number it may report instead of counting work it then discards.
        if !crate::output::claim_work() {
            // The budget is spent: this entry is dropped without being
            // rewritten, which is the walk being cut.
            counters
                .walk_quit
                .store(true, std::sync::atomic::Ordering::Release);
            return ignore::WalkState::Quit;
        }
        let count = outcome.count;
        let suppressed = outcome.suppressed;
        counters
            .total_replacements
            .fetch_add(count, Ordering::Relaxed);
        counters
            .replacements_suppressed
            .fetch_add(suppressed, Ordering::Relaxed);

        let redone = update_lock.is_some();
        match deliver_replacement(job, counters, tx, path.clone(), &content, outcome, redone) {
            Delivery::Done => return ignore::WalkState::Continue,
            Delivery::Stale => {
                counters
                    .total_replacements
                    .fetch_sub(count, Ordering::Relaxed);
                counters
                    .replacements_suppressed
                    .fetch_sub(suppressed, Ordering::Relaxed);
                match crate::lock::acquire_for_update(&path) {
                    Ok(guard) => update_lock = Some(guard),
                    Err(error) => {
                        crate::best_effort::send_to_consumer(
                            tx,
                            ReplaceEvent::Error {
                                path,
                                kind: ReplaceErrorKind::Failed(error),
                            },
                        );
                        return ignore::WalkState::Continue;
                    }
                }
                let Some(fresh) = read_candidate(job, counters, tx, &path) else {
                    return ignore::WalkState::Continue;
                };
                content = fresh;
            }
        }
    }
}

/// What became of a replacement handed to [`deliver_replacement`].
enum Delivery {
    /// Reported or written; nothing left to do for this file.
    Done,
    /// The file changed after it was read; the caller redoes it under the lock.
    Stale,
}

/// Report a mismatch between `--expect-checksum` and `actual` as a drift error.
///
/// Returns `true` when the event was sent and the caller must stop this file.
fn report_expect_drift(
    job: &ReplaceJob,
    tx: &crossbeam_channel::Sender<ReplaceEvent>,
    path: &std::path::Path,
    actual: &str,
) -> bool {
    let Some(ref expected) = job.expect_checksum else {
        return false;
    };
    if actual == &**expected {
        return false;
    }
    crate::best_effort::send_to_consumer(
        tx,
        ReplaceEvent::Error {
            path: path.to_path_buf(),
            kind: ReplaceErrorKind::StateDrift {
                expected: expected.to_string(),
                actual: actual.to_owned(),
            },
        },
    );
    true
}

/// Hand a successful replacement to the consumer: drift check, plan, preview or write.
///
/// A write takes the file lock first and checks that the file still is what
/// `content` was read from; if not it returns [`Delivery::Stale`] so the caller
/// redoes the replacement. `redone` means the caller already holds the lock on
/// fresh content, so a second mismatch is reported by the writer as drift.
fn deliver_replacement(
    job: &ReplaceJob,
    counters: &ReplaceCounters,
    tx: &crossbeam_channel::Sender<ReplaceEvent>,
    path: PathBuf,
    content: &str,
    outcome: FileReplacement<'_>,
    redone: bool,
) -> Delivery {
    let FileReplacement {
        replaced,
        count,
        fuzzy_meta,
        word_ignored,
        ..
    } = outcome;
    let checksum_before = checksum::hash_bytes(content.as_bytes());

    // Dry-run and preview never take the lock, so they check the content they
    // read. The write path checks again under the lock, below.
    if (job.dry_run || job.preview) && report_expect_drift(job, tx, &path, &checksum_before) {
        return Delivery::Done;
    }

    // Risk only: `replace` rewrites matches inside the file, so a shorter
    // result is the substitution working and not a whole-content overwrite.
    // Assessed before the dry-run and preview branches, because those are the
    // runs an agent reads to decide whether to write at all.
    let risk_assessment = crate::commands::write_guards::assess_content_risk(
        replaced.as_bytes(),
        content.len() as u64,
        replaced.len() as u64,
        &job.write_cfg,
    );

    if job.dry_run {
        crate::best_effort::send_to_consumer(
            tx,
            ReplaceEvent::DryRun {
                path,
                replacements: count,
                risk_assessment,
            },
        );
        counters.files_modified.fetch_add(1, Ordering::Relaxed);
        return Delivery::Done;
    }

    if job.preview {
        let diff = similar::TextDiff::from_lines(content, &replaced);
        let unified = diff.unified_diff().to_string();
        crate::best_effort::send_to_consumer(
            tx,
            ReplaceEvent::Preview {
                path,
                replacements: count,
                diff: unified,
                risk_assessment,
            },
        );
        counters.files_modified.fetch_add(1, Ordering::Relaxed);
        return Delivery::Done;
    }

    // The consumer may have given up while this file was being rewritten.
    if job.stop.load(Ordering::Acquire) {
        return Delivery::Done;
    }

    // A1: lock, then make sure the file still is what was read.
    let _update_lock = match crate::lock::acquire_for_update(&path) {
        Ok(lock) => lock,
        Err(error) => {
            crate::best_effort::send_to_consumer(
                tx,
                ReplaceEvent::Error {
                    path,
                    kind: ReplaceErrorKind::Failed(error),
                },
            );
            return Delivery::Done;
        }
    };
    if !redone && checksum::hash_file(&path, u64::MAX).ok().as_deref() != Some(&checksum_before) {
        return Delivery::Stale;
    }
    // Compare the expected checksum here, under the lock, against the content
    // the file holds now: checking before the lock left a TOCTOU window.
    if report_expect_drift(job, tx, &path, &checksum_before) {
        return Delivery::Done;
    }

    let opts = AtomicWriteOptions {
        backup: job.backup,
        retention: job.retention,
        preserve_timestamps: job.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: job.strict_atomic,
        syntax_check: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        // ADR-0038: only --keep-backup retains the file. Deriving retention
        // from `backup` kept a `.bak` on every run, because `[defaults].backup`
        // resolves to true and `[defaults]` is not an operator opt-in.
        keep_backup: job.keep_backup,
        durability: crate::platform::Durability::Auto,
    };

    match crate::atomic::atomic_write_based(
        &path,
        replaced.as_bytes(),
        &opts,
        &job.workspace,
        Some(&checksum_before),
    ) {
        Ok(result) => {
            counters.files_modified.fetch_add(1, Ordering::Relaxed);
            let (fuzzy, strategy, similarity, strategies_tried) = match fuzzy_meta {
                Some((f, s, sim, tried)) => (Some(f), Some(s), sim, Some(tried)),
                None => (Some(false), Some("exact".into()), None, Some(1)),
            };
            crate::best_effort::send_to_consumer(
                tx,
                ReplaceEvent::Replaced {
                    path,
                    replacements: count,
                    bytes_before: content.len() as u64,
                    bytes_after: replaced.len() as u64,
                    checksum_before,
                    checksum_after: result.checksum,
                    elapsed_ms: result.elapsed_ms,
                    mtime_preserved: job.preserve_timestamps,
                    fuzzy,
                    strategy,
                    similarity,
                    strategies_tried,
                    word_ignored: if word_ignored { Some(true) } else { None },
                    risk_assessment,
                    backup_path: result.backup_path,
                    write_strategy: result.write_strategy,
                    backup_skipped: result.backup_skipped,
                },
            );
        }
        Err(e) => {
            crate::best_effort::send_to_consumer(
                tx,
                ReplaceEvent::Error {
                    path,
                    kind: ReplaceErrorKind::Failed(e),
                },
            );
        }
    }
    Delivery::Done
}
