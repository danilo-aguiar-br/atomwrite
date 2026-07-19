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

use anyhow::{Context, Result};
use ignore::WalkBuilder;

use crate::cli::{GlobalArgs, ListArgs};
use crate::commands::BACKUP_FILENAME_RE;
use crate::concurrency::{apply_walk_threads, collect_mapped_parallel, sort_by_parallel};
use crate::ndjson_types::{ListEntry, ListSummary};
use crate::output::NdjsonWriter;

fn epoch_days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

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
            let secs = d.as_secs();
            let days = secs / 86400;
            let rem = secs % 86400;
            let h = rem / 3600;
            let m = (rem % 3600) / 60;
            let s = rem % 60;
            let (y, mo, da) = epoch_days_to_ymd(days);
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
            return Err(crate::error::AtomwriteError::NotFound {
                path: root.clone(),
            }
            .into());
        }
    }

    // docs.rs: multi-dir → one WalkBuilder + `.add`, not N separate walks.
    let mut builder = WalkBuilder::new(&roots[0]);
    for root in roots.iter().skip(1) {
        builder.add(root);
    }
    builder.hidden(!args.all).git_ignore(!global.no_gitignore);
    // sort_by_file_path only applies to sequential `build()` — parallel walk
    // materialises then sorts (PAR-035/036).
    apply_walk_threads(&mut builder, global.threads);

    if let Some(depth) = args.depth {
        builder.max_depth(Some(depth));
    }

    if !args.include.is_empty() {
        let mut types_builder = ignore::types::TypesBuilder::new();
        for pattern in &args.include {
            types_builder
                .add_def(&format!("custom:{pattern}"))
                .context("invalid include glob")?;
        }
        types_builder.select("custom");
        builder.types(types_builder.build().context("build types")?);
    }

    if !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&roots[0]);
        for pattern in &args.exclude {
            overrides.add(&format!("!{pattern}"))?;
        }
        builder.overrides(overrides.build()?);
    }

    let want_long = args.long;
    let want_ext = args.count_by_ext;
    let ws = workspace;
    let roots_cap = roots.clone();

    let mut records: Vec<ListRecord> = collect_mapped_parallel(&builder, move |entry| {
        let path = entry.path();
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
                Err(_) => (None, None, 0),
            }
        } else {
            let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            (None, None, bytes)
        };

        let ext = if want_ext {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let e = if BACKUP_FILENAME_RE.is_match(file_name) {
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
    });

    // Deterministic NDJSON order after parallel discovery.
    sort_by_parallel(&mut records, |a, b| a.path.cmp(&b.path));

    // Same contract as `search`: on cooperative cancel, skip summary.
    if crate::signal::is_global_shutdown() {
        return Ok(());
    }

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
        total_bytes += rec.bytes;
        if let Some(ext) = rec.ext {
            *by_ext.entry(ext).or_default() += 1;
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

    let summary = ListSummary {
        r#type: "summary",
        files,
        dirs,
        symlinks,
        total_bytes: Some(total_bytes),
        by_extension: if args.count_by_ext {
            Some(by_ext)
        } else {
            None
        },
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    writer.write_event(&summary)?;

    Ok(())
}
