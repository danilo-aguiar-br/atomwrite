// SPDX-License-Identifier: MIT OR Apache-2.0

//! Subcommand argument structs and value enums for clap.

use std::path::PathBuf;

use clap::{Args, ValueEnum};

/// Arguments for shell completion script generation.
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Target shell for completion scripts.
    #[arg(value_enum)]
    pub shell: ShellType,

    /// Install completion script to XDG data directory.
    #[arg(
        long,
        help = "Install completion script to XDG data directory (Bash: ~/.local/share/bash-completion/completions/atomwrite)"
    )]
    pub install: bool,
}

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
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Expected BLAKE3 hash for verification.
    #[arg(long, help = "Verify file checksum against expected BLAKE3 hash")]
    pub verify: Option<String>,

    /// Hash content from stdin.
    #[arg(long, help = "Hash content from stdin instead of files")]
    pub stdin: bool,

    /// Recurse into directories.
    #[arg(short, long, help = "Recurse into directories")]
    pub recursive: bool,
}

/// Arguments for the delete subcommand.
#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// File paths to delete.
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Create backup before deleting.
    #[arg(long, help = "Create backup before deleting")]
    pub backup: bool,

    /// Number of backups to retain.
    #[arg(long, default_value_t = 5, help = "Number of backups to retain")]
    pub retention: u8,

    /// Recurse into directories.
    #[arg(short, long, help = "Recurse into directories")]
    pub recursive: bool,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,

    /// Preview without deleting.
    #[arg(long, help = "Show what would be done without deleting")]
    pub dry_run: bool,

    /// Skip confirmation prompt.
    #[arg(short = 'y', long, help = "Skip confirmation")]
    pub yes: bool,
}

/// Arguments for the count subcommand.
#[derive(Args, Debug)]
pub struct CountArgs {
    /// Paths to count within.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Group counts by file extension.
    #[arg(long, help = "Group counts by file extension")]
    pub by_extension: bool,

    /// Sort results by file size.
    #[arg(long, help = "Sort by file size (top N)")]
    pub by_size: bool,

    /// Number of top results to show.
    #[arg(long, default_value_t = 10, help = "Number of top results")]
    pub top: usize,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,
}

/// Arguments for the diff subcommand.
#[derive(Args, Debug)]
pub struct DiffArgs {
    /// First file to compare.
    pub file_a: PathBuf,
    /// Second file to compare.
    pub file_b: PathBuf,

    /// Output unified diff format.
    #[arg(long, help = "Output unified diff format")]
    pub unified: bool,

    /// Show only summary statistics.
    #[arg(long, help = "Only show summary statistics")]
    pub stat: bool,

    /// Lines of context in unified diff.
    #[arg(
        short = 'C',
        long,
        default_value_t = 3,
        help = "Lines of context in unified diff"
    )]
    pub context: usize,

    /// Diff algorithm to use.
    #[arg(long, value_enum, default_value_t = DiffAlgorithm::Patience, help = "Diff algorithm")]
    pub algorithm: DiffAlgorithm,
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

/// Arguments for the move subcommand.
#[derive(Args, Debug)]
pub struct MoveArgs {
    /// Source file path.
    pub source: PathBuf,
    /// Destination file path.
    pub target: PathBuf,

    /// Create backup of destination if it exists.
    #[arg(long, help = "Create backup of destination if it exists")]
    pub backup: bool,

    /// Number of backups to retain.
    #[arg(long, default_value_t = 5, help = "Number of backups to retain")]
    pub retention: u8,

    /// Overwrite destination if it exists.
    #[arg(short, long, help = "Overwrite destination if it exists")]
    pub force: bool,

    /// Preview without moving.
    #[arg(long, help = "Show what would be done without moving")]
    pub dry_run: bool,
}

/// Arguments for the copy subcommand.
#[derive(Args, Debug)]
pub struct CopyArgs {
    /// Source file path.
    pub source: PathBuf,
    /// Destination file path.
    pub target: PathBuf,

    /// Create backup of destination if it exists.
    #[arg(long, help = "Create backup of destination if it exists")]
    pub backup: bool,

    /// Overwrite destination if it exists.
    #[arg(short, long, help = "Overwrite destination if it exists")]
    pub force: bool,

    /// Copy directories recursively.
    #[arg(short, long, help = "Copy directories recursively")]
    pub recursive: bool,

