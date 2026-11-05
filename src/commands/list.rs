// SPDX-License-Identifier: MIT OR Apache-2.0

//! Directory listing with metadata, gitignore support, and depth control.
//!
//! Workload: I/O-bound (directory walk + stat per entry).
//! Parallelism: multi-root via one `WalkBuilder` + `.add` (docs.rs);
//! discovery via `WalkParallel` bound by `--threads` / `--max-concurrency`
//! (`apply_walk_threads` + `collect_mapped_parallel`). Entries are
//! materialised, sorted (`sort_parallel`), then emitted for deterministic
//! NDJSON order. Trade-off: RAM for path+meta vectors vs monorepo readdir
//! wall-clock (paths are small; `--long` stats fan out during the walk).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;

use crate::cli::{GlobalArgs, ListArgs};
use crate::commands::is_backup_timestamp_name;
use crate::concurrency::{collect_mapped_parallel_tallied, sort_by_parallel};
use crate::ndjson_types::{ListEntry, ListSummary};
use crate::output::NdjsonWriter;

/// Intermediate walk record (Send) — sorted then emitted as NDJSON.
struct ListRecord {
    path: String,
    kind: &'static str,
    size: Option<u64>,
    modified: Option<String>,
    bytes: u64,
    ext: Option<String>,
}

fn rel_path_for(path: &Path, workspace: &Path, roots: &[PathBuf]) -> String {
    path.strip_prefix(workspace)
        .ok()
        .or_else(|| roots.iter().find_map(|r| path.strip_prefix(r).ok()))
        .unwrap_or(path)
        .display()
        .to_string()
}

fn format_mtime(meta: &std::fs::Metadata) -> Option<String> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| {
            let (y, mo, da, h, m, s) = crate::atomic::epoch_to_utc(d.as_secs());
            format!("{y:04}-{mo:02}-{da:02}T{h:02}:{m:02}:{s:02}Z")
        })
}

