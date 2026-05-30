// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entry point: signal setup, tracing init, and dispatch.

#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    atomwrite::signal::reset_sigpipe();
    human_panic::setup_panic!();

    let cli = atomwrite::cli::Cli::parse();

    init_locale(&cli.global.lang);
    init_tracing(cli.global.verbose, cli.global.quiet);

    let shutdown = atomwrite::signal::install_handlers().ok();

    let stdin = io::stdin();
    let stdout = io::stdout();

    match atomwrite::run(&cli, stdin.lock(), stdout.lock()) {
        Ok(()) => {
            if let Some(ref sig) = shutdown {
                if sig.is_shutdown() {
                    return ExitCode::from(sig.exit_code());
                }
            }
            ExitCode::from(0)
        }
        Err(err) => {
            if let Some(aw_err) = err.downcast_ref::<atomwrite::error::AtomwriteError>() {
                let mut out = io::stdout().lock();
                let _ = atomwrite::output::write_error_json(&mut out, aw_err, None);
                let _ = out.flush();
                ExitCode::from(aw_err.exit_code())
            } else {
                let _ = writeln!(io::stderr(), "atomwrite: {err:#}");
                ExitCode::from(1)
            }
        }
    }
}

fn init_tracing(verbose: u8, quiet: u8) {
    let level = match (verbose, quiet) {
        (0, 0) => "warn",
        (1, _) => "info",
        (2, _) => "debug",
        (3.., _) => "trace",
        (_, 1) => "error",
        (_, 2..) => "off",
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    let ansi = !no_color();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .with_target(false)
        .with_ansi(ansi)
        .compact()
        .init();
}

fn no_color() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
}

fn init_locale(lang_override: &Option<String>) {
    let locale = if let Some(lang) = lang_override {
        lang.clone()
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
