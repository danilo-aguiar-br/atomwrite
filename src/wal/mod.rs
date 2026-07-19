// SPDX-License-Identifier: MIT OR Apache-2.0

//! G114 — Write-Ahead Log (WAL) sidecar for crash-safe atomic writes.
//!
//! Workload: I/O-bound (sidecar JSON append + multi-journal scan for heal/stats).
//! Parallelism: hot write path stays ordered (journal before mutate — single
//! target). Multi-journal discovery uses `ignore::WalkParallel` bound by the
//! process-wide thread pool; parse/classify/unlink of independent journals
//! fan out via `rayon::par_iter` (stats, heal, recover).

mod journal;
pub mod heuristics;
pub mod policy;
mod stats;
mod heal;

pub use journal::{
    generate_op_id, journal_aborted, journal_committed, journal_path, journal_started,
    journal_started_with_guard, JournalEntry, JournalGuard, JournalOp, L1_LARGE_FILE_BYTES,
};
pub use stats::{compute_wal_stats, walk_journal_paths, WalDirEntry, WalStateBreakdown, WalStats};
pub use heal::{auto_heal_on_startup, recover_orphan_journals, AutoHealReport, OrphanJournalReport};
pub use policy::{WalPolicy, should_create_sidecar};
pub use heuristics::heuristics_should_preserve;


include!("tests.inc.rs");
