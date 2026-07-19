// SPDX-License-Identifier: MIT OR Apache-2.0

//! Budgeted sparse list/read/outline for monorepo agents (v0.1.29 P1-2).
//!
//! Workload: I/O-bound (walk / read under hard budgets).
//! Parallelism: `sparse list` discovery uses budgeted `WalkParallel`
//! (`collect_mapped_parallel_budgeted` + `--threads` / `--max-concurrency`),
//! then path-sorted emit with clamp on `max_files` and path-string `max_bytes`.
//! `sparse read` fans out file reads with `rayon` when multiple paths remain
//! under `max_files`. `sparse outline` stage-1 discovery uses the same budgeted
//! walk; stage-2 read+AST uses `par_iter`, then ordered emit.
//! Bound: process-wide rayon pool + `WalkBuilder::threads`.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand, ValueHint};
use rayon::prelude::*;
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::GlobalArgs;
use crate::concurrency::{
    apply_walk_threads, collect_mapped_parallel_budgeted, should_parallelize, sort_paths_parallel,
};
use crate::output::NdjsonWriter;
use crate::path_safety::validate_path;
use crate::signal::ShutdownSignal;

/// Arguments for `sparse`.
#[derive(Args, Debug)]
pub struct SparseArgs {
    /// Sparse subcommand (`list` or `read`).
    #[command(subcommand)]
    pub action: SparseAction,
}

/// Sparse subcommands.
#[derive(Subcommand, Debug)]
pub enum SparseAction {
    /// List paths with a hard file budget.
    List(SparseListArgs),
    /// Read files listed in a paths file with head budget.
    Read(SparseReadArgs),
    /// Outline structure for files under a hard file budget (v0.1.30).
    Outline(SparseOutlineArgs),
}

/// `sparse list` arguments.
#[derive(Args, Debug)]
pub struct SparseListArgs {
    /// Root path to walk.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub path: PathBuf,
    /// Maximum files to emit.
    #[arg(long, default_value_t = 100)]
    pub max_files: u64,
    /// Maximum total bytes of path strings to emit.
    #[arg(long, default_value_t = 1_048_576)]
    pub max_bytes: u64,
    /// Include glob.
    #[arg(long, action = clap::ArgAction::Append)]
    pub include: Vec<String>,
    /// Exclude glob.
    #[arg(long, action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,
}

/// `sparse read` arguments.
#[derive(Args, Debug)]
pub struct SparseReadArgs {
    /// File containing paths (one per line).
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub paths_file: PathBuf,
    /// Head lines per file.
    #[arg(long, default_value_t = 50)]
    pub head: u64,
    /// Maximum files to read.
    #[arg(long, default_value_t = 20)]
    pub max_files: u64,
}

/// `sparse outline` arguments (v0.1.30).
#[derive(Args, Debug)]
pub struct SparseOutlineArgs {
    /// Root path to walk.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub path: PathBuf,
    /// Maximum files to outline.
    #[arg(long, default_value_t = 50)]
    pub max_files: u64,
    /// Include glob.
    #[arg(long, action = clap::ArgAction::Append)]
    pub include: Vec<String>,
    /// Exclude glob.
    #[arg(long, action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,
}

#[derive(Serialize, JsonSchema)]
struct SparseEntry {
    r#type: &'static str,
    path: String,
    size: u64,
}

#[derive(Serialize, JsonSchema)]
struct SparseSummary {
    r#type: &'static str,
    emitted: u64,
    truncated: bool,
    elapsed_ms: u64,
}

/// Dispatch sparse actions.
#[tracing::instrument(skip_all, fields(command = "sparse"))]
pub fn cmd_sparse(
    args: &SparseArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    _defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    match &args.action {
        SparseAction::List(a) => sparse_list(a, global, writer, shutdown),
        SparseAction::Read(a) => sparse_read(a, global, writer, shutdown),
        SparseAction::Outline(a) => sparse_outline(a, global, writer, shutdown),
    }
}

