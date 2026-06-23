// SPDX-License-Identifier: MIT OR Apache-2.0

//! G119 L3 + L5 — `wal-stats` and `wal-heal` subcommands.
//!
//! `wal-stats` is read-only telemetry (G119 L5). `wal-heal` is the
//! explicit operator-facing version of the auto-heal pass (G119 L3)
//! that future releases will run on startup.

use anyhow::Result;

use crate::cli::{GlobalArgs, WalHealArgs, WalStatsArgs};
use crate::output::NdjsonWriter;

/// Emit a NDJSON snapshot of the workspace's WAL sidecar state.
///
/// Read-only and safe to call from any context. Used by CI gates and
/// agent health checks to detect accumulating junk before it pollutes
/// `git status --porcelain`.
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
            details: Some("scan workspace for .atomwrite.journal.*.json sidecars".into()),
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
pub fn cmd_wal_heal(
    args: &WalHealArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl std::io::Write>,
) -> Result<()> {
    let workspace = global.resolve_workspace()?;

    if args.dry_run {
        let stats = crate::wal::compute_wal_stats(&workspace)?;
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "wal-heal".into(),
            path: workspace.display().to_string(),
            would_modify: true,
            details: Some(format!(
                "would remove up to {} terminal journals older than {}s (preserving Started orphans)",
                stats.total_journals, args.threshold_secs
            )),
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let report =
        crate::wal::auto_heal_on_startup(&workspace, args.threshold_secs, args.max_duration_ms)?;
    writer.write_event(&report)?;
    Ok(())
}
