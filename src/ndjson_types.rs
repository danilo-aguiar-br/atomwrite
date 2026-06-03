// SPDX-License-Identifier: MIT OR Apache-2.0

//! NDJSON output type definitions with schemars JSON Schema support.

use schemars::JsonSchema;
use serde::Serialize;

/// NDJSON output for write, delete, move, copy, and hash operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct PlatformInfo {
    /// File fsync method name (e.g. `F_FULLFSYNC` or `sync_data`).
    pub fsync: &'static str,
    /// Directory fsync method name (e.g. `sync_all` or `best_effort`).
    pub dir_fsync: &'static str,
}

/// NDJSON output for read operations with metadata and optional content.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct LineRange {
    /// First line number returned (1-based).
    pub start: usize,
    /// Last line number returned (1-based).
    pub end: usize,
}

/// NDJSON event emitted when search begins processing a file.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SearchBegin {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file being searched.
    pub path: String,
}

/// NDJSON event for a single search match within a file.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Submatch {
    /// The matched text.
    pub r#match: String,
    /// Byte offset of the match start within the line.
    pub start: usize,
    /// Byte offset of the match end within the line.
    pub end: usize,
}

/// NDJSON event for a context line surrounding a search match.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SearchEnd {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file that was searched.
    pub path: String,
    /// Per-file search statistics.
    pub stats: FileStats,
}

/// Per-file match and line statistics for search operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct FileStats {
    /// Number of matches found in the file.
    pub matches: u64,
    /// Total lines examined in the file.
    pub lines_searched: u64,
}

/// NDJSON event for count-only search mode (match count per file).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SearchCount {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file.
    pub path: String,
    /// Number of matches in the file.
    pub count: u64,
}

/// NDJSON event for files-only search mode (path of matching file).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SearchFile {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file with at least one match.
    pub path: String,
}

/// Aggregate summary emitted at the end of multi-file operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
    /// Whether the original modification time was preserved (true) or updated to now (false).
    /// Critical for build systems: false ensures cargo/make/cmake detect the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime_preserved: Option<bool>,
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
    /// Whether the original modification time was preserved (true) or updated to now (false).
    /// Critical for build systems: false ensures cargo/make/cmake detect the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime_preserved: Option<bool>,
}

/// NDJSON output for dry-run and diff preview operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct RollbackResult {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the restored file.
    pub path: String,
    /// Path of the backup that was restored.
    pub restored_from: String,
    /// BLAKE3 checksum before restoration.
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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

/// NDJSON output for diff stat mode.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DiffStatOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Whether files are identical.
    pub identical: bool,
    /// First file path.
    pub file_a: String,
    /// Second file path.
    pub file_b: String,
    /// Lines inserted.
    pub insertions: u64,
    /// Lines deleted.
    pub deletions: u64,
    /// Similarity ratio.
    pub similarity_ratio: f32,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for unified diff format.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DiffUnifiedOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Whether files are identical.
    pub identical: bool,
    /// Output format name.
    pub format: &'static str,
    /// Unified diff content.
    pub content: String,
    /// Similarity ratio.
    pub similarity_ratio: f32,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON event for a single diff change line.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DiffChangeOutput<'a> {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Change tag: insert or delete.
    pub tag: &'static str,
    /// Line number of the change.
    pub line: usize,
    /// Changed text content.
    pub text: &'a str,
}

/// NDJSON summary for a diff operation.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DiffSummaryOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Whether files are identical.
    pub identical: bool,
    /// First file path.
    pub file_a: String,
    /// Second file path.
    pub file_b: String,
    /// Line count of first file.
    pub lines_a: usize,
    /// Line count of second file.
    pub lines_b: usize,
    /// Similarity ratio.
    pub similarity_ratio: f32,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for BLAKE3 hash computation.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct HashOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// File path, absent for stdin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Source identifier for stdin input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<&'static str>,
    /// Hash algorithm name.
    pub algorithm: &'static str,
    /// Computed hash value.
    pub value: String,
    /// File size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// Verification result against expected hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON dry-run plan for move/copy operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct TransferPlan {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Operation name.
    pub operation: &'static str,
    /// Source path.
    pub source: String,
    /// Target path.
    pub target: String,
    /// Whether the operation would modify files.
    pub would_modify: bool,
}

