// SPDX-License-Identifier: MIT OR Apache-2.0

//! The sanctioned ways to ignore a failure on purpose.
//!
//! A bare `let _ = fallible();` hides the failure from everyone, including the
//! person debugging it later. [`best_effort`] is for cleanup and other steps
//! whose failure cannot change the result of the command (a leftover tempfile,
//! a probe file that is already gone): the failure is logged at `debug` level on
//! stderr and the run continues. A failure that alters the result of the
//! command must be propagated with `?` instead.
//!
//! Two narrower helpers cover the places where logging is impossible or
//! circular: [`no_channel_left`](crate::best_effort::no_channel_left) for a write to a diagnostic stream that is the
//! last channel there is, and [`settle_error_report`](crate::best_effort::settle_error_report) for the final error
//! envelope of a run that has already failed.

use std::fmt::Display;
use std::io::Write;

/// Log a failed best-effort step and discard it.
///
/// Returns the success value so a caller that wants it can still use it.
/// `ctx` names the step in the log line (for example `"remove probe file"`).
pub fn best_effort<T, E: Display>(result: Result<T, E>, ctx: &str) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::debug!(context = ctx, %error, "best-effort step failed; continuing");
            None
        }
    }
}

/// Hand an event from a producer thread to its consumer.
///
/// A send fails only when the receiver was dropped. A consumer drops its
/// receiver after it raised the shared stop flag (cancel, deadline or its own
/// failure), and every producer already polls that flag before its next file,
/// so an event lost this way belongs to a run that is ending. The loss is the
/// expected outcome of a cancel, not a failure to report. Returns `true` when
/// the event was delivered.
pub fn send_to_consumer<T>(tx: &crossbeam_channel::Sender<T>, event: T) -> bool {
    let delivered = tx.send(event).is_ok();
    if !delivered {
        tracing::trace!("consumer already stopped; event dropped");
    }
    delivered
}

/// Discard the failure of a write to a stream that is the last channel left.
///
/// Used for the log sink itself, for stderr when it is the only place a message
/// can go, and from signal-time code: a failed write there has nowhere else to
/// be reported, and logging it would recurse into the very stream that failed.
/// Do not use it for stdout payload, whose failure must propagate.
pub fn no_channel_left<T, E>(result: Result<T, E>) {
    drop(result.err());
}

/// Settle the exit status of a run whose error envelope was just written.
///
/// The run already failed with `status`. When the envelope could not be
/// delivered because the reader closed the pipe, the contract status for a
/// closed reader (`EXIT_BROKEN_PIPE`, 141) wins: the data channel is gone and
/// no envelope can reach it. Any other write failure is told on stderr, the only
/// channel left, and the original `status` stands.
pub fn settle_error_report(written: anyhow::Result<()>, status: u8) -> u8 {
    let Err(error) = written else {
        return status;
    };
    let closed_reader = matches!(
        error.downcast_ref::<crate::error::AtomwriteError>(),
        Some(crate::error::AtomwriteError::BrokenPipe)
    ) || crate::runtime::find_io_error(&error)
        == Some(std::io::ErrorKind::BrokenPipe);
    if closed_reader {
        return crate::constants::EXIT_BROKEN_PIPE as u8;
    }
    no_channel_left(writeln!(
        std::io::stderr(),
        "atomwrite: could not write the error report: {error:#}"
    ));
    status
}

#[cfg(test)]
mod tests {
    use super::{best_effort, no_channel_left, settle_error_report};

    #[test]
    fn ok_value_is_returned() {
        assert_eq!(best_effort(Ok::<_, String>(7), "step"), Some(7));
    }

    #[test]
    fn error_is_discarded() {
        assert_eq!(best_effort(Err::<u8, _>("boom"), "step"), None);
    }

    #[test]
    fn no_channel_left_accepts_both_outcomes() {
        no_channel_left(Ok::<_, String>(()));
        no_channel_left(Err::<(), _>("gone"));
    }

    #[test]
    fn send_reports_whether_the_consumer_was_there() {
        let (tx, rx) = crossbeam_channel::unbounded::<u8>();
        assert!(super::send_to_consumer(&tx, 1));
        assert_eq!(rx.recv(), Ok(1));
        drop(rx);
        assert!(!super::send_to_consumer(&tx, 2));
    }

    #[test]
    fn delivered_report_keeps_the_status() {
        assert_eq!(settle_error_report(Ok(()), 65), 65);
    }

    #[test]
    fn closed_reader_wins_over_the_status() {
        let closed = std::io::Error::from(std::io::ErrorKind::BrokenPipe);
        assert_eq!(settle_error_report(Err(closed.into()), 65), 141);
        let typed = crate::error::AtomwriteError::BrokenPipe;
        assert_eq!(settle_error_report(Err(typed.into()), 65), 141);
    }

    #[test]
    fn other_write_failure_keeps_the_status() {
        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_eq!(settle_error_report(Err(denied.into()), 65), 65);
    }
}
