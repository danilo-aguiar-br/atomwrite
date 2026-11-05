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
//! | Generic walk map/collect | [`crate::concurrency::collect_mapped_parallel`] | same walk threads |
//! | Budgeted walk collect | [`crate::concurrency::collect_mapped_parallel_budgeted`] | walk threads + Atomic cap |
//! | CPU-bound multi-file (hash, score) | `rayon::par_iter` | global rayon pool size `N` |
//! | Independent multi-file mutations | `rayon::par_iter` | same pool |
//! | Event fan-in to NDJSON stdout | `crossbeam_channel::bounded` | [`crate::concurrency::EVENT_CHANNEL_CAP`] |
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
//! // CLI: --threads / -j / --max-concurrency
//! // Omitted or 0: N = min(available_parallelism(), MAX_AUTO_THREADS, ram_cap)
//! // Explicit N > 0: N, as asked (the operator names the number, so the
//! //                 automatic ceiling does not apply)
//! //
//! // RAM safety (Linux /proc/meminfo MemAvailable; else skip):
//! //   ram_cap = max(1, (MemAvailable_bytes * RAM_FRACTION_PERCENT%) / RAM_PER_TASK_BYTES)
//! //
//! // Directory walks, automatic: min(N, WALK_AUTO_MAX_THREADS), the same
//! // protection `ignore::WalkParallel` applies on its own.
//! //
//! // RAM_PER_TASK_BYTES = 16 MiB — conservative ceiling for one CLI file
//! // task (open + buffer + BLAKE3). Ground truth sample (debug binary,
//! // single-file `hash`): `/usr/bin/time -v` Maximum RSS ≈ 25 MiB process
//! // total; per-task incremental budget kept at 16 MiB. Re-measure when
//! // heavy optional features change the RSS profile.
//! ```
//!
//! The rayon pool is built **on demand**, once per process, and only for
//! commands that can reach `par_iter` or `rayon::join` (see
//! [`command_uses_rayon`](crate::concurrency::command_uses_rayon)); `read` and the other single-file verbs never pay for
//! it. Code that reaches rayon from a command outside that list must call
//! [`ensure_global_pool`](crate::concurrency::ensure_global_pool) first so the pool still honours the operator bound.
//! Every `par_iter` and every `WalkBuilder::threads` share that bound.
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
/// A-031: re-export of [`crate::constants::EVENT_CHANNEL_CAP`] (policy of record).
/// Bounded (never unbounded): a slow stdout consumer stalls walkers instead of
/// growing RAM without limit.
pub const EVENT_CHANNEL_CAP: usize = crate::constants::EVENT_CHANNEL_CAP;

/// Conservative RSS budget per concurrent file task (bytes).
///
/// A-031: re-export of [`crate::constants::RAM_PER_TASK_BYTES`].
/// Grounded as a safety ceiling for typical CLI file work (not ML). Operators
/// with tighter RAM are protected via `min(cpus, ram_cap)` even when
/// `--threads` is omitted.
pub const RAM_PER_TASK_BYTES: u64 = crate::constants::RAM_PER_TASK_BYTES;

static CONFIGURED_SIZE: OnceLock<usize> = OnceLock::new();

/// `--threads` value recorded by [`plan_global_pool`] for the lazy pool build.
static POOL_REQUEST: OnceLock<Option<usize>> = OnceLock::new();

/// Guards the one-time build of the rayon pool.
static POOL_BUILD: std::sync::Once = std::sync::Once::new();

/// Logical CPU count (`available_parallelism`), falling back to
/// [`crate::constants::CPU_FALLBACK_COUNT`].
#[must_use]
pub fn available_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(crate::constants::CPU_FALLBACK_COUNT)
        .max(1)
}

/// Free RAM in bytes when the platform exposes it (Linux `MemAvailable`).
#[must_use]
pub fn free_ram_bytes() -> Option<u64> {
    free_ram_bytes_impl()
}

