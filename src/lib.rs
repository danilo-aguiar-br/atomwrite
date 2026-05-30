// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file operations CLI library for LLM agents.

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![warn(clippy::doc_markdown)]
#![doc(html_root_url = "https://docs.rs/atomwrite/0.1.0")]

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
/// Named constants for buffer sizes, thresholds, and identifiers.
pub mod constants;
/// Domain-specific error types.
pub mod error;
/// Smart file reading with memmap2 for large files.
pub mod file_io;
/// Line ending detection and normalization.
pub mod line_endings;
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
        Commands::Hash(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Delete(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Count(_) => schemars::schema_for!(ndjson_types::Summary),
        Commands::Diff(_) => schemars::schema_for!(ndjson_types::DryRunPlan),
        Commands::Move(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Copy(_) => schemars::schema_for!(ndjson_types::WriteOutput),
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
        Commands::Completions(_) => schemars::schema_for!(ndjson_types::CalcOutput),
    };
    serde_json::to_writer_pretty(&mut out, &schema)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
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

    if let Some(threads) = cli.global.threads {
        let n = if threads == 0 { num_cpus() } else { threads };
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .ok();
    }

    let mut writer = NdjsonWriter::new(stdout);
    let shutdown = signal::get_or_install_handlers()?;
    let _workspace = cli.global.resolve_workspace()?;

    match &cli.command {
        Commands::Read(args) => commands::read::cmd_read(args, &cli.global, &mut writer),
        Commands::Write(args) => {
            commands::write::cmd_write(args, &cli.global, stdin, &mut writer, &shutdown)
        }
        Commands::Edit(args) => {
            commands::edit::cmd_edit(args, &cli.global, stdin, &mut writer, &_workspace)
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
        Commands::Batch(args) => commands::batch::cmd_batch(
            &cli.global,
            stdin,
            &mut writer,
            args.dry_run,
            args.transaction,
        ),
        Commands::Scope(args) => {
            commands::scope::cmd_scope(args, &cli.global, &mut writer, &shutdown)
        }
        Commands::Backup(args) => commands::backup::cmd_backup(args, &cli.global, &mut writer),
        Commands::Rollback(args) => {
            commands::rollback::cmd_rollback(args, &cli.global, &mut writer)
        }
        Commands::Apply(args) => commands::apply::cmd_apply(args, &cli.global, stdin, &mut writer),
        Commands::Completions(_) => unreachable!(),
    }
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
    };
    clap_complete::generate(shell, &mut cli::Cli::command(), "atomwrite", &mut out);
    Ok(())
}
