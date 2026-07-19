// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parallel text replacement across files with atomic writes.
//! Workload: I/O-bound (file reading + regex matching + atomic write) with
//! optional per-file CPU fuzzy cascade (single-buffer).
//! Parallelism: `ignore::WalkParallel` + bounded channel; worker bound via
//! `concurrency::apply_walk_threads` (`--threads` / `--max-concurrency`).
//! Progress precount (when enabled) also uses WalkParallel; skipped when
//! `--no-progress` / quiet / `progress_every=0` to avoid a double-walk.
//!
//! v0.1.33 one-shot: fuzzy multi uses [`crate::fuzzy::apply_fuzzy_one_pass`]
//! (never re-scans inserted replacement; default 1 apply; embeds force 1).

use std::borrow::Cow;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use regex::Regex;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;

use crate::cli::{GlobalArgs, ReplaceArgs};
use crate::commands::resolve_backup;
use crate::fuzzy;
use crate::ndjson_types::{DryRunPlan, ProgressEvent, ReplaceResult, Summary};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Replace text across files in parallel with atomic writes.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if reading or writing files fails.
/// Returns `AtomwriteError::NoMatches` if no replacements are found.
#[tracing::instrument(skip_all, fields(command = "replace"))]
pub fn cmd_replace(
    args: &ReplaceArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<()> {
    run_replace(args, global, writer, shutdown, defaults, fuzzy_cfg)
}

include!("replace_run.inc.rs");
include!("replace_helpers.inc.rs");