    /// Preserve timestamps and permissions.
    #[arg(long, help = "Preserve timestamps and permissions")]
    pub preserve: bool,

    /// Preview without copying.
    #[arg(long, help = "Show what would be done without copying")]
    pub dry_run: bool,
}

/// Arguments for the read subcommand.
#[derive(Args, Debug)]
pub struct ReadArgs {
    /// File path to read.
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
    #[arg(long, help = "Return only metadata (no content)")]
    pub stat: bool,

    /// Output format selection.
    #[arg(long, value_enum, default_value_t = OutputFormat::Ndjson, help = "Output format")]
    pub format: OutputFormat,

    /// Expected BLAKE3 hash for verification.
    #[arg(long, help = "Verify file checksum against expected BLAKE3 hash")]
    pub verify_checksum: Option<String>,

    /// Filter lines matching this regex (substring of file content).
    #[arg(long, help = "Filter returned lines to those matching this regex")]
    pub grep: Option<String>,
}

/// Output format for the read subcommand.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Structured NDJSON output.
    Ndjson,
    /// Raw file content output.
    Raw,
}

/// Arguments for the write subcommand.
#[derive(Args, Debug)]
pub struct WriteArgs {
    /// Target file path.
    pub target: PathBuf,

    /// Create backup before overwriting.
    #[arg(long, help = "Create backup before overwriting")]
    pub backup: bool,

    /// Number of backups to retain.
    #[arg(long, default_value_t = 5, help = "Number of backups to retain")]
    pub retention: u8,

    /// Maximum input size in bytes.
    #[arg(long, help = "Maximum input size in bytes")]
    pub max_size: Option<u64>,

    /// Append content to end of file.
    #[arg(long, help = "Append content to end of existing file")]
    pub append: bool,

    /// Prepend content to beginning of file.
    #[arg(long, help = "Prepend content to beginning of existing file")]
    pub prepend: bool,

    /// Run post-write tree-sitter syntax check (G72). Aborts the
    /// write with exit code 88 if the new content has parse errors.
    /// Languages covered: rust, python, javascript, typescript, tsx,
    /// go, c, cpp, java, ruby, php, bash, html, css, json, yaml,
    /// toml, markdown, lua, scala, swift, kotlin, sql. Files with
    /// no parser available fall back to a bracket-balance heuristic.
    #[arg(
        long,
        help = "Run tree-sitter syntax check (G72). Aborts on parse errors (exit 88)."
    )]
    pub syntax_check: bool,

    /// Expected checksum for optimistic locking.
    #[arg(
        long,
        help = "Only write if current checksum matches (optimistic lock)"
    )]
    pub expect_checksum: Option<String>,

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
        help = "Allow zero-byte stdin (G120 L1 guard; default: reject empty stdin)"
    )]
    pub allow_empty_stdin: bool,

    /// Skip the `--expect-checksum` verification when the resolved
    /// stdin payload is empty (G120 L3 cross-validation). Use this
    /// when the combination `--append --expect-checksum <HASH> < /dev/null`
    /// is intentional (no-op append, checksum match preserved).
    #[arg(
        long,
        help = "Allow --expect-checksum to be skipped when stdin is empty (G120 L3)"
    )]
    pub no_checksum_when_empty: bool,

    /// WAL sidecar creation policy (G119 L1 prevention).
    ///
    /// `auto` (default) skips the sidecar for trivial writes (small file in
    /// a git-tracked directory, plain write, set/del). `always` forces the
    /// sidecar even for trivial cases (legacy semantics, equivalent to
    /// `--strict-atomic`). `never` suppresses sidecar creation entirely,
    /// even when `--strict-atomic` is set or `ATOMWRITE_WAL=1`.
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
        help = "Preserve original mtime/atime of the target file (default: update to now)"
    )]
    pub preserve_timestamps: bool,

    /// GAP-2026-011 L2: Require `--backup` to be set. Aborts the write with
    /// exit 65 if the target file exists and `--backup` is not provided.
    /// Useful for CI/CD pipelines where backups are non-negotiable.
    #[arg(
        long,
        help = "Require --backup; abort if missing and target file exists (defense-in-depth L2)"
    )]
    pub require_backup: bool,

    /// GAP-2026-011 L3: Interactive Y/N confirmation when the target file
    /// exists and is larger than 100KB. Reads from stdin; abort if input
    /// is not "y" or "yes".
    #[arg(
        long,
        help = "Require interactive Y/N confirmation for large files (>100KB) (defense-in-depth L3)"
    )]
    pub confirm: bool,

    /// GAP-2026-011 L5: Auto-rotation. When `--backup` is active, ensures a
    /// rotation backup is created if the target file was modified within
    /// the last 24 hours (heuristic: recent files need backups).
    #[arg(
        long,
        help = "Force auto-rotation backup for recently-modified files (<24h) (defense-in-depth L5)"
    )]
    pub auto_rotate: bool,

    /// GAP-2026-011: Size delta threshold (in percent) to trigger the
    /// L1 size guard warning. Default: 50 (any change larger than 50%
    /// of the original file size emits a warning to stderr).
    #[arg(
        long,
        value_name = "PERCENT",
        default_value_t = 50,
        help = "Size delta threshold in percent to trigger risk warning (L1, default: 50)"
    )]
    pub risk_threshold: u8,

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing")]
    pub dry_run: bool,

    /// v0.1.21 GAP-014 v2: keep the backup after a successful write.
    /// By default the backup is deleted quietly (cleanup is idempotent).
    /// Use this flag to retain the backup for audit or version control.
    /// On failure the backup is ALWAYS preserved regardless of this flag.
    #[arg(
        long,
        help = "Keep backup after success (default: delete quietly). On failure backup is always preserved."
    )]
    pub keep_backup: bool,
}

