// SPDX-License-Identifier: MIT OR Apache-2.0

//! Retry with exponential backoff, full jitter and a hard deadline.
//!
//! Workload: wait-bound (the caller is blocked on a contended resource).
//! Parallelism: none — one [`Backoff`](crate::retry::Backoff) belongs to one waiting thread and owns its
//! own random generator, so nothing is shared between threads.
//!
//! Every wait in the product that retries a contended resource goes through
//! this module: the advisory lock ([`crate::lock`]) and the Windows tempfile
//! persist. The rules it enforces:
//!
//! - **Full jitter.** Wait `n` lasts a uniform random time in
//!   `[0, min(cap, base * 2^n)]`, so writers that collided once do not collide
//!   again in lockstep.
//! - **Two stops.** A wait ends the retry after `max_attempts` waits or when the
//!   time budget is spent, whichever comes first.
//! - **Budget under the global deadline.** The budget is the smaller of the
//!   policy's `max_elapsed` and the time left of `--timeout-secs`, and no sleep
//!   ever runs past it.
//! - **Cancellation at any moment.** A sleep is cut into slices of
//!   [`crate::constants::RETRY_SLICE_MS`]; the shutdown flag is polled between
//!   slices, so SIGINT, SIGTERM, SIGHUP and the deadline are seen within one
//!   slice.
//! - **No immediate retry.** The first wait comes before the second attempt.

use std::time::{Duration, Instant};

use crate::constants::{RETRY_GLOBAL_DEADLINE_GRACE_MS, RETRY_MAX_BUDGET_SECS, RETRY_SLICE_MS};

/// Largest time budget accepted; keeps `Instant` arithmetic from overflowing.
const MAX_BUDGET: Duration = Duration::from_secs(RETRY_MAX_BUDGET_SECS);

/// Backoff shape and stop conditions of one kind of retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    base: Duration,
    cap: Duration,
    max_elapsed: Duration,
    max_attempts: u32,
}

impl RetryPolicy {
    /// Build a policy from millisecond values.
    ///
    /// `base_ms` is the ceiling of the first wait, `cap_ms` the ceiling no wait
    /// exceeds, `max_elapsed_ms` the time budget and `max_attempts` the number
    /// of waits allowed before the retry gives up.
    #[must_use]
    pub const fn new(base_ms: u64, cap_ms: u64, max_elapsed_ms: u64, max_attempts: u32) -> Self {
        Self {
            base: Duration::from_millis(base_ms),
            cap: Duration::from_millis(cap_ms),
            max_elapsed: Duration::from_millis(max_elapsed_ms),
            max_attempts,
        }
    }

    /// Apply the `retry.base_ms` and `retry.max_ms` overrides of the config.
    ///
    /// A key left unset keeps the value of this policy.
    #[must_use]
    pub fn with_shape_overrides(mut self, tuning: &crate::config::Tuning) -> Self {
        if let Some(base_ms) = tuning.retry_base_ms {
            self.base = Duration::from_millis(base_ms);
        }
        if let Some(max_ms) = tuning.retry_max_ms {
            self.cap = Duration::from_millis(max_ms);
        }
        self
    }

    /// Apply the `retry.max_elapsed_ms` override of the config.
    ///
    /// Only for a retry whose budget has no key of its own; the advisory lock
    /// budget is `lock.acquire_timeout_ms`.
    #[must_use]
    pub fn with_budget_override(mut self, tuning: &crate::config::Tuning) -> Self {
        if let Some(max_elapsed_ms) = tuning.retry_max_elapsed_ms {
            self.max_elapsed = Duration::from_millis(max_elapsed_ms);
        }
        self
    }

    /// Upper bound of wait number `attempt` (zero based): `min(cap, base * 2^attempt)`.
    #[must_use]
    pub fn ceiling(&self, attempt: u32) -> Duration {
        let factor = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
        self.base.saturating_mul(factor).min(self.cap)
    }

    /// Full-jitter wait for `attempt`: uniform in `[0, ceiling(attempt)]`.
    #[must_use]
    pub fn jittered(&self, attempt: u32, rng: &mut fastrand::Rng) -> Duration {
        let ceiling = u64::try_from(self.ceiling(attempt).as_micros()).unwrap_or(u64::MAX);
        Duration::from_micros(rng.u64(0..=ceiling))
    }
}

/// Why a retry stopped waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RetryStop {
    /// A signal or the global `--timeout-secs` deadline cancelled the run.
    #[error("retry cancelled by a signal or the global deadline")]
    Cancelled,
    /// The time budget of the policy is spent.
    #[error("retry time budget exhausted")]
    DeadlineElapsed,
    /// The policy allows no more waits.
    #[error("retry attempts exhausted")]
    AttemptsExhausted,
}

/// State of one retry loop: how many waits happened and when the budget ends.
#[derive(Debug)]
pub struct Backoff {
    policy: RetryPolicy,
    started: Instant,
    deadline: Instant,
    /// The deadline comes from `--timeout-secs`, not from the policy.
    global_bound: bool,
    attempt: u32,
    rng: fastrand::Rng,
}

impl Backoff {
    /// Start a retry loop under `policy` and the live global deadline.
    #[must_use]
    pub fn new(policy: RetryPolicy) -> Self {
        Self::with_parts(
            policy.with_shape_overrides(&crate::config::tuning()),
            fastrand::Rng::new(),
            crate::signal::global_deadline_remaining(),
        )
    }

