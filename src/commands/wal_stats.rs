// SPDX-License-Identifier: MIT OR Apache-2.0

//! G119 L3 + L5 — `wal-stats` and `wal-heal` subcommands.
//!
//! `wal-stats` is read-only local diagnostics (G119 L5). `wal-heal` is the
//! explicit operator-facing version of the auto-heal pass (G119 L3)
//! that future releases will run on startup.
//!
//! Workload: I/O-bound (WalkParallel discovery + multi-journal parse).
//! Parallelism: inherits `wal::walk_journal_paths` (WalkParallel) and
//! `par_iter` parse/classify/unlink in stats/heal. Bound: process-wide pool.

use anyhow::Result;

use crate::cli::{GlobalArgs, WalHealArgs, WalStatsArgs};
use crate::output::NdjsonWriter;

/// Emit a NDJSON snapshot of the workspace's WAL sidecar state.
///
/// Read-only and safe to call from any context. Used by local gates and
/// agent health checks to detect accumulating junk before it pollutes
/// `git status --porcelain`.
///
/// # Errors
///
/// Returns the error raised when the workspace cannot be resolved, and the walk error from
/// [`crate::wal::compute_wal_stats`] when journal discovery fails.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
#[tracing::instrument(skip_all, fields(command = "wal-stats"))]
pub fn cmd_wal_stats(
    args: &WalStatsArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl std::io::Write>,
) -> Result<()> {
    let workspace = global.resolve_workspace()?;

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "wal-stats".into(),
            path: workspace.display().to_string(),
            would_modify: false,
            details: Some(format!(
                "scan workspace for {}*.json sidecars",
                crate::constants::WAL_JOURNAL_PREFIX
            )),
            // A read-only scan writes no content to assess.
            risk_assessment: None,
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let stats = crate::wal::compute_wal_stats(&workspace)?;
    writer.write_event(&stats)?;
    Ok(())
}

/// Remove stale terminal journals (G119 L3).
///
/// Walks the workspace, removes every `Committed`/`Aborted` sidecar
/// whose last entry is older than `--threshold-secs`, and emits a
/// NDJSON report. `Started` sidecars are NEVER removed (they are
/// potential orphans that need `recover_orphan_journals`).
///
/// # Errors
///
/// Returns the error raised when the workspace cannot be resolved, and
/// `AtomwriteError::WorkspaceJail` when `--path` escapes the workspace.
/// Returns the walk error from [`crate::wal::compute_wal_stats`] on the dry-run path and from
/// [`crate::wal::auto_heal_on_startup`] on the live path when journal discovery fails;
/// unlink failures are best-effort and never surface here.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
#[tracing::instrument(skip_all, fields(command = "wal-heal"))]
pub fn cmd_wal_heal(
    args: &WalHealArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl std::io::Write>,
) -> Result<()> {
    let workspace = global.resolve_workspace()?;
    // ETD: the reaped tree is the target named in argv, never the ambient workspace.
    let target = crate::path_safety::validate_path(&args.path, &workspace)?;

    if args.dry_run {
        let stats = crate::wal::compute_wal_stats(&target)?;
        // B-003: honest would_modify — 0 journals means no mutation planned.
        let would_modify = stats.total_journals > 0;
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "wal-heal".into(),
            path: target.display().to_string(),
            would_modify,
            details: Some(format!(
                "would remove up to {} terminal journals older than {}s (preserving Started orphans)",
                stats.total_journals, args.threshold_secs
            )),
            // Removing a journal writes no content to assess.
            risk_assessment: None,
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let report =
        crate::wal::auto_heal_on_startup(&target, args.threshold_secs, args.max_duration_ms)?;
    writer.write_event(&report)?;
    Ok(())
}
