/// Shared backup flags for mutating subcommands.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct BackupOpts {
    /// Create a backup before writing (default: on, unless `[defaults]` in `.atomwrite.toml` turns it off).
    // Tri-state on purpose: `None` follows the config default, `Some(true)` is
    // `--backup`, and `no_backup` forces it off.
    #[arg(long, action = clap::ArgAction::SetTrue, conflicts_with = "no_backup", help_heading = "Backup")]
    pub backup: Option<bool>,
    /// Disable backup creation
    #[arg(long, action = clap::ArgAction::SetTrue, help_heading = "Backup")]
    pub no_backup: bool,
    /// Keep the backup file after success (default: auto-remove on success)
    #[arg(long, action = clap::ArgAction::SetTrue, help_heading = "Backup")]
    pub keep_backup: bool,
    /// Number of backups to retain (default: `[defaults]` retention in `.atomwrite.toml`, else 5).
    #[arg(long, help_heading = "Backup")]
    pub retention: Option<u8>,
}

/// Refuse the cross-device copy-then-rename fallback.
///
/// A target on another filesystem than its temp file normally falls back to
/// copying the bytes and then renaming, which is no longer a single atomic
/// step. With this flag the write instead fails with
/// `EXDEV_FALLBACK_DISABLED` (exit 91), so a caller that needs the single
/// atomic step can refuse the weaker path.
///
/// Flattened into every verb that writes through [`crate::atomic`], so the flag
/// is spelled the same way everywhere and no verb silently drops it.
#[derive(Debug, Clone, Copy, Default, clap::Args)]
pub struct StrictAtomicArgs {
    /// Refuse the cross-device copy-then-rename fallback, failing with exit 91.
    #[arg(
        long,
        help = "Fail with exit 91 instead of the cross-device copy fallback",
        action = clap::ArgAction::SetTrue
    )]
    pub strict_atomic: bool,
}

/// Acknowledgement flags flattened into every command that changes bytes on disk.
///
/// The global `--confirm-side-effect` is optional for reads and discovery, so
/// the mutation contract has to live on the mutating commands themselves where
/// clap can enforce it. Carries no field of its own.
#[derive(Debug, Clone, Copy, Default, clap::Args)]
pub struct MutateArgs {
    /// Acknowledge the command side effect before it runs.
    #[arg(
        long = "confirm-side-effect",
        help = "Acknowledge this command side effect before it runs",
        action = clap::ArgAction::SetTrue
    )]
    pub confirm_side_effect: bool,

    /// Declare non-interactive execution: never prompt, never block on stdin.
    #[arg(
        long = "non-interactive",
        help = "Never prompt or block on stdin (agent-hosted run)",
        action = clap::ArgAction::SetTrue
    )]
    pub non_interactive: bool,
}

/// Arguments for the count subcommand.
#[derive(Args, Debug)]
pub struct CountArgs {
    /// Paths to count within.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// Group counts by file extension.
    #[arg(long, help = "Group counts by file extension", action = clap::ArgAction::SetTrue)]
    pub by_extension: bool,

    /// Sort results by file size.
    #[arg(long, help = "Sort by file size (top N)", action = clap::ArgAction::SetTrue)]
    pub by_size: bool,

    /// Number of top results to show.
    #[arg(long, default_value_t = crate::constants::DEFAULT_QUERY_TOP, help = "Number of top results")]
    pub top: usize,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,
}

/// Available diff algorithms for file comparison.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DiffAlgorithm {
    /// Myers linear-space diff algorithm.
    Myers,
    /// Patience diff algorithm.
    Patience,
    /// Longest common subsequence algorithm.
    Lcs,
}

/// Arguments for the copy subcommand.
#[derive(Args, Debug)]
pub struct CopyArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    /// Source file path.
    #[arg(value_hint = ValueHint::AnyPath)]
    pub source: PathBuf,
    /// Destination file path.
    #[arg(value_hint = ValueHint::AnyPath)]
    pub target: PathBuf,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Overwrite destination if it exists.
    #[arg(short, long, help = "Overwrite destination if it exists", action = clap::ArgAction::SetTrue)]
    pub force: bool,

    /// Copy directories recursively.
    #[arg(short, long, help = "Copy directories recursively", action = clap::ArgAction::SetTrue)]
    pub recursive: bool,

    /// Preserve timestamps and permissions.
    #[arg(long, help = "Preserve timestamps and permissions", action = clap::ArgAction::SetTrue)]
    pub preserve: bool,

    /// Preview without copying.
    #[arg(long, help = "Show what would be done without copying", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Disable reflink (copy-on-write) optimization.
    #[arg(long, help = "Disable reflink optimization; force full byte copy", action = clap::ArgAction::SetTrue)]
    pub no_reflink: bool,

    /// Preserve extended attributes on copy.
    #[arg(long, help = "Preserve extended attributes (xattr) on copy", action = clap::ArgAction::SetTrue)]
    pub preserve_xattr: bool,
}

