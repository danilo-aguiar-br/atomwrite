// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file operations CLI library for LLM agents.
//!
//! Provides the library surface used by the `atomwrite` binary: atomic
//! write pipeline, NDJSON output types, fuzzy matching, path safety, and
//! subcommand handlers for agent-first file operations.
//!
//! # Features
//!
//! - `core` — baseline library surface (always on via `default`)
//! - `ast` — tree-sitter / ast-grep based query, outline, transform, syntax check
//! - `lang-rust` / `lang-ts` / `lang-py` / `lang-go` — language markers for AST features
//! - `lang-full` — enables all language markers
//! - `watch` — filesystem watch subcommand (`notify`)
//! - `semantic` — semantic search/merge experimental surface
//! - `full` — convenience meta-feature (`default` + `lang-full` + `watch` + `semantic`)
//! - `slow-tests` — opt-in longer property tests
//!
//! Feature-gated items are auto-tagged on docs.rs via `doc_cfg` (see `# Safety`).
//!
//! # Safety
//!
//! This crate denies `unsafe` in library code (`#![deny(unsafe_code)]`).
//!
//! **docs.rs / rustdoc nightly:** docs.rs builds with nightly and passes
//! `--cfg docsrs`. Under that cfg this crate enables
//! `#![feature(doc_cfg)]` so feature/platform gates appear as badges in HTML.
//! `doc_auto_cfg` was **removed** (merged into `doc_cfg` in October 2025;
//! rust-lang/rust#138907). Do not reintroduce `feature(doc_auto_cfg)`.
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
#![warn(rustdoc::private_intra_doc_links)]
#![warn(rustdoc::invalid_html_tags)]
#![warn(clippy::doc_markdown)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::needless_return)]

rust_i18n::i18n!("locales", fallback = "en");

/// Atomic file write pipeline.
pub mod concurrency;
pub mod atomic;
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
/// Configuration file loading (`.atomwrite.toml`).
pub mod config;
/// Named constants for buffer sizes, thresholds, and identifiers.
pub mod constants;
/// Domain-specific error types.
pub mod error;
/// Runtime environment autodetection (WSL, container, CI, Termux, …).
pub mod env_detect;
/// Smart file reading with memmap2 for large files.
pub mod file_io;
/// Shared 9-strategy fuzzy match cascade (edit/replace/batch/edit-loop).
pub mod fuzzy;
/// Shared language utilities for AST commands.
pub mod lang_utils;
/// UI locale detection, negotiation, and resolved language state.
pub mod locale;
/// Line ending detection and normalization.
pub mod line_endings;
/// Advisory file locking for concurrent edit protection (G54).
pub mod lock;
/// NDJSON output type definitions.
pub mod ndjson_types;
/// NDJSON output writer utilities.
pub mod output;
/// Workspace path jail validation.
pub mod path_safety;
/// Platform-specific fsync helpers.
pub mod platform;
/// Graceful shutdown signal handling.
pub mod signal;
/// Cross-platform storage paths (`ATOMWRITE_HOME` / XDG / ProjectDirs).
pub mod storage;
/// Binary startup helpers (tracing, locale, clap error mapping).
pub mod runtime;
/// G72 — Real syntax check via `tree-sitter-language-pack` (v0.1.12).
#[cfg(feature = "ast")]
#[cfg_attr(docsrs, doc(cfg(feature = "ast")))]
pub mod syntax_check;
#[cfg(not(feature = "ast"))]
#[path = "syntax_check_stub.rs"]
pub mod syntax_check;
/// G114 — Write-Ahead Log (WAL) sidecar for crash recovery (v0.1.12).
pub mod wal;
/// Extended attribute (xattr) save and restore for atomic writes (G39).
pub mod xattr_restore;

use std::io::{Read, Write};

use anyhow::Result;

use crate::cli::{Cli, Commands};
use crate::output::NdjsonWriter;

// Re-export fuzzy cascade for property tests and external reuse (v0.1.29).
pub use crate::cli_args::FuzzyMode;
pub use crate::fuzzy::{FuzzyInfo, match_pair};
pub use crate::ndjson_types::BestCandidate;

