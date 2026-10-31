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
///
/// H3 is a CREATION-time vote only: it is consulted by
/// [`crate::wal::creation_throttled`] when a journal is about to be
/// opened. It must never be consulted when a committed journal is released,
/// otherwise every journal past the K-th of a multi-file run would outlive
/// its successful write.
pub fn h3_rate_limit() -> Decision {
    // Process-local window. A `static` (not `const`) so every call shares one
    // address — interior-mutable atomics must never live in `const` (each use
    // site would get a fresh counter).
    static GLOBAL: RateWindow = RateWindow::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    GLOBAL.hit(now, crate::constants::WAL_RATE_LIMIT_DEFAULT)
}

/// Fixed-length counting window behind [`h3_rate_limit`].
///
/// Kept as a type, with the clock passed in, so a test can drive its own
/// instance deterministically instead of racing the process-wide one.
///
/// `Ordering::Relaxed` is intentional: this is a best-effort throttle for
/// agent floods, not a data-publication barrier. Concurrent threads may
/// briefly overshoot the cap; absolute accuracy is not required. The window
/// reset uses `compare_exchange` so two threads that both observe an expired
/// window do not clobber each other's start/count with plain stores.
#[derive(Debug)]
pub struct RateWindow {
    start: AtomicU64,
    count: AtomicU64,
}

impl RateWindow {
    /// An empty window; the first hit opens one.
    pub const fn new() -> Self {
        Self {
            start: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Record one event at `now_secs` (Unix seconds). Returns `true` when more
    /// than `max_per_window` events fell in the current window, i.e. throttle.
    /// A `max_per_window` of 0 disables the limit.
    pub fn hit(&self, now_secs: u64, max_per_window: u64) -> bool {
        if max_per_window == 0 {
            return false;
        }
        let start = self.start.load(Ordering::Relaxed);
        if now_secs.saturating_sub(start) >= crate::constants::WAL_RATE_WINDOW_SECS {
            match self
                .start
                .compare_exchange(start, now_secs, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => {
                    self.count.store(1, Ordering::Relaxed);
                    return false;
                }
                Err(_) => {
                    // Another thread already opened a new window; fall through
                    // to increment the shared count under the new start.
                }
            }
        }
        self.count.fetch_add(1, Ordering::Relaxed) + 1 > max_per_window
    }
}

impl Default for RateWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// H4 — Keep sentinel: a `.atomwrite_keep_wal` file in the target directory
/// PRESERVES the sidecars written there.
///
/// The legacy name `.atomwrite_no_wal` is honoured too, because it always had
/// this exact effect: a directory holding it kept its journal, the opposite of
/// what the name promised. Both names preserve; neither disables the WAL.
pub fn h4_sentinel(target: &Path) -> Decision {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    dir.join(crate::constants::WAL_KEEP_SENTINEL).exists()
        || dir.join(crate::constants::WAL_LEGACY_SENTINEL).exists()
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
    let age_days = now.saturating_sub(journal_committed_at_unix) / crate::constants::SECS_PER_DAY;
    age_days >= archive_days
}

/// Compose the RELEASE-time L4 heuristics (H1, H2, H4, H5) into a single
/// decision. Returns `true` when AT LEAST ONE heuristic votes to PRESERVE the
/// sidecar of a write that already committed.
///
/// H3 is deliberately absent: it throttles journal creation (see
/// [`h3_rate_limit`]) and has no say once the write succeeded.
pub fn heuristics_should_preserve(
    target: &Path,
    journal_committed_at_unix: u64,
    workspace_committed_count: u64,
    age_rank: u64,
) -> bool {
    h1_ttl(journal_committed_at_unix)
        || h2_lru_within_cap(workspace_committed_count, age_rank)
        || h4_sentinel(target)
        || h5_archive(journal_committed_at_unix)
}
