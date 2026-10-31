// SPDX-License-Identifier: MIT OR Apache-2.0

//! Signal handling for graceful shutdown on SIGINT, SIGTERM, SIGHUP and SIGPIPE.

#![allow(unsafe_code)]

#[cfg(unix)]
use std::io::Write;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use anyhow::{Context, Result};

use crate::constants::{EXIT_BROKEN_PIPE, EXIT_SIGHUP, EXIT_SIGINT, EXIT_SIGTERM};
use crate::error::AtomwriteError;

/// Bytes of work between two cooperative-cancellation checks.
///
/// A "block" bounds how long a scan loop can run past the global deadline
/// before it observes the cancel flag. Smaller values tighten the deadline;
/// the check itself is a single `Acquire` load, so the cost is one atomic
/// load per block — no allocation and no syscall per check.
pub const CANCEL_BLOCK_BYTES: u64 = crate::constants::CANCEL_BLOCK_BYTES;

/// Matches between two cooperative-cancellation checks.
///
/// Used by loops that iterate matches instead of bytes. Keeps the delay
/// between the deadline firing and the loop noticing it bounded by
/// `CANCEL_BLOCK_MATCHES` matches rather than by the whole file.
pub const CANCEL_BLOCK_MATCHES: u64 = crate::constants::CANCEL_BLOCK_MATCHES;

/// Reason reported when a cooperative check observes a cancelled run.
const CANCEL_REASON: &str = "cancelled by signal or --timeout-secs";

/// Process-wide shutdown coordinator, installed at most once.
///
/// Uses `OnceLock` (not `const` / not `LazyLock`) so the value can be set
/// from `install_handlers_early` or `install_handlers` with runtime data
/// (`Arc<ShutdownSignal>`). Concurrent access is thread-safe: readers use
/// `OnceLock::get`, writers use `OnceLock::set` (first wins). Interior
/// mutability of the flag/count lives in atomics inside `ShutdownSignal`.
static GLOBAL_SHUTDOWN: OnceLock<Arc<ShutdownSignal>> = OnceLock::new();

/// Instant at which the cooperative cancel flag was first raised.
///
/// A cancel checkpoint that observes the flag can only say how late it was if
/// the moment of the raise is known. Recording it once turns "the deadline
/// overshot" into a measured distance instead of an inference, which is what
/// the cooperative-cancel rule of ADR-0036 requires.
static SHUTDOWN_RAISED_AT: OnceLock<std::time::Instant> = OnceLock::new();

/// Record the moment the cancel flag was raised. The first raise wins.
fn note_shutdown_raised() {
    SHUTDOWN_RAISED_AT.get_or_init(std::time::Instant::now);
}

/// How long ago the cancel flag was raised, or `None` while the run is live.
#[must_use]
pub fn shutdown_raised_ago() -> Option<std::time::Duration> {
    SHUTDOWN_RAISED_AT.get().map(std::time::Instant::elapsed)
}

/// GNU `timeout(1)` convention: command timed out (one-shot global deadline).
const EXIT_TIMEOUT: u8 = crate::constants::EXIT_TIMEOUT as u8;

/// Thread-safe shutdown coordination for signal-driven graceful exit.
pub struct ShutdownSignal {
    flag: Arc<AtomicBool>,
    count: AtomicU8,
    signal_code: AtomicU8,
}

/// Return true if the process-wide shutdown flag is set (v0.1.29 cooperative cancel).
#[inline]
pub fn is_global_shutdown() -> bool {
    GLOBAL_SHUTDOWN.get().is_some_and(|s| s.is_shutdown())
}

/// True when a cancel that must abort work in flight is in progress.
///
/// Like [`is_global_shutdown`], except that a closed reader (SIGPIPE) does not
/// count: stopping new work is right, but a local write already under way is
/// short and harmless to finish, and aborting it would leave its journal behind.
#[inline]
pub fn is_global_hard_shutdown() -> bool {
    GLOBAL_SHUTDOWN.get().is_some_and(|s| s.is_hard_shutdown())
}

/// True when the cooperative cancel in progress was caused by a closed reader.
#[inline]
pub fn is_global_broken_pipe() -> bool {
    GLOBAL_SHUTDOWN.get().is_some_and(|s| s.is_broken_pipe())
}

