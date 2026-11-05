


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
    ///
    /// `default` keeps the field out of `required` in the published schema,
    /// because the event omits it when the match captured nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
///
/// Terminal record of an interrupted operation. Three fields answer the three
/// questions a consumer asks, and none of them requires reading the human text:
/// `deadline_exceeded` says whether the global `--timeout-secs` budget elapsed,
/// `signal` names the signal when one arrived, and `reason` carries the prose.
///
/// Build one with [`CancelledEvent::from_shutdown`] rather than by hand, so the
/// machine-readable verdict and the prose can never describe different causes.
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
    /// True when the global `--timeout-secs` deadline elapsed.
    ///
    /// Always serialized, never skipped: an omitted field cannot be told apart
    /// from a false one by a consumer, and the whole point of the field is that
    /// the deadline case is decidable without parsing `reason`.
    pub deadline_exceeded: bool,
}

impl CancelledEvent {
    /// Build the terminal cancel record from the live shutdown signal.
    ///
    /// Single construction point for every scanning verb, so the deadline
    /// verdict, the signal name and the prose all come from one observation of
    /// [`crate::signal::ShutdownSignal`] and cannot diverge.
    ///
    /// `path` is the target the verb was working on when it observed the
    /// cancel, when the verb tracks one.
    #[must_use]
    pub fn from_shutdown(
        shutdown: &crate::signal::ShutdownSignal,
        path: Option<String>,
    ) -> Self {
        Self {
            r#type: "cancelled",
            path,
            reason: shutdown.cancel_reason().to_string(),
            signal: shutdown.signal_name().map(str::to_string),
            deadline_exceeded: shutdown.is_timeout(),
        }
    }
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
    /// Write strategy the atomic layer used: `rename`, `inplace` or `copyback`.
    ///
    /// A dry run reports `none`, because the event describes the rewrite the
    /// run would make and no write path was taken.
    pub write_strategy: &'static str,
    /// Why the requested backup was skipped, when the directory refused it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    pub backup_skipped: Option<String>,
    /// Local diagnostic for a payload that looks destructive (B-013).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_assessment: Option<WriteRiskAssessment>,
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
    /// Write strategy the operation used, absent when it wrote no content.
    ///
    /// The value is `'static` so the parallel fan-out can carry it across
    /// threads without borrowing the manifest entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    pub write_strategy: Option<&'static str>,
    /// Why a requested backup was skipped, when the directory refused it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    pub backup_skipped: Option<String>,
    /// Copy this operation created, or the snapshot the transaction already held.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    pub backup_path: Option<String>,
    /// Advice the operation publishes because a flag changed nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
}

/// NDJSON output for unified diff format.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(inline)]
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

include!("part2c.inc.rs");
