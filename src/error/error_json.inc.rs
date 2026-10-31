/// Serializable error envelope emitted as a single NDJSON line.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ErrorJson {
    /// Always true, marks this line as an error event.
    pub error: bool,
    /// Always false on error envelopes (GAP-2026-020 agent sentinel).
    ///
    /// Success envelopes keep their own `status` field; agents MUST check
    /// `error` / `ok` / process exit before reading success-only keys.
    pub ok: bool,
    /// Machine-readable error code string.
    pub code: &'static str,
    /// Suggested process exit code.
    pub exit: u8,
    /// Human-readable error message.
    pub message: String,
    /// Filesystem path related to the error, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Error class: transient, conflict, `precondition_failed`, or permanent.
    pub error_class: &'static str,
    /// Whether a retry may resolve this error.
    pub retryable: bool,
    /// Optional actionable suggestion for the caller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Workspace root used for jail validation, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    /// 1-based index of the failed `--old`/`--new` pair (multi-pair edit, G117).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_pair_index: Option<u64>,
    /// Total number of `--old`/`--new` pairs in the invocation.
    ///
    /// Emitted by multi-pair edit (G117) and by a partial `edit-loop` run
    /// (v0.1.36) — one key, two producers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairs_total: Option<u64>,
    /// Per-pair diagnostics up to and including the failed pair (multi-pair edit, G117).
    /// Pairs after the failure were never attempted and are absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pair_results: Option<Vec<PairResult>>,
    /// Closest near-miss when a match cascade fails (v0.1.29 P0-2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_candidate: Option<BestCandidate>,
    /// Additional near-miss candidates for `did_you_mean` (v0.1.30).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<BestCandidate>>,
    /// Match count when ambiguity is reported (v0.1.30).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_count: Option<u64>,
    /// Similar basenames when path is missing (v0.1.30).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similar_paths: Option<Vec<String>>,
    /// Number of conflicting regions a three-way merge could not align (v0.1.36).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_count: Option<u64>,
    /// Flag literal the operation required but did not receive (v0.1.36).
    ///
    /// Present only for [`AtomwriteError::MissingFlag`]; the duplicate `code`
    /// on that variant is deliberate — callers branch on `missing_flag` and
    /// still see the `INVALID_INPUT` family in `code`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_flag: Option<String>,
    /// Pairs that did not match in a partial `edit-loop` run (v0.1.36).
    ///
    /// `pairs_total` already exists above for the multi-pair edit family and
    /// is reused by this variant instead of being duplicated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairs_unmatched: Option<u64>,
    /// Pairs that were written to the target before the error (v0.1.37).
    ///
    /// Always `0` for the all-or-nothing `EditPairFailed` abort (nothing is
    /// written) and `total - unmatched` for `edit-loop --partial`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairs_applied: Option<u64>,
    /// UI language the catalogue-derived `suggestion` was rendered in (B4).
    ///
    /// The two human strings on this envelope do not share a source: `message`
    /// is built from the error variant, which is English by contract, while
    /// `suggestion` comes from the i18n catalogue and follows the resolved UI
    /// language. One envelope can therefore carry an English `message` and a
    /// Portuguese `suggestion`. Without this field a consumer cannot tell which
    /// language the suggestion is in, and cannot decide whether to re-render
    /// it. `code` and every other field stay machine literals.
    pub suggestion_lang: &'static str,
}

impl ErrorJson {
    /// Build an [`ErrorJson`] from a domain error with default empty context.
    ///
    /// Equivalent to `from_error_with_context(err, &ErrorContext::default())`.
    /// Use [`Self::from_error_with_context`] when workspace provenance is known
    /// so the suggestion for `WorkspaceJail` is precise.
    #[cold]
    #[track_caller]
    pub fn from_error(err: &AtomwriteError) -> Self {
        Self::from_error_with_context(err, &ErrorContext::default())
    }

    /// Build an [`ErrorJson`] from a domain error and a diagnostic context.
    ///
    /// The context allows the suggestion text to be precise. In particular,
    /// `WorkspaceJail` errors report different remediation depending on
    /// whether the user already supplied a workspace root via `--workspace`
    /// or CLI `--workspace` (GAP 13 fix).
    #[cold]
    #[track_caller]
    pub fn from_error_with_context(err: &AtomwriteError, ctx: &ErrorContext) -> Self {
        let workspace = match err {
            AtomwriteError::WorkspaceJail { workspace, .. } => {
                Some(workspace.display().to_string())
            }
            _ => None,
        };
        // Cold error path: clone out of `Box` / `Option` fields for the
        // public JSON surface. Source remains borrowed (`&AtomwriteError`).
        let (failed_pair_index, pairs_total, pair_results) = match err {
            AtomwriteError::EditPairFailed {
                index,
                total,
                pair_results,
                ..
            } => (
                Some(*index),
                Some(*total),
                Some(pair_results.as_ref().clone()),
            ),
            AtomwriteError::EditLoopPartial { total, .. } => (None, Some(*total), None),
            _ => (None, None, None),
        };
        let conflict_count = match err {
            AtomwriteError::MergeConflict { count, .. } => Some(*count),
            _ => None,
        };
        let missing_flag = match err {
            AtomwriteError::MissingFlag { flag, .. } => Some(flag.clone()),
            _ => None,
        };
        let pairs_unmatched = match err {
            AtomwriteError::EditLoopPartial { unmatched, .. } => Some(*unmatched),
            _ => None,
        };
        let pairs_applied = match err {
            AtomwriteError::EditPairFailed { .. } => Some(0),
            AtomwriteError::EditLoopPartial { unmatched, total } => {
                Some(total.saturating_sub(*unmatched))
            }
            _ => None,
        };
        let best_candidate = match err {
            AtomwriteError::MatchFailed { best_candidate, .. }
            | AtomwriteError::MatchAmbiguous { best_candidate, .. }
            | AtomwriteError::EditPairFailed { best_candidate, .. } => {
                best_candidate.as_deref().cloned()
            }
            _ => None,
        };
        let candidates = match err {
            AtomwriteError::MatchFailed { candidates, .. } => candidates.clone(),
            _ => None,
        };
        let match_count = match err {
            AtomwriteError::MatchAmbiguous { count, .. } => Some(*count),
            // GAP-2026-020: stable sentinels so agents never see "all null" success fields.
            AtomwriteError::MatchFailed { .. } | AtomwriteError::NoMatches => Some(0),
            _ => None,
        };
        Self {
            error: true,
            ok: false,
            code: err.error_code(),
            exit: err.exit_code(),
            message: err.to_string(),
            path: err.path().map(|p| p.display().to_string()),
            error_class: err.error_class().as_str(),
            retryable: err.is_retryable(),
            suggestion: suggestion_for(err, ctx),
            workspace,
            failed_pair_index,
            pairs_total,
            pair_results,
            best_candidate,
            candidates,
            match_count,
            similar_paths: similar_paths_for(err, ctx),
            conflict_count,
            missing_flag,
            pairs_unmatched,
            pairs_applied,
            suggestion_lang: crate::locale::resolved_language().as_str(),
        }
    }
}
