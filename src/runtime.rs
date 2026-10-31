// SPDX-License-Identifier: MIT OR Apache-2.0

//! Binary startup helpers: tracing, locale, clap error enrichment, schema prescan.
//!
//! Kept out of `main.rs` so the entrypoint stays parse/dispatch/exit only
//! (rules_rust_cli_com_clap — layout: main ≤ 100 lines).

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tracing_appender::non_blocking::{NonBlockingBuilder, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::util::TryInitError;

/// Initialize UI locale (sys-locale → BCP 47 → fluent-langneg → rust-i18n).
///
/// Delegates to [`crate::locale::init_locale`]. Kept as a thin re-export so
/// `main` / binary startup stay on the `runtime` surface.
#[inline]
pub fn init_locale(lang_override: Option<&str>) {
    crate::locale::init_locale(lang_override);
}

/// Log line format for the tracing fmt layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-oriented compact lines (default for TTY stderr).
    Compact,
    /// JSON lines for aggregators / file sinks.
    Json,
}

/// Resolve CLI `-v`/`-q` to a default `EnvFilter` directive (no env override).
pub fn cli_log_level(verbose: u8, quiet: u8) -> &'static str {
    // Quiet only applies when verbose is unset; `-v -q` keeps the verbose level.
    match (verbose, quiet) {
        (0, 0) => "warn",
        (1, _) => "info",
        (2, _) => "debug",
        (3.., _) => "trace",
        (_, 1) => "error",
        (_, 2..) => "off",
    }
}

/// Build an [`EnvFilter`] from CLI verbosity only (G-007: no product env).
///
/// Precedence: `-v`/`-q` → default. XDG log settings may be added via config
/// later; process environment is never consulted for filter directives.
pub fn build_env_filter(cli_default: &str) -> EnvFilter {
    EnvFilter::new(cli_default)
}

/// Resolve log format: JSON whenever the XDG state directory resolves, compact otherwise.
///
/// G-007: no `ATOMWRITE_LOG_FORMAT` env. Stderr diagnostics stay JSON lines
/// (agent contract) independently of the opt-in file sink (`[log] file`).
pub fn resolve_log_format(file_sink: bool) -> LogFormat {
    if file_sink {
        LogFormat::Json
    } else {
        LogFormat::Compact
    }
}

/// Optional file directory for diagnostics under XDG state (G-007: no env).
///
/// When `atomwrite` state dir is available, logs go to `{state}/logs`.
/// Operators can still use `--verbose` on stderr without a file sink.
fn log_dir_from_xdg() -> Option<PathBuf> {
    crate::storage::state_dir().map(|s| s.join("logs"))
}

/// Default: lossy (latency-first for CLI one-shot). G-007: not env-tunable.
fn log_lossy_default() -> bool {
    true
}

/// Combined stderr (+ optional file) sink for the non-blocking worker.
struct LogSink {
    stderr: io::Stderr,
    file: Option<RollingFileAppender>,
}

impl Write for LogSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Always attempt stderr (agent/container contract). File is best-effort.
        let stderr_res = self.stderr.write_all(buf);
        if let Some(file) = self.file.as_mut() {
            if let Err(e) = file.write_all(buf) {
                self.drop_file_sink("write", &e);
            }
        }
        stderr_res.map(|()| buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stderr.flush()?;
        if let Some(file) = self.file.as_mut() {
            if let Err(e) = file.flush() {
                self.drop_file_sink("flush", &e);
            }
        }
        Ok(())
    }
}

impl LogSink {
    /// Stop using the file sink after a disk error and say so once on stderr.
    ///
    /// The process must not fail on a disk error, and stderr is the only channel
    /// left to report it on, so a failed notice has nowhere further to go.
    fn drop_file_sink(&mut self, step: &str, error: &io::Error) {
        crate::best_effort::no_channel_left(writeln!(
            self.stderr,
            "atomwrite: log file {step} failed: {error}; continuing on stderr only"
        ));
        self.file = None;
    }
}