/// CLI durability policy.
#[derive(Debug, Clone, Copy, ValueEnum, Default, PartialEq, Eq)]
pub enum DurabilityCli {
    /// Strongest fsync (`F_FULLFSYNC` / `sync_all`) + dir fsync.
    Full,
    /// Fast path: file `sync_data` only.
    Fast,
    /// Auto: full for config files, fast for source.
    #[default]
    Auto,
}

impl From<DurabilityCli> for crate::platform::Durability {
    fn from(v: DurabilityCli) -> Self {
        match v {
            DurabilityCli::Full => crate::platform::Durability::Full,
            DurabilityCli::Fast => crate::platform::Durability::Fast,
            DurabilityCli::Auto => crate::platform::Durability::Auto,
        }
    }
}

/// Arguments for the edit subcommand.
#[derive(Args, Debug)]
pub struct EditArgs {
    /// File path to edit.
    #[arg(value_hint = ValueHint::FilePath)]
    pub path: PathBuf,

    /// Insert stdin content after line N.
    #[arg(
        long,
        conflicts_with_all = ["old", "new", "old_file", "new_file"],
        help = "Insert content from stdin after line N"
    )]
    pub after_line: Option<usize>,

    /// Insert stdin content before line N.
    #[arg(
        long,
        conflicts_with_all = ["old", "new", "old_file", "new_file"],
        help = "Insert content from stdin before line N"
    )]
    pub before_line: Option<usize>,

    /// Replace line range N:M with stdin content.
    #[arg(
        long,
        conflicts_with_all = ["old", "new", "old_file", "new_file"],
        help = "Replace line range N:M with stdin content"
    )]
    pub range: Option<String>,

    /// Delete line range N:M.
    #[arg(long, help = "Delete line range N:M")]
    pub delete_range: Option<String>,

    /// Insert stdin content after first match of text.
    #[arg(
        long,
        allow_hyphen_values = true,
        conflicts_with_all = ["old", "new", "old_file", "new_file"],
        help = "Insert stdin content after first match of text"
    )]
    pub after_match: Option<String>,

    /// Insert stdin content before first match of text.
    #[arg(
        long,
        allow_hyphen_values = true,
        conflicts_with_all = ["old", "new", "old_file", "new_file"],
        help = "Insert stdin content before first match of text"
    )]
    pub before_match: Option<String>,

    /// Two markers delimiting content to replace with stdin.
    #[arg(
        long,
        num_args = 2,
        allow_hyphen_values = true,
        conflicts_with_all = ["old", "new", "old_file", "new_file"],
        help = "Replace content between two markers with stdin"
    )]
    pub between: Option<Vec<String>>,

    /// Exact text to find (repeatable for multiple replacements).
    #[arg(long, allow_hyphen_values = true, action = clap::ArgAction::Append, help = "Exact text to find (repeatable; for content >1KB or with special chars, prefer --old-file)")]
    pub old: Vec<String>,

    /// Replacement text for --old (repeatable, must match --old count).
    #[arg(long, allow_hyphen_values = true, action = clap::ArgAction::Append, help = "Replacement text for --old (repeatable; for content >1KB or with special chars, prefer --new-file)")]
    pub new: Vec<String>,

    /// Path to file containing exact text to find (alternative to --old for large content).
    #[arg(long, conflicts_with = "old", action = clap::ArgAction::Append,
          help = "Read match text from file (repeatable; alternative to --old for large content)")]
    #[arg(value_hint = ValueHint::AnyPath)]
    pub old_file: Vec<PathBuf>,

    /// Path to file containing replacement text (alternative to --new for large content).
    #[arg(long, conflicts_with = "new", action = clap::ArgAction::Append,
          help = "Read replacement text from file (repeatable; alternative to --new for large content)")]
    #[arg(value_hint = ValueHint::AnyPath)]
    pub new_file: Vec<PathBuf>,

    /// Fuzzy matching mode for --old/--new.
    #[arg(
        long,
        value_enum,
        default_value_t = FuzzyMode::Auto,
        help = "Fuzzy match mode: auto (default), aggressive, or off (exact match only, MATCH_FAILED on a miss)"
    )]
    pub fuzzy: FuzzyMode,

    /// Read multiple edit operations as NDJSON from stdin (inherits --fuzzy mode).
    #[arg(
        long,
        conflicts_with_all = ["old", "new", "old_file", "new_file"],
        help = "Read multiple edit operations as NDJSON from stdin (inherits --fuzzy mode)",
        action = clap::ArgAction::SetTrue
    )]
    pub multi: bool,

    /// Expected checksum for optimistic locking.
    #[arg(long, help = "Only edit if current checksum matches (optimistic lock)")]
    pub expect_checksum: Option<String>,

    /// Line ending normalization mode.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::line_endings::LineEnding::Auto,
        help = "Normalize line endings: lf, crlf, cr, auto (preserve original)"
    )]
    pub line_ending: crate::line_endings::LineEnding,

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Preserve original modification time (mtime) of the file.
    /// Default is false: mtime is updated to reflect the edit.
    /// Set true for backup workflows, version control snapshots, or
    /// reproducible builds that depend on stable timestamps.
    /// Note: setting true may break build systems that use mtime to
    /// detect source changes (cargo, make, cmake, gradle).
    #[arg(long, help = "Preserve original mtime (default: update mtime to now)", action = clap::ArgAction::SetTrue)]
    pub preserve_timestamps: bool,

    /// Apply only the `--old`/`--new` pairs that match instead of failing the
    /// whole batch. Default (off) is all-or-nothing: any unmatched pair
    /// aborts with exit 65 and no write. With `--partial`, unmatched pairs are
    /// reported in `pair_results` with `matched: false`; if zero pairs apply,
    /// the command exits 1 (`NO_MATCHES`) without writing.
    #[arg(
        long,
        help = "Apply matching --old/--new pairs and report the rest (default: all-or-nothing)",
        action = clap::ArgAction::SetTrue
    )]
    pub partial: bool,

    /// Override the fuzzy similarity threshold (0.0–1.0).
    #[arg(long, help = "Override fuzzy similarity threshold (0.0-1.0)")]
    pub fuzzy_threshold: Option<f64>,

    /// Replace every occurrence of --old (default: require a unique match).
    #[arg(
        long,
        help = "Replace all occurrences of --old (default: fail if match is not unique)",
        action = clap::ArgAction::SetTrue
    )]
    pub replace_all: bool,

    /// WAL sidecar creation policy. See `write` for the full description;
    /// the same enum applies to edit operations (which by default DO create a
    /// sidecar because they lack native atomicity).
    #[arg(
        long,
        value_enum,
        default_value_t = crate::wal::WalPolicy::Auto,
        help = "WAL sidecar policy: auto (default), always, never"
    )]
    pub wal_policy: crate::wal::WalPolicy,

    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: StrictAtomicArgs,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Accept `STATE_DRIFT` between sequential edits by
    /// the same agent. Use this when chaining multiple `edit` calls
    /// without re-capturing the checksum. Default (off) keeps the
    /// fail-loud `STATE_DRIFT` for true concurrency.
    #[arg(
        long,
        help = "Accept STATE_DRIFT between sequential edits (default: reject). For agent pipelines that chain edits.",
        action = clap::ArgAction::SetTrue
    )]
    pub allow_sequential_drift: bool,
}

