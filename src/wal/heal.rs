// SPDX-License-Identifier: MIT OR Apache-2.0

//! WAL auto-heal and orphan journal recovery.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::Serialize;

use super::journal::{JournalEntry, JournalOp, JOURNAL_EXT};
use super::stats::{parse_journal_state, walk_journal_paths};

/// Result of an auto-heal pass on startup (G119 L3).
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct AutoHealReport {
    /// Envelope type discriminator for NDJSON consumers.
    #[serde(rename = "type")]
    pub r#type: &'static str,
    /// Number of stale `Committed`/`Aborted` journals removed.
    pub removed: u64,
    /// Number of journals preserved (`Started` = potential orphans).
    pub preserved: u64,
    /// Number of malformed journals preserved for manual inspection.
    pub malformed: u64,
    /// Total bytes reclaimed by the removal.
    pub bytes_reclaimed: u64,
    /// Age threshold (seconds) used for this pass.
    pub threshold_secs: u64,
}

/// Auto-heal stale terminal journals on startup (G119 L3).
///
/// Walks the workspace, finds every sidecar whose terminal state is
/// `Committed` or `Aborted` AND whose last entry is older than
/// `threshold_secs`, and removes them. Journals in the `Started` state
/// are NEVER removed automatically — they may represent a real orphan
/// that needs `recover_orphan_journals` to inspect.
///
/// This function is bounded: discovery + classify fan out in parallel
/// chunks, with a wall-clock budget of `max_duration_ms` (default 100ms)
/// checked **between** chunks so startup cost stays bounded on 10k-journal
/// workspaces while still using multi-core I/O when time allows.
pub fn auto_heal_on_startup(
    workspace: &Path,
    threshold_secs: u64,
    max_duration_ms: u64,
) -> Result<AutoHealReport> {
    let start = Instant::now();
    let mut removed = 0u64;
    let mut preserved = 0u64;
    let mut malformed = 0u64;
    let mut bytes_reclaimed = 0u64;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let paths = walk_journal_paths(workspace)?;
    // Chunk size ≈ pool size so each batch saturates cores once, then we
    // re-check the wall budget (cooperative bound for startup heal).
    let chunk = crate::concurrency::effective_threads(None).max(1) * 4;

    for chunk_paths in paths.chunks(chunk) {
        if start.elapsed().as_millis() as u64 > max_duration_ms {
            break;
        }
        let actions: Vec<HealClass> = if crate::concurrency::should_parallelize(chunk_paths.len()) {
            chunk_paths
                .par_iter()
                .map(|path| classify_heal(path, now, threshold_secs))
                .collect()
        } else {
            chunk_paths
                .iter()
                .map(|path| classify_heal(path, now, threshold_secs))
                .collect()
        };

        let to_remove: Vec<(PathBuf, u64)> = actions
            .into_iter()
            .filter_map(|a| match a {
                HealClass::Remove { path, size } => Some((path, size)),
                HealClass::Preserve => {
                    preserved += 1;
                    None
                }
                HealClass::Malformed => {
                    malformed += 1;
                    None
                }
            })
            .collect();

        if start.elapsed().as_millis() as u64 > max_duration_ms {
            // Budget exhausted after classify — count remaining as preserved
            // rather than unlinking past the deadline.
            preserved += to_remove.len() as u64;
            break;
        }

        let outcomes: Vec<(bool, u64)> = if crate::concurrency::should_parallelize(to_remove.len())
        {
            to_remove
                .par_iter()
                .map(|(path, size)| match std::fs::remove_file(path) {
                    Ok(()) => (true, *size),
                    Err(_) => (false, 0),
                })
                .collect()
        } else {
            to_remove
                .iter()
                .map(|(path, size)| match std::fs::remove_file(path) {
                    Ok(()) => (true, *size),
                    Err(_) => (false, 0),
                })
                .collect()
        };
        for (ok, size) in outcomes {
            if ok {
                removed += 1;
                bytes_reclaimed += size;
            } else {
                preserved += 1;
            }
        }
    }

    Ok(AutoHealReport {
        r#type: "wal_heal",
        removed,
        preserved,
        malformed,
        bytes_reclaimed,
        threshold_secs,
    })
}

/// Classification of one journal for auto-heal (Send for rayon).
pub(crate) enum HealClass {
    Remove { path: PathBuf, size: u64 },
    Preserve,
    Malformed,
}

