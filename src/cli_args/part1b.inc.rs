// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by part1.inc.rs, after EditLoopArgs.

/// How `search` writes its answers on stdout.
///
/// Deliberately separate from [`crate::cli::OutputFormat`], which belongs to
/// `read`: adding a `Text` value there would make `read --format text` fall
/// through to the NDJSON branch, because `read` only tests for `Raw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum SearchFormat {
    /// One NDJSON record per event (the agent contract, and the default).
    #[default]
    Ndjson,
    /// Plain text, one line per match, shaped like `rg -n`.
    Text,
}

/// Arguments for the search subcommand.
#[derive(Args, Debug)]
#[derive(Clone)]
pub struct SearchArgs {
    /// Search pattern (regex by default).
    ///
    /// A pattern that starts with a hyphen must come after `--`, so that an
    /// unknown flag is reported as an argument error instead of being taken
    /// as the pattern. Absent when the caller supplied every pattern through
    /// `-e/--regex`; a run with neither is a clap usage error, exit 2, which
    /// is what a malformed invocation has always reported here.
    #[arg(required_unless_present = "regex", value_name = "PATTERN")]
    pub pattern: Option<String>,

    /// Paths to search within.
    ///
    /// When omitted, `search` scans the workspace root (never the process
    /// current directory) and reports `target_source = "workspace"`. A root
    /// named here is resolved against `--workspace` and reported as `"argv"`.
    #[arg(value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// Add another regex pattern.
    ///
    /// Repeatable like `rg -e`: every value accumulates, and the run matches
    /// the union of the positional pattern and each one supplied here. The
    /// flag always takes a value, so a repeating `-e A -e B FILE` keeps FILE
    /// as a positional path instead of consuming it as a third pattern.
    #[arg(
        short = 'e',
        long,
        value_name = "PATTERN",
        num_args = 1,
        action = clap::ArgAction::Append,
        conflicts_with = "fixed",
        help = "Add another regex pattern (repeatable, like rg -e)"
    )]
    pub regex: Vec<String>,

    /// Accepted for `rg -n` compatibility; the output already numbers lines.
    ///
    /// `--format text` prefixes every line with its number, so the flag never
    /// changes the answer. It exists so an `rg`-shaped caller does not fail on
    /// an unknown argument.
    #[arg(
        short = 'n',
        help = "Accepted for rg compatibility; line numbers already appear in --format text",
        action = clap::ArgAction::SetTrue
    )]
    pub line_number: bool,

    /// Treat pattern as fixed string.
    #[arg(
        short = 'F',
        long,
        conflicts_with = "regex",
        help = "Treat pattern as fixed string",
        action = clap::ArgAction::SetTrue
    )]
    pub fixed: bool,

    /// Match whole words only.
    #[arg(short = 'w', long, help = "Match whole words only", action = clap::ArgAction::SetTrue)]
    pub word: bool,

    /// Case-insensitive search.
    #[arg(short = 'i', long, help = "Case-insensitive search", action = clap::ArgAction::SetTrue)]
    pub case_insensitive: bool,

    /// Smart case: insensitive when pattern is lowercase.
    #[arg(
        short = 'S',
        long,
        help = "Smart case: insensitive if pattern is lowercase",
        action = clap::ArgAction::SetTrue
    )]
    pub smart_case: bool,

    /// Lines of context around matches.
    #[arg(
        short = 'C',
        long,
        default_value_t = 0,
        help = "Lines of context around matches"
    )]
    pub context: usize,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,

    /// Show only match count per file.
    #[arg(short = 'c', long, help = "Only show match count per file", action = clap::ArgAction::SetTrue)]
    pub count: bool,

    /// Show only filenames with matches.
    #[arg(short = 'l', long, help = "Only show filenames with matches", action = clap::ArgAction::SetTrue)]
    pub files: bool,

    /// Maximum matches per file.
    #[arg(short = 'm', long, help = "Maximum matches per file")]
    pub max_count: Option<u64>,

    /// Enable multi-line matching.
    #[arg(short = 'U', long, help = "Enable multi-line matching", action = clap::ArgAction::SetTrue)]
    pub multiline: bool,

    /// Show non-matching lines.
    #[arg(long, help = "Show lines that do NOT match", action = clap::ArgAction::SetTrue)]
    pub invert: bool,

    /// Sort results by criterion.
    #[arg(long, value_enum, help = "Sort results by criterion")]
    pub sort: Option<SortBy>,

    /// Include FIFO / named pipe files in the search.
    ///
    /// By default `search` skips FIFOs, because opening one blocks until the
    /// other end connects, which can hang automated runs and containers that
    /// have FIFOs in /tmp or /var. Pass `--include-fifo` to open them anyway
    /// (this may hang).
    #[arg(
        long,
        help = "Include FIFO / named pipe files (default: skip to avoid hangs)",
        action = clap::ArgAction::SetTrue
    )]
    pub include_fifo: bool,

    /// Maximum file size in bytes for `search`.
    ///
    /// Larger files are skipped; the run counts them in `files_skipped` on the
    /// `summary` event instead of emitting one event per file. The default is
    /// 10 MiB. Useful for skipping `node_modules`, `target/`, log archives,
    /// and other large generated files.
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_SEARCH_MAX_FILESIZE_BYTES,
        help = "Skip files larger than N bytes (10 MiB by default)"
    )]
    pub max_filesize: u64,

    /// Maximum line length in columns for `search` matches.
    ///
    /// Longer lines are truncated with a `...[truncated]` marker. The default
    /// is 500 columns. Useful for minified bundle.js, styles.min.css and other
    /// single-line giant files that explode context windows.
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_SEARCH_MAX_COLUMNS,
        help = "Truncate matches longer than N columns (500 by default)"
    )]
    pub max_columns: usize,

    /// Search binary files. Default: skip files that look binary
    /// (`NUL` bytes or binary content detection), matching ripgrep-style safety for agents.
    #[arg(
        long,
        help = "Search binary files (default: skip binary / NUL content)",
        action = clap::ArgAction::SetTrue
    )]
    pub binary: bool,

    /// Suppress per-file `begin` and `end` NDJSON events for files with
    /// zero matches. Default: emit `begin`/`end` for every file visited.
    #[arg(
        long,
        help = "Suppress begin/end events for files with no matches (cleaner output for empty searches)",
        action = clap::ArgAction::SetTrue
    )]
    pub no_begin_end: bool,

    /// Use PCRE2 regex engine for lookahead/lookbehind support.
    /// Requires the `pcre2` feature flag at build time.
    #[arg(
        short = 'P',
        long,
        help = "Use PCRE2 regex engine (requires pcre2 feature)",
        action = clap::ArgAction::SetTrue
    )]
    pub pcre2: bool,

    /// Search target: content (default), files (basename), or both.
    #[arg(
        long,
        value_enum,
        default_value_t = SearchTarget::Content,
        help = "Search target: content, files (basename), or both"
    )]
    pub target: SearchTarget,

    /// Skip first N matches when paginating results.
    #[arg(long, default_value_t = 0, help = "Skip first N matches (pagination offset)")]
    pub offset: u64,

    /// Limit number of match events emitted.
    #[arg(long, help = "Limit number of match events emitted")]
    pub limit: Option<u64>,

    /// Output format: `ndjson` (default) or `text`.
    ///
    /// `text` prints one line per match as `LINE:COL:TEXT`, or
    /// `PATH:LINE:COL:TEXT` when more than one file can match, with `PATH`
    /// relative to `--workspace` and `/` as the separator on every OS. `COL`
    /// is the 1-based byte column of the first match on the line. `--context N`
    /// prints context lines with `-` instead of `:` and separates groups with
    /// `--`, the way `rg` does. Nothing else reaches stdout: no `begin`, `end`,
    /// `summary` or colour. `--count` prints `N`, or `PATH:N` for several
    /// files, and `--files` prints `PATH`, matching `rg -c` and `rg -l`.
    /// `--multiline`, `--target files` and `--target both` are rejected,
    /// because a match there is not a single line of a single file.
    /// `--max-columns` does not apply: `rg` returns the whole line, and a
    /// truncated line could not be compared against it byte for byte.
    // No `default_value_t`: `None` is what tells the dispatch that the caller
    // said nothing, so the `[search] format` config may answer. A parsed
    // `Ndjson` here could be either the flag or the default, and the flag must
    // win over the config.
    #[arg(long, value_enum, help = "Output format: ndjson (default) or text")]
    pub format: Option<SearchFormat>,
}

