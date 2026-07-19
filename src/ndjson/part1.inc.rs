


/// GAP-2026-011 L1/L6: Local diagnostics describing why a write was classified
/// as risky (G-031: not product telemetry). Emitted in `WriteOutput.risk_assessment`
/// when the size delta exceeds the user-configured `--risk-threshold`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct WriteRiskAssessment {
    /// File size before the write.
    pub original_bytes: u64,
    /// New file size after the write.
    pub new_bytes: u64,
    /// |new - original| / original * 100.
    pub size_delta_pct: u32,
    /// "low" (<70%) | "medium" (70-89%) | "high" (>=90%).
    pub risk_level: &'static str,
    /// Which guard triggered the assessment (always "size" for L1).
    pub guard_triggered: &'static str,
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
    /// Number of lines in the returned content.
    ///
    /// As of v0.1.20 (GAP-2026-008), this is the FILTERED count when a partial
    /// read filter (`--head`, `--tail`, `--line`, `--lines`, `--grep`) is
    /// in effect. The original file total is preserved in `lines_total`.
    pub lines: u64,
    /// Total lines in the original file before any filter (GAP-2026-008).
    /// Omitted when no filter is applied or the filter did not change the
    /// line count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_total: Option<u64>,
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
    /// Read mode discriminator (GAP-2026-009): "full" | "head" | "tail" |
    /// "line" | "lines" | "grep" | "stat" | "raw". Allows downstream consumers to
    /// distinguish partial reads from full reads without parsing content.
    pub mode: String,
    /// Base64 payload for binary `--format raw` under NDJSON agent contract (G-013).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_b64: Option<String>,
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
    /// Whether any replacement used a non-exact fuzzy strategy (v0.1.29).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuzzy: Option<bool>,
    /// Last fuzzy strategy that applied (v0.1.29).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    /// Similarity of the last fuzzy match when applicable (v0.1.29).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f64>,
    /// Strategies tried for the last fuzzy application (v0.1.29).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategies_tried: Option<u64>,
    /// True when `--word` was ignored because fuzzy path was used (v0.1.29).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_ignored: Option<bool>,
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
    /// Unified diff preview showing what the fuzzy match changed (truncated to 500 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_preview: Option<String>,
    /// Total number of `--old`/`--new` pairs requested (multi-pair mode only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairs_total: Option<u64>,
    /// Per-pair match results (multi-pair mode only) — the per-item ground truth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pair_results: Option<Vec<PairResult>>,
    /// Whether the original modification time was preserved (true) or updated to now (false).
    /// Critical for build systems: false ensures cargo/make/cmake detect the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime_preserved: Option<bool>,
    /// Number of occurrences replaced in old/new mode (1 unless `--replace-all`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_count: Option<u64>,
    /// True when fuzzy indent delta realigned the replacement text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent_adjusted: Option<bool>,
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
    /// Path to backup file, if created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
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

/// NDJSON output for count sorted by file size (top-N).
/// Added in v0.1.20 to close GAP-2026-001 (HELP-FIRST-DRIFT).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct CountBySizeOutput {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Count mode name.
    pub mode: &'static str,
    /// Top-N files by descending size.
    pub items: Vec<SizeEntry>,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
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

/// NDJSON output for text value extraction.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TextValuesOutput<'a> {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Extracted values.
    pub values: Vec<&'a str>,
}

// ============================================================================
// G119 L5 — Re-export of WAL stats for `--json-schema wal-stats` consumers
// ============================================================================

pub use crate::wal::{AutoHealReport, WalDirEntry, WalStateBreakdown, WalStats};

// ============================================================================
// v0.1.22 — edit-loop NDJSON output types (ADR-0039)
// ============================================================================

/// Per-pair result emitted in the `pair_results` array of `edit-loop`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct EditLoopPairResult {
    /// 1-based index of the pair in NDJSON order.
    pub index: usize,
    /// Whether the pair matched and was applied.
    pub matched: bool,
    /// The search text for this pair.
    pub old: String,
    /// The replacement text for this pair.
    pub new: String,
}

/// NDJSON event for `watch` (feature-gated command).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct WatchEvent {
    /// Event type discriminator: `"watch"`.
    pub r#type: &'static str,
    /// Path that changed.
    pub path: String,
    /// Notify event kind (debug format of the backend kind).
    pub kind: String,
    /// Optional BLAKE3 hex when `--checksum` is set and path is a regular file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

/// Terminal NDJSON summary for `watch` (A-WATCH-001 — always emitted on exit).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct WatchSummary {
    /// Event type discriminator: `"watch_summary"`.
    pub r#type: &'static str,
    /// Number of filesystem change events emitted.
    pub events: u64,
    /// Exit reason: `idle`, `max_events`, `signal`, or `complete`.
    pub reason: String,
    /// Idle-exit bound in milliseconds (0 = disabled).
    pub idle_exit_ms: u64,
    /// Debounce window in milliseconds.
    pub debounce_ms: u64,
    /// Configured max events (0 = unlimited until other bound).
    pub max_events: u64,
    /// Wall time of the watch session in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON final summary for `semantic-search`.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct SemanticSummaryEvent {
    /// Event type discriminator: `"semantic_summary"`.
    pub r#type: &'static str,
    /// Original free-text query.
    pub query: String,
    /// Requested top-k.
    pub k: u64,
    /// Number of results emitted.
    pub results: usize,
    /// Ranking backend label.
    pub backend: &'static str,
}

/// Built-in recipe listing row (`recipe list`).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct RecipeListEvent {
    /// Event type discriminator: `"recipe"`.
    pub r#type: &'static str,
    /// Recipe name.
    pub name: &'static str,
    /// Always true for built-ins.
    pub builtin: bool,
}

/// `codemod` campaign start envelope.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct CodemodStartEvent {
    /// Event type discriminator: `"codemod"`.
    pub r#type: &'static str,
    /// Phase label (`"start"`).
    pub phase: &'static str,
    /// Rules file path.
    pub rules: String,
    /// Parsed rule ids from the manifest.
    pub rule_ids: Vec<String>,
    /// Campaign id (rules file stem).
    pub rule_id: String,
    /// Whether mutations are suppressed.
    pub dry_run: bool,
}

/// Final `codemod` summary.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct CodemodSummaryEvent {
    /// Event type discriminator: `"codemod_summary"`.
    pub r#type: &'static str,
    /// Rules file path.
    pub rules: String,
    /// Campaign id.
    pub rule_id: String,
    /// Dry-run flag.
    pub dry_run: bool,
    /// Stats keyed by rule id.
    pub by_rule_id: std::collections::BTreeMap<String, CodemodRuleStats>,
}

/// Multi-rule transform: rule failure (partial success continues).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct TransformRuleError {
    /// Event type discriminator: `"rule_error"`.
    pub r#type: &'static str,
    /// Optional rule id from YAML.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Error message.
    pub error: String,
}