    /// Start a retry loop with an explicit generator and global time left.
    ///
    /// `global_left` is the time remaining of `--timeout-secs` (`None` when no
    /// deadline is armed). A seeded `rng` makes the waits reproducible in tests.
    #[must_use]
    pub fn with_parts(
        policy: RetryPolicy,
        rng: fastrand::Rng,
        global_left: Option<Duration>,
    ) -> Self {
        let started = Instant::now();
        let (left, global_bound) = match global_left {
            Some(global) if global <= policy.max_elapsed => (global, true),
            _ => (policy.max_elapsed, false),
        };
        Self {
            policy,
            started,
            deadline: started + left.min(MAX_BUDGET),
            global_bound,
            attempt: 0,
            rng,
        }
    }

    /// Waits completed so far.
    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.attempt
    }

    /// Time since the loop started.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Sleep one jittered backoff step, polling the global shutdown flag.
    ///
    /// # Errors
    ///
    /// [`RetryStop::Cancelled`] when the run is cancelled, [`RetryStop::DeadlineElapsed`]
    /// when the time budget is spent and [`RetryStop::AttemptsExhausted`] when
    /// the policy allows no more waits. The caller then makes its last decision
    /// (usually a typed timeout error) instead of looping on.
    pub fn wait(&mut self) -> Result<(), RetryStop> {
        self.wait_with(crate::signal::is_global_shutdown)
    }

    /// Like [`Self::wait`], with an explicit cancellation probe.
    ///
    /// # Errors
    ///
    /// Same as [`Self::wait`].
    pub fn wait_with(&mut self, cancelled: impl Fn() -> bool) -> Result<(), RetryStop> {
        if cancelled() {
            return Err(RetryStop::Cancelled);
        }
        if self.attempt >= self.policy.max_attempts {
            return Err(RetryStop::AttemptsExhausted);
        }
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(self.stop_at_deadline(&cancelled));
        }
        // Never sleep past the budget: the last wait is cut at the deadline.
        let delay = self.policy.jittered(self.attempt, &mut self.rng).min(left);
        sleep_sliced(delay, &cancelled)?;
        self.attempt = self.attempt.saturating_add(1);
        Ok(())
    }

    /// Classify the stop at the deadline.
    ///
    /// When the deadline is the global one, the watchdog thread raises the
    /// cancel flag at the same instant, so give it a short grace to do so: the
    /// run then ends as a cancel (exit 124) and not as a lock timeout (exit 83).
    fn stop_at_deadline(&self, cancelled: &impl Fn() -> bool) -> RetryStop {
        if !self.global_bound {
            return RetryStop::DeadlineElapsed;
        }
        let grace = Duration::from_millis(RETRY_GLOBAL_DEADLINE_GRACE_MS);
        match sleep_sliced(grace, cancelled) {
            Err(stop) => stop,
            Ok(()) => RetryStop::DeadlineElapsed,
        }
    }
}

/// Sleep `total` in slices, returning early with `Cancelled` when `cancelled` fires.
fn sleep_sliced(total: Duration, cancelled: &impl Fn() -> bool) -> Result<(), RetryStop> {
    let slice = Duration::from_millis(RETRY_SLICE_MS);
    let end = Instant::now() + total;
    loop {
        if cancelled() {
            return Err(RetryStop::Cancelled);
        }
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(());
        }
        std::thread::sleep(left.min(slice));
    }
}

/// Outcome of one attempt run by [`retry`].
#[derive(Debug)]
pub enum Step<T, E> {
    /// The operation succeeded.
    Done(T),
    /// The operation failed in a way worth retrying.
    Retry(E),
    /// The operation failed for good; do not retry.
    Fail(E),
}

/// How [`retry`] ended without a value.
#[derive(Debug, thiserror::Error)]
pub enum RetryFailure<E> {
    /// The operation reported a permanent failure.
    #[error("operation failed and cannot be retried")]
    Permanent(E),
    /// The retry stopped waiting; `last` is the error of the final attempt.
    #[error("retry stopped after {attempts} waits: {stop}")]
    Stopped {
        /// Why the retry stopped.
        stop: RetryStop,
        /// Error of the last attempt.
        last: E,
        /// Waits completed before the stop.
        attempts: u32,
    },
}

/// Run `op` until it succeeds, fails for good or the policy stops the retry.
///
/// The operation runs once before the first wait. Between attempts the loop
/// waits with [`Backoff::wait`].
///
/// # Errors
///
/// [`RetryFailure::Permanent`] when `op` returns [`Step::Fail`], and
/// [`RetryFailure::Stopped`] when the policy gives up on a [`Step::Retry`].
///
/// # Examples
///
/// ```
/// use atomwrite::retry::{RetryPolicy, Step, retry};
///
/// let mut calls = 0;
/// let value = retry(RetryPolicy::new(1, 2, 1000, 5), || {
///     calls += 1;
///     if calls < 3 {
///         Step::Retry("busy")
///     } else {
///         Step::Done(calls)
///     }
/// })
/// .expect("succeeds on the third call");
/// assert_eq!(value, 3);
/// ```
pub fn retry<T, E>(
    policy: RetryPolicy,
    mut op: impl FnMut() -> Step<T, E>,
) -> Result<T, RetryFailure<E>> {
    let mut backoff = Backoff::new(policy.with_budget_override(&crate::config::tuning()));
    loop {
        let last = match op() {
            Step::Done(value) => return Ok(value),
            Step::Fail(e) => return Err(RetryFailure::Permanent(e)),
            Step::Retry(e) => e,
        };
        if let Err(stop) = backoff.wait() {
            return Err(RetryFailure::Stopped {
                stop,
                last,
                attempts: backoff.attempts(),
            });
        }
    }
}

include!("retry_tests.inc.rs");
