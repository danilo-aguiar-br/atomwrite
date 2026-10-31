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
}

/// Run one operation with the batch-wide options.
#[allow(clippy::too_many_arguments)]
fn execute_op_with(op: &BatchOp, idx: usize, opts: &BatchOpOptions<'_>) -> Result<String> {
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
    )
}

/// Snapshot one file before the transaction touches it.
///
/// A free function, not a method: the fan-out closure cannot borrow the run
/// (the writer is not `Sync`), and this needs nothing but the retention bound.
fn snapshot_file(path: &std::path::Path, retention: u8) -> Result<(PathBuf, PathBuf)> {
    let backup = crate::atomic::create_backup(path, retention)
        .with_context(|| format!("transaction pre-backup failed for {}", path.display()))?;
    Ok((path.to_path_buf(), backup))
}

/// One batch invocation: the parsed manifest plus the mutable state a run keeps.
pub(crate) struct BatchRun<'a, W: Write> {
    pub(crate) ops: &'a [BatchOp],
    pub(crate) workspace: &'a std::path::Path,
    pub(crate) global: &'a GlobalArgs,
    pub(crate) dry_run: bool,
    pub(crate) transaction: bool,
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
    fn execute(&mut self) -> Result<()> {
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
            let paths = collect_target_paths(self.ops, self.workspace);
            // The closure captures only the retention bound, never `self`: the
            // writer is not `Sync`, and the snapshot needs no access to it.
            let retention = self.retention;
            if should_parallelize(paths.len()) {
                paths
                    .par_iter()
                    .map(|path| snapshot_file(path, retention))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                let mut pairs = Vec::with_capacity(paths.len());
                for path in paths {
                    pairs.push(snapshot_file(&path, retention)?);
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
            let opts = self.op_options();
            let shutdown = self.shutdown;
            let results: Vec<(usize, Result<String, String>, u64)> = self
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
                    Ok(details) => {
                        succeeded += 1;
                        self.writer.write_event(&BatchOpResult {
                            r#type: "batch_op",
                            index: idx as u64,
                            op: &op.op,
                            status: "ok",
                            details: Some(details),
                            error: None,
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

                match self.run_op(op, idx) {
                    Ok(details) => {
                        succeeded += 1;
                        self.track_created(op, was_new_file, &mut created_files);
                        self.track_move(op, &mut moves_to_reverse);
                        self.writer.write_event(&BatchOpResult {
                            r#type: "batch_op",
                            index: idx as u64,
                            op: &op.op,
                            status: "ok",
                            details: Some(details),
                            error: None,
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
                            elapsed_ms: op_start.elapsed().as_millis() as u64,
                        })?;
                        if self.transaction {
                            self.rollback(&backups, &created_files, &moves_to_reverse, idx, &e)?;
                        }
                    }
                }
            }
        }

        self.succeeded = succeeded;
        self.failed = failed;
        Ok(())
    }

    /// Execute one operation with the batch-wide options.
    fn run_op(&self, op: &BatchOp, idx: usize) -> Result<String> {
        execute_op_with(op, idx, &self.op_options())
    }

    /// The batch-wide execution options, cheap to copy into a fan-out closure.
    fn op_options(&self) -> BatchOpOptions<'_> {
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
        match rollback_transaction(backups, created, moves, self.workspace) {
            Ok((restored, removed)) => {
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
                        files_restored: restored,
                        files_removed: removed,
                        total_reverted: restored + removed,
                    },
                    err,
                ))
            }
            Err(rb_err) => {
                tracing::error!(error = %rb_err, "rollback failed");
                Err(crate::error::AtomwriteError::TransactionRollbackFailed {
                    reason: format!("{rb_err:#}"),
                }
                .into())
            }
        }
    }

    /// Close the run: the summary is the single terminal event either way.
    fn verdict(&mut self, dry_run: bool, transaction: bool) -> Result<()> {
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
