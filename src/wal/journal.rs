// SPDX-License-Identifier: MIT OR Apache-2.0

//! WAL journal sidecar: entries, path, append, RAII guard.
//!
//! Workload: I/O-bound ordered journal I/O for a single target.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use blake3::Hash;
use serde::{Deserialize, Serialize};

use super::heuristics::heuristics_should_preserve;

/// Threshold above which a target file is considered "large" for the L1
/// prevention heuristic. Files smaller than this on a routine write do
/// not generate a WAL sidecar in `WalPolicy::Auto` mode.
///
/// A-013: re-export from [`crate::constants::WAL_L1_LARGE_FILE_BYTES`].
pub const L1_LARGE_FILE_BYTES: u64 = crate::constants::WAL_L1_LARGE_FILE_BYTES;

/// File extension used for WAL sidecar journals.
pub(crate) const JOURNAL_EXT: &str = ".atomwrite.journal.json";

/// A single entry in the append-only WAL journal.
///
/// Two variants:
/// - `Started`: written BEFORE the atomic write
/// - `Committed`: written AFTER the atomic write completes
/// - `Aborted`: written if the write fails before commit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum JournalEntry {
    /// Write was started but not yet completed.
    Started {
        /// 16-hex-char operation ID for correlation with Committed/Aborted.
        op_id: String,
        /// Operation type.
        op: JournalOp,
        /// Target file path (string form).
        target: String,
        /// BLAKE3 of the original content (None for new file creation).
        checksum_before: Option<String>,
        /// BLAKE3 of the new content (precomputed before write).
        checksum_after: String,
        /// Process ID that initiated the write.
        pid: u32,
        /// Unix seconds when the Started entry was appended.
        started_at_unix: u64,
    },
    /// Write was successfully committed.
    Committed {
        /// Operation ID matching the prior Started entry.
        op_id: String,
        /// Unix seconds when the Committed entry was appended.
        committed_at_unix: u64,
    },
    /// Write was aborted (interrupted) before commit.
    Aborted {
        /// Operation ID matching the prior Started entry.
        op_id: String,
        /// Unix seconds when the Aborted entry was appended.
        aborted_at_unix: u64,
        /// Human-readable reason for the abort.
        reason: String,
    },
}

/// Operation type recorded in the journal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JournalOp {
    /// Atomic write (write subcommand).
    Write,
    /// In-place edit (edit subcommand).
    Edit,
    /// Bulk replace (replace subcommand).
    Replace,
    /// Tier 3 config set (set subcommand).
    Set,
}

/// Compute the sidecar path used as the WAL journal for `target`.
///
/// Pattern: `<dir>/.atomwrite.journal.<basename>.atomwrite.journal.json`
/// so that orphans are visible via `ls -A` but do not clash with the lock
/// sidecar `.<target>.atomwrite.lock` from `crate::lock`.
pub fn journal_path(target: &Path) -> PathBuf {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let basename = target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    dir.join(format!(".atomwrite.journal.{}{}", basename, JOURNAL_EXT))
}

/// Generate a 16-hex-char op ID from random bytes.
///
/// Uses `blake3::hash` over `(pid, nanos)` for portability — no extra
/// `rand` dependency. Collisions in practice are astronomically rare.
pub fn generate_op_id() -> String {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let input = format!("{}-{}", pid, nanos);
    blake3::hash(input.as_bytes())
        .to_hex()
        .as_str()
        .chars()
        .take(16)
        .collect()
}

/// Append a `Started` entry to the journal for `target`.
///
/// Returns the generated `op_id` so the caller can correlate the
/// subsequent `committed` / `aborted` entry.
pub fn journal_started(
    target: &Path,
    op: JournalOp,
    checksum_before: Option<Hash>,
    checksum_after: Hash,
) -> Result<String> {
    let op_id = generate_op_id();
    let started_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let entry = JournalEntry::Started {
        op_id: op_id.clone(),
        op,
        target: target.display().to_string(),
        checksum_before: checksum_before.map(|h| h.to_hex().to_string()),
        checksum_after: checksum_after.to_hex().to_string(),
        pid: std::process::id(),
        started_at_unix,
    };
    append_entry(target, &entry)?;
    Ok(op_id)
}