/// NDJSON output for a completed move operation.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct MoveOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Source path.
    pub source: String,
    /// Target path.
    pub target: String,
    /// File size in bytes.
    pub bytes: u64,
    /// BLAKE3 checksum.
    pub checksum: String,
    /// Whether a cross-device copy was needed.
    pub cross_device: bool,
    /// Whether the operation was atomic.
    pub atomic: bool,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a completed copy operation.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct CopyOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Source path.
    pub source: String,
    /// Target path.
    pub target: String,
    /// File size in bytes.
    pub bytes: usize,
    /// BLAKE3 checksum.
    pub checksum: String,
    /// Whether checksum was verified.
    pub verified: bool,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for a completed delete operation.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct DeleteOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Deleted file path.
    pub path: String,
    /// File size in bytes before deletion.
    pub bytes: u64,
    /// BLAKE3 checksum before deletion.
    pub checksum_before: String,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for count grouped by file extension.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct CountByExtOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Count mode name.
    pub mode: &'static str,
    /// Counts grouped by extension.
    pub by_extension: std::collections::BTreeMap<String, ExtCountOutput>,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for total line/file counts.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct CountTotalOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Count mode name.
    pub mode: &'static str,
    /// Aggregate totals.
    pub total: CountTotals,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// Aggregate file and line counts.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct CountTotals {
    /// Total files counted.
    pub files: u64,
    /// Total lines counted.
    pub lines: u64,
    /// Total blank lines.
    pub blank: u64,
    /// Total bytes.
    pub bytes: u64,
}

/// Per-extension file and line counts.
#[derive(Default, Debug, PartialEq, Serialize, JsonSchema)]
pub struct ExtCountOutput {
    /// Files with this extension.
    pub files: u64,
    /// Lines in files with this extension.
    pub lines: u64,
    /// Blank lines in files with this extension.
    pub blank: u64,
    /// Bytes in files with this extension.
    pub bytes: u64,
}

/// NDJSON dry-run plan for backup operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct BackupPlan {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Operation name.
    pub operation: &'static str,
    /// File path.
    pub path: String,
    /// File size in bytes.
    pub bytes: u64,
    /// BLAKE3 checksum.
    pub checksum: String,
}

/// NDJSON summary for backup operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct BackupSummary {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Number of files backed up.
    pub files_backed_up: u64,
    /// Total bytes backed up.
    pub total_bytes: u64,
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON dry-run plan for rollback operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct RollbackPlan {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Operation name.
    pub operation: &'static str,
    /// Target file path.
    pub path: String,
    /// Backup path to restore from.
    pub restore_from: String,
}

/// NDJSON dry-run plan for patch apply operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ApplyPlan {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Operation name.
    pub operation: &'static str,
    /// Target file path.
    pub path: String,
    /// Detected patch format.
    pub format_detected: String,
    /// Number of hunks detected.
    pub hunks: usize,
}

/// NDJSON output for replace preview mode.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ReplacePreview {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// File path.
    pub path: String,
    /// Number of replacements.
    pub replacements: u64,
    /// Unified diff of changes.
    pub diff: String,
}

/// NDJSON error event for replace operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ReplaceErrorEvent {
    /// Error status.
    pub status: &'static str,
    /// File path.
    pub path: String,
    /// Error message.
    pub message: String,
    /// Error classification.
    pub error_class: &'static str,
    /// Whether the operation can be retried.
    pub retryable: bool,
}

/// NDJSON output for text field extraction.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TextFieldsOutput<'a> {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Extracted fields.
    pub fields: Vec<&'a str>,
}

