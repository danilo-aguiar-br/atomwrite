// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entry point: signal setup, tracing init, and dispatch.

#![deny(unsafe_code)]
#![deny(static_mut_refs)]
// Match lib.rs: never take refs to `static mut` / never declare interior-mutable
// values as `const` (each use site would get a fresh cell).
#![deny(clippy::declare_interior_mutable_const)]
#![deny(clippy::borrow_interior_mutable_const)]
// Ownership hygiene (aligned with lib.rs).
#![deny(clippy::redundant_clone)]
#![deny(clippy::ptr_arg)]
#![deny(clippy::needless_borrow)]
#![deny(clippy::cloned_instead_of_copied)]

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use clap::Parser;

/// Process-wide heap allocator (`mimalloc`).
///
/// Rules Rust economia + eficiência: scripts/CLIs use mimalloc as
/// `#[global_allocator]` by default (lower fragmentation vs system allocator
/// on multi-thread `ignore` walks). Declared as `static` (stable address
/// required by `#[global_allocator]`), not `const` — singleton for the whole
/// binary. Exactly one global allocator in the dependency tree (binary only).
/// Re-validate with `dhat` / criterion if the allocation mix changes.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    atomwrite::signal::reset_sigpipe();
    atomwrite::platform::init_console();
    // Full signal set early so SIGINT/SIGTERM during setup still set the flag.
    let _early_shutdown = atomwrite::signal::install_handlers_early();
    human_panic::setup_panic!();

    if let Some(schema_cmd) = atomwrite::runtime::prescan_json_schema() {
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
        Err(e) => return atomwrite::runtime::handle_clap_parse_error(e),
    };

    atomwrite::runtime::init_locale(cli.global.lang.as_deref());
    // Keep WorkerGuard until end of main so non_blocking log worker flushes.
    let _guard = atomwrite::runtime::init_telemetry(
        cli.global.verbose,
        cli.global.quiet,
        cli.global.no_color,
    );
    // Panic hook after subscriber so panic events reach tracing (+ human_panic chain).
    atomwrite::runtime::install_panic_hook();
    // Observability for locale detection failure / resolved tag (after tracing).
    atomwrite::locale::log_resolved_locale();

    let shutdown = atomwrite::signal::install_handlers()
        .inspect_err(|e| tracing::warn!(%e, "signal handler registration failed"))
        .ok();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let exit = match atomwrite::run(&cli, stdin.lock(), stdout.lock(), stdin.is_terminal()) {
        Ok(()) => match &shutdown {
            Some(sig) if sig.is_shutdown() => {
                if sig.is_timeout() {
                    atomwrite::signal::write_timeout_message();
                } else {
                    atomwrite::signal::write_shutdown_message();
                }
                tracing::info!(exit = sig.exit_code(), "shutdown initiated");
                ExitCode::from(sig.exit_code())
            }
            _ => ExitCode::from(0),
        },
        Err(err) => map_run_error(&err, &cli),
    };

    tracing::info!("shutdown complete");
    exit
}

fn map_run_error(err: &anyhow::Error, cli: &atomwrite::cli::Cli) -> ExitCode {
    if let Some(aw_err) = err.downcast_ref::<atomwrite::error::AtomwriteError>() {
        if matches!(aw_err, atomwrite::error::AtomwriteError::BrokenPipe) {
            return ExitCode::from(141);
        }
        // Cooperative cancel: prefer the live signal/timeout exit and the
        // human banner on stderr (same path as Ok + is_shutdown), not only
        // a CANCELLED NDJSON line — agents still get the envelope when we
        // emit it below for non-signal programmatic cancels.
        if let atomwrite::error::AtomwriteError::Cancelled { exit, .. } = aw_err {
            if atomwrite::signal::is_global_shutdown() {
                if *exit == 124 {
                    atomwrite::signal::write_timeout_message();
                } else {
                    atomwrite::signal::write_shutdown_message();
                }
                tracing::info!(exit = *exit, "shutdown via Cancelled");
                return ExitCode::from(*exit);
            }
        }
        let mut out = io::stdout().lock();
        let ctx = atomwrite::error::ErrorContext {
            workspace_provided: cli.global.workspace.is_some(),
            workspace: cli.global.workspace.clone(),
        };
        let _ = atomwrite::output::write_error_json_with_context(&mut out, aw_err, None, &ctx);
        let _ = out.flush();
        return ExitCode::from(aw_err.exit_code());
    }
    if let Some(io_err) = atomwrite::runtime::find_io_error(err) {
        let aw_err = atomwrite::runtime::io_to_atomwrite_error(io_err, err);
        let mut out = io::stdout().lock();
        let ctx = atomwrite::error::ErrorContext {
            workspace_provided: cli.global.workspace.is_some(),
            workspace: cli.global.workspace.clone(),
        };
        let _ = atomwrite::output::write_error_json_with_context(&mut out, &aw_err, None, &ctx);
        let _ = out.flush();
        return ExitCode::from(aw_err.exit_code());
    }
    let _ = writeln!(io::stderr(), "atomwrite: {err:#}");
    ExitCode::from(1)
}
