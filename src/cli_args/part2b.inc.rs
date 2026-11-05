// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by part2.inc.rs, from SearchTarget onward.


/// Where `search` looks for the pattern.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SearchTarget {
    /// Search file contents (default).
    Content,
    /// Match basenames only.
    Files,
    /// Search content and basenames.
    Both,
}

/// Sort criterion for search results.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortBy {
    /// Sort by file path.
    Path,
    /// Sort by modification time.
    Modified,
    /// Sort by creation time.
    Created,
    /// No sorting.
    None,
}

/// Arguments for the replace subcommand.
#[derive(Args, Debug)]
pub struct ReplaceArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    /// Pattern to search for.
    ///
    /// A pattern that starts with a hyphen must come after `--`, so that an
    /// unknown flag is reported as an argument error instead of being taken
    /// as the pattern.
    pub pattern: String,
    /// Replacement text.
    ///
    /// A replacement that starts with a hyphen must come after `--`.
    pub replacement: String,

    /// Paths to rewrite within.
    ///
    /// A verb with side effects requires the subject in the invocation line.
    /// At least one path is mandatory: `replace OLD NEW` with no path is a
    /// usage error (exit 2) and never walks the current directory. Name the
    /// root explicitly (e.g. `src/` or `.`).
    #[arg(required = true, value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// Treat pattern as regex.
    #[arg(long, conflicts_with = "literal", help = "Treat pattern as regex", action = clap::ArgAction::SetTrue)]
    pub regex: bool,

    /// Match whole words only.
    #[arg(short = 'w', long, help = "Match whole words only", action = clap::ArgAction::SetTrue)]
    pub word: bool,

    /// Treat pattern as literal string.
    #[arg(
        short = 'F',
        long,
        conflicts_with = "regex",
        help = "Treat pattern as literal string (escape regex chars)",
        action = clap::ArgAction::SetTrue
    )]
    pub literal: bool,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,


    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,

    /// Show diff preview without writing.
    #[arg(long, help = "Show diff preview without writing", action = clap::ArgAction::SetTrue)]
    pub preview: bool,

    /// Maximum replacements per file.
    #[arg(short = 'n', long, help = "Maximum replacements per file")]
    pub max_replacements: Option<usize>,

    /// Expected checksum for optimistic locking.
    #[arg(
        long,
        help = "Only replace if current checksum matches (optimistic lock)"
    )]
    pub expect_checksum: Option<String>,

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Preserve original casing during replacement (UPPER→UPPER, lower→lower, Title→Title).
    #[arg(
        long,
        help = "Preserve original casing (UPPER→UPPER, lower→lower, Title→Title)",
        action = clap::ArgAction::SetTrue
    )]
    pub preserve_case: bool,

    /// Fuzzy matching when the fixed pattern does not match exactly.
    ///
    /// Ignored when `--regex` is set. Default `auto` runs the fuzzy cascade
    /// after the exact multi-occurrence replacement finds zero hits. `off` is
    /// exact-only.
    #[arg(
        long,
        value_enum,
        default_value_t = FuzzyMode::Auto,
        help = "Fuzzy match cascade for fixed-string replace: auto (default), aggressive, or off (exact match only)"
    )]
    pub fuzzy: FuzzyMode,

    /// Override fuzzy similarity threshold (0.0–1.0) for replace cascade.
    #[arg(
        long,
        help = "Override fuzzy similarity threshold (0.0-1.0) for replace"
    )]
    pub fuzzy_threshold: Option<f64>,

    /// Emit progress NDJSON every N files (0 disables; default 50).
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_PROGRESS_EVERY_FILES,
        help = "Emit progress NDJSON every N files visited (0 = off)"
    )]
    pub progress_every: u64,

    /// Preserve original modification time (mtime) of replaced files.
    /// Default is false: mtime is updated to reflect the change.
    /// Set true for backup workflows, version control snapshots, or
    /// reproducible builds that depend on stable timestamps.
    /// Note: setting true may break build systems that use mtime to
    /// detect source changes (cargo, make, cmake, gradle).
    #[arg(long, help = "Preserve original mtime (default: update mtime to now)", action = clap::ArgAction::SetTrue)]
    pub preserve_timestamps: bool,
}

/// Arguments for the backup subcommand.
#[derive(Args, Debug)]
pub struct BackupArgs {
    /// File paths to back up.
    #[arg(required = true, value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// Directory to store backups (default: same as source).
    #[arg(long, help = "Directory to store backup files", value_hint = ValueHint::DirPath)]
    pub output_dir: Option<PathBuf>,

    /// Maximum number of backups to retain per file.
    #[arg(long, default_value_t = crate::constants::DEFAULT_BACKUP_RETENTION, help = "Number of backup copies to keep")]
    pub retention: u8,