fn sparse_list(
    args: &SparseListArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = std::time::Instant::now();
    let workspace = global.resolve_workspace()?;
    let root = validate_path(&args.path, &workspace)?;
    let mut builder = ignore::WalkBuilder::new(&root);
    builder.hidden(false);
    builder.git_ignore(true);
    for g in &args.exclude {
        builder.add_ignore(g);
    }
    // Bound discovery to CLI --threads (no-op without build_parallel).
    apply_walk_threads(&mut builder, global.threads);

    // Budgeted WalkParallel: workers may overshoot slightly; clamp after sort.
    let (mut entries, hit_budget) =
        collect_mapped_parallel_budgeted(&builder, args.max_files, move |entry| {
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                return None;
            }
            let path = entry.path().display().to_string();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            Some((path, size))
        });

    if shutdown.is_shutdown() {
        return Ok(());
    }

    // Deterministic NDJSON: path order (parallel walk order is not stable).
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut truncated = hit_budget || entries.len() as u64 > args.max_files;
    if entries.len() as u64 > args.max_files {
        entries.truncate(args.max_files as usize);
    }

    let mut emitted = 0u64;
    let mut bytes = 0u64;
    for (path, size) in entries {
        if shutdown.is_shutdown() {
            break;
        }
        let path_cost = path.len() as u64;
        if bytes.saturating_add(path_cost) > args.max_bytes {
            truncated = true;
            break;
        }
        bytes += path_cost;
        writer.write_event(&SparseEntry {
            r#type: "sparse_entry",
            path,
            size,
        })?;
        emitted += 1;
    }
    writer.write_event(&SparseSummary {
        r#type: "sparse_summary",
        emitted,
        truncated,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;
    Ok(())
}

fn sparse_read(
    args: &SparseReadArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = std::time::Instant::now();
    let workspace = global.resolve_workspace()?;
    let list_path = validate_path(&args.paths_file, &workspace)?;
    let list = crate::file_io::read_file_string(&list_path, global.effective_max_filesize())?;
    let max_size = global.effective_max_filesize();
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut truncated = false;
    for line in list.lines() {
        if shutdown.is_shutdown() {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if paths.len() as u64 >= args.max_files {
            truncated = true;
            break;
        }
        match validate_path(std::path::Path::new(line), &workspace) {
            Ok(p) => paths.push(p),
            Err(_) => continue,
        }
    }

    let head_n = args.head as usize;
    let heads: Vec<Result<(String, String), anyhow::Error>> = if should_parallelize(paths.len()) {
        paths
            .par_iter()
            .map(|path| {
                if shutdown.is_shutdown() {
                    return Err(crate::signal::cancelled_error("sparse read cancelled").into());
                }
                let content = crate::file_io::read_file_string(path, max_size)?;
                let head: String = content.lines().take(head_n).collect::<Vec<_>>().join("\n");
                Ok((path.display().to_string(), head))
            })
            .collect()
    } else {
        paths
            .iter()
            .map(|path| {
                let content = crate::file_io::read_file_string(path, max_size)?;
                let head: String = content.lines().take(head_n).collect::<Vec<_>>().join("\n");
                Ok((path.display().to_string(), head))
            })
            .collect()
    };

    let mut emitted = 0u64;
    for item in heads {
        let (path, head) = item?;
        writer.write_event(&crate::ndjson_types::SparseReadEvent {
            r#type: "sparse_read",
            path,
            head,
            lines: args.head,
        })?;
        emitted += 1;
    }
    writer.write_event(&SparseSummary {
        r#type: "sparse_summary",
        emitted,
        truncated,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;
    Ok(())
}

fn sparse_outline(
    args: &SparseOutlineArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    use std::time::Instant;
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let root = validate_path(&args.path, &workspace)?;
    let mut walker = ignore::WalkBuilder::new(&root);
    walker.standard_filters(true);
    for g in &args.exclude {
        walker.add_ignore(g);
    }
    // Bound discovery to CLI --threads (no-op without build_parallel).
    apply_walk_threads(&mut walker, global.threads);

    // Stage 1 — budgeted parallel path collect (WalkParallel + Atomic cap).
    // Workers may overshoot slightly; clamp after sort for a hard max_files.
    let include = args.include.clone();
    let _ = shutdown; // stage-2 still checks; stage-1 uses process-wide signal
    let (mut paths, hit_budget) =
        collect_mapped_parallel_budgeted(&walker, args.max_files, move |entry| {
            if crate::signal::is_global_shutdown() {
                return None;
            }
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                return None;
            }
            let path = entry.path().to_path_buf();
            if !include.is_empty() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let ok = include.iter().any(|pat| {
                    if let Some(suf) = pat.strip_prefix("*.") {
                        name.ends_with(suf)
                            || path.extension().and_then(|e| e.to_str()) == Some(suf)
                    } else {
                        name.contains(pat) || path.to_string_lossy().contains(pat)
                    }
                });
                if !ok {
                    return None;
                }
            }
            Some(path)
        });
    sort_paths_parallel(&mut paths);
    let truncated = hit_budget || paths.len() as u64 > args.max_files;
    if paths.len() as u64 > args.max_files {
        paths.truncate(args.max_files as usize);
    }

    let files_seen = paths.len() as u64;
    let max = global.effective_max_filesize();

    // Stage 2 — independent read+AST (rayon-safe via collect_outline_for_path).
    let outlined: Vec<(PathBuf, Vec<crate::commands::outline::OutlineItem>)> =
        if should_parallelize(paths.len()) {
            paths
                .par_iter()
                .filter_map(|path| {
                    if shutdown.is_shutdown() {
                        return None;
                    }
                    let content = crate::file_io::read_file_bytes(path, max).ok()?;
                    let items =
                        crate::commands::outline::collect_outline_for_path(path, &content, None);
                    Some((path.clone(), items))
                })
                .collect()
        } else {
            paths
                .iter()
                .filter_map(|path| {
                    if shutdown.is_shutdown() {
                        return None;
                    }
                    let content = crate::file_io::read_file_bytes(path, max).ok()?;
                    let items =
                        crate::commands::outline::collect_outline_for_path(path, &content, None);
                    Some((path.clone(), items))
                })
                .collect()
        };

    // Stage 3 — emit in path order for stable NDJSON.
    let mut outlined = outlined;
    outlined.sort_by(|a, b| a.0.cmp(&b.0));
    let mut items_emitted = 0u64;
    for (_, items) in outlined {
        items_emitted += items.len() as u64;
        for item in items {
            writer.write_event(&item)?;
        }
    }

    writer.write_event(&SparseSummary {
        r#type: "sparse_summary",
        emitted: items_emitted.max(files_seen),
        truncated,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;
    writer.write_event(&crate::ndjson_types::SparseOutlineBudget {
        r#type: "sparse_outline_budget",
        files_seen,
        items: items_emitted,
        truncated,
        max_files: args.max_files,
    })?;
    Ok(())
}