/// Fuzzy matching behavior for --old/--new edit mode.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FuzzyMode {
    /// Try exact match first, then fuzzy strategies automatically.
    Auto,
    /// Exact match only, no fuzzy fallback.
    Off,
    /// Try all fuzzy strategies including low-confidence block anchor.
    Aggressive,
}

/// Arguments for the edit subcommand.
#[derive(Args, Debug)]
pub struct EditArgs {
    /// File path to edit.
    pub path: PathBuf,

    /// Insert stdin content after line N.
    #[arg(long, help = "Insert content from stdin after line N")]
    pub after_line: Option<usize>,

    /// Insert stdin content before line N.
    #[arg(long, help = "Insert content from stdin before line N")]
    pub before_line: Option<usize>,

    /// Replace line range N:M with stdin content.
    #[arg(long, help = "Replace line range N:M with stdin content")]
    pub range: Option<String>,

    /// Delete line range N:M.
    #[arg(long, help = "Delete line range N:M")]
    pub delete_range: Option<String>,

    /// Insert stdin content after first match of text.
    #[arg(long, help = "Insert stdin content after first match of text")]
    pub after_match: Option<String>,

    /// Insert stdin content before first match of text.
    #[arg(long, help = "Insert stdin content before first match of text")]
    pub before_match: Option<String>,

