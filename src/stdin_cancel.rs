// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cancellable stdin: a reader that gives up when the run is cancelled.
//!
//! # Why a thread
//!
//! A blocking `read(2)` on stdin cannot be woken by the global deadline or by
//! a signal on any of the three supported systems:
//!
//! - `signal-hook` installs handlers with `SA_RESTART`, so the kernel restarts
//!   the interrupted `read(2)` instead of returning `EINTR`. The cancel flag is
//!   published, but nothing wakes the reader.
//! - Polling the descriptor (`poll`, `select`) is not portable: Windows pipes
//!   need `PeekNamedPipe` and console handles need a different call, so the
//!   three systems would carry three code paths and `unsafe` blocks.
//!
//! The one portable primitive is a reader thread that owns the blocking read
//! and hands blocks to the caller through a bounded channel. The caller waits
//! on the channel with a short timeout and checks the shutdown flag between
//! waits, so cancellation is observed within `STDIN_POLL_INTERVAL` whatever
//! the descriptor is doing. This uses `std::thread` on purpose, against the
//! general one-shot preference to avoid manual threads: it is the only way to
//! abandon a blocked `read(2)`. The thread is started lazily on the first
//! `read`, so verbs that never touch stdin never spawn it, and a thread stuck
//! in `read(2)` dies with the process (the process always exits right after a
//! cancel). No `unsafe`, no new platform feature.
//!
//! # Contract
//!
//! - Data blocks arrive in order; channel disconnect means end of input.
//! - A read error on the real stdin is delivered once, then input ends.
//! - Once the shutdown flag is set, `read` returns an [`std::io::Error`] that
//!   carries [`crate::error::AtomwriteError::Cancelled`] with the live exit code (124 / 129 /
//!   130 / 143). [`crate::stdin_cancel::surface_cancel`] turns that I/O error back into the typed
//!   error after it travelled through `anyhow` contexts.

use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, bounded};

use crate::error::AtomwriteError;

/// How long one wait on the data channel lasts before the shutdown flag is
/// checked again.
///
/// Bounds the delay between a signal or the deadline and the reader noticing it.
/// Tunable: moves to XDG config in a later step.
const STDIN_POLL_INTERVAL: Duration =
    Duration::from_millis(crate::constants::STDIN_POLL_INTERVAL_MS);

/// Number of blocks the reader thread may queue ahead of the consumer.
///
/// Small on purpose: the thread blocks on a full channel, which applies
/// back-pressure to the pipe instead of buffering stdin in memory.
/// Tunable: moves to XDG config in a later step.
const STDIN_CHANNEL_BLOCKS: usize = crate::constants::STDIN_CHANNEL_BLOCKS;

/// Size in bytes of one block read from the real stdin.
const STDIN_BLOCK_BYTES: usize = crate::constants::STDIN_BLOCK_BYTES;

/// One block read by the thread, or the error that ended the input.
type Block = io::Result<Vec<u8>>;

/// Where the reader is in its life.
enum Source<R> {
    /// Real stdin, thread not started yet.
    Idle(R),
    /// Thread running; blocks arrive on the channel.
    Running(Receiver<Block>),
    /// Input ended (EOF or error already delivered).
    Ended,
}

/// `Read` adapter over a real stdin that stops waiting once the run is cancelled.
pub struct CancellableStdin<R> {
    source: Source<R>,
    cancel: Arc<AtomicBool>,
    /// Block being drained and the offset already served from it.
    current: Vec<u8>,
    offset: usize,
}