/// Append a `Committed` entry to the journal for `target`.
pub fn journal_committed(target: &Path, op_id: &str) -> Result<()> {
    let committed_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let entry = JournalEntry::Committed {
        op_id: op_id.to_owned(),
        committed_at_unix,
    };
    append_entry(target, &entry)
}

/// Append an `Aborted` entry to the journal for `target`.
pub fn journal_aborted(target: &Path, op_id: &str, reason: &str) -> Result<()> {
    let aborted_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let entry = JournalEntry::Aborted {
        op_id: op_id.to_owned(),
        aborted_at_unix,
        reason: reason.to_owned(),
    };
    append_entry(target, &entry)
}

/// Append a single entry to the journal sidecar file.
///
/// Uses `fs::OpenOptions::append(true).create(true)` so the sidecar is
/// created on first write. Each entry is one JSON object followed by `\n`
/// (NDJSON inside the sidecar), so the file remains human-readable and
/// trivially parseable.
pub(crate) fn append_entry(target: &Path, entry: &JournalEntry) -> Result<()> {
    let path = journal_path(target);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create journal dir {}", parent.display()))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open journal {}", path.display()))?;
    // Stream serialize (to_writer) rather than String intermediate — NDJSON rules.
    serde_json::to_writer(&mut file, entry).with_context(|| {
        format!("failed to serialize journal entry for {}", target.display())
    })?;
    file.write_all(b"\n")
        .with_context(|| format!("failed to write journal entry to {}", path.display()))?;
    file.sync_data()
        .with_context(|| format!("failed to fsync journal {}", path.display()))?;
    Ok(())
}

/// RAII guard that removes the sidecar journal on normal drop (G119 L2).
///
/// When `atomic_write` is called with `--strict-atomic` or WAL policy Always,
/// a `.atomwrite.journal.<basename>.json` sidecar is created with a
/// `Started` entry, then a `Committed` entry is appended when the rename
/// succeeds. Before v0.1.15, the sidecar was left in place forever,
/// polluting the working tree (60+ orphans observed in the G119 audit).
///
/// `JournalGuard` binds the lifecycle of the sidecar to the `atomic_write`
/// scope: on normal drop (after the `Committed` entry was written) the
/// sidecar is removed. On panic or early return (where the caller can set
/// `keep_on_drop = true`), the sidecar survives so `recover_orphan_journals`
/// can still inspect it.
///
/// This is the canonical Rust RAII pattern: acquire the sidecar in
/// `journal_started`, release it via `Drop` at the end of the scope.
/// Mirrors `tempfile::TempPath` (stebalien/tempfile) which is the
/// reference implementation of this pattern in the ecosystem.
///
/// As of v0.1.17, the Drop consults the L4 heuristics engine before
/// removing. If any of `h1_ttl`, `h3_rate_limit`, `h4_sentinel`, or
/// `h5_archive` vote to preserve, the sidecar survives. The
/// `h2_lru_within_cap` heuristic is intentionally bypassed here because
/// the per-file Drop context has no cheap way to know the global
/// committed count; a workspace-wide sweep is the right place for that
/// (see `wal-heal` and `auto_heal_on_startup`).
/// RAII journal sidecar guard.
///
/// `#[must_use]`: the guard owns the decision to keep or remove the journal
/// on drop (`keep` / `release`). Ignoring it would drop immediately and may
/// delete a sidecar before the write pipeline finishes (resource ownership).
#[must_use = "JournalGuard controls sidecar lifetime on drop; call keep()/release()"]
#[derive(Debug)]
pub struct JournalGuard {
    pub(crate) path: PathBuf,
    pub(crate) keep_on_drop: bool,
    pub(crate) op_id: Option<String>,
    /// Unix seconds when the `Committed` entry was appended; populated
    /// by `release()` and consumed by `Drop` for the L4 heuristics. `None`
    /// for the inert guard and for callers that call `keep()` instead of
    /// `release()` (no Committed entry existed).
    pub(crate) committed_at_unix: Option<u64>,
}