    /// Preview without creating backups.
    #[arg(long, help = "Show what would be done without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Arguments for the rollback subcommand.
#[derive(Args, Debug)]
pub struct RollbackArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    #[arg(value_hint = ValueHint::FilePath)]
    /// File path to restore from backup.
    pub path: PathBuf,

    /// Restore a specific backup by timestamp (`YYYYMMDD_HHMMSS`).
    #[arg(long, help = "Timestamp of the backup to restore")]
    pub timestamp: Option<String>,

    /// Restore the most recent backup.
    #[arg(long, help = "Restore the most recent backup (default)", action = clap::ArgAction::SetTrue)]
    pub latest: bool,

    /// Verify BLAKE3 checksum after restore.
    #[arg(long, help = "Verify checksum after restoring", action = clap::ArgAction::SetTrue)]
    pub verify: bool,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Preview without restoring.
    #[arg(long, help = "Show what would be done without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Arguments for the `get` subcommand.
#[derive(Args, Debug)]
pub struct GetArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    /// Path to the structured config file (TOML or JSON).
    pub path: PathBuf,
    /// Dotted path to the key (e.g. `package.version`).
    pub key_path: String,
}

/// Arguments for the `del` subcommand.
#[derive(Args, Debug)]
pub struct DelArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    #[arg(value_hint = ValueHint::FilePath)]
    /// Path to the structured config file (TOML or JSON).
    pub path: PathBuf,
    /// Dotted path to the key (e.g. `dependencies.serde`).
    pub key_path: String,
    /// Report the planned change and write nothing.
    #[arg(long, help = "Report the planned change and write nothing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,
    /// Preserve original file timestamps.
    #[arg(long, help = "Preserve original mtime/atime", action = clap::ArgAction::SetTrue)]
    pub preserve_timestamps: bool,
    /// Treat missing key as a no-op success instead of an error.
    #[arg(long, help = "Succeed silently if the key is already missing", action = clap::ArgAction::SetTrue)]
    pub force_missing: bool,
}

/// Arguments for the `outline` subcommand.
///
/// Extracts the high-level structure of a source file (functions,
/// classes, structs, enums, traits, modules, top-level consts) as
/// NDJSON. Uses `tree-sitter-language-pack`. Without `--kind`, emits
/// all structural items.
#[derive(Args, Debug)]
pub struct OutlineArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    /// Source file to outline.
    pub path: PathBuf,
    /// Tree-sitter language override.
    #[arg(
        long,
        value_name = "LANG",
        help = "Language override (auto-detected from extension)"
    )]
    pub language: Option<String>,
    /// Filter by item kind (e.g. "function", "class", "struct", "enum",
    /// "trait", "impl", "module", "const", "static", "`type_alias`").
    /// Repeatable.
    #[arg(
        long = "kind",
        value_name = "KIND",
        help = "Filter by kind (repeat for multiple)"
    )]
    pub kinds: Vec<String>,
    /// Show byte offsets and start positions.
    #[arg(long, help = "Include byte offsets and start positions", action = clap::ArgAction::SetTrue)]
    pub positions: bool,
    /// Output shape: `ndjson` (default) or `tsv`, one item per line.
    #[arg(
        long,
        value_enum,
        default_value_t = OutlineFormat::Ndjson,
        help = "Output shape: ndjson or tsv"
    )]
    pub format: OutlineFormat,
}

/// How `outline` prints its items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutlineFormat {
    /// One NDJSON record per item (the default agent contract).
    #[default]
    Ndjson,
    /// One tab-separated line per item, with the path printed once.
    Tsv,
}

/// Arguments for the `wal-heal` subcommand.
///
/// Removes stale `Committed`/`Aborted` journals older than the
/// threshold. Preserves `Started` journals (potential orphans) and
/// malformed journals (manual inspection required). Bounded by a
/// wall-clock budget to keep startup cost predictable.
#[derive(Args, Debug)]
pub struct WalHealArgs {
    /// Directory whose journal sidecars are reaped (required; never the ambient workspace).
    #[arg(value_hint = ValueHint::DirPath)]
    pub path: PathBuf,

    /// Minimum age in seconds for a terminal journal to be reaped.
    /// Defaults to 3600 (1h), the same threshold as the startup auto-heal.
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_WAL_HEAL_THRESHOLD_SECS,
        help = "Minimum age in seconds for a terminal journal to be removed"
    )]
    pub threshold_secs: u64,

    /// Wall-clock budget for the walk (milliseconds). The pass stops
    /// once this budget is exceeded so startup cost is bounded.
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_WAL_HEAL_MAX_DURATION_MS,
        help = "Wall-clock budget in milliseconds for the directory walk"
    )]
    pub max_duration_ms: u64,

    /// Preview without removing any sidecar.
    #[arg(long, help = "Show what would be removed without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}