pub(crate) fn classify_heal(path: &Path, now: u64, threshold_secs: u64) -> HealClass {
    let (state, last_unix) = match parse_journal_state(path) {
        Some(s) => s,
        None => return HealClass::Malformed,
    };
    match state {
        "Committed" | "Aborted" => {
            let age = now.saturating_sub(last_unix);
            if age > threshold_secs {
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                HealClass::Remove {
                    path: path.to_path_buf(),
                    size,
                }
            } else {
                HealClass::Preserve
            }
        }
        "Started" => HealClass::Preserve,
        _ => HealClass::Malformed,
    }
}

/// Per-sidecar recovery report emitted by `recover_orphan_journals`.
/// Populated for every sidecar whose last entry is `Started` (a real
/// orphan that needs operator attention) and for sidecars whose last
/// entry is `Committed` or `Aborted` (informational; safe to clean).
#[derive(Debug, Clone, Serialize)]
#[allow(clippy::struct_field_names)]
pub struct OrphanJournalReport {
    /// Absolute path to the journal file.
    pub journal_path: String,
    /// The target file the journal was protecting.
    pub target: String,
    /// The original `op_id` from the `Started` entry.
    pub op_id: String,
    /// Whether the target was Created (`None` `checksum_before`) or Replaced.
    pub op: JournalOp,
    /// The precomputed `checksum_after` from the `Started` entry.
    pub expected_new_checksum: String,
    /// The recorded `checksum_before` (if any) for the original content.
    pub checksum_before: Option<String>,
    /// When the `Started` entry was first appended (Unix seconds).
    pub started_at_unix: u64,
    /// The PID of the process that started the write.
    pub pid: u32,
}

/// Scan `dir` (non-recursive) for `.atomwrite.journal.*.json` sidecars
/// and emit a recovery report for each orphan.
///
/// A journal is considered orphaned if its last entry is `Started`
/// (no matching `Committed` was appended before the crash).
///
/// **This function does NOT touch the filesystem** — it only reads the
/// sidecars. The caller is responsible for deciding what to do with the
/// orphan (replay, abort, or ignore).
pub fn recover_orphan_journals(dir: &Path) -> Result<Vec<OrphanJournalReport>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries =
        fs::read_dir(dir).with_context(|| format!("failed to read dir {}", dir.display()))?;
    let journal_paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|name| {
                        name.starts_with(".atomwrite.journal.") && name.ends_with(JOURNAL_EXT)
                    })
        })
        .collect();

    // Independent parse per journal → parallel when multi-item.
    let parsed: Vec<Result<Option<OrphanJournalReport>>> =
        if crate::concurrency::should_parallelize(journal_paths.len()) {
            journal_paths.par_iter().map(|path| parse_orphan(path)).collect()
        } else {
            journal_paths.iter().map(|path| parse_orphan(path)).collect()
        };

    let mut reports = Vec::new();
    for (path, result) in journal_paths.iter().zip(parsed) {
        match result {
            Ok(Some(report)) => reports.push(report),
            Ok(None) => {
                // Journal is intact (last entry = Committed or Aborted).
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "failed to parse journal");
            }
        }
    }
    Ok(reports)
}

/// Parse a single journal file and return `Some(report)` if the last
/// entry is `Started` (orphan), `None` if the journal is intact.
pub(crate) fn parse_orphan(path: &Path) -> Result<Option<OrphanJournalReport>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read journal {}", path.display()))?;
    let mut last_started: Option<JournalEntry> = None;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: JournalEntry = serde_json::from_str(line)
            .with_context(|| format!("invalid JSON in journal {}", path.display()))?;
        match &entry {
            JournalEntry::Started { .. } => last_started = Some(entry),
            JournalEntry::Committed { .. } | JournalEntry::Aborted { .. } => {
                last_started = None;
            }
        }
    }
    let Some(last) = last_started else {
        return Ok(None);
    };
    let JournalEntry::Started {
        op_id,
        op,
        target,
        checksum_before,
        checksum_after,
        pid,
        started_at_unix,
    } = last
    else {
        return Ok(None);
    };
    Ok(Some(OrphanJournalReport {
        journal_path: path.display().to_string(),
        target,
        op_id,
        op,
        expected_new_checksum: checksum_after,
        checksum_before,
        started_at_unix,
        pid,
    }))
}

