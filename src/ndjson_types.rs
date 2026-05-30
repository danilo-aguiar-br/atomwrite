// SPDX-License-Identifier: MIT OR Apache-2.0

//! NDJSON output type definitions with schemars JSON Schema support.

use schemars::JsonSchema;
use serde::Serialize;

/// NDJSON output for write, delete, move, copy, and hash operations.
#[derive(Debug, Serialize, JsonSchema)]
pub struct WriteOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Operation outcome: "ok" or "error".
    pub status: &'static str,
    /// Absolute path of the target file.
    pub path: String,
    /// Number of bytes written.
    pub bytes_written: u64,
    /// BLAKE3 checksum after writing.
    pub checksum: String,
    /// BLAKE3 checksum before writing, if the file existed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_before: Option<String>,
    /// Backup file path, if a backup was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
    /// Platform-specific fsync methods used.
    pub platform: PlatformInfo,
}

/// Platform-specific fsync method names for diagnostics.
#[derive(Debug, Serialize, JsonSchema)]
pub struct PlatformInfo {
    /// File fsync method name (e.g. `F_FULLFSYNC` or `sync_data`).
    pub fsync: &'static str,
    /// Directory fsync method name (e.g. `sync_all` or `best_effort`).
    pub dir_fsync: &'static str,
}

/// NDJSON output for read operations with metadata and optional content.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ReadOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Absolute path of the file.
    pub path: String,
    /// File content, omitted in stat-only mode or for binary files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Total number of lines in the file.
    pub lines: u64,
    /// File size in bytes.
    pub bytes: u64,
    /// BLAKE3 checksum of the file contents.
    pub checksum: String,
    /// Filesystem permissions string.
    pub permissions: String,
    /// Last modification timestamp.
    pub modified: String,
    /// File kind (file, directory, symlink).
    pub kind: String,
    /// Whether the file was detected as binary.
    pub binary: bool,
    /// Line range returned when a subset was requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<LineRange>,
    /// Checksum verification result, if --verify-checksum was used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}

/// Inclusive 1-based line range for partial file reads.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LineRange {
    /// First line number returned (1-based).
    pub start: usize,
    /// Last line number returned (1-based).
    pub end: usize,
}

/// NDJSON event emitted when search begins processing a file.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchBegin {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file being searched.
    pub path: String,
}

/// NDJSON event for a single search match within a file.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchMatch {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file containing the match.
    pub path: String,
    /// 1-based line number of the match.
    pub line_number: u64,
    /// Matched line content.
    pub lines: String,
    /// Byte offset of the match from the start of the file.
    pub byte_offset: u64,
    /// Individual capture groups within the matched line.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub submatches: Vec<Submatch>,
}

/// A single regex capture within a matched line.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Submatch {
    /// The matched text.
    pub r#match: String,
    /// Byte offset of the match start within the line.
    pub start: usize,
    /// Byte offset of the match end within the line.
    pub end: usize,
}

/// NDJSON event for a context line surrounding a search match.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchContext {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file containing the context line.
    pub path: String,
    /// 1-based line number of the context line.
    pub line_number: u64,
    /// Context line content.
    pub lines: String,
}

/// NDJSON event emitted when search finishes processing a file.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchEnd {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file that was searched.
    pub path: String,
    /// Per-file search statistics.
    pub stats: FileStats,
}

/// Per-file match and line statistics for search operations.
#[derive(Debug, Serialize, JsonSchema)]
pub struct FileStats {
    /// Number of matches found in the file.
    pub matches: u64,
    /// Total lines examined in the file.
    pub lines_searched: u64,
}

/// NDJSON event for count-only search mode (match count per file).
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchCount {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file.
    pub path: String,
    /// Number of matches in the file.
    pub count: u64,
}

/// NDJSON event for files-only search mode (path of matching file).
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchFile {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file with at least one match.
    pub path: String,
}

/// Aggregate summary emitted at the end of multi-file operations.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Summary {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Total files examined.
    pub files_visited: u64,
    /// Files with at least one match.
    pub files_matched: u64,
    /// Files actually modified (replace/transform only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_modified: Option<u64>,
    /// Files skipped due to binary detection or size limits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_skipped: Option<u64>,
    /// Total matches found across all files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_matches: Option<u64>,
    /// Total replacements performed across all files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_replacements: Option<u64>,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a per-file replace operation.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ReplaceResult {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the modified file.
    pub path: String,
    /// Number of replacements performed.
    pub replacements: u64,
    /// File size in bytes before replacement.
    pub bytes_before: u64,
    /// File size in bytes after replacement.
    pub bytes_after: u64,
    /// BLAKE3 checksum before replacement.
    pub checksum_before: String,
    /// BLAKE3 checksum after replacement.
    pub checksum_after: String,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a surgical edit operation.
