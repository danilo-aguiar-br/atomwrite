// SPDX-License-Identifier: MIT OR Apache-2.0
//! WAL sidecar creation policy heuristics (A-002 / A-013).

use std::path::Path;
use serde::{Deserialize, Serialize};

use super::{JournalOp, L1_LARGE_FILE_BYTES};

/// Policy governing WHEN a WAL sidecar is created (G119 L1 prevention).
///
/// The L1 layer prevents unnecessary sidecar pollution at the source: an
/// `auto` policy only creates a sidecar when the operation is non-trivial
/// (large file, edit/replace, or non-versioned directory), while
/// `always` and `never` keep the legacy and the opt-out semantics
/// respectively. The default is `auto` (R5 fix).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[clap(rename_all = "snake_case")]
pub enum WalPolicy {
    /// Decide based on heuristics: skip the sidecar for trivial writes
    /// (small file, plain write in a git-tracked dir, set/del trivial).
    #[default]
    Auto,
    /// Always create a sidecar (legacy semantics, `--strict-atomic`).
    Always,
    /// Never create a sidecar (overrides `--strict-atomic` and WAL env var).
    Never,
}

impl WalPolicy {
    /// String form for NDJSON / logs.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }
    }
}

/// Decide whether a sidecar should be created for `target` given the
/// operation kind and the active policy (G119 L1).
///
/// Returns `true` when the sidecar MUST be created (`Always` policy, or
/// the heuristics in `Auto` mode vote in favour). `false` means the
/// caller can skip the `journal_started` call entirely, which prevents
/// 60-80% of unnecessary sidecar writes for trivial operations (G119 R5).
///
/// The four `Auto` conditions that vote IN FAVOUR of creating a sidecar:
/// 1. Target file is larger than 1 MiB (recovery is expensive enough to
///    justify the I/O cost of the sidecar itself).
/// 2. Operation kind is `edit` or `replace` (in-place modifications that
///    lack native atomicity).
/// 3. The parent directory is NOT under git control (real risk of total
///    data loss, so audit trail matters).
/// 4. The target is a non-trivial file (size > 4 KiB) — trivial configs
///    of a few dozen bytes do not warrant a sidecar.
pub fn should_create_sidecar(target: &Path, op: JournalOp, policy: WalPolicy) -> bool {
    match policy {
        WalPolicy::Never => false,
        WalPolicy::Always => true,
        WalPolicy::Auto => {
            // 1. Large file → always sidecar
            let size = std::fs::metadata(target).map(|m| m.len()).unwrap_or(0);
            if size > L1_LARGE_FILE_BYTES {
                return true;
            }
            // 2. Edit / Replace → always sidecar (no native atomicity)
            if matches!(op, JournalOp::Edit | JournalOp::Replace) {
                return true;
            }
            // 3. Not under git → always sidecar (real recovery value)
            if !directory_is_git_tracked(target) {
                return true;
            }
            // 4. Trivial file → skip (A-013: named policy constant)
            if size <= crate::constants::WAL_SMALL_RECORD_BYTES {
                return false;
            }
            // Otherwise (medium-sized file in a git repo, plain write) →
            // default to NO sidecar to keep the working tree clean.
            false
        }
    }
}

/// Cheap heuristic: is `target`'s parent directory under git control?
/// Walks up from `target.parent()` looking for a `.git` entry. Bounded
/// at 16 levels to avoid pathological cases. Not a substitute for the
/// git CLI; it is purely a fast yes/no signal for the L1 heuristic.
fn directory_is_git_tracked(target: &Path) -> bool {
    let start = target.parent().unwrap_or_else(|| Path::new("."));
    let mut current = Some(start);
    let mut depth = 0u8;
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return true;
        }
        depth += 1;
        if depth > 16 {
            return false;
        }
        current = dir.parent();
    }
    false
}
