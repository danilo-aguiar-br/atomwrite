// SPDX-License-Identifier: MIT OR Apache-2.0
// Execution and verdict, included by `run.inc.rs`.

/// The batch-wide options one operation needs, copied out of the run.
///
/// A struct rather than thirteen parameters: the fan-out closure receives it by
/// value, so it never borrows the writer.
#[derive(Clone, Copy)]
pub(crate) struct BatchOpOptions<'a> {
    workspace: &'a std::path::Path,
    global: &'a GlobalArgs,
    dry_run: bool,
    keep_backup: bool,
    no_backup: bool,
    backup_explicit: bool,
    retention: u8,
    strict_atomic: bool,
    fuzzy_mode: crate::cli::FuzzyMode,
    fuzzy_threshold: Option<f64>,
    fuzzy_cfg: &'a crate::config::FuzzySection,
    /// Copies the transaction already made, keyed by the path they preserve.
    ///
    /// Borrowed rather than owned, so a fan-out closure stays `Copy` and no
    /// path is cloned per operation.
    snapshots: &'a [(PathBuf, PathBuf)],
}

/// Run one operation with the batch-wide options.
#[allow(clippy::too_many_arguments)]
fn execute_op_with(
    op: &BatchOp,
    idx: usize,
    opts: &BatchOpOptions<'_>,
) -> Result<BatchOpOutcome> {
    execute_op(
        op,
        idx,
        opts.workspace,
        opts.global,
        opts.dry_run,
        opts.keep_backup,
        opts.no_backup,
        opts.backup_explicit,
        opts.retention,
        opts.strict_atomic,
        opts.fuzzy_mode,
        opts.fuzzy_threshold,
        opts.fuzzy_cfg,
        opts.snapshots,
    )
}

/// Snapshot one file before the transaction touches it.
///
/// A free function, not a method: the fan-out closure cannot borrow the run
/// (the writer is not `Sync`), and this needs nothing but the retention bound.
fn snapshot_file(
    path: &std::path::Path,
    retention: u8,
) -> Result<Option<(PathBuf, PathBuf)>> {
    if crate::commands::backup_names::is_backup_path(path) {
        return Ok(None);
    }
    let backup = crate::atomic::create_backup(path, retention)
        .with_context(|| format!("transaction pre-backup failed for {}", path.display()))?;
    Ok(Some((path.to_path_buf(), backup)))
}

/// Remove the pre-transaction snapshots once they are no longer needed.
///
/// The snapshots exist so a failed run can be undone. Once every operation
/// committed, the copy has served its purpose and follows the same rule as
/// any other backup: it is removed unless the run asked to keep it, and
/// `--no-backup` never leaves one behind. A failed reversal keeps its copies
/// on purpose, because they are the last known state of those files.
fn discard_snapshots(backups: &[(PathBuf, PathBuf)]) {
    for (_, backup) in backups {
        crate::atomic::delete_backup_quietly(backup);
    }
}

/// Remove the snapshots that became residue, keeping the copies that are the
/// way back.
///
/// A file the run deleted, moved or overwrote has no other way back, and a
/// rewrite keeps its snapshot only when `--keep-backup` asked for it. The
/// caller decides which paths belong to that set and when a copy is forbidden.
fn discard_residue(backups: &[(PathBuf, PathBuf)], destructive: &HashSet<PathBuf>) {
    for (original, backup) in backups {
        if destructive.contains(original) {
            continue;
        }
        crate::atomic::delete_backup_quietly(backup);
    }
}

