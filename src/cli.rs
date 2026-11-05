// SPDX-License-Identifier: MIT OR Apache-2.0

//! CLI argument parser and subcommand dispatch definitions.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueHint};

pub use crate::cli_args::*;

/// Build-time version line for clap (`env!` / `option_env!` — compile-time macros).
///
/// `ATOMWRITE_GIT_SHA` and `TARGET` are injected by `build.rs` via
/// `cargo:rustc-env`. Outside a git repository the source is `"no-git"`.
fn version_string() -> String {
    format!(
        "{} ({}) {}",
        env!("CARGO_PKG_VERSION"),
        env!("ATOMWRITE_GIT_SHA"),
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
    // Derived from the error enum, so `--help`, `agent-surface` and the README
    // table list the same statuses.
    after_help = crate::commands::agent_surface::exit_codes_help(),
    propagate_version = true
)]
/// Top-level CLI definition parsed by clap.
///
/// # Examples
///
/// ```
/// use atomwrite::cli::{Cli, Commands};
/// use clap::Parser;
///
/// let cli = Cli::try_parse_from(["atomwrite", "--workspace", ".", "read", "--head", "5", "src/main.rs"])?;
/// assert!(matches!(cli.command, Commands::Read(_)));
/// assert_eq!(cli.global.timeout_secs, 120);
/// # Ok::<(), clap::Error>(())
/// ```
pub struct Cli {
    /// Global flags shared across all subcommands.
    #[command(flatten)]
    pub global: GlobalArgs,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

include!("cli_global.inc.rs");
include!("cli_examples.inc.rs");

impl Cli {
    /// Parser with the parts that cannot be written as an attribute applied.
    ///
    /// `config --help` publishes the key table, so it is generated from the
    /// same `KEYS` the verb reads. The derive attribute can only hold a
    /// `'static` string, and a hand-written copy of the table is what drifted
    /// before (22 rows against 35). `main` builds the parser here instead of
    /// calling `try_parse`, so the page printed from argv carries the table the
    /// verb itself reads; `agent-surface` and `commands` publish the summary
    /// only, and completions carry no prose at all.
    #[must_use]
    pub fn parser() -> clap::Command {
        use clap::CommandFactory;
        let keys = crate::commands::config_cmd::keys_help_block();
        // The 34 global flags carry `hide_short_help`, so `-h` lists the verb's
        // own flags first. That trades one line of footer for the several
        // hundred lines the shared block cost in every short help page: a
        // caller that asks a command what it accepts no longer reads 34 flags
        // it does not take there.
        Cli::command()
            // The root page keeps the same pointer, because the schema door is
            // one of the flags the short help no longer prints.
            // The root page keeps the status table and gains the pointer, so a
            // caller that asks the product what it answers still reads every
            // code, and still learns where the shared flags live.
            .after_help(format!(
                "{}\n{}",
                crate::commands::agent_surface::exit_codes_help(),
                GLOBALS_FOOTER
            ))
            .mut_subcommands(|sub| sub.after_help(GLOBALS_FOOTER))
            .mut_subcommand("config", |sub| {
                // The generated table plus the scopes block, in one blank line.
                sub.after_long_help(format!("{keys}\n{}", EX_CONFIG_SCOPES))
            })
    }
}

impl GlobalArgs {
    /// Return the workspace root as an absolute path, defaulting to the current directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomwrite::cli::Cli;
    /// use clap::Parser;
    ///
    /// let cli = Cli::try_parse_from(["atomwrite", "--workspace", "/srv/project", "doctor"])?;
    /// assert_eq!(cli.global.resolve_workspace()?, std::path::PathBuf::from("/srv/project"));
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be determined.
    pub fn resolve_workspace(&self) -> Result<PathBuf> {
        let base = match &self.workspace {
            Some(p) => p.clone(),
            // Classified as a typed error so an unreadable current directory
            // keeps its semantic exit (13 or 74) instead of falling to 255.
            None => std::env::current_dir().map_err(|e| {
                crate::error::AtomwriteError::from_io_at(std::path::Path::new("."), e)
            })?,
        };
        if base.is_relative() {
            let cwd = std::env::current_dir().map_err(|e| {
                crate::error::AtomwriteError::from_io_at(std::path::Path::new("."), e)
            })?;
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

/// Side-effect acknowledgement policy resolved from the global flags.
///
/// Commands that mutate declare which policy they require; a policy the
/// caller did not grant fails closed before any filesystem work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffectMode {
    /// Command has no side effect gated by acknowledgement.
    Unspecified,
    /// Command mutates and requires `--confirm-side-effect`.
    Acknowledged,
}

impl GlobalArgs {
    /// Resolve the acknowledgement policy of this invocation.
    ///
    /// The flag is only meaningful on a verb that changes bytes on disk. On any
    /// other verb it is not a harmless no-op: it is a caller telling the CLI
    /// they believe the run writes, and the CLI reporting success would confirm
    /// a belief it never checked. The dispatch refuses it there instead.
    pub fn side_effect_mode(&self) -> SideEffectMode {
        if self.confirm_side_effect {
            SideEffectMode::Acknowledged
        } else {
            SideEffectMode::Unspecified
        }
    }

    /// Resolve whether the process is agent-hosted and must never prompt.
    ///
    /// True when `--non-interactive` is set or stdin is not a terminal. A
    /// one-shot CLI that can block on a terminal prompt is the failure this
    /// covers, so a pipe or `/dev/null` counts as non-interactive.
    pub fn non_interactive(&self, stdin_is_tty: bool) -> bool {
        self.non_interactive || !stdin_is_tty
    }
}

/// Available subcommands for the CLI.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Read files with metadata, checksum, and optional content; --symbol, --pages, --max-tokens and --crop pick the slice
    #[command(after_long_help = EX_READ)]
    Read(ReadArgs),

    /// Create or overwrite files atomically via stdin
    #[command(after_long_help = EX_WRITE)]
    Write(WriteArgs),

    /// Surgically edit files by line number, text marker, or exact match
    #[command(after_long_help = EX_EDIT)]
    Edit(EditArgs),

    /// Search file contents in parallel (ripgrep engine); --format text answers one line per match
    #[command(after_long_help = EX_SEARCH)]
    Search(SearchArgs),

    /// Replace text across files in parallel with atomic writes
    #[command(after_long_help = EX_REPLACE)]
    Replace(ReplaceArgs),

    /// Calculate BLAKE3 checksums for files, or for stdin when --stdin is given
    #[command(after_long_help = EX_HASH)]
    Hash(HashArgs),

    /// Delete files, copying one only when the call asks (`--backup` or `[defaults].backup = true`)
    #[command(after_long_help = EX_DELETE)]
    Delete(DeleteArgs),

    /// Count lines, matches, or files by extension
    #[command(after_long_help = EX_COUNT)]
    Count(CountArgs),

    /// Compare two files or file vs stdin (unified diff)
    #[command(after_long_help = EX_DIFF)]
    Diff(DiffArgs),

    /// Move or rename files/dirs atomically (same-FS dir rename; cross-device dirs: copy -r + delete)
    #[command(after_long_help = EX_MOVE)]
    Move(MoveArgs),

    /// Copy files with checksum verification and atomic destination
    #[command(after_long_help = EX_COPY)]
    Copy(CopyArgs),

    /// List project file structure with metadata (NDJSON per entry)
    #[command(after_long_help = EX_LIST)]
    List(ListArgs),

    /// Extract fields from NDJSON stdin or text columns
    #[command(after_long_help = EX_EXTRACT)]
    Extract(ExtractArgs),

    /// Evaluate math expressions and unit conversions (fend engine)
    #[command(after_long_help = EX_CALC)]
    Calc(CalcArgs),

    /// Generate regex from examples (grex engine)
    #[command(after_long_help = EX_REGEX)]
    Regex(RegexArgs),

    /// Structural code search and rewrite via AST patterns (ast-grep engine)
    #[command(after_long_help = EX_TRANSFORM)]
    Transform(TransformArgs),

    /// Grammatical scoping: select AST categories and apply actions (delete, upper, lower, etc.)
    #[command(after_long_help = EX_SCOPE)]
    Scope(crate::commands::scope::ScopeArgs),

    /// Execute multiple operations from an NDJSON manifest (batch mode)
    #[command(after_long_help = EX_BATCH)]
    Batch(BatchArgs),

    /// Create timestamped backups of files with BLAKE3 checksums
    #[command(after_long_help = EX_BACKUP)]
    Backup(BackupArgs),

    /// Restore a file from a previous backup
    #[command(after_long_help = EX_ROLLBACK)]
    Rollback(RollbackArgs),

    /// Apply a patch (unified diff, SEARCH/REPLACE, or full file) from stdin
    #[command(after_long_help = EX_APPLY)]
    Apply(ApplyArgs),
    /// Set a value in a TOML or JSON file by dotted key path
    #[command(after_long_help = EX_SET)]
    Set(crate::cli_args::SetArgs),
    /// Read a value from a TOML or JSON file by dotted key path
    #[command(after_long_help = EX_GET)]
    Get(crate::cli_args::GetArgs),
    /// Delete a key from a TOML or JSON file by dotted key path
    #[command(after_long_help = EX_DEL)]
    Del(crate::cli_args::DelArgs),
    /// Convert identifier case in source files
    #[command(after_long_help = EX_CASE)]
    Case(crate::cli_args::CaseArgs),
    /// Run a tree-sitter S-expression query against a file
    #[command(after_long_help = EX_QUERY)]
    Query(crate::cli_args::QueryArgs),
    /// Extract the high-level structure (functions, classes, structs, enums) from a source file
    #[command(after_long_help = EX_OUTLINE)]
    Outline(crate::cli_args::OutlineArgs),

    /// Count journal sidecars by terminal state, size, age and directory
    #[command(after_long_help = EX_WAL_STATS)]
    WalStats(crate::cli_args::WalStatsArgs),

    /// Remove terminal journals older than the threshold
    #[command(after_long_help = EX_WAL_HEAL)]
    WalHeal(crate::cli_args::WalHealArgs),

    /// Three-way line-based merge for multi-agent writes (not AST based)
    #[command(after_long_help = EX_SEMANTIC_MERGE)]
    SemanticMerge(crate::commands::semantic_merge::SemanticMergeArgs),
    /// List, read and outline files under a hard file budget
    #[command(after_long_help = EX_SPARSE)]
    Sparse(crate::commands::sparse::SparseArgs),
    /// List and run named multi-step recipes
    #[command(after_long_help = EX_RECIPE)]
    Recipe(crate::commands::recipe::RecipeArgs),
    /// Metadata only, alias of `read --stat`
    #[command(after_long_help = EX_STAT)]
    Stat(crate::cli_args::ReadArgs),
    /// Print the agent tool manifest as NDJSON
    #[command(after_long_help = EX_AGENT_SURFACE)]
    AgentSurface(crate::commands::agent_surface::AgentSurfaceArgs),
    /// Watch filesystem events (needs a build with the `watch` feature)
    #[command(after_long_help = EX_WATCH)]
    Watch(crate::commands::watch::WatchArgs),
    /// Extract a bounded signal from PDF, OOXML and image headers
    ///
    /// Reads PDF, OOXML (document / spreadsheet / presentation) and image
    /// headers, emitting a bounded signal and never raw artifact bytes.
    #[command(after_long_help = EX_EXTRACT_ARTIFACT)]
    ExtractArtifact(crate::commands::extract_artifact::ExtractArtifactArgs),
    /// Run a multi-rule AST codemod campaign from a YAML rules file
    #[command(after_long_help = EX_CODEMOD)]
    Codemod(crate::commands::codemod::CodemodArgs),
    /// Offline token-overlap (Jaccard) search
    #[command(after_long_help = EX_SEMANTIC_SEARCH)]
    SemanticSearch(crate::commands::semantic_search::SemanticSearchArgs),
    /// Diagnose environment and dependencies (agent host readiness)
    #[command(after_long_help = EX_DOCTOR)]
    Doctor(crate::commands::doctor::DoctorArgs),

    /// Create agent staging dir under workspace (never OS /tmp)
    #[command(after_long_help = EX_SCRATCH)]
    Scratch(crate::commands::scratch::ScratchArgs),

    /// Show resolved UI locale / persist preference (XDG)
    #[command(after_long_help = EX_LOCALE)]
    Locale(crate::commands::locale_cmd::LocaleArgs),

    /// Read or write persistent tuning keys (`config.toml`, `.atomwrite.toml`) without env vars
    // `after_long_help` is filled by `Cli::parser` from the live key table:
    // the attribute can only hold a `'static` string, and the copy in it had
    // drifted from `KEYS` (22 rows against 35).
    Config(crate::commands::config_cmd::ConfigArgs),

    /// Emit full command tree as JSON for agent discovery
    #[command(name = "commands", after_long_help = EX_COMMANDS)]
    CommandsTree(crate::commands::command_tree::CommandsTreeArgs),

    /// Generate shell completions for bash, zsh, fish, elvish, or powershell
    #[command(after_long_help = EX_COMPLETIONS)]
    Completions(CompletionsArgs),

    /// Prune `.bak.YYYYMMDD_HHMMSS` backups by age or count
    #[command(after_long_help = EX_PRUNE_BACKUPS)]
    PruneBackups(PruneBackupsArgs),

    /// Apply N `old`/`new` pairs from stdin (JSON array or NDJSON) in one write
    #[command(after_long_help = EX_EDIT_LOOP)]
    EditLoop(EditLoopArgs),

    /// Verify file integrity by comparing BLAKE3 checksum
    #[command(after_long_help = EX_VERIFY)]
    Verify(crate::cli_args::VerifyArgs),
}