    /// Two markers delimiting content to replace with stdin.
    #[arg(
        long,
        num_args = 2,
        help = "Replace content between two markers with stdin"
    )]
    pub between: Option<Vec<String>>,

    /// Exact text to find (repeatable for multiple replacements).
    #[arg(long, action = clap::ArgAction::Append, help = "Exact text to find (repeatable)")]
    pub old: Vec<String>,

    /// Replacement text for --old (repeatable, must match --old count).
    #[arg(long, action = clap::ArgAction::Append, help = "Replacement text for --old (repeatable)")]
    pub new: Vec<String>,

    /// Fuzzy matching mode for --old/--new.
    #[arg(
        long,
        value_enum,
        default_value_t = FuzzyMode::Auto,
        help = "Fuzzy match mode for --old/--new: auto, off, aggressive"
    )]
    pub fuzzy: FuzzyMode,

    /// Read multiple edit operations as NDJSON from stdin.
    #[arg(long, help = "Read multiple edit operations as NDJSON from stdin")]
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
    #[arg(long, help = "Show what would be done without writing")]
    pub dry_run: bool,

    /// Preserve original modification time (mtime) of the file.
    /// Default is false: mtime is updated to reflect the edit.
    /// Set true for backup workflows, version control snapshots, or
    /// reproducible builds that depend on stable timestamps.
    /// Note: setting true may break build systems that use mtime to
    /// detect source changes (cargo, make, cmake, gradle).
    #[arg(long, help = "Preserve original mtime (default: update mtime to now)")]
    pub preserve_timestamps: bool,

    /// Apply only the `--old`/`--new` pairs that match instead of failing the
    /// whole batch (G117). Default (off) is all-or-nothing: any unmatched pair
    /// aborts with exit 65 and no write. With `--partial`, unmatched pairs are
    /// reported in `pair_results` with `matched: false`; if zero pairs apply,
    /// the command exits 1 (`NO_MATCHES`) without writing.
    #[arg(
        long,
        help = "Apply matching --old/--new pairs and report the rest (default: all-or-nothing)"
    )]
    pub partial: bool,

    /// WAL sidecar creation policy (G119 L1 prevention). See `write` for
    /// the full description; the same enum applies to edit operations
    /// (which by default DO create a sidecar because they lack native
    /// atomicity).
    #[arg(
        long,
        value_enum,
        default_value_t = crate::wal::WalPolicy::Auto,
        help = "WAL sidecar policy: auto (default), always, never (G119 L1)"
    )]
    pub wal_policy: crate::wal::WalPolicy,

    /// v0.1.21 GAP-013 C: create a `.bak` file before editing. Default is
    /// `false` for back-compat; pass `--backup` to enable the same
    /// pre-write snapshot semantics as `write --backup`.
    #[arg(
        long,
        help = "Create .bak backup before editing (default: false; paridade com write/replace)"
    )]
    pub backup: bool,

    /// v0.1.21 GAP-013 C: number of backups to retain when --backup is active.
    /// Mirrors `write --retention`. Default: 5.
    #[arg(
        long,
        default_value_t = 5,
        help = "Maximum number of backups to retain when --backup is active (default: 5)"
    )]
    pub retention: u8,

    /// v0.1.21 GAP-014 v2: keep the backup after a successful edit.
    /// By default the backup is deleted quietly. On failure the backup
    /// is ALWAYS preserved regardless of this flag.
    #[arg(
        long,
        help = "Keep backup after success (default: delete quietly). On failure backup is always preserved."
    )]
    pub keep_backup: bool,

    /// v0.1.21 GAP-012: accept `STATE_DRIFT` between sequential edits by
    /// the same agent. Use this when chaining multiple `edit` calls
    /// without re-capturing the checksum. Default (off) keeps the
    /// fail-loud `STATE_DRIFT` for true concurrency.
    #[arg(
        long,
        help = "Accept STATE_DRIFT between sequential edits (default: reject). For agent pipelines that chain edits."
    )]
    pub allow_sequential_drift: bool,
}

/// Arguments for the `edit-loop` subcommand (ADR-0039).
#[derive(Args, Debug)]
pub struct EditLoopArgs {
    /// File path to apply all pairs against.
    pub path: PathBuf,

    /// Accept `STATE_DRIFT` between iterations (default: reject). Useful
    /// when chaining edits; for `edit-loop` this is informational since
    /// the whole batch is computed in memory and a single atomic write
    /// is performed at the end (no per-pair drift to validate).
    #[arg(
        long,
        help = "Accept STATE_DRIFT (informational for edit-loop; default: reject)"
    )]
    pub allow_sequential_drift: bool,

    /// Create a `.bak` snapshot of the target before writing.
    #[arg(long, help = "Create .bak backup before writing (default: false)")]
    pub backup: bool,

    /// Number of backups to retain when `--backup` is active.
    #[arg(
        long,
        default_value_t = 5,
        value_name = "N",
        help = "Maximum number of backups to retain when --backup is active (default: 5)"
    )]
    pub retention: u8,

    /// Keep the backup after a successful write (default: delete quietly).
    #[arg(
        long,
        help = "Keep backup after success (default: delete quietly). On failure backup is always preserved."
    )]
    pub keep_backup: bool,

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