#[cfg(target_os = "linux")]
fn free_ram_bytes_impl() -> Option<u64> {
    let text = std::fs::read_to_string(crate::constants::PROC_MEMINFO_PATH).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(crate::constants::PROC_MEMINFO_AVAILABLE_KEY) {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb.saturating_mul(crate::constants::BYTES_PER_KIB));
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn free_ram_bytes_impl() -> Option<u64> {
    None
}

/// RAM-derived concurrency cap
/// (`max(1, free * RAM_FRACTION_PERCENT% / RAM_PER_TASK)`).
#[must_use]
pub fn ram_concurrency_cap() -> Option<usize> {
    let free = free_ram_bytes()?;
    let usable = free.saturating_mul(crate::config::tuning().ram_fraction_percent)
        / crate::constants::PERCENT_SCALE;
    let cap = (usable / RAM_PER_TASK_BYTES).max(1);
    usize::try_from(cap).ok().map(|n| n.max(1))
}

/// Resolve the effective worker bound from CLI `--threads` / `--max-concurrency`.
///
/// * `None` or `Some(0)` → all logical CPUs, bounded by the automatic ceiling
///   [`crate::constants::MAX_AUTO_THREADS`] and by the RAM cap when known.
/// * `Some(n)` → exactly `n`. The operator named the number, so neither the
///   automatic ceiling nor the RAM cap applies; `--threads 1` determinism and
///   explicit load tests stay predictable.
#[must_use]
pub fn effective_threads(cli_threads: Option<usize>) -> usize {
    match cli_threads {
        Some(0) | None => {
            let cpus = available_cpus().min(crate::config::tuning().max_threads);
            match ram_concurrency_cap() {
                Some(ram) => cpus.min(ram).max(1),
                None => cpus,
            }
        }
        // An explicit bound is honoured, up to the hard ceiling the config
        // validator accepts: past it `-j` stops being a request and starts
        // being a way to exhaust the process.
        Some(n) => n
            .max(1)
            .min(crate::constants::CONCURRENCY_MAX_THREADS_CEILING as usize),
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
///
/// An explicit `--threads N` is passed as is. When it is omitted or `0`, the
/// count is the automatic bound capped at
/// [`crate::constants::WALK_AUTO_MAX_THREADS`], the protection `ignore` applies
/// by itself; handing the walker the full CPU count would override it.
pub fn apply_walk_threads(builder: &mut WalkBuilder, cli_threads: Option<usize>) {
    builder.threads(walk_thread_count(cli_threads));
}

/// Walk threads for a `--threads` value (see [`apply_walk_threads`]).
#[must_use]
pub fn walk_thread_count(cli_threads: Option<usize>) -> usize {
    const CEILING: usize = crate::constants::CONCURRENCY_MAX_THREADS_CEILING as usize;
    match cli_threads {
        Some(n) if n > 0 => n.min(CEILING),
        // The automatic bound was already resolved when the run started: a
        // single-file target resolves to one thread there, and reading the raw
        // `--threads` again here would discard it and spawn one worker per CPU
        // for a walk that has one file to visit.
        _ => match POOL_REQUEST.get().copied().flatten() {
            Some(n) if n > 0 => n.min(crate::constants::WALK_AUTO_MAX_THREADS),
            _ => effective_threads(None).min(crate::constants::WALK_AUTO_MAX_THREADS),
        },
    }
}

/// Walk threads of this run, from the `--threads` value `run()` recorded.
#[must_use]
pub fn requested_walk_threads() -> usize {
    walk_thread_count(POOL_REQUEST.get().copied().flatten())
}

/// Minimum length for parallel path sort (`par_sort_unstable`).
///
/// Below this, sequential `sort` is cheaper than rayon split overhead.
pub const PAR_SORT_THRESHOLD: usize = crate::constants::PAR_SORT_THRESHOLD;

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
        // A verb that never fans out eagerly (see
        // [`command_uses_rayon`]) reaches its pool only here, so the bound
        // still comes from `--threads` and not from rayon's default.
        ensure_global_pool();
        items.par_sort_unstable();
    } else {
        items.sort();
    }
}

/// Sort with a custom comparator (same threshold as [`sort_parallel`]).
pub fn sort_by_parallel<T: Send>(
    items: &mut [T],
    cmp: impl Fn(&T, &T) -> std::cmp::Ordering + Sync,
) {
    if items.len() >= PAR_SORT_THRESHOLD {
        ensure_global_pool();
        items.par_sort_unstable_by(cmp);
    } else {
        items.sort_by(cmp);
    }
}

include!("concurrency_walk.inc.rs");

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
        let mut paths = vec![PathBuf::from("c"), PathBuf::from("a"), PathBuf::from("b")];
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
    fn auto_threads_stay_under_the_hard_ceiling() {
        assert!(effective_threads(None) <= crate::constants::MAX_AUTO_THREADS);
        assert!(effective_threads(Some(0)) <= crate::constants::MAX_AUTO_THREADS);
    }

    #[test]
    fn explicit_threads_may_exceed_the_ceiling() {
        let asked = crate::constants::MAX_AUTO_THREADS + 8;
        assert_eq!(effective_threads(Some(asked)), asked);
    }

    #[test]
    fn single_file_verbs_skip_the_rayon_pool() {
        use clap::Parser;
        let uses = |argv: &[&str]| {
            let cli = crate::cli::Cli::try_parse_from(argv).expect("argv parses");
            command_uses_rayon(&cli.command)
        };
        assert!(!uses(&["atomwrite", "read", "f.txt"]));
        assert!(!uses(&["atomwrite", "write", "f.txt"]));
        assert!(uses(&["atomwrite", "search", "needle", "."]));
        assert!(uses(&[
            "atomwrite",
            "edit",
            "f.txt",
            "--old",
            "a",
            "--new",
            "b"
        ]));
        assert!(uses(&["atomwrite", "hash", "f.txt"]));
    }

    #[test]
    fn effective_threads_never_exceeds_explicit_cap() {
        let n = effective_threads(Some(1));
        assert_eq!(n, 1, "peak workers must not exceed operator --threads 1");
    }
}
