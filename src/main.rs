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

use atomwrite::constants::{EXIT_BROKEN_PIPE, EXIT_FAILURE, EXIT_SUCCESS, EXIT_TIMEOUT};
use clap::FromArgMatches;

/// Process-wide heap allocator (`mimalloc`).
///
/// Rules Rust economy + efficiency: scripts/CLIs use mimalloc as
/// `#[global_allocator]` by default (lower fragmentation vs system allocator
/// on multi-thread `ignore` walks). Declared as `static` (stable address
/// required by `#[global_allocator]`), not `const` — singleton for the whole
/// binary. Exactly one global allocator in the dependency tree (binary only).
/// Re-validate with `dhat` / criterion if the allocation mix changes.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    real_main()
}

/// Whole process lifecycle with a single exit point.
///
/// Every outcome is computed as a value and the function ends on one
/// expression, so no early `return` can skip the shutdown bookkeeping.
fn real_main() -> ExitCode {
    // `args_os` + lossy: a non-UTF-8 argument must reach clap (which handles
    // `OsString`) instead of panicking here; `String`s are only needed by the
    // schema prescans, which compare against ASCII names.
    let args: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    atomwrite::platform::init_console();
    // Full signal set early so SIGINT/SIGTERM/SIGHUP during setup still set the flag.
    let _early_shutdown = atomwrite::signal::install_handlers_early();
    human_panic::setup_panic!();

    match answer_schema_request(&args) {
        Some(exit) => exit,
        // `Cli::parser` is the one place the parser is built: it fills the key
        // table of `config --help` from the live `KEYS`, which the derive
        // attribute cannot express. `try_parse` would build the parser a second
        // time and print the stale table.
        None => match atomwrite::cli::Cli::parser()
            .try_get_matches()
            .and_then(|m| atomwrite::cli::Cli::from_arg_matches(&m))
        {
            Ok(cli) => execute(&cli),
            Err(e) => atomwrite::runtime::handle_clap_parse_error(e),
        },
    }
}

/// Answer `--json-schema` and `--print-schema` before clap runs.
///
/// Schema discovery is useful on an install that is still being wired up, and
/// must never depend on a subcommand parsing. Handled here so the data channel
/// carries the schema and nothing else. `None` means "not a schema request".
fn answer_schema_request(args: &[String]) -> Option<ExitCode> {
    if let Some(schema_cmd) = atomwrite::runtime::prescan_json_schema() {
        let mut out = io::stdout().lock();
        match atomwrite::emit_schema_by_name(&schema_cmd, &mut out) {
            Ok(true) => return Some(ExitCode::from(EXIT_SUCCESS as u8)),
            Ok(false) => {}
            Err(e) => return Some(schema_write_failure(e)),
        }
    }

    // `--print-schema [NAME]` is answered before clap for the same reason as
    // `--json-schema`.
    match atomwrite::runtime::prescan_print_schema(args) {
        Ok(exit) => exit,
        Err(e) => Some(schema_write_failure(e)),
    }
}

/// Settle the exit status of a schema answer that could not be written.
///
/// A reader that closed the pipe is the broken-pipe status (141), the same as
/// for every other answer on the data channel; any other failure is told on
/// stderr and exits 1.
fn schema_write_failure(error: anyhow::Error) -> ExitCode {
    ExitCode::from(atomwrite::best_effort::settle_error_report(
        Err(error),
        EXIT_FAILURE as u8,
    ))
}