/// NDJSON output for text value extraction.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TextValuesOutput<'a> {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Extracted values.
    pub values: Vec<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_valid_ndjson_object<T: serde::Serialize>(val: &T) {
        let json = serde_json::to_value(val).expect("serialize to Value");
        assert!(json.is_object(), "expected JSON object, got: {json}");
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("type"), "missing 'type' field");
    }

    fn assert_roundtrip_json<T: serde::Serialize>(val: &T) {
        let json_str = serde_json::to_string(val).expect("serialize to string");
        let reparsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("reparse from string");
        assert!(reparsed.is_object(), "roundtrip produced non-object");
    }

    #[test]
    fn roundtrip_write_output() {
        let val = WriteOutput {
            r#type: "write",
            status: "ok",
            path: "/tmp/test.rs".into(),
            bytes_written: 42,
            checksum: "abc123".into(),
            checksum_before: None,
            backup_path: None,
            elapsed_ms: 5,
            platform: PlatformInfo {
                fsync: "sync_data",
                dir_fsync: "sync_all",
            },
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_batch_summary() {
        let val = BatchSummary {
            r#type: "summary",
            operations: 10,
            succeeded: 9,
            failed: 1,
            dry_run: false,
            elapsed_ms: 100,
            transaction: Some(true),
            committed: Some(false),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_diff_stat() {
        let val = DiffStatOutput {
            r#type: "diff",
            identical: false,
            file_a: "a.rs".into(),
            file_b: "b.rs".into(),
            insertions: 10,
            deletions: 5,
            similarity_ratio: 0.85,
            elapsed_ms: 3,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_summary() {
        let val = Summary {
            r#type: "summary",
            files_visited: 100,
            files_matched: 5,
            files_modified: Some(3),
            files_skipped: None,
            total_matches: Some(42),
            total_replacements: None,
            elapsed_ms: 200,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_edit_output() {
        let val = EditOutput {
            r#type: "edit",
            path: "/tmp/edit.rs".into(),
            edits: 1,
            mode: "old_new".into(),
            bytes_before: 100,
            bytes_after: 110,
            checksum_before: "aaa".into(),
            checksum_after: "bbb".into(),
            lines_before: 10,
            lines_after: 11,
            elapsed_ms: 2,
            fuzzy: Some(true),
            strategy: Some("block_anchor".into()),
            strategies_tried: Some(8),
            similarity: Some(0.95),
            mtime_preserved: Some(false),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_diff_summary() {
        let val = DiffSummaryOutput {
            r#type: "summary",
            identical: true,
            file_a: "x.rs".into(),
            file_b: "y.rs".into(),
            lines_a: 50,
            lines_b: 50,
            similarity_ratio: 1.0,
            elapsed_ms: 1,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn skip_serializing_if_omits_none_fields() {
        let val = WriteOutput {
            r#type: "write",
            status: "ok",
            path: "/tmp/t.rs".into(),
            bytes_written: 10,
            checksum: "x".into(),
            checksum_before: None,
            backup_path: None,
            elapsed_ms: 1,
            platform: PlatformInfo {
                fsync: "sync_data",
                dir_fsync: "best_effort",
            },
        };
        let json = serde_json::to_value(&val).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("checksum_before"));
        assert!(!obj.contains_key("backup_path"));
    }

    #[test]
    fn roundtrip_read_output() {
        let val = ReadOutput {
            r#type: "read",
            path: "/tmp/read.rs".into(),
            content: Some("hello".into()),
            lines: 1,
            bytes: 5,
            checksum: "abc".into(),
            permissions: "0644".into(),
            modified: "2026-01-01T00:00:00Z".into(),
            kind: "file".into(),
            binary: false,
            range: None,
            verified: None,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_search_match() {
        let val = SearchMatch {
            r#type: "match",
            path: "/tmp/s.rs".into(),
            line_number: 10,
            lines: "fn main()".into(),
            byte_offset: 42,
            submatches: vec![],
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_replace_result() {
        let val = ReplaceResult {
            r#type: "replaced",
            path: "/tmp/r.rs".into(),
            replacements: 3,
            bytes_before: 100,
            bytes_after: 110,
            checksum_before: "aaa".into(),
            checksum_after: "bbb".into(),
            elapsed_ms: 5,
            mtime_preserved: Some(false),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_transform_result() {
        let val = TransformResult {
            r#type: "transform",
            path: "/tmp/t.rs".into(),
            language: "rust".into(),
            matches: 2,
            replacements: 2,
            bytes_before: 50,
            bytes_after: 55,
            checksum_before: "aa".into(),
            checksum_after: "bb".into(),
            elapsed_ms: 3,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_scope_result() {
        let val = ScopeResult {
            r#type: "scope",
            path: "/tmp/sc.rs".into(),
            language: "rust".into(),
            query: "comments".into(),
            action: "delete".into(),
            scopes_matched: 5,
            bytes_before: 200,
            bytes_after: 180,
            checksum_before: "x".into(),
            checksum_after: "y".into(),
            elapsed_ms: 10,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_backup_result() {
        let val = BackupResult {
            r#type: "backup",
            path: "/tmp/src.rs".into(),
            backup_path: "/tmp/src.rs.bak".into(),
            checksum: "hash".into(),
            bytes: 500,
            elapsed_ms: 2,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_rollback_result() {
        let val = RollbackResult {
            r#type: "rollback",
            path: "/tmp/rb.rs".into(),
            restored_from: "/tmp/rb.rs.bak".into(),
            checksum_before: Some("old".into()),
            checksum_after: "new".into(),
            verified: Some(true),
            elapsed_ms: 3,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_apply_result() {
        let val = ApplyResult {
            r#type: "apply",
            path: "/tmp/ap.rs".into(),
            format_detected: "unified".into(),
            hunks_applied: 2,
            bytes_before: 100,
            bytes_after: 120,
            checksum_before: "a".into(),
            checksum_after: "b".into(),
            elapsed_ms: 4,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_hash_output() {
        let val = HashOutput {
            r#type: "hash",
            path: Some("/tmp/h.rs".into()),
            source: None,
            algorithm: "blake3",
            value: "blake3hash".into(),
            bytes: Some(1024),
            verified: None,
            elapsed_ms: 1,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_calc_output() {
        let val = CalcOutput {
            r#type: "calc",
            expression: "2+2".into(),
            result: "4".into(),
            elapsed_ms: 1,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_regex_output() {
        let val = RegexOutput {
            r#type: "regex",
            regex: "\\d+".into(),
            examples: 3,
            anchored: false,
            elapsed_ms: 1,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_list_entry() {
        let val = ListEntry {
            r#type: "entry",
            path: "/tmp/le.rs".into(),
            kind: "file".into(),
            size: Some(100),
            modified: None,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_list_summary() {
        let val = ListSummary {
            r#type: "summary",
            files: 10,
            dirs: 3,
            symlinks: 0,
            total_bytes: Some(5000),
            by_extension: None,
            elapsed_ms: 15,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_dry_run_plan() {
        let val = DryRunPlan {
            r#type: "plan",
            operation: "write".into(),
            path: "/tmp/dr.rs".into(),
            would_modify: true,
            details: None,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_copy_output() {
        let val = CopyOutput {
            r#type: "copy",
            source: "/tmp/a.rs".into(),
            target: "/tmp/b.rs".into(),
            bytes: 200,
            checksum: "hash".into(),
            verified: true,
            elapsed_ms: 2,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_move_output() {
        let val = MoveOutput {
            r#type: "move",
            source: "/tmp/old.rs".into(),
            target: "/tmp/new.rs".into(),
            bytes: 300,
            checksum: "mhash".into(),
            cross_device: false,
            atomic: true,
            elapsed_ms: 3,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_delete_output() {
        let val = DeleteOutput {
            r#type: "delete",
            path: "/tmp/del.rs".into(),
            bytes: 150,
            checksum_before: "dhash".into(),
            elapsed_ms: 1,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_count_total_output() {
        let val = CountTotalOutput {
            r#type: "count",
            mode: "total",
            total: CountTotals {
                files: 50,
                lines: 2000,
                blank: 300,
                bytes: 50000,
            },
            elapsed_ms: 20,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_search_begin() {
        let val = SearchBegin {
            r#type: "begin",
            path: "/tmp/proj".into(),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_search_end() {
        let val = SearchEnd {
            r#type: "end",
            path: "/tmp/proj".into(),
            stats: FileStats {
                matches: 12,
                lines_searched: 50,
            },
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_transfer_plan() {
        let val = TransferPlan {
            r#type: "plan",
            operation: "copy",
            source: "/tmp/a.rs".into(),
            target: "/tmp/b.rs".into(),
            would_modify: true,
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_backup_plan() {
        let val = BackupPlan {
            r#type: "plan",
            operation: "backup",
            path: "/tmp/src.rs".into(),
            bytes: 500,
            checksum: "hash".into(),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_rollback_plan() {
        let val = RollbackPlan {
            r#type: "plan",
            operation: "rollback",
            path: "/tmp/rb.rs".into(),
            restore_from: "/tmp/rb.bak".into(),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }

    #[test]
    fn roundtrip_replace_preview() {
        let val = ReplacePreview {
            r#type: "preview",
            path: "/tmp/rp.rs".into(),
            replacements: 3,
            diff: "-old\n+new".into(),
        };
        assert_valid_ndjson_object(&val);
        assert_roundtrip_json(&val);
    }
}