/// Arguments for the search subcommand.
#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search pattern (regex by default).
    pub pattern: String,

    /// Paths to search within.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Treat pattern as regex.
    #[arg(short = 'e', long, help = "Treat pattern as regex (default)")]
    pub regex: bool,

    /// Treat pattern as fixed string.
    #[arg(short = 'F', long, help = "Treat pattern as fixed string")]
    pub fixed: bool,

    /// Match whole words only.
    #[arg(short = 'w', long, help = "Match whole words only")]
    pub word: bool,

    /// Case-insensitive search.
    #[arg(short = 'i', long, help = "Case-insensitive search")]
    pub case_insensitive: bool,

    /// Smart case: insensitive when pattern is lowercase.
    #[arg(
        short = 'S',
        long,
        help = "Smart case: insensitive if pattern is lowercase"
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
    #[arg(short = 'c', long, help = "Only show match count per file")]
    pub count: bool,

    /// Show only filenames with matches.
    #[arg(short = 'l', long, help = "Only show filenames with matches")]
    pub files: bool,

    /// Maximum matches per file.
    #[arg(short = 'm', long, help = "Maximum matches per file")]
    pub max_count: Option<u64>,

    /// Enable multi-line matching.
    #[arg(short = 'U', long, help = "Enable multi-line matching")]
    pub multiline: bool,

    /// Show non-matching lines.
    #[arg(long, help = "Show lines that do NOT match")]
    pub invert: bool,

    /// Sort results by criterion.
    #[arg(long, value_enum, help = "Sort results by criterion")]
    pub sort: Option<SortBy>,

    /// Include FIFO / named pipe files in the search (G56).
    ///
    /// By default, atomwrite skips FIFOs because `open()` on a FIFO blocks
    /// indefinitely until the other end connects — this can cause atomwrite
    /// to hang in CI / Docker environments that have FIFOs in /tmp or /var.
    /// Pass `--include-fifo` to opt back into the legacy behavior of
    /// opening FIFOs (which may hang).
    #[arg(
        long,
        help = "Include FIFO / named pipe files (default: skip to avoid hangs)"
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
        default_value_t = 10 * 1024 * 1024,
        help = "Skip files larger than N bytes (default: 10 MiB)"
    )]
    pub max_filesize: u64,

    /// Maximum line length in columns for `search` matches (G68).
    ///
    /// Lines longer than this are truncated with a `...[truncated]` marker.
    /// Default: 500. Useful for skipping minified bundle.js, styles.min.css,
    /// and other single-line giant files that explode context windows.
    #[arg(
        long,
        default_value_t = 500,
        help = "Truncate matches longer than N columns (default: 500)"
    )]
    pub max_columns: usize,

    /// Suppress per-file `begin` and `end` NDJSON events for files with
    /// zero matches (GAP-2026-010). Default: emit `begin`/`end` for every
    /// file visited (back-compat).
    #[arg(
        long,
        help = "Suppress begin/end events for files with no matches (cleaner output for empty searches)"
    )]
    pub no_begin_end: bool,
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
    pub pattern: String,
    /// Replacement text.
    pub replacement: String,

    /// Paths to search within.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Treat pattern as regex.
    #[arg(long, help = "Treat pattern as regex")]
    pub regex: bool,

    /// Match whole words only.
    #[arg(short = 'w', long, help = "Match whole words only")]
    pub word: bool,

    /// Treat pattern as literal string.
    #[arg(
        short = 'F',
        long,
        help = "Treat pattern as literal string (escape regex chars)"
    )]
    pub literal: bool,

    /// Create backup before modifying.
    #[arg(long, help = "Create backup before modifying")]
    pub backup: bool,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,

    /// Show diff preview without writing.
    #[arg(long, help = "Show diff preview without writing")]
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
    #[arg(long, help = "Show what would be done without writing")]
    pub dry_run: bool,

    /// Preserve original modification time (mtime) of replaced files.
    /// Default is false: mtime is updated to reflect the change.
    /// Set true for backup workflows, version control snapshots, or
    /// reproducible builds that depend on stable timestamps.
    /// Note: setting true may break build systems that use mtime to
    /// detect source changes (cargo, make, cmake, gradle).
    #[arg(long, help = "Preserve original mtime (default: update mtime to now)")]
    pub preserve_timestamps: bool,

    /// v0.1.21 GAP-014 v2: keep backups after a successful replace.
    /// Default is to delete them quietly. On failure the backup is
    /// always preserved regardless of this flag.
    #[arg(
        long,
        help = "Keep backup after success (default: delete quietly). On failure backup is always preserved."
    )]
    pub keep_backup: bool,
}