/// Run the parsed command and map the outcome to the process exit status.
fn execute(cli: &atomwrite::cli::Cli) -> ExitCode {
    atomwrite::runtime::init_locale(cli.global.lang.as_deref());
    // The workspace is resolved here, before the subscriber, because the
    // workspace `.atomwrite.toml` may turn the log sink on. A failure to resolve
    // it is not fatal at this point: `run` resolves it again and reports the
    // same error through the normal envelope.
    let workspace = cli
        .global
        .resolve_workspace()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    // Keep WorkerGuard until end of main so non_blocking log worker flushes.
    // Color: CLI only (G-007) via GlobalArgs::color_mode.
    let _guard = atomwrite::runtime::init_tracing(
        cli.global.verbose,
        cli.global.quiet,
        cli.global.color_mode(),
        &workspace,
        cli.global.config.as_deref(),
    );
    // Panic hook after subscriber so panic events reach tracing (+ human_panic chain).
    atomwrite::runtime::install_panic_hook();
    // Local diagnostics for locale detection failure / resolved tag (after tracing).
    atomwrite::locale::log_resolved_locale();

    // G-007: readiness probe via CLI `--ready-file` (handlers already installed early).
    if let Some(ref ready) = cli.global.ready_file {
        atomwrite::signal::write_ready_file(ready);
    }

    let shutdown = atomwrite::signal::install_handlers()
        .inspect_err(|e| tracing::warn!(%e, "signal handler registration failed"))
        .ok();

    // `Stdin` (not its lock): the cancellable reader moves it to a thread.
    let stdout = io::stdout();
    let stdin_is_tty = io::stdin().is_terminal();
    let exit = match atomwrite::run(cli, io::stdin(), stdout.lock(), stdin_is_tty) {
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
            _ => ExitCode::from(EXIT_SUCCESS as u8),
        },
        Err(err) => map_run_error(&err, cli),
    };

    tracing::info!("shutdown complete");
    exit
}

fn map_run_error(err: &anyhow::Error, cli: &atomwrite::cli::Cli) -> ExitCode {
    // A run that already published its verdict on the data channel (`hash
    // --verify`, or a summary written by `NdjsonWriter::publish_failure`) must
    // not write the generic error envelope as well: that would state one fact
    // twice and leave two terminal events (B2). The marker is tested before any
    // downcast, and the status comes from the error the marker wraps. An error
    // without the marker has published nothing and gets its envelope below.
    if err
        .downcast_ref::<atomwrite::output::VerdictPublished>()
        .is_some()
    {
        let (_, exit) = atomwrite::output::failure_identity(err);
        return ExitCode::from(exit);
    }
    if let Some(aw_err) = err.downcast_ref::<atomwrite::error::AtomwriteError>() {
        if matches!(aw_err, atomwrite::error::AtomwriteError::BrokenPipe) {
            return ExitCode::from(EXIT_BROKEN_PIPE as u8);
        }
        // Cooperative cancel: prefer the live signal/timeout exit and the
        // human banner on stderr (same path as Ok + is_shutdown), not only
        // a CANCELLED NDJSON line — agents still get the envelope when we
        // emit it below for non-signal programmatic cancels.
        if let atomwrite::error::AtomwriteError::Cancelled { exit, .. } = aw_err
            && atomwrite::signal::is_global_shutdown()
        {
            if *exit == EXIT_TIMEOUT as u8 {
                atomwrite::signal::write_timeout_message();
            } else {
                atomwrite::signal::write_shutdown_message();
            }
            tracing::info!(exit = *exit, "shutdown via Cancelled");
            return ExitCode::from(*exit);
        }
        return publish_error_envelope(aw_err, cli);
    }
    if let Some(io_err) = atomwrite::runtime::find_io_error(err) {
        // A closed reader is not a failure to report: the data channel is gone,
        // so no envelope can reach it. The SIGPIPE handler has already raised
        // the cooperative cancel; the run unwound through its normal error path
        // (every `Drop` ran), and the status is the one a shell shows for SIGPIPE.
        if io_err == io::ErrorKind::BrokenPipe {
            return ExitCode::from(EXIT_BROKEN_PIPE as u8);
        }
        let aw_err = atomwrite::runtime::io_to_atomwrite_error(io_err, err);
        return publish_error_envelope(&aw_err, cli);
    }
    // An error that carries no type is still an error the agent must see: the
    // envelope goes out as `INTERNAL_ERROR` instead of a bare stderr line.
    let untyped = atomwrite::error::AtomwriteError::InternalError {
        reason: format!("{err:#}"),
    };
    publish_error_envelope(&untyped, cli)
}

