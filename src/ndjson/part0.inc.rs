


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
    /// Number of bytes read from stdin (G120 L4 local diagnostics). 0 means the
    /// caller passed empty stdin and explicitly opted in via
    /// `--allow-empty-stdin`.
    pub stdin_bytes_read: u64,
    /// G119 L1: sidecar creation policy in effect for this write.
    /// "auto" (default heuristic), "always" (legacy), or "never"
    /// (suppressed).
    pub wal_policy: &'static str,
    /// Platform-specific fsync methods used.
    pub platform: PlatformInfo,
    /// Whether the original mtime was preserved (--preserve-timestamps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime_preserved: Option<bool>,
    /// GAP-2026-011 L6: risk diagnostics for size-delta warnings. Omitted
    /// when the write is below the configured risk threshold (default
    /// 50% delta) and no other guard was triggered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_assessment: Option<WriteRiskAssessment>,
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

/// Closest near-miss when a fuzzy/exact match fails (v0.1.29 P0-2).
///
/// Agents use this to correct `old` without re-reading the full file.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct BestCandidate {
    /// Snippet of the best matching region (truncated to 500 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// 1-based line of the candidate start in the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    /// 1-based column of the candidate start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u64>,
    /// Similarity score 0.0–1.0 of the best attempt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f64>,
    /// Strategy that produced this candidate score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    /// Optional short unified-diff style preview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_preview: Option<String>,
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

/// One entry in `CountBySizeOutput.items` (GAP-2026-001).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SizeEntry {
    /// Absolute or workspace-relative file path.
    pub path: String,
    /// File size in bytes.
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

// ============================================================================
// v0.1.22 — prune-backups NDJSON output types
// ============================================================================

/// NDJSON per-entry event for `prune-backups` (pruned, skipped, or error).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct PruneBackupEntry {
    /// Event type discriminator: `"pruned"` | `"skipped"` | `"error"`.
    pub r#type: &'static str,
    /// Absolute path of the backup file (or target when skipped).
    pub path: String,
    /// Reason string: `"age"` | `"count"` | `"not_found"` | `"remove_failed"`.
    pub reason: String,
    /// Error message when `type == "error"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// Macros audit — typed NDJSON replaces ad-hoc `serde_json::json!` for stable
// agent contracts (prefer derive Serialize/JsonSchema over free-form macros).
// ============================================================================

/// NDJSON event when `search --target files` finds a path match by name/content.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct FileMatchEvent {
    /// Event type discriminator: `"file_match"`.
    pub r#type: &'static str,
    /// Absolute or display path of the matching file.
    pub path: String,
    /// File name component used for the match.
    pub name: String,
    /// Search target mode that produced this event (e.g. `"files"`).
    pub target: &'static str,
}

/// Offline inverted-index token record written by `semantic-search --index-dir`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SemanticIndexToken {
    /// Token text.
    pub t: String,
    /// Path of the source file.
    pub p: String,
    /// 1-based line number.
    pub l: u64,
    /// Truncated line snippet.
    pub s: String,
}

/// Single AST node hit from `query` (`--query`, `--tree`, or S-expression).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct QueryMatchEvent {
    /// Event type discriminator: `"query_match"`.
    pub r#type: &'static str,
    /// Source path.
    pub path: String,
    /// Detected or overridden language.
    pub language: String,
    /// tree-sitter node kind.
    pub kind: String,
    /// Whether the node is named in the grammar.
    pub is_named: bool,
    /// Truncated node text.
    pub text: String,
    /// S-expression capture name when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_name: Option<String>,
    /// Byte offset start (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_byte: Option<usize>,
    /// Byte offset end (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_byte: Option<usize>,
    /// 1-based start line (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    /// 1-based start column (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_column: Option<usize>,
    /// 1-based end line (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    /// 1-based end column (`--positions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
}

/// `sparse read` per-file head content.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SparseReadEvent {
    /// Event type discriminator: `"sparse_read"`.
    pub r#type: &'static str,
    /// File path.
    pub path: String,
    /// Head content joined with newlines.
    pub head: String,
    /// Number of head lines requested.
    pub lines: u64,
}

