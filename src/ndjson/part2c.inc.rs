// SPDX-License-Identifier: MIT OR Apache-2.0
// NDJSON event types, continued (hash, delete, count, backup, loop and sparse outputs).
// Included from part2.inc.rs.

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

/// NDJSON output for one checksum verification (`hash --verify` / `verify`).
///
/// One verification emits exactly one event of this type, and the verdict
/// (`ok` together with `verified`) travels inside that single object. Before
/// v0.1.36 a failing verification emitted a `HashOutput` carrying
/// `verified: false` and then a separate error envelope, so the same fact was
/// published twice with different shapes and the pair could disagree (B2).
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct HashVerifyOutput {
    /// Event type discriminator: `"hash_verify"`.
    pub r#type: &'static str,
    /// File path that was verified.
    pub path: String,
    /// Hash algorithm name.
    pub algorithm: &'static str,
    /// Checksum actually computed from the file contents.
    #[serde(rename = "checksum")]
    pub value: String,
    /// Caller-provided checksum the file was verified against.
    pub expected: String,
    /// Whether the computed checksum equals `expected`.
    pub verified: bool,
    /// Whether the verification succeeded. Always `true` on a match and
    /// always `false` on a mismatch, so the verdict never has to be inferred.
    pub ok: bool,
    /// Machine-readable outcome code, present only when `ok` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<&'static str>,
    /// Process exit status this verification produces, present only when
    /// `ok` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<u8>,
    /// File size in bytes.
    pub bytes: u64,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
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
    /// Sources the run skipped because they were not regular files.
    ///
    /// `files_backed_up` counts what was copied; this counts the origin that
    /// was named and deliberately left alone, so the two add up to the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_skipped: Option<u64>,
    /// Total bytes backed up.
    pub total_bytes: u64,
    /// Sources the `--limit` budget cut before the fan-out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_budget_dropped: Option<u64>,
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
    /// Present and true when `--limit` stopped the run before every source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walker_stopped_early: Option<bool>,
    /// Sources that could not be backed up; the run that copied at least one
    /// of them still answers, and this is the size of the hole.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_failed: Option<u64>,
    /// Present exactly when `files_failed` is, like every other terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<bool>,
    /// First failures, capped at `summary.errors_max_entries`; `files_failed`
    /// is the full count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FileFailure>>,
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
    /// Risk diagnostics of the content the preview would write. Omitted when
    /// nothing matched the configured patterns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_assessment: Option<WriteRiskAssessment>,
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
    /// Present and true when `--limit` cut the list before the run ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walker_stopped_early: Option<bool>,

    /// Backups that matched the criteria but could not be removed; the run
    /// answers with `total`, and this is what it could not do.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_failed: Option<u64>,
    /// Present exactly when `files_failed` is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<bool>,
    /// First removal failures, capped at `summary.errors_max_entries`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FileFailure>>,
    /// Backups the `--limit` budget cut before the fan-out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_budget_dropped: Option<u64>,
    /// Targets named on the command line that did not exist.
    ///
    /// `total` counts backups pruned; this counts subjects the run could not
    /// even look at, so a zero `total` with a missing target reads as the
    /// missing target instead of an empty tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets_missing: Option<u64>,
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
    /// Local diagnostic for a payload that looks destructive (B-013).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_assessment: Option<WriteRiskAssessment>,
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
    /// Terminal record the child run published for this step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_summary: Option<serde_json::Value>,
}

/// Per-rule aggregate inside `codemod_summary.by_rule_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CodemodRuleStats {
    /// AST matches produced by the rule, summed over its files.
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

