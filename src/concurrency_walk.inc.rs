// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by concurrency.rs (A-MONO-001).

/// Collect mapped walk entries via `WalkParallel` (visitor-local shards).
///
/// Call [`apply_walk_threads`] on the builder first so `--threads` /
/// `--max-concurrency` bound discovery. Stops early on global shutdown.
/// Per-entry walk errors are skipped (same posture as sequential call sites).
///
/// # No Mutex on the hot path
///
/// Each walk worker keeps a **thread-local** `Vec`. Workers push into a shared
/// `Mutex<Vec<Vec<_>>>` only on Drop (once per thread), never per entry.
///
/// # Why not sequential `build()`
///
/// `threads(N)` is a no-op on `build()` — only `build_parallel` fans out
/// directory traversal (docs.rs ignore / ddgs).
pub fn collect_mapped_parallel<T, F>(builder: &WalkBuilder, map: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(&ignore::DirEntry) -> Option<T> + Sync + Send + 'static,
{
    collect_mapped_parallel_tallied(builder, map, None)
}

/// Like [`collect_mapped_parallel`], but records every walk error it skips.
///
/// A directory the walk cannot open is a hole in the result, and a verb whose
/// answer is "everything under this root" cannot answer while a subtree was
/// never read. The caller passes the tally it will report in `files_failed`
/// and `errors[]`; `None` keeps the old posture of skipping in silence.
pub fn collect_mapped_parallel_tallied<T, F>(
    builder: &WalkBuilder,
    map: F,
    failures: Option<std::sync::Arc<Mutex<crate::ndjson_types::FailureTally>>>,
) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(&ignore::DirEntry) -> Option<T> + Sync + Send + 'static,
{
    use std::sync::Arc;

    // Arc so each walk worker gets a clone (FnMut factory cannot move once).
    let shards: Arc<Mutex<Vec<Vec<T>>>> = Arc::new(Mutex::new(Vec::new()));
    let map = Arc::new(map);
    struct ShardGuard<T> {
        local: Vec<T>,
        shards: Arc<Mutex<Vec<Vec<T>>>>,
    }
    impl<T> Drop for ShardGuard<T> {
        fn drop(&mut self) {
            let batch = std::mem::take(&mut self.local);
            if batch.is_empty() {
                return;
            }
            if let Ok(mut guard) = self.shards.lock() {
                guard.push(batch);
            }
        }
    }
    builder.build_parallel().run(|| {
        let mut guard = ShardGuard {
            local: Vec::new(),
            shards: Arc::clone(&shards),
        };
        let map = Arc::clone(&map);
        let failures = failures.clone();
        Box::new(move |result| {
            // `--limit` reached: more files cannot change the answer.
            if crate::signal::is_global_shutdown() || crate::output::limit_reached() {
                return WalkState::Quit;
            }
            let entry = match result {
                Ok(e) => e,
                Err(e) => {
                    if let Some(ref tally) = failures
                        && let Ok(mut tally) = tally.lock()
                    {
                        let (path, err) = classify_walk_error(&e);
                        tally.record(&path, err);
                    }
                    return WalkState::Continue;
                }
            };
            if let Some(item) = map(&entry) {
                guard.local.push(item);
            }
            WalkState::Continue
        })
    });
    match Arc::try_unwrap(shards) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => arc
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default(),
    }
    .into_iter()
    .flatten()
    .collect()
}

/// Type a walk error so the tally reports its real code.
///
/// The classifier in [`crate::error::AtomwriteError::from_walk_error`] keeps
/// the path and the OS kind, so a directory the walk cannot open shows up as
/// `PERMISSION_DENIED` on that directory. An error with no path of its own
/// (a metadata failure raised for `path`) is classified against the entry.
pub fn walk_failure(err: &ignore::Error, path: &std::path::Path) -> anyhow::Error {
    match crate::error::AtomwriteError::from_walk_error(err) {
        Some((_, error)) => error.into(),
        None => crate::error::AtomwriteError::from_io_at(
            path,
            std::io::Error::other(err.to_string()),
        )
        .into(),
    }
}

/// The path a walk failure carries when the walker names no path of its own.
///
/// `ignore` reports a path only when it wraps the error in `WithPath`; a bare
/// `Io`, `Glob` or `Partial` error reaches the tally without one. An empty
/// string in `errors[].path` reads like a file named `""`, so those failures
/// are attributed to this marker instead.
pub const WALK_ERROR_PATH: &str = "<walk>";

