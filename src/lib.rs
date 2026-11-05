// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file operations CLI library for LLM agents.
//!
//! Provides the library surface used by the `atomwrite` binary: atomic
//! write pipeline, NDJSON output types, fuzzy matching, path safety, and
//! subcommand handlers for agent-first file operations.
//!
//! # Examples
//!
//! Write a file atomically (tempfile, fsync, rename) and check its BLAKE3 checksum:
//!
//! ```
//! use atomwrite::atomic::{AtomicWriteOptions, atomic_write};
//! use atomwrite::checksum::hash_bytes;
//!
//! let dir = tempfile::tempdir()?;
//! let target = dir.path().join("notes.txt");
//! let opts = AtomicWriteOptions {
//!     backup: false,
//!     ..Default::default()
//! };
//! let result = atomic_write(&target, b"hello\n", &opts, dir.path())?;
//! assert_eq!(result.checksum, hash_bytes(b"hello\n"));
//! assert_eq!(std::fs::read_to_string(&target)?, "hello\n");
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Features
//!
//! - `core` — baseline library surface, part of `default`
//! - `ast` — tree-sitter and ast-grep based query, outline, transform and syntax check
//! - `lang-rust`, `lang-ts`, `lang-py`, `lang-go` — language markers for the AST features
//! - `lang-full` — enables all language markers
//! - `watch` — the `watch` subcommand (`notify`), opt-in
//! - `full` — `default` plus `lang-full` plus `watch`
//! - `i18n-cjk`, `i18n-rtl`, `i18n-europe`, `i18n-full` — placeholders for optional locale packs
//! - `slow-tests` — opt-in longer property tests
//!
//! The default features are `core`, `ast`, `lang-rust`, `lang-ts` and `lang-py`.
//! Feature-gated items are tagged on docs.rs via `doc_cfg` (see `# docs.rs`).
//!
//! # MSRV
//!
//! Rust 1.90, edition 2024.
//!
//! # Stability
//!
//! The supported interface is the `atomwrite` CLI and its NDJSON contract. The
//! library API is an implementation detail and may change between 0.x
//! versions: release 0.1.37 failed 10 `cargo-semver-checks` major checks
//! against 0.1.36, listed in `docs/MIGRATION.md`.
//!
//! # Safety
//!
//! This crate denies `unsafe` code (`#![deny(unsafe_code)]`). Only the
//! `signal` and `platform` modules opt back in, for operating-system calls.
//!
//! # docs.rs
//!
//! docs.rs builds with nightly and passes `--cfg docsrs`. Under that cfg this
//! crate enables `#![feature(doc_cfg)]` so feature and platform gates appear as
//! badges in the HTML. Do not reintroduce `feature(doc_auto_cfg)`: it was merged
//! into `doc_cfg` in October 2025 (rust-lang/rust#138907).
//! Local validation:
//!
//! ```text
//! RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps --all-features
//! cargo +stable doc --no-deps
//! ```
//!
//! Architecture diagrams for humans live in `ARCHITECTURE.md` (not rustdoc Mermaid).

// docs.rs nightly: doc_cfg replaces removed doc_auto_cfg (Oct 2025).
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
// Const/static hygiene: never allow interior-mutable `const` (would clone cells
// per use site) and never allow taking references to `static mut` (edition 2024).
#![deny(static_mut_refs)]
#![deny(clippy::declare_interior_mutable_const)]
#![deny(clippy::borrow_interior_mutable_const)]
// Ownership / borrowing hygiene (rules_rust_ownership_borrowing_lifetimes):
// refuse redundant clones, `&Vec`/`&String` params, and needless borrows.
#![deny(clippy::redundant_clone)]
#![deny(clippy::ptr_arg)]
#![deny(clippy::needless_borrow)]
#![deny(clippy::cloned_instead_of_copied)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::bare_urls)]
#![deny(rustdoc::redundant_explicit_links)]
#![deny(rustdoc::unescaped_backticks)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![warn(clippy::doc_markdown)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::needless_return)]

rust_i18n::i18n!("locales", fallback = "en");

