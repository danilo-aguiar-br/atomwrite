/// Execute multiple operations from an NDJSON manifest in batch mode.
///
/// When `manifest_path` is `Some`, the NDJSON manifest is read from that file
/// instead of from stdin. The path is validated against the workspace jail.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if reading stdin or writing results fails.
/// Returns `AtomwriteError::InvalidInput` if the manifest contains invalid operations.
#[tracing::instrument(skip_all, fields(command = "batch"))]
#[allow(clippy::too_many_arguments)]
pub fn cmd_batch(
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    dry_run: bool,
    transaction: bool,
    manifest_path: Option<&std::path::Path>,
    shutdown: &ShutdownSignal,
    backup_opts: &crate::cli_args::BackupOpts,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<()> {
    let resolved = crate::commands::resolve_backup(backup_opts, defaults);
    let (batch_fuzzy_mode, batch_fuzzy_threshold) =
        crate::config::resolve_fuzzy(crate::cli::FuzzyMode::Auto, None, fuzzy_cfg)?;
    let retention = resolved.retention;
    let keep_backup = resolved.keep;
    let no_backup = backup_opts.no_backup;
    let backup_explicit = backup_opts.backup == Some(true);
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    // Resolve manifest source: file (validated) or stdin
    let mut reader: Box<dyn Read> = if let Some(manifest_path) = manifest_path {
        let validated_manifest = crate::path_safety::validate_path(manifest_path, &workspace)
            .with_context(|| {
                format!(
                    "manifest path escapes workspace: {}",
                    manifest_path.display()
                )
            })?;
        if !validated_manifest.is_file() {
            return Err(crate::error::AtomwriteError::NotFound {
                path: validated_manifest,
            }
            .into());
        }
        let file = std::fs::File::open(&validated_manifest)
            .with_context(|| format!("cannot open manifest {}", validated_manifest.display()))?;
        Box::new(file)
    } else {
        Box::new(stdin)
    };

    let mut buf_reader =
        std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, &mut *reader);
    let mut ops: Vec<BatchOp> = Vec::with_capacity(16);
    let mut line_buf = String::new();
    let mut idx = 0usize;

    loop {
        let n = crate::output::read_limited_line(
            &mut buf_reader,
            &mut line_buf,
            crate::constants::MAX_NDJSON_LINE_SIZE,
        )
        .with_context(|| format!("failed to read manifest line {}", idx + 1))?;
        if n == 0 {
            break;
        }
        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            idx += 1;
            continue;
        }
        let jd = &mut serde_json::Deserializer::from_str(trimmed);
        let op: BatchOp = serde_path_to_error::deserialize(jd).map_err(|e| {
            if e.inner().classify() == serde_json::error::Category::Io {
                crate::error::AtomwriteError::Io {
                    source: std::io::Error::other(e.to_string()),
                }
            } else {
                crate::error::AtomwriteError::InvalidInput {
                    reason: format!("invalid batch operation at line {}: {e}", idx + 1),
                }
            }
        })?;
        ops.push(op);
        idx += 1;
    }

    if ops.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "empty batch manifest: no operations provided".into(),
        }
        .into());
    }

    // In transaction mode, snapshot all existing files before any mutation.
    // Paths are unique (collect_target_paths dedup); independent I/O → par_iter.
    // Ops + rollback stay sequential (ordered side-effects / reverse restore).
    let backups: Vec<(PathBuf, PathBuf)> = if transaction && !dry_run {
        let paths = collect_target_paths(&ops, &workspace);
        if should_parallelize(paths.len()) {
            paths
                .par_iter()
                .map(|path| {
                    let backup = crate::atomic::create_backup(path, retention).with_context(|| {
                        format!("transaction pre-backup failed for {}", path.display())
                    })?;
                    Ok::<_, anyhow::Error>((path.clone(), backup))
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut pairs = Vec::with_capacity(paths.len());
            for path in paths {
                let backup = crate::atomic::create_backup(&path, retention).with_context(|| {
                    format!("transaction pre-backup failed for {}", path.display())
                })?;
                pairs.push((path, backup));
            }
            pairs
        }
    } else {
        Vec::new()
    };

    // Track files created during the transaction so they can be removed on rollback.
    let mut created_files: Vec<PathBuf> = Vec::new();
    // Track moves so they can be reversed (target → source) on rollback.
    let mut moves_to_reverse: Vec<(PathBuf, PathBuf)> = Vec::new();

    let mut succeeded: u64 = 0;
    let mut failed: u64 = 0;
    let total_ops = ops.len() as u64;
    // A-018: adaptive progress cadence from named constants (not magic literals).
    let progress_every = if total_ops == 0 {
        0
    } else {
        (total_ops / crate::constants::BATCH_PROGRESS_DIVISOR).clamp(
            crate::constants::BATCH_PROGRESS_MIN,
            crate::constants::BATCH_PROGRESS_MAX,
        )
    };

    // Parallel path: non-transactional + unique targets + multi-op.
    // Transaction / overlapping targets stay sequential for rollback + safety.
    let parallel_ok = !transaction
        && should_parallelize(ops.len())
        && batch_targets_unique(&ops);

    if parallel_ok {
        tracing::debug!(
            ops = ops.len(),
            "batch: parallel non-transactional fan-out"
        );
        let results: Vec<(usize, Result<String, String>, u64)> = ops
            .par_iter()
            .enumerate()
            .map(|(idx, op)| {
                if shutdown.is_shutdown() {
                    return (
                        idx,
                        Err("batch cancelled by signal".to_string()),
                        0,
                    );
                }
                let op_start = Instant::now();
                let result = execute_op(
                    op,
                    idx,
                    &workspace,
                    global,
                    dry_run,
                    keep_backup,
                    no_backup,
                    backup_explicit,
                    retention,
                    batch_fuzzy_mode,
                    batch_fuzzy_threshold,
                    fuzzy_cfg,
                )
                .map_err(|e| format!("{e:#}"));
                (idx, result, op_start.elapsed().as_millis() as u64)
            })
            .collect();

        // Emit in original manifest order (stable agent contract).
        let mut ordered = results;
        ordered.sort_by_key(|(idx, _, _)| *idx);
        for (idx, result, elapsed_ms) in ordered {
            let op = &ops[idx];
            match result {
                Ok(details) => {
                    succeeded += 1;
                    writer.write_event(&BatchOpResult {
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
                    writer.write_event(&BatchOpResult {
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
        for (idx, op) in ops.iter().enumerate() {
            if shutdown.is_shutdown() {
                tracing::info!(
                    completed = idx,
                    total = ops.len(),
                    "batch interrupted by signal"
                );
                break;
            }
            if progress_every > 0
                && idx > 0
                && (idx as u64).is_multiple_of(progress_every)
                && global.quiet == 0
                && !global.no_progress
            {
                let done = idx as u64;
                let elapsed = start.elapsed().as_secs_f64().max(0.001);
                let rate = done as f64 / elapsed;
                let remaining = total_ops.saturating_sub(done);
                let eta_ms = if rate > 0.0 {
                    Some(((remaining as f64 / rate) * 1000.0) as u64)
                } else {
                    None
                };
                writer.write_event(&ProgressEvent {
                    r#type: "progress",
                    done,
                    total: total_ops,
                    rate_per_s: Some(rate),
                    eta_ms,
                    phase: "batch".into(),
                })?;
            }

            let op_start = Instant::now();
            // Pre-snapshot existence so we know if the op created a new file at target.
            let creates_target = matches!(op.op.as_str(), "write" | "move" | "copy");
            let was_new_file = if transaction && !dry_run && creates_target {
                op.resolve_file_path()
                    .ok()
                    .map(std::path::Path::new)
                    .and_then(|p| crate::path_safety::validate_path(p, &workspace).ok())
                    .map(|p| !p.exists())
                    .unwrap_or(false)
            } else {
                false
            };
            let result = execute_op(
                op,
                idx,
                &workspace,
                global,
                dry_run,
                keep_backup,
                no_backup,
                backup_explicit,
                retention,
                batch_fuzzy_mode,
                batch_fuzzy_threshold,
                    fuzzy_cfg,
                );

            match result {
                Ok(details) => {
                    succeeded += 1;
                    if transaction && !dry_run && was_new_file {
                        if let Some(target) = op
                            .resolve_file_path()
                            .ok()
                            .map(std::path::Path::new)
                            .and_then(|p| crate::path_safety::validate_path(p, &workspace).ok())
                        {
                            created_files.push(target);
                        }
                    }
                    if transaction && !dry_run && op.op == "move" {
                        if let (Some(src), Some(tgt)) = (
                            op.source.as_deref().or(op.path.as_deref()),
                            op.target.as_deref(),
                        ) {
                            if let (Ok(s), Ok(t)) = (
                                crate::path_safety::validate_path(
                                    std::path::Path::new(src),
                                    &workspace,
                                ),
                                crate::path_safety::validate_path(
                                    std::path::Path::new(tgt),
                                    &workspace,
                                ),
                            ) {
                                tracing::debug!(source = %s.display(), target = %t.display(), "recorded move for rollback");
                                moves_to_reverse.push((s, t));
                            }
                        }
                    }
                    let event = BatchOpResult {
                        r#type: "batch_op",
                        index: idx as u64,
                        op: &op.op,
                        status: "ok",
                        details: Some(details),
                        error: None,
                        elapsed_ms: op_start.elapsed().as_millis() as u64,
                    };
                    writer.write_event(&event)?;
                }
                Err(e) => {
                    failed += 1;
                    let event = BatchOpResult {
                        r#type: "batch_op",
                        index: idx as u64,
                        op: &op.op,
                        status: "failed",
                        details: None,
                        error: Some(format!("{e:#}")),
                        elapsed_ms: op_start.elapsed().as_millis() as u64,
                    };
                    writer.write_event(&event)?;

                    if transaction {
                        match rollback_transaction(
                            &backups,
                            &created_files,
                            &moves_to_reverse,
                            &workspace,
                        ) {
                            Ok((restored, removed)) => {
                                let rollback_event = RollbackEvent {
                                    r#type: "rollback",
                                    files_restored: restored,
                                    files_removed: removed,
                                    total_reverted: restored + removed,
                                };
                                writer.write_event(&rollback_event)?;
                                return Err(crate::error::AtomwriteError::InvalidInput {
                                    reason: format!(
                                        "transaction rolled back after failure at operation {idx}: {e:#}"
                                    ),
                                }
                                .into());
                            }
                            Err(rb_err) => {
                                tracing::error!(error = %rb_err, "rollback failed");
                                return Err(crate::error::AtomwriteError::InvalidInput {
                                    reason: format!("transaction rollback failed: {rb_err:#}"),
                                }
                                .into());
                            }
                        }
                    }
                }
            }
        }
    }

    let committed = if transaction { Some(failed == 0) } else { None };
    let summary = BatchSummary {
        r#type: "summary",
        operations: ops.len() as u64,
        succeeded,
        failed,
        dry_run,
        elapsed_ms: start.elapsed().as_millis() as u64,
        transaction: if transaction { Some(true) } else { None },
        committed,
    };
    writer.write_event(&summary)?;

    if failed > 0 {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("{failed} batch operation(s) failed"),
        }
        .into());
    }

    Ok(())
}