/// List project file structure with optional metadata as NDJSON.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if traversing the directory fails.
#[tracing::instrument(skip_all, fields(command = "list"))]
pub fn cmd_list(
    args: &ListArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    // Multi-root: clap accepts `paths: Vec`; previously only paths[0] was walked.
    let roots: Vec<PathBuf> = if args.paths.is_empty() {
        vec![workspace.clone()]
    } else {
        args.paths
            .iter()
            .map(|p| crate::path_safety::validate_path(p, &workspace))
            .collect::<Result<Vec<_>>>()?
    };

    // GAP-110: return FILE_NOT_FOUND when any root does not exist
    for root in &roots {
        if !root.exists() {
            return Err(crate::error::AtomwriteError::NotFound { path: root.clone() }.into());
        }
    }

    // `list` names hidden files with its own `--all`, so the shared walker is
    // built from the `--all` policy rather than from `--hidden`.
    let mut builder = crate::commands::build_scoped_walker_flavored(
        &roots,
        global,
        if args.all {
            crate::commands::WalkerFlavor::AlwaysVisitHidden
        } else {
            crate::commands::WalkerFlavor::RespectGlobalFlagsByType
        },
        &args.include,
        &args.exclude,
    )?;

    if let Some(depth) = args.depth {
        builder.max_depth(Some(depth));
    }

    let want_long = args.long;
    let want_ext = args.count_by_ext;
    let ws = workspace;
    let roots_cap = roots;

    // A directory the walk cannot open is a hole in a listing that claims to
    // show the whole tree, so the tally travels into the walker with the map.
    let failures: std::sync::Arc<std::sync::Mutex<crate::ndjson_types::FailureTally>> =
        std::sync::Arc::new(std::sync::Mutex::new(
            crate::ndjson_types::FailureTally::default(),
        ));
    let failures_in_walk = std::sync::Arc::clone(&failures);
    // An ignore file the walk half-reads is a hole too, and `ignore` never
    // reports it by itself.
    crate::concurrency::record_ignore_file_failures(&roots_cap, Some(&failures));
    // The map is `move` (it must outlive the call), so it takes its own handle
    // and the one after the walk stays aliased to the same tally.
    let failures_in_map = std::sync::Arc::clone(&failures);
    let mut records: Vec<ListRecord> = collect_mapped_parallel_tallied(
        &builder,
        move |entry| {
            let path = entry.path();
            // The roots are where the walk starts, not entries of the listing.
            // Emitting them spent a `--limit` slot on a name the caller
            // already typed, and pushed the first real entry out of the page.
            if roots_cap.iter().any(|root| root.as_path() == path) {
                return None;
            }
            let rel_path = rel_path_for(path, &ws, &roots_cap);
            if rel_path.is_empty() {
                return None;
            }

            let ft = entry.file_type();
            let kind: &'static str = if ft.is_some_and(|t| t.is_dir()) {
                "dir"
            } else if ft.is_some_and(|t| t.is_symlink()) {
                "symlink"
            } else {
                "file"
            };

            let (size, modified, bytes) = if want_long {
                match entry.metadata() {
                    Ok(meta) => {
                        let sz = meta.len();
                        (Some(sz), format_mtime(&meta), sz)
                    }
                    Err(e) => {
                        if let Ok(mut tally) = failures_in_map.lock() {
                            tally.record(path, crate::concurrency::walk_failure(&e, path));
                        }
                        (None, None, 0)
                    }
                }
            } else {
                let bytes = match entry.metadata() {
                    Ok(m) => m.len(),
                    Err(e) => {
                        if let Ok(mut tally) = failures_in_map.lock() {
                            tally.record(path, crate::concurrency::walk_failure(&e, path));
                        }
                        0
                    }
                };
                (None, None, bytes)
            };

            let ext = if want_ext {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let e = if is_backup_timestamp_name(file_name) {
                    "backup".to_owned()
                } else {
                    path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("(none)")
                        .to_owned()
                };
                Some(e)
            } else {
                None
            };

            Some(ListRecord {
                path: rel_path,
                kind,
                size,
                modified,
                bytes,
                ext,
            })
        },
        Some(failures_in_walk),
    );

    // Deterministic NDJSON order after parallel discovery.
    sort_by_parallel(&mut records, |a, b| a.path.cmp(&b.path));

    // `--limit` is a work budget: the tail of the listing is cut before a single
    // entry is written, and the summary reports that the walk stopped short.
    crate::output::cap_collected(&mut records);

    // Same contract as `search`: on cooperative cancel, skip summary.
    if crate::signal::is_global_shutdown() {
        return Ok(());
    }

    let text_mode = args.format == crate::cli::SearchFormat::Text;
    if text_mode {
        writer.set_text_verb("list");
    }
    let records_total = records.len() as u64;
    let mut text_written: u64 = 0;
    let mut files: u64 = 0;
    let mut dirs: u64 = 0;
    let mut symlinks: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut by_ext: BTreeMap<String, u64> = BTreeMap::new();

    for rec in records {
        if crate::signal::is_global_shutdown() {
            return Ok(());
        }
        match rec.kind {
            "dir" => dirs += 1,
            "symlink" => symlinks += 1,
            _ => files += 1,
        }
        // A directory has no bytes of its own. Counting it summed the whole
        // `--long` size column into a number the caller reads as file weight.
        if rec.kind == "file" {
            total_bytes += rec.bytes;
        }
        if let Some(ext) = rec.ext {
            *by_ext.entry(ext).or_default() += 1;
        }
        if text_mode {
            if crate::output::text_capped() {
                continue;
            }
            writer.write_text_bytes(format!("{}\n", rec.path).as_bytes())?;
            if !crate::output::text_capped() {
                text_written = text_written.saturating_add(1);
            }
            continue;
        }
        writer.write_event(&ListEntry {
            r#type: "entry",
            path: rec.path,
            kind: rec.kind.into(),
            size: rec.size,
            modified: rec.modified,
        })?;
    }

    if crate::signal::is_global_shutdown() {
        return Ok(());
    }

    let (failed_count, failed_items, first_failure) = match std::sync::Arc::try_unwrap(failures) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default().into_parts(),
        Err(arc) => {
            let taken = arc
                .lock()
                .map(|mut g| std::mem::take(&mut *g))
                .unwrap_or_default();
            taken.into_parts()
        }
    };
    let files_failed = (failed_count > 0).then_some(failed_count);
    let summary = ListSummary {
        r#type: "summary",
        files,
        dirs,
        symlinks,
        total_bytes: Some(total_bytes),
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        by_extension: if args.count_by_ext {
            Some(by_ext)
        } else {
            None
        },
        elapsed_ms: start.elapsed().as_millis() as u64,
        files_failed,
        partial: crate::ndjson_types::partial_of(files_failed),
        errors: (!failed_items.is_empty()).then_some(failed_items),
        walker_stopped_early: crate::output::limit_reached().then_some(true),
    };
    if let Some(err) = first_failure {
        return Err(writer.publish_failure(&summary, err));
    }
    if text_mode {
        // A page that stopped short must say how much of the listing the
        // caller did not receive; the entries are the unit `--limit` pages in.
        if crate::output::text_capped() {
            let omitted = records_total.saturating_sub(text_written);
            crate::output::note_text_omitted("list", omitted);
        }
        return Ok(());
    }
    writer.write_terminal(&summary)?;

    Ok(())
}
