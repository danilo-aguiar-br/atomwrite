// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by part1.inc.rs (A-MONO-001 — after EditLoopArgs).

/// Arguments for the search subcommand.
#[derive(Args, Debug)]
#[derive(Clone)]
pub struct SearchArgs {
    /// Search pattern (regex by default).
    #[arg(allow_hyphen_values = true)]
    pub pattern: String,

    /// Paths to search within.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,

    /// Treat pattern as regex.
    #[arg(
        short = 'e',
        long,
        conflicts_with = "fixed",
        help = "Treat pattern as regex (default)",
        action = clap::ArgAction::SetTrue
    )]
    pub regex: bool,

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

    /// Include FIFO / named pipe files in the search (G56).
    ///
    /// By default, atomwrite skips FIFOs because `open()` on a FIFO blocks
    /// indefinitely until the other end connects — this can cause atomwrite
    /// to hang in automated local runs / Docker environments that have FIFOs in /tmp or /var.
    /// Pass `--include-fifo` to opt back into the legacy behavior of
    /// opening FIFOs (which may hang).
    #[arg(
        long,
        help = "Include FIFO / named pipe files (default: skip to avoid hangs)",
        action = clap::ArgAction::SetTrue
    )]
    pub include_fifo: bool,

    /// Maximum file size in bytes for `search` (G68).
    ///
    /// Files larger than this are skipped silently (with a `skipped` event
    /// when `--count` or `--files` is active). Default: 10 MiB. Useful for
    /// skipping `node_modules`, `target/`, log archives, and other large
    /// generated files.
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_SEARCH_MAX_FILESIZE_BYTES,
        help = "Skip files larger than N bytes (default: constants::DEFAULT_SEARCH_MAX_FILESIZE_BYTES)"
    )]
    pub max_filesize: u64,

    /// Maximum line length in columns for `search` matches (G68).
    ///
    /// Lines longer than this are truncated with a `...[truncated]` marker.
    /// Default: 500. Useful for skipping minified bundle.js, styles.min.css,
    /// and other single-line giant files that explode context windows.
    #[arg(
        long,
        default_value_t = crate::constants::DEFAULT_SEARCH_MAX_COLUMNS,
        help = "Truncate matches longer than N columns (default: constants::DEFAULT_SEARCH_MAX_COLUMNS)"
    )]
    pub max_columns: usize,

    /// Search binary files (A-008). Default: skip files that look binary
    /// (`NUL` / `content_inspector`), matching ripgrep-style safety for agents.
    #[arg(
        long,
        help = "Search binary files (default: skip binary / NUL content)",
        action = clap::ArgAction::SetTrue
    )]
    pub binary: bool,

    /// Suppress per-file `begin` and `end` NDJSON events for files with
    /// zero matches (GAP-2026-010). Default: emit `begin`/`end` for every
    /// file visited (back-compat).
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

    /// Search target: content (default), files (basename), or both (v0.1.30).
    #[arg(
        long,
        value_enum,
        default_value_t = SearchTarget::Content,
        help = "Search target: content, files (basename), or both"
    )]
    pub target: SearchTarget,

    /// Skip first N matches when paginating results (v0.1.30).
    #[arg(long, default_value_t = 0, help = "Skip first N matches (pagination offset)")]
    pub offset: u64,

    /// Limit number of match events emitted (v0.1.30).
    #[arg(long, help = "Limit number of match events emitted")]
    pub limit: Option<u64>,
}

/// Arguments for the extract subcommand.
#[derive(Args, Debug)]
pub struct ExtractArgs {
    /// Field names or indices to extract.
    pub fields: Vec<String>,

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
    #[arg(allow_hyphen_values = true)]
    pub expression: Option<String>,

    /// Read expressions from stdin.
    #[arg(long, help = "Read expressions from stdin (one per line)", action = clap::ArgAction::SetTrue)]
    pub stdin: bool,
}

/// Arguments for the transform subcommand.
#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Paths to search for transforms.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
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

    /// Path to a YAML file containing multiple rules (G44).
    #[arg(long, help = "Apply multiple rules from a YAML file", value_hint = ValueHint::FilePath)]
    pub rules: Option<PathBuf>,

    /// Inline YAML rules (alternative to --rules).
    #[arg(
        long,
        allow_hyphen_values = true,
        help = "Apply multiple rules from inline YAML string"
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

/// Arguments for the `prune-backups` subcommand (ADR-0040).
///
/// G-036: at least one of `--max-age-secs` / `--max-count` is required (clap group).
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
    /// mtime is strictly older than `now - max_age_secs` are pruned.
    /// When both `--max-age-secs` and `--max-count` are passed, age is
    /// applied first and count is applied to the survivors.
    #[arg(
        long,
        value_name = "SECONDS",
        help = "Drop backups older than N seconds"
    )]
    pub max_age_secs: Option<u32>,

    /// Maximum number of backups to keep (most recent by mtime).
    #[arg(long, value_name = "N", help = "Keep at most N most-recent backups")]
    pub max_count: Option<u8>,

    /// Preview without deleting anything.
    #[arg(long, help = "Show what would be pruned without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Arguments for the apply subcommand.
#[derive(Args, Debug)]
pub struct ApplyArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    /// Target file to apply the patch to.
    pub file: PathBuf,

    /// Patch format (default: auto-detect).
    #[arg(long, value_enum, default_value_t = PatchFormat::Auto, help = "Patch format: auto, unified, search-replace, full, markdown")]
    pub format: PatchFormat,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
}

/// Identifier case style (v14 Tier 3 `case` subcommand).
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

/// Arguments for the `query` subcommand (v14 Tier 3, v0.1.12).
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
    /// **Must use `-Q` / `--query`** — global `-q` is quiet verbosity (A-005).
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

