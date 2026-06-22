// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entry point: signal setup, tracing init, and dispatch.

#![deny(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::EnvFilter;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    atomwrite::signal::reset_sigpipe();
    atomwrite::platform::init_console();
    // Install signal handlers as EARLY as possible, before any other
    // initialization. This guarantees that if SIGINT or SIGTERM arrives
    // during the setup phase (Cli::try_parse, init_tracing, rayon pool
    // build, etc.), our handler still runs and sets the shutdown flag
    // instead of the default handler terminating the process via signal.
    // Without this, tests that send SIGINT within 100ms of spawning the
    // child can race with the signal handler installation and the child
    // gets killed by the default disposition with exit 128+SIGINT (130),
    // which is a different code path than our graceful shutdown and
    // produces no "shutting down" message on stderr.
    //
    // We install the FULL handler set here (not just the flag-only
    // early-install variant) so that the main thread, the search polling
    // inside `atomwrite::run`, and any other future consumer all share
    // a single `Arc<ShutdownSignal>` instance. The previous design
    // installed two separate `ShutdownSignal` instances (`install_handlers_early`
    // returning signal A and `install_handlers` returning signal B) and
    // the search polling observed flag A while the main-thread shutdown
    // check observed flag B, which under signal-hook's chain-of-handlers
    // ordering caused B to remain false in some timing windows — leading
    // to the main thread taking the `Ok(())` branch with `is_shutdown()`
    // returning false and the user-facing "shutting down" banner never
    // being emitted.
    let _early_shutdown = atomwrite::signal::install_handlers_early();
    human_panic::setup_panic!();

    if let Some(schema_cmd) = prescan_json_schema() {
        let mut out = io::stdout().lock();
        match atomwrite::emit_schema_by_name(&schema_cmd, &mut out) {
            Ok(true) => return ExitCode::from(0),
            Ok(false) => {}
            Err(e) => {
                let _ = writeln!(io::stderr(), "atomwrite: {e:#}");
                return ExitCode::from(1);
            }
        }
    }

    let cli = match atomwrite::cli::Cli::try_parse() {
        Ok(c) => c,
        Err(clap_err) => {
            use clap::error::ErrorKind;
            match clap_err.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    clap_err.exit();
                }
                _ => {
                    let msg = clap_err.to_string();
                    let suggestion = enrich_clap_suggestion(&msg);
                    let ej = atomwrite::error::ErrorJson {
                        error: true,
                        code: "ARGUMENT_PARSE_ERROR",
                        exit: 2,
                        message: msg,
                        path: None,
                        error_class: atomwrite::error::ErrorClass::Permanent.as_str(),
                        retryable: false,
                        suggestion,
                        workspace: None,
                        failed_pair_index: None,
                        pairs_total: None,
                        pair_results: None,
                    };
                    let mut out = io::stdout().lock();
                    if let Err(e) = serde_json::to_writer(&mut out, &ej) {
                        let _ =
                            writeln!(io::stderr(), "atomwrite: failed to write error JSON: {e}");
                    }
                    let _ = out.write_all(b"\n");
                    let _ = out.flush();
                    return ExitCode::from(2);
                }
            }
        }
    };

    init_locale(cli.global.lang.as_deref());
    let _guard = init_tracing(cli.global.verbose, cli.global.quiet, cli.global.no_color);
    install_panic_hook();

    let shutdown = atomwrite::signal::install_handlers()
        .inspect_err(|e| tracing::warn!(%e, "signal handler registration failed"))
        .ok();

    let stdin = io::stdin();
    let stdout = io::stdout();

    let exit = match atomwrite::run(&cli, stdin.lock(), stdout.lock()) {
        Ok(()) => {
            if let Some(ref sig) = shutdown {
                if sig.is_shutdown() {
                    // Emit user-facing shutdown message from the main thread.
                    // This is the async-signal-safe equivalent of a handler-side
                    // eprintln!. The signal handler is forbidden from calling
                    // eprintln! per POSIX.1 signal-safety(7) because Rust's
                    // stderr uses a global Mutex that can deadlock or lose
                    // output if the signal arrives while another thread holds
                    // the lock (observed on Linux/glibc; eprintln! output was
                    // silently dropped before reaching the captured stderr pipe
                    // in tests).
                    //
                    // `atomwrite::signal::write_shutdown_message` uses
                    // `libc::write(STDERR_FILENO, ...)` which is async-signal-
                    // safe per POSIX.1-2017 signal-safety(7) and writes directly
                    // to fd 2 without any userspace buffering. This bypasses
                    // Rust's stderr buffer (which is fully-buffered when stderr
                    // is redirected to a pipe via Stdio::piped() in cargo test,
                    // causing writeln! output to remain in the buffer and be
                    // lost when the process exits before the buffer is flushed).
                    // The libc::write goes straight to the kernel, guaranteeing
                    // the bytes reach the captured pipe before the process
                    // exits with the signal exit code.
                    atomwrite::signal::write_shutdown_message();
                    tracing::info!(signal = sig.exit_code(), "shutdown initiated");
                    ExitCode::from(sig.exit_code())
                } else {
                    ExitCode::from(0)
                }
            } else {
                ExitCode::from(0)
            }
        }
        Err(err) => {
            if let Some(aw_err) = err.downcast_ref::<atomwrite::error::AtomwriteError>() {
                if matches!(aw_err, atomwrite::error::AtomwriteError::BrokenPipe) {
                    return ExitCode::from(141);
                }
                let mut out = io::stdout().lock();
                let ctx = atomwrite::error::ErrorContext {
                    workspace_provided: cli.global.workspace.is_some(),
                    workspace: cli.global.workspace.clone(),
                };
                let _ =
                    atomwrite::output::write_error_json_with_context(&mut out, aw_err, None, &ctx);
                let _ = out.flush();
                ExitCode::from(aw_err.exit_code())
            } else {
                let _ = writeln!(io::stderr(), "atomwrite: {err:#}");
                ExitCode::from(1)
            }
        }
    };

    tracing::info!("shutdown complete");
    exit
}

