// SPDX-License-Identifier: MIT OR Apache-2.0

//! G114 — Write-Ahead Log (WAL) sidecar for crash-safe atomic writes.
//!
//! ## Problem
//!
//! The `atomic_write` pipeline has 13 steps. If the process is killed
//! (SIGKILL, OOM, power loss) between any two steps, the target file may
//! be left in a partially-written, truncated, or otherwise inconsistent
//! state. The variant `AtomwriteError::CopyBackBlake3Failed` (exit 92)
//! indicates a partial `WriteStrategy::CopyBack` failure, but offers no
//! recovery mechanism.
//!
//! ## Solution
//!
//! Before performing any write, append a JSON journal entry to a sidecar
//! file `.atomwrite.journal.<target>.json` that records:
//! - The operation type (`write` / `replace` / `edit`)
//! - The target path (relative to workspace)
//! - The BLAKE3 checksum of the *previous* content (if any)
//! - The BLAKE3 checksum of the *new* content (precomputed)
//! - The PID and start timestamp
//! - A 16-byte random `op_id` for idempotency
//!
//! After the atomic rename succeeds, append a `committed` entry with the
//! same `op_id`. On startup, the `recover_orphan_journals` function scans
//! for `.atomwrite.journal.*.json` files whose last entry is not
//! `committed`, and emits a structured NDJSON event `wal_recovery` per
//! orphan, listing the original `target`, the pre-computed
//! `expected_new_checksum`, and the recorded `op_id` for the caller to
//! decide whether to replay or abort.
//!
//! ## Causa x Efeito
//!
//! - **Causa**: `atomic_write` é uma sequência de 13 passos; crash entre
//!   qualquer par pode deixar o arquivo em estado inconsistente.
//! - **Efeito**: Sem WAL, recovery exige intervenção manual via
//!   `git checkout`, `cp` do backup mais recente, ou heurística ad-hoc.
//! - **Solução**: Sidecar journal append-only + recovery idempotente via
//!   `op_id`; recovery é puramente consultivo (não toca o filesystem).
//! - **Benefício**: Operador recebe `wal_recovery` NDJSON estruturado com
//!   `target`, `expected_new_checksum`, `op_id` para replay manual ou
//!   script de recovery.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use blake3::Hash;
use serde::{Deserialize, Serialize};

/// File extension used for WAL sidecar journals.
const JOURNAL_EXT: &str = ".atomwrite.journal.json";

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
fn append_entry(target: &Path, entry: &JournalEntry) -> Result<()> {
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
    let json = serde_json::to_string(entry)
        .with_context(|| format!("failed to serialize journal entry for {}", target.display()))?;
    writeln!(file, "{}", json)
        .with_context(|| format!("failed to write journal entry to {}", path.display()))?;
    file.sync_data()
        .with_context(|| format!("failed to fsync journal {}", path.display()))?;
    Ok(())
}

/// Result of scanning a single sidecar journal for orphans.
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
    let mut reports = Vec::new();
    if !dir.exists() {
        return Ok(reports);
    }
    let entries =
        fs::read_dir(dir).with_context(|| format!("failed to read dir {}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with(".atomwrite.journal.") || !name.ends_with(JOURNAL_EXT) {
            continue;
        }
        match parse_orphan(&path) {
            Ok(Some(report)) => reports.push(report),
            Ok(None) => {
                // Journal is intact (last entry = Committed or Aborted).
                // No recovery needed.
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
fn parse_orphan(path: &Path) -> Result<Option<OrphanJournalReport>> {
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

/// Test-only helper: read all entries from a journal file.
#[cfg(test)]
pub(crate) fn read_entries(path: &Path) -> Result<Vec<JournalEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read journal {}", path.display()))?;
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).context("invalid JSON"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn journal_path_appends_atomwrite_journal_json() {
        let target = Path::new("/tmp/foo.txt");
        let jp = journal_path(target);
        assert!(jp.ends_with(".atomwrite.journal.foo.txt.atomwrite.journal.json"));
    }

    #[test]
    fn journal_started_creates_sidecar_and_records_op_id() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("file.txt");
        let before = blake3::hash(b"old");
        let after = blake3::hash(b"new");
        let op_id = journal_started(&target, JournalOp::Write, Some(before), after).unwrap();
        assert_eq!(op_id.len(), 16);
        let jp = journal_path(&target);
        assert!(jp.exists());
        let entries = read_entries(&jp).unwrap();
        assert_eq!(entries.len(), 1);
        let JournalEntry::Started {
            op_id: recorded_id,
            op,
            target: t,
            checksum_before: cb,
            checksum_after: ca,
            pid,
            started_at_unix,
        } = &entries[0]
        else {
            panic!("expected Started entry");
        };
        assert_eq!(recorded_id, &op_id);
        assert_eq!(*op, JournalOp::Write);
        assert_eq!(t, &target.display().to_string());
        assert_eq!(cb.as_deref(), Some(before.to_hex().to_string().as_str()));
        assert_eq!(ca, &after.to_hex().to_string());
        assert_eq!(*pid, std::process::id());
        assert!(*started_at_unix > 0);
    }

    #[test]
    fn journal_committed_after_started_does_not_orphan() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("file.txt");
        let op_id = journal_started(&target, JournalOp::Edit, None, blake3::hash(b"x")).unwrap();
        journal_committed(&target, &op_id).unwrap();
        let reports = recover_orphan_journals(tmp.path()).unwrap();
        assert!(
            reports.is_empty(),
            "expected zero orphans, got {:?}",
            reports
        );
    }

    #[test]
    fn orphan_detected_when_started_without_committed() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("file.txt");
        let op_id = journal_started(
            &target,
            JournalOp::Write,
            Some(blake3::hash(b"old")),
            blake3::hash(b"new"),
        )
        .unwrap();
        let reports = recover_orphan_journals(tmp.path()).unwrap();
        assert_eq!(reports.len(), 1);
        let r = &reports[0];
        assert_eq!(r.op_id, op_id);
        assert_eq!(r.op, JournalOp::Write);
        assert_eq!(r.target, target.display().to_string());
        assert!(r.checksum_before.is_some());
        assert_eq!(r.pid, std::process::id());
    }

    #[test]
    fn journal_aborted_clears_orphan() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("file.txt");
        let op_id = journal_started(&target, JournalOp::Replace, None, blake3::hash(b"x")).unwrap();
        journal_aborted(&target, &op_id, "caller cancelled").unwrap();
        let reports = recover_orphan_journals(tmp.path()).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn generate_op_id_is_16_hex_chars_and_unique() {
        let a = generate_op_id();
        let b = generate_op_id();
        assert_eq!(a.len(), 16);
        assert_eq!(b.len(), 16);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn recover_on_empty_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let reports = recover_orphan_journals(tmp.path()).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn recover_on_missing_dir_returns_empty() {
        let missing = std::env::temp_dir().join("atomwrite-test-missing-dir-xyz");
        let _ = fs::remove_dir_all(&missing);
        let reports = recover_orphan_journals(&missing).unwrap();
        assert!(reports.is_empty());
    }
}