impl JournalGuard {
    /// Create an inert guard that does nothing on drop. Used as a
    /// fallback when `journal_started` failed (e.g. no permissions to
    /// write the sidecar) — the caller still has a `JournalGuard` to
    /// keep the API uniform, but no sidecar exists to remove.
    pub fn inert() -> Self {
        Self {
            path: PathBuf::new(),
            keep_on_drop: true,
            op_id: None,
            committed_at_unix: None,
        }
    }

    /// Mark the guard so the sidecar will NOT be removed on drop.
    /// Use this when the write failed in a way that leaves the sidecar
    /// useful for crash recovery or audit.
    pub fn keep(&mut self) {
        self.keep_on_drop = true;
    }

    /// Mark the guard so the sidecar WILL be removed on drop. This is
    /// the default after `Committed` is appended. Captures the current
    /// Unix timestamp so the Drop guard can feed the L4 heuristics that
    /// reason about post-commit age (`h1_ttl`, `h5_archive`).
    pub fn release(&mut self) {
        self.keep_on_drop = false;
        self.committed_at_unix = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
    }

    /// The `op_id` of the journal entry this guard is protecting.
    #[allow(dead_code)]
    pub fn op_id(&self) -> Option<&str> {
        self.op_id.as_deref()
    }
}

impl Drop for JournalGuard {
    fn drop(&mut self) {
        if self.keep_on_drop {
            return;
        }
        if self.path.as_os_str().is_empty() {
            // Inert guard — nothing to remove.
            return;
        }

        // G119 L4: consult the heuristics engine before removing. We pass
        // `u64::MAX` for both `workspace_committed_count` and `age_rank`
        // to deliberately disable `h2_lru_within_cap` (which returns true
        // when `count <= max_count`); the L4 logic is OR-composed so any
        // OTHER heuristic voting true will keep the sidecar. This keeps
        // the per-write Drop path bounded: no workspace-wide scan here.
        let committed_at = self.committed_at_unix.unwrap_or(0);
        if heuristics_should_preserve(&self.path, committed_at, u64::MAX, u64::MAX) {
            tracing::debug!(
                path = %self.path.display(),
                "G119 L4: heuristics voted to preserve sidecar; skipping remove"
            );
            return;
        }

        if let Err(e) = fs::remove_file(&self.path) {
            // Removal is best-effort: a leak is preferable to a panic
            // during unwinding. The next `auto_heal_on_startup` will
            // pick it up.
            tracing::debug!(path = %self.path.display(), error = %e,
                "journal guard: sidecar removal failed (will be reaped later)");
        }
    }
}

/// Wrap a `Started` journal write in a Drop guard. On normal scope exit
/// (the caller did not call `keep()`), the sidecar is removed — the
/// `Committed` entry is the last useful record and the file is deleted
/// to keep the working tree clean (G119 R1 fix).
///
/// Returns the `op_id` and the guard. The caller MUST call `release()`
/// after `Committed` is appended and the rename succeeded; on failure
/// the caller MUST call `keep()` so `recover_orphan_journals` can
/// inspect the orphan at the next startup.
pub fn journal_started_with_guard(
    target: &Path,
    op: JournalOp,
    checksum_before: Option<Hash>,
    checksum_after: Hash,
) -> Result<(String, JournalGuard)> {
    let op_id = journal_started(target, op, checksum_before, checksum_after)?;
    let path = journal_path(target);
    // Default: keep on drop until the caller explicitly `release()`s after
    // the write succeeds. This makes the safe-by-default behaviour match
    // the pre-v0.1.15 semantics (sidecar survives on panic) while making
    // the new "auto-clean on success" path explicit.
    let guard = JournalGuard {
        path,
        keep_on_drop: true,
        op_id: Some(op_id.clone()),
        committed_at_unix: None,
    };
    Ok((op_id, guard))
}