/// Arguments for the list subcommand.
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Paths to list.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Maximum directory depth.
    #[arg(short = 'd', long, help = "Maximum directory depth")]
    pub depth: Option<usize>,

    /// Show size and modification time.
    #[arg(short = 'l', long, help = "Show size and modification time")]
    pub long: bool,

    /// Group file counts by extension.
    #[arg(long, help = "Group file counts by extension")]
    pub count_by_ext: bool,

    /// Show all files including hidden.
    #[arg(long, help = "Show all files including hidden")]
    pub all: bool,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,
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
    #[arg(long, help = "Read input from stdin")]
    pub stdin: bool,
}

/// Arguments for the calc subcommand.
#[derive(Args, Debug)]
pub struct CalcArgs {
    /// Math expression to evaluate.
    pub expression: Option<String>,

    /// Read expressions from stdin.
    #[arg(long, help = "Read expressions from stdin (one per line)")]
    pub stdin: bool,
}

/// Arguments for the regex subcommand.
#[derive(Args, Debug)]
pub struct RegexArgs {
    /// Example strings for regex generation.
    pub examples: Vec<String>,

    /// Read examples from stdin.
    #[arg(long, help = "Read examples from stdin (one per line)")]
    pub stdin: bool,

    /// Convert digits to \\d.
    #[arg(short = 'd', long, help = "Convert digits to \\d")]
    pub digits: bool,

    /// Convert words to \\w.
    #[arg(short = 'w', long, help = "Convert words to \\w")]
    pub words: bool,

    /// Convert whitespace to \\s.
    #[arg(short = 's', long, help = "Convert whitespace to \\s")]
    pub spaces: bool,

    /// Detect repetitions.
    #[arg(short = 'r', long, help = "Detect repetitions")]
    pub repetitions: bool,

    /// Case-insensitive matching.
    #[arg(short = 'i', long, help = "Case-insensitive matching")]
    pub case_insensitive: bool,

    /// Remove anchors (^ and $).
    #[arg(long, help = "Remove anchors (^ and $)")]
    pub no_anchors: bool,
}

/// Arguments for the transform subcommand.
#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Paths to search for transforms.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// AST pattern to match.
    #[arg(short = 'p', long, help = "AST pattern to match (single-rule mode)")]
    pub pattern: Option<String>,

    /// Rewrite template for matched patterns.
    #[arg(short = 'r', long, help = "Rewrite template (single-rule mode)")]
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
    #[arg(long, help = "Show diff preview without writing")]
    pub dry_run: bool,

    /// Path to a YAML file containing multiple rules (G44).
    #[arg(long, help = "Apply multiple rules from a YAML file")]
    pub rules: Option<PathBuf>,

    /// Inline YAML rules (alternative to --rules).
    #[arg(long, help = "Apply multiple rules from inline YAML string")]
    pub inline_rules: Option<String>,

    /// Create backup before modifying.
    #[arg(long, help = "Create backup before modifying")]
    pub backup: bool,
}

/// Arguments for the batch subcommand.
#[derive(Args, Debug)]
pub struct BatchArgs {
    /// Preview without executing.
    #[arg(long, help = "Show what would be done without executing")]
    pub dry_run: bool,

    /// Manifest file path (default: stdin).
    #[arg(long, help = "Read manifest from file instead of stdin")]
    pub file: Option<PathBuf>,

    /// Execute all operations as a transaction (all-or-nothing).
    #[arg(
        long,
        help = "All-or-nothing: rollback all changes if any operation fails"
    )]
    pub transaction: bool,

    /// Emit JSON Schema for the NDJSON input manifest format.
    #[arg(long, help = "Print JSON Schema for the batch input manifest")]
    pub input_schema: bool,

    /// Hint for NDJSON streaming: number of operations to buffer before
    /// emitting the summary line (G77).
    ///
    /// atomwrite reads the manifest incrementally (one line at a time), so
    /// memory usage is O(1) regardless of this value. This flag only
    /// controls the granularity of the final `summary` event. Default: 100.
    #[arg(
        long,
        default_value_t = 100usize,
        help = "Operations to buffer before emitting the summary line (default: 100)"
    )]
    pub batch_size: usize,

    /// v0.1.21 GAP-014 v2: keep per-op backups after success. Applies to
    /// every operation in the batch that creates a backup. Default is
    /// to delete each backup quietly once its op completes.
    #[arg(
        long,
        help = "Keep per-op backups after success (default: delete quietly). On failure backups are always preserved."
    )]
    pub keep_backup: bool,
}

