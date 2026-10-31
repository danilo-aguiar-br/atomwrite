// SPDX-License-Identifier: MIT OR Apache-2.0
// Signal handler installation, ready file, banners and global timeout.
// Included from signal.rs.

/// Signals that cancel the run, each with the exit code it reports (128 + signo).
///
/// SIGHUP is here so a closed terminal ends the run through the same
/// cooperative path as SIGTERM, with a `cancelled` envelope, instead of the
/// default action killing the process silently.
#[cfg(unix)]
const CANCEL_SIGNALS: [(i32, u8); 3] = [
    (signal_hook::consts::SIGINT, EXIT_SIGINT as u8),
    (signal_hook::consts::SIGTERM, EXIT_SIGTERM as u8),
    (signal_hook::consts::SIGHUP, EXIT_SIGHUP as u8),
];

/// Register the flag handler and the counter handler for one cancel signal.
///
/// The flag handler raises the cooperative cancel flag; the counter handler
/// records the cause and force-exits on the second hit.
#[cfg(unix)]
fn register_cancel_signal(signal: &Arc<ShutdownSignal>, sig: i32, code: u8) -> Result<()> {
    signal_hook::flag::register(sig, Arc::clone(&signal.flag)).context("flag handler")?;
    let target = Arc::clone(signal);
    // SAFETY: signal_hook::low_level::register requires unsafe because the
    // callback runs in signal context. The callback does ONLY async-signal-safe
    // operations (atomic loads/stores and a `_exit` syscall via `record_signal`):
    // no eprintln, no Mutex, no allocation. The user-facing banner is emitted by
    // the main thread after it observes `is_shutdown() == true`, which is the
    // only way to guarantee the message reaches a captured stderr pipe.
    unsafe {
        signal_hook::low_level::register(sig, move || target.record_signal(code))
            .context("counter handler")?;
    }
    Ok(())
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
        for (sig, code) in CANCEL_SIGNALS {
            if let Err(e) = register_cancel_signal(&signal, sig, code) {
                crate::best_effort::no_channel_left(writeln!(
                    std::io::stderr(),
                    "atomwrite: failed to register handler for signal {sig}: {e:#}"
                ));
            }
        }
        if let Err(e) = register_broken_pipe(&signal) {
            crate::best_effort::no_channel_left(writeln!(
                std::io::stderr(),
                "atomwrite: failed to register the SIGPIPE handler: {e:#}"
            ));
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
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::debug!(error = %e, parent = %parent.display(), "cannot create --ready-file parent");
    }
    // A harness waits for this file; say so when it cannot appear instead of
    // leaving the harness to time out on a silent failure.
    if let Err(e) = std::fs::write(path, std::process::id().to_string()) {
        tracing::warn!(error = %e, path = %path.display(), "cannot write --ready-file");
    }
}

/// Register the SIGPIPE handler: a closed reader ends the run gracefully.
///
/// SIGPIPE is neither ignored nor left at `SIG_DFL`. Ignoring it hides the
/// closed reader from every worker except the one that writes, and the default
/// action kills the process without running a single destructor, which leaves
/// lock sidecars, journals and tempfiles of the writes in flight on disk
/// (measured: 7 to 11 locks and 3 to 7 journals per `replace | head -1`).
/// The handler raises the cooperative cancel flag with exit 141 instead, so
/// the walkers stop at the next file, the write that hit the closed pipe fails
/// with `BrokenPipe` and takes the normal error path, every `Drop` runs and
/// `main` exits 141, the status a shell reports for SIGPIPE.
///
/// Unlike the cancel signals, a repeated SIGPIPE never force-exits: several
/// threads can hit the closed pipe before the first one has unwound, and a
/// forced `_exit` would skip the cleanup this handler exists for.
#[cfg(unix)]
fn register_broken_pipe(signal: &Arc<ShutdownSignal>) -> Result<()> {
    let target = Arc::clone(signal);
    // SAFETY: signal_hook::low_level::register requires unsafe because the
    // callback runs in signal context. The callback does ONLY atomic stores
    // (`record_broken_pipe`): no allocation, no lock, no I/O.
    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGPIPE, move || {
            target.record_broken_pipe();
        })
        .context("SIGPIPE handler")?;
    }
    Ok(())
}

