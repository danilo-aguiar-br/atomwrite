


/// NDJSON output for the `write` verb.
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
    #[schemars(extend("type" = "string"))]
    pub backup_path: Option<String>,
    /// Why the requested backup was skipped, when the target directory
    /// refused to accept a new entry. The write itself succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    pub backup_skipped: Option<String>,
    /// Write strategy the layer actually used: `rename`, `inplace` or
    /// `copyback`.
    ///
    /// `platform.rename_method` names the syscall (`renameat2`), so it cannot
    /// tell an in-place write from a swap. This field is the only place the
    /// verdict says which one ran, and an in-place write keeps the inode.
    pub write_strategy: &'static str,
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
#[derive(Debug, PartialEq, Default, Serialize, JsonSchema)]
pub struct Summary {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Total files examined.
    ///
    /// `search --target files` counts the files the walk handed over, and
    /// `copy` counts the sources it examined, so a copy that failed is counted
    /// here and in `files_failed` instead of disappearing.
    pub files_visited: u64,
    /// Files that contributed at least one published answer record.
    pub files_matched: u64,
    /// Files actually modified: rewritten by `replace`, `transform`, `case`,
    /// `scope` or `codemod`, deleted by `delete`, copied by `copy`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_modified: Option<u64>,
    /// Files a dry run would have modified.
    ///
    /// Present only on a dry run: `files_modified` stays at zero because no
    /// byte changed, and this is the blast radius the caller asked to see. It
    /// is never a claim that the write happened.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_planned: Option<u64>,
    /// Files the run did not answer: binary detection, size cap, no match,
    /// unchanged after a rewrite or an unreadable file. Which of these apply
    /// depends on the verb, and the verb's own section in `HOW_TO_USE` says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_skipped: Option<u64>,
    /// Answer records `search` paged out because `--offset` skipped them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_skipped: Option<u64>,
    /// Files where `search --max-count` cut at least one match.
    ///
    /// Each file counted here hid at least one more match, so the number is a
    /// floor on what the page left out, not an exact count of the remainder.
    /// The ceiling is on records, not on reading: to prove that a file held
    /// more, the search reads at most one match past the ceiling in that file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches_suppressed: Option<u64>,
    /// Occurrences `replace --max-replacements` left unchanged.
    ///
    /// The verb counts every occurrence of `old` before it applies the ceiling,
    /// so this is exact and the agent can size the second run from it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacements_suppressed: Option<u64>,
    /// Lines `semantic-search --min-score` discarded for scoring below it.
    ///
    /// A zero-match answer with a high floor is otherwise indistinguishable from
    /// a tree that has nothing to say about the query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub below_min_score: Option<u64>,
    /// Files opened and read, the stage between the walk handing a path over
    /// and the scoring that decides whether it answers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_read: Option<u64>,
    /// Items the `--limit` budget cut before the verb worked on them.
    ///
    /// The unit is the one the verb produces: files for a verb that walks
    /// paths, records for a verb that streams them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_budget_dropped: Option<u64>,
    /// Backups this run created and left on disk.
    ///
    /// A destructive verb that keeps a copy must say so: the caller decides
    /// whether the bytes are still there, and silence read as "removed".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backups_kept: Option<u64>,
    /// Bytes those backups occupy on disk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_bytes: Option<u64>,
    /// Directories the run could not remove because entries remained.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirs_left: Option<u64>,
    /// One sentence telling the caller what to change to finish the job.
    ///
    /// `dirs_left` counts the leftovers, and the count alone leaves the reader
    /// to guess why. The hint appears only when the run itself asked to skip
    /// entries, through `--include`, `--exclude`, `--older-than` or `--limit`,
    /// so it names the filter that kept the directory in place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Files the progress precount found eligible, published only when that
    /// precount ran (`--progress-every` on and quiet off); it is the total the
    /// run is working through, not the page it published.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_total: Option<u64>,
    /// Files refused because they hold binary content, a subset of
    /// `files_skipped` that lets the agent tell a binary refusal from a size cap
    /// or an unchanged rewrite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_rejects: Option<u64>,
    /// Answer records actually published, after `--limit` and `--offset`; for
    /// `transform` and `transform --rules` it is the replacements performed,
    /// and for `scope` it is the scopes matched across every file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_matches: Option<u64>,
    /// Total replacements performed across all files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_replacements: Option<u64>,
    /// Bytes the run left on disk or moved: the size after a rewrite, the size
    /// copied, or the size hashed, depending on the verb.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
    /// Present and true when the walker itself quit before the tree ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walker_stopped_early: Option<bool>,
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
    /// Risk diagnostics of the payload the plan would write. Omitted when
    /// nothing matched the configured patterns, so the field never appears
    /// as an empty object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_assessment: Option<WriteRiskAssessment>,
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
    /// Total bytes across all files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    /// Entries the `--limit` budget cut before the page was written.
    ///
    /// `files`, `dirs` and `total_bytes` describe the page actually published;
    /// this is how much of the tree that page left out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_budget_dropped: Option<u64>,
    /// File counts grouped by extension, present when --count-by-ext is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_extension: Option<std::collections::BTreeMap<String, u64>>,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
    /// Entries the walk could not stat; present only when some failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_failed: Option<u64>,
    /// First failures, capped at `summary.errors_max_entries` (50 by default);
    /// `files_failed` is the full count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FileFailure>>,
    /// The run answered from part of its targets, so the answer is partial.
    ///
    /// Present exactly when `files_failed` is present, so a consumer reads one
    /// flag instead of comparing two counters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<bool>,
    /// Present and true when the walker itself quit before the tree ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walker_stopped_early: Option<bool>,
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
    /// Write strategy the atomic layer used: `rename`, `inplace` or `copyback`.
    ///
    /// A read-only run and a matched-but-unchanged file report `none`, because
    /// neither takes a write path.
    pub write_strategy: &'static str,
    /// Why the requested backup was skipped, when the directory refused it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    pub backup_skipped: Option<String>,
    /// Risk diagnostics of the content the scoping wrote. Omitted on a
    /// read-only run and when nothing matched the configured patterns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_assessment: Option<WriteRiskAssessment>,
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
    /// 1-based line the first hunk landed on, when the format names one.
    ///
    /// `search-replace` and `full` report their own line, and `unified`
    /// reports the first hunk header. Omitted when the format cannot say.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// File size in bytes before patching.
    pub bytes_before: u64,
    /// File size in bytes after patching.
    pub bytes_after: u64,
    /// BLAKE3 checksum before patching.
    pub checksum_before: String,
    /// BLAKE3 checksum after patching.
    pub checksum_after: String,
    /// Write strategy the layer actually used: `rename`, `inplace` or
    /// `copyback`.
    ///
    /// `apply` rewrites a file that already exists, so a read-only directory
    /// makes it write in place and keep the inode. The caller cannot tell the
    /// two apart from the checksums, and this field is where it learns which
    /// one ran.
    pub write_strategy: &'static str,
    /// Backup file path, if a backup was created for this patch.
    ///
    /// The patch rewrote the file through the atomic writer, which is where
    /// the copy came from. A run without a backup omits the key, so a caller
    /// never reads a `null` it cannot tell from a missing copy.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    pub backup_path: Option<String>,
    /// Why the requested backup was skipped, when the target directory
    /// refused to accept a new entry. The patch itself succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    pub backup_skipped: Option<String>,
    /// Operation duration in milliseconds.
    pub elapsed_ms: u64,
    /// Changed lines with their 1-based numbers (`--show`).
    ///
    /// Present only when the caller asked: it is the evidence that removes the
    /// re-read the caller would otherwise pay to confirm the swap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed: Option<String>,
}

