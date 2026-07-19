// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bounded concurrency and multi-core fan-out for atomwrite.
//!
//! # Design posture (Rules Rust — paralelismo)
//!
//! atomwrite is a **sync** one-shot CLI. Parallelism is the default modus
//! operandi for multi-item work; async/`tokio` is intentionally absent
//! (no long-lived I/O server). Coordination uses:
//!
//! | Workload | Tool | Bound |
//! |----------|------|-------|
//! | Directory walk + per-file I/O | `ignore::WalkParallel` | `WalkBuilder::threads(N)` |
//! | Generic walk map/collect | [`collect_mapped_parallel`] | same walk threads |
//! | Budgeted walk collect | [`collect_mapped_parallel_budgeted`] | walk threads + Atomic cap |
//! | CPU-bound multi-file (hash, score) | `rayon::par_iter` | global rayon pool size `N` |
//! | Independent multi-file mutations | `rayon::par_iter` | same pool |
//! | Event fan-in to NDJSON stdout | `crossbeam_channel::bounded` | [`EVENT_CHANNEL_CAP`] |
//!
//! # Workload classification (per command family)
//!
//! - **CPU-bound** — multi-file BLAKE3 (`hash`), Jaccard scoring
//!   (`semantic-search`), regex synthesis (`regex`): rayon.
//! - **I/O-bound** — `search` (content + `--target files`) / `replace` /
//!   `count` / multi-path + recursive `delete` / recursive `copy` /
//!   multi-path `backup` / multi-path `case` / multi-target `prune-backups` /
//!   non-txn `batch`: WalkParallel and/or rayon over independent paths.
//! - **Mixed** — `transform` / `scope` / `codemod` (walk I/O + AST CPU):
//!   WalkParallel (codemod delegates to transform).
//! - **Sequential by contract** — single-file ops (`read`/`write`/`edit`/…),
//!   transactional `batch` **ops** (ordered rollback; pre-backup is parallel),
//!   `recipe` step order, `watch` event coalescing loop (checksums fan out
//!   when multi-ready). Budgeted walks (`sparse list`/`outline`) use
//!   `collect_mapped_parallel_budgeted` + clamp, not sequential `build()`.
//!
//! # Bound formula
//!
//! ```text
//! // CLI: --threads / -j / --max-concurrency  (0 = all logical CPUs)
//! // When omitted: N = available_parallelism()  (full-core modus operandi)
//! //
//! // RAM safety (Linux /proc/meminfo MemAvailable; else skip):
//! //   ram_cap = max(1, (MemAvailable_bytes * 50%) / RAM_PER_TASK_BYTES)
//! //   N       = min(cpu_cap, ram_cap)
//! //
//! // RAM_PER_TASK_BYTES = 16 MiB — conservative ceiling for one CLI file
//! // task (open + buffer + BLAKE3). Ground truth sample (debug binary,
//! // single-file `hash`): `/usr/bin/time -v` Maximum RSS ≈ 25 MiB process
//! // total; per-task incremental budget kept at 16 MiB. Re-measure when
//! // heavy optional features change the RSS profile.
//! ```
//!
//! Rayon `build_global` is called **once** per process from `run()` so every
//! `par_iter` and every `WalkBuilder::threads` share the same operator bound.
//!
//! # Nested rayon (BLAKE3 × multi-file)
//!
//! Large-file BLAKE3 may use `Hasher::update_rayon` / `update_mmap_rayon` while
//! multi-file commands already run on the global pool. Rayon work-stealing
//! tolerates nesting; operators who need strict core caps set `--threads N`.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use ignore::{WalkBuilder, WalkState};
use rayon::prelude::*;

/// Backpressure capacity for walker → main NDJSON event channels.
///
/// Bounded (never unbounded): a slow stdout consumer stalls walkers instead of
/// growing RAM without limit.
pub const EVENT_CHANNEL_CAP: usize = 1024;

/// Conservative RSS budget per concurrent file task (bytes).
///
/// Grounded as a safety ceiling for typical CLI file work (not ML). Operators
/// with tighter RAM are protected via `min(cpus, ram_cap)` even when
/// `--threads` is omitted.
pub const RAM_PER_TASK_BYTES: u64 = 16 * 1024 * 1024;

