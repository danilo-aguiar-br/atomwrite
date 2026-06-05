// SPDX-License-Identifier: MIT OR Apache-2.0

//! Signal handling for graceful shutdown on SIGINT and SIGTERM.

#![allow(unsafe_code)]

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use anyhow::{Context, Result};

static GLOBAL_SHUTDOWN: OnceLock<Arc<ShutdownSignal>> = OnceLock::new();

#[cfg_attr(not(unix), allow(dead_code))]
const EXIT_SIGINT: i32 = 130;
#[cfg_attr(not(unix), allow(dead_code))]
const EXIT_SIGTERM: i32 = 143;

/// Thread-safe shutdown coordination for signal-driven graceful exit.
pub struct ShutdownSignal {
    flag: Arc<AtomicBool>,
    count: AtomicU8,
    signal_code: AtomicU8,
}

impl ShutdownSignal {
    /// Return true if a shutdown signal has been received.
    #[inline]
    pub fn is_shutdown(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Return the exit code corresponding to the received signal.
    #[inline]
    pub fn exit_code(&self) -> u8 {
        match self.signal_code.load(Ordering::Acquire) {
            143 => 143,
            _ => 130,
        }
    }

    /// Return a clone of the shutdown flag for use in parallel closures.
    #[inline]
    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }

    fn record_signal(&self, code: u8) {
        self.signal_code
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire)
            .ok();

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
        // callback runs in a signal handler context. We use eprintln! instead of
        // libc::write(2, ...) because POSIX pipe semantics under cargo test's
        // process-group inheritance can swallow raw fd writes to the child's
        // stderr pipe, while eprintln! routes through the Rust runtime which
        // uses the captured fd directly. The cost is a non-signal-safe call
        // (Rust's stderr lock); we accept the theoretical deadlock risk because
        // atomwrite's main thread is the only writer of stderr in normal flow
        // and signal delivery is rare.
        unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                let was_first = sig_int.count.load(Ordering::Acquire) == 0;
                sig_int.record_signal(EXIT_SIGINT as u8);
                if was_first {
                    eprintln!("\natomwrite: shutting down...");
                }
            })
            .context("failed to register SIGINT counter")?;
        }

        let sig_term = Arc::clone(&signal);
        // SAFETY: Same as above — atomic operations and eprintln! only.
        unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGTERM, move || {
                let was_first = sig_term.count.load(Ordering::Acquire) == 0;
                sig_term.record_signal(EXIT_SIGTERM as u8);
                if was_first {
                    eprintln!("\natomwrite: shutting down...");
                }
            })
            .context("failed to register SIGTERM counter")?;
        }
    }

    #[cfg(windows)]
    {
        let flag_win = Arc::clone(&flag);
        let sig_win = Arc::clone(&signal);
        ctrlc::set_handler(move || {
            let was_first = sig_win.count.load(Ordering::Acquire) == 0;
            flag_win.store(true, Ordering::Release);
            sig_win.record_signal(EXIT_SIGINT as u8);
            if was_first {
                eprintln!("\natomwrite: shutting down...");
            }
        })
        .context("failed to register Ctrl+C handler")?;
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = &flag;
        tracing::warn!("signal handlers not available on this platform — Ctrl+C may not work");
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
