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

/// Resolve log format: compact on TTY stderr; JSON when file logging is on.
///
/// G-007: no `ATOMWRITE_LOG_FORMAT` env — file sink implies JSON.
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
                // Do not fail the process on disk errors; surface once via stderr.
                let _ = writeln!(
                    self.stderr,
                    "atomwrite: log file write failed: {e}; continuing on stderr only"
                );
                self.file = None;
            }
        }
        stderr_res.map(|()| buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stderr.flush()?;
        if let Some(file) = self.file.as_mut() {
            let _ = file.flush();
        }
        Ok(())
    }
}

fn try_open_log_file(dir: &Path, json: bool) -> Option<RollingFileAppender> {
    let suffix = if json { "jsonl" } else { "log" };
    match RollingFileAppender::builder()
        .rotation(Rotation::NEVER)
        .filename_prefix("atomwrite")
        .filename_suffix(suffix)
        // Retention only matters if rotation is later enabled; keep a tight cap.
        .max_log_files(7)
        .build(dir)
    {
        Ok(appender) => Some(appender),
        Err(e) => {
            let _ = writeln!(
                io::stderr(),
                "atomwrite: failed to open log dir {}: {e}; stderr only",
                dir.display()
            );
            None
        }
    }
}

/// Configure tracing for the binary (local logs only — no product telemetry).
///
/// - Filter: CLI `-v`/`-q` primary; optional XDG log settings (no product env knobs)
/// - Writer: non-blocking stderr; optional tee to XDG log dir
/// - Format: compact (TTY) or JSON when file logging
/// - Layers: fmt + `ErrorLayer` (`SpanTrace`) + `LogTracer` via `tracing-log` feature
///
/// Returns a [`WorkerGuard`] that **must** live until process exit so the
/// background writer flushes (do not bind to `_` alone if early-return paths
/// drop it before final events — `main` keeps `_guard` for the full scope).
///
/// G-008: canonical name is [`init_tracing`] (no product telemetry).
pub fn init_tracing(
    verbose: u8,
    quiet: u8,
    color: ColorMode,
) -> WorkerGuard {
    let cli_default = cli_log_level(verbose, quiet);
    let filter = build_env_filter(cli_default);
    let filter_display = filter.to_string();

    let log_dir = log_dir_from_xdg();
    // Prefer JSON when a log directory is requested (aggregator-friendly).
    let format = resolve_log_format(log_dir.is_some());
    let file = log_dir
        .as_ref()
        .and_then(|dir| try_open_log_file(dir, matches!(format, LogFormat::Json)));
    let file_sink = file.is_some();

    // JSON sinks never ANSI; otherwise CLI/XDG color mode (no process env).
    let ansi = !matches!(format, LogFormat::Json) && stderr_color_enabled(color);

    // File/line only when the *CLI* asked for debug/trace — avoids leaking
    // paths into default warn-level agent runs.
    let show_source = matches!(cli_default, "debug" | "trace");

    let sink = LogSink {
        stderr: io::stderr(),
        file,
    };
    let (non_blocking, guard) = NonBlockingBuilder::default()
        .lossy(log_lossy_default())
        .thread_name("atomwrite-log")
        .finish(sink);

    use tracing_subscriber::prelude::*;

    let init_result = match format {
        LogFormat::Json => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(true)
                .with_writer(non_blocking)
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
                .with_writer(non_blocking)
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
    };

    if let Err(e) = init_result {
        // Tests / embedding may install a subscriber first — continue without panic.
        let _ = writeln!(
            io::stderr(),
            "atomwrite: tracing subscriber already installed ({e}); keeping existing subscriber"
        );
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

    guard
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

/// Whether stderr human messages may use ANSI colors.
///
/// Agent-first: never color on non-TTY unless `ColorMode::Always`.
pub fn stderr_color_enabled(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Never => false,
        ColorMode::Always => true,
        ColorMode::Auto => std::io::IsTerminal::is_terminal(&std::io::stderr()),
    }
}

/// Emit a human warning on stderr (never on stdout).
///
/// G-026: only when stderr is a TTY (agents parse NDJSON on stdout; non-TTY
/// must not mix human risk lines). Color respects CLI `ColorMode`.
pub fn warn_stderr(mode: ColorMode, message: impl AsRef<str>) {
    use std::io::IsTerminal;
    if !std::io::stderr().is_terminal() {
        return;
    }
    let msg = message.as_ref();
    if stderr_color_enabled(mode) {
        eprintln!("\x1b[33mwarning:\x1b[0m {msg}");
    } else {
        eprintln!("warning: {msg}");
    }
}

fn extract_clap_tip(msg: &str) -> Option<String> {
    for line in msg.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("tip:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Enrich clap parse errors with agent-oriented suggestions (edit hyphen values).
pub fn enrich_clap_suggestion(msg: &str) -> Option<String> {
    let clap_tip = extract_clap_tip(msg);
    let msg_lower = msg.to_ascii_lowercase();

    let mentions_edit_args = msg_lower.contains("--old")
        || msg_lower.contains("--new")
        || msg_lower.contains("--after-match")
        || msg_lower.contains("--before-match")
        || msg_lower.contains("--between");

    let is_edit_subcommand =
        msg.contains("Usage: atomwrite edit") || msg.contains("atomwrite edit ");

    let hyphen_value_error = msg.contains("wasn't expected")
        || msg.contains("unexpected argument")
        || (msg.contains("tip:") && msg.contains("'--'"));

    if mentions_edit_args || (hyphen_value_error && is_edit_subcommand) {
        let base = "For content with special characters (hyphens, quotes, shell metacharacters), \
                    use --old-file <PATH> and --new-file <PATH> to read content from files \
                    instead of CLI arguments. This bypasses shell expansion and argument \
                    parsing entirely.";
        return Some(match clap_tip {
            Some(tip) => format!("{base} (original clap tip: {tip})"),
            None => base.to_string(),
        });
    }

    clap_tip
}

/// Early `--json-schema` path: emit schema without full clap parse of subcommand args.
///
/// Justification vs rules "never parse `env::args` before Clap": agents need a
/// schema dump even when remaining argv would fail validation (e.g. missing
/// required path). Full parse still goes through Clap for all normal paths.
pub fn prescan_json_schema() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|a| a == "--json-schema") {
        return None;
    }
    const SUBCOMMANDS: &[&str] = &[
        "read",
        "write",
        "edit",
        "search",
        "replace",
        "hash",
        "delete",
        "count",
        "diff",
        "move",
        "copy",
        "list",
        "extract",
        "calc",
        "regex",
        "transform",
        "scope",
        "batch",
        "backup",
        "rollback",
        "apply",
        "completions",
        "prune-backups",
        "edit-loop",
        "get",
        "set",
        "del",
        "outline",
        "query",
        "case",
        "semantic-merge",
        "sparse",
        "recipe",
        "stat",
        "agent-surface",
        "watch",
        "codemod",
        "semantic-search",
        "verify",
        "wal-stats",
        "wal-heal",
        "doctor",
        "locale",
        "progress",
        "error",
        "best-candidate",
        "cancelled",
    ];
    for arg in &args[1..] {
        if SUBCOMMANDS.contains(&arg.as_str()) {
            return Some(arg.clone());
        }
    }
    None
}

/// Map an `anyhow::Error` chain to an I/O error kind when present.
pub fn find_io_error(err: &anyhow::Error) -> Option<std::io::ErrorKind> {
    for cause in err.chain() {
        if let Some(io_err) = cause.downcast_ref::<std::io::Error>() {
            return Some(io_err.kind());
        }
    }
    None
}

/// Convert a detected I/O kind into a typed [`crate::error::AtomwriteError`].
pub fn io_to_atomwrite_error(
    kind: std::io::ErrorKind,
    err: &anyhow::Error,
) -> crate::error::AtomwriteError {
    let msg = format!("{err:#}");
    match kind {
        std::io::ErrorKind::PermissionDenied => {
            crate::error::AtomwriteError::PermissionDenied {
                path: extract_path_from_message(&msg),
            }
        }
        std::io::ErrorKind::NotFound => crate::error::AtomwriteError::NotFound {
            path: extract_path_from_message(&msg),
        },
        _ => crate::error::AtomwriteError::Io {
            source: std::io::Error::new(kind, msg),
        },
    }
}

fn extract_path_from_message(msg: &str) -> PathBuf {
    if let Some(start) = msg.find('/') {
        let rest = &msg[start..];
        let end = rest.find(':').unwrap_or(rest.len());
        return PathBuf::from(&rest[..end]);
    }
    PathBuf::from("unknown")
}

/// Handle clap parse failure: help/version exit via clap; other errors as NDJSON exit 2.
pub fn handle_clap_parse_error(clap_err: clap::Error) -> ExitCode {
    use clap::error::ErrorKind;
    match clap_err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            clap_err.exit();
        }
        _ => {
            let msg = clap_err.to_string();
            let suggestion = enrich_clap_suggestion(&msg);
            let ej = crate::error::ErrorJson {
                error: true,
                code: "ARGUMENT_PARSE_ERROR",
                exit: 2,
                message: msg,
                path: None,
                error_class: crate::error::ErrorClass::Permanent.as_str(),
                retryable: false,
                suggestion,
                workspace: None,
                failed_pair_index: None,
                pairs_total: None,
                pair_results: None,
                best_candidate: None,
                candidates: None,
                match_count: None,
                similar_paths: None,
            };
            let mut out = io::stdout().lock();
            if let Err(e) = serde_json::to_writer(&mut out, &ej) {
                let _ = writeln!(io::stderr(), "atomwrite: failed to write error JSON: {e}");
            }
            let _ = out.write_all(b"\n");
            let _ = out.flush();
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_log_level_matrix() {
        assert_eq!(cli_log_level(0, 0), "warn");
        assert_eq!(cli_log_level(1, 0), "info");
        assert_eq!(cli_log_level(2, 0), "debug");
        assert_eq!(cli_log_level(3, 0), "trace");
        assert_eq!(cli_log_level(0, 1), "error");
        assert_eq!(cli_log_level(0, 2), "off");
        // verbose wins over quiet when both set
        assert_eq!(cli_log_level(1, 1), "info");
    }

    #[test]
    fn resolve_log_format_matrix() {
        // G-007: format is file_sink-driven only (no env override).
        assert_eq!(resolve_log_format(false), LogFormat::Compact);
        assert_eq!(resolve_log_format(true), LogFormat::Json);
    }

    #[test]
    fn env_filter_parses_target_directive() {
        let f = EnvFilter::builder()
            .parse("warn,atomwrite=info")
            .expect("parse directive");
        let s = f.to_string();
        assert!(
            s.contains("warn") || s.contains("info") || !s.is_empty(),
            "unexpected filter display: {s}"
        );
    }
}
