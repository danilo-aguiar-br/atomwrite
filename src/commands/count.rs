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

use anyhow::{Context, Result};

use crate::cli::{CountArgs, GlobalArgs};
use crate::commands::BACKUP_FILENAME_RE;
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
    let mut walker = ignore::WalkBuilder::new(&canonical_paths[0]);
    for p in canonical_paths.iter().skip(1) {
        walker.add(p);
    }
    walker
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore);

    crate::concurrency::apply_walk_threads(&mut walker, global.threads);

    if !args.include.is_empty() {
        let mut types_builder = ignore::types::TypesBuilder::new();
        for pat in &args.include {
            types_builder
                .add_def(&format!("custom:{pat}"))
                .context("invalid include glob")?;
        }
        types_builder.select("custom");
        walker.types(types_builder.build().context("build types")?);
    }
    if !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&canonical_paths[0]);
        for pat in &args.exclude {
            overrides.add(&format!("!{pat}"))?;
        }
        walker.overrides(overrides.build()?);
    }

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

            let path = entry.path();
            let Ok(validated) = crate::path_safety::validate_path(path, &ws) else {
                return ignore::WalkState::Continue;
            };

            let meta = match std::fs::metadata(&validated) {
                Ok(m) => m,
                Err(_) => return ignore::WalkState::Continue,
            };

            let size = meta.len();
            // Relaxed: pure counters, no dependent payload published via these atomics.
            total_bytes.fetch_add(size, Ordering::Relaxed);
            total_files.fetch_add(1, Ordering::Relaxed);

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
                    Err(_) => (0, 0),
                }
            } else {
                (0, 0)
            };

            total_lines.fetch_add(lines, Ordering::Relaxed);
            total_blank.fetch_add(blank, Ordering::Relaxed);

            if let Some(ref mut guard) = ext_guard {
                let file_name = validated.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let ext = if BACKUP_FILENAME_RE.is_match(file_name) {
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
        writer.write_event(&CountByExtOutput {
            r#type: "count",
            mode: "by_extension",
            by_extension: by_extension_map,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
    } else if args.by_size {
        let shards = match by_size_shards {
            Some(arc) => take_mutex_map(arc),
            None => Vec::new(),
        };
        let mut items: Vec<(PathBuf, u64)> = shards.into_iter().flatten().collect();
        // Sort descending by size and truncate to top N.
        items.sort_by(|a, b| b.1.cmp(&a.1));
        let top = args.top.min(items.len());
        let items: Vec<SizeEntry> = items
            .into_iter()
            .take(top)
            .map(|(p, s)| SizeEntry {
                path: p.display().to_string(),
                bytes: s,
            })
            .collect();
        writer.write_event(&CountBySizeOutput {
            r#type: "count",
            mode: "by_size",
            items,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
    } else {
        writer.write_event(&CountTotalOutput {
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
        })?;
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
