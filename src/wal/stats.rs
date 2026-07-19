// SPDX-License-Identifier: MIT OR Apache-2.0

//! WAL statistics and journal walk (CPU+I/O; rayon over independent journals).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::Serialize;

use super::journal::JOURNAL_EXT;

/// Snapshot of journal state for `wal-stats` (G119 L5 local diagnostics).
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct WalStats {
    /// Envelope type discriminator for NDJSON consumers.
    #[serde(rename = "type")]
    pub r#type: &'static str,
    /// Total number of sidecar journals found in the workspace.
    pub total_journals: u64,
    /// Breakdown by terminal state.
    pub by_state: WalStateBreakdown,
    /// Age of the oldest journal in seconds (0 if none).
    pub oldest_journal_age_secs: u64,
    /// Total bytes occupied by all sidecar files.
    pub total_size_bytes: u64,
    /// Top directories by journal count (up to 10).
    pub by_directory: Vec<WalDirEntry>,
    /// True if the workspace has accumulated enough journals to warrant
    /// an auto-heal pass.
    pub auto_heal_recommended: bool,
    /// Estimated bytes that would be reclaimed by an auto-heal pass.
    pub estimated_reclaim_bytes: u64,
}

/// Breakdown of journals by terminal state.
#[derive(Debug, Clone, Default, Serialize, schemars::JsonSchema)]
pub struct WalStateBreakdown {
    /// Journals whose last entry is `Started` (potential orphans).
    pub started: u64,
    /// Journals whose last entry is `Committed` (safe to clean).
    pub committed: u64,
    /// Journals whose last entry is `Aborted` (safe to clean).
    pub aborted: u64,
    /// Journals that could not be parsed (do not auto-clean).
    pub malformed: u64,
}

/// Count of journals in a single directory.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct WalDirEntry {
    /// Directory path relative to the workspace root.
    pub path: String,
    /// Number of journals in this directory.
    pub count: u64,
}

/// Compute a snapshot of the current journal state. Read-only and safe
/// to call from any context. Used by the `wal-stats` subcommand (G119 L5).
///
/// Discovery is `WalkParallel`; per-journal parse fans out with `par_iter`
/// when more than one sidecar is present (independent I/O).
pub fn compute_wal_stats(workspace: &Path) -> Result<WalStats> {
    use std::collections::BTreeMap;

    let paths = walk_journal_paths(workspace)?;
    let workspace = workspace.to_path_buf();

    // Independent read+parse per journal → parallel when multi-item.
    let rows: Vec<(PathBuf, u64, &'static str, u64)> =
        if crate::concurrency::should_parallelize(paths.len()) {
            paths
                .par_iter()
                .map(|path| {
                    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                    let (state, last_unix) =
                        parse_journal_state(path).unwrap_or(("malformed", 0));
                    (path.clone(), size, state, last_unix)
                })
                .collect()
        } else {
            paths
                .iter()
                .map(|path| {
                    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                    let (state, last_unix) =
                        parse_journal_state(path).unwrap_or(("malformed", 0));
                    (path.clone(), size, state, last_unix)
                })
                .collect()
        };

    let mut total: u64 = 0;
    let mut by_state = WalStateBreakdown::default();
    let mut oldest_unix: u64 = u64::MAX;
    let mut total_size: u64 = 0;
    let mut by_dir: BTreeMap<String, u64> = BTreeMap::new();

    for (path, size, state, last_unix) in rows {
        total += 1;
        total_size += size;

        let rel_dir = path
            .parent()
            .and_then(|p| p.strip_prefix(&workspace).ok())
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| ".".to_string());
        *by_dir.entry(rel_dir).or_insert(0) += 1;

        match state {
            "Committed" => by_state.committed += 1,
            "Aborted" => by_state.aborted += 1,
            "Started" => by_state.started += 1,
            _ => by_state.malformed += 1,
        }
        if state != "malformed" && last_unix < oldest_unix {
            oldest_unix = last_unix;
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let oldest_age = if oldest_unix == u64::MAX {
        0
    } else {
        now.saturating_sub(oldest_unix)
    };

    let mut by_directory: Vec<WalDirEntry> = by_dir
        .into_iter()
        .map(|(path, count)| WalDirEntry { path, count })
        .collect();
    by_directory.sort_by(|a, b| b.count.cmp(&a.count));
    by_directory.truncate(crate::constants::WAL_STATS_TOP_DIRS);

    // A-013: named thresholds (constants / XDG-aligned defaults)
    let auto_heal_recommended = total > crate::constants::WAL_AUTO_HEAL_COUNT_THRESHOLD
        || oldest_age > crate::constants::WAL_AUTO_HEAL_AGE_SECS;
    let estimated_reclaim_bytes = if auto_heal_recommended { total_size } else { 0 };

    Ok(WalStats {
        r#type: "wal_stats",
        total_journals: total,
        by_state,
        oldest_journal_age_secs: oldest_age,
        total_size_bytes: total_size,
        by_directory,
        auto_heal_recommended,
        estimated_reclaim_bytes,
    })
}

/// Recursively walk a directory and yield all `*.atomwrite.journal.json`
/// sidecar paths. Returns an empty Vec if the workspace does not exist.
///
/// Uses `WalkParallel` (hidden sidecars included) so discovery scales on
/// monorepos; paths are sorted for stable stats/heal order.
pub fn walk_journal_paths(workspace: &Path) -> Result<Vec<PathBuf>> {
    if !workspace.exists() {
        return Ok(Vec::new());
    }
    let mut builder = WalkBuilder::new(workspace);
    // Journals are hidden-name sidecars (`.atomwrite.journal.*`).
    builder.hidden(false).git_ignore(false);
    // Bound by process-wide pool (configured in run via --threads).
    crate::concurrency::apply_walk_threads(&mut builder, None);
    let mut out = crate::concurrency::collect_mapped_parallel(&builder, |entry| {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            return None;
        }
        let name = entry.file_name().to_str()?;
        if name.ends_with(JOURNAL_EXT) {
            Some(entry.path().to_path_buf())
        } else {
            None
        }
    });
    crate::concurrency::sort_paths_parallel(&mut out);
    Ok(out)
}

/// Parse a sidecar and return `(terminal_state, last_unix)` for the last
/// entry. Used by `wal-stats` to classify journals without paying the
/// full `OrphanJournalReport` cost.
pub(crate) fn parse_journal_state(path: &Path) -> Option<(&'static str, u64)> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut state = "malformed";
    let mut last_unix: u64 = 0;
    for line in content.lines() {
        let val: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                state = "malformed";
                continue;
            }
        };
        let phase = val.get("phase").and_then(|v| v.as_str()).unwrap_or("");
        match phase {
            "started" => {
                state = "Started";
                last_unix = val
                    .get("started_at_unix")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }
            "committed" => {
                state = "Committed";
                last_unix = val
                    .get("committed_at_unix")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }
            "aborted" => {
                state = "Aborted";
                last_unix = val
                    .get("aborted_at_unix")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }
            _ => {
                state = "malformed";
            }
        }
    }
    Some((state, last_unix))
}
