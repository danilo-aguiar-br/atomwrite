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
        help = "Lines of context in unified diff"
    )]
    pub context: usize,

    /// Diff algorithm to use.
    #[arg(long, value_enum, default_value_t = DiffAlgorithm::Patience, help = "Diff algorithm")]
    pub algorithm: DiffAlgorithm,
}

/// Arguments for the move subcommand.
///
/// Directories are renamed on the same filesystem without `-r`.
/// For cross-device directory trees use `copy --recursive` then `delete`.
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

    /// Preserve hardlink count on move.
    #[arg(long, help = "Preserve hardlink count on move", action = clap::ArgAction::SetTrue)]
    pub preserve_hardlinks: bool,
}

/// Output format for the read subcommand.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Structured NDJSON output.
    Ndjson,
    /// Raw file bytes on stdout with no NDJSON envelope, on a terminal and in a pipe alike.
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

    /// Run post-write tree-sitter syntax check. Aborts the
    /// write with exit code 88 if the new content has parse errors.
    /// Languages covered: rust, python, javascript, typescript, tsx,
    /// go, c, cpp, java, ruby, php, bash, html, css, json, yaml,
    /// toml, markdown, lua, scala, swift, kotlin, sql. Files with
    /// no parser available fall back to a bracket-balance heuristic.
    #[arg(
        long,
        help = "Run a tree-sitter syntax check and abort on parse errors (exit 88)",
        action = clap::ArgAction::SetTrue
    )]
    pub syntax_check: bool,

    /// Durability trade-off for fsync: full|fast|auto.
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

    /// Allow writes that shrink the target past `[write].shrink_block_percent` (default 50).
    #[arg(
        long,
        help = "Allow writes that shrink the target past [write].shrink_block_percent of the config file (default 50), also without --expect-checksum",
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

    /// Allow zero-byte stdin (default: reject empty stdin as invalid input).
    /// Use this flag to confirm the empty write is intentional
    /// (e.g. truncating a file to zero bytes).
    #[arg(
        long,
        help = "Allow zero-byte stdin (default: reject empty stdin)",
        action = clap::ArgAction::SetTrue
    )]
    pub allow_empty_stdin: bool,

    /// Skip the `--expect-checksum` verification when the resolved
    /// stdin payload is empty. Use this
    /// when the combination `--append --expect-checksum <HASH> < /dev/null`
    /// is intentional (no-op append, checksum match preserved).
    #[arg(
        long,
        help = "Allow --expect-checksum to be skipped when stdin is empty",
        action = clap::ArgAction::SetTrue
    )]
    pub no_checksum_when_empty: bool,

    /// WAL sidecar creation policy.
    ///
    /// `auto` (default) skips the sidecar for trivial writes (small file in
    /// a git-tracked directory, plain write, set/del). `always` forces the
    /// sidecar even for trivial cases. `never` suppresses sidecar creation
    /// entirely.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::wal::WalPolicy::Auto,
        help = "WAL sidecar policy: auto (default), always, never"
    )]
    pub wal_policy: crate::wal::WalPolicy,

    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    /// Preserve original mtime+atime of the target file (default: update to now).
    /// Useful for backup/snapshot workflows that depend on stable mtimes.
    /// Parity with edit/replace/set/del/case (which expose --preserve-timestamps).
    #[arg(
        long,
        help = "Preserve original mtime/atime of the target file (default: update to now)",
        action = clap::ArgAction::SetTrue
    )]
    pub preserve_timestamps: bool,

    /// Require `--backup` to be set. Aborts the write with
    /// exit 65 if the target file exists and `--backup` is not provided.
    /// Useful for scripted agent runs where backups are non-negotiable.
    #[arg(
        long,
        help = "Require --backup: abort with exit 65 when the target exists and --backup is missing",
        action = clap::ArgAction::SetTrue
    )]
    pub require_backup: bool,

    /// Optional legacy flag for large-file awareness.
    ///
    /// Overwriting a large file is denied by default: existing targets above
    /// `[write].confirm_large_bytes` always require `--ack-overwrite`.
    /// `--confirm` is optional and does not enable the guard by itself. It is
    /// not `delete --confirm` (rejected) and is independent of
    /// `--require-large-ack`.
    #[arg(
        long,
        help = "Optional legacy flag; large overwrite still requires --ack-overwrite (not delete --confirm)",
        action = clap::ArgAction::SetTrue
    )]
    pub confirm: bool,

    /// Optional large-ack reminder flag with the same semantics as `--confirm`:
    /// it does not replace `--ack-overwrite`.
    #[arg(
        long = "require-large-ack",
        help = "Optional large-ack reminder flag; still pass --ack-overwrite for large targets",
        action = clap::ArgAction::SetTrue
    )]
    pub require_large_ack: bool,

    /// Explicit non-interactive acknowledgement that overwriting a large file is intentional.
    #[arg(
        long,
        help = "Acknowledge overwriting an existing target above [write].confirm_large_bytes (100 KiB by default); required for large targets",
        action = clap::ArgAction::SetTrue
    )]
    pub ack_overwrite: bool,

    /// Auto-rotation. When `--backup` is active, ensures a
    /// rotation backup is created if the target file was modified within
    /// the last 24 hours (heuristic: recent files need backups).
    #[arg(
        long,
        help = "Force a rotation backup for recently modified files (age from [write].auto_rotate_max_age_secs of the config file, default 24 h)",
        action = clap::ArgAction::SetTrue
    )]
    pub auto_rotate: bool,

    /// Size delta threshold (percent) for the risk warning.
    /// Default 255 (off). Enable with e.g. 50.
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
