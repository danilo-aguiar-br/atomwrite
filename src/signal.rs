// SPDX-License-Identifier: MIT OR Apache-2.0

//! Signal handling for graceful shutdown on SIGINT and SIGTERM.

#![allow(unsafe_code)]

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use anyhow::{Context, Result};

static GLOBAL_SHUTDOWN: OnceLock<Arc<ShutdownSignal>> = OnceLock::new();

const EXIT_SIGINT: i32 = 130;
const EXIT_SIGTERM: i32 = 143;

/// Thread-safe shutdown coordination for signal-driven graceful exit.
pub struct ShutdownSignal {
    flag: Arc<AtomicBool>,
    count: AtomicU8,
    signal_code: AtomicU8,
}

impl ShutdownSignal {
    /// Return true if a shutdown signal has been received.
    pub fn is_shutdown(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Return the exit code corresponding to the received signal.
    pub fn exit_code(&self) -> u8 {
        match self.signal_code.load(Ordering::SeqCst) {
            143 => 143,
            _ => 130,
        }
    }

    fn record_signal(&self, code: u8) {
        self.signal_code
            .compare_exchange(0, code, Ordering::SeqCst, Ordering::SeqCst)
            .ok();

        let prev = self.count.fetch_add(1, Ordering::SeqCst);
        if prev >= 1 {
            std::process::exit(self.exit_code() as i32);
        }
    }
}

/// Reset SIGPIPE to default disposition for standard Unix CLI behavior.
pub fn reset_sigpipe() {
    #[cfg(unix)]
    {
        // SAFETY: SIG_DFL is a valid signal disposition and SIGPIPE is a standard
        // POSIX signal. Resetting to default prevents Rust's runtime from converting
        // SIGPIPE into a BrokenPipe error, which is the expected Unix CLI behavior.
        unsafe {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        }
    }
}

/// Register SIGINT and SIGTERM handlers and return the shared shutdown signal.
///
/// # Errors
///
/// Returns an I/O error if signal handler registration fails.
pub fn install_handlers() -> Result<Arc<ShutdownSignal>> {
    let flag = Arc::new(AtomicBool::new(false));

    let signal = Arc::new(ShutdownSignal {
        flag: Arc::clone(&flag),
        count: AtomicU8::new(0),
        signal_code: AtomicU8::new(0),
    });

    #[cfg(unix)]
    {
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&flag))
            .context("failed to register SIGINT handler")?;
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&flag))
            .context("failed to register SIGTERM handler")?;

        let sig_int = Arc::clone(&signal);
        // SAFETY: signal_hook::low_level::register requires unsafe because the
        // callback runs in a signal handler context. Our callback only performs
        // atomic operations (compare_exchange, fetch_add on AtomicU8 and
        // process::exit), which are async-signal-safe.
        unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                sig_int.record_signal(EXIT_SIGINT as u8);
            })
            .context("failed to register SIGINT counter")?;
        }

        let sig_term = Arc::clone(&signal);
        // SAFETY: Same as above — only atomic operations in signal context.
        unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGTERM, move || {
                sig_term.record_signal(EXIT_SIGTERM as u8);
            })
            .context("failed to register SIGTERM counter")?;
        }
    }

    GLOBAL_SHUTDOWN.set(Arc::clone(&signal)).ok();

    Ok(signal)
}

/// Return the existing shutdown signal or install handlers if none exists.
///
/// # Errors
///
/// Returns an I/O error if signal handler registration fails on first call.
pub fn get_or_install_handlers() -> Result<Arc<ShutdownSignal>> {
    if let Some(existing) = GLOBAL_SHUTDOWN.get() {
        return Ok(Arc::clone(existing));
    }
    install_handlers()
}