/// Arguments for the extract subcommand.
#[derive(Args, Debug)]
pub struct ExtractArgs {
    /// Field names or indices to extract (positional).
    pub fields: Vec<String>,

    /// Local field list: the input columns to extract.
    ///
    /// Use either positional fields or `--field`, not both (exit 65).
    ///
    /// This is not the global `--select` / `--fields`: those project the whole
    /// NDJSON envelope after the extraction, while this flag chooses the input
    /// columns before anything is emitted.
    #[arg(
        long = "field",
        value_name = "FIELD",
        num_args = 1,
        action = clap::ArgAction::Append,
        help = "Local field to extract (repeatable; not global --select; do not combine with positionals)"
    )]
    pub field: Vec<String>,

    /// Delimiter for text mode (default: whitespace).
    #[arg(
        short = 'd',
        long,
        help = "Delimiter for text mode (default: whitespace)"
    )]
    pub delimiter: Option<String>,

    /// Read input from stdin.
    #[arg(long, help = "Read input from stdin", action = clap::ArgAction::SetTrue)]
    pub stdin: bool,
}

/// Arguments for the calc subcommand.
#[derive(Args, Debug)]
pub struct CalcArgs {
    /// Math expression to evaluate.
    ///
    /// The attribute stays: a stolen flag lands here as an expression that
    /// fails to evaluate, so the run exits 65 instead of answering with a
    /// plausible number for something else.
    #[arg(allow_hyphen_values = true)]
    pub expression: Option<String>,

