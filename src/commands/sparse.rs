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
use crate::concurrency::{collect_mapped_parallel_budgeted_tallied, should_parallelize};
use crate::output::NdjsonWriter;
use crate::path_safety::validate_path;
use crate::signal::ShutdownSignal;

/// Arguments for `sparse`.
#[derive(Args, Debug)]
pub struct SparseArgs {
    /// Sparse subcommand (`list`, `read` or `outline`).
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
    /// Outline structure for files under a hard file budget.
    Outline(SparseOutlineArgs),
}

/// `sparse list` arguments.
#[derive(Args, Debug)]
pub struct SparseListArgs {
    /// Root path to walk.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub path: PathBuf,
    /// Maximum files to emit.
    #[arg(long, default_value_t = crate::constants::DEFAULT_SPARSE_MAX_ENTRIES as u64)]
    pub max_files: u64,
    /// Maximum total bytes of path strings to emit.
    #[arg(long, default_value_t = crate::constants::DEFAULT_SPARSE_MAX_BYTES as u64)]
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
    #[arg(long, default_value_t = crate::constants::DEFAULT_SPARSE_OUTLINE_DEPTH as u64)]
    pub head: u64,
    /// Maximum files to read.
    #[arg(long, default_value_t = crate::constants::DEFAULT_SPARSE_MAX_SYMBOLS as u64)]
    pub max_files: u64,
}

/// `sparse outline` arguments.
#[derive(Args, Debug)]
pub struct SparseOutlineArgs {
    /// Root path to walk.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub path: PathBuf,
    /// Maximum files to outline.
    #[arg(long, default_value_t = crate::constants::DEFAULT_SPARSE_OUTLINE_DEPTH as u64)]
    pub max_files: u64,
    /// Include glob.
    #[arg(long, action = clap::ArgAction::Append)]
    pub include: Vec<String>,
    /// Exclude glob.
    #[arg(long, action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,
}

