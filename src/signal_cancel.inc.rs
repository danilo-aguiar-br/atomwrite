// SPDX-License-Identifier: MIT OR Apache-2.0
// Cooperative cancellation: cancel error and `CancelCheck`.
// Included from signal.rs.

/// Build a [`crate::error::AtomwriteError::Cancelled`] with the live signal exit code.
///
/// Prefer this over hard-coding `exit: 143` so SIGINT and `--timeout-secs`
/// surface the correct Unix convention (130 / 124).
#[inline]
pub fn cancelled_error(reason: impl Into<String>) -> crate::error::AtomwriteError {
    crate::error::AtomwriteError::Cancelled {
        reason: reason.into(),
        exit: current_exit_code(),
    }
}

/// Instant at which the global `--timeout-secs` deadline fires, once armed.
///
/// Written once by [`arm_global_timeout`]; retry waits read it so they never
/// sleep past the deadline the caller has already promised.
static GLOBAL_DEADLINE: OnceLock<std::time::Instant> = OnceLock::new();

/// Remember when the global deadline `secs` seconds from now fires.
///
/// Called by [`arm_global_timeout`] before its watchdog thread starts, so the
/// recorded instant is never later than the moment the watchdog raises the flag.
fn record_global_deadline(secs: u64) {
    if let Some(at) = std::time::Instant::now().checked_add(std::time::Duration::from_secs(secs)) {
        GLOBAL_DEADLINE.set(at).ok();
    }
}

/// Time left until the global `--timeout-secs` deadline.
///
/// Returns `None` when no deadline is armed (`--timeout-secs 0` or not armed
/// yet) and [`std::time::Duration::ZERO`] once it has passed.
#[must_use]
pub fn global_deadline_remaining() -> Option<std::time::Duration> {
    GLOBAL_DEADLINE
        .get()
        .map(|at| at.saturating_duration_since(std::time::Instant::now()))
}

/// Cooperative cancellation check point with **block** granularity (v0.1.36).
///
/// This is the single primitive every long-running loop uses to observe the
/// global deadline (`--timeout-secs`), SIGINT and SIGTERM. Before v0.1.36 the
/// only check points sat on the file boundary, so a single large file blew
/// through the deadline and the summary still reported a full run.
///
/// The check point is deliberately cheaper than any alternative:
/// - one `Acquire` load of an `AtomicBool` (pairs with the `Release` store in
///   `ShutdownSignal::record_signal`), no syscall and no allocation;
/// - the `flag` handle is cloned **once** per file, so the steady-state cost
///   inside the loop is a single saturating decrement plus a compare;
/// - it does **not** consult the process-wide shutdown slot, so it stays correct under
///   tests and library callers that wire their own [`ShutdownSignal`].
///
/// Call [`Self::advance`] once per work unit and [`Self::check`] at natural
/// boundaries; a cancelled run returns [`AtomwriteError::Cancelled`] carrying
/// the live exit code (124 / 130 / 143).
#[derive(Clone, Debug)]
pub struct CancelCheck {
    flag: Arc<AtomicBool>,
    /// Block size restored after each poll.
    block: u32,
    /// Work units still allowed before the next atomic load.
    ///
    /// `u32` keeps the struct small and the decrement cheap; block sizes are
    /// far below `u32::MAX`, so the cast never truncates.
    budget: u32,
}

impl CancelCheck {
    /// Wrap a shutdown flag for block-granular polling.
    ///
    /// `block` is clamped to at least 1 so a zero cannot turn the check into
    /// an unconditional atomic load per work unit.
    #[inline]
    #[must_use]
    pub fn new(flag: Arc<AtomicBool>, block: u32) -> Self {
        let block = block.max(1);
        Self {
            flag,
            block,
            budget: block,
        }
    }

    /// Cancel check point for a byte-oriented scan loop.
    ///
    /// Uses [`CANCEL_BLOCK_BYTES`] as the block size.
    #[inline]
    #[must_use]
    pub fn for_bytes(flag: Arc<AtomicBool>) -> Self {
        Self::new(flag, CANCEL_BLOCK_BYTES as u32)
    }

    /// Cancel check point for a match-oriented loop.
    ///
    /// Uses [`CANCEL_BLOCK_MATCHES`] as the block size.
    #[inline]
    #[must_use]
    pub fn for_matches(flag: Arc<AtomicBool>) -> Self {
        Self::new(flag, CANCEL_BLOCK_MATCHES as u32)
    }

    /// Consume `units` of work and return the cancellation error when the run
    /// was cancelled during this block.
    ///
    /// Returns `Ok(())` when the deadline has not fired. The atomic is loaded
    /// only once the accumulated units cross a block boundary, which is what
    /// keeps the steady-state cost near zero.
    ///
    /// # Errors
    ///
    /// Returns [`AtomwriteError::Cancelled`] once the shutdown flag is set.
    #[inline]
    pub fn advance(&mut self, units: u64) -> std::result::Result<(), AtomwriteError> {
        let units = u32::try_from(units).unwrap_or(u32::MAX);
        self.budget = self.budget.saturating_sub(units);
        if self.budget > 0 {
            return Ok(());
        }
        self.budget = self.block;
        self.check()
    }

    /// Poll the shutdown flag unconditionally, ignoring the block budget.
    ///
    /// Call at a natural boundary where the cost is irrelevant, such as
    /// before an atomic write or right after a write.
    ///
    /// # Errors
    ///
    /// Returns [`AtomwriteError::Cancelled`] once the shutdown flag is set.
    #[inline]
    pub fn check(&self) -> std::result::Result<(), AtomwriteError> {
        if self.flag.load(Ordering::Acquire) {
            return Err(cancelled_error(CANCEL_REASON));
        }
        Ok(())
    }
}
