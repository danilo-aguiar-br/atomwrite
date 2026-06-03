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

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing")]
    pub dry_run: bool,
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
    #[arg(short = 'p', long, required = true, help = "AST pattern to match")]
    pub pattern: String,

    /// Rewrite template for matched patterns.
    #[arg(short = 'r', long, required = true, help = "Rewrite template")]
    pub rewrite: String,

    /// Source language for AST parsing.
    #[arg(
        short = 'l',
        long = "language",
        required = true,
        help = "Language (rust, js, ts, py, go, etc)"
    )]
    pub language: String,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,

    /// Preview without writing.
    #[arg(long, help = "Show diff preview without writing")]
    pub dry_run: bool,

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

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing")]
    pub dry_run: bool,
}
