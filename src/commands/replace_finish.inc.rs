// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by replace_run.inc.rs (responsibility split, v0.1.37 G1).
//
// The consumer side of the run: the per-run tallies and the worker events,
// the loop that writes those events to the data channel, and closing the run
// (read the final counters, decide the verdict, emit the terminal record).
// Split from the scan so the part that reads totals and the part that walks
// the tree can be reasoned about and tested apart.

/// Tallies shared by every walker worker and read once after the join.
///
/// `Ordering::Relaxed`: independent tallies; the final loads run after the
/// parallel join (no data-publication barrier needed).
struct ReplaceCounters {
    files_visited: AtomicU64,
    files_modified: AtomicU64,
    files_skipped: AtomicU64,
    total_replacements: AtomicU64,
    /// Matches left alone because `--max-replacements` cut the file short.
    replacements_suppressed: AtomicU64,
    /// G-027: binary rejections, counted apart from generic I/O failures.
    binary_rejects: AtomicU64,
    first_binary_path: std::sync::Mutex<Option<PathBuf>>,
    /// Eligible-file count from the progress precount (0 when progress is off).
    files_total: AtomicU64,
    /// Set when a walker thread returns `WalkState::Quit`.
    ///
    /// The summary used to read `output::limit_reached()`, which records that
    /// the writer dropped a record. That is the writer's budget, not the walk:
    /// the same defect found in `search`, where a walk that stopped after 9 of
    /// 4000 files still published `false`.
    walk_quit: std::sync::atomic::AtomicBool,
}

