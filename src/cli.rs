// SPDX-License-Identifier: MIT OR Apache-2.0

//! CLI argument parser and subcommand dispatch definitions.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

pub use crate::cli_args::*;

fn version_string() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let git_sha = option_env!("ATOMWRITE_GIT_SHA").unwrap_or("unknown");
    let target = env!("TARGET");
    format!("{version} ({git_sha}) {target}")
}

#[derive(Parser, Debug)]
#[command(
    name = "atomwrite",
    version = version_string(),
    about = "Atomic file operations CLI for LLM agents",
    long_about = "A single, self-contained Rust CLI that gives LLM agents superpowers \
                  for file operations. Every write is atomic (tempfile → fsync → rename), \
                  every output is NDJSON, every search is parallel.",
    propagate_version = true
)]
/// Top-level CLI definition parsed by clap.
pub struct Cli {
    /// Global flags shared across all subcommands.
    #[command(flatten)]
    pub global: GlobalArgs,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Args, Debug)]
/// Global flags shared across all subcommands.
pub struct GlobalArgs {
    /// Verbosity level (repeat for more: -v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count, global = true, help = "Increase verbosity (-v info, -vv debug, -vvv trace)")]
    pub verbose: u8,

    /// Quiet level (repeat for less: -q, -qq).
    #[arg(short, long, action = clap::ArgAction::Count, global = true, help = "Decrease verbosity (-q error, -qq off)")]
    pub quiet: u8,

    /// Workspace root for path jail validation.
    #[arg(long, global = true, help = "Workspace root for path jail validation")]
    pub workspace: Option<PathBuf>,

    /// Color output mode.
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto, help = "Control colored output")]
    pub color: ColorChoice,

    /// Disable colored output (equivalent to --color never).
    #[arg(long, global = true, help = "Disable colored output")]
    pub no_color: bool,

    /// Disable .gitignore filtering.
    #[arg(long, global = true, help = "Do not respect .gitignore files")]
    pub no_gitignore: bool,

    /// Include hidden files and directories.
    #[arg(long, global = true, help = "Include hidden files and directories")]
    pub hidden: bool,

    /// Follow symbolic links during traversal.
    #[arg(long, global = true, help = "Follow symbolic links")]
    pub follow_symlinks: bool,

    /// Number of parallel threads (0 = all cores). Env: `RAYON_NUM_THREADS`.
    // rayon respects RAYON_NUM_THREADS natively when --threads is not passed.
    #[arg(
        short = 'j',
        long,
        global = true,
        help = "Number of parallel threads (0 = all cores). Env: RAYON_NUM_THREADS"
    )]
    pub threads: Option<usize>,

    /// Maximum allowed file size in bytes.
    #[arg(
        long,
        global = true,
        help = "Maximum file size in bytes (default: 1GB, reject larger)"
    )]
    pub max_filesize: Option<u64>,

    /// Global operation timeout in seconds. 0 disables timeout.
    #[arg(
        long,
        global = true,
        default_value_t = 0u64,
        help = "Global operation timeout in seconds (0 = no timeout, default: 0)"
    )]
    pub timeout_secs: u64,

    /// Emit JSON Schema for subcommand output and exit.
    #[arg(
        long,
        global = true,
        help = "Emit JSON Schema for the subcommand output and exit"
    )]
    pub json_schema: bool,

    /// Accepted for compatibility but ignored — output is always NDJSON.
    #[arg(long, global = true, hide = true)]
    pub json: bool,

    /// Override locale for translated messages (e.g. en, pt-BR).
    ///
    /// ADR-0037: long flag renamed `--lang` → `--locale` in v0.1.20 to
    /// free the `--lang` namespace for subcommand-level use (e.g.
    /// `scope --lang` as an alias for `--language`). The env var
    /// `ATOMWRITE_LANG` and the Rust field name `lang` are unchanged
    /// to preserve backward compatibility for env-var consumers and
    /// programmatic API users. Existing `--lang` flag invocations
    /// will fail loudly with `unknown argument` — this is a deliberate
    /// breaking change in CLI surface, documented in CHANGELOG v0.1.20.
    #[arg(
        long = "locale",
        global = true,
        env = "ATOMWRITE_LANG",
        help = "Override locale (en, pt-BR); renamed from --lang in v0.1.20"
    )]
    pub lang: Option<String>,

    /// Skip the on-startup `wal-heal` pass (G119 L3). Default: every
    /// invocation walks the workspace and reaps stale `Committed`/
    /// `Aborted` sidecars older than 3600s within a 100ms wall-clock
    /// budget. Set this flag in tight CI loops or in benchmarks that
    /// measure the subcommand cost in isolation.
    #[arg(
        long,
        global = true,
        env = "ATOMWRITE_WAL_NO_AUTO_HEAL",
        help = "Skip startup wal-heal pass (G119 L3); default: run with 3600s threshold and 100ms budget"
    )]
    pub no_auto_heal: bool,
}

