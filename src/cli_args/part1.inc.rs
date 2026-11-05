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

    /// Recurse into directories and own the whole named tree.
    ///
    /// Without a narrowing filter the verb removes hidden, ignored, linked,
    /// fifo, socket and device entries, and a link is unlinked without being
    /// followed. `--hidden`, `--no-gitignore` and `--follow-symlinks` are
    /// accepted as no-ops that emit one diagnostic line, and `--backup` is
    /// refused with a directory root because the copy would outlive the tree.
    #[arg(
        short,
        long,
        help = "Recurse into directories and remove the whole named tree",
        action = clap::ArgAction::SetTrue
    )]
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
    /// A one-shot CLI cannot prompt interactively; run again without `--plan`
    /// or `--dry-run` to actually delete. `--confirm` is not an alias of this
    /// flag: it is rejected (fail-closed).
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

    /// Rejected legacy flag: `delete` never asks for confirmation.
    ///
    /// It was once an alias of `--plan`, which agents misread as "confirm and
    /// delete". Use `--plan` to list targets or omit the flag to delete.
    #[arg(
        long,
        help = "Rejected: use --plan to list targets, or omit flags to delete (one-shot; not write --confirm)",
        action = clap::ArgAction::SetTrue
    )]
    pub confirm: bool,

    /// Rejected legacy flag: there is no interactive confirmation to skip.
    ///
    /// Use `--plan` to list targets or omit the flag to delete.
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
    /// One or more files to read.
    ///
    /// Explicit paths are read in argv order. An operand that carries `*`, `?`
    /// or `[` is a pattern expanded by the binary itself against ONE level of
    /// its directory, in byte order (the `LC_ALL=C` shell order), skipping
    /// hidden names. An absolute operand is judged by the part that sits under
    /// `--workspace`, so `'/w/*.rs'` and `/w/a.rs` face the same one-level rule
    /// and the same jail. `--format raw` concatenates every target with no
    /// header, no separator and no added newline, the way `cat` does.
    #[arg(value_hint = ValueHint::FilePath, num_args = 1.., required = true)]
    pub paths: Vec<PathBuf>,

    /// Line range to read (1-based, e.g. "1:50").
    ///
    /// Repeat the flag for several ranges (`--lines 1:3 --lines 10:12`), or
    /// separate them with commas (`--lines 1:3,10:12`); they are emitted in
    /// ascending order, with no line repeated.
    #[arg(
        long,
        action = clap::ArgAction::Append,
        visible_alias = "range",
        help = "Line range to read (1-based, e.g. 1:50; repeatable)"
    )]
    pub lines: Vec<String>,

    /// Read the whole file, ignoring the byte ceiling.
    #[arg(
        long,
        help = "Read the whole file, ignoring the byte ceiling",
        action = clap::ArgAction::SetTrue
    )]
    pub all: bool,

    /// Prefix every raw line with its 1-based number.
    #[arg(
        long,
        help = "Prefix every raw line with its 1-based number",
        action = clap::ArgAction::SetTrue
    )]
    pub number: bool,

    /// Drop comment and blank lines before selecting.
    ///
    /// The selector still counts the original line numbers, so `--lines` keeps
    /// meaning the same lines it means without this flag.
    #[arg(
        long,
        help = "Drop comment and blank lines before selecting",
        action = clap::ArgAction::SetTrue
    )]
    pub no_comments: bool,

    /// Read only the named symbol, resolved through tree-sitter.
    ///
    /// The name is matched against the structural nodes of the file, so the
    /// answer is the whole definition and never a text guess.
    #[arg(
        long,
        value_name = "NAME",
        help = "Read only the named symbol, resolved through tree-sitter"
    )]
    pub symbol: Option<String>,

    /// Expand `--line` to the structural block that contains it.
    #[arg(
        long,
        help = "Expand --line to the enclosing structural block",
        action = clap::ArgAction::SetTrue
    )]
    pub enclosing: bool,

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
    /// `raw` always writes the file bytes to stdout, on a terminal and in a
    /// pipe alike. The default `ndjson` is the agent contract.
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Ndjson,
        help = "Output format (raw: always raw file bytes; default NDJSON for agents)"
    )]
    pub format: OutputFormat,

    /// Redundant with `--format raw`, which always emits bytes.
    ///
    /// Kept for compatibility with pipelines that already pass `--bytes`.
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

    /// Crop window `X,Y,W,H` in source pixels, applied before the resize.
    ///
    /// A picture is delivered as a `data:` URI on stdout; cropping first means
    /// the model pays for the region it asked about and not for the page
    /// around it.
    #[arg(long, value_name = "X,Y,W,H", help = "Crop a picture to X,Y,W,H before resizing")]
    pub crop: Option<String>,

    /// Hard ceiling on visual tokens, overrides the tier ceiling.
    ///
    /// The picture is resized until the 28-pixel patch rule fits the number,
    /// so the cost is chosen by the caller instead of discovered afterwards.
    #[arg(long, value_name = "N", help = "Resize a picture to at most N visual tokens")]
    pub max_tokens: Option<u32>,

    /// Deliver a picture again after it was already delivered in this session.
    ///
    /// The first read of a picture records its BLAKE3 under the agent session,
    /// and a repeat answers with the facts instead of paying for the same
    /// `data:` URI twice. This flag asks for the bytes again.
    #[arg(
        long,
        help = "Deliver a picture again after it was already delivered in this session",
        action = clap::ArgAction::SetTrue
    )]
    pub force: bool,

    /// Pages of a PDF to read, as `N` or `A:B`.
    ///
    /// Repeatable, so several windows arrive in one call. A page that carries
    /// no text is reported with `needs_vision`, because a scanned page is not
    /// an empty page.
    #[arg(
        long,
        value_name = "PAGES",
        action = clap::ArgAction::Append,
        help = "PDF pages to read, as N or A:B (repeatable)"
    )]
    pub pages: Vec<String>,

    /// Render one PDF page and deliver it as a picture.
    ///
    /// A scanned page has no text to extract, so `--pages` reports it as
    /// `needs_vision`. This flag renders the page instead, and the result
    /// arrives through the same `data:` URI a picture uses.
    #[arg(long, value_name = "N", help = "Render PDF page N and deliver it as a picture")]
    pub page: Option<u32>,

    /// Resolution a rendered PDF page comes out at, in dots per inch.
    ///
    /// The default is the smallest one that keeps small print legible. Drop it
    /// when the page would exceed the pixel ceiling, and raise it when the
    /// text is too small to read.
    #[arg(
        long,
        value_name = "DPI",
        help = "Resolution of a rendered PDF page, in dots per inch"
    )]
    pub dpi: Option<f32>,

    /// Read the page as text by recognizing the characters on it.
    ///
    /// A scan has no text layer, so every text verb answers "nothing" and the
    /// caller believes the page is empty. This runs the recognition models in
    /// process and returns what the page actually says. The model files are
    /// fetched once into the cache directory, so the first run needs a network.
    #[arg(
        long,
        help = "Read a picture or a rendered page as text, by recognizing its characters"
    )]
    pub ocr: bool,

    /// Use the high-resolution image tier.
    #[arg(
        long,
        help = "Use the high-resolution image tier (2576 px, 4784 visual tokens)",
        action = clap::ArgAction::SetTrue
    )]
    pub high_res: bool,
}