pub mod atomic;
/// Best-effort helper for failures that may be ignored on purpose.
pub mod best_effort;
/// Binary content detection heuristics.
pub mod binary_detect;
/// BLAKE3 checksum computation.
pub mod checksum;
/// CLI definition and argument parsing.
pub mod cli;
/// Subcommand argument structs.
pub mod cli_args;
/// Subcommand handler implementations.
pub mod commands;
/// Bounded concurrency and multi-core fan-out.
pub mod concurrency;
/// Configuration file loading (`.atomwrite.toml`).
pub mod config;
/// Named constants for buffer sizes, thresholds, and identifiers.
pub mod constants;
/// Runtime environment autodetection (WSL, container, CI, Termux, …).
pub mod env_detect;
/// Domain-specific error types.
pub mod error;
/// Smart file reading with memmap2 for large files.
pub mod file_io;
/// Shared fuzzy match cascade (edit/replace/batch/edit-loop).
pub mod fuzzy;
/// Typed loading of a tree-sitter grammar (one decision for three verbs).
#[cfg(feature = "ast")]
#[cfg_attr(docsrs, doc(cfg(feature = "ast")))]
pub mod grammar;
/// Shared language utilities for AST commands.
#[cfg(feature = "ast")]
#[cfg_attr(docsrs, doc(cfg(feature = "ast")))]
pub mod lang_utils;
/// Line ending detection and normalization.
pub mod line_endings;
/// UI locale detection, negotiation, and resolved language state.
pub mod locale;
/// Advisory file locking for concurrent edit protection.
pub mod lock;
/// Media the text channel cannot carry: pictures and PDF text.
pub mod media;
/// NDJSON output type definitions.
pub mod ndjson;
#[allow(deprecated)]
pub mod ndjson_types {
    //! Compatibility shim — types live in [`crate::ndjson`].
    pub use crate::ndjson::*;
}
/// NDJSON output writer utilities.
pub mod output;
/// Workspace path jail validation.
pub mod path_safety;
/// Platform-specific fsync helpers.
pub mod platform;
/// Retry with exponential backoff, full jitter and a hard deadline.
pub mod retry;
/// Binary startup helpers (tracing, locale, clap error mapping).
pub mod runtime;
/// Graceful shutdown signal handling.
pub mod signal;
/// Stdin reader that stops waiting once the run is cancelled.
pub mod stdin_cancel;
/// Cross-platform storage paths (XDG / ProjectDirs; no product env knobs).
pub mod storage;
/// Syntax check via `tree-sitter-language-pack`.
#[cfg(feature = "ast")]
#[cfg_attr(docsrs, doc(cfg(feature = "ast")))]
pub mod syntax_check;
#[cfg(not(feature = "ast"))]
#[path = "syntax_check_stub.rs"]
pub mod syntax_check;
/// Write-Ahead Log (WAL) sidecar for crash recovery.
pub mod wal;
/// Include and exclude glob compilation shared by every verb (typed `INVALID_INPUT`).
pub mod walk_globs;
/// Extended attribute (xattr) save and restore for atomic writes.
pub mod xattr_restore;

use std::io::{Read, Write};

use anyhow::Result;

use crate::cli::{Cli, Commands};
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;

// Re-export fuzzy cascade for property tests and external reuse (v0.1.29).
pub use crate::cli_args::FuzzyMode;
pub use crate::fuzzy::{FuzzyInfo, match_pair};
pub use crate::ndjson_types::BestCandidate;

include!("schema_dispatch.inc.rs");
include!("lib_dispatch.inc.rs");

