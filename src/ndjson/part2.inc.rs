


/// Platform-specific fsync method names for diagnostics.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct PlatformInfo {
    /// File fsync method name (e.g. `F_FULLFSYNC` or `sync_data`).
    pub fsync: &'static str,
    /// Directory fsync method name (e.g. `sync_all` or `best_effort`).
    pub dir_fsync: &'static str,
    /// Durability mode resolved for this write (v0.1.29 P2-1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub durability: Option<&'static str>,
    /// Rename method used (v0.1.29 P2-2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rename_method: Option<&'static str>,
    /// Backup method when a backup was created: hardlink|reflink|copy (v0.1.29 P2-3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_method: Option<&'static str>,
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

/// Per-file match and line statistics for search operations.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct FileStats {
    /// Number of matches found in the file.
    pub matches: u64,
    /// Total lines examined in the file.
    pub lines_searched: u64,
}

/// NDJSON event for files-only search mode (path of matching file).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SearchFile {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Path of the file with at least one match.
    pub path: String,
}

/// Periodic progress heartbeat for long batch/replace jobs (v0.1.29 P1-3).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ProgressEvent {
    /// Event type discriminator (`"progress"`).
    pub r#type: &'static str,
    /// Items completed so far.
    pub done: u64,
    /// Total items known (0 if unknown).
    pub total: u64,
    /// Throughput estimate (items per second).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_per_s: Option<f64>,
    /// Estimated remaining milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_ms: Option<u64>,
    /// Phase label (e.g. `replace`, `batch`).
    pub phase: String,
}

/// Cooperative cancel envelope (v0.1.29 P0-4).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct CancelledEvent {
    /// Event type discriminator (`"cancelled"`).
    pub r#type: &'static str,
    /// Path being written when cancel was observed, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Human-readable cancel reason.
    pub reason: String,
    /// Signal name when known (`SIGINT`, `SIGTERM`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
}

/// Outcome of matching a single `--old`/`--new` pair in multi-pair edit mode.
///
/// Emitted in `pair_results` on both success and error envelopes so agents
/// can verify each pair objectively without re-running the edit (G117).
/// In the default all-or-nothing mode, pairs after the first failure are
/// never attempted and are absent from the array.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct PairResult {
    /// 1-based position of the pair in the `--old`/`--new` invocation order.
    pub index: u64,
    /// Whether the pair matched and was applied to the content.
    pub matched: bool,
    /// Matching strategy that succeeded (e.g. `exact`, `whitespace_normalized`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    /// Similarity score of the fuzzy match, 0.0-1.0 (block-anchor and context-aware only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f64>,
    /// Source of the match/replacement content: `"arg"` or `"file"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
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
    /// Computed hash value (GAP-107: renamed to `checksum` for schema consistency).
    #[serde(rename = "checksum")]
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
    /// Non-fatal warnings about backup flag combinations that had no
    /// effect (v0.1.28 GAP-CLI-SURFACE-DRIFT), e.g. `--keep-backup` is
    /// redundant for delete because deletion backups are always
    /// preserved. Empty when there is nothing to warn about.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
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

/// NDJSON output for text field extraction.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TextFieldsOutput<'a> {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Extracted fields.
    pub fields: Vec<&'a str>,
}

/// NDJSON final summary for `prune-backups`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct PruneBackupSummary {
    /// Event type discriminator: "summary".
    pub r#type: &'static str,
    /// Action label: `"pruned"` | `"dry_run"`.
    pub action: String,
    /// Total number of backups pruned (or matching criteria, in dry-run).
    pub total: usize,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON final summary for `edit-loop`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct EditLoopSummary {
    /// Event type discriminator: "result".
    pub r#type: &'static str,
    /// Action label: `"edit_loop"`.
    pub action: String,
    /// Target file path.
    pub path: String,
    /// Total number of pairs read from stdin.
    pub pairs_total: usize,
    /// Number of pairs that matched and were applied.
    pub pairs_applied: usize,
    /// Number of pairs that did not match.
    pub pairs_unmatched: usize,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
    /// Per-pair result array, in input order.
    pub pair_results: Vec<EditLoopPairResult>,
}

/// NDJSON ranked hit for `semantic-search`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SemanticMatchEvent {
    /// Event type discriminator: `"semantic_match"`.
    pub r#type: &'static str,
    /// 1-based rank in the result list.
    pub rank: usize,
    /// Jaccard similarity score in \[0.0, 1.0\].
    pub score: f64,
    /// Path of the matching file.
    pub path: String,
    /// 1-based line number of the snippet.
    pub line: u64,
    /// Truncated line content.
    pub snippet: String,
    /// Ranking backend label (`"jaccard"` or `"inverted-index"`).
    pub backend: &'static str,
}

/// Aggregate kind count from `query --kinds`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct QueryKindEvent {
    /// Event type discriminator: `"query_kind"`.
    pub r#type: &'static str,
    /// Source path.
    pub path: String,
    /// Detected or overridden language.
    pub language: String,
    /// tree-sitter node kind name.
    pub kind: String,
    /// Occurrences of this kind in the tree.
    pub count: usize,
}

/// Per-step progress line during `recipe run`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct RecipeStepEvent {
    /// Event type discriminator: `"recipe_step"`.
    pub r#type: &'static str,
    /// 1-based step number.
    pub step: u64,
    /// Step name.
    pub name: String,
    /// Status string.
    pub status: String,
    /// Detail or error message.
    pub detail: String,
    /// Optional checksum from the step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

/// Per-rule aggregate inside `codemod_summary.by_rule_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CodemodRuleStats {
    /// Match / transform event count attributed to the rule.
    pub matches: u64,
    /// Distinct files touched by the rule.
    pub files: u64,
}

/// Multi-rule transform: rule boundary marker.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct TransformRuleBegin {
    /// Event type discriminator: `"rule_begin"`.
    pub r#type: &'static str,
    /// Optional rule id from YAML (null when omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Language of the rule.
    pub language: String,
}

/// Budget diagnostics for `sparse outline`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SparseOutlineBudget {
    /// Event type discriminator: `"sparse_outline_budget"`.
    pub r#type: &'static str,
    /// Files visited under the budget.
    pub files_seen: u64,
    /// Outline items emitted.
    pub items: u64,
    /// Whether the walk stopped due to `max_files`.
    pub truncated: bool,
    /// Configured `max_files`.
    pub max_files: u64,
}

