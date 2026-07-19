// SPDX-License-Identifier: MIT OR Apache-2.0

//! WAL retention heuristics H1–H5.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};


/// Per-heuristic outcome: true = preserve the sidecar, false = clean it.
pub type Decision = bool;

/// H1 — TTL (time to live): preserve the sidecar for N seconds after
/// `Committed` even if everything looks OK. Default 0 (no TTL;
/// the Drop guard removes immediately).
pub fn h1_ttl(journal_committed_at_unix: u64) -> Decision {
    // G-007: defaults from constants / XDG config — no product env knobs.
    let ttl_secs: u64 = crate::constants::WAL_KEEP_SECS_DEFAULT;
    if ttl_secs == 0 {
        return false;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let age = now.saturating_sub(journal_committed_at_unix);
    age < ttl_secs
}

/// H2 — LRU by count: if the workspace currently has more than M
/// `Committed` sidecars, the OLDEST ones beyond the cap are
/// candidates for removal. This function returns `true` ONLY when
/// the sidecar IS within the cap (so the engine will preserve it).
/// The caller passes the total `Committed` count and the per-file
/// age rank.
pub fn h2_lru_within_cap(workspace_committed_count: u64, age_rank: u64) -> Decision {
    let max_count: u64 = crate::constants::WAL_MAX_COUNT_DEFAULT;
    age_rank < max_count || workspace_committed_count <= max_count
}

/// H3 — Rate limit: throttle agent floods. If more than K sidecars
/// are created in the last 60 seconds, return `true` to suppress
/// further creation attempts. The counter is process-local via a
/// static `AtomicU64` of the start-of-window timestamp.
pub fn h3_rate_limit() -> Decision {
    let max_per_min: u64 = crate::constants::WAL_RATE_LIMIT_DEFAULT;
    if max_per_min == 0 {
        return false;
    }
    // Process-local rate window. Function-local `static` (not `const`) so
    // every call shares one address — interior-mutable atomics must never
    // live in `const` (each use site would get a fresh counter).
    //
    // Ordering::Relaxed is intentional: this is a best-effort throttle for
    // agent floods, not a data-publication barrier. Concurrent threads may
    // briefly overshoot the cap; absolute accuracy is not required.
    // Window reset uses compare_exchange so two threads that both observe
    // an expired window do not clobber each other's start/count with plain
    // stores (still Relaxed — no cross-field happens-before needed).
    static WINDOW_START: AtomicU64 = AtomicU64::new(0);
    static WINDOW_COUNT: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let start = WINDOW_START.load(Ordering::Relaxed);
    if now.saturating_sub(start) >= 60 {
        match WINDOW_START.compare_exchange(start, now, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => {
                WINDOW_COUNT.store(1, Ordering::Relaxed);
                return false;
            }
            Err(_) => {
                // Another thread already opened a new window; fall through
                // to increment the shared count under the new start.
            }
        }
    }
    let count = WINDOW_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    count > max_per_min
}

/// H4 — Opt-out sentinel: a `.atomwrite_no_wal` file in the target
/// directory disables sidecar creation for that directory tree.
pub fn h4_sentinel(target: &Path) -> Decision {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    dir.join(".atomwrite_no_wal").exists()
}

/// H5 — Archive threshold: sidecars older than N days are candidates
/// for the zstd-compressed archive. Returns `true` to preserve
/// (archive instead of delete). The actual archive step is the
/// caller's responsibility; this heuristic only votes.
pub fn h5_archive(journal_committed_at_unix: u64) -> Decision {
    let archive_days: u64 = crate::constants::WAL_ARCHIVE_DAYS_DEFAULT;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let age_days = now.saturating_sub(journal_committed_at_unix) / 86_400;
    age_days >= archive_days
}

/// Compose all 5 L4 heuristics into a single decision. Returns `true`
/// when AT LEAST ONE heuristic votes to PRESERVE the sidecar.
pub fn heuristics_should_preserve(
    target: &Path,
    journal_committed_at_unix: u64,
    workspace_committed_count: u64,
    age_rank: u64,
) -> bool {
    h1_ttl(journal_committed_at_unix)
        || h2_lru_within_cap(workspace_committed_count, age_rank)
        || h3_rate_limit()
        || h4_sentinel(target)
        || h5_archive(journal_committed_at_unix)
}
