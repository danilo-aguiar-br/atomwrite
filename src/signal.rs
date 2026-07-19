// SPDX-License-Identifier: MIT OR Apache-2.0

//! Signal handling for graceful shutdown on SIGINT and SIGTERM.

#![allow(unsafe_code)]

use std::io::Write;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use anyhow::{Context, Result};

/// Process-wide shutdown coordinator, installed at most once.
///
/// Uses `OnceLock` (not `const` / not `LazyLock`) so the value can be set
/// from `install_handlers_early` or `install_handlers` with runtime data
/// (`Arc<ShutdownSignal>`). Concurrent access is thread-safe: readers use
/// `OnceLock::get`, writers use `OnceLock::set` (first wins). Interior
/// mutability of the flag/count lives in atomics inside `ShutdownSignal`.
static GLOBAL_SHUTDOWN: OnceLock<Arc<ShutdownSignal>> = OnceLock::new();

/// Exit status for SIGINT (128 + 2).
#[cfg_attr(not(unix), allow(dead_code))]
const EXIT_SIGINT: i32 = 130;
/// Exit status for SIGTERM (128 + 15).
#[cfg_attr(not(unix), allow(dead_code))]
const EXIT_SIGTERM: i32 = 143;
/// GNU `timeout(1)` convention: command timed out (one-shot global deadline).
const EXIT_TIMEOUT: u8 = 124;

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

/// Exit code for cooperative cancel (SIGINT 130 / SIGTERM 143 / timeout 124).
///
/// Defaults to **130** when handlers are not installed yet (defensive for
/// library callers that cancel before `install_handlers`).
#[inline]
pub fn current_exit_code() -> u8 {
    GLOBAL_SHUTDOWN
        .get()
        .map(|s| s.exit_code())
        .unwrap_or(130)
}

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

impl ShutdownSignal {
    /// Return true if a shutdown signal has been received.
    ///
    /// Ordering: **Acquire** — pairs with the **Release** store in
    /// [`Self::record_signal`] so observers see a consistent `signal_code`.
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
            143 => 143,
            124 => EXIT_TIMEOUT,
            _ => 130,
        }
    }

    /// True when shutdown was caused by the global `--timeout-secs` deadline.
    #[inline]
    pub fn is_timeout(&self) -> bool {
        self.signal_code.load(Ordering::Acquire) == EXIT_TIMEOUT
    }

    /// Return a clone of the shutdown flag for use in parallel closures.
    #[inline]
    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
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
    // Build the shared signal state with the full handler chain (both the
    // `flag::register` fast-path AND the `low_level::register` counter).
    // Previously this function installed only the flag handlers and
    // `install_handlers` later added the counter handlers — that created
    // two separate `ShutdownSignal` instances whose flags could disagree
    // because `signal_hook`'s global handler table is consulted in
    // registration order, and the second `register` did not retroactively
    // flip the flag of the first instance. Consolidating to a single
    // instance here ensures the search polling inside `atomwrite::run`
    // and the main-thread `is_shutdown()` check observe the same flag.
    let flag = Arc::new(AtomicBool::new(false));
    let signal = Arc::new(ShutdownSignal {
        flag: Arc::clone(&flag),
        count: AtomicU8::new(0),
        signal_code: AtomicU8::new(0),
    });

    #[cfg(unix)]
    {
        // Rules graceful-shutdown: never silently ignore handler registration
        // failure. Tracing is not ready this early — report to stderr only.
        if let Err(e) =
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&flag))
        {
            let _ = writeln!(
                std::io::stderr(),
                "atomwrite: failed to register SIGINT flag handler: {e}"
            );
        }
        if let Err(e) =
            signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&flag))
        {
            let _ = writeln!(
                std::io::stderr(),
                "atomwrite: failed to register SIGTERM flag handler: {e}"
            );
        }

        let sig_int = Arc::clone(&signal);
        // SAFETY: signal_hook::low_level::register requires unsafe because
        // the callback runs in signal context. We do ONLY async-signal-safe
        // operations: atomic loads, stores, and a `_exit` syscall. No
        // eprintln, no libc::write to the user's stderr, no Mutex — the
        // user-facing banner is emitted by the main thread after it
        // observes `is_shutdown() == true`.
        let reg_int = unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                sig_int.record_signal(EXIT_SIGINT as u8);
            })
        };
        if let Err(e) = reg_int {
            let _ = writeln!(
                std::io::stderr(),
                "atomwrite: failed to register SIGINT counter: {e}"
            );
        }

        let sig_term = Arc::clone(&signal);
        // SAFETY: Same contract as SIGINT registration above — callback only
        // performs async-signal-safe atomics via `record_signal`.
        let reg_term = unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGTERM, move || {
                sig_term.record_signal(EXIT_SIGTERM as u8);
            })
        };
        if let Err(e) = reg_term {
            let _ = writeln!(
                std::io::stderr(),
                "atomwrite: failed to register SIGTERM counter: {e}"
            );
        }
    }

    // G-007: readiness probe is CLI `--ready-file` only (see [`write_ready_file`]).
    // Callers (main after clap parse) write the PID after handlers are installed.

    // Pre-populate the OnceLock so subsequent `get_or_install_handlers()`
    // calls return THIS instance (with all of its handlers already
    // registered) and do not overwrite them. `install_handlers` checks
    // the OnceLock and returns the existing value if set.
    GLOBAL_SHUTDOWN.set(Arc::clone(&signal)).ok();

    Some(signal)
}

