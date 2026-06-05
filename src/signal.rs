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

/// Install minimal signal handlers as early as possible.
///
/// This registers only the flag-setting handlers via `signal_hook::flag::register`
/// (which is async-signal-safe and uses only atomic operations), without
/// installing the low-level counter handlers or the OnceLock-based shared
/// state. The full installation (with counter tracking and the `OnceLock`
/// `ShutdownSignal`) is performed later by [`install_handlers`], which is
/// safe to call multiple times — the `OnceLock` ensures only one full install
/// takes effect. If a signal arrives between [`install_handlers_early`] and
/// the full [`install_handlers`], the flag is set and the full install
/// observes the flag on the same atomic via the same `Arc`.
///
/// This is critical for tests and real-world scenarios where SIGINT arrives
/// within the first few hundred milliseconds of process startup. Without
/// early installation, the signal hits the default disposition (terminate
/// via signal) before our handlers are registered, and the process exits
/// with status 128+SIGINT (130) without any graceful shutdown.
pub fn install_handlers_early() -> Option<Arc<ShutdownSignal>> {
    // Build the shared signal state with a flag-only handler. This is
    // upgraded in install_handlers by calling signal_hook::low_level::register
    // which is a no-op if already registered.
    let flag = Arc::new(AtomicBool::new(false));
    let signal = Arc::new(ShutdownSignal {
        flag: Arc::clone(&flag),
        count: AtomicU8::new(0),
        signal_code: AtomicU8::new(0),
    });

    #[cfg(unix)]
    {
        let _ = signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&flag));
        let _ = signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&flag));
    }

    // Pre-populate the OnceLock so subsequent get_or_install_handlers() calls
    // return THIS instance (with its already-registered flag handlers) and
    // do not overwrite them. install_handlers checks the OnceLock and
    // returns the existing value if set.
    GLOBAL_SHUTDOWN.set(Arc::clone(&signal)).ok();

    Some(signal)
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
        // callback runs in a signal handler context. We do ONLY async-signal-safe
        // operations here: atomic loads/stores. We do NOT call eprintln! or
        // libc::write(2, ...) because POSIX.1 signal-safety(7) requires handlers
        // to use only functions that are reentrant or atomic w.r.t. signals; the
        // Rust runtime's stdio uses a global Mutex which is not signal-safe, and
        // raw libc::write can race with the child's normal stderr writes. The
        // user-facing "shutting down" message is emitted by the main thread in
        // main.rs when it observes is_shutdown() == true, which is the only
        // async-signal-safe way to guarantee the message reaches the captured
        // stderr pipe before the process exits.
        unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                sig_int.record_signal(EXIT_SIGINT as u8);
            })
            .context("failed to register SIGINT counter")?;
        }

        let sig_term = Arc::clone(&signal);
        // SAFETY: Same as above — atomic operations only.
        unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGTERM, move || {
                sig_term.record_signal(EXIT_SIGTERM as u8);
            })
            .context("failed to register SIGTERM counter")?;
        }
    }

    #[cfg(windows)]
    {
        let flag_win = Arc::clone(&flag);
        let sig_win = Arc::clone(&signal);
        // On Windows, ctrlc::set_handler runs the callback in a normal thread
        // (not signal context), so eprintln! is safe to use here. We still keep
        // the Unix code path async-signal-safe for parity and to satisfy any
        // future platform that might run this in true signal context.
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
