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

    /// Legacy alias flag for large-file awareness (A-WRITE-001/002).
    ///
    /// Large overwrite is **default-deny**: existing targets above
    /// `[write].confirm_large_bytes` always require `--ack-overwrite`.
    /// `--confirm` is optional documentation noise and does **not** enable the
    /// guard by itself. **Not** `delete --confirm` (rejected). Independent of
    /// `--require-large-ack` (no clap alias collision).
    #[arg(
        long,
        help = "Optional legacy flag; large overwrite still requires --ack-overwrite (not delete --confirm)",
        action = clap::ArgAction::SetTrue
    )]
    pub confirm: bool,

    /// Independent documentation flag (A-WRITE-002); same semantics as `--confirm`
    /// — does not replace `--ack-overwrite`. Not a clap alias of `--confirm`.
    #[arg(
        long = "require-large-ack",
        help = "Optional large-ack reminder flag; still pass --ack-overwrite for large targets",
        action = clap::ArgAction::SetTrue
    )]
    pub require_large_ack: bool,

    /// Explicit non-interactive acknowledgement that overwriting a large file is intentional.
    #[arg(
        long,
        help = "Acknowledge overwrite of large existing target above XDG [write].confirm_large_bytes (agent one-shot; required)",
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

include!("part2b.inc.rs");