/// Fraction of free RAM eligible for concurrent tasks (50% safety margin).
const RAM_SAFETY_FRACTION_NUM: u64 = 1;
const RAM_SAFETY_FRACTION_DEN: u64 = 2;

static CONFIGURED_SIZE: OnceLock<usize> = OnceLock::new();

/// Logical CPU count (`available_parallelism`), falling back to 4.
#[must_use]
pub fn available_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .max(1)
}

/// Free RAM in bytes when the platform exposes it (Linux `MemAvailable`).
#[must_use]
pub fn free_ram_bytes() -> Option<u64> {
    free_ram_bytes_impl()
}

#[cfg(target_os = "linux")]
fn free_ram_bytes_impl() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb.saturating_mul(1024));
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn free_ram_bytes_impl() -> Option<u64> {
    None
}

/// RAM-derived concurrency cap (`max(1, free*50% / RAM_PER_TASK)`).
#[must_use]
pub fn ram_concurrency_cap() -> Option<usize> {
    let free = free_ram_bytes()?;
    let usable = free.saturating_mul(RAM_SAFETY_FRACTION_NUM) / RAM_SAFETY_FRACTION_DEN;
    let cap = (usable / RAM_PER_TASK_BYTES).max(1);
    usize::try_from(cap).ok().map(|n| n.max(1))
}

/// Resolve the effective worker bound from CLI `--threads` / `--max-concurrency`.
///
/// * `None` → full-core modus operandi, still RAM-capped when possible.
/// * `Some(0)` → all logical CPUs, still RAM-capped.
/// * `Some(n)` → exactly `n` (operator override; not RAM-clamped so explicit
///   load tests and `--threads 1` determinism stay predictable).
#[must_use]
pub fn effective_threads(cli_threads: Option<usize>) -> usize {
    match cli_threads {
        Some(0) | None => {
            let cpus = available_cpus();
            match ram_concurrency_cap() {
                Some(ram) => cpus.min(ram).max(1),
                None => cpus,
            }
        }
        Some(n) => n.max(1),
    }
}

/// True when multi-item fan-out is worth coordinating (strictly > 1 item).
#[must_use]
pub fn should_parallelize(item_count: usize) -> bool {
    item_count > 1
}

/// Apply the shared worker bound to an `ignore` walk builder.
///
/// **Honesty:** `WalkBuilder::threads` only affects [`WalkBuilder::build_parallel`]
/// (docs.rs / ignore). Pair this with [`collect_files_parallel`] or
/// `build_parallel().run(...)`, never with bare `build()` if the operator
/// expects `--threads` to speed discovery.
pub fn apply_walk_threads(builder: &mut WalkBuilder, cli_threads: Option<usize>) {
    builder.threads(effective_threads(cli_threads));
}

/// Minimum length for parallel path sort (`par_sort_unstable`).
///
/// Below this, sequential `sort` is cheaper than rayon split overhead.
pub const PAR_SORT_THRESHOLD: usize = 4096;

/// Sort paths for stable NDJSON: sequential for small lists, parallel for large.
///
/// Rules Rust: prefer `par_sort_unstable` on large collections.
pub fn sort_paths_parallel(paths: &mut [PathBuf]) {
    if paths.len() >= PAR_SORT_THRESHOLD {
        paths.par_sort_unstable();
    } else {
        paths.sort();
    }
}

/// Sort any `Ord` slice with the same threshold policy as path lists.
pub fn sort_parallel<T: Ord + Send>(items: &mut [T]) {
    if items.len() >= PAR_SORT_THRESHOLD {
        items.par_sort_unstable();
    } else {
        items.sort();
    }
}

