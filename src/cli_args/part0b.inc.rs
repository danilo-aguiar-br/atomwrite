// SPDX-License-Identifier: MIT OR Apache-2.0
// Subcommand argument structs, continued (batch, patch format, set, case, wal-stats).
// Included from part0.inc.rs.

/// Arguments for the batch subcommand.
#[derive(Args, Debug)]
pub struct BatchArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    /// Preview without executing.
    #[arg(long, help = "Show what would be done without executing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Manifest file path (default: stdin).
    #[arg(long, help = "Read manifest from file instead of stdin", value_hint = ValueHint::FilePath)]
    pub file: Option<PathBuf>,

    /// Execute all operations as a transaction (all-or-nothing).
    #[arg(
        long,
        help = "All-or-nothing: rollback all changes if any operation fails",
        action = clap::ArgAction::SetTrue
    )]
    pub transaction: bool,

    /// Emit JSON Schema for the NDJSON input manifest format.
    #[arg(long, help = "Print JSON Schema for the batch input manifest", action = clap::ArgAction::SetTrue)]
    pub input_schema: bool,

    /// Hint for NDJSON streaming: number of operations to buffer before
    /// emitting the summary line.
    ///
    /// atomwrite reads the manifest incrementally (one line at a time), so
    /// memory usage is O(1) regardless of this value. This flag only
    /// controls the granularity of the final `summary` event.
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_LIST_LIMIT,
        help = "Operations to buffer before emitting the summary line"
    )]
    pub batch_size: usize,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,
}

/// Patch format for the apply subcommand.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum PatchFormat {
    /// Auto-detect format from content.
    #[default]
    Auto,
    /// Standard unified diff (--- +++ @@ markers).
    Unified,
    /// SEARCH/REPLACE block format (<<<<<<< SEARCH markers).
    SearchReplace,
    /// Full file replacement.
    Full,
    /// Markdown-fenced diff (`` ```diff `` blocks).
    Markdown,
}

// ─────────────────────────────────────────────────────────────────────────────
// Structured config edits and identifier case conversion.
// ─────────────────────────────────────────────────────────────────────────────

/// Arguments for the `set` subcommand.
#[derive(Args, Debug)]
pub struct SetArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    #[arg(value_hint = ValueHint::FilePath)]
    /// Path to the structured config file (TOML or JSON).
    pub path: PathBuf,
    /// Dotted path to the key (e.g. `package.version`).
    pub key_path: String,
    /// New value (auto-coerced to bool/int/float/string).
    pub value: String,
    /// Report the planned change and write nothing.
    #[arg(long, help = "Report the planned change and write nothing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,
    /// Preserve original file timestamps.
    #[arg(long, help = "Preserve original mtime/atime", action = clap::ArgAction::SetTrue)]
    pub preserve_timestamps: bool,
}

/// Arguments for the `case` subcommand.
#[derive(Args, Debug)]
pub struct CaseArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    /// Target files or directories to rewrite (at least one; an empty list is a
    /// usage error).
    ///
    /// A directory is walked for regular files, honouring `--include`,
    /// `--exclude` and the workspace jail. Every path is named in the
    /// invocation line, never inherited from ambient state.
    #[arg(required = true, value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,
    /// Glob patterns for file inclusion when a directory is walked.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,
    /// Glob patterns for file exclusion when a directory is walked.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,
    /// Pairs of old new identifiers (must be even count).
    /// Required: there is no global identifier scan, so clap fails closed without it.
    #[arg(
        long = "subvert",
        num_args = 2,
        value_name = "OLD NEW",
        required = true,
        help = "Old and new identifier (repeat for multiple pairs; required — no global scan)"
    )]
    pub subvert: Vec<String>,
    /// Target case style for the new identifier.
    #[arg(long, value_enum, default_value_t = IdentifierCase::Snake, help = "Target case style")]
    pub to: IdentifierCase,
    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,
    /// Preserve original file timestamps.
    #[arg(long, help = "Preserve original mtime/atime", action = clap::ArgAction::SetTrue)]
    pub preserve_timestamps: bool,
    /// Preview without writing.
    #[arg(long, help = "Show what would be changed without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Arguments for the `wal-stats` subcommand.
///
/// Computes a snapshot of all `.atomwrite.journal.*.json` sidecars
/// under the workspace, classified by terminal state (`Started`/
/// `Committed`/`Aborted`/malformed) and broken down by directory.
/// Read-only and safe to call from any context.
#[derive(Args, Debug)]
pub struct WalStatsArgs {
    /// Preview without scanning the workspace.
    #[arg(long, help = "Show what would be done without scanning", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

