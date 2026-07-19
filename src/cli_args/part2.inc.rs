/// Supported shell types for completion generation.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShellType {
    /// Bash shell.
    Bash,
    /// Zsh shell.
    Zsh,
    /// Fish shell.
    Fish,
    /// `PowerShell`.
    #[value(name = "powershell")]
    PowerShell,
    /// Elvish shell.
    Elvish,
}

/// Arguments for the hash subcommand.
#[derive(Args, Debug)]
pub struct HashArgs {
    /// File paths to hash.
    #[arg(required_unless_present = "stdin", value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// Expected BLAKE3 hash for verification.
    #[arg(long, help = "Verify file checksum against expected BLAKE3 hash")]
    pub verify: Option<String>,

    /// Hash content from stdin.
    #[arg(long, help = "Hash content from stdin instead of files", action = clap::ArgAction::SetTrue)]
    pub stdin: bool,

    /// Recurse into directories.
    #[arg(short, long, help = "Recurse into directories", action = clap::ArgAction::SetTrue)]
    pub recursive: bool,

    /// Glob patterns to exclude (e.g. `*.bak.*`). Appendable.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob when hashing")]
    pub exclude: Vec<String>,
}

/// Arguments for the diff subcommand.
#[derive(Args, Debug)]
pub struct DiffArgs {
    /// First file to compare.
    #[arg(value_hint = ValueHint::FilePath)]
    pub file_a: PathBuf,
    /// Second file to compare.
    #[arg(value_hint = ValueHint::FilePath)]
    pub file_b: PathBuf,

    /// Output unified diff format.
    #[arg(long, help = "Output unified diff format", action = clap::ArgAction::SetTrue)]
    pub unified: bool,

    /// Show only summary statistics.
    #[arg(long, help = "Only show summary statistics", action = clap::ArgAction::SetTrue)]
    pub stat: bool,

    /// Lines of context in unified diff.
    #[arg(
        short = 'C',
        long,
        default_value_t = crate::constants::DEFAULT_DIFF_CONTEXT_LINES,
        help = "Lines of context in unified diff (default constants::DEFAULT_DIFF_CONTEXT_LINES)"
    )]
    pub context: usize,

    /// Diff algorithm to use.
    #[arg(long, value_enum, default_value_t = DiffAlgorithm::Patience, help = "Diff algorithm")]
    pub algorithm: DiffAlgorithm,
}

/// Arguments for the move subcommand.
///
/// G-021: directories rename on the same filesystem without `-r`.
/// Cross-device directory trees: use `copy --recursive` then `delete`.
#[derive(Args, Debug)]
pub struct MoveArgs {
    /// Source file or directory path.
    #[arg(value_hint = ValueHint::AnyPath)]
    pub source: PathBuf,
    /// Destination file or directory path.
    #[arg(value_hint = ValueHint::AnyPath)]
    pub target: PathBuf,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Overwrite destination if it exists.
    #[arg(short, long, help = "Overwrite destination if it exists", action = clap::ArgAction::SetTrue)]
    pub force: bool,

    /// Preview without moving.
    #[arg(long, help = "Show what would be done without moving", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Preserve hardlink count on move (G55).
    #[arg(long, help = "Preserve hardlink count on move", action = clap::ArgAction::SetTrue)]
    pub preserve_hardlinks: bool,
}

/// Output format for the read subcommand.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Structured NDJSON output.
    Ndjson,
    /// Dual-mode raw: TTY → file bytes; non-TTY → NDJSON envelope (agent contract G-013).
    /// Use `--bytes` to force raw bytes on non-TTY pipes (A-003).
    Raw,
}

/// Arguments for the write subcommand.
#[derive(Args, Debug)]
pub struct WriteArgs {
    /// Target file path.
    #[arg(value_hint = ValueHint::FilePath)]
    pub target: PathBuf,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Maximum input size in bytes.
    #[arg(long, help = "Maximum input size in bytes")]
    pub max_size: Option<u64>,

