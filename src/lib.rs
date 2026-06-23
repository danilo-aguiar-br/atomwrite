// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file operations CLI library for LLM agents.

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![warn(clippy::doc_markdown)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::needless_return)]

rust_i18n::i18n!("locales", fallback = "en");

/// Atomic file write pipeline.
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
/// Smart file reading with memmap2 for large files.
pub mod file_io;
/// Shared language utilities for AST commands.
pub mod lang_utils;
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
/// G72 — Real syntax check via `tree-sitter-language-pack` (v0.1.12).
pub mod syntax_check;
/// G114 — Write-Ahead Log (WAL) sidecar for crash recovery (v0.1.12).
pub mod wal;
/// Extended attribute (xattr) save and restore for atomic writes (G39).
pub mod xattr_restore;

use std::io::{Read, Write};

use anyhow::Result;

use crate::cli::{Cli, Commands};
use crate::output::NdjsonWriter;

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
pub fn run(cli: &Cli, stdin: impl Read, stdout: impl Write) -> Result<()> {
    if cli.global.json_schema {
        return emit_json_schema(&cli.command, stdout);
    }
    if let Commands::Completions(args) = &cli.command {
        return generate_completions(args, stdout);
    }

    if cli.global.json {
        tracing::debug!("--json is a no-op; output is always NDJSON");
    }

    if let Some(threads) = cli.global.threads {
        let n = if threads == 0 { num_cpus() } else { threads };
        if let Err(e) = rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
        {
            tracing::warn!(error = %e, "failed to configure rayon global pool");
        }
    }

    let mut writer = NdjsonWriter::new(stdout);
    let shutdown = signal::get_or_install_handlers()?;
    let workspace = cli.global.resolve_workspace()?;

    // G119 L3 — autonomous startup `wal-heal` pass. Walks the workspace
    // once, removes every `Committed`/`Aborted` sidecar older than
    // `threshold_secs` (default 3600s = 1h), and is bounded by a
    // 100ms wall-clock budget so the per-invocation overhead is
    // predictable. Disabled via `--no-auto-heal` or
    // `ATOMWRITE_WAL_NO_AUTO_HEAL=1` for tight CI loops and benchmarks.
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
            commands::write::cmd_write(args, &cli.global, stdin, &mut writer, &shutdown)
        }
        Commands::Edit(args) => {
            commands::edit::cmd_edit(args, &cli.global, stdin, &mut writer, &workspace)
        }
        Commands::Search(args) => {
            commands::search::cmd_search(args, &cli.global, &mut writer, &shutdown)
        }
        Commands::Replace(args) => {
            commands::replace::cmd_replace(args, &cli.global, &mut writer, &shutdown)
        }
        Commands::Hash(args) => commands::hash::cmd_hash(args, &cli.global, stdin, &mut writer),
        Commands::Delete(args) => commands::delete::cmd_delete(args, &cli.global, &mut writer),
        Commands::Count(args) => commands::count::cmd_count(args, &cli.global, &mut writer),
        Commands::Diff(args) => commands::diff::cmd_diff(args, &cli.global, &mut writer),
        Commands::Move(args) => commands::r#move::cmd_move(args, &cli.global, &mut writer),
        Commands::Copy(args) => commands::copy::cmd_copy(args, &cli.global, &mut writer),
        Commands::List(args) => commands::list::cmd_list(args, &cli.global, &mut writer),
        Commands::Extract(args) => commands::extract::cmd_extract(args, stdin, &mut writer),
        Commands::Calc(args) => commands::calc::cmd_calc(args, stdin, &mut writer),
        Commands::Regex(args) => commands::regex_gen::cmd_regex(args, stdin, &mut writer),
        Commands::Transform(args) => {
            commands::transform::cmd_transform(args, &cli.global, &mut writer, &shutdown)
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
                args.keep_backup,
            )
        }
        Commands::Scope(args) => {
            commands::scope::cmd_scope(args, &cli.global, &mut writer, &shutdown)
        }
        Commands::Backup(args) => commands::backup::cmd_backup(args, &cli.global, &mut writer),
        Commands::Rollback(args) => {
            commands::rollback::cmd_rollback(args, &cli.global, &mut writer)
        }
        Commands::Apply(args) => commands::apply::cmd_apply(args, &cli.global, stdin, &mut writer),
        Commands::Set(args) => commands::set::cmd_set(args, &cli.global, &mut writer),
        Commands::Get(args) => commands::get::cmd_get(args, &cli.global, &mut writer),
        Commands::Del(args) => commands::del::cmd_del(args, &cli.global, &mut writer),
        Commands::Case(args) => commands::case::cmd_case(args, &cli.global, &mut writer),
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
            commands::edit_loop::cmd_edit_loop(args, &cli.global, stdin, &mut writer)
        }
        Commands::Verify(args) => {
            let hash_args = cli_args::HashArgs {
                paths: vec![args.path.clone()],
                verify: Some(args.checksum.clone()),
                stdin: false,
                recursive: false,
            };
            commands::hash::cmd_hash(&hash_args, &cli.global, stdin, &mut writer)
        }
        Commands::Completions(_) => unreachable!("completions handled in prescan_json_schema"),
    };

    let _ = writer.flush();
    result
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
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