/// Exit code for cooperative cancel (SIGINT 130 / SIGTERM 143 / SIGPIPE 141 / timeout 124).
///
/// Defaults to **130** when handlers are not installed yet (defensive for
/// library callers that cancel before `install_handlers`).
#[inline]
pub fn current_exit_code() -> u8 {
    GLOBAL_SHUTDOWN
        .get()
        .map(|s| s.exit_code())
        .unwrap_or(EXIT_SIGINT as u8)
}

/// Human-readable reason for the cancel in progress, from the live signal.
///
/// The deadline and a signal are different causes and must not share one
/// sentence. Call sites that wrote a fixed string — "cancelled by signal" —
/// stated the cause they assumed rather than the one observed, and were wrong
/// whenever the global `--timeout-secs` deadline was what fired. This reads the
/// recorded cause instead.
///
/// Falls back to `CANCEL_REASON` when handlers are not installed yet, which
/// is the same generic text the cooperative check points already report.
#[inline]
pub fn current_cancel_reason() -> &'static str {
    GLOBAL_SHUTDOWN
        .get()
        .map(|s| s.cancel_reason())
        .unwrap_or(CANCEL_REASON)
}

include!("signal_cancel.inc.rs");

impl ShutdownSignal {
    /// Return true if a shutdown signal has been received.
    ///
    /// Ordering: **Acquire** — pairs with the **Release** store in
    /// `record_signal` so observers see a consistent `signal_code`.
    #[inline]
    pub fn is_shutdown(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Return the exit code corresponding to the received signal or timeout.
    ///
    /// Ordering: **Acquire** on `signal_code` (published via `AcqRel` CAS).
    #[inline]
    pub fn exit_code(&self) -> u8 {
        match self.signal_code.load(Ordering::Acquire) {
            c if c == EXIT_SIGTERM as u8
                || c == EXIT_SIGHUP as u8
                || c == EXIT_BROKEN_PIPE as u8
                || c == EXIT_TIMEOUT =>
            {
                c
            }
            _ => EXIT_SIGINT as u8,
        }
    }

    /// True when a cancel that must abort work in flight was recorded: any
    /// cancel except a closed reader.
    #[inline]
    pub fn is_hard_shutdown(&self) -> bool {
        self.is_shutdown() && !self.is_broken_pipe()
    }

    /// True when shutdown was caused by SIGPIPE (the reader of stdout closed).
    #[inline]
    pub fn is_broken_pipe(&self) -> bool {
        self.signal_code.load(Ordering::Acquire) == EXIT_BROKEN_PIPE as u8
    }

    /// True when shutdown was caused by the global `--timeout-secs` deadline.
    #[inline]
    pub fn is_timeout(&self) -> bool {
        self.signal_code.load(Ordering::Acquire) == EXIT_TIMEOUT
    }

    /// Signal name for the recorded cancel, when one was caught.
    ///
    /// Returns `None` for the global deadline, which is enforced by
    /// `--timeout-secs` rather than by a signal, and `None` before any cancel
    /// was recorded. A name is only ever produced for SIGINT / SIGTERM / SIGHUP /
    /// SIGPIPE.
    #[inline]
    #[must_use]
    pub fn signal_name(&self) -> Option<&'static str> {
        match self.signal_code.load(Ordering::Acquire) {
            c if c == EXIT_SIGINT as u8 => Some("SIGINT"),
            c if c == EXIT_SIGTERM as u8 => Some("SIGTERM"),
            c if c == EXIT_SIGHUP as u8 => Some("SIGHUP"),
            c if c == EXIT_BROKEN_PIPE as u8 => Some("SIGPIPE"),
            _ => None,
        }
    }