/// One batch invocation: the parsed manifest plus the mutable state a run keeps.
pub(crate) struct BatchRun<'a, W: Write> {
    pub(crate) ops: &'a [BatchOp],
    pub(crate) workspace: &'a std::path::Path,
    pub(crate) global: &'a GlobalArgs,
    pub(crate) dry_run: bool,
    pub(crate) transaction: bool,
    /// Wire token of where the manifest came from (`argv` or `stdin`), so the
    /// rollback record can name the origin of the targets it reverted.
    pub(crate) target_source: &'static str,
    pub(crate) keep_backup: bool,
    pub(crate) no_backup: bool,
    pub(crate) backup_explicit: bool,
    pub(crate) retention: u8,
    pub(crate) strict_atomic: bool,
    pub(crate) fuzzy_mode: crate::cli::FuzzyMode,
    pub(crate) fuzzy_threshold: Option<f64>,
    pub(crate) fuzzy_cfg: &'a crate::config::FuzzySection,
    pub(crate) shutdown: &'a ShutdownSignal,
    pub(crate) writer: &'a mut NdjsonWriter<W>,
    pub(crate) start: Instant,
    /// Operations that completed, as counted by [`Self::execute`].
    pub(crate) succeeded: u64,
    /// Operations that failed, as counted by [`Self::execute`].
    pub(crate) failed: u64,
}