    /// Append content to end of file.
    #[arg(
        long,
        conflicts_with = "prepend",
        help = "Append content to end of existing file",
        action = clap::ArgAction::SetTrue
    )]
    pub append: bool,

    /// Prepend content to beginning of file.
    #[arg(
        long,
        conflicts_with = "append",
        help = "Prepend content to beginning of existing file",
        action = clap::ArgAction::SetTrue
    )]
    pub prepend: bool,

    /// Run post-write tree-sitter syntax check (G72). Aborts the
    /// write with exit code 88 if the new content has parse errors.
    /// Languages covered: rust, python, javascript, typescript, tsx,
    /// go, c, cpp, java, ruby, php, bash, html, css, json, yaml,
    /// toml, markdown, lua, scala, swift, kotlin, sql. Files with
    /// no parser available fall back to a bracket-balance heuristic.
    #[arg(
        long,
        help = "Run tree-sitter syntax check (G72). Aborts on parse errors (exit 88).",
        action = clap::ArgAction::SetTrue
    )]
    pub syntax_check: bool,

    /// Durability trade-off for fsync (v0.1.29 P2-1): full|fast|auto.
    #[arg(
        long,
        value_enum,
        default_value_t = DurabilityCli::Auto,
        help = "Durability: full (strongest fsync), fast (sync_data only), auto (config=full else fast)"
    )]
    pub durability: DurabilityCli,

    /// Expected checksum for optimistic locking.
    #[arg(
        long,
        help = "Only write if current checksum matches (optimistic lock)"
    )]
    pub expect_checksum: Option<String>,

    /// Allow writes that shrink past XDG `[write].shrink_block_percent` (default 50).
    #[arg(
        long,
        help = "Allow writes that shrink past XDG [write].shrink_block_percent (default constants::SHRINK_BLOCK_PERCENT=50; also applies without CAS)",
        action = clap::ArgAction::SetTrue
    )]
    pub allow_shrink: bool,

    /// Line ending normalization mode.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::line_endings::LineEnding::Auto,
        help = "Normalize line endings: lf, crlf, cr, auto (preserve original)"
    )]
    pub line_ending: crate::line_endings::LineEnding,

    /// Allow zero-byte stdin (default: reject empty stdin as invalid input,
    /// G120 L1 guard). Use this flag to confirm the empty write is intentional
    /// (e.g. truncating a file to zero bytes).
    #[arg(
        long,
        help = "Allow zero-byte stdin (G120 L1 guard; default: reject empty stdin)",
        action = clap::ArgAction::SetTrue
    )]
    pub allow_empty_stdin: bool,

    /// Skip the `--expect-checksum` verification when the resolved
    /// stdin payload is empty (G120 L3 cross-validation). Use this
    /// when the combination `--append --expect-checksum <HASH> < /dev/null`
    /// is intentional (no-op append, checksum match preserved).
    #[arg(
        long,
        help = "Allow --expect-checksum to be skipped when stdin is empty (G120 L3)",
        action = clap::ArgAction::SetTrue
    )]
    pub no_checksum_when_empty: bool,

    /// WAL sidecar creation policy (G119 L1 prevention).
    ///
    /// `auto` (default) skips the sidecar for trivial writes (small file in
    /// a git-tracked directory, plain write, set/del). `always` forces the
    /// sidecar even for trivial cases (legacy semantics, equivalent to
    /// `--strict-atomic`). `never` suppresses sidecar creation entirely,
    /// even when `--strict-atomic` is set or WAL policy Always.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::wal::WalPolicy::Auto,
        help = "WAL sidecar policy: auto (default), always, never (G119 L1)"
    )]
    pub wal_policy: crate::wal::WalPolicy,

    /// Preserve original mtime+atime of the target file (default: update to now).
    /// Useful for backup/snapshot workflows that depend on stable mtimes.
    /// Parity with edit/replace/set/del/case (which expose --preserve-timestamps).
    #[arg(
        long,
        help = "Preserve original mtime/atime of the target file (default: update to now)",
        action = clap::ArgAction::SetTrue
    )]
    pub preserve_timestamps: bool,

    /// GAP-2026-011 L2: Require `--backup` to be set. Aborts the write with
    /// exit 65 if the target file exists and `--backup` is not provided.
    /// Useful for scripted agent runs where backups are non-negotiable.
    #[arg(
        long,
        help = "Require --backup; abort if missing and target file exists (defense-in-depth L2)",
        action = clap::ArgAction::SetTrue
    )]
    pub require_backup: bool,

    /// GAP-2026-011 L3 / G-001: Non-interactive large-file overwrite guard.
    ///
    /// **Not** the same as `delete --confirm` (rejected). This only gates large
    /// existing targets; pass `--ack-overwrite` in the same argv (B-014).
    #[arg(
        long,
        visible_alias = "require-large-ack",
        help = "Large-file overwrite guard: require --ack-overwrite when target exceeds XDG [write].confirm_large_bytes (not delete --confirm)",
        action = clap::ArgAction::SetTrue
    )]
    pub confirm: bool,

    /// Explicit non-interactive acknowledgement that overwriting a large file is intentional.
    #[arg(
        long,
        help = "Acknowledge overwrite of large existing target when --confirm is set (agent one-shot)",
        action = clap::ArgAction::SetTrue
    )]
    pub ack_overwrite: bool,

    /// GAP-2026-011 L5: Auto-rotation. When `--backup` is active, ensures a
    /// rotation backup is created if the target file was modified within
    /// the last 24 hours (heuristic: recent files need backups).
    #[arg(
        long,
        help = "Force auto-rotation backup for recently-modified files (age from XDG [write].auto_rotate_max_age_secs, default 24h) (defense-in-depth L5)",
        action = clap::ArgAction::SetTrue
    )]
    pub auto_rotate: bool,

    /// GAP-2026-011 / G-039: Size delta threshold (percent) for L1 risk warning.
    /// Default [`crate::constants::DEFAULT_RISK_THRESHOLD_OFF`] (255 = off).
    /// Enable with e.g. [`crate::constants::DEFAULT_RISK_THRESHOLD_ON`] (50).
    #[arg(
        long,
        value_name = "PERCENT",
        default_value_t = crate::constants::DEFAULT_RISK_THRESHOLD_OFF,
        help = "Size delta threshold (percent) for risk warning; default off (255); set e.g. 50 to enable"
    )]
    pub risk_threshold: u8,

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Where `search` looks for the pattern (v0.1.30).
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
    /// Pattern to search for.
    #[arg(allow_hyphen_values = true)]
    pub pattern: String,
    /// Replacement text.
    #[arg(allow_hyphen_values = true)]
    pub replacement: String,

    /// Paths to search within.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
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

    /// Fuzzy matching when the fixed pattern does not match exactly (v0.1.29).
    ///
    /// Ignored when `--regex` is set. Default `auto` runs the 9-strategy cascade
    /// after exact multi-occurrence replacement finds zero hits.
    #[arg(
        long,
        value_enum,
        default_value_t = FuzzyMode::Auto,
        help = "Fuzzy match cascade for fixed-string replace (auto|aggressive; off rejected since v0.1.30)"
    )]
    pub fuzzy: FuzzyMode,

    /// Override fuzzy similarity threshold (0.0–1.0) for replace cascade.
    #[arg(
        long,
        help = "Override fuzzy similarity threshold (0.0-1.0) for replace"
    )]
    pub fuzzy_threshold: Option<f64>,

    /// Emit progress NDJSON every N files (0 disables; default 50) (v0.1.29 P1-3).
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_PROGRESS_EVERY_FILES,
        help = "Emit progress NDJSON every N files visited (0=off; default constants::DEFAULT_PROGRESS_EVERY_FILES)"
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
    #[arg(long, default_value_t = crate::constants::DEFAULT_BACKUP_RETENTION, help = "Number of backup copies to keep (default constants::DEFAULT_BACKUP_RETENTION)")]
    pub retention: u8,

    /// Preview without creating backups.
    #[arg(long, help = "Show what would be done without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Arguments for the rollback subcommand.