/// Emit the JSON Schema for the given subcommand's NDJSON output.
fn emit_json_schema(command: &Commands, mut out: impl Write) -> Result<()> {
    let schema = match command {
        Commands::Read(_) => schemars::schema_for!(ndjson_types::ReadOutput),
        Commands::Write(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Edit(_) => schemars::schema_for!(ndjson_types::EditOutput),
        Commands::Search(_) => schemars::schema_for!(ndjson_types::SearchMatch),
        Commands::Replace(_) => schemars::schema_for!(ndjson_types::ReplaceResult),
        Commands::Hash(_) => schemars::schema_for!(ndjson_types::HashOutput),
        Commands::Delete(_) => schemars::schema_for!(ndjson_types::DeleteOutput),
        Commands::Count(_) => schemars::schema_for!(ndjson_types::Summary),
        Commands::Diff(_) => schemars::schema_for!(ndjson_types::DryRunPlan),
        Commands::Move(_) => schemars::schema_for!(ndjson_types::MoveOutput),
        Commands::Copy(_) => schemars::schema_for!(ndjson_types::CopyOutput),
        Commands::List(_) => schemars::schema_for!(ndjson_types::ListEntry),
        Commands::Extract(_) => schemars::schema_for!(ndjson_types::CalcOutput),
        Commands::Calc(_) => schemars::schema_for!(ndjson_types::CalcOutput),
        Commands::Regex(_) => schemars::schema_for!(ndjson_types::RegexOutput),
        Commands::Transform(_) => schemars::schema_for!(ndjson_types::TransformResult),
        Commands::Batch(_) => schemars::schema_for!(ndjson_types::BatchSummary),
        Commands::Scope(_) => schemars::schema_for!(ndjson_types::ScopeResult),
        Commands::Backup(_) => schemars::schema_for!(ndjson_types::BackupResult),
        Commands::Rollback(_) => schemars::schema_for!(ndjson_types::RollbackResult),
        Commands::Apply(_) => schemars::schema_for!(ndjson_types::ApplyResult),
        Commands::Set(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Get(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Del(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Case(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Query(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Outline(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::WalStats(_) => schemars::schema_for!(ndjson_types::WalStats),
        Commands::WalHeal(_) => schemars::schema_for!(ndjson_types::AutoHealReport),
        Commands::PruneBackups(_) => schemars::schema_for!(ndjson_types::PruneBackupSummary),
        Commands::EditLoop(_) => schemars::schema_for!(ndjson_types::EditLoopSummary),
        Commands::Verify(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::SemanticMerge(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Sparse(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Recipe(_) => schemars::schema_for!(crate::commands::recipe::RecipeResult),
        Commands::Stat(_) => schemars::schema_for!(ndjson_types::ReadOutput),
        Commands::AgentSurface(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Watch(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::Codemod(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::SemanticSearch(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::Doctor(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::Locale(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::CommandsTree(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::Completions(_) => schemars::schema_for!(ndjson_types::CalcOutput),
    };
    serde_json::to_writer_pretty(&mut out, &schema)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// Emit the JSON Schema for a subcommand by name, without requiring parsed args.
///
/// Returns `Ok(true)` if the schema was emitted, `Ok(false)` if the name is unknown.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn emit_schema_by_name(name: &str, mut out: impl Write) -> Result<bool> {
    let schema = match name {
        "read" => schemars::schema_for!(ndjson_types::ReadOutput),
        "write" => schemars::schema_for!(ndjson_types::WriteOutput),
        "edit" => schemars::schema_for!(ndjson_types::EditOutput),
        "search" => schemars::schema_for!(ndjson_types::SearchMatch),
        "replace" => schemars::schema_for!(ndjson_types::ReplaceResult),
        "hash" => schemars::schema_for!(ndjson_types::HashOutput),
        "delete" => schemars::schema_for!(ndjson_types::DeleteOutput),
        "count" => schemars::schema_for!(ndjson_types::Summary),
        "diff" => schemars::schema_for!(ndjson_types::DryRunPlan),
        "move" => schemars::schema_for!(ndjson_types::MoveOutput),
        "copy" => schemars::schema_for!(ndjson_types::CopyOutput),
        "list" => schemars::schema_for!(ndjson_types::ListEntry),
        "extract" => schemars::schema_for!(ndjson_types::CalcOutput),
        "calc" => schemars::schema_for!(ndjson_types::CalcOutput),
        "regex" => schemars::schema_for!(ndjson_types::RegexOutput),
        "transform" => schemars::schema_for!(ndjson_types::TransformResult),
        "batch" => schemars::schema_for!(ndjson_types::BatchSummary),
        "scope" => schemars::schema_for!(ndjson_types::ScopeResult),
        "backup" => schemars::schema_for!(ndjson_types::BackupResult),
        "rollback" => schemars::schema_for!(ndjson_types::RollbackResult),
        "apply" => schemars::schema_for!(ndjson_types::ApplyResult),
        "set" => schemars::schema_for!(ndjson_types::WriteOutput),
        "get" => schemars::schema_for!(ndjson_types::WriteOutput),
        "del" => schemars::schema_for!(ndjson_types::WriteOutput),
        "case" => schemars::schema_for!(ndjson_types::WriteOutput),
        "query" => schemars::schema_for!(ndjson_types::WriteOutput),
        "outline" => schemars::schema_for!(ndjson_types::WriteOutput),
        "prune-backups" => schemars::schema_for!(ndjson_types::PruneBackupSummary),
        "edit-loop" => schemars::schema_for!(ndjson_types::EditLoopSummary),
        "completions" => schemars::schema_for!(ndjson_types::WriteOutput),
        "recipe" => schemars::schema_for!(crate::commands::recipe::RecipeResult),
        "progress" => schemars::schema_for!(ndjson_types::ProgressEvent),
        "error" => schemars::schema_for!(crate::error::ErrorJson),
        "best-candidate" => schemars::schema_for!(ndjson_types::BestCandidate),
        "cancelled" => schemars::schema_for!(ndjson_types::CancelledEvent),
        "semantic-merge" | "sparse" | "agent-surface" | "watch" | "codemod" | "semantic-search"
        | "doctor" | "locale" | "commands" => {
            schemars::schema_for!(ndjson_types::ProgressEvent)
        }
        _ => return Ok(false),
    };
    serde_json::to_writer_pretty(&mut out, &schema)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(true)
}

/// Dispatch the parsed CLI to the appropriate subcommand handler.
///
/// # Errors
///
/// Returns the error from whichever subcommand handler fails.
pub fn run(cli: &Cli, stdin: impl Read, stdout: impl Write, stdin_is_tty: bool) -> Result<()> {
    if cli.global.json_schema {
        return emit_json_schema(&cli.command, stdout);
    }
    if let Commands::Completions(args) = &cli.command {
        return generate_completions(args, stdout);
    }

    if cli.global.json {
        tracing::debug!("--json is a no-op; output is always NDJSON");
    }

    // Parallelism modus operandi: always size the process-wide rayon pool
    // (and share the same bound with ignore WalkParallel via helpers).
    // `--threads` / `--max-concurrency` / `0` / omit → see `concurrency::effective_threads`.
    concurrency::configure_global_pool(cli.global.threads);

    let mut writer = NdjsonWriter::new(stdout);
    let shutdown = signal::get_or_install_handlers()?;
    // Rules Rust one-shot: global wall-clock deadline (cooperative cancel).
    signal::arm_global_timeout(cli.global.timeout_secs);
    let workspace = cli.global.resolve_workspace()?;
    let config = crate::config::load_config(&workspace, cli.global.config.as_deref())?;
    crate::config::validate_fuzzy(&config.fuzzy)?;
    let defaults = &config.defaults;
    let fuzzy_cfg = &config.fuzzy;

    // G119 L3 — autonomous startup `wal-heal` pass. Walks the workspace
    // once, removes every `Committed`/`Aborted` sidecar older than
    // `threshold_secs` (default 3600s = 1h), and is bounded by a
    // 100ms wall-clock budget so the per-invocation overhead is
    // predictable. Disabled via `--no-auto-heal` or
    // `ATOMWRITE_WAL_NO_AUTO_HEAL=1` for tight local loops and benchmarks.
    // `Started` journals are NEVER removed automatically — they are the
    // orphans worth operator attention.
    if !cli.global.no_auto_heal {
        match crate::wal::auto_heal_on_startup(&workspace, 3600, 100) {
            Ok(report) if report.removed > 0 || report.malformed > 0 => {
                tracing::info!(
                    removed = report.removed,
                    preserved = report.preserved,
                    malformed = report.malformed,
                    bytes_reclaimed = report.bytes_reclaimed,
                    "G119 L3: startup wal-heal reaped stale sidecars"
                );
            }
            Ok(_) => {
                tracing::debug!("G119 L3: startup wal-heal found nothing to reap");
            }
            Err(e) => {
                // Best-effort: a failed heal pass must never block the
                // actual subcommand. Operators see the warning on stderr.
                tracing::warn!(
                    error = %e,
                    workspace = %workspace.display(),
                    "G119 L3: startup wal-heal failed; continuing without reaping"
                );
            }
        }
    }

    let result = match &cli.command {
        Commands::Read(args) => commands::read::cmd_read(args, &cli.global, &mut writer),
        Commands::Write(args) => {
            commands::write::cmd_write(args, &cli.global, stdin, &mut writer, &shutdown, defaults)
        }
        Commands::Edit(args) => commands::edit::cmd_edit(
            args,
            &cli.global,
            stdin,
            &mut writer,
            &workspace,
            defaults,
            fuzzy_cfg,
            stdin_is_tty,
        ),
        Commands::Search(args) => {
            commands::search::cmd_search(args, &cli.global, &mut writer, &shutdown)
        }
        Commands::Replace(args) => {
            commands::replace::cmd_replace(
                args,
                &cli.global,
                &mut writer,
                &shutdown,
                defaults,
                fuzzy_cfg,
            )
        }
        Commands::Hash(args) => commands::hash::cmd_hash(args, &cli.global, stdin, &mut writer),
        Commands::Delete(args) => {
            commands::delete::cmd_delete(args, &cli.global, &mut writer, defaults)
        }
        Commands::Count(args) => commands::count::cmd_count(args, &cli.global, &mut writer),
        Commands::Diff(args) => commands::diff::cmd_diff(args, &cli.global, &mut writer),
        Commands::Move(args) => {
            commands::r#move::cmd_move(args, &cli.global, &mut writer, defaults)
        }
        Commands::Copy(args) => commands::copy::cmd_copy(args, &cli.global, &mut writer, defaults),
        Commands::List(args) => commands::list::cmd_list(args, &cli.global, &mut writer),
        Commands::Extract(args) => commands::extract::cmd_extract(args, stdin, &mut writer),
        Commands::Calc(args) => commands::calc::cmd_calc(args, stdin, &mut writer),
        Commands::Regex(args) => commands::regex_gen::cmd_regex(args, stdin, &mut writer),
        Commands::Transform(args) => {
            commands::transform::cmd_transform(args, &cli.global, &mut writer, &shutdown, defaults)
        }
        Commands::Batch(args) => {
            if args.input_schema {
                return commands::batch::emit_input_schema(&mut writer);
            }
            commands::batch::cmd_batch(
                &cli.global,
                stdin,
                &mut writer,
                args.dry_run,
                args.transaction,
                args.file.as_deref(),
                &shutdown,
                &args.backup_opts,
                defaults,
                fuzzy_cfg,
            )
        }
        Commands::Scope(args) => {
            commands::scope::cmd_scope(args, &cli.global, &mut writer, &shutdown, defaults)
        }
        Commands::Backup(args) => commands::backup::cmd_backup(args, &cli.global, &mut writer),
        Commands::Rollback(args) => {
            commands::rollback::cmd_rollback(args, &cli.global, &mut writer, defaults)
        }
        Commands::Apply(args) => {
            commands::apply::cmd_apply(args, &cli.global, stdin, &mut writer, defaults)
        }
        Commands::Set(args) => commands::set::cmd_set(args, &cli.global, &mut writer, defaults),
        Commands::Get(args) => commands::get::cmd_get(args, &cli.global, &mut writer),
        Commands::Del(args) => commands::del::cmd_del(args, &cli.global, &mut writer, defaults),
        Commands::Case(args) => commands::case::cmd_case(args, &cli.global, &mut writer, defaults),
        Commands::Query(args) => commands::query::cmd_query(args, &cli.global, &mut writer),
        Commands::Outline(args) => commands::outline::cmd_outline(args, &cli.global, &mut writer),
        Commands::WalStats(args) => {
            commands::wal_stats::cmd_wal_stats(args, &cli.global, &mut writer)
        }
        Commands::WalHeal(args) => {
            commands::wal_stats::cmd_wal_heal(args, &cli.global, &mut writer)
        }
        Commands::PruneBackups(args) => {
            commands::prune_backups::cmd_prune_backups(args, &cli.global, &mut writer)
        }
        Commands::EditLoop(args) => {
            // Pass the already-locked `stdin` from `run()`'s argument
            // instead of re-locking with `std::io::stdin().lock()`. The
            // latter is a deadlock: `Stdin::lock` is non-recursive, so
            // attempting to re-acquire it from the same thread that
            // holds it (via `main.rs:106 stdin.lock()`) blocks forever.
            // See audit 2026-06-17.
            commands::edit_loop::cmd_edit_loop(
                args,
                &cli.global,
                stdin,
                &mut writer,
                defaults,
                fuzzy_cfg,
            )
        }
        Commands::Verify(args) => {
            let hash_args = cli_args::HashArgs {
                paths: vec![args.path.clone()],
                verify: Some(args.checksum.clone()),
                stdin: false,
                recursive: false,
                exclude: Vec::new(),
            };
            commands::hash::cmd_hash(&hash_args, &cli.global, stdin, &mut writer)
        }
        Commands::SemanticMerge(args) => commands::semantic_merge::cmd_semantic_merge(
            args,
            &cli.global,
            &mut writer,
            &shutdown,
            defaults,
        ),
        Commands::Sparse(args) => {
            commands::sparse::cmd_sparse(args, &cli.global, &mut writer, &shutdown, defaults)
        }
        Commands::Recipe(args) => {
            commands::recipe::cmd_recipe(
                args,
                &cli.global,
                &mut writer,
                &shutdown,
                defaults,
                fuzzy_cfg,
            )
        }
        Commands::Stat(args) => {
            let mut a = args.clone();
            a.stat = true;
            commands::read::cmd_read(&a, &cli.global, &mut writer)
        }
        Commands::AgentSurface(args) => commands::agent_surface::cmd_agent_surface(
            args,
            &cli.global,
            &mut writer,
            &shutdown,
            defaults,
        ),
        Commands::Watch(args) => {
            commands::watch::cmd_watch(args, &cli.global, &mut writer, &shutdown, defaults)
        }
        Commands::Codemod(args) => {
            commands::codemod::cmd_codemod(args, &cli.global, &mut writer, &shutdown, defaults)
        }
        Commands::SemanticSearch(args) => commands::semantic_search::cmd_semantic_search(
            args,
            &cli.global,
            &mut writer,
            &shutdown,
            defaults,
        ),
        Commands::Doctor(args) => {
            commands::doctor::cmd_doctor(args, &cli.global, &mut writer, &shutdown, defaults)
        }
        Commands::Locale(args) => {
            commands::locale_cmd::cmd_locale(args, &cli.global, &mut writer, &shutdown, defaults)
        }
        Commands::CommandsTree(args) => commands::command_tree::cmd_commands_tree(
            args,
            &cli.global,
            &mut writer,
            &shutdown,
            defaults,
        ),
        Commands::Completions(_) => unreachable!("completions handled in prescan_json_schema"),
    };

    let _ = writer.flush();
    result
}

fn generate_completions(args: &cli::CompletionsArgs, mut out: impl Write) -> Result<()> {
    use clap::CommandFactory;
    let shell = match args.shell {
        cli::ShellType::Bash => clap_complete::Shell::Bash,
        cli::ShellType::Zsh => clap_complete::Shell::Zsh,
        cli::ShellType::Fish => clap_complete::Shell::Fish,
        cli::ShellType::PowerShell => clap_complete::Shell::PowerShell,
        cli::ShellType::Elvish => clap_complete::Shell::Elvish,
    };

    if args.install {
        // Install to XDG data directory
        let xdg_data = std::env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| std::path::PathBuf::from(h).join(".local").join("share"))
            })
            .ok_or_else(|| anyhow::anyhow!("cannot determine XDG data directory"))?;

        let (subdir, filename) = match args.shell {
            cli::ShellType::Bash => ("bash-completion/completions", "atomwrite"),
            cli::ShellType::Zsh => ("zsh/site-functions", "_atomwrite"),
            cli::ShellType::Fish => ("fish/vendor_completions.d", "atomwrite.fish"),
            cli::ShellType::PowerShell => ("powershell/Completions", "_atomwrite.ps1"),
            cli::ShellType::Elvish => ("elvish/lib", "atomwrite.elv"),
        };

        let install_dir = xdg_data.join(subdir);
        std::fs::create_dir_all(&install_dir)
            .map_err(|e| anyhow::anyhow!("cannot create {}: {e}", install_dir.display()))?;
        let install_path = install_dir.join(filename);

        let mut file = std::fs::File::create(&install_path)
            .map_err(|e| anyhow::anyhow!("cannot create {}: {e}", install_path.display()))?;
        clap_complete::generate(shell, &mut cli::Cli::command(), "atomwrite", &mut file);

        let path_str = install_path.display().to_string();
        writeln!(out, "{{\"type\":\"installed\",\"path\":\"{path_str}\"}}")?;
        Ok(())
    } else {
        clap_complete::generate(shell, &mut cli::Cli::command(), "atomwrite", &mut out);
        Ok(())
    }
}