/// Write process PID to `path` after signal handlers are installed.
///
/// G-007: replaces env `ATOMWRITE_READY_FILE`. Used by tests via CLI
/// `--ready-file` so harnesses wait for handlers without product env knobs.
pub fn write_ready_file(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, std::process::id().to_string());
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

/// Emit the user-facing shutdown banner to stderr from the main thread.
///
/// On Unix this writes directly to fd 2 via `libc::write(2)`, which is the only
/// async-signal-safe way to bypass Rust's userspace stderr buffer when stderr
/// is redirected to a pipe (e.g. `Stdio::piped()` in cargo test, where stderr
/// becomes fully-buffered and `writeln!` output would be lost on process exit).
/// The message is intentionally short and self-contained so the kernel pipe
/// buffer (default 64 KiB on Linux) is never at risk of filling.
///
/// The implementation loops on `EINTR` and `EAGAIN`:
/// - `EINTR` is retried because the kernel may interrupt a slow `write(2)`
///   syscall when other signals arrive concurrently.
/// - `EAGAIN` is retried after a `yield_now` so we never block the main thread
///   waiting for the consumer to drain a full pipe. In practice a 26-byte
///   message fits the default pipe buffer, but the loop makes the call robust
///   to tighter `fcntl(F_SETPIPE_SZ)` limits imposed by some restricted sandboxes.
///
/// On non-Unix platforms this is a no-op; the Windows ctrlc handler in
/// [`install_handlers`] emits the message inline because ctrlc callbacks run in
/// a normal thread (not signal context), so `eprintln!` is safe there.
pub fn write_shutdown_message() {
    #[cfg(unix)]
    {
        const SHUTDOWN_MSG: &[u8] = b"atomwrite: shutting down...\n";
        let mut written = 0usize;
        while written < SHUTDOWN_MSG.len() {
            // SAFETY: STDERR_FILENO is a valid fd constant defined by POSIX and
            // SHUTDOWN_MSG is a `'static` byte slice whose pointer remains valid
            // for the duration of the call. The caller is the main thread, not a
            // signal context, so the only operations that may interrupt the
            // syscall are handled by the EINTR/EAGAIN loop below.
            let ret = unsafe {
                libc::write(
                    libc::STDERR_FILENO,
                    SHUTDOWN_MSG[written..].as_ptr().cast::<libc::c_void>(),
                    SHUTDOWN_MSG.len() - written,
                )
            };
            if ret < 0 {
                let err = std::io::Error::last_os_error();
                match err.raw_os_error() {
                    Some(libc::EINTR) => continue,
                    Some(libc::EAGAIN) => {
                        std::thread::yield_now();
                        continue;
                    }
                    _ => break,
                }
            }
            written += ret as usize;
        }
    }
}