/// Dispatch the parsed CLI to the appropriate subcommand handler.
///
/// # Errors
///
/// Returns the error from whichever subcommand handler fails.
pub fn run(
    cli: &Cli,
    stdin: impl Read + Send + 'static,
    stdout: impl Write,
    stdin_is_tty: bool,
) -> Result<()> {
    if cli.global.json_schema {
        return emit_json_schema(&cli.command, stdout);
    }
    // `--print-schema` reached here only in the one case the early path defers:
    // the flag sat AFTER the command name and carried no value. A flag placed
    // after the command is not a discovery invocation — the caller was running
    // a command — and the previous behaviour answered the listing, silently
    // replacing the operation that was asked for.
    //
    // The field cannot be dead: a declared arg that nothing reads is a promise
    // the parser keeps and the program ignores, and clap would accept the flag
    // here only for the binary to drop it. It is read here to say so.
    if cli.global.print_schema.is_some() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "`--print-schema` must precede the command name; placed after a command it \
                     would replace that command with a schema dump. Run `atomwrite --print-schema` \
                     (no command) to list every schema, or `atomwrite --print-schema NAME` to emit one"
                .to_string(),
        }
        .into());
    }
    if let Commands::Completions(args) = &cli.command {
        return generate_completions(args, stdout);
    }

    if cli.global.json {
        tracing::debug!("--json is a no-op; output is always NDJSON");
    }

    // Single construction point for the data-channel shape. `from_global_args`
    // supersedes `from_cli` here so every global shaping flag (projection,
    // filtering, the universal data operations, redaction, encoding, diff) is
    // decided in one place instead of being added here per flag.
    // The work budget is armed before any verb can collect a list: `--limit` is
    // a bound on the WORK, not on the printing, so collection reads what is left
    // before it touches the disk.
    crate::output::arm_limit(cli.global.limit);
    // The path jail answers for the whole run, including the rayon workers, so
    // the policy is recorded here once instead of being passed to 70 call sites.
    crate::path_safety::set_follow_symlinks(cli.global.follow_symlinks);
    let shape = crate::output::OutputShape::from_global_args(&cli.global)?;
    let mut writer = NdjsonWriter::with_shape(stdout, shape);
    let shutdown = signal::get_or_install_handlers()?;
    // Rules Rust one-shot: global wall-clock deadline (cooperative cancel).
    signal::arm_global_timeout(cli.global.timeout_secs);
    let workspace = cli.global.resolve_workspace()?;
    // ETD step 2: the workspace is ambient when `--workspace` is absent, so
    // every terminal envelope says whether the root came from argv or the cwd.
    writer.set_workspace(if cli.global.workspace.is_some() {
        "argv"
    } else {
        "cwd"
    });
    // Every stdin reader goes through this adapter, so a blocked read gives up
    // when the deadline or a signal cancels the run (see `stdin_cancel`).
    let mut stdin = stdin_cancel::CancellableStdin::new(stdin, shutdown.flag());
    // `--join` reads its table from stdin here, once, before any command runs.
    writer.load_joins(
        &mut stdin,
        stdin_is_tty,
        crate::output::command_consumes_stdin(&cli.command),
    )?;
    // `config` runs BEFORE the config load: a broken file must never stop the
    // verb that repairs it (exit 78 would fire before the dispatch otherwise).
    if let Commands::Config(args) = &cli.command {
        let result = run_config_verb(args, &cli.global, &mut writer, &workspace);
        return finish_run(result, &mut writer, &shutdown);
    }
    let config = crate::config::load_config(&workspace, cli.global.config.as_deref())?;
    crate::config::validate_all(&config)?;
    // The profile is a stored choice, not a flag, so it is applied here: the
    // writer is built before the config load because the `config` verb must
    // run even when that file is broken.
    writer.set_profile(crate::output::OutputProfile::from_name(
        &config.output.profile,
    ));
    crate::config::install_tuning(&config);

    // Parallelism: record the worker bound (shared with ignore WalkParallel via
    // helpers) and build the process-wide rayon pool only for commands that can
    // reach it; `read` and the other single-file verbs spawn no pool threads.
    // `--threads` / `--max-concurrency` / `0` / omit → see `concurrency::effective_threads`.
    // It runs after the config load because `[concurrency]` bounds the automatic count.
    concurrency::plan_global_pool(&cli.command, cli.global.threads);

    startup_wal_heal(&cli.command, &workspace, cli.global.no_auto_heal);

    // `--input-schema` answers before any command work starts; it returns
    // without the terminal cancel point below, exactly as it did when it sat
    // inside the dispatch table.
    if let Commands::Batch(args) = &cli.command
        && args.input_schema
    {
        return commands::batch::emit_input_schema(&mut writer);
    }

    let result = dispatch_command(
        cli,
        stdin,
        stdin_is_tty,
        &mut writer,
        &shutdown,
        &workspace,
        &config,
    );

    // Single terminal-cancel point for every scanning verb. Before this, each
    // verb decided for itself what an interrupted run should say: `replace`
    // emitted the cancel envelope, while `search`, `hash`, `count`, `copy` and
    // `list` emitted nothing at all and `transform`, `scope`, `case` and
    // `batch` emitted a partial summary that reads like a finished run. A
    // consumer could not tell an interrupted run from a short one without
    // knowing which verb it had called.
    //
    // The envelope is emitted HERE, from one observation of the shutdown
    // signal, so the deadline verdict, the signal name and the prose always
    // describe the same cause. Events the scan did emit before the cancel stay
    // on the channel: the agent keeps what was found AND learns the scan was
    // cut short, instead of being asked to choose between the two.
    //
    // A run that was cancelled is terminal, so the record goes out even when
    // shaping flags are active. `--filter`, `--select` and `--limit` describe
    // how to shape a complete answer; letting them drop the cancel would put
    // the interrupted run back into the shape of a finished one, which is the
    // defect this point exists to remove.
    finish_run(result, &mut writer, &shutdown)
}