/// Arguments for the backup subcommand.
#[derive(Args, Debug)]
pub struct BackupArgs {
    /// File paths to back up.
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Directory to store backups (default: same as source).
    #[arg(long, help = "Directory to store backup files")]
    pub output_dir: Option<PathBuf>,

    /// Maximum number of backups to retain per file.
    #[arg(long, default_value_t = 5, help = "Number of backup copies to keep")]
    pub retention: u8,

    /// Preview without creating backups.
    #[arg(long, help = "Show what would be done without writing")]
    pub dry_run: bool,
}

/// Arguments for the `prune-backups` subcommand (ADR-0040).
#[derive(Args, Debug)]
pub struct PruneBackupsArgs {
    /// Target file paths whose `.bak.YYYYMMDD_HHMMSS` siblings will be
    /// considered for pruning.
    #[arg(required = true)]
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
    #[arg(long, help = "Show what would be pruned without writing")]
    pub dry_run: bool,
}

/// Arguments for the rollback subcommand.
#[derive(Args, Debug)]
pub struct RollbackArgs {
    /// File path to restore from backup.
    pub path: PathBuf,

    /// Restore a specific backup by timestamp (`YYYYMMDD_HHMMSS`).
    #[arg(long, help = "Timestamp of the backup to restore")]
    pub timestamp: Option<String>,

    /// Restore the most recent backup.
    #[arg(long, help = "Restore the most recent backup (default)")]
    pub latest: bool,

    /// Verify BLAKE3 checksum after restore.
    #[arg(long, help = "Verify checksum after restoring")]
    pub verify: bool,

    /// v0.1.21 GAP-013 C: create a `.bak` snapshot of the current target
    /// before restoring the chosen backup. Default `false` to keep the
    /// rollback lean. Pass `--backup` when you want an extra safety net.
    #[arg(
        long,
        help = "Create .bak of the current target before restoring (default: false; paridade com edit)"
    )]
    pub backup: bool,

    /// v0.1.21 GAP-014 v2: keep the pre-rollback snapshot after success.
    /// Default is to delete it quietly. On failure it is always preserved.
    #[arg(
        long,
        help = "Keep pre-rollback snapshot after success (default: delete quietly)"
    )]
    pub keep_backup: bool,

    /// v0.1.21: number of backups to retain when --keep-backup is active.
    #[arg(
        long,
        default_value_t = 5,
        help = "Number of backups to retain when --keep-backup is active (default: 5)"
    )]
    pub retention: u8,

    /// Preview without restoring.
    #[arg(long, help = "Show what would be done without writing")]
    pub dry_run: bool,
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

/// Arguments for the apply subcommand.
#[derive(Args, Debug)]
pub struct ApplyArgs {
    /// Target file to apply the patch to.
    pub file: PathBuf,

    /// Patch format (default: auto-detect).
    #[arg(long, value_enum, default_value_t = PatchFormat::Auto, help = "Patch format: auto, unified, search-replace, full, markdown")]
    pub format: PatchFormat,

    /// Create backup before applying patch.
    #[arg(long, help = "Create backup of target before patching")]
    pub backup: bool,

    /// v0.1.21 GAP-014 v2: keep the backup after a successful apply.
    /// Default is to delete it quietly. On failure the backup is always
    /// preserved regardless of this flag.
    #[arg(
        long,
        help = "Keep backup after success (default: delete quietly). On failure backup is always preserved."
    )]
    pub keep_backup: bool,

    /// v0.1.21: number of backups to retain when --keep-backup is active.
    #[arg(
        long,
        default_value_t = 5,
        help = "Number of backups to retain when --keep-backup is active (default: 5)"
    )]
    pub retention: u8,

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing")]
    pub dry_run: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// v14 Tier 3: structured config edits + identifier case conversion.
// ─────────────────────────────────────────────────────────────────────────────

/// Arguments for the `set` subcommand (v14 Tier 3).
#[derive(Args, Debug)]
pub struct SetArgs {
    /// Path to the structured config file (TOML or JSON).
    pub path: PathBuf,
    /// Dotted path to the key (e.g. `package.version`).
    pub key_path: String,
    /// New value (auto-coerced to bool/int/float/string).
    pub value: String,
    /// Create backup before modifying.
    #[arg(long, help = "Create backup before modifying")]
    pub backup: bool,
    /// Preserve original file timestamps.
    #[arg(long, help = "Preserve original mtime/atime")]
    pub preserve_timestamps: bool,
}