    /// Human-readable reason for the recorded cancel.
    ///
    /// Kept next to [`Self::is_timeout`] so the `cancelled` envelope and the
    /// exit code can never describe different causes.
    #[inline]
    #[must_use]
    pub fn cancel_reason(&self) -> &'static str {
        if self.is_timeout() {
            "global --timeout-secs deadline elapsed"
        } else if self.is_broken_pipe() {
            "the reader of stdout closed the pipe"
        } else {
            "cancelled by signal"
        }
    }

    /// Return a clone of the shutdown flag for use in parallel closures.
    #[inline]
    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }

    /// Record a closed reader (SIGPIPE) as a cooperative cancel with exit 141.
    ///
    /// Raises the flag so every worker stops at its next check, but never counts
    /// toward the force-exit of [`Self::record_signal`]: more than one thread
    /// can hit the closed pipe, and a forced `_exit` would skip the destructors
    /// this path exists to run. Async-signal-safe: atomic stores only. A cancel
    /// already recorded (SIGINT, SIGTERM, timeout) keeps its own code.
    ///
    /// Only the Unix SIGPIPE handler and the tests below call this, so on
    /// Windows the method would be dead code.
    #[cfg(unix)]
    fn record_broken_pipe(&self) {
        self.signal_code
            .compare_exchange(
                0,
                EXIT_BROKEN_PIPE as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .ok();
        self.flag.store(true, Ordering::Release);
    }

    /// Record a cooperative cancel (signal or timeout).
    ///
    /// Orderings (Rules Rust atomics — documented per op):
    /// - `signal_code` **`AcqRel`** CAS: first writer wins; subsequent signals
    ///   keep the original code (SIGINT vs SIGTERM vs timeout).
    /// - `flag` **Release** store: publishes shutdown to walkers that **Acquire**.
    /// - `count` **`AcqRel`** `fetch_add`: second hit force-exits; must observe prior.
    fn record_signal(&self, code: u8) {
        self.signal_code
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire)
            .ok();
        // Always raise the cooperative cancel flag. Signal handlers also flip
        // the flag via `signal_hook::flag::register` on the same `Arc`, but
        // timeout / programmatic cancel only go through this method.
        note_shutdown_raised();
        self.flag.store(true, Ordering::Release);

        let prev = self.count.fetch_add(1, Ordering::AcqRel);
        if prev >= 1 {
            #[cfg(unix)]
            {
                // SAFETY: libc::_exit is async-signal-safe unlike std::process::exit
                // which runs destructors and flushes stdio. Second Ctrl+C is force-kill.
                unsafe { libc::_exit(self.exit_code() as i32) };
            }
            #[cfg(not(unix))]
            {
                std::process::exit(self.exit_code() as i32);
            }
        }
    }
}

include!("signal_handlers.inc.rs");

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> ShutdownSignal {
        ShutdownSignal {
            flag: Arc::new(AtomicBool::new(false)),
            count: AtomicU8::new(0),
            signal_code: AtomicU8::new(0),
        }
    }

    #[cfg(unix)]
    #[test]
    fn broken_pipe_stops_new_work_but_not_the_write_in_flight() {
        let signal = fresh();
        assert!(!signal.is_shutdown() && !signal.is_hard_shutdown());
        signal.record_broken_pipe();
        // New work stops (the per-file checks use `is_shutdown`) ...
        assert!(signal.is_shutdown());
        // ... but the write already under way is not aborted.
        assert!(!signal.is_hard_shutdown());
        assert!(signal.is_broken_pipe());
        assert_eq!(signal.exit_code(), EXIT_BROKEN_PIPE as u8);
        assert_eq!(signal.signal_name(), Some("SIGPIPE"));
    }

    #[cfg(unix)]
    #[test]
    fn repeated_broken_pipe_never_force_exits() {
        let signal = fresh();
        for _ in 0..5 {
            signal.record_broken_pipe();
        }
        assert_eq!(signal.count.load(Ordering::Acquire), 0);
        assert!(signal.is_broken_pipe());
    }

    #[test]
    fn every_other_cancel_still_aborts_the_write_in_flight() {
        for code in [
            EXIT_SIGINT as u8,
            EXIT_SIGTERM as u8,
            EXIT_SIGHUP as u8,
            EXIT_TIMEOUT,
        ] {
            let signal = fresh();
            signal.record_signal(code);
            assert!(signal.is_hard_shutdown(), "code {code}");
            assert!(!signal.is_broken_pipe(), "code {code}");
            assert_eq!(signal.exit_code(), code);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_cancel_recorded_first_keeps_its_own_code_over_a_broken_pipe() {
        let signal = fresh();
        signal.record_signal(EXIT_SIGTERM as u8);
        signal.record_broken_pipe();
        assert_eq!(signal.exit_code(), EXIT_SIGTERM as u8);
        assert!(signal.is_hard_shutdown());
    }
}