/// Classify a walk error into the path it names and a typed error.
///
/// The path is the one the walker reported. When it reports none, or reports
/// an empty one, the failure is attributed to [`WALK_ERROR_PATH`], so
/// `errors[]` never carries an empty path.
#[must_use]
pub fn classify_walk_error(err: &ignore::Error) -> (std::path::PathBuf, anyhow::Error) {
    match crate::error::AtomwriteError::from_walk_error(err) {
        Some((path, error)) if !path.as_os_str().is_empty() => (path, error.into()),
        Some((_, error)) => (std::path::PathBuf::from(WALK_ERROR_PATH), error.into()),
        None => {
            let path = std::path::PathBuf::from(WALK_ERROR_PATH);
            let error = walk_failure(err, &path);
            (path, error)
        }
    }
}

/// True when an ignore-file error came from I/O rather than from bad syntax.
fn ignore_error_is_io(err: &ignore::Error) -> bool {
    match err {
        ignore::Error::Io(_) => true,
        ignore::Error::Partial(errs) => errs.iter().any(ignore_error_is_io),
        ignore::Error::WithLineNumber { err, .. }
        | ignore::Error::WithPath { err, .. }
        | ignore::Error::WithDepth { err, .. } => ignore_error_is_io(err),
        _ => false,
    }
}

/// Every ignore file under `roots` that the walk would half-read in silence.
///
/// `ignore` keeps a malformed or unreadable `.gitignore` or `.ignore` inside
/// its own `Ignore` value and never yields it as a walk error, so a run over
/// that tree answers from a tree it never fully understood. Reading the file
/// here turns that into a failure that names the ignore file itself.
///
/// The cost is one small file per root, paid once before the walk starts.
#[must_use]
pub fn ignore_file_failures(
    roots: &[std::path::PathBuf],
) -> Vec<(std::path::PathBuf, anyhow::Error)> {
    let mut found = Vec::new();
    for root in roots {
        for name in [".gitignore", ".ignore"] {
            let path = root.join(name);
            if !path.is_file() {
                continue;
            }
            let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
            let Some(err) = builder.add(&path) else {
                continue;
            };
            let error: anyhow::Error = if ignore_error_is_io(&err) {
                crate::error::AtomwriteError::from_io_at(
                    &path,
                    std::io::Error::other(err.to_string()),
                )
                .into()
            } else {
                crate::error::AtomwriteError::InvalidInput {
                    reason: format!("{}: {err}", path.display()),
                }
                .into()
            };
            found.push((path, error));
        }
    }
    found
}

/// Record [`ignore_file_failures`] into a shared tally.
pub fn record_ignore_file_failures(
    roots: &[std::path::PathBuf],
    failures: Option<&std::sync::Arc<Mutex<crate::ndjson_types::FailureTally>>>,
) {
    let Some(tally) = failures else {
        return;
    };
    for (path, error) in ignore_file_failures(roots) {
        if let Ok(mut tally) = tally.lock() {
            tally.record(&path, error);
        }
    }
}

/// Record [`ignore_file_failures`] into a tally the caller owns.
pub fn record_ignore_file_failures_into(
    roots: &[std::path::PathBuf],
    tally: &mut crate::ndjson_types::FailureTally,
) {
    for (path, error) in ignore_file_failures(roots) {
        tally.record(&path, error);
    }
}

/// Like [`collect_mapped_parallel`], but stops after approximately `max_items`
/// accepted mappings (budgeted discovery for sparse outline).
///
/// Workers may overshoot slightly under race; callers should `truncate` after
/// sort for a hard cap. Returns `(items, truncated)` where `truncated` is true
/// when the budget was hit (or walk quit early for budget).
pub fn collect_mapped_parallel_budgeted<T, F>(
    builder: &WalkBuilder,
    max_items: u64,
    map: F,
) -> (Vec<T>, bool)
where
    T: Send + 'static,
    F: Fn(&ignore::DirEntry) -> Option<T> + Sync + Send + 'static,
{
    collect_mapped_parallel_budgeted_tallied(builder, max_items, map, None)
}