/// Register SIGINT and SIGTERM handlers and return the shared shutdown signal.
///
/// If [`install_handlers_early`] (or any prior call to this function) has
/// already populated the global shutdown slot, this function is a no-op and
/// simply clones the existing `Arc<ShutdownSignal>`. The handlers
/// installed by `install_handlers_early` cover the full surface that
/// `install_handlers` historically added (flag + counter), so a second
/// registration would just install duplicate handlers against a fresh
/// `ShutdownSignal` whose flag would never be flipped by signals the
/// original instance already consumed.
///
/// # Errors
///
/// Returns an I/O error if signal handler registration fails on first
/// install. Subsequent calls never error because they return the existing
/// instance.
pub fn install_handlers() -> Result<Arc<ShutdownSignal>> {
    if let Some(existing) = GLOBAL_SHUTDOWN.get() {
        return Ok(Arc::clone(existing));
    }

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
        let sig_win = Arc::clone(&signal);
        // On Windows, ctrlc::set_handler runs the callback in a normal thread
        // (not signal context), so eprintln! is safe to use here. We still keep
        // the Unix code path async-signal-safe for parity and to satisfy any
        // future platform that might run this in true signal context.
        // Flag publication goes only through `record_signal` (Release store) —
        // single path for interior mutability of the shutdown flag.
        ctrlc::set_handler(move || {
            let was_first = sig_win.count.load(Ordering::Acquire) == 0;
            sig_win.record_signal(EXIT_SIGINT as u8);
            if was_first {
                eprintln!("\natomwrite: shutting down...");
            }
        })
        .context("failed to register Ctrl+C handler")?;
        let _ = &flag; // flag lives inside `signal`; silence unused when only Windows
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

/// Arm a one-shot wall-clock deadline that cooperatively cancels the process.
///
/// When `secs == 0`, this is a no-op (timeout disabled). Otherwise a dedicated
/// background thread sleeps for `secs` and then sets the shared shutdown flag
/// with exit code **124** (GNU `timeout` convention). Long-running subcommands
/// that poll [`ShutdownSignal::is_shutdown`] (search/replace/batch/watch/…)
/// observe the flag and exit cleanly; the main thread maps the exit code.
///
/// # Note
///
/// v0.1.33: fuzzy cascade and `apply_fuzzy_one_pass` poll this flag on each
/// strategy / sliding window so `--timeout-secs` is effective mid-file. Callers
/// with pure CPU work must keep polling; otherwise the deadline only fires after
/// the next cooperative checkpoint.
pub fn arm_global_timeout(secs: u64) {
    if secs == 0 {
        return;
    }
    let Some(sig) = GLOBAL_SHUTDOWN.get().cloned() else {
        tracing::warn!(
            timeout_secs = secs,
            "arm_global_timeout called before handlers installed; deadline ignored"
        );
        return;
    };
    let builder = std::thread::Builder::new().name("atomwrite-timeout".into());
    if let Err(e) = builder.spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(secs));
        if !sig.is_shutdown() {
            tracing::warn!(
                timeout_secs = secs,
                "global --timeout-secs deadline elapsed; cooperative cancel"
            );
            sig.record_signal(EXIT_TIMEOUT);
        }
    }) {
        tracing::warn!(error = %e, "failed to spawn global timeout watchdog");
    }
}

/// Emit the user-facing timeout banner to stderr (main thread only).
pub fn write_timeout_message() {
    eprintln!("atomwrite: global timeout elapsed (exit 124)");
}