impl<R: Read + Send + 'static> CancellableStdin<R> {
    /// Wrap `inner`; `cancel` is the shared shutdown flag
    /// ([`crate::signal::ShutdownSignal::flag`]).
    #[must_use]
    pub fn new(inner: R, cancel: Arc<AtomicBool>) -> Self {
        Self {
            source: Source::Idle(inner),
            cancel,
            current: Vec::new(),
            offset: 0,
        }
    }

    /// Start the reader thread and return the channel it feeds.
    fn start(inner: R) -> io::Result<Receiver<Block>> {
        let (tx, rx) = bounded::<Block>(STDIN_CHANNEL_BLOCKS);
        std::thread::Builder::new()
            .name(crate::constants::THREAD_NAME_STDIN.into())
            .spawn(move || {
                let mut inner = inner;
                let mut block = vec![0u8; STDIN_BLOCK_BYTES];
                loop {
                    let item = match inner.read(&mut block) {
                        Ok(0) => return,
                        Ok(n) => Ok(block[..n].to_vec()),
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(e) => Err(e),
                    };
                    let failed = item.is_err();
                    if tx.send(item).is_err() || failed {
                        return;
                    }
                }
            })?;
        Ok(rx)
    }

    /// Error returned to the consumer once the run is cancelled.
    fn cancelled() -> io::Error {
        io::Error::other(crate::signal::cancelled_error(
            crate::signal::current_cancel_reason(),
        ))
    }

    /// Wait for the next block, checking the shutdown flag between waits.
    ///
    /// Returns `Ok(false)` at end of input.
    fn fill(&mut self) -> io::Result<bool> {
        if let Source::Idle(_) = self.source {
            let Source::Idle(inner) = std::mem::replace(&mut self.source, Source::Ended) else {
                return Ok(false);
            };
            self.source = Source::Running(Self::start(inner)?);
        }
        let Source::Running(rx) = &self.source else {
            return Ok(false);
        };
        let rx = rx.clone();
        loop {
            if self.cancel.load(Ordering::Acquire) {
                return Err(Self::cancelled());
            }
            match rx.recv_timeout(STDIN_POLL_INTERVAL) {
                Ok(Ok(block)) => {
                    self.current = block;
                    self.offset = 0;
                    return Ok(true);
                }
                Ok(Err(e)) => {
                    self.source = Source::Ended;
                    return Err(e);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.source = Source::Ended;
                    return Ok(false);
                }
            }
        }
    }
}

impl<R: Read + Send + 'static> Read for CancellableStdin<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.cancel.load(Ordering::Acquire) {
            return Err(Self::cancelled());
        }
        if self.offset >= self.current.len() && !self.fill()? {
            return Ok(0);
        }
        let n = buf.len().min(self.current.len() - self.offset);
        buf[..n].copy_from_slice(&self.current[self.offset..self.offset + n]);
        self.offset += n;
        Ok(n)
    }
}

/// Recover the typed cancel error from an error that crossed `anyhow` contexts.
///
/// [`CancellableStdin`] reports the cancel as an `io::Error`; readers wrap it
/// with `.context("failed to read stdin")`, which hides the typed variant from
/// `downcast_ref::<AtomwriteError>()`. Any other error is returned unchanged.
#[must_use]
pub fn surface_cancel(err: anyhow::Error) -> anyhow::Error {
    let cancelled = err.chain().find_map(|cause| {
        let inner = cause.downcast_ref::<io::Error>()?.get_ref()?;
        match inner.downcast_ref::<AtomwriteError>()? {
            AtomwriteError::Cancelled { reason, exit } => Some((reason.clone(), *exit)),
            _ => None,
        }
    });
    match cancelled {
        Some((reason, exit)) => AtomwriteError::Cancelled { reason, exit }.into(),
        None => err,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flag(on: bool) -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(on))
    }

    #[test]
    fn passes_bytes_through_and_ends_at_eof() {
        let mut r = CancellableStdin::new(&b"hello world"[..], flag(false));
        let mut out = String::new();
        r.read_to_string(&mut out).expect("read");
        assert_eq!(out, "hello world");
    }

    #[test]
    fn empty_input_is_eof() {
        let mut r = CancellableStdin::new(io::empty(), flag(false));
        let mut out = Vec::new();
        assert_eq!(r.read_to_end(&mut out).expect("read"), 0);
    }

    #[test]
    fn cancelled_flag_yields_typed_error() {
        let mut r = CancellableStdin::new(&b"data"[..], flag(true));
        let err = r.read(&mut [0u8; 8]).expect_err("must be cancelled");
        let surfaced = surface_cancel(anyhow::Error::new(err).context("failed to read stdin"));
        assert!(matches!(
            surfaced.downcast_ref::<AtomwriteError>(),
            Some(AtomwriteError::Cancelled { .. })
        ));
    }

    #[test]
    fn other_errors_are_left_alone() {
        let surfaced = surface_cancel(anyhow::anyhow!("boom"));
        assert_eq!(surfaced.to_string(), "boom");
    }
}