/// What `sparse list` measured before the page was written.
pub(crate) struct SparseScan {
    /// Files the walk delivered.
    pub files_scanned: u64,
    /// Path bytes actually emitted, when the verb measures them.
    pub path_bytes: Option<u64>,
    /// Entries discovery delivered that the page did not.
    pub dropped: u64,
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct SparseEntry {
    r#type: &'static str,
    path: String,
    size: u64,
}

/// Terminal event of `sparse`: entries emitted under the budget.
#[derive(Serialize, JsonSchema)]
pub(crate) struct SparseSummary {
    r#type: &'static str,
    emitted: u64,
    truncated: bool,
    elapsed_ms: u64,
    /// Files the walk saw but could not read; present only when some failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    files_failed: Option<u64>,
    /// First failures, capped; `files_failed` is the full count.
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<crate::ndjson_types::FileFailure>>,
    /// The run answered from part of its targets, so the answer is partial.
    ///
    /// Present exactly when `files_failed` is present, so a consumer reads one
    /// flag instead of comparing two counters.
    #[serde(skip_serializing_if = "Option::is_none")]
    partial: Option<bool>,
    /// Present and true when the discovery stopped before the tree ended.
    ///
    /// It reads `truncated`, which is the measured fact of this verb: the
    /// collection hit `--max-files` or the writer's budget and stopped. The
    /// field used to read `output::limit_reached()`, so a run truncated by
    /// `--max-files` alone published nothing while the walk had stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    walker_stopped_early: Option<bool>,
    /// Files the walk delivered before the page cut.
    ///
    /// The discovery budget stops the walk at `--max-files`, so this is what
    /// the run measured, and never the size of the whole tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    files_scanned: Option<u64>,
    /// Total bytes of the path strings the run emitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    path_bytes: Option<u64>,
    /// Entries discovery delivered that the page did not.
    ///
    /// It counts the cut by `--max-files`, by the `--limit` work budget and by
    /// `--max-bytes`, so the difference between `files_scanned` and `emitted`
    /// has a name.
    #[serde(skip_serializing_if = "Option::is_none")]
    files_budget_dropped: Option<u64>,
    /// Files the outline could not parse.
    ///
    /// Zero items on such a file is a hole and not an empty file, and the
    /// outline reader uses this to tell the two apart.
    #[serde(skip_serializing_if = "Option::is_none")]
    files_unparsed: Option<u64>,
    /// Outline items whose signature was cut by the ceiling or the first line.
    #[serde(skip_serializing_if = "Option::is_none")]
    items_truncated: Option<u64>,
}

/// Write the terminal summary, failing the run when a file could not be read.
///
/// `sparse` answers with what it saw, so an unreadable file is a hole in the
/// answer and not something to skip in silence: the count and the first
/// failures travel in the summary, and the record reports `ok: false`.
fn emit_sparse_summary(
    writer: &mut NdjsonWriter<impl Write>,
    emitted: u64,
    truncated: bool,
    start: std::time::Instant,
    tally: crate::ndjson_types::FailureTally,
    scan: SparseScan,
) -> Result<()> {
    let (count, items, first) = tally.into_parts();
    let files_failed = (count > 0).then_some(count);
    let summary = SparseSummary {
        r#type: "sparse_summary",
        emitted,
        truncated,
        elapsed_ms: start.elapsed().as_millis() as u64,
        files_failed,
        partial: crate::ndjson_types::partial_of(files_failed),
        errors: (!items.is_empty()).then_some(items),
        walker_stopped_early: truncated.then_some(true),
        files_scanned: Some(scan.files_scanned),
        path_bytes: scan.path_bytes,
        files_budget_dropped: (scan.dropped > 0).then_some(scan.dropped),
        files_unparsed: None,
        items_truncated: None,
    };
    // The entries the budget allowed were published, so the run answered: a
    // directory it could not open is a hole the summary reports through
    // `partial` and `errors`, not a reason to discard the whole page.
    if let Some(err) = first
        && emitted == 0
    {
        return Err(writer.publish_failure(&summary, err));
    }
    writer.write_terminal(&summary)?;
    Ok(())
}

/// Dispatch sparse actions.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` when a root or the `--paths-file` escapes the
/// workspace, and `AtomwriteError::SymlinkBlocked`, `AtomwriteError::FifoDetected` or
/// `AtomwriteError::DeviceFile` when a path is a link, a FIFO or a device.
/// Returns `AtomwriteError::InvalidInput` when an include or exclude glob does not compile.
/// Returns `AtomwriteError::NotFound` when the paths file does not exist, and
/// `AtomwriteError::FileTooLarge` when it exceeds the effective max filesize.
/// Returns `AtomwriteError::Cancelled` when shutdown interrupts the parallel read.
/// Returns the first read or walk failure, published as an `ok: false` summary, when a file under
/// a root could not be read.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
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
    // The one shared walker, in the always-hidden flavor: a sparse listing that
    // skipped `.env` would not describe the directory.
    let builder = crate::commands::build_scoped_walker_flavored(
        std::slice::from_ref(&root),
        global,
        crate::commands::WalkerFlavor::AlwaysVisitHidden,
        &args.include,
        &args.exclude,
    )?;

