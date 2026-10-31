// SPDX-License-Identifier: MIT OR Apache-2.0

//! File and line counting with optional grouping by extension.
//!
//! Workload: I/O-bound (directory walk + optional content read for line counts).
//! Parallelism: `ignore::WalkParallel` via `WalkBuilder::build_parallel`.
//! Worker bound from `concurrency::apply_walk_threads` (default = all cores).
//! Aggregation uses atomics for totals. For `--by-extension` / `--by-size`,
//! each walk worker keeps a **local** map/vec and merges once on Drop (no
//! Mutex on the per-file hot path).
//!
//! Sequential alternative rejected: per-file open+read dominates; walk
//! parallelism matches `search` / `replace` and has real gain on large trees.
//!
//! Interior mutability (Rules Rust):
//! - `AtomicU64` + `Ordering::Relaxed` for independent tallies (no data publication).
//! - `AtomicBool` shutdown poll uses `Ordering::Acquire` (pairs with Release stores
//!   in `signal::ShutdownSignal`).
//! - `Mutex` only at worker Drop (merge shards once per thread), never per entry.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use anyhow::Result;

use crate::cli::{CountArgs, GlobalArgs};
use crate::commands::is_backup_timestamp_name;
use crate::ndjson_types::{
    CountByExtOutput, CountBySizeOutput, CountTotalOutput, CountTotals, ExtCountOutput, SizeEntry,
};
use crate::output::NdjsonWriter;

/// Lock a walk-aggregation mutex, recovering from poison so a panicked worker
/// cannot silently drop subsequent tallies (Rules: treat `PoisonError`).
#[inline]
fn lock_agg<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Consume an `Arc<Mutex<T>>` after the walker joins, recovering poison.
fn take_mutex_map<T: Default>(arc: Arc<Mutex<T>>) -> T {
    match Arc::try_unwrap(arc) {
        Ok(mutex) => mutex.into_inner().unwrap_or_else(|e| e.into_inner()),
        Err(arc) => std::mem::take(&mut *lock_agg(&arc)),
    }
}

