// SPDX-License-Identifier: MIT OR Apache-2.0

//! CLI argument parser and subcommand dispatch definitions.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueHint};

pub use crate::cli_args::*;

/// Build-time version line for clap (`env!` / `option_env!` — compile-time macros).
///
/// `ATOMWRITE_GIT_SHA` and `TARGET` are injected by `build.rs` via
/// `cargo:rustc-env`. When git is unavailable the SHA falls back to `"unknown"`.
fn version_string() -> String {
    format!(
        "{} ({}) {}",
        env!("CARGO_PKG_VERSION"),
        option_env!("ATOMWRITE_GIT_SHA").unwrap_or("unknown"),
        env!("TARGET"),
    )
}

#[derive(Parser, Debug)]
#[command(
    name = "atomwrite",
    version = version_string(),
    author = env!("CARGO_PKG_AUTHORS"),
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
    #[arg(
        long,
        global = true,
        value_hint = ValueHint::DirPath,
        help = "Workspace root for path jail validation"
    )]
    pub workspace: Option<PathBuf>,

    /// Explicit config file path (overrides workspace `.atomwrite.toml` and XDG config).
    #[arg(
        long,
        global = true,
        value_hint = ValueHint::FilePath,
        help = "Path to .atomwrite.toml (explicit override; fails if missing/malformed)"
    )]
    pub config: Option<PathBuf>,

    /// Color output mode.
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto, help = "Control colored output")]
    pub color: ColorChoice,

    /// Disable colored output (equivalent to --color never).
    #[arg(long, global = true, help = "Disable colored output", action = clap::ArgAction::SetTrue)]
    pub no_color: bool,

    /// Write PID to this path after signal handlers install (test harness only).
    ///
    /// G-007: replaces env readiness probes. One-shot agents normally omit this.
    #[arg(
        long = "ready-file",
        global = true,
        value_hint = ValueHint::FilePath,
        help = "Write PID after signal handlers are ready (test harness; no product env)"
    )]
    pub ready_file: Option<PathBuf>,

    /// Disable .gitignore filtering.
    #[arg(long, global = true, help = "Do not respect .gitignore files", action = clap::ArgAction::SetTrue)]
    pub no_gitignore: bool,

    /// Include hidden files and directories.
    #[arg(long, global = true, help = "Include hidden files and directories", action = clap::ArgAction::SetTrue)]
    pub hidden: bool,

    /// Follow symbolic links during traversal.
    #[arg(long, global = true, help = "Follow symbolic links", action = clap::ArgAction::SetTrue)]
    pub follow_symlinks: bool,

    /// Parallel worker bound for multi-file / multi-path commands.
    ///
    /// Sets both `ignore::WalkBuilder::threads` (directory fan-out) and the
    /// process-wide rayon pool (CPU-bound `par_iter` helpers: hash, backup,
    /// recursive delete/copy, semantic-search scoring, non-txn batch, …).
    ///
    /// * omitted → all logical CPUs, RAM-capped (`concurrency` module formula)
    /// * `0` → same as omitted (explicit “use all cores”)
    /// * `N` → exactly N workers (deterministic tests / constrained hosts)
    ///
    /// Alias: `--max-concurrency` (Rules Rust bounded-concurrency surface).
    /// Bound is **CLI-only** (`--threads` / `--max-concurrency`); atomwrite does
    /// not document or require process env knobs for concurrency (G-007 / O-009).
    #[arg(
        short = 'j',
        long = "threads",
        visible_alias = "max-concurrency",
        global = true,
        value_name = "N",
        help = "Max concurrent workers for walks + rayon (0 = all cores; default = all cores, RAM-capped). Alias: --max-concurrency"
    )]
    pub threads: Option<usize>,

    /// Maximum allowed file size in bytes.
    #[arg(
        long,
        global = true,
        help = "Maximum file size in bytes (default: 1GB, reject larger)"
    )]
    pub max_filesize: Option<u64>,

    /// Global operation timeout in seconds (one-shot default: [`crate::constants::DEFAULT_TIMEOUT_SECS`]).
    ///
    /// When non-zero, a watchdog sets the cooperative cancel flag after N
    /// seconds (exit 124). Alias: `--timeout`. Pass `0` to disable (not
    /// recommended for agents — pure CPU sections must poll the flag).
    #[arg(
        long = "timeout-secs",
        visible_alias = "timeout",
        global = true,
        default_value_t = crate::constants::DEFAULT_TIMEOUT_SECS,
        help = "Global operation timeout in seconds (default from constants::DEFAULT_TIMEOUT_SECS=120; 0 = disable; exit 124 on deadline)"
    )]
    pub timeout_secs: u64,

    /// Suppress NDJSON `progress` heartbeats (batch/replace).
    #[arg(
        long,
        global = true,
        help = "Disable NDJSON progress events on long operations",
        action = clap::ArgAction::SetTrue
    )]
    pub no_progress: bool,

    /// Emit JSON Schema for subcommand output and exit.
    #[arg(
        long,
        global = true,
        help = "Emit JSON Schema for the subcommand output and exit",
        action = clap::ArgAction::SetTrue
    )]
    pub json_schema: bool,

    /// Accepted for compatibility but ignored — output is always NDJSON.
    #[arg(long, global = true, hide = true, action = clap::ArgAction::SetTrue)]
    pub json: bool,

    /// Override locale for translated suggestions / human stderr (e.g. en, pt-BR).
    ///
    /// ADR-0037: long flag renamed `--lang` → `--locale` in v0.1.20 to
    /// free the `--lang` namespace for subcommand-level use (e.g.
    /// `scope --lang` as an alias for `--language`). Field name remains `lang`.
    ///
    /// Precedence (Rules Rust i18n): `--locale` → XDG preference → OS via
    /// `sys-locale` → `en`. NDJSON error **codes** and Display **messages**
    /// stay English; **suggestions** follow the locale.
    /// G-007: locale via CLI / XDG only (no product env knobs).
    #[arg(
        long = "locale",
        global = true,
        value_parser = crate::locale::parse_cli_locale,
        help = "Override locale (en, pt-BR); renamed from --lang in v0.1.20"
    )]
    pub lang: Option<String>,

    /// Skip the on-startup `wal-heal` pass (G119 L3). Default: every
    /// invocation walks the workspace and reaps stale `Committed`/
    /// `Aborted` sidecars older than 3600s within a 100ms wall-clock
    /// budget. Set this flag in tight local loops or in benchmarks that
    /// measure the subcommand cost in isolation.
    /// G-007: flag only (no `ATOMWRITE_WAL_NO_AUTO_HEAL` env).
    #[arg(
        long,
        global = true,
        help = "Skip startup wal-heal pass (G119 L3); default: run with 3600s threshold and 100ms budget",
        action = clap::ArgAction::SetTrue
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

    /// Resolve stderr/tracing ANSI policy (G-007: CLI only, no process env).
    pub fn color_mode(&self) -> crate::runtime::ColorMode {
        if self.no_color {
            return crate::runtime::ColorMode::Never;
        }
        match self.color {
            ColorChoice::Auto => crate::runtime::ColorMode::Auto,
            ColorChoice::Always => crate::runtime::ColorMode::Always,
            ColorChoice::Never => crate::runtime::ColorMode::Never,
        }
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

    /// Move or rename files/dirs atomically (same-FS dir rename; cross-device dirs: copy -r + delete)
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
    /// breakdown by directory (G119 L5 journal stats).
    WalStats(crate::cli_args::WalStatsArgs),

    /// Remove stale terminal journals older than the threshold (G119 L3).
    WalHeal(crate::cli_args::WalHealArgs),

    /// Three-way line-based merge (not AST/embedding) for multi-agent writes (v0.1.29 P1-1).
    SemanticMerge(crate::commands::semantic_merge::SemanticMergeArgs),
    /// Budgeted sparse list/read (v0.1.29 P1-2).
    Sparse(crate::commands::sparse::SparseArgs),
    /// Named multi-step recipes (v0.1.29 P1-4).
    Recipe(crate::commands::recipe::RecipeArgs),
    /// Metadata alias for `read --stat` (v0.1.29 P2-4).
    Stat(crate::cli_args::ReadArgs),
    /// Agent tool surface manifesto without MCP (v0.1.29 P2-5).
    AgentSurface(crate::commands::agent_surface::AgentSurfaceArgs),
    /// Watch filesystem events (feature `watch`) (v0.1.29 P3-1).
    Watch(crate::commands::watch::WatchArgs),
    /// Multi-rule codemod campaign (v0.1.29 P3-3).
    Codemod(crate::commands::codemod::CodemodArgs),
    /// Offline token Jaccard search (v0.1.29 P3-2).
    SemanticSearch(crate::commands::semantic_search::SemanticSearchArgs),
    /// Diagnose environment and dependencies (agent host readiness)
    Doctor(crate::commands::doctor::DoctorArgs),

    /// Show resolved UI locale / persist preference (XDG)
    Locale(crate::commands::locale_cmd::LocaleArgs),

    /// Emit full command tree as JSON for agent discovery
    #[command(name = "commands")]
    CommandsTree(crate::commands::command_tree::CommandsTreeArgs),

    /// Generate shell completions for bash, zsh, fish, or powershell
    Completions(CompletionsArgs),

    /// v0.1.22 ADR-0040: prune `.bak.YYYYMMDD_HHMMSS` backups by age or count.
    PruneBackups(PruneBackupsArgs),

    /// v0.1.22 ADR-0039: apply N `old`/`new` pairs from NDJSON stdin in one write.
    EditLoop(EditLoopArgs),

    /// Verify file integrity by comparing BLAKE3 checksum
    Verify(crate::cli_args::VerifyArgs),
}
