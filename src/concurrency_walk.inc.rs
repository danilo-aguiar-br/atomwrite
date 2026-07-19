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
        Box::new(move |result| {
            if crate::signal::is_global_shutdown() {
                return WalkState::Quit;
            }
            let entry = match result {
                Ok(e) => e,
                Err(_) => return WalkState::Continue,
            };
            if let Some(item) = map(&entry) {
                guard.local.push(item);
            }
            WalkState::Continue
        })
    });
    match Arc::try_unwrap(shards) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => arc.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default(),
    }
    .into_iter()
    .flatten()
    .collect()
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
        Box::new(move |result| {
            if crate::signal::is_global_shutdown() {
                return WalkState::Quit;
            }
            if hit_budget.load(Ordering::Relaxed) {
                return WalkState::Quit;
            }
            let entry = match result {
                Ok(e) => e,
                Err(_) => return WalkState::Continue,
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
        Err(arc) => arc.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default(),
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
    let mut paths = collect_mapped_parallel(builder, |entry| {
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            Some(entry.path().to_path_buf())
        } else {
            None
        }
    });
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
    if roots.is_empty() {
        return Vec::new();
    }
    let mut builder = WalkBuilder::new(&roots[0]);
    for root in roots.iter().skip(1) {
        builder.add(root);
    }
    configure(&mut builder);
    apply_walk_threads(&mut builder, cli_threads);
    collect_files_parallel(&builder)
}

/// Configure the process-wide rayon pool once from the CLI bound.
///
/// Subsequent calls are no-ops for the size record; rayon's `build_global`
/// only succeeds once per process. Failures are non-fatal.
pub fn configure_global_pool(cli_threads: Option<usize>) {
    let n = effective_threads(cli_threads);
    let _ = CONFIGURED_SIZE.set(n);
    match rayon::ThreadPoolBuilder::new().num_threads(n).build_global() {
        Ok(()) => {
            tracing::debug!(threads = n, "rayon global pool configured");
        }
        Err(e) => {
            // Already built (tests / double entry) or OS limit — non-fatal.
            tracing::debug!(error = %e, threads = n, "rayon global pool not reconfigured");
        }
    }
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

