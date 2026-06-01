// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entry point: signal setup, tracing init, and dispatch.

#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::EnvFilter;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    atomwrite::signal::reset_sigpipe();
    atomwrite::platform::init_console();
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
                    let suggestion = extract_clap_tip(&msg);
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
                let _ = atomwrite::output::write_error_json(&mut out, aw_err, None);
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