/// Sort with a custom comparator (same threshold as [`sort_parallel`]).
pub fn sort_by_parallel<T: Send>(items: &mut [T], cmp: impl Fn(&T, &T) -> std::cmp::Ordering + Sync) {
    if items.len() >= PAR_SORT_THRESHOLD {
        items.par_sort_unstable_by(cmp);
    } else {
        items.sort_by(cmp);
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_cpus_at_least_one() {
        assert!(available_cpus() >= 1);
    }

    #[test]
    fn effective_threads_explicit_honored() {
        assert_eq!(effective_threads(Some(1)), 1);
        assert_eq!(effective_threads(Some(3)), 3);
    }

    #[test]
    fn effective_threads_zero_uses_cpus() {
        let n = effective_threads(Some(0));
        assert!(n >= 1);
        assert!(n <= available_cpus().max(ram_concurrency_cap().unwrap_or(usize::MAX)));
    }

    #[test]
    fn should_parallelize_threshold() {
        assert!(!should_parallelize(0));
        assert!(!should_parallelize(1));
        assert!(should_parallelize(2));
    }

    #[test]
    fn ram_cap_sane_when_present() {
        if let Some(cap) = ram_concurrency_cap() {
            assert!(cap >= 1);
        }
    }

    #[test]
    fn collect_files_parallel_matches_sequential_build() {
        let dir = tempfile::tempdir().expect("tempdir");
        for name in ["a.txt", "b.txt", "c.txt"] {
            std::fs::write(dir.path().join(name), b"x").expect("write");
        }
        std::fs::create_dir(dir.path().join("sub")).expect("mkdir");
        std::fs::write(dir.path().join("sub/d.txt"), b"y").expect("write");

        let mut builder = WalkBuilder::new(dir.path());
        builder.hidden(false).git_ignore(false);
        apply_walk_threads(&mut builder, Some(2));

        let mut sequential = Vec::new();
        for entry in builder.build().flatten() {
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                sequential.push(entry.into_path());
            }
        }
        sequential.sort();

        let parallel = collect_files_parallel(&builder);
        assert_eq!(
            parallel, sequential,
            "parallel collect must match sequential file set"
        );
        assert_eq!(parallel.len(), 4);
    }

    #[test]
    fn collect_mapped_parallel_includes_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("f.txt"), b"x").expect("write");
        std::fs::create_dir(dir.path().join("sub")).expect("mkdir");

        let mut builder = WalkBuilder::new(dir.path());
        builder.hidden(false).git_ignore(false);
        apply_walk_threads(&mut builder, Some(2));

        let mut kinds: Vec<String> = collect_mapped_parallel(&builder, |e| {
            let ft = e.file_type()?;
            let kind = if ft.is_dir() {
                "dir"
            } else if ft.is_file() {
                "file"
            } else {
                "other"
            };
            Some(kind.to_string())
        });
        kinds.sort();
        assert!(kinds.iter().any(|k| k == "dir"));
        assert!(kinds.iter().any(|k| k == "file"));
    }

    #[test]
    fn collect_mapped_parallel_budgeted_respects_cap() {
        let dir = tempfile::tempdir().expect("tempdir");
        for i in 0..20 {
            std::fs::write(dir.path().join(format!("f{i}.txt")), b"x").expect("write");
        }
        let mut builder = WalkBuilder::new(dir.path());
        builder.hidden(false).git_ignore(false);
        apply_walk_threads(&mut builder, Some(2));

        let (paths, truncated) = collect_mapped_parallel_budgeted(&builder, 5, |e| {
            if e.file_type().is_some_and(|ft| ft.is_file()) {
                Some(e.path().to_path_buf())
            } else {
                None
            }
        });
        assert!(truncated, "budget of 5 over 20 files must truncate");
        assert!(
            paths.len() <= 8,
            "budgeted collect should not wildly overshoot (got {})",
            paths.len()
        );
        assert!(!paths.is_empty());
    }

    #[test]
    fn sort_paths_parallel_orders_small_lists() {
        let mut paths = vec![
            PathBuf::from("c"),
            PathBuf::from("a"),
            PathBuf::from("b"),
        ];
        sort_paths_parallel(&mut paths);
        assert_eq!(
            paths,
            vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")]
        );
    }

    #[test]
    fn configure_global_pool_records_size() {
        // May no-op if pool already built in this process; size is still recorded
        // on first set. Accept either first-set or already-configured.
        configure_global_pool(Some(2));
        let n = configured_pool_size();
        assert!(n.is_some(), "pool size must be recorded");
        assert!(n.unwrap() >= 1);
        // Explicit override request is stored even if build_global fails later.
        assert!(effective_threads(Some(2)) == 2);
    }

    #[test]
    fn effective_threads_never_exceeds_explicit_cap() {
        let n = effective_threads(Some(1));
        assert_eq!(n, 1, "peak workers must not exceed operator --threads 1");
    }
}
