// SPDX-License-Identifier: MIT OR Apache-2.0
// Per-file failure report shared by the scanning verbs' terminal summaries.
// Included from ndjson/mod.rs.

/// One file an operation could not process, as listed in a summary's `errors[]`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct FileFailure {
    /// Path of the file that failed.
    pub path: String,
    /// Machine error code of the failure (for example `PERMISSION_DENIED`).
    pub code: &'static str,
    /// Human-readable failure message.
    pub message: String,
}

/// The `partial` flag that travels with `files_failed`.
///
/// Present exactly when `files_failed` is present, so a consumer reads one
/// flag instead of comparing two counters. Both are derived from the same
/// `Option`, which is what keeps them from drifting apart.
#[must_use]
pub fn partial_of(files_failed: Option<u64>) -> Option<bool> {
    files_failed.map(|_| true)
}

/// [`Summary`] plus the per-file failure fields; both are omitted when no file failed.
///
/// Terminal event of `search`, `replace` and `transform`: a run that reached
/// every file reads exactly like before, and a run that could not read or write
/// some files says how many and why instead of staying silent.
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct FailureSummary {
    /// The counters every scanning verb reports.
    #[serde(flatten)]
    pub base: Summary,
    /// Files that matched or were visited but could not be processed, plus
    /// the root named in the invocation that does not exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_failed: Option<u64>,
    /// The run answered from part of its targets, so the answer is partial.
    ///
    /// Present exactly when `files_failed` is present, so a consumer reads one
    /// flag instead of comparing two counters. A run that reached every file
    /// keeps the shape it always had.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<bool>,
    /// First failures, capped at `summary.errors_max_entries` (50 by default);
    /// `files_failed` is the full count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FileFailure>>,
}

/// Failures of one run, collected while the files are scanned.
///
/// Keeps the full count, the first `summary.errors_max_entries` entries and the
/// first error itself, so a run that found nothing and failed somewhere exits
/// with that error instead of a misleading `NO_MATCHES`.
#[derive(Debug, Default)]
pub struct FailureTally {
    count: u64,
    items: Vec<FileFailure>,
    first: Option<anyhow::Error>,
}

impl FailureTally {
    /// Record that `path` could not be processed because of `err`.
    pub fn record(&mut self, path: &std::path::Path, err: anyhow::Error) {
        self.count = self.count.saturating_add(1);
        if self.items.len() < crate::config::summary_errors_max_entries() {
            self.items.push(FileFailure {
                path: path.display().to_string(),
                code: crate::output::failure_identity(&err).0,
                message: crate::output::error_message(&err),
            });
        }
        self.first.get_or_insert(err);
    }

    /// Record every root the invocation named that does not exist.
    ///
    /// A root the caller typed and the run never reached is a caller mistake,
    /// not a scan failure, so every scanning verb counts it the same way here
    /// instead of building its own tally.
    pub fn record_missing_roots(&mut self, missing: &[std::path::PathBuf]) {
        for path in missing {
            self.record(
                path,
                crate::error::AtomwriteError::NotFound { path: path.clone() }.into(),
            );
        }
    }

    /// Number of files recorded as failed.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Whether a recorded failure names `path`, an ancestor or a descendant.
    ///
    /// A directory that survives because something inside it failed is already
    /// explained by that failure, so the caller does not add a second verdict
    /// saying the removal left an unexplained entry. The list is capped at
    /// `summary.errors_max_entries`, so this answers from the published set.
    #[must_use]
    pub fn explains(&self, path: &std::path::Path) -> bool {
        self.items.iter().any(|item| {
            let failed = std::path::Path::new(&item.path);
            path.starts_with(failed) || failed.starts_with(path)
        })
    }

    /// Split the tally into the count, the capped entries and the first error.
    ///
    /// For a terminal event whose shape is its own (not [`Summary`]), which
    /// still has to report the same two fields as [`FailureSummary`].
    #[must_use]
    pub fn into_parts(self) -> (u64, Vec<FileFailure>, Option<anyhow::Error>) {
        (self.count, self.items, self.first)
    }

    /// Build the terminal summary and hand back the first failure, if any.
    #[must_use]
    pub fn into_summary(self, base: Summary) -> (FailureSummary, Option<anyhow::Error>) {
        let files_failed = (self.count > 0).then_some(self.count);
        let summary = FailureSummary {
            base,
            files_failed,
            partial: partial_of(files_failed),
            errors: (!self.items.is_empty()).then_some(self.items),
        };
        (summary, self.first)
    }
}

/// Every record `diff` can write, as one schema.
///
/// The default mode streams `change` records and ends with a `summary`,
/// `--unified` writes one `diff` record with the patch text, and `--stat`
/// writes one `diff` record with the counts. The variants are inlined, so the
/// served schema lists the real keys of each shape.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum DiffOutput<'a> {
    /// One changed line (default mode).
    Change(DiffChangeOutput<'a>),
    /// Terminal summary (default mode).
    Summary(DiffSummaryOutput),
    /// The patch as text (`--unified`).
    Unified(DiffUnifiedOutput),
    /// Insertion and deletion counts (`--stat`).
    Stat(DiffStatOutput),
}