/// Arguments for the `get` subcommand (v14 Tier 3).
#[derive(Args, Debug)]
pub struct GetArgs {
    /// Path to the structured config file (TOML or JSON).
    pub path: PathBuf,
    /// Dotted path to the key (e.g. `package.version`).
    pub key_path: String,
}

/// Arguments for the `del` subcommand (v14 Tier 3).
#[derive(Args, Debug)]
pub struct DelArgs {
    /// Path to the structured config file (TOML or JSON).
    pub path: PathBuf,
    /// Dotted path to the key (e.g. `dependencies.serde`).
    pub key_path: String,
    /// Create backup before modifying.
    #[arg(long, help = "Create backup before modifying")]
    pub backup: bool,
    /// Preserve original file timestamps.
    #[arg(long, help = "Preserve original mtime/atime")]
    pub preserve_timestamps: bool,
    /// Treat missing key as a no-op success instead of an error.
    #[arg(long, help = "Succeed silently if the key is already missing")]
    pub force_missing: bool,
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

/// Arguments for the `case` subcommand (v14 Tier 3).
#[derive(Args, Debug)]
pub struct CaseArgs {
    /// Target file paths to rewrite.
    pub paths: Vec<PathBuf>,
    /// Pairs of old new identifiers (must be even count).
    #[arg(long = "subvert", num_args = 2.., value_name = "OLD NEW", help = "Old and new identifier (repeat for multiple pairs)")]
    pub subvert: Vec<String>,
    /// Target case style for the new identifier.
    #[arg(long, value_enum, default_value_t = IdentifierCase::Snake, help = "Target case style")]
    pub to: IdentifierCase,
    /// Create backup before modifying.
    #[arg(long, help = "Create backup before modifying")]
    pub backup: bool,
    /// Preserve original file timestamps.
    #[arg(long, help = "Preserve original mtime/atime")]
    pub preserve_timestamps: bool,
    /// Preview without writing.
    #[arg(long, help = "Show what would be changed without writing")]
    pub dry_run: bool,
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
    #[arg(
        short = 'Q',
        long,
        value_name = "PATTERN",
        help = "S-expression pattern (e.g. '(function_item name: (identifier) @name)')"
    )]
    pub query: Option<String>,
    /// Print the full parse tree (no S-expression matching).
    #[arg(long, help = "Print the full tree (no S-expression matching)")]
    pub tree: bool,
    /// Print all named node kinds found in the file (no S-expression matching).
    #[arg(long, help = "Print all named node kinds in the file (counts)")]
    pub kinds: bool,
    /// Show byte offsets and start positions for every match.
    #[arg(
        long,
        help = "Include byte offsets and start positions for every match"
    )]
    pub positions: bool,
}

/// Arguments for the `outline` subcommand (v14 Tier 3, v0.1.12).
///
/// Extracts the high-level structure of a source file (functions,
/// classes, structs, enums, traits, modules, top-level consts) as
/// NDJSON. Uses `tree-sitter-language-pack`. Without `--kind`, emits
/// all structural items.
#[derive(Args, Debug)]
pub struct OutlineArgs {
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
    #[arg(long, help = "Include byte offsets and start positions")]
    pub positions: bool,
}

/// Arguments for the `wal-stats` subcommand (G119 L5 telemetry).
///
/// Computes a snapshot of all `.atomwrite.journal.*.json` sidecars
/// under the workspace, classified by terminal state (`Started`/
/// `Committed`/`Aborted`/malformed) and broken down by directory.
/// Read-only and safe to call from any context.
#[derive(Args, Debug)]
pub struct WalStatsArgs {
    /// Preview without scanning the workspace.
    #[arg(long, help = "Show what would be done without scanning")]
    pub dry_run: bool,
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
        default_value_t = 3600,
        help = "Minimum age (seconds) for removal"
    )]
    pub threshold_secs: u64,

    /// Wall-clock budget for the walk (milliseconds). The pass stops
    /// once this budget is exceeded so startup cost is bounded.
    #[arg(
        long,
        default_value_t = 100,
        help = "Wall-clock budget (ms) for the walk"
    )]
    pub max_duration_ms: u64,

    /// Preview without removing any sidecar.
    #[arg(long, help = "Show what would be removed without writing")]
    pub dry_run: bool,
}