/// Count lines, pattern matches, or files grouped by extension.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if reading files fails.
#[tracing::instrument(skip_all, fields(command = "count"))]
pub fn cmd_count(
    args: &CountArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let canonical_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;
    // `--include` selects by file name here, so this verb takes the by-type
    // flavor of the one shared walker.
    let walker = crate::commands::build_scoped_walker_flavored(
        &canonical_paths,
        global,
        crate::commands::WalkerFlavor::RespectGlobalFlagsByType,
        &args.include,
        &args.exclude,
    )?;

    // `--by-size` only needs metadata; skip content reads (zero heap for file bodies).
    let need_line_counts = !args.by_size;
    let max_size = global.effective_max_filesize();
    let by_size = args.by_size;
    let by_extension = args.by_extension;

    // Per-run counters. Ordering::Relaxed: independent tallies; final loads
    // happen after WalkParallel joins, so no cross-field publication barrier.
    let total_files = Arc::new(AtomicU64::new(0));
    let total_lines = Arc::new(AtomicU64::new(0));
    let total_blank = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));
    // Shard bags: workers push local maps/vecs once on Drop (not per entry).
    type ExtShardBag = Arc<Mutex<Vec<BTreeMap<String, ExtCountOutput>>>>;
    type SizeShardBag = Arc<Mutex<Vec<Vec<(PathBuf, u64)>>>>;
    let by_ext_shards: Option<ExtShardBag> = if by_extension {
        Some(Arc::new(Mutex::new(Vec::new())))
    } else {
        None
    };
    let by_size_shards: Option<SizeShardBag> = if by_size {
        Some(Arc::new(Mutex::new(Vec::new())))
    } else {
        None
    };
    let ws: Arc<Path> = Arc::from(workspace.as_path());
    // Failures are recorded on the error path only, so the mutex never sits on
    // the per-entry hot path. A file the walk saw but could not open must show
    // up in the summary: a total computed without it is not the tree's total.
    let failures: Arc<Mutex<crate::ndjson_types::FailureTally>> =
        Arc::new(Mutex::new(crate::ndjson_types::FailureTally::default()));
    // An ignore file the walk half-reads is a hole too, and `ignore` never
    // reports it by itself.
    crate::concurrency::record_ignore_file_failures(&canonical_paths, Some(&failures));

    // Cooperative cancel: same AtomicBool as SIGINT/SIGTERM / --timeout-secs
    // (graceful-shutdown rules: poll every long loop; Quit stops WalkParallel).
    // Ordering::Acquire pairs with Release stores in `signal::record_signal`.
    let shutdown_flag = crate::signal::get_or_install_handlers()
        .map(|s| s.flag())
        .unwrap_or_else(|_| Arc::new(std::sync::atomic::AtomicBool::new(false)));

    walker.build_parallel().run(|| {
        let total_files = Arc::clone(&total_files);
        let total_lines = Arc::clone(&total_lines);
        let total_blank = Arc::clone(&total_blank);
        let total_bytes = Arc::clone(&total_bytes);
        let by_ext_shards = by_ext_shards.clone();
        let by_size_shards = by_size_shards.clone();
        let ws = Arc::clone(&ws);
        let failures = Arc::clone(&failures);
        let shutdown_flag = Arc::clone(&shutdown_flag);

        // Worker-local composites — merge once on Drop (Mutex-free hot path).
        type ExtShardBag = Arc<Mutex<Vec<BTreeMap<String, ExtCountOutput>>>>;
        struct ExtGuard {
            local: BTreeMap<String, ExtCountOutput>,
            shards: ExtShardBag,
        }
        impl Drop for ExtGuard {
            fn drop(&mut self) {
                let batch = std::mem::take(&mut self.local);
                if batch.is_empty() {
                    return;
                }
                if let Ok(mut g) = self.shards.lock() {
                    g.push(batch);
                }
            }
        }
        type SizeShardBag = Arc<Mutex<Vec<Vec<(PathBuf, u64)>>>>;
        struct SizeGuard {
            local: Vec<(PathBuf, u64)>,
            shards: SizeShardBag,
        }
        impl Drop for SizeGuard {
            fn drop(&mut self) {
                let batch = std::mem::take(&mut self.local);
                if batch.is_empty() {
                    return;
                }
                if let Ok(mut g) = self.shards.lock() {
                    g.push(batch);
                }
            }
        }

        let mut ext_guard = by_ext_shards.map(|shards| ExtGuard {
            local: BTreeMap::new(),
            shards,
        });
        let mut size_guard = by_size_shards.map(|shards| SizeGuard {
            local: Vec::new(),
            shards,
        });

        Box::new(move |entry| {
            // Acquire: observe shutdown published by signal/timeout threads.
            if shutdown_flag.load(Ordering::Acquire) || crate::output::limit_reached() {
                return ignore::WalkState::Quit;
            }

            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    // A directory the walk cannot open hides every file under
                    // it, so the total computed without it is not the tree's.
                    if let Ok(mut tally) = failures.lock() {
                        let (path, err) = crate::concurrency::classify_walk_error(&e);
                        tally.record(&path, err);
                    }
                    return ignore::WalkState::Continue;
                }
            };

            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                return ignore::WalkState::Continue;
            }

            let path = entry.path();
            let Ok(validated) = crate::path_safety::validate_path(path, &ws) else {
                return ignore::WalkState::Continue;
            };

            let meta = match std::fs::metadata(&validated) {
                Ok(m) => m,
                Err(e) => {
                    // A file the walk listed and `count` cannot stat is a hole
                    // in the total, not a file to ignore.
                    if let Ok(mut tally) = failures.lock() {
                        tally.record(
                            &validated,
                            crate::error::AtomwriteError::Io { source: e }.into(),
                        );
                    }
                    return ignore::WalkState::Continue;
                }
            };

            let size = meta.len();

            // `--limit` is a work budget: counting stops at the cap, so the
            // published total names what was measured and the summary says the
            // walk stopped short instead of pretending the tree was read. The
            // claim is taken BEFORE the counters move, so a worker that loses
            // the race drops its entry instead of overshooting the cap.
            let seen = total_files.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(cap) = crate::output::limit_cap()
                && seen > cap
            {
                total_files.fetch_sub(1, Ordering::Relaxed);
                crate::output::mark_limit_reached();
                return ignore::WalkState::Quit;
            }
            let _reached = crate::output::spend_budget();
            // Relaxed: pure counters, no dependent payload published via these atomics.
            total_bytes.fetch_add(size, Ordering::Relaxed);

            if by_size {
                if let Some(ref mut guard) = size_guard {
                    guard.local.push((validated, size));
                }
                return ignore::WalkState::Continue;
            }

            let (lines, blank) = if need_line_counts {
                // I/O outside any Mutex (rules: critical sections free of file I/O).
                match crate::file_io::read_file_string(&validated, max_size) {
                    Ok(content) => count_lines_and_blank(&content),
                    Err(e) => {
                        // Too large, binary or not UTF-8 is a file the caller
                        // told `count` to leave alone; anything else is a hole.
                        if !crate::output::is_content_rejection(&e)
                            && let Ok(mut tally) = failures.lock()
                        {
                            tally.record(&validated, e);
                        }
                        (0, 0)
                    }
                }
            } else {
                (0, 0)
            };

            total_lines.fetch_add(lines, Ordering::Relaxed);
            total_blank.fetch_add(blank, Ordering::Relaxed);

            if let Some(ref mut guard) = ext_guard {
                let file_name = validated.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let ext = if is_backup_timestamp_name(file_name) {
                    "backup".to_owned()
                } else {
                    validated
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("(none)")
                        .to_owned()
                };
                let entry_count = guard.local.entry(ext).or_default();
                entry_count.files += 1;
                entry_count.bytes += size;
                entry_count.lines += lines;
                entry_count.blank += blank;
            }

            ignore::WalkState::Continue
        })
    });

    if crate::signal::is_global_shutdown() {
        return Ok(());
    }

    let (failed_count, failed_items, first_failure) = take_mutex_map(failures).into_parts();
    let files_failed = (failed_count > 0).then_some(failed_count);
    let errors = (!failed_items.is_empty()).then_some(failed_items);
    let stopped_early = crate::output::limit_reached().then_some(true);

    if args.by_extension {
        let shards = match by_ext_shards {
            Some(arc) => take_mutex_map(arc),
            None => Vec::new(),
        };
        let mut by_extension_map: BTreeMap<String, ExtCountOutput> = BTreeMap::new();
        for shard in shards {
            for (ext, c) in shard {
                let e = by_extension_map.entry(ext).or_default();
                e.files += c.files;
                e.bytes += c.bytes;
                e.lines += c.lines;
                e.blank += c.blank;
            }
        }
        let total = CountTotals {
            files: total_files.load(Ordering::Relaxed),
            lines: total_lines.load(Ordering::Relaxed),
            blank: total_blank.load(Ordering::Relaxed),
            bytes: total_bytes.load(Ordering::Relaxed),
        };
        // `--top` means the same thing in all three modes: keep the N largest
        // groups and name how many were left out. Ordering by files first
        // makes the cut independent of the BTreeMap's alphabetical keys.
        let mut groups: Vec<(String, ExtCountOutput)> = by_extension_map.into_iter().collect();
        groups.sort_by(|a, b| b.1.files.cmp(&a.1.files).then_with(|| a.0.cmp(&b.0)));
        let top = args.top.min(groups.len());
        let items_omitted = (groups.len() - top) as u64;
        let by_extension: BTreeMap<String, ExtCountOutput> = groups.into_iter().take(top).collect();
        let out = CountByExtOutput {
            r#type: "count",
            mode: "by_extension",
            by_extension,
            total,
            items_omitted: Some(items_omitted),
            elapsed_ms: start.elapsed().as_millis() as u64,
            files_failed,
            partial: crate::ndjson_types::partial_of(files_failed),
            errors,
            walker_stopped_early: stopped_early,
        };
        if let Some(err) = first_failure {
            return Err(writer.publish_failure(&out, err));
        }
        writer.write_terminal(&out)?;
    } else if args.by_size {
        let shards = match by_size_shards {
            Some(arc) => take_mutex_map(arc),
            None => Vec::new(),
        };
        let mut items: Vec<(PathBuf, u64)> = shards.into_iter().flatten().collect();
        // Sort descending by size and truncate to top N.
        items.sort_by(|a, b| b.1.cmp(&a.1));
        let top = args.top.min(items.len());
        let items_omitted = (items.len() - top) as u64;
        let items: Vec<SizeEntry> = items
            .into_iter()
            .take(top)
            .map(|(p, s)| SizeEntry {
                path: p.display().to_string(),
                bytes: s,
            })
            .collect();
        let out = CountBySizeOutput {
            r#type: "count",
            mode: "by_size",
            items,
            total: CountTotals {
                files: total_files.load(Ordering::Relaxed),
                lines: total_lines.load(Ordering::Relaxed),
                blank: total_blank.load(Ordering::Relaxed),
                bytes: total_bytes.load(Ordering::Relaxed),
            },
            items_omitted: Some(items_omitted),
            elapsed_ms: start.elapsed().as_millis() as u64,
            files_failed,
            partial: crate::ndjson_types::partial_of(files_failed),
            errors,
            walker_stopped_early: stopped_early,
        };
        if let Some(err) = first_failure {
            return Err(writer.publish_failure(&out, err));
        }
        writer.write_terminal(&out)?;
    } else {
        let out = CountTotalOutput {
            r#type: "count",
            mode: "lines",
            total: CountTotals {
                // Post-join loads: walker threads finished → Relaxed is sufficient.
                files: total_files.load(Ordering::Relaxed),
                lines: total_lines.load(Ordering::Relaxed),
                blank: total_blank.load(Ordering::Relaxed),
                bytes: total_bytes.load(Ordering::Relaxed),
            },
            elapsed_ms: start.elapsed().as_millis() as u64,
            files_failed,
            partial: crate::ndjson_types::partial_of(files_failed),
            errors,
            walker_stopped_early: stopped_early,
        };
        if let Some(err) = first_failure {
            return Err(writer.publish_failure(&out, err));
        }
        writer.write_terminal(&out)?;
    }

    Ok(())
}

/// Single-pass line + blank-line tally (avoids dual `content.lines()` scans).
#[inline]
fn count_lines_and_blank(content: &str) -> (u64, u64) {
    let mut lines = 0u64;
    let mut blank = 0u64;
    for line in content.lines() {
        lines += 1;
        if line.trim().is_empty() {
            blank += 1;
        }
    }
    (lines, blank)
}

#[cfg(test)]
mod tests {
    use super::count_lines_and_blank;

    #[test]
    fn count_lines_and_blank_mixed() {
        let (lines, blank) = count_lines_and_blank("a\n\nb\n  \nc\n");
        assert_eq!(lines, 5);
        assert_eq!(blank, 2);
    }

    #[test]
    fn count_lines_empty() {
        let (lines, blank) = count_lines_and_blank("");
        assert_eq!(lines, 0);
        assert_eq!(blank, 0);
    }
}