/// NDJSON event emitted when a failed `apply` put the files it had written back.
///
/// The record is the terminal line of the run, published through the writer's
/// failure path, so it carries `ok: false` and the very `code` and `exit` of
/// the failure it reports. It names no single target: the run restored more
/// than one file, and `failed_paths` is where the caller reads the ones that
/// did not come back.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ApplyRollbackEvent {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Verb that performed the rollback.
    pub operation: &'static str,
    /// Number of files whose content was restored to what it was before the run.
    pub files_restored: u64,
    /// Time the whole restoration took, in milliseconds.
    pub elapsed_ms: u64,
    /// Files the rollback could not restore, in reverse commit order.
    ///
    /// Omitted when every file came back, so an empty list never reads as a
    /// success the run cannot vouch for.
    // `default` keeps the field out of `required` in the published schema, so
    // the omission the binary makes and the contract it advertises agree.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_paths: Vec<String>,
    /// Copies the rollback left on disk, in commit order, the failed file last.
    ///
    /// A restoration rewrites content and never deletes a backup, so a run
    /// that committed with `--keep-backup` still has those copies, and the
    /// file whose write failed keeps the copy its own attempt had created.
    /// Omitted when the rollback left none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backups_preserved: Vec<String>,
}

/// NDJSON output for diff stat mode.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(inline)]
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
#[schemars(inline)]
pub struct DiffChangeOutput<'a> {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Change tag: insert or delete.
    pub tag: &'static str,
    /// 1-based line of the change: the old file for a delete, the new file for
    /// an insert.
    pub line: usize,
    /// 1-based line in the file before the change (`delete` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_line: Option<usize>,
    /// 1-based line in the file after the change (`insert` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line: Option<usize>,
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
    /// Write strategy the atomic engine used: `rename`, `inplace` or `copyback`.
    ///
    /// The file landed through the atomic writer, so the caller learns which
    /// mechanism carried it instead of inferring it from the inode.
    pub write_strategy: &'static str,
    /// Why a requested backup was skipped, when the directory refused it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    pub backup_skipped: Option<String>,
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
    /// Counts grouped by extension, after the `--top` cut.
    pub by_extension: std::collections::BTreeMap<String, ExtCountOutput>,
    /// Totals over every file counted, across all groups and before the cut.
    pub total: CountTotals,
    /// Groups the `--top` cut left out of `by_extension`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items_omitted: Option<u64>,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
    /// Files the walk saw but could not read; present only when some failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_failed: Option<u64>,
    /// First read failures, capped; `files_failed` is the full count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FileFailure>>,
    /// The run answered from part of its targets, so the answer is partial.
    ///
    /// Present exactly when `files_failed` is present, so a consumer reads one
    /// flag instead of comparing two counters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<bool>,
    /// Present and true when the walker itself quit before the tree ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walker_stopped_early: Option<bool>,
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
    /// Present and true when `text` was cut at `QUERY_NODE_TEXT_MAX_CHARS`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_truncated: Option<bool>,
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
