// SPDX-License-Identifier: MIT OR Apache-2.0

//! Windows tempfile persist with retry backoff (A-022).
//!
//! Another process holding the target open (an editor, an antivirus scan, an
//! indexer) makes `MoveFileEx` fail with `ERROR_ACCESS_DENIED` or
//! `ERROR_SHARING_VIOLATION` for a short while. The persist is retried through
//! [`crate::retry`]: full-jitter backoff, a time budget capped by the global
//! deadline, and cancellation polled between sleeps.

#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
use anyhow::Result;

#[cfg(windows)]
use tempfile::NamedTempFile;

#[cfg(windows)]
use crate::error::AtomwriteError;
#[cfg(windows)]
use crate::retry::{RetryFailure, RetryPolicy, RetryStop, Step, retry};

/// True when a persist error is the transient "someone has the file open" case.
#[cfg(windows)]
fn is_transient_persist_error(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::PermissionDenied
        || e.raw_os_error() == Some(crate::constants::WINDOWS_ERROR_SHARING_VIOLATION)
}

/// Persist the tempfile onto `target`, retrying the transient share violation.
///
/// # Errors
///
/// Returns a permanent I/O error when the persist fails for a reason other
/// than `ERROR_ACCESS_DENIED` / `ERROR_SHARING_VIOLATION`, or when the retry
/// budget in [`crate::constants::PERSIST_RETRY_MAX_ELAPSED_MS`] runs out.
/// Returns [`crate::signal::cancelled_error`] (130/143/124) when a shutdown
/// arrives between attempts, and
/// [`crate::error::AtomwriteError::PermissionDenied`] (exit 13) when the
/// budget stops for any other reason.
#[cfg(windows)]
pub(crate) fn persist_with_retry(temp: NamedTempFile, target: &Path) -> Result<()> {
    let policy = RetryPolicy::new(
        crate::constants::PERSIST_RETRY_BASE_MS,
        crate::constants::PERSIST_RETRY_CAP_MS,
        crate::constants::PERSIST_RETRY_MAX_ELAPSED_MS,
        crate::constants::PERSIST_RETRY_MAX_ATTEMPTS,
    );
    // `persist` consumes the tempfile and hands it back inside the error, so the
    // slot carries it from one attempt to the next.
    let mut slot = Some(temp);
    let outcome = retry(policy, || {
        let Some(file) = slot.take() else {
            return Step::Fail(std::io::Error::other(
                "tempfile lost between persist attempts",
            ));
        };
        match file.persist(target) {
            Ok(_) => Step::Done(()),
            Err(e) => {
                let transient = is_transient_persist_error(&e.error);
                slot = Some(e.file);
                if transient {
                    Step::Retry(e.error)
                } else {
                    Step::Fail(e.error)
                }
            }
        }
    });
    match outcome {
        Ok(()) => Ok(()),
        Err(RetryFailure::Permanent(e)) => Err(crate::error::AtomwriteError::from_io_at(
            target,
            std::io::Error::new(
                e.kind(),
                format!("rename error for {}: {e}", target.display()),
            ),
        )
        .into()),
        Err(RetryFailure::Stopped {
            stop: RetryStop::Cancelled,
            ..
        }) => Err(crate::signal::cancelled_error(format!(
            "persist wait cancelled for {}",
            target.display()
        ))
        .into()),
        Err(RetryFailure::Stopped { .. }) => Err(AtomwriteError::PermissionDenied {
            path: target.to_path_buf(),
        }
        .into()),
    }
}