    /// Read expressions from stdin.
    #[arg(long, help = "Read expressions from stdin (one per line)", action = clap::ArgAction::SetTrue)]
    pub stdin: bool,
}

/// Arguments for the transform subcommand.
#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    /// Paths to search for transforms.
    ///
    /// A verb with side effects requires the subject in the invocation line,
    /// so the root list is mandatory: omitting it is a usage error and never
    /// rewrites a tree chosen from ambient state. Name the root explicitly
    /// (e.g. `.` or `src/`). The multi-rule mode (`--rules` /
    /// `--inline-rules`) needs a root for the same reason.
    #[arg(required = true, value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// AST pattern to match.
    #[arg(
        short = 'p',
        long,
        allow_hyphen_values = true,
        help = "AST pattern to match (single-rule mode)"
    )]
    pub pattern: Option<String>,

    /// Rewrite template for matched patterns.
    #[arg(
        short = 'r',
        long,
        allow_hyphen_values = true,
        help = "Rewrite template (single-rule mode)"
    )]
    pub rewrite: Option<String>,

    /// Source language for AST parsing.
    #[arg(
        short = 'l',
        long = "language",
        help = "Language (rust, js, ts, py, go, etc.) — required for single-rule mode"
    )]
    pub language: Option<String>,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,

    /// Preview without writing.
    #[arg(long, help = "Show diff preview without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Path to a YAML file containing multiple rules.
    #[arg(
        long,
        conflicts_with = "inline_rules",
        help = "Apply multiple rules from a YAML file (conflicts with --inline-rules)",
        value_hint = ValueHint::FilePath
    )]
    pub rules: Option<PathBuf>,

    /// Inline YAML rules (alternative to --rules).
    #[arg(
        long,
        allow_hyphen_values = true,
        conflicts_with = "rules",
        help = "Apply multiple rules from inline YAML string (conflicts with --rules)"
    )]
    pub inline_rules: Option<String>,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,


    /// Re-parse output with tree-sitter to detect syntax errors introduced by the rewrite.
    #[arg(
        long,
        help = "Re-parse output with tree-sitter to verify syntax (exit 88 on error)",
        action = clap::ArgAction::SetTrue
    )]
    pub verify_parse: bool,
}

/// Arguments for the `prune-backups` subcommand.
///
/// At least one of `--max-age-secs` / `--max-count` is required (clap group).
#[derive(Args, Debug)]
#[command(group(
    clap::ArgGroup::new("prune_policy")
        .required(true)
        .args(["max_age_secs", "max_count"])
))]
pub struct PruneBackupsArgs {
    /// Target file paths whose `.bak.YYYYMMDD_HHMMSS` siblings will be
    /// considered for pruning.
    #[arg(required = true, value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// Maximum age (in seconds) of backups that survive. Backups whose
    /// creation stamp is strictly older than `now - max_age_secs` are pruned.
    /// When both `--max-age-secs` and `--max-count` are passed, age is
    /// applied first and count is applied to the survivors.
    #[arg(
        long,
        value_name = "SECONDS",
        help = "Drop backups older than N seconds"
    )]
    pub max_age_secs: Option<u32>,

    /// Maximum number of backups to keep (most recent by creation stamp).
    #[arg(long, value_name = "N", help = "Keep at most N most-recent backups")]
    pub max_count: Option<u8>,

    /// Treat a directory argument as a tree to search for backups.
    ///
    /// Without it a directory is refused, because a verb that deletes files
    /// must not guess a subject from a tree. With it, every `*.bak.<stamp>`
    /// under the root is a candidate, which is how an orphan copy left by
    /// `delete` is reached: no `--backup` target names it anymore.
    #[arg(
        short = 'r',
        long,
        help = "Search directory arguments for backups instead of refusing them"
    )]
    pub recursive: bool,

    /// Preview without deleting anything.
    #[arg(long, help = "Show what would be pruned without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Arguments for the apply subcommand.
