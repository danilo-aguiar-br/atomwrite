/// What one batch operation reports beside its detail line.
pub(crate) struct BatchOpOutcome {
    /// Detail line the event publishes as `details`.
    pub details: String,
    /// Write strategy used, `none` when the operation wrote no file content.
    pub write_strategy: &'static str,
    /// Why a requested backup was skipped, when the directory refused it.
    pub backup_skipped: Option<String>,
    /// Copy this operation created, or the snapshot the transaction already held.
    ///
    /// The standalone verbs already name it; without the same field here the
    /// caller has to list the directory to find the way back.
    pub backup_path: Option<String>,
    /// Advice the operation publishes because a flag changed nothing.
    pub warnings: Vec<String>,
}

impl BatchOpOutcome {
    /// Outcome of an operation that wrote no file content.
    fn none(details: String) -> Self {
        Self {
            details,
            write_strategy: "none",
            backup_skipped: None,
            backup_path: None,
            warnings: Vec::new(),
        }
    }

    /// Outcome of an operation that ran the atomic writer.
    fn wrote(
        details: String,
        result: &crate::atomic::WriteResult,
        copy: CopyDecision,
    ) -> Self {
        Self {
            details,
            write_strategy: result.write_strategy,
            backup_skipped: copy.skipped.or_else(|| result.backup_skipped.clone()),
            backup_path: copy.reported.or_else(|| result.backup_path.clone()),
            warnings: copy.warning.into_iter().collect(),
        }
    }
}

/// What one operation should do with the copy of its target.
///
/// A transaction may already hold that copy. When it does, the operation must
/// not write a second one, and the record has to name the path that survives
/// the commit instead of the path the writer would have produced.
struct CopyDecision {
    /// Whether the atomic writer should create the copy itself.
    create: bool,
    /// Why a requested copy was skipped, when the target is itself a copy.
    skipped: Option<String>,
    /// Path the operation publishes as `backup_path`.
    reported: Option<String>,
    /// Sentence explaining that the transaction already held the copy.
    warning: Option<String>,
}

impl CopyDecision {
    /// No copy is wanted, so the writer runs without one.
    fn none() -> Self {
        Self {
            create: false,
            skipped: None,
            reported: None,
            warning: None,
        }
    }
}

/// True when the flags ask this operation to have a copy of its target.
fn wants_copy(
    no_backup: bool,
    op_backup: bool,
    keep_backup: bool,
    backup_explicit: bool,
) -> bool {
    !no_backup && (op_backup || keep_backup || backup_explicit)
}