fn init_tracing(
    verbose: u8,
    quiet: u8,
    cli_no_color: bool,
) -> tracing_appender::non_blocking::WorkerGuard {
    let level = match (verbose, quiet) {
        (0, 0) => "warn",
        (1, _) => "info",
        (2, _) => "debug",
        (3.., _) => "trace",
        (_, 1) => "error",
        (_, 2..) => "off",
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    let ansi = if no_color() || cli_no_color {
        false
    } else if force_color() {
        true
    } else {
        std::io::IsTerminal::is_terminal(&std::io::stderr())
    };

    let show_source = matches!(level, "debug" | "trace");
    let (non_blocking, guard) = tracing_appender::non_blocking(io::stderr());

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_target(false)
        .with_ansi(ansi)
        .with_thread_ids(true)
        .with_file(show_source)
        .with_line_number(show_source)
        .compact();

    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(tracing_error::ErrorLayer::default())
        .init();

    tracing::debug!(filter = %level, "tracing initialized");

    guard
}

fn install_panic_hook() {
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

fn no_color() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
}

fn force_color() -> bool {
    std::env::var_os("CLICOLOR_FORCE").is_some_and(|v| v == "1")
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

fn enrich_clap_suggestion(msg: &str) -> Option<String> {
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

fn prescan_json_schema() -> Option<String> {
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
    ];
    for arg in &args[1..] {
        if SUBCOMMANDS.contains(&arg.as_str()) {
            return Some(arg.clone());
        }
    }
    None
}

fn init_locale(lang_override: Option<&str>) {
    let locale = if let Some(lang) = lang_override {
        lang.to_owned()
    } else {
        sys_locale::get_locale().unwrap_or_else(|| "en".to_string())
    };

    let resolved = if locale.starts_with("pt") {
        "pt-BR"
    } else {
        "en"
    };

    rust_i18n::set_locale(resolved);
}
