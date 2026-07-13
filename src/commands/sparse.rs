// SPDX-License-Identifier: MIT OR Apache-2.0

//! Budgeted sparse list/read/outline for monorepo agents (v0.1.29 P1-2).

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::GlobalArgs;
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
    #[arg(default_value = ".")]
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
    #[arg(long)]
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
    #[arg(default_value = ".")]
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
    let mut emitted = 0u64;
    let mut bytes = 0u64;
    let mut truncated = false;
    for entry in builder.build() {
        if shutdown.is_shutdown() {
            break;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if emitted >= args.max_files {
            truncated = true;
            break;
        }
        let path = entry.path().display().to_string();
        bytes += path.len() as u64;
        if bytes > args.max_bytes {
            truncated = true;
            break;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
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
    let list = std::fs::read_to_string(&list_path)?;
    let mut emitted = 0u64;
    let mut truncated = false;
    for line in list.lines() {
        if shutdown.is_shutdown() {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if emitted >= args.max_files {
            truncated = true;
            break;
        }
        let path = validate_path(std::path::Path::new(line), &workspace)?;
        let content = crate::file_io::read_file_string(&path, global.effective_max_filesize())?;
        let head: String = content
            .lines()
            .take(args.head as usize)
            .collect::<Vec<_>>()
            .join("\n");
        writer.write_event(&serde_json::json!({
            "type": "sparse_read",
            "path": path.display().to_string(),
            "head": head,
            "lines": args.head,
        }))?;
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
    let mut files_seen = 0u64;
    let mut items_emitted = 0u64;
    let mut truncated = false;
    for entry in walker.build() {
        if shutdown.is_shutdown() {
            break;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        if files_seen >= args.max_files {
            truncated = true;
            break;
        }
        let path = entry.path();
        if !args.include.is_empty() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let ok = args.include.iter().any(|pat| {
                if let Some(suf) = pat.strip_prefix("*.") {
                    name.ends_with(suf) || path.extension().and_then(|e| e.to_str()) == Some(suf)
                } else {
                    name.contains(pat) || path.to_string_lossy().contains(pat)
                }
            });
            if !ok {
                continue;
            }
        }
        files_seen += 1;
        let max = global.effective_max_filesize();
        let content = match crate::file_io::read_file_bytes(path, max) {
            Ok(c) => c,
            Err(_) => match std::fs::read(path) {
                Ok(c) if (c.len() as u64) <= max => c,
                _ => continue,
            },
        };
        match crate::commands::outline::emit_outline_for_path(path, &content, None, writer) {
            Ok(n) => items_emitted += n as u64,
            Err(_) => continue,
        }
    }
    writer.write_event(&SparseSummary {
        r#type: "sparse_summary",
        emitted: items_emitted.max(files_seen),
        truncated,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;
    // Also emit a budget line for agents.
    writer.write_event(&serde_json::json!({
        "type": "sparse_outline_budget",
        "files_seen": files_seen,
        "items": items_emitted,
        "truncated": truncated,
        "max_files": args.max_files,
    }))?;
    Ok(())
}