impl ReplaceCounters {
    fn new(files_total: u64) -> Self {
        Self {
            files_visited: AtomicU64::new(0),
            files_modified: AtomicU64::new(0),
            files_skipped: AtomicU64::new(0),
            total_replacements: AtomicU64::new(0),
            replacements_suppressed: AtomicU64::new(0),
            binary_rejects: AtomicU64::new(0),
            first_binary_path: std::sync::Mutex::new(None),
            files_total: AtomicU64::new(files_total),
            walk_quit: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

/// Totals read once, after the parallel join, before the run is concluded.
struct ReplaceTotals {
    total_replacements: u64,
    /// Matches the runs never touched because the per-file limit was reached.
    replacements_suppressed: u64,
    files_matched: u64,
    files_visited: u64,
    files_skipped: u64,
    binary_rejects: u64,
    /// Eligible-file count from the progress precount, 0 when it did not run.
    files_total: u64,
    /// Files that could not be read, matched or written, for the summary.
    failures: crate::ndjson_types::FailureTally,
    /// Roots named in the invocation that do not exist on disk.
    missing_roots: Vec<std::path::PathBuf>,
    /// Whether a walker thread quit before the tree ended.
    walk_quit: bool,
}

fn finish_replace(
    totals: ReplaceTotals,
    first_binary_path: &std::sync::Mutex<Option<PathBuf>>,
    dry_run: bool,
    preview: bool,
    shutdown: &ShutdownSignal,
    writer: &mut NdjsonWriter<impl Write>,
    start: Instant,
) -> Result<()> {
    let total_repl = totals.total_replacements;
    let matched = totals.files_matched;
    // B-002g / R-DRY-001: dry-run / preview must not claim disk mutations.
    let modified_count =
        crate::commands::summary_metrics::files_modified_for_summary(matched, dry_run || preview);
    let planned_count =
        crate::commands::summary_metrics::files_planned_for_summary(matched, dry_run || preview);

    // v0.1.36 A3: a cancelled run emits NO summary. The counters below are
    // always partial — the walk stopped before covering its input — and any
    // partial summary is read by an agent as a completed conclusion. That was
    // the real defect: a run killed by the deadline reported
    // `total_replacements: 14035087` with no mention of the cancel, so a
    // consumer reading only the last NDJSON line saw an unqualified success.
    //
    // This matches the guard `search` (search.rs:378) and `delete`
    // (delete.rs:313) already apply. The terminal record is the `cancelled`
    // envelope instead, whose schema the project already publishes.
    //
    // Returning `Ok(())` also stops the `NO_MATCHES` path below, which used to
    // rewrite the cooperative cancel (124 / 130 / 143) into a misleading exit
    // 1 and make the unprocessed remainder look like an empty workspace.
    if shutdown.is_shutdown() {
        // The terminal record is no longer built here: `run` emits the cancel
        // envelope once for every scanning verb as the scan unwinds, so
        // returning `Ok(())` lets that single point decide. Building it here as
        // well would publish the cancel twice.
        return Ok(());
    }

    let base = Summary {
        r#type: "summary",
        files_visited: totals.files_visited,
        files_matched: matched,
        files_modified: modified_count,
        files_planned: planned_count,
        files_skipped: Some(totals.files_skipped),
        total_matches: None,
        total_replacements: Some(total_repl),
        replacements_suppressed: Some(totals.replacements_suppressed).filter(|n| *n > 0),
        binary_rejects: Some(totals.binary_rejects),
        files_total: Some(totals.files_total).filter(|n| *n > 0),
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        elapsed_ms: start.elapsed().as_millis() as u64,
        walker_stopped_early: totals.walk_quit.then_some(true),
        ..Summary::default()
    };
    // A file that could not be read, matched or written is reported by the
    // summary (`files_failed`, `errors[]`) instead of staying in a side event.
    let (summary, first_failure) = totals.failures.into_summary(base);

    // A root named in the invocation that does not exist outranks a rewrite
    // that landed: the caller asked about a place the run never reached.
    if let Some(first_missing) = totals.missing_roots.first() {
        let err = crate::error::AtomwriteError::NotFound {
            path: first_missing.clone(),
        };
        return Err(writer.publish_failure(&summary, err.into()));
    }

    // A replacement that landed is an answer the run did produce, and the
    // summary names every file it could not read or write in `files_failed`
    // and `errors[]`. Failing the run would throw that answer away.
    //
    // The count of planned replacements is not that answer: under
    // `--expect-checksum` a drift reports one replacement found and zero
    // files modified, and the earlier `total_repl > 0` arm published it as
    // `ok: true` with exit 0. Only a write that landed, or a planned one in
    // `--dry-run`/`--preview`, may close the run as a success.
    if total_repl > 0
        && (modified_count.is_some_and(|n| n > 0) || planned_count.is_some_and(|n| n > 0))
    {
        writer.write_terminal(&summary)?;
        return Ok(());
    }
    // R7: a run that ends in an error envelope must not publish `ok: true`, and
    // the summary is its one terminal event. A file that failed means "no
    // matches" would be false, so the first failure wins. G-027: all-binary /
    // binary-only skips must not look like NO_MATCHES + retryable.
    let binaries = totals.binary_rejects;
    let failure = if let Some(err) = first_failure {
        err
    } else if binaries > 0 && totals.files_matched == 0 {
        let path = first_binary_path
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_else(|| PathBuf::from("."));
        crate::error::AtomwriteError::BinaryFile { path }.into()
    } else {
        crate::error::AtomwriteError::NoMatches.into()
    };
    Err(writer.publish_failure(&summary, failure))
}

enum ReplaceEvent {
    Replaced {
        path: PathBuf,
        replacements: u64,
        bytes_before: u64,
        bytes_after: u64,
        checksum_before: String,
        checksum_after: String,
        elapsed_ms: u64,
        mtime_preserved: bool,
        fuzzy: Option<bool>,
        strategy: Option<String>,
        similarity: Option<f64>,
        strategies_tried: Option<u64>,
        word_ignored: Option<bool>,
        risk_assessment: Option<crate::ndjson_types::WriteRiskAssessment>,
    },
    Progress {
        done: u64,
        total: u64,
    },
    DryRun {
        path: PathBuf,
        replacements: u64,
        risk_assessment: Option<crate::ndjson_types::WriteRiskAssessment>,
    },
    Preview {
        path: PathBuf,
        replacements: u64,
        diff: String,
        risk_assessment: Option<crate::ndjson_types::WriteRiskAssessment>,
    },
    Error {
        path: PathBuf,
        kind: ReplaceErrorKind,
    },
}

enum ReplaceErrorKind {
    StateDrift {
        expected: String,
        actual: String,
    },
    /// A file the run could not read, match or write; the typed error travels
    /// whole so the summary reports its own code, class and exit status.
    Failed(anyhow::Error),
    /// G-027: binary content is a permanent precondition, not a transient write failure.
    BinaryRejected,
    JailViolation,
}

/// Write each worker event to the data channel, in arrival order.
///
/// Stops early, leaving the rest undrained, once the shutdown flag is set.
/// Returns the files that failed, for the terminal summary.
fn drain_replace_events(
    rx: &crossbeam_channel::Receiver<ReplaceEvent>,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    start: Instant,
) -> Result<crate::ndjson_types::FailureTally> {
    let mut failures = crate::ndjson_types::FailureTally::default();
    for event in rx {
        if shutdown.is_shutdown() {
            break;
        }

        match event {
            ReplaceEvent::Replaced {
                path,
                replacements,
                bytes_before,
                bytes_after,
                checksum_before,
                checksum_after,
                elapsed_ms,
                mtime_preserved,
                fuzzy,
                strategy,
                similarity,
                strategies_tried,
                word_ignored,
                risk_assessment,
            } => {
                let path_str = path.display().to_string();
                writer.write_event(&ReplaceResult {
                    r#type: "replaced",
                    path: path_str,
                    replacements,
                    bytes_before,
                    bytes_after,
                    checksum_before,
                    checksum_after,
                    elapsed_ms,
                    mtime_preserved: Some(mtime_preserved),
                    fuzzy,
                    strategy,
                    similarity,
                    strategies_tried,
                    word_ignored,
                    risk_assessment,
                })?;
            }
            ReplaceEvent::Progress { done, total } => {
                let elapsed = start.elapsed().as_secs_f64();
                let rate = if elapsed > 0.0 {
                    done as f64 / elapsed
                } else {
                    0.0
                };
                let eta_ms = if rate > 0.0 && total > done {
                    Some(
                        ((total - done) as f64 / rate * crate::constants::MILLIS_PER_SECOND_F64)
                            as u64,
                    )
                } else {
                    None
                };
                writer.write_event(&ProgressEvent {
                    r#type: "progress",
                    done,
                    total,
                    rate_per_s: Some(rate),
                    eta_ms,
                    phase: "replace".into(),
                })?;
            }
            ReplaceEvent::DryRun {
                path,
                replacements,
                risk_assessment,
            } => {
                writer.write_event(&DryRunPlan {
                    r#type: "plan",
                    operation: "replace".into(),
                    path: path.display().to_string(),
                    would_modify: true,
                    details: Some(format!("{replacements} replacements")),
                    risk_assessment,
                })?;
            }
            ReplaceEvent::Preview {
                path,
                replacements,
                diff,
                risk_assessment,
            } => {
                writer.write_event(&crate::ndjson_types::ReplacePreview {
                    r#type: "preview",
                    path: path.display().to_string(),
                    replacements,
                    diff,
                    risk_assessment,
                })?;
            }
            ReplaceEvent::Error { path, kind } => {
                // Binary and jail refusals are skips by design (`files_skipped`);
                // the other kinds are failures and join the summary's tally.
                let (message, error_class, retryable, failure) = match kind {
                    ReplaceErrorKind::StateDrift { expected, actual } => (
                        format!("state drift: expected {expected}, got {actual}"),
                        crate::error::ErrorClass::Conflict.as_str(),
                        true,
                        Some(anyhow::Error::from(
                            crate::error::AtomwriteError::StateDrift {
                                path: path.clone(),
                                expected,
                                actual,
                            },
                        )),
                    ),
                    ReplaceErrorKind::Failed(err) => {
                        let (class, retryable) = crate::output::failure_class(&err);
                        (
                            crate::output::error_message(&err),
                            class,
                            retryable,
                            Some(err),
                        )
                    }
                    ReplaceErrorKind::BinaryRejected => (
                        "binary file rejected for text replace (G-015/G-027)".to_string(),
                        crate::error::ErrorClass::Permanent.as_str(),
                        false,
                        None,
                    ),
                    ReplaceErrorKind::JailViolation => (
                        "path escapes workspace jail; use --workspace to set a different root"
                            .to_string(),
                        crate::error::ErrorClass::Permanent.as_str(),
                        false,
                        None,
                    ),
                };
                writer.write_event(&crate::ndjson_types::ReplaceErrorEvent {
                    status: "error",
                    path: path.display().to_string(),
                    message,
                    error_class,
                    retryable,
                })?;
                if let Some(failure) = failure {
                    failures.record(&path, failure);
                }
            }
        }
    }

    Ok(failures)
}