/// Like [`collect_mapped_parallel_budgeted`], but records every walk error.
///
/// Same contract as [`collect_mapped_parallel_tallied`]: a directory the walk
/// cannot open is a hole in the answer, so the caller passes the tally it will
/// report in `files_failed`.
pub fn collect_mapped_parallel_budgeted_tallied<T, F>(
    builder: &WalkBuilder,
    max_items: u64,
    map: F,
    failures: Option<std::sync::Arc<Mutex<crate::ndjson_types::FailureTally>>>,
) -> (Vec<T>, bool)
where
    T: Send + 'static,
    F: Fn(&ignore::DirEntry) -> Option<T> + Sync + Send + 'static,
{
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    if max_items == 0 {
        return (Vec::new(), true);
    }

    let accepted = Arc::new(AtomicU64::new(0));
    let hit_budget = Arc::new(AtomicBool::new(false));
    let shards: Arc<Mutex<Vec<Vec<T>>>> = Arc::new(Mutex::new(Vec::new()));
    let map = Arc::new(map);
    struct ShardGuard<T> {
        local: Vec<T>,
        shards: Arc<Mutex<Vec<Vec<T>>>>,
    }
    impl<T> Drop for ShardGuard<T> {
        fn drop(&mut self) {
            let batch = std::mem::take(&mut self.local);
            if batch.is_empty() {
                return;
            }
            if let Ok(mut guard) = self.shards.lock() {
                guard.push(batch);
            }
        }
    }
    builder.build_parallel().run(|| {
        let mut guard = ShardGuard {
            local: Vec::new(),
            shards: Arc::clone(&shards),
        };
        let map = Arc::clone(&map);
        let accepted = Arc::clone(&accepted);
        let hit_budget = Arc::clone(&hit_budget);
        let failures = failures.clone();
        Box::new(move |result| {
            if crate::signal::is_global_shutdown() || crate::output::limit_reached() {
                return WalkState::Quit;
            }
            if hit_budget.load(Ordering::Relaxed) {
                return WalkState::Quit;
            }
            let entry = match result {
                Ok(e) => e,
                Err(e) => {
                    if let Some(ref tally) = failures
                        && let Ok(mut tally) = tally.lock()
                    {
                        let (path, err) = classify_walk_error(&e);
                        tally.record(&path, err);
                    }
                    return WalkState::Continue;
                }
            };
            let Some(item) = map(&entry) else {
                return WalkState::Continue;
            };
            let prev = accepted.fetch_add(1, Ordering::Relaxed);
            if prev >= max_items {
                hit_budget.store(true, Ordering::Relaxed);
                return WalkState::Quit;
            }
            guard.local.push(item);
            if prev + 1 >= max_items {
                hit_budget.store(true, Ordering::Relaxed);
                return WalkState::Quit;
            }
            WalkState::Continue
        })
    });
    let items = match Arc::try_unwrap(shards) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => arc
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default(),
    }
    .into_iter()
    .flatten()
    .collect();
    let truncated =
        hit_budget.load(Ordering::Relaxed) || accepted.load(Ordering::Relaxed) > max_items;
    (items, truncated)
}

/// Collect regular-file paths via `WalkParallel`, then sort for stable NDJSON.
///
/// Uses the builder's thread bound (`apply_walk_threads` first).
pub fn collect_files_parallel(builder: &WalkBuilder) -> Vec<PathBuf> {
    collect_files_parallel_tallied(builder, None)
}

/// Like [`collect_files_parallel`], but records every walk error it skips.
///
/// A verb whose answer is "every file under this root" cannot answer while a
/// subtree was never read, so the caller passes the tally it reports in
/// `files_failed`.
pub fn collect_files_parallel_tallied(
    builder: &WalkBuilder,
    failures: Option<std::sync::Arc<Mutex<crate::ndjson_types::FailureTally>>>,
) -> Vec<PathBuf> {
    let mut paths = collect_mapped_parallel_tallied(
        builder,
        |entry| {
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                Some(entry.path().to_path_buf())
            } else {
                None
            }
        },
        failures,
    );
    sort_paths_parallel(&mut paths);
    paths
}

/// Build a multi-root file collect: `WalkBuilder::new(first)` + `.add()` rest.
///
/// Prefer this over N separate walks (docs.rs: better to call `add`).
pub fn collect_files_from_roots(
    roots: &[PathBuf],
    cli_threads: Option<usize>,
    configure: impl FnOnce(&mut WalkBuilder),
) -> Vec<PathBuf> {
    collect_files_from_roots_tallied(roots, cli_threads, configure, None)
}

/// Like [`collect_files_from_roots`], but records every walk error it skips.
pub fn collect_files_from_roots_tallied(
    roots: &[PathBuf],
    cli_threads: Option<usize>,
    configure: impl FnOnce(&mut WalkBuilder),
    failures: Option<std::sync::Arc<Mutex<crate::ndjson_types::FailureTally>>>,
) -> Vec<PathBuf> {
    if roots.is_empty() {
        return Vec::new();
    }
    let mut builder = WalkBuilder::new(&roots[0]);
    for root in roots.iter().skip(1) {
        builder.add(root);
    }
    configure(&mut builder);
    apply_walk_threads(&mut builder, cli_threads);
    record_ignore_file_failures(roots, failures.as_ref());
    collect_files_parallel_tallied(&builder, failures)
}