#[derive(Debug, Serialize, JsonSchema)]
pub struct EditOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the edited file.
    pub path: String,
    /// Number of edit operations applied.
    pub edits: u64,
    /// Edit mode used (e.g. `after_line`, `range`, `old_new`, `exact`).
    pub mode: String,
    /// File size in bytes before editing.
    pub bytes_before: u64,
    /// File size in bytes after editing.
    pub bytes_after: u64,
    /// BLAKE3 checksum before editing.
    pub checksum_before: String,
    /// BLAKE3 checksum after editing.
    pub checksum_after: String,
    /// Line count before editing.
    pub lines_before: u64,
    /// Line count after editing.
    pub lines_after: u64,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
    /// Whether fuzzy matching was used (only present in --old/--new mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuzzy: Option<bool>,
    /// Fuzzy strategy that succeeded (only present when fuzzy=true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    /// Number of strategies tried before success (only present in --old/--new mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategies_tried: Option<u64>,
    /// Similarity score of the fuzzy match, 0.0-1.0 (only present for `block_anchor` strategy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f64>,
}

/// NDJSON output for dry-run and diff preview operations.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DryRunPlan {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Name of the planned operation.
    pub operation: String,
    /// Path that would be affected.
    pub path: String,
    /// Whether the operation would modify the file.
    pub would_modify: bool,
    /// Additional details about the planned change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// NDJSON output for a single entry in a directory listing.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ListEntry {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the entry.
    pub path: String,
    /// Entry kind (file, dir, symlink).
    pub kind: String,
    /// File size in bytes, present when --long is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Last modification timestamp, present when --long is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

/// Aggregate summary for a directory listing operation.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ListSummary {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Total number of files found.
    pub files: u64,
    /// Total number of directories found.
    pub dirs: u64,
    /// Total number of symlinks found.
    pub symlinks: u64,
    /// Total bytes across all files, present when --long is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    /// File counts grouped by extension, present when --count-by-ext is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_extension: Option<std::collections::BTreeMap<String, u64>>,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for math expression evaluation and field extraction.
#[derive(Debug, Serialize, JsonSchema)]
pub struct CalcOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// The input expression that was evaluated.
    pub expression: String,
    /// Computed result as a string.
    pub result: String,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for regex generation from examples.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RegexOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Generated regular expression.
    pub regex: String,
    /// Number of input examples used.
    pub examples: u64,
    /// Whether the regex includes anchors.
    pub anchored: bool,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a structural AST-based code transform.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TransformResult {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the transformed file.
    pub path: String,
    /// Language used for AST parsing.
    pub language: String,
    /// Number of AST pattern matches found.
    pub matches: u64,
    /// Number of AST rewrites applied.
    pub replacements: u64,
    /// File size in bytes before transform.
    pub bytes_before: u64,
    /// File size in bytes after transform.
    pub bytes_after: u64,
    /// BLAKE3 checksum before transform.
    pub checksum_before: String,
    /// BLAKE3 checksum after transform.
    pub checksum_after: String,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a grammatical scoping operation on a single file.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ScopeResult {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the scoped file.
    pub path: String,
    /// Language used for scoping.
    pub language: String,
    /// Query name used.
    pub query: String,
    /// Action applied on matched scopes.
    pub action: String,
    /// Number of AST scopes matched.
    pub scopes_matched: u64,
    /// File size in bytes before scoping.
    pub bytes_before: u64,
    /// File size in bytes after scoping.
    pub bytes_after: u64,
    /// BLAKE3 checksum before scoping.
    pub checksum_before: String,
    /// BLAKE3 checksum after scoping.
    pub checksum_after: String,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a backup operation on a single file.
#[derive(Debug, Serialize, JsonSchema)]
pub struct BackupResult {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Original file path.
    pub path: String,
    /// Backup file path.
    pub backup_path: String,
    /// BLAKE3 checksum of the backed up file.
    pub checksum: String,
    /// File size in bytes.
    pub bytes: u64,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a rollback (restore from backup) operation.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RollbackResult {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the restored file.
    pub path: String,
    /// Path of the backup that was restored.
    pub restored_from: String,
    /// BLAKE3 checksum before restoration.
    pub checksum_before: Option<String>,
    /// BLAKE3 checksum after restoration.
    pub checksum_after: String,
    /// Whether checksum was verified post-restore.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a patch apply operation.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ApplyResult {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the patched file.
    pub path: String,
    /// Detected or specified patch format.
    pub format_detected: String,
    /// Number of hunks or blocks applied.
    pub hunks_applied: u64,
    /// File size in bytes before patching.
    pub bytes_before: u64,
    /// File size in bytes after patching.
    pub bytes_after: u64,
    /// BLAKE3 checksum before patching.
    pub checksum_before: String,
    /// BLAKE3 checksum after patching.
    pub checksum_after: String,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a single operation within a batch run.
#[derive(Debug, Serialize, JsonSchema)]
pub struct BatchOpResult<'a> {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Zero-based index of this operation in the manifest.
    pub index: u64,
    /// Operation name (e.g. "write", "delete", "replace").
    pub op: &'a str,
    /// Outcome: "ok" or "error".
    pub status: &'static str,
    /// Additional details about the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Error message if the operation failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// Aggregate summary emitted at the end of a batch run.
#[derive(Debug, Serialize, JsonSchema)]
pub struct BatchSummary {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Total operations in the manifest.
    pub operations: u64,
    /// Number of operations that succeeded.
    pub succeeded: u64,
    /// Number of operations that failed.
    pub failed: u64,
    /// Whether this was a dry-run execution.
    pub dry_run: bool,
    /// Total batch duration in milliseconds.
    pub elapsed_ms: u64,
    /// Whether transaction mode was active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<bool>,
    /// Whether the transaction was committed (all operations succeeded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub committed: Option<bool>,
}