/// Arguments for the list subcommand.
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Paths to list.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// Maximum directory depth.
    #[arg(short = 'd', long, help = "Maximum directory depth")]
    pub depth: Option<usize>,

    /// Show size and modification time.
    #[arg(short = 'l', long, help = "Show size and modification time", action = clap::ArgAction::SetTrue)]
    pub long: bool,

    /// Group file counts by extension.
    #[arg(long, help = "Group file counts by extension", action = clap::ArgAction::SetTrue)]
    pub count_by_ext: bool,

    /// Show all files including hidden.
    #[arg(long, help = "Show all files including hidden", action = clap::ArgAction::SetTrue)]
    pub all: bool,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,
}

/// Arguments for the regex subcommand.
#[derive(Args, Debug)]
pub struct RegexArgs {
    /// Example strings for regex generation.
    /// Use `--` before examples that start with a hyphen: `regex --digits -- "-ex1" "--ex2"`
    #[arg(help = "Example strings (use -- before hyphenated examples)")]
    pub examples: Vec<String>,

    /// Read examples from stdin.
    #[arg(long, help = "Read examples from stdin (one per line)", action = clap::ArgAction::SetTrue)]
    pub stdin: bool,

    /// Convert digits to \\d.
    #[arg(short = 'd', long, help = "Convert digits to \\d", action = clap::ArgAction::SetTrue)]
    pub digits: bool,

    /// Convert words to \\w.
    #[arg(short = 'w', long, help = "Convert words to \\w", action = clap::ArgAction::SetTrue)]
    pub words: bool,

    /// Convert whitespace to \\s.
    #[arg(short = 's', long, help = "Convert whitespace to \\s", action = clap::ArgAction::SetTrue)]
    pub spaces: bool,

    /// Detect repetitions.
    #[arg(short = 'r', long, help = "Detect repetitions", action = clap::ArgAction::SetTrue)]
    pub repetitions: bool,

    /// Case-insensitive matching.
    #[arg(short = 'i', long, help = "Case-insensitive matching", action = clap::ArgAction::SetTrue)]
    pub case_insensitive: bool,

    /// Remove anchors (^ and $).
    #[arg(long, help = "Remove anchors (^ and $)", action = clap::ArgAction::SetTrue)]
    pub no_anchors: bool,
}

include!("part0b.inc.rs");