/// Whether this invocation owns stdout as a byte or text channel.
///
/// `search --format text` promises stdout carries result lines and nothing
/// else, so the error envelope cannot land there: an `{"ok":false,...}`
/// record would be a document in place of the format the caller asked for.
/// `read --format raw` makes the same promise over bytes, and a `cat`
/// replacement whose error arrives as JSON on stdout is not a `cat`: the
/// consumer that pipes it into another program gets a record where the file
/// should be. Both modes therefore report on stderr.
fn is_byte_stream_mode(cli: &atomwrite::cli::Cli) -> bool {
    match &cli.command {
        atomwrite::cli::Commands::Search(args) => match args.format {
            Some(format) => format == atomwrite::cli::SearchFormat::Text,
            // The dispatch is what applies the config, so a failure raised
            // before it still has to know that stdout was promised as text.
            None => atomwrite::config::tuning().search_format_is_text,
        },
        atomwrite::cli::Commands::Read(args) | atomwrite::cli::Commands::Stat(args) => {
            matches!(args.format, atomwrite::cli::OutputFormat::Raw)
        }
        _ => false,
    }
}

/// Report a text-mode failure on stderr and settle the exit status.
///
/// `rg` exits 1 in silence when nothing matched, and the status alone carries
/// that verdict, so `NO_MATCHES` prints no line. Every other failure names
/// itself on stderr, which is where `rg` puts its own diagnostics.
fn publish_text_mode_error(aw_err: &atomwrite::error::AtomwriteError) -> ExitCode {
    if !matches!(aw_err, atomwrite::error::AtomwriteError::NoMatches) {
        // A closed stderr must not change the verdict: the status already
        // carries it, so the failed write is logged and dropped.
        atomwrite::best_effort::best_effort(
            writeln!(io::stderr().lock(), "atomwrite: {aw_err}"),
            "report text-mode error on stderr",
        );
    }
    ExitCode::from(aw_err.exit_code())
}

/// Write the error envelope on the data channel and settle the exit status.
///
/// A closed reader turns the status into the broken-pipe one; any other write
/// failure is told on stderr and the error's own status stands.
fn publish_error_envelope(
    aw_err: &atomwrite::error::AtomwriteError,
    cli: &atomwrite::cli::Cli,
) -> ExitCode {
    if is_byte_stream_mode(cli) {
        return publish_text_mode_error(aw_err);
    }
    let mut out = io::stdout().lock();
    let ctx = atomwrite::error::ErrorContext {
        workspace_provided: cli.global.workspace.is_some(),
        workspace: cli.global.workspace.clone(),
        verb: match cli.command {
            atomwrite::cli::Commands::Apply(_) => Some("apply"),
            atomwrite::cli::Commands::Edit(_) => Some("edit"),
            _ => None,
        },
    };
    // `from_global_args`, not `from_cli`: the flat constructor only carries
    // select/filter/limit, so `--redact` arrived inert here and a credential
    // inside a path survived the error envelope while the same credential in
    // a success payload was redacted. One constructor for both channels is
    // what stops the two from drifting apart again.
    // The constructor refuses a malformed `--filter`, `--join`, `--aggregate`
    // or `--truncate-content` spec, and that refusal IS the failure this
    // envelope reports. Falling back to a bare default there dropped both the
    // `-v` the caller asked for and the redaction switch, so the two flags that
    // cannot be malformed are carried over by hand.
    let shape =
        atomwrite::output::OutputShape::from_global_args(&cli.global).unwrap_or_else(|_| {
            atomwrite::output::OutputShape {
                verbose: cli.global.verbose > 0,
                redact: cli.global.redact,
                ..atomwrite::output::OutputShape::default()
            }
        });
    // A run that failed before writing its terminal record still names the
    // subject it did not reach, so the caller can tell which target is intact.
    let target = atomwrite::error_envelope_target(cli);
    let written = atomwrite::output::write_error_json_shaped_with_target(
        &mut out,
        aw_err,
        None,
        &ctx,
        &shape,
        target.as_ref().map(|(r, s)| (r, s.as_str())),
    )
    .and_then(|()| out.flush().map_err(anyhow::Error::from));
    ExitCode::from(atomwrite::best_effort::settle_error_report(
        written,
        aw_err.exit_code(),
    ))
}
