/// Arguments for shell completion script generation.
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Target shell for completion scripts.
    #[arg(value_enum)]
    pub shell: ShellType,

    /// Install completion script to XDG data directory.
    #[arg(
        long,
        help = "Install completion script to XDG data directory (Bash: ~/.local/share/bash-completion/completions/atomwrite)",
        action = clap::ArgAction::SetTrue
    )]
    pub install: bool,
}

/// Arguments for the verify subcommand (alias for hash --verify).
#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Path to the file to verify.
    #[arg(value_hint = ValueHint::FilePath)]
    pub path: PathBuf,

    /// Expected BLAKE3 checksum.
    pub checksum: String,
}

/// Arguments for the delete subcommand.
#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// File paths to delete.
    #[arg(required = true, value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Recurse into directories.
    #[arg(short, long, help = "Recurse into directories", action = clap::ArgAction::SetTrue)]
    pub recursive: bool,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,

    /// Only delete files older than this duration (e.g. 30d, 24h, 1w, 60s).
    #[arg(
        long,
        help = "Only delete files older than duration (e.g. 30d, 24h, 1w)"
    )]
    pub older_than: Option<String>,

    /// Plan-only: list files that would be deleted without mutating.
    ///
    /// One-shot CLIs cannot prompt interactively; run again without `--plan`/
    /// `--dry-run` to actually delete (B-005 / A-004).
    /// `--confirm` is **not** an alias — it is rejected (fail-closed).
    #[arg(
        long,
        conflicts_with = "dry_run",
        help = "Plan only: list targets without deleting (no interactive prompt; one-shot)",
        action = clap::ArgAction::SetTrue
    )]
    pub plan: bool,

    /// Preview without deleting.
    #[arg(long, help = "Show what would be done without deleting", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Legacy interactive flag — **rejected** (B-005/B-014).
    ///
    /// Was previously an alias of `--plan`, which agents misread as “confirm and
    /// delete”. Use `--plan` to list or omit flags to delete.
    #[arg(
        long,
        help = "Rejected: use --plan to list targets, or omit flags to delete (one-shot; not write --confirm)",
        action = clap::ArgAction::SetTrue
    )]
    pub confirm: bool,

    /// Legacy interactive “yes” — **rejected** (B-015).
    ///
    /// There is no interactive confirmation to skip. Use `--plan` to list or
    /// omit flags to delete.
    #[arg(
        short = 'y',
        long,
        help = "Rejected: no interactive confirm in one-shot; use --plan to list, omit flags to delete",
        action = clap::ArgAction::SetTrue
    )]
    pub yes: bool,
}

/// Arguments for the read subcommand.
#[derive(Args, Debug, Clone)]
pub struct ReadArgs {
    /// File path to read.
    #[arg(value_hint = ValueHint::FilePath)]
    pub path: PathBuf,

    /// Line range to read (1-based, e.g. "1:50").
    #[arg(long, help = "Line range to read (1-based, e.g. 1:50)")]
    pub lines: Option<String>,

    /// Single line number to read.
    #[arg(long, help = "Single line number with optional context")]
    pub line: Option<usize>,

    /// Lines of context around --line.
    #[arg(
        short = 'C',
        long,
        default_value_t = 0,
        help = "Lines of context around --line"
    )]
    pub context: usize,

    /// Read first N lines.
    #[arg(long, help = "Read first N lines")]
    pub head: Option<usize>,

    /// Read last N lines.
    #[arg(long, help = "Read last N lines")]
    pub tail: Option<usize>,

    /// Return only metadata without content.
    #[arg(long, help = "Return only metadata (no content)", action = clap::ArgAction::SetTrue)]
    pub stat: bool,

    /// Output format selection.
    ///
    /// `raw`: always emit file bytes on stdout (TTY and non-TTY) — B-006.
    /// Agent NDJSON contract uses the default format (not `raw`).
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Ndjson,
        help = "Output format (raw: always raw file bytes; default NDJSON for agents)"
    )]
    pub format: OutputFormat,

    /// Redundant with `--format raw` (B-006): raw always emits bytes.
    ///
    /// Kept for compatibility with pipelines that passed `--bytes` under the
    /// old dual-mode contract.
    #[arg(
        long,
        help = "With --format raw: emit raw bytes (implied by --format raw; kept for compatibility)",
        action = clap::ArgAction::SetTrue
    )]
    pub bytes: bool,

    /// Expected BLAKE3 hash for verification.
    #[arg(long, help = "Verify file checksum against expected BLAKE3 hash")]
    pub verify_checksum: Option<String>,

    /// Filter lines matching this regex (substring of file content).
    #[arg(
        long,
        allow_hyphen_values = true,
        help = "Filter returned lines to those matching this regex"
    )]
    pub grep: Option<String>,
}

/// Fuzzy matching behavior for --old/--new edit mode.
///
/// v0.1.30: fuzzy is mandatory. `Off` is rejected at runtime with exit 65.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum FuzzyMode {
    /// Try exact match first, then fuzzy strategies automatically (default).
    Auto,
    /// Removed in v0.1.30 — rejected with migration note (kept for clap parse of legacy scripts).
    Off,
    /// Try all fuzzy strategies including low-confidence block anchor.
    Aggressive,
}

/// Arguments for the `edit-loop` subcommand (ADR-0039).
#[derive(Args, Debug)]
pub struct EditLoopArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    /// File path to apply all pairs against.
    pub path: PathBuf,

    /// Accept `STATE_DRIFT` between iterations (default: reject). Useful
    /// when chaining edits; for `edit-loop` this is informational since
    /// the whole batch is computed in memory and a single atomic write
    /// is performed at the end (no per-pair drift to validate).
    #[arg(
        long,
        help = "Accept STATE_DRIFT (informational for edit-loop; default: reject)",
        action = clap::ArgAction::SetTrue
    )]
    pub allow_sequential_drift: bool,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Validate syntax after writing (G72). Pass a language name
    /// (`rust`, `python`, `js`, etc.). When the file is invalid, the
    /// write is aborted with `SyntaxError` (exit 88).
    #[arg(
        long,
        value_name = "LANG",
        help = "Validate syntax of the written file via tree-sitter (e.g. rust, python, js)"
    )]
    pub syntax_check: Option<String>,

    /// Normalize line endings of the written file.
    #[arg(
        long,
        value_enum,
        default_value_t = crate::line_endings::LineEnding::Auto,
        help = "Normalize line endings: lf, crlf, cr, auto (preserve original)"
    )]
    pub line_ending: crate::line_endings::LineEnding,
}


include!("part1b.inc.rs");
