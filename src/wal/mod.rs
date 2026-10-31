// SPDX-License-Identifier: MIT OR Apache-2.0

//! G114 — Write-Ahead Log (WAL) sidecar for crash-safe atomic writes.
//!
//! Workload: I/O-bound (sidecar JSON append + multi-journal scan for heal/stats).
//! Parallelism: hot write path stays ordered (journal before mutate — single
//! target). Multi-journal discovery uses `ignore::WalkParallel` bound by the
//! process-wide thread pool; parse/classify/unlink of independent journals
//! fan out via `rayon::par_iter` (stats, heal, recover).

mod heal;
pub mod heuristics;
mod journal;
pub mod policy;
mod stats;

pub use heal::{
    AutoHealReport, OrphanJournalReport, auto_heal_on_startup, auto_heal_scoped,
    recover_orphan_journals,
};
pub use heuristics::heuristics_should_preserve;
pub use journal::{
    JournalEntry, JournalGuard, JournalOp, L1_LARGE_FILE_BYTES, generate_op_id, journal_aborted,
    journal_committed, journal_path, journal_started, journal_started_with_guard,
};
pub use policy::{WalPolicy, creation_throttled, should_create_sidecar};
pub use stats::{
    HealScope, WalDirEntry, WalStateBreakdown, WalStats, compute_wal_stats, walk_journal_paths,
    walk_journal_paths_bounded,
};

include!("tests.inc.rs");
include!("tests_scoped.inc.rs");