impl GlobalArgs {
    /// Return the workspace root as an absolute path, defaulting to the current directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be determined.
    pub fn resolve_workspace(&self) -> Result<PathBuf> {
        let base = match &self.workspace {
            Some(p) => p.clone(),
            None => std::env::current_dir()
                .map_err(|e| anyhow::anyhow!("cannot resolve workspace: {e}"))?,
        };
        if base.is_relative() {
            let cwd = std::env::current_dir()
                .map_err(|e| anyhow::anyhow!("cannot resolve workspace: {e}"))?;
            Ok(cwd.join(base))
        } else {
            Ok(base)
        }
    }

    /// Return the maximum allowed file size, defaulting to 1 GiB.
    pub fn effective_max_filesize(&self) -> u64 {
        self.max_filesize
            .unwrap_or(crate::constants::DEFAULT_MAX_FILESIZE)
    }
}

/// Terminal color output preference.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ColorChoice {
    /// Detect color support automatically.
    Auto,
    /// Always emit colored output.
    Always,
    /// Never emit colored output.
    Never,
}

/// Available subcommands for the CLI.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Read files with metadata, checksum, and optional content
    Read(ReadArgs),

    /// Create or overwrite files atomically via stdin
    Write(WriteArgs),

    /// Surgically edit files by line number, text marker, or exact match
    Edit(EditArgs),

    /// Search file contents in parallel (ripgrep engine)
    Search(SearchArgs),

    /// Replace text across files in parallel with atomic writes
    Replace(ReplaceArgs),

    /// Calculate BLAKE3 checksums for files
    Hash(HashArgs),

    /// Delete files with optional backup
    Delete(DeleteArgs),

    /// Count lines, matches, or files by extension
    Count(CountArgs),

    /// Compare two files or file vs stdin (unified diff)
    Diff(DiffArgs),

    /// Move or rename files atomically
    Move(MoveArgs),

    /// Copy files with checksum verification and atomic destination
    Copy(CopyArgs),

    /// List project file structure with metadata (NDJSON per entry)
    List(ListArgs),

    /// Extract fields from NDJSON stdin or text columns
    Extract(ExtractArgs),

    /// Evaluate math expressions and unit conversions (fend engine)
    Calc(CalcArgs),

    /// Generate regex from examples (grex engine)
    Regex(RegexArgs),

    /// Structural code search and rewrite via AST patterns (ast-grep engine)
    Transform(TransformArgs),

    /// Grammatical scoping: select AST categories and apply actions (delete, upper, lower, etc.)
    Scope(crate::commands::scope::ScopeArgs),

    /// Execute multiple operations from an NDJSON manifest (batch mode)
    Batch(BatchArgs),

    /// Create timestamped backups of files with BLAKE3 checksums
    Backup(BackupArgs),

    /// Restore a file from a previous backup
    Rollback(RollbackArgs),

    /// Apply a patch (unified diff, SEARCH/REPLACE, or full file) from stdin
    Apply(ApplyArgs),
    /// v14 Tier 3: set a value in a structured config file (TOML/JSON).
    Set(crate::cli_args::SetArgs),
    /// v14 Tier 3: get a value from a structured config file (TOML/JSON).
    Get(crate::cli_args::GetArgs),
    /// v14 Tier 3: delete a key from a structured config file (TOML/JSON).
    Del(crate::cli_args::DelArgs),
    /// v14 Tier 3: convert identifier case in source files.
    Case(crate::cli_args::CaseArgs),
    /// v14 Tier 3 (v0.1.12): tree-sitter S-expression query against a file.
    Query(crate::cli_args::QueryArgs),
    /// v14 Tier 3 (v0.1.12): extract high-level structure (functions, classes,
    /// structs, enums, etc.) from a source file.
    Outline(crate::cli_args::OutlineArgs),

    /// Snapshot of journal state: count by terminal state, size, age,
    /// breakdown by directory (G119 L5 telemetry).
    WalStats(crate::cli_args::WalStatsArgs),

    /// Remove stale terminal journals older than the threshold (G119 L3).
    WalHeal(crate::cli_args::WalHealArgs),

    /// Generate shell completions for bash, zsh, fish, or powershell
    Completions(CompletionsArgs),

    /// v0.1.22 ADR-0040: prune `.bak.YYYYMMDD_HHMMSS` backups by age or count.
    PruneBackups(PruneBackupsArgs),

    /// v0.1.22 ADR-0039: apply N `old`/`new` pairs from NDJSON stdin in one write.
    EditLoop(EditLoopArgs),

    /// Verify file integrity by comparing BLAKE3 checksum
    Verify(crate::cli_args::VerifyArgs),
}