fn generate_completions(args: &cli::CompletionsArgs, mut out: impl Write) -> Result<()> {
    let shell = match args.shell {
        cli::ShellType::Bash => clap_complete::Shell::Bash,
        cli::ShellType::Zsh => clap_complete::Shell::Zsh,
        cli::ShellType::Fish => clap_complete::Shell::Fish,
        cli::ShellType::PowerShell => clap_complete::Shell::PowerShell,
        cli::ShellType::Elvish => clap_complete::Shell::Elvish,
    };

    if args.install {
        // Install under platform data dir (XDG via `directories` / storage layer).
        let xdg_data = crate::storage::data_dir()
            .and_then(|d| d.parent().map(|p| p.to_path_buf()))
            .or_else(crate::storage::data_dir)
            .ok_or_else(|| anyhow::anyhow!("cannot determine XDG data directory (storage)"))?;

        let (subdir, filename) = match args.shell {
            cli::ShellType::Bash => crate::constants::COMPLETION_BASH,
            cli::ShellType::Zsh => crate::constants::COMPLETION_ZSH,
            cli::ShellType::Fish => crate::constants::COMPLETION_FISH,
            cli::ShellType::PowerShell => crate::constants::COMPLETION_POWERSHELL,
            cli::ShellType::Elvish => crate::constants::COMPLETION_ELVISH,
        };

        let install_dir = xdg_data.join(subdir);
        // A denied directory is exit 13 `PERMISSION_DENIED` and a vanished one
        // is `NOT_FOUND`, through the same classifier every other verb uses.
        // `anyhow::anyhow!` here erased the kind and the run answered 255
        // `INTERNAL_ERROR` with "this is a bug", which blames the tool for a
        // permission the operator can fix.
        std::fs::create_dir_all(&install_dir)
            .map_err(|e| AtomwriteError::from_io_at(&install_dir, e))?;
        let install_path = install_dir.join(filename);

        let mut file = std::fs::File::create(&install_path)
            .map_err(|e| AtomwriteError::from_io_at(&install_path, e))?;
        file.write_all(&render_completions(shell))
            .map_err(|e| AtomwriteError::from_io_at(&install_path, e))?;

        let event = CompletionsInstalled {
            r#type: "installed",
            path: install_path.display().to_string(),
        };
        // Through the writer, so the event carries the same `ok` verdict and
        // `target_resolved` / `target_source` as every other success envelope.
        let mut writer = NdjsonWriter::new(out);
        writer.set_target(event.path.clone(), "argv");
        writer.write_event(&event)?;
        writer.flush()?;
        Ok(())
    } else {
        // `clap_complete::generate` panics on a failed write, so it renders to
        // memory and the write goes out here, where a closed reader is an error.
        out.write_all(&render_completions(shell))?;
        out.flush()?;
        Ok(())
    }
}

/// Render the completion script of `shell` in memory.
fn render_completions(shell: clap_complete::Shell) -> Vec<u8> {
    use clap::CommandFactory;
    let mut script = Vec::new();
    clap_complete::generate(shell, &mut cli::Cli::command(), "atomwrite", &mut script);
    script
}