/// Wait until stderr is writable, for at most `max` and one poll slice.
///
/// Used by [`write_shutdown_message`] on `EAGAIN`: the kernel wakes the call as
/// soon as the pipe has room, so the caller sleeps instead of spinning.
#[cfg(unix)]
fn wait_stderr_writable(max: std::time::Duration) {
    let slice = max.min(std::time::Duration::from_millis(
        crate::constants::SHUTDOWN_BANNER_POLL_SLICE_MS,
    ));
    let timeout_ms = i32::try_from(slice.as_millis()).unwrap_or(i32::MAX).max(1);
    let mut fd = libc::pollfd {
        fd: libc::STDERR_FILENO,
        events: libc::POLLOUT,
        revents: 0,
    };
    // SAFETY: `fd` is a valid, initialised `pollfd` that outlives the call and
    // `nfds` is 1, matching the single entry. The call only reads `fd` and
    // writes `revents`. Its result is not needed: on timeout or `EINTR` the
    // caller retries the write and re-checks its own deadline.
    unsafe {
        libc::poll(&mut fd, 1, timeout_ms);
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
/// - `EAGAIN` (a full non-blocking pipe) waits in `poll(2)` slices until stderr is
///   writable again, for at most [`crate::constants::SHUTDOWN_BANNER_MAX_WAIT_MS`].
///   The wait is event-driven, so the loop never spins a core, and it is bounded,
///   so a consumer that never drains the pipe cannot hold the shutdown back.
///   In practice a 26-byte message fits the default pipe buffer, but the loop
///   makes the call robust to tighter `fcntl(F_SETPIPE_SZ)` limits imposed by
///   some restricted sandboxes.
///
/// On non-Unix platforms this is a no-op; the Windows ctrlc handler in
/// [`install_handlers`] emits the message inline because ctrlc callbacks run in
/// a normal thread (not signal context), so `eprintln!` is safe there.
pub fn write_shutdown_message() {
    // A closed reader is not a user interruption: there is nothing to announce.
    if is_global_broken_pipe() {
        return;
    }
    #[cfg(unix)]
    {
        // The caller is the main thread, so building the localized text here
        // allocates safely; only the write itself must bypass the stderr buffer.
        let banner = format!("{}\n", rust_i18n::t!("cancel.shutting-down"));
        let shutdown_msg = banner.as_bytes();
        let mut written = 0usize;
        let give_up_at = std::time::Instant::now()
            + std::time::Duration::from_millis(crate::constants::SHUTDOWN_BANNER_MAX_WAIT_MS);
        while written < shutdown_msg.len() {
            // SAFETY: STDERR_FILENO is a valid fd constant defined by POSIX and
            // `shutdown_msg` borrows `banner`, which outlives the call. The
            // caller is the main thread, not a signal context, so the only
            // operations that may interrupt the syscall are handled by the
            // EINTR/EAGAIN loop below.
            let ret = unsafe {
                libc::write(
                    libc::STDERR_FILENO,
                    shutdown_msg[written..].as_ptr().cast::<libc::c_void>(),
                    shutdown_msg.len() - written,
                )
            };
            if ret < 0 {
                let err = std::io::Error::last_os_error();
                match err.raw_os_error() {
                    Some(libc::EINTR) => continue,
                    Some(libc::EAGAIN) => {
                        let left = give_up_at.saturating_duration_since(std::time::Instant::now());
                        if left.is_zero() {
                            break;
                        }
                        wait_stderr_writable(left);
                        continue;
                    }
                    _ => break,
                }
            }
            written += ret as usize;
        }
    }
}

/// Register SIGINT, SIGTERM and SIGHUP handlers and return the shared shutdown signal.
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
        for (sig, code) in CANCEL_SIGNALS {
            register_cancel_signal(&signal, sig, code)
                .with_context(|| format!("failed to register handler for signal {sig}"))?;
        }
        register_broken_pipe(&signal)?;
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
                eprintln!("\n{}", rust_i18n::t!("cancel.shutting-down"));
            }
        })
        .context("failed to register Ctrl+C handler")?;
        // No SIGPIPE on Windows: a closed pipe arrives as `ERROR_BROKEN_PIPE`,
        // which `std` maps to `BrokenPipe`, and the run unwinds through the same
        // error path and exit 141 as on Unix.
    }

    #[cfg(not(any(unix, windows)))]
    {
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
    record_global_deadline(secs);
    let builder = std::thread::Builder::new().name(crate::constants::THREAD_NAME_TIMEOUT.into());
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
    eprintln!("{}", rust_i18n::t!("cancel.timeout-elapsed"));
}