#[derive(Args, Debug)]
pub struct ApplyArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    #[arg(value_hint = ValueHint::FilePath)]
    /// Target file to apply the patch to.
    ///
    /// Omit it when the patch names its own files: a path line above each
    /// file's blocks, or a pair of header lines per file. Without it, the
    /// path comes from the patch, never from ambient state.
    pub file: Option<PathBuf>,

    /// Patch format (default: auto-detect).
    #[arg(long, value_enum, default_value_t = PatchFormat::Auto, help = "Patch format: auto (detect, refuse when no marker), unified, search-replace, full (whole-file replacement), markdown")]
    pub format: PatchFormat,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Accept replacing a file above XDG `[write].confirm_large_bytes`.
    #[arg(long, help = "Acknowledge replacing a target above XDG [write].confirm_large_bytes", action = clap::ArgAction::SetTrue)]
    pub ack_overwrite: bool,

    /// Accept a result smaller than XDG `[write].shrink_block_percent`.
    #[arg(long, help = "Allow a result smaller than XDG [write].shrink_block_percent", action = clap::ArgAction::SetTrue)]
    pub allow_shrink: bool,

    /// Return the changed lines with their numbers, instead of only counts.
    ///
    /// The patch already knows what it changed, so this is the answer that
    /// removes the re-read a caller pays for just to confirm the swap.
    #[arg(long, help = "Return the changed lines with their numbers", action = clap::ArgAction::SetTrue)]
    pub show: bool,

    /// Replace every whole-line occurrence of each SEARCH block, not just one.
    ///
    /// Without it, a SEARCH that matches more than once is refused with
    /// `MATCH_AMBIGUOUS`, which is the safe answer for a blind rewrite.
    #[arg(long, help = "Replace every whole-line occurrence of each block", action = clap::ArgAction::SetTrue)]
    pub all: bool,

    /// Accept a patch that leaves the file with more syntax errors than before.
    #[arg(
        long,
        help = "Skip the syntax check that refuses a patch that breaks the parse",
        action = clap::ArgAction::SetTrue
    )]
    pub no_syntax_check: bool,
}

/// Identifier case style for the `case` subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum IdentifierCase {
    /// `snake_case`
    Snake,
    /// `camelCase`
    Camel,
    /// `PascalCase`
    Pascal,
    /// `kebab-case`
    Kebab,
    /// `SCREAMING_SNAKE_CASE`
    ScreamingSnake,
}

/// Arguments for the `query` subcommand.
///
/// Runs a tree-sitter S-expression pattern against a source file and
/// returns all matching AST nodes as NDJSON. Uses
/// `tree-sitter-language-pack` (downloads parsers on first use; 305
/// languages supported). Without `--query`, prints the parsed tree
/// structure as a compact JSON dump for debugging.
#[derive(Args, Debug)]
pub struct QueryArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    /// Source file to query.
    pub path: PathBuf,
    /// Tree-sitter language override (e.g. "rust", "python"). Auto-detected
    /// from extension if omitted.
    #[arg(
        long,
        value_name = "LANG",
        help = "Language override (auto-detected from extension)"
    )]
    pub language: Option<String>,
    /// Tree-sitter S-expression pattern (e.g. `(function_item name: (identifier) @name)`).
    ///
    /// Use `-Q` or `--query`: the global `-q` is quiet verbosity.
    #[arg(
        short = 'Q',
        long,
        allow_hyphen_values = true,
        value_name = "PATTERN",
        help = "S-expression pattern (-Q/--query; global -q is quiet, not query)"
    )]
    pub query: Option<String>,
    /// Print the full parse tree (no S-expression matching).
    #[arg(long, help = "Print the full tree (no S-expression matching)", action = clap::ArgAction::SetTrue)]
    pub tree: bool,
    /// Print all named node kinds found in the file (no S-expression matching).
    #[arg(long, help = "Print all named node kinds in the file (counts)", action = clap::ArgAction::SetTrue)]
    pub kinds: bool,
    /// Show byte offsets and start positions for every match.
    #[arg(
        long,
        help = "Include byte offsets and start positions for every match",
        action = clap::ArgAction::SetTrue
    )]
    pub positions: bool,
}