#[derive(Args, Debug)]
pub struct RollbackArgs {
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

/// Arguments for the `get` subcommand (v14 Tier 3).
#[derive(Args, Debug)]
pub struct GetArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    /// Path to the structured config file (TOML or JSON).
    pub path: PathBuf,
    /// Dotted path to the key (e.g. `package.version`).
    pub key_path: String,
}

/// Arguments for the `del` subcommand (v14 Tier 3).
#[derive(Args, Debug)]
pub struct DelArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    /// Path to the structured config file (TOML or JSON).
    pub path: PathBuf,
    /// Dotted path to the key (e.g. `dependencies.serde`).
    pub key_path: String,
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

/// Arguments for the `outline` subcommand (v14 Tier 3, v0.1.12).
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
}

/// Arguments for the `wal-heal` subcommand (G119 L3 auto-heal).
///
/// Removes stale `Committed`/`Aborted` journals older than the
/// threshold. Preserves `Started` journals (potential orphans) and
/// malformed journals (manual inspection required). Bounded by a
/// wall-clock budget to keep startup cost predictable.
#[derive(Args, Debug)]
pub struct WalHealArgs {
    /// Minimum age in seconds for a terminal journal to be reaped.
    /// Defaults to 3600 (1h) to match the v0.1.17 auto-heal default.
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_WAL_HEAL_THRESHOLD_SECS,
        help = "Minimum age (seconds) for removal (default constants::DEFAULT_WAL_HEAL_THRESHOLD_SECS)"
    )]
    pub threshold_secs: u64,

    /// Wall-clock budget for the walk (milliseconds). The pass stops
    /// once this budget is exceeded so startup cost is bounded.
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_WAL_HEAL_MAX_DURATION_MS,
        help = "Wall-clock budget (ms) for the walk (default constants::DEFAULT_WAL_HEAL_MAX_DURATION_MS)"
    )]
    pub max_duration_ms: u64,

    /// Preview without removing any sidecar.
    #[arg(long, help = "Show what would be removed without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}
