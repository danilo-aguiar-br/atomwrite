// SPDX-License-Identifier: MIT OR Apache-2.0
// Startup WAL maintenance of `run`.
// Included from `lib_dispatch.inc.rs`.

/// Reap stale WAL sidecars before the command runs (G119 L3).
///
/// Best-effort: a failed pass is logged and never blocks the command.
fn startup_wal_heal(command: &Commands, workspace: &std::path::Path, no_auto_heal: bool) {
    // G119 L3 — autonomous startup `wal-heal` pass. Removes every
    // `Committed`/`Aborted` sidecar older than `threshold_secs` (default
    // 3600s = 1h) and is bounded by a 100ms wall-clock budget that the
    // directory walk itself obeys. It runs only for a verb that mutates
    // files, and only in the directories that verb names in argv (never the
    // whole workspace, never `target/` or `.git`). A read-only verb pays
    // nothing. Use `--no-auto-heal` for tight local loops and benchmarks.
    // `Started` journals are NEVER removed automatically — they are the
    // orphans worth operator attention.
    if no_auto_heal {
        return;
    }
    let scopes = startup_heal_scopes(command, workspace);
    if scopes.is_empty() {
        return;
    }
    let report = crate::wal::auto_heal_scoped(
        &scopes,
        crate::constants::DEFAULT_WAL_HEAL_THRESHOLD_SECS,
        crate::constants::DEFAULT_WAL_HEAL_MAX_DURATION_MS,
    );
    if report.removed > 0 || report.malformed > 0 {
        tracing::info!(
            removed = report.removed,
            preserved = report.preserved,
            malformed = report.malformed,
            bytes_reclaimed = report.bytes_reclaimed,
            "G119 L3: startup wal-heal reaped stale sidecars"
        );
    } else {
        tracing::debug!("G119 L3: startup wal-heal found nothing to reap");
    }
}

/// Directories the startup heal may scan for `command`.
///
/// Empty for read-only verbs, for `wal-heal` (it heals explicitly) and when a
/// root cannot be resolved inside the jail. A directory root is scanned whole,
/// a file root only through its parent directory (the journal sits beside it).
fn startup_heal_scopes(
    command: &Commands,
    workspace: &std::path::Path,
) -> Vec<crate::wal::HealScope> {
    use commands::path_resolution::{TargetMode, resolve_mutation_targets};
    if matches!(command, Commands::WalHeal(_)) {
        return Vec::new();
    }
    let Some(roots) = explicit_target_roots(command) else {
        return Vec::new();
    };
    let Ok(resolved) = resolve_mutation_targets(&roots, workspace, TargetMode::Explicit) else {
        return Vec::new();
    };
    let mut scopes: Vec<crate::wal::HealScope> = Vec::new();
    for target in resolved {
        let path = target.target_resolved;
        let scope = if path.is_dir() {
            crate::wal::HealScope {
                root: path,
                max_depth: None,
            }
        } else if let Some(parent) = path.parent() {
            crate::wal::HealScope {
                root: parent.to_path_buf(),
                max_depth: Some(crate::constants::WAL_STARTUP_HEAL_FILE_DEPTH),
            }
        } else {
            continue;
        };
        if !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }
    scopes
}
