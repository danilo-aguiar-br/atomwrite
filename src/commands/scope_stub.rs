// SPDX-License-Identifier: MIT OR Apache-2.0
//! Stub when feature `ast` is disabled.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalArgs;
use crate::cli_args::BackupOpts;
use crate::config::DefaultsSection;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for the scope subcommand (shared shape with the full module).
#[derive(Args, Debug)]
pub struct ScopeArgs {
    /// Paths to search within.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,
    /// Source language for AST parsing.
    #[arg(short = 'l', long = "language", alias = "lang", required = true)]
    pub language: String,
    /// Prepared query name.
    #[arg(long)]
    pub query: Option<String>,
    /// Custom AST pattern.
    #[arg(long)]
    pub pattern: Option<String>,
    /// Delete matched content.
    #[arg(long)]
    pub delete: bool,
    /// Action to apply.
    #[arg(long, value_enum)]
    pub action: Option<ScopeAction>,
    /// Replacement text.
    #[arg(long)]
    pub replace_with: Option<String>,
    /// Include globs.
    #[arg(short = 'g', long, action = clap::ArgAction::Append)]
    pub include: Vec<String>,
    /// Exclude globs.
    #[arg(long, action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,
    /// Dry-run.
    #[arg(long)]
    pub dry_run: bool,
    /// Backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,
}

/// Available actions for the scope subcommand.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ScopeAction {
    /// Uppercase.
    Upper,
    /// Lowercase.
    Lower,
    /// Title case.
    Titlecase,
    /// Squeeze whitespace.
    Squeeze,
    /// Unicode symbols.
    Symbols,
    /// NFC normalize.
    Normalize,
}

/// Grammatical scope requires feature `ast`.
pub fn cmd_scope(
    _args: &ScopeArgs,
    _global: &GlobalArgs,
    _writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
    _defaults: &DefaultsSection,
) -> Result<()> {
    Err(AtomwriteError::ConfigInvalid {
        reason: "scope requires --features ast (included in default features)".into(),
    }
    .into())
}
