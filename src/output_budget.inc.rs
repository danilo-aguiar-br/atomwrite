// SPDX-License-Identifier: MIT OR Apache-2.0
// The `--limit` work budget, included from output.rs.
//
// The budget has two clocks on purpose. LIMIT_SPENT counts what the writer
// emitted, which is what the consumer can read. WORK_SPENT counts side effects
// a verb performs BEFORE it emits the record, so `--limit 1` on a rewrite verb
// rewrites one file instead of rewriting forty and printing one.
// `include!` places every item at the crate root, so `crate::arm_limit` and
// `crate::claim_work` resolve without a re-export.

/// Set once `--limit` made the writer drop a record.
///
/// Scanning verbs poll this to stop the walker at the point where more work
/// could not change the answer. It is process-wide because the writer and the
/// walk run on different threads and share no other state.
static LIMIT_REACHED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// True once `--limit` has been reached in this process.
///
/// A verb that scans a tree uses it to quit the walk instead of reading files
/// whose records the writer would discard anyway.
#[must_use]
pub fn limit_reached() -> bool {
    LIMIT_REACHED.load(std::sync::atomic::Ordering::Acquire)
}

/// Record that `--limit` dropped a record.
///
/// Scanning verbs poll [`limit_reached`] and quit their walk: the records the
/// walker would still produce could not change the answer.
pub fn mark_limit_reached() {
    LIMIT_REACHED.store(true, std::sync::atomic::Ordering::Release);
}

/// Records the run is allowed to produce, as armed by [`arm_limit`].
///
/// `None` means `--limit` was not given (or was given as `0`, which disables it)
/// and the run is unbounded.
static LIMIT_CAP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Records the run has already produced, as counted by the writer.
static LIMIT_SPENT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Arm the process-wide work budget from `--limit`.
///
/// Called once, right after the CLI is parsed and before any verb runs. The
/// writer spends the budget as it emits ([`spend_budget`]), and every verb reads
/// what is left through [`remaining_budget`] so collection can be cut BEFORE the
/// side effect instead of after it.
pub fn arm_limit(limit: Option<u64>) {
    let cap = limit.filter(|n| *n > 0).unwrap_or(0);
    LIMIT_CAP.store(cap, std::sync::atomic::Ordering::Release);
    LIMIT_SPENT.store(0, std::sync::atomic::Ordering::Release);
    WORK_SPENT.store(0, std::sync::atomic::Ordering::Release);
    BUDGET_DROPPED.store(0, std::sync::atomic::Ordering::Release);
    LIMIT_REACHED.store(false, std::sync::atomic::Ordering::Release);
}

/// The `--limit` cap armed for this process, or `None` when unbounded.
#[must_use]
pub fn limit_cap() -> Option<u64> {
    match LIMIT_CAP.load(std::sync::atomic::Ordering::Acquire) {
        0 => None,
        n => Some(n),
    }
}

/// Records the budget still allows, or `None` when the run is unbounded.
#[must_use]
pub fn remaining_budget() -> Option<u64> {
    let cap = limit_cap()?;
    let spent = LIMIT_SPENT.load(std::sync::atomic::Ordering::Acquire);
    Some(cap.saturating_sub(spent))
}

/// Charge one produced record against the budget and report whether the cap is
/// now full.
///
/// Returns `true` when this record filled the cap, which is the moment a
/// scanning verb must stop: doing one more unit of work could not change the
/// answer the consumer is allowed to read.
#[must_use]
pub fn spend_budget() -> bool {
    let Some(cap) = limit_cap() else {
        return false;
    };
    let spent = LIMIT_SPENT.fetch_add(1, std::sync::atomic::Ordering::AcqRel) + 1;
    if spent >= cap {
        mark_limit_reached();
        return true;
    }
    false
}

/// Work units the run may still claim, counted apart from what the writer emits.
///
/// A verb that rewrites a file before its record is emitted cannot wait for the
/// writer to spend the budget: by then the write already happened. It claims the
/// unit here instead, so `--limit 1` really writes one file even with many walk
/// workers running.
static WORK_SPENT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Claim one side effect against the `--limit` budget.
///
/// Returns `false` when the budget is already exhausted, in which case the
/// caller must NOT perform the side effect. The claim that fills the cap returns
/// `true` and marks the limit as reached, so the other walk workers quit instead
/// of starting work whose record could never be read.
pub fn claim_work() -> bool {
    let Some(cap) = limit_cap() else {
        return true;
    };
    let spent = WORK_SPENT.fetch_add(1, std::sync::atomic::Ordering::AcqRel) + 1;
    if spent >= cap {
        mark_limit_reached();
    }
    spent <= cap
}

/// Items the `--limit` budget cut from a collected list in this process.
///
/// `walker_stopped_early` says the run stopped; this says how much of the
/// answer the budget removed, which is the number a consumer needs to know
/// whether the page is the whole tree or a slice of it.
static BUDGET_DROPPED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Items the `--limit` budget cut from collected lists in this process.
#[must_use]
pub fn budget_dropped() -> u64 {
    BUDGET_DROPPED.load(std::sync::atomic::Ordering::Acquire)
}

/// Charge one record the writer refused because the budget was already spent.
///
/// A streaming verb hands records to the writer one by one, so its cut happens
/// here and not in [`cap_collected`]. Both are the same `--limit` budget, so
/// both feed the same counter: two counters for one budget would let a summary
/// report only the half of the cut its verb happened to hit.
pub fn note_stream_cut() {
    BUDGET_DROPPED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    mark_limit_reached();
}

/// Cut a collected list down to the remaining budget.
///
/// Returns `true` when the list was truncated, and marks the limit as reached so
/// the terminal summary can report `walker_stopped_early`. Verbs that collect
/// everything before acting call this between the collection and the side
/// effect, which is what makes `--limit 1` touch exactly one target. The number
/// of items removed is charged to [`budget_dropped`], so the summary can say how
/// much of the answer the budget cut instead of only that it stopped.
pub fn cap_collected<T>(items: &mut Vec<T>) -> bool {
    let Some(budget) = remaining_budget() else {
        return false;
    };
    let budget = usize::try_from(budget).unwrap_or(usize::MAX);
    if items.len() <= budget {
        return false;
    }
    let dropped = items.len().saturating_sub(budget) as u64;
    items.truncate(budget);
    BUDGET_DROPPED.fetch_add(dropped, std::sync::atomic::Ordering::Relaxed);
    mark_limit_reached();
    true
}