/// Decide who owns the copy of `path` in this operation.
///
/// A transactional run snapshots every target before the first mutation. When
/// that snapshot exists, the operation must not write a second copy: when the
/// commit keeps the snapshot, it is the copy the caller keeps and the record
/// names it; when the commit removes it, the record must not promise a path
/// that no longer exists. Outside a transaction the writer creates the copy as
/// before.
fn copy_decision(
    snapshots: &[(std::path::PathBuf, std::path::PathBuf)],
    path: &std::path::Path,
    wants_backup: bool,
    snapshot_survives: bool,
) -> CopyDecision {
    if !wants_backup {
        return CopyDecision::none();
    }
    if crate::commands::backup_names::is_backup_path(path) {
        return CopyDecision {
            create: false,
            skipped: Some(
                crate::commands::backup_names::BACKUP_OF_BACKUP_SKIPPED.to_string(),
            ),
            reported: None,
            warning: None,
        };
    }
    let Some(snapshot) = transaction_snapshot(snapshots, path) else {
        return CopyDecision {
            create: true,
            skipped: None,
            reported: None,
            warning: None,
        };
    };
    if !snapshot_survives {
        return CopyDecision::none();
    }
    CopyDecision {
        create: false,
        skipped: None,
        reported: Some(snapshot.display().to_string()),
        warning: Some(
            crate::commands::backup_names::TRANSACTION_SNAPSHOT_SUPERSEDES_COPY.to_string(),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_op(
    op: &BatchOp,
    _idx: usize,
    workspace: &std::path::Path,
    global: &GlobalArgs,
    dry_run: bool,
    keep_backup: bool,
    no_backup: bool,
    backup_explicit: bool,
    retention: u8,
    strict_atomic: bool,
    fuzzy_mode: crate::cli::FuzzyMode,
    fuzzy_threshold: Option<f64>,
    fuzzy_cfg: &crate::config::FuzzySection,
    snapshots: &[(std::path::PathBuf, std::path::PathBuf)],
) -> Result<BatchOpOutcome> {
    let max_size = global.effective_max_filesize();
    match op.op.as_str() {
        "write" => execute_write(
            op,
            workspace,
            dry_run,
            keep_backup,
            no_backup,
            backup_explicit,
            retention,
            strict_atomic,
            snapshots,
        ),
        "replace" => execute_replace(
            op,
            workspace,
            dry_run,
            max_size,
            keep_backup,
            no_backup,
            backup_explicit,
            retention,
            strict_atomic,
            fuzzy_mode,
            fuzzy_threshold,
            fuzzy_cfg,
            snapshots,
        ),
        "delete" => execute_delete(
            op,
            workspace,
            dry_run,
            max_size,
            keep_backup,
            no_backup,
            backup_explicit,
            retention,
            snapshots,
        ),
        "edit" => execute_edit(
            op,
            workspace,
            dry_run,
            max_size,
            keep_backup,
            no_backup,
            backup_explicit,
            retention,
            strict_atomic,
            fuzzy_mode,
            fuzzy_threshold,
            fuzzy_cfg,
            snapshots,
        ),
        "hash" => execute_hash(op, workspace, max_size),
        "move" => execute_move(
            op,
            workspace,
            dry_run,
            max_size,
            keep_backup,
            no_backup,
            backup_explicit,
            retention,
            snapshots,
        ),
        "copy" => execute_copy(
            op,
            workspace,
            dry_run,
            max_size,
            keep_backup,
            no_backup,
            backup_explicit,
            retention,
            strict_atomic,
            snapshots,
        ),
        _ => bail!("unsupported batch operation: {}", op.op),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_write(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    keep_backup: bool,
    no_backup: bool,
    backup_explicit: bool,
    retention: u8,
    strict_atomic: bool,
    snapshots: &[(std::path::PathBuf, std::path::PathBuf)],
) -> Result<BatchOpOutcome> {
    let target = op.resolve_file_path()?;
    let content = op
        .content
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("write operation requires 'content' field"))?;

    let target_path =
        crate::path_safety::validate_path(std::path::Path::new(target), workspace)?;

    if dry_run {
        return Ok(BatchOpOutcome::none(format!(
            "would write {} bytes to {target}",
            content.len()
        )));
    }

    let copy = copy_decision(
        snapshots,
        &target_path,
        wants_copy(no_backup, op.backup, keep_backup, backup_explicit),
        keep_backup && !no_backup,
    );
    let opts = AtomicWriteOptions {
        backup: copy.create,
        syntax_check: false,
        retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    let result = atomic_write(&target_path, content.as_bytes(), &opts, workspace)?;
    let details = format!(
        "wrote {} bytes, checksum={}",
        result.bytes_written, result.checksum
    );
    Ok(BatchOpOutcome::wrote(details, &result, copy))
}

#[allow(clippy::too_many_arguments)]
fn execute_replace(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    max_size: u64,
    keep_backup: bool,
    no_backup: bool,
    backup_explicit: bool,
    retention: u8,
    strict_atomic: bool,
    fuzzy_mode: crate::cli::FuzzyMode,
    fuzzy_threshold: Option<f64>,
    fuzzy_cfg: &crate::config::FuzzySection,
    snapshots: &[(std::path::PathBuf, std::path::PathBuf)],
) -> Result<BatchOpOutcome> {
    let path_str = op.resolve_file_path()?;
    let pattern =
        op.pattern.as_deref().or(op.old.as_deref()).ok_or_else(|| {
            anyhow::anyhow!("replace operation requires 'pattern' or 'old' field")
        })?;
    let replacement = op
        .replacement
        .as_deref()
        .or(op.new.as_deref())
        .ok_or_else(|| {
            anyhow::anyhow!("replace operation requires 'replacement' or 'new' field")
        })?;

    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    // A1: lock BEFORE reading, hold it to the commit.
    let _update_lock = validated
        .exists()
        .then(|| crate::lock::acquire_for_update(&validated))
        .transpose()?;
    let content = crate::file_io::read_file_string(&validated, max_size)
        .with_context(|| format!("cannot read {}", validated.display()))?;

    // Prefer exact multi-replace; fall back to fuzzy cascade (v0.1.29).
    let new_content = if content.contains(pattern) {
        content.replace(pattern, replacement)
    } else {
        match crate::fuzzy::match_pair_cfg(
            &content,
            pattern,
            replacement,
            fuzzy_mode,
            fuzzy_threshold,
            fuzzy_cfg,
            false,
        ) {
            Ok((edited, _)) => edited,
            // Only "no match" means zero replacements; a cancel, an oversize
            // refusal or an ambiguous match must reach the caller.
            Err(crate::error::AtomwriteError::MatchFailed { .. }) => content.clone(),
            Err(e) => return Err(e.into()),
        }
    };
    if new_content == content {
        return Ok(BatchOpOutcome::none(format!("no matches in {path_str}")));
    }

    let count = content.matches(pattern).count();

    if dry_run {
        return Ok(BatchOpOutcome::none(format!(
            "would replace {count} occurrence(s) in {path_str}"
        )));
    }

    let checksum_before = checksum::hash_bytes(content.as_bytes());
    let copy = copy_decision(
        snapshots,
        &validated,
        wants_copy(no_backup, op.backup, keep_backup, backup_explicit),
        keep_backup && !no_backup,
    );
    let opts = AtomicWriteOptions {
        backup: copy.create,
        syntax_check: false,
        retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    let result = atomic_write(&validated, new_content.as_bytes(), &opts, workspace)?;
    let details = format!(
        "replaced {count} occurrence(s), checksum_before={checksum_before}, checksum_after={}",
        result.checksum
    );
    Ok(BatchOpOutcome::wrote(details, &result, copy))
}

/// The copy the transaction already made for this exact path, when there is one.
///
/// A transactional run snapshots every target before its first mutation, so
/// the operation that would copy the same file again finds the work done: one
/// copy on disk, one path to report, and no second file to prune later.
fn transaction_snapshot<'a>(
    snapshots: &'a [(std::path::PathBuf, std::path::PathBuf)],
    path: &std::path::Path,
) -> Option<&'a std::path::Path> {
    snapshots
        .iter()
        .find(|(original, _)| original == path)
        .map(|(_, backup)| backup.as_path())
}

#[allow(clippy::too_many_arguments)]
fn execute_delete(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    max_size: u64,
    keep_backup: bool,
    no_backup: bool,
    backup_explicit: bool,
    retention: u8,
    snapshots: &[(std::path::PathBuf, std::path::PathBuf)],
) -> Result<BatchOpOutcome> {
    let target = op.resolve_file_path()?;

    let path = std::path::Path::new(target);
    let validated = crate::path_safety::validate_path(path, workspace)?;

    if !validated.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: validated }.into());
    }

    // A file that is itself a backup gets no backup of its own, by the same rule
    // the standalone `delete` applies: the copy doubles the bytes and no verb
    // ever prunes copies of copies.
    // `--keep-backup` does not create a copy here, by the same rule the
    // standalone `delete` follows: it only keeps one that already exists.
    let wants_backup = !no_backup && (op.backup || backup_explicit);

    if dry_run {
        // A preview creates no copy either, so the flag changes nothing and
        // the answer travels with the record.
        let mut warnings = Vec::new();
        if keep_backup {
            warnings.push(
                crate::commands::backup_names::KEEP_BACKUP_REDUNDANT_FOR_DELETE.to_string(),
            );
        }
        return Ok(BatchOpOutcome {
            details: format!("would delete {target}"),
            write_strategy: "none",
            backup_skipped: None,
            backup_path: None,
            warnings,
        });
    }

    let target_is_backup = wants_backup
        && crate::commands::backup_names::is_backup_path(std::path::Path::new(&validated));
    let mut backup_skipped = None;
    let mut backup_path = None;
    let mut warnings = Vec::new();
    if wants_backup && target_is_backup {
        backup_skipped = Some(
            crate::commands::backup_names::BACKUP_OF_BACKUP_SKIPPED.to_string(),
        );
    } else if wants_backup {
        if let Some(snapshot) = transaction_snapshot(snapshots, &validated) {
            // The transaction copied this file before the first mutation, so a
            // second copy would double the bytes without adding a way back.
            backup_path = Some(snapshot.display().to_string());
            warnings.push(
                crate::commands::backup_names::TRANSACTION_SNAPSHOT_SUPERSEDES_COPY.to_string(),
            );
        } else {
            let created = crate::atomic::create_backup(&validated, retention)
                .with_context(|| format!("cannot backup {target}"))?;
            backup_path = Some(created.display().to_string());
        }
    }

    let checksum = checksum::hash_file(&validated, max_size)?;
    std::fs::remove_file(&validated).with_context(|| format!("cannot delete {target}"))?;

    if let Some(parent) = validated.parent() {
        if let Err(e) = crate::platform::fsync_dir(parent) {
            tracing::warn!(
                path = %parent.display(),
                error = %e,
                "fsync_dir after batch delete failed"
            );
        }
    }

    // The flag only keeps a copy that exists, so the warning is per
    // operation: a target that is itself a backup, or a call that never asked
    // for the copy, has nothing for the flag to keep.
    if keep_backup && backup_path.is_none() {
        warnings.push(
            crate::commands::backup_names::KEEP_BACKUP_REDUNDANT_FOR_DELETE.to_string(),
        );
    }

    Ok(BatchOpOutcome {
        details: format!("deleted {target}, checksum_before={checksum}"),
        write_strategy: "none",
        backup_skipped,
        backup_path,
        warnings,
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_edit(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    max_size: u64,
    keep_backup: bool,
    no_backup: bool,
    backup_explicit: bool,
    retention: u8,
    strict_atomic: bool,
    fuzzy_mode: crate::cli::FuzzyMode,
    fuzzy_threshold: Option<f64>,
    fuzzy_cfg: &crate::config::FuzzySection,
    snapshots: &[(std::path::PathBuf, std::path::PathBuf)],
) -> Result<BatchOpOutcome> {
    let path_str = op.resolve_file_path()?;
    let old = op
        .old
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("edit operation requires 'old' field"))?;
    let new = op.new.as_deref().unwrap_or("");

    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    // A1: lock BEFORE reading, hold it to the commit.
    let _update_lock = validated
        .exists()
        .then(|| crate::lock::acquire_for_update(&validated))
        .transpose()?;
    let content = crate::file_io::read_file_string(&validated, max_size)
        .with_context(|| format!("cannot read {}", validated.display()))?;

    if !content.contains(old) {
        // Still try fuzzy path; exact-only early fail is too strict with config cascade.
        // Fall through to match_pair below.
    }

    if dry_run {
        return Ok(BatchOpOutcome::none(format!("would edit {path_str}")));
    }

    let edited = match crate::fuzzy::match_pair_cfg(
        &content,
        old,
        new,
        fuzzy_mode,
        fuzzy_threshold,
        fuzzy_cfg,
        false,
    ) {
        Ok((e, _)) => e,
        Err(e) => return Err(e.into()),
    };
    let checksum_before = checksum::hash_bytes(content.as_bytes());
    let copy = copy_decision(
        snapshots,
        &validated,
        wants_copy(no_backup, op.backup, keep_backup, backup_explicit),
        keep_backup && !no_backup,
    );
    let opts = AtomicWriteOptions {
        backup: copy.create,
        syntax_check: false,
        retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    let result = atomic_write(&validated, edited.as_bytes(), &opts, workspace)?;
    let details = format!(
        "edited {path_str}, checksum_before={checksum_before}, checksum_after={}",
        result.checksum
    );
    Ok(BatchOpOutcome::wrote(details, &result, copy))
}

fn execute_hash(op: &BatchOp, workspace: &std::path::Path, max_size: u64) -> Result<BatchOpOutcome> {
    let path_str = op.resolve_file_path()?;
    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    let hash = checksum::hash_file(&validated, max_size)?;
    Ok(BatchOpOutcome::none(format!("hash={hash}")))
}

#[allow(clippy::too_many_arguments)]
fn execute_move(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    max_size: u64,
    keep_backup: bool,
    no_backup: bool,
    backup_explicit: bool,
    retention: u8,
    snapshots: &[(std::path::PathBuf, std::path::PathBuf)],
) -> Result<BatchOpOutcome> {
    let source_str = op
        .source
        .as_deref()
        .or(op.path.as_deref())
        .ok_or_else(|| anyhow::anyhow!("move operation requires 'source' field"))?;
    let dest_str = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("move operation requires 'target' (destination) field"))?;

    let source = crate::path_safety::validate_path(std::path::Path::new(source_str), workspace)?;
    let dest = crate::path_safety::validate_path(std::path::Path::new(dest_str), workspace)?;

    if !source.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: source }.into());
    }

    // GAP-108: batch move must check target existence like standalone move
    if dest.exists() && !op.force.unwrap_or(false) {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "target {} already exists, use \"force\":true in the batch op to overwrite",
                dest.display()
            ),
        }
        .into());
    }

    if dry_run {
        return Ok(BatchOpOutcome::none(format!(
            "would move {source_str} to {dest_str}"
        )));
    }

    let wants_backup = !no_backup && (op.backup || keep_backup || backup_explicit);
    let mut backup_skipped = None;
    let mut backup_path = None;
    let mut warnings = Vec::new();
    if wants_backup && dest.exists() {
        if crate::commands::backup_names::is_backup_path(&dest) {
            backup_skipped = Some(
                crate::commands::backup_names::BACKUP_OF_BACKUP_SKIPPED.to_string(),
            );
        } else if let Some(snapshot) = transaction_snapshot(snapshots, &dest) {
            // Same rule as delete: the transaction owns the copy of a target
            // this operation was about to overwrite.
            backup_path = Some(snapshot.display().to_string());
            warnings.push(
                crate::commands::backup_names::TRANSACTION_SNAPSHOT_SUPERSEDES_COPY.to_string(),
            );
        } else {
            let created = crate::atomic::create_backup(&dest, retention)
                .with_context(|| format!("cannot backup {}", dest.display()))?;
            backup_path = Some(created.display().to_string());
        }
    }

    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create parent dir for {dest_str}"))?;
        }
    }
    match std::fs::rename(&source, &dest) {
        Ok(()) => {}
        Err(error) if error.raw_os_error() == Some(crate::constants::ERRNO_EXDEV) => {
            // Cross-device fallback: copy, verify and remove, the same path the
            // standalone `move` takes.
            if source.is_dir() {
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: format!(
                        "cannot move directory {} across devices; copy -r then delete, or keep same filesystem",
                        source.display()
                    ),
                }
                .into());
            }
            let expected = checksum::hash_file(&source, max_size)?;
            let content = crate::file_io::read_file_bytes(&source, max_size)?;
            crate::atomic::atomic_write(
                &dest,
                &content,
                &crate::atomic::AtomicWriteOptions::default(),
                workspace,
            )?;
            let verify = checksum::hash_file(&dest, max_size)?;
            if verify != expected {
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: format!(
                        "checksum mismatch after cross-device copy: expected {expected}, got {verify}"
                    ),
                }
                .into());
            }
            std::fs::remove_file(&source)
                .with_context(|| format!("cannot remove source {}", source.display()))?;
        }
        Err(error) => {
            return Err(error).with_context(|| format!("cannot move {source_str} to {dest_str}"));
        }
    }
    // The rename is only durable once both parent directories are flushed.
    if let Some(parent) = dest.parent() {
        crate::platform::fsync_dir(parent)
            .with_context(|| format!("cannot flush destination directory of {dest_str}"))?;
    }
    if let Some(parent) = source.parent() {
        crate::platform::fsync_dir(parent)
            .with_context(|| format!("cannot flush source directory of {source_str}"))?;
    }
    Ok(BatchOpOutcome {
        details: format!("moved {source_str} to {dest_str}"),
        write_strategy: "rename",
        backup_skipped,
        backup_path,
        warnings,
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_copy(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    max_size: u64,
    keep_backup: bool,
    no_backup: bool,
    backup_explicit: bool,
    retention: u8,
    strict_atomic: bool,
    snapshots: &[(std::path::PathBuf, std::path::PathBuf)],
) -> Result<BatchOpOutcome> {
    let source_str = op
        .source
        .as_deref()
        .or(op.path.as_deref())
        .ok_or_else(|| anyhow::anyhow!("copy operation requires 'source' field"))?;
    let dest_str = op
        .target
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("copy operation requires 'target' (destination) field"))?;

    let source = crate::path_safety::validate_path(std::path::Path::new(source_str), workspace)?;
    let dest = crate::path_safety::validate_path(std::path::Path::new(dest_str), workspace)?;

    if !source.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: source }.into());
    }

    // GAP-108: batch copy must check target existence like standalone copy
    if dest.exists() && !op.force.unwrap_or(false) {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "target {} already exists, use \"force\":true in the batch op to overwrite",
                dest.display()
            ),
        }
        .into());
    }

    if dry_run {
        return Ok(BatchOpOutcome::none(format!(
            "would copy {source_str} to {dest_str}"
        )));
    }

    let content = crate::file_io::read_file_bytes(&source, max_size)
        .with_context(|| format!("cannot read {}", source.display()))?;
    let copy = copy_decision(
        snapshots,
        &dest,
        wants_copy(no_backup, op.backup, keep_backup, backup_explicit),
        !no_backup,
    );
    let opts = AtomicWriteOptions {
        backup: copy.create,
        syntax_check: false,
        retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    let result = atomic_write(&dest, &content, &opts, workspace)?;
    let details = format!(
        "copied {source_str} to {dest_str}, checksum={}",
        result.checksum
    );
    Ok(BatchOpOutcome::wrote(details, &result, copy))
}