/// Fuzzy matching behavior for --old/--new edit mode.
///
/// `Off` is exact-only (no cascade). Exact miss
/// yields [`crate::error::AtomwriteError::MatchFailed`] (exit 65, code `MATCH_FAILED`).
/// Prefer `Auto` (default) or `Aggressive` for agent workflows.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum FuzzyMode {
    /// Try exact match first, then fuzzy strategies automatically (default).
    Auto,
    /// Exact match only — no fuzzy cascade.
    Off,
    /// Try all fuzzy strategies including low-confidence block anchor.
    Aggressive,
}

/// Arguments for the `edit-loop` subcommand.
#[derive(Args, Debug)]
pub struct EditLoopArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

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

    /// Write the pairs that match instead of aborting the whole batch.
    /// Default (off) is all-or-nothing, like `edit`: any unmatched pair
    /// aborts with exit 65 and nothing is written. With `--partial` the
    /// matching pairs are written and the command exits 87
    /// (`EDIT_LOOP_PARTIAL`) when at least one pair missed.
    #[arg(
        long,
        help = "Write the matching pairs and exit 87 when some miss (default: all-or-nothing, exit 65)",
        action = clap::ArgAction::SetTrue
    )]
    pub partial: bool,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,


    /// Validate syntax after writing. Pass a language name
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