    // Budgeted WalkParallel: workers may overshoot slightly; clamp after sort.
    let failures = std::sync::Arc::new(std::sync::Mutex::new(
        crate::ndjson_types::FailureTally::default(),
    ));
    let failures_in_map = std::sync::Arc::clone(&failures);
    // An ignore file the walk half-reads is a hole too, and `ignore` never
    // reports it by itself.
    crate::concurrency::record_ignore_file_failures(std::slice::from_ref(&root), Some(&failures));
    let (mut entries, hit_budget) = collect_mapped_parallel_budgeted_tallied(
        &builder,
        args.max_files,
        move |entry| {
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                return None;
            }
            let path = entry.path().display().to_string();
            let size = match entry.metadata() {
                Ok(meta) => meta.len(),
                Err(e) => {
                    if let Ok(mut tally) = failures_in_map.lock() {
                        tally.record(
                            entry.path(),
                            crate::concurrency::walk_failure(&e, entry.path()),
                        );
                    }
                    0
                }
            };
            Some((path, size))
        },
        Some(std::sync::Arc::clone(&failures)),
    );

    if shutdown.is_shutdown() {
        return Ok(());
    }

    // Deterministic NDJSON: path order (parallel walk order is not stable).
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    // What discovery delivered, before any page cut: with `--max-files` the
    // walk stops at the budget, so this names the measured side and never the
    // size of the whole tree.
    let files_scanned = entries.len() as u64;
    let mut dropped = 0u64;
    let mut truncated = hit_budget || entries.len() as u64 > args.max_files;
    if entries.len() as u64 > args.max_files {
        dropped += entries.len() as u64 - args.max_files;
        entries.truncate(args.max_files as usize);
    }
    // `--limit` bounds the work on top of `--max-files`, which bounds discovery.
    if crate::output::cap_collected(&mut entries) {
        dropped += crate::output::budget_dropped();
        truncated = true;
    }

    let total_entries = entries.len();
    let mut bytes = 0u64;
    for (idx, (path, size)) in entries.into_iter().enumerate() {
        if shutdown.is_shutdown() {
            dropped += (total_entries - idx) as u64;
            break;
        }
        let path_cost = path.len() as u64;
        if bytes.saturating_add(path_cost) > args.max_bytes {
            truncated = true;
            dropped += (total_entries - idx) as u64;
            break;
        }
        bytes += path_cost;
        writer.write_event(&SparseEntry {
            r#type: "sparse_entry",
            path,
            size,
        })?;
    }
    // Count what the writer actually accepted: a record dropped by `--limit` or
    // `--filter` never reached the consumer, so it is not "emitted".
    let emitted = writer.emitted_count();
    let tally = match std::sync::Arc::try_unwrap(failures) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => std::mem::take(&mut *arc.lock().unwrap_or_else(|e| e.into_inner())),
    };
    emit_sparse_summary(
        writer,
        emitted,
        truncated,
        start,
        tally,
        SparseScan {
            files_scanned,
            path_bytes: Some(bytes),
            dropped,
        },
    )
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
    let mut tally = crate::ndjson_types::FailureTally::default();
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
            Err(e) => tally.record(std::path::Path::new(line), e),
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

    for item in heads {
        let (path, head) = match item {
            Ok(pair) => pair,
            Err(e) => {
                // The read error may be context-wrapped, so look through the
                // chain for the typed error that names the file.
                let failed = e
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<crate::error::AtomwriteError>())
                    .and_then(crate::error::AtomwriteError::path)
                    .cloned()
                    .unwrap_or_default();
                tally.record(&failed, e);
                continue;
            }
        };
        writer.write_event(&crate::ndjson_types::SparseReadEvent {
            r#type: "sparse_read",
            path,
            head,
            lines: args.head,
        })?;
    }
    let emitted = writer.emitted_count();
    emit_sparse_summary(
        writer,
        emitted,
        truncated,
        start,
        tally,
        SparseScan {
            files_scanned: paths.len() as u64,
            path_bytes: None,
            dropped: 0,
        },
    )
}

include!("sparse_outline.inc.rs");

/// Stub when feature `ast` is disabled: fails closed instead of faking success.
#[cfg(not(feature = "ast"))]
fn sparse_outline(
    _args: &SparseOutlineArgs,
    _global: &GlobalArgs,
    _writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
) -> Result<()> {
    Err(crate::error::AtomwriteError::ConfigInvalid {
        reason: rust_i18n::t!("feature.ast-required", command = "sparse outline").to_string(),
    }
    .into())
}