/// Open the opt-in rotating JSON-lines sink; `None` falls back to stderr only.
///
/// The directory is owner-only (0700) before the appender creates any file, so
/// no other user can read a file in the window before [`restrict_log_files`].
fn try_open_log_file(dir: &Path, rotation: &str) -> Option<RollingFileAppender> {
    let rotation = if rotation == "hourly" {
        Rotation::HOURLY
    } else {
        Rotation::DAILY
    };
    let opened = crate::storage::ensure_dir(dir)
        .map_err(|e| e.to_string())
        .and_then(|()| {
            RollingFileAppender::builder()
                .rotation(rotation)
                .filename_prefix("atomwrite")
                .filename_suffix("jsonl")
                .max_log_files(crate::constants::LOG_MAX_FILES)
                .build(dir)
                .map_err(|e| e.to_string())
        });
    match opened {
        Ok(appender) => {
            restrict_log_files(dir);
            Some(appender)
        }
        Err(e) => {
            crate::best_effort::no_channel_left(writeln!(
                io::stderr(),
                "atomwrite: failed to open log dir {}: {e}; stderr only",
                dir.display()
            ));
            None
        }
    }
}

/// Make the sink files owner-only (0600) on Unix; no-op elsewhere.
#[cfg(unix)]
fn restrict_log_files(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(crate::constants::LOG_FILE_PREFIX)
        {
            if let Err(e) = std::fs::set_permissions(
                entry.path(),
                std::fs::Permissions::from_mode(crate::constants::LOG_FILE_PERMISSIONS),
            ) {
                crate::best_effort::no_channel_left(writeln!(
                    io::stderr(),
                    "atomwrite: could not restrict log file {} to its owner: {e}",
                    entry.path().display()
                ));
            }
        }
    }
}

/// Make the sink files owner-only (0600) on Unix; no-op elsewhere.
#[cfg(not(unix))]
fn restrict_log_files(_dir: &Path) {}

/// Keeps the file-sink writer thread alive until dropped; empty when the sink is off.
#[must_use = "dropping the guard early loses buffered log lines"]
pub struct LogGuard {
    _worker: Option<WorkerGuard>,
}

/// Install the global subscriber over `writer` (JSON lines or compact text).
fn install_subscriber<W>(
    writer: W,
    format: LogFormat,
    filter: EnvFilter,
    ansi: bool,
    show_source: bool,
) -> Result<(), TryInitError>
where
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    use tracing_subscriber::prelude::*;

    match format {
        LogFormat::Json => {
            // Diagnostics and payload are both JSON. They stay separable after
            // a `2>&1` merge because every diagnostic carries `level` and
            // `timestamp` and no payload carries either; the guard for that is
            // in the CLI tests. The formatter keeps its default shape, since
            // `tracing-subscriber` offers no way to inject a constant marker.
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(true)
                .with_writer(writer)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(show_source)
                .with_line_number(show_source);
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .with(tracing_error::ErrorLayer::default())
                .try_init()
        }
        LogFormat::Compact => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_target(true)
                .with_ansi(ansi)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(show_source)
                .with_line_number(show_source)
                .compact();
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .with(tracing_error::ErrorLayer::default())
                .try_init()
        }
    }
}