impl<W: Write> BatchRun<'_, W> {
    /// Run every operation, fanning out when the batch allows it.
    ///
    /// A non-transactional batch with unique targets fans out through
    /// `rayon::par_iter`; a transactional batch, or one with overlapping
    /// targets, stays sequential so rollback order and per-file ordering hold.
    fn execute(&mut self) -> Result<Vec<String>> {
        // A transaction holds every target lock from before the snapshot until
        // the rollback or commit ends this call. Paths are sorted so two
        // concurrent batches with the same targets always wait in the same
        // order and can never deadlock. Targets whose parent directory does
        // not exist are skipped: creating them is the operation's own failure,
        // not a reason to fail the whole batch earlier.
        let _batch_locks: Vec<crate::lock::LockGuard> = if self.transaction && !self.dry_run {
            let mut paths = collect_target_paths(self.ops, self.workspace);
            paths.sort();
            paths.dedup();
            paths
                .iter()
                .filter(|path| path.parent().is_some_and(std::path::Path::is_dir))
                .map(|path| crate::lock::acquire_for_update(path))
                .collect::<Result<Vec<_>>>()?
        } else {
            Vec::new()
        };

        // Transaction mode snapshots every existing file before any mutation.
        // Paths are unique (`collect_target_paths` dedups); independent I/O →
        // `par_iter`. Ops and rollback stay sequential.
        let backups: Vec<(PathBuf, PathBuf)> = if self.transaction && !self.dry_run {
            let paths = collect_snapshot_paths(self.ops, self.workspace);
            // The closure captures only the retention bound, never `self`: the
            // writer is not `Sync`, and the snapshot needs no access to it.
            let retention = self.retention;
            if should_parallelize(paths.len()) {
                paths
                    .par_iter()
                    .map(|path| snapshot_file(path, retention))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect()
            } else {
                let mut pairs = Vec::with_capacity(paths.len());
                for path in paths {
                    if let Some(pair) = snapshot_file(&path, retention)? {
                        pairs.push(pair);
                    }
                }
                pairs
            }
        } else {
            Vec::new()
        };

        // Files created during the transaction, removed on rollback.
        let mut created_files: Vec<PathBuf> = Vec::new();
        // Moves to reverse (target → source) on rollback.
        let mut moves_to_reverse: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut succeeded: u64 = 0;
        let mut failed: u64 = 0;
        let total_ops = self.ops.len() as u64;
        // A-018: adaptive progress cadence from named constants (not literals).
        let progress_every = if total_ops == 0 {
            0
        } else {
            (total_ops / crate::constants::BATCH_PROGRESS_DIVISOR).clamp(
                crate::constants::BATCH_PROGRESS_MIN,
                crate::constants::BATCH_PROGRESS_MAX,
            )
        };

        let parallel_ok =
            !self.transaction && should_parallelize(self.ops.len()) && batch_targets_unique(self.ops);

        if parallel_ok {
            tracing::debug!(ops = self.ops.len(), "batch: parallel non-transactional fan-out");
            // Every value the closure needs is copied out first: it cannot
            // borrow the run, whose writer is not `Sync`.
            let opts = self.op_options(&backups);
            let shutdown = self.shutdown;
            let results: Vec<(usize, Result<BatchOpOutcome, String>, u64)> = self
                .ops
                .par_iter()
                .enumerate()
                .map(|(idx, op)| {
                    if shutdown.is_shutdown() {
                        return (idx, Err("batch cancelled by signal".to_string()), 0);
                    }
                    let op_start = Instant::now();
                    let result = execute_op_with(op, idx, &opts).map_err(|e| format!("{e:#}"));
                    (idx, result, op_start.elapsed().as_millis() as u64)
                })
                .collect();

            let mut ordered = results;
            ordered.sort_by_key(|(idx, _, _)| *idx);
            for (idx, result, elapsed_ms) in ordered {
                let op = &self.ops[idx];
                match result {
                    Ok(outcome) => {
                        succeeded += 1;
                        self.writer.write_event(&BatchOpResult {
                            r#type: "batch_op",
                            index: idx as u64,
                            op: &op.op,
                            status: "ok",
                            details: Some(outcome.details),
                            error: None,
                            write_strategy: Some(outcome.write_strategy),
                            backup_skipped: outcome.backup_skipped,
                            backup_path: outcome.backup_path,
                            warnings: outcome.warnings,
                            elapsed_ms,
                        })?;
                    }
                    Err(e) => {
                        failed += 1;
                        self.writer.write_event(&BatchOpResult {
                            r#type: "batch_op",
                            index: idx as u64,
                            op: &op.op,
                            status: "failed",
                            details: None,
                            error: Some(e),
                            write_strategy: None,
                            backup_skipped: None,
                            backup_path: None,
                            warnings: Vec::new(),
                            elapsed_ms,
                        })?;
                    }
                }
            }
        } else {
            for idx in 0..self.ops.len() {
                if self.shutdown.is_shutdown() {
                    tracing::info!(
                        completed = idx,
                        total = self.ops.len(),
                        "batch interrupted by signal"
                    );
                    break;
                }
                self.report_progress(idx, total_ops, progress_every)?;

                let op = &self.ops[idx];
                let op_start = Instant::now();
                // Pre-snapshot existence so the run knows if the op created a
                // new file at its target, which rollback must remove.
                let creates_target = matches!(op.op.as_str(), "write" | "move" | "copy");
                let was_new_file = if self.transaction && !self.dry_run && creates_target {
                    op.resolve_file_path()
                        .ok()
                        .map(std::path::Path::new)
                        .and_then(|p| crate::path_safety::validate_path(p, self.workspace).ok())
                        .map(|p| !p.exists())
                        .unwrap_or(false)
                } else {
                    false
                };

                match self.run_op(op, idx, &backups) {
                    Ok(outcome) => {
                        succeeded += 1;
                        self.track_created(op, was_new_file, &mut created_files);
                        self.track_move(op, &mut moves_to_reverse);
                        self.writer.write_event(&BatchOpResult {
                            r#type: "batch_op",
                            index: idx as u64,
                            op: &op.op,
                            status: "ok",
                            details: Some(outcome.details),
                            error: None,
                            write_strategy: Some(outcome.write_strategy),
                            backup_skipped: outcome.backup_skipped,
                            backup_path: outcome.backup_path,
                            warnings: outcome.warnings,
                            elapsed_ms: op_start.elapsed().as_millis() as u64,
                        })?;
                    }
                    Err(e) => {
                        failed += 1;
                        self.writer.write_event(&BatchOpResult {
                            r#type: "batch_op",
                            index: idx as u64,
                            op: &op.op,
                            status: "failed",
                            details: None,
                            error: Some(format!("{e:#}")),
                            write_strategy: None,
                            backup_skipped: None,
                            backup_path: None,
                            warnings: Vec::new(),
                            elapsed_ms: op_start.elapsed().as_millis() as u64,
                        })?;
                        if self.transaction {
                            self.rollback(&backups, &created_files, &moves_to_reverse, idx, &e)?;
                        }
                    }
                }
            }
        }

        // Every operation committed, so the pre-transaction snapshots are no
        // longer needed, except the one that undoes a deletion: that file lives
        // nowhere else. `--no-backup` still wins, because the invocation
        // forbade any copy.
        if self.no_backup {
            discard_snapshots(&backups);
        } else {
            let destructive =
                destructive_paths(self.ops, self.workspace, self.keep_backup, self.no_backup);
            discard_residue(&backups, &destructive);
        }

        // Whatever survived the commit is the only copy left of a file the run
        // removed or replaced, so the summary names it.
        let backups_preserved = backups
            .iter()
            .filter(|(_, backup)| backup.exists())
            .map(|(_, backup)| backup.display().to_string())
            .collect();

        self.succeeded = succeeded;
        self.failed = failed;
        Ok(backups_preserved)
    }

    /// Execute one operation with the batch-wide options.
    fn run_op(
        &self,
        op: &BatchOp,
        idx: usize,
        snapshots: &[(PathBuf, PathBuf)],
    ) -> Result<BatchOpOutcome> {
        execute_op_with(op, idx, &self.op_options(snapshots))
    }

    /// The batch-wide execution options, cheap to copy into a fan-out closure.
    ///
    /// `snapshots` is the list this run already holds, empty outside a
    /// transaction, so an operation can name the copy instead of writing one.
    fn op_options<'s>(&'s self, snapshots: &'s [(PathBuf, PathBuf)]) -> BatchOpOptions<'s> {
        BatchOpOptions {
            workspace: self.workspace,
            global: self.global,
            dry_run: self.dry_run,
            keep_backup: self.keep_backup,
            no_backup: self.no_backup,
            backup_explicit: self.backup_explicit,
            retention: self.retention,
            strict_atomic: self.strict_atomic,
            fuzzy_mode: self.fuzzy_mode,
            fuzzy_threshold: self.fuzzy_threshold,
            fuzzy_cfg: self.fuzzy_cfg,
            snapshots,
        }
    }

    /// Emit the progress record every `progress_every` operations.
    fn report_progress(&mut self, idx: usize, total_ops: u64, progress_every: u64) -> Result<()> {
        if progress_every == 0
            || idx == 0
            || !(idx as u64).is_multiple_of(progress_every)
            || self.global.quiet != 0
            || self.global.no_progress
        {
            return Ok(());
        }
        let done = idx as u64;
        let elapsed = self
            .start
            .elapsed()
            .as_secs_f64()
            .max(crate::constants::BATCH_RATE_MIN_ELAPSED_SECS);
        let rate = done as f64 / elapsed;
        let remaining = total_ops.saturating_sub(done);
        let eta_ms = if rate > 0.0 {
            Some(
                ((remaining as f64 / rate) * crate::constants::MILLIS_PER_SECOND_F64) as u64,
            )
        } else {
            None
        };
        self.writer.write_event(&ProgressEvent {
            r#type: "progress",
            done,
            total: total_ops,
            rate_per_s: Some(rate),
            eta_ms,
            phase: "batch".into(),
        })?;
        Ok(())
    }

    /// Remember a file the transaction created, so rollback can remove it.
    fn track_created(&self, op: &BatchOp, was_new_file: bool, created: &mut Vec<PathBuf>) {
        if !(self.transaction && !self.dry_run && was_new_file) {
            return;
        }
        if let Some(target) = op
            .resolve_file_path()
            .ok()
            .map(std::path::Path::new)
            .and_then(|p| crate::path_safety::validate_path(p, self.workspace).ok())
        {
            created.push(target);
        }
    }

    /// Remember a move so rollback can reverse it.
    fn track_move(&self, op: &BatchOp, moves: &mut Vec<(PathBuf, PathBuf)>) {
        if !(self.transaction && !self.dry_run && op.op == "move") {
            return;
        }
        let (Some(src), Some(tgt)) = (
            op.source.as_deref().or(op.path.as_deref()),
            op.target.as_deref(),
        ) else {
            return;
        };
        if let (Ok(s), Ok(t)) = (
            crate::path_safety::validate_path(std::path::Path::new(src), self.workspace),
            crate::path_safety::validate_path(std::path::Path::new(tgt), self.workspace),
        ) {
            tracing::debug!(source = %s.display(), target = %t.display(), "recorded move for rollback");
            moves.push((s, t));
        }
    }

    /// Revert the transaction after a failed operation.
    ///
    /// A failed reversal is a disk-side condition, not malformed input, so it
    /// exits 80 / `TRANSACTION_ROLLBACK_FAILED` instead of the exit 65 that
    /// used to report it as a bad manifest.
    fn rollback(
        &mut self,
        backups: &[(PathBuf, PathBuf)],
        created: &[PathBuf],
        moves: &[(PathBuf, PathBuf)],
        idx: usize,
        failure: &anyhow::Error,
    ) -> Result<()> {
        let outcome = rollback_transaction(backups, created, moves, self.workspace);
        // The targets travel as an array because a transaction touches several
        // files, and the shape stays an array even for one file, because the
        // caller cannot know how many came back before reading the field.
        let mut targets: Vec<String> = backups
            .iter()
            .map(|(original, _)| original.display().to_string())
            .collect();
        targets.extend(created.iter().map(|path| path.display().to_string()));
        for (source, target) in moves {
            targets.push(source.display().to_string());
            targets.push(target.display().to_string());
        }
        targets.sort();
        targets.dedup();
        self.writer.set_targets(targets, self.target_source);

        if let Some(failed) = outcome.failed_path.clone() {
            // A reversal that stops halfway leaves the tree in a state the run
            // cannot vouch for, so the snapshots stay and the record names
            // them. Without that list the caller would have to list the
            // directory to find the last known copy of each file.
            tracing::error!(path = %failed, reason = %outcome.reason, "rollback failed");
            let preserved: Vec<String> = backups
                .iter()
                .filter(|(_, backup)| backup.exists())
                .map(|(_, backup)| backup.display().to_string())
                .collect();
            let err: anyhow::Error = crate::error::AtomwriteError::TransactionRollbackFailed {
                reason: outcome.reason,
            }
            .into();
            return Err(self.writer.publish_failure(
                &RollbackEvent {
                    r#type: "rollback",
                    files_restored: outcome.restored,
                    files_removed: outcome.removed,
                    total_reverted: outcome.restored + outcome.removed,
                    failed_paths: vec![failed],
                    backups_preserved: preserved,
                },
                err,
            ));
        }

        // Every file this rollback could restore is back, so the snapshots
        // have served their purpose.
        if !self.keep_backup || self.no_backup {
            discard_snapshots(backups);
        }
        // The record goes through `publish_failure`, not `write_event`:
        // the rollback is the consequence of a failed transaction, so
        // the record must carry `ok: false` and the very `code`/`exit`
        // of the error it reports. Stamped as a success it said
        // `ok: true` one line above the failure envelope, which the
        // agent reads as "the run that reverted everything succeeded".
        let err: anyhow::Error = crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "transaction rolled back after failure at operation {idx}: {failure:#}"
            ),
        }
        .into();
        Err(self.writer.publish_failure(
            &RollbackEvent {
                r#type: "rollback",
                files_restored: outcome.restored,
                files_removed: outcome.removed,
                total_reverted: outcome.restored + outcome.removed,
                failed_paths: Vec::new(),
                backups_preserved: Vec::new(),
            },
            err,
        ))
    }

    /// Close the run: the summary is the single terminal event either way.
    fn verdict(
        &mut self,
        dry_run: bool,
        transaction: bool,
        backups_preserved: Vec<String>,
    ) -> Result<()> {
        let summary = BatchSummary {
            r#type: "summary",
            operations: self.ops.len() as u64,
            succeeded: self.succeeded,
            failed: self.failed,
            dry_run,
            elapsed_ms: self.start.elapsed().as_millis() as u64,
            transaction: if transaction { Some(true) } else { None },
            committed: if transaction {
                Some(self.failed == 0)
            } else {
                None
            },
            backups_preserved,
        };
        if self.failed > 0 {
            // The summary carries the failure itself, so no second,
            // contradicting envelope follows it.
            let failure = crate::error::AtomwriteError::InvalidInput {
                reason: format!("{} batch operation(s) failed", self.failed),
            };
            return Err(self.writer.publish_failure(&summary, failure.into()));
        }
        self.writer.write_terminal(&summary)?;
        Ok(())
    }
}