/// True when `command` can reach `par_iter` or `rayon::join`, directly or
/// through the verbs it calls.
///
/// The commands listed as false never fan out eagerly: single-file reads and
/// writes, key/value edits, metadata and discovery verbs. They skip the pool
/// at startup. `list` and `count` are here too: discovery is a `WalkParallel`
/// walk that needs no rayon thread, and `list` only reaches rayon from
/// [`crate::concurrency::sort_by_parallel`] past its threshold, which builds
/// the pool itself. A command missing from this list keeps the pool, which is
/// the safe default.
#[must_use]
pub fn command_uses_rayon(command: &crate::cli::Commands) -> bool {
    use crate::cli::Commands;
    !matches!(
        command,
        Commands::Read(_)
            | Commands::Stat(_)
            | Commands::List(_)
            | Commands::Count(_)
            | Commands::Write(_)
            | Commands::Move(_)
            | Commands::Get(_)
            | Commands::Set(_)
            | Commands::Del(_)
            | Commands::Rollback(_)
            | Commands::Calc(_)
            | Commands::Regex(_)
            | Commands::Extract(_)
            | Commands::ExtractArtifact(_)
            | Commands::Verify(_)
            | Commands::Query(_)
            | Commands::Outline(_)
            | Commands::Scratch(_)
            | Commands::Locale(_)
            | Commands::CommandsTree(_)
            | Commands::AgentSurface(_)
            | Commands::Completions(_)
    )
}

/// Record the worker bound and build the rayon pool only when `command` can
/// use it.
///
/// Called once from `run()`. A command that never reaches rayon pays nothing:
/// no worker threads are spawned for it. The bound is kept so a later
/// [`ensure_global_pool`] builds the pool with the operator's `--threads`.
pub fn plan_global_pool(command: &crate::cli::Commands, cli_threads: Option<usize>) {
    POOL_REQUEST.set(cli_threads).ok();
    if command_uses_rayon(command) {
        ensure_global_pool();
    }
}

/// Build the process-wide rayon pool from the recorded bound, at most once.
///
/// Idempotent and cheap after the first call. Code that is about to reach rayon
/// from a command [`command_uses_rayon`] reports as pool-free (blake3 hashing
/// of large buffers, the startup journal sweep) calls this first, so the pool
/// still honours `--threads` and the automatic ceiling instead of rayon's
/// default of one thread per CPU.
pub fn ensure_global_pool() {
    POOL_BUILD.call_once(|| {
        let requested = POOL_REQUEST.get().copied().flatten();
        let n = effective_threads(requested);
        CONFIGURED_SIZE.set(n).ok();
        match rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
        {
            Ok(()) => tracing::debug!(threads = n, "rayon global pool configured"),
            // Already built by another path or an OS limit — non-fatal.
            Err(e) => {
                tracing::debug!(error = %e, threads = n, "rayon global pool not reconfigured")
            }
        }
    });
}

/// Configure the process-wide rayon pool from the CLI bound, right now.
///
/// Records the bound and builds the pool at once. Prefer [`plan_global_pool`]
/// in `run()`, which builds only when the command needs it.
pub fn configure_global_pool(cli_threads: Option<usize>) {
    POOL_REQUEST.set(cli_threads).ok();
    ensure_global_pool();
}

/// Last requested pool size (for tests / observability).
#[must_use]
pub fn configured_pool_size() -> Option<usize> {
    CONFIGURED_SIZE.get().copied()
}

/// Bounded event channel capacity (alias for call sites).
#[must_use]
pub fn event_channel_cap() -> usize {
    EVENT_CHANNEL_CAP
}

/// Stop a scan's walker thread and wait for it.
///
/// Every scanning verb calls this on both the success and the error path of
/// its consumer. `stop` tells workers to quit at their next entry or send.
/// While the walker is still running, the queue is drained and dropped: a
/// queued message can own a resource a worker is blocked on (the per-file
/// stream of `search`), and a dropped receiver alone does not free queued
/// messages while a sender is alive. The thread is never left running after
/// the verb returns.
///
/// # Panics
///
/// Resumes the walker thread's panic, if it had one.
pub fn join_walker<T>(
    rx: crossbeam_channel::Receiver<T>,
    stop: &std::sync::atomic::AtomicBool,
    walker: std::thread::JoinHandle<()>,
) {
    use crossbeam_channel::RecvTimeoutError;
    stop.store(true, std::sync::atomic::Ordering::Release);
    while !walker.is_finished() {
        let polled = rx.recv_timeout(std::time::Duration::from_millis(
            crate::constants::JOIN_WALKER_POLL_MS,
        ));
        if matches!(polled, Err(RecvTimeoutError::Disconnected)) {
            break;
        }
    }
    drop(rx);
    if let Err(panic_payload) = walker.join() {
        std::panic::resume_unwind(panic_payload);
    }
}