/// Configure tracing for the binary (local logs only — no product telemetry).
///
/// - Filter: CLI `-v`/`-q` primary; optional XDG log settings (no product env knobs)
/// - Writer: stderr; opt-in rotating tee to the XDG log dir (`[log] file = true`)
/// - Format: JSON lines when the XDG state dir resolves, compact otherwise
/// - Layers: fmt + `ErrorLayer` (`SpanTrace`) + `LogTracer` via `tracing-log` feature
///
/// Returns a [`LogGuard`] that **must** live until process exit so the file
/// sink's background writer flushes (do not bind to `_` alone if early-return
/// paths drop it before final events — `main` keeps `_guard` for the full scope).
///
/// G-008: canonical name is [`init_tracing`] (no product telemetry).
pub fn init_tracing(
    verbose: u8,
    quiet: u8,
    color: ColorMode,
    workspace: &std::path::Path,
    explicit_config: Option<&std::path::Path>,
) -> LogGuard {
    let cli_default = cli_log_level(verbose, quiet);
    let filter = build_env_filter(cli_default);
    let filter_display = filter.to_string();

    let log_dir = log_dir_from_xdg();
    let format = resolve_log_format(log_dir.is_some());
    // R26: the file sink is opt-in (`[log] file = true` in either config layer).
    // A malformed `[log]` cannot be reported as exit 78 here — the subscriber
    // does not exist yet, and this only decides a diagnostic sink. `run` loads
    // the same files through `load_config` and fails the command with 78 there.
    let log_cfg =
        crate::config::startup_log_section(workspace, explicit_config).unwrap_or_else(|error| {
            eprintln!("atomwrite: {error}");
            crate::config::LogSection::default()
        });
    let file = log_dir
        .as_deref()
        .filter(|_| log_cfg.file)
        .and_then(|dir| try_open_log_file(dir, &log_cfg.rotation));
    let file_sink = file.is_some();

    // JSON sinks never ANSI; otherwise CLI/XDG color mode (no process env).
    let ansi = !matches!(format, LogFormat::Json) && stderr_color_enabled(color);

    // File/line only when the *CLI* asked for debug/trace — avoids leaking
    // paths into default warn-level agent runs.
    let show_source = matches!(cli_default, "debug" | "trace");

    // With the sink off (default) diagnostics go straight to stderr: no writer
    // thread and no file. The thread exists only to keep disk I/O off the hot path.
    let (init_result, worker) = match file {
        Some(file) => {
            let sink = LogSink {
                stderr: io::stderr(),
                file: Some(file),
            };
            let (non_blocking, guard) = NonBlockingBuilder::default()
                .lossy(log_lossy_default())
                .thread_name(crate::constants::THREAD_NAME_LOG)
                .finish(sink);
            let result = install_subscriber(non_blocking, format, filter, ansi, show_source);
            (result, Some(guard))
        }
        None => (
            install_subscriber(io::stderr, format, filter, ansi, show_source),
            None,
        ),
    };

    if let Err(e) = init_result {
        // Tests / embedding may install a subscriber first — continue without panic.
        crate::best_effort::no_channel_left(writeln!(
            io::stderr(),
            "atomwrite: tracing subscriber already installed ({e}); keeping existing subscriber"
        ));
    } else {
        // Confirmation of effective filter (rules: log after init). Use info so
        // `-v` shows it; default warn filter still allows this only when raised.
        tracing::info!(
            filter = %filter_display,
            cli_default,
            format = ?format,
            file_sink,
            "tracing initialized"
        );
    }

    LogGuard { _worker: worker }
}

/// Install a panic hook that logs via tracing before the previous hook.
///
/// Must run **after** [`init_tracing`] so the event reaches the subscriber.
/// Chains to the prior hook (`human_panic` when installed first in `main`).
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };

        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()));

        tracing::error!(
            panic.payload = %payload,
            panic.location = location.as_deref().unwrap_or("unknown"),
            "process panicked"
        );

        default_hook(info);
    }));
}

/// Color policy for stderr human messages and tracing ANSI.
///
/// G-007: no process env (`NO_COLOR` / `CLICOLOR_FORCE`). Controlled only by
/// CLI (`--color` / `--no-color`) and optional XDG `ui.color` (wired by caller).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Color when stderr is a TTY.
    #[default]
    Auto,
    /// Always emit ANSI (even on pipes).
    Always,
    /// Never emit ANSI.
    Never,
}

include!("runtime_cli.inc.rs");
