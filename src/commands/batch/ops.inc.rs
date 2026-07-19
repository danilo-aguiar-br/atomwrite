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
    fuzzy_mode: crate::cli::FuzzyMode,
    fuzzy_threshold: Option<f64>,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<String> {
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
            fuzzy_mode,
            fuzzy_threshold,
                    fuzzy_cfg,
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
            fuzzy_mode,
            fuzzy_threshold,
                    fuzzy_cfg,
                ),
        "hash" => execute_hash(op, workspace, max_size),
        "move" => execute_move(op, workspace, dry_run),
        "copy" => execute_copy(
            op,
            workspace,
            dry_run,
            max_size,
            keep_backup,
            no_backup,
            backup_explicit,
            retention,
        ),
        _ => bail!("unsupported batch operation: {}", op.op),
    }
}

fn execute_write(
    op: &BatchOp,
    workspace: &std::path::Path,
    dry_run: bool,
    keep_backup: bool,
    no_backup: bool,
    backup_explicit: bool,
    retention: u8,
) -> Result<String> {
    let target = op.resolve_file_path()?;
    let content = op
        .content
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("write operation requires 'content' field"))?;

    let target_path = std::path::Path::new(target);

    if dry_run {
        return Ok(format!("would write {} bytes to {target}", content.len()));
    }

    let opts = AtomicWriteOptions {
        backup: !no_backup && (op.backup || keep_backup || backup_explicit),
        syntax_check: false,
        retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    let result = atomic_write(target_path, content.as_bytes(), &opts, workspace)?;
    Ok(format!(
        "wrote {} bytes, checksum={}",
        result.bytes_written, result.checksum
    ))
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
    fuzzy_mode: crate::cli::FuzzyMode,
    fuzzy_threshold: Option<f64>,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<String> {
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
            Err(_) => content.clone(),
        }
    };
    if new_content == content {
        return Ok(format!("no matches in {path_str}"));
    }

    let count = content.matches(pattern).count();

    if dry_run {
        return Ok(format!("would replace {count} occurrence(s) in {path_str}"));
    }

    let checksum_before = checksum::hash_bytes(content.as_bytes());
    let opts = AtomicWriteOptions {
        backup: !no_backup && (op.backup || keep_backup || backup_explicit),
        syntax_check: false,
        retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    let result = atomic_write(&validated, new_content.as_bytes(), &opts, workspace)?;
    Ok(format!(
        "replaced {count} occurrence(s), checksum_before={checksum_before}, checksum_after={}",
        result.checksum
    ))
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
) -> Result<String> {
    let target = op.resolve_file_path()?;

    let path = std::path::Path::new(target);
    let validated = crate::path_safety::validate_path(path, workspace)?;

    if !validated.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: validated }.into());
    }

    if dry_run {
        return Ok(format!("would delete {target}"));
    }

    if !no_backup && (op.backup || keep_backup || backup_explicit) {
        crate::atomic::create_backup(&validated, retention)
            .with_context(|| format!("cannot backup {target}"))?;
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

    Ok(format!("deleted {target}, checksum_before={checksum}"))
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
    fuzzy_mode: crate::cli::FuzzyMode,
    fuzzy_threshold: Option<f64>,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<String> {
    let path_str = op.resolve_file_path()?;
    let old = op
        .old
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("edit operation requires 'old' field"))?;
    let new = op.new.as_deref().unwrap_or("");

    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    let content = crate::file_io::read_file_string(&validated, max_size)
        .with_context(|| format!("cannot read {}", validated.display()))?;

    if !content.contains(old) {
        // Still try fuzzy path; exact-only early fail is too strict with config cascade.
        // Fall through to match_pair below.
    }

    if dry_run {
        return Ok(format!("would edit {path_str}"));
    }

    let edited =
        match crate::fuzzy::match_pair_cfg(
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
    let opts = AtomicWriteOptions {
        backup: !no_backup && (op.backup || keep_backup || backup_explicit),
        syntax_check: false,
        retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    let result = atomic_write(&validated, edited.as_bytes(), &opts, workspace)?;
    Ok(format!(
        "edited {path_str}, checksum_before={checksum_before}, checksum_after={}",
        result.checksum
    ))
}

fn execute_hash(op: &BatchOp, workspace: &std::path::Path, max_size: u64) -> Result<String> {
    let path_str = op.resolve_file_path()?;
    let path = std::path::Path::new(path_str);
    let validated = crate::path_safety::validate_path(path, workspace)?;
    let hash = checksum::hash_file(&validated, max_size)?;
    Ok(format!("hash={hash}"))
}

fn execute_move(op: &BatchOp, workspace: &std::path::Path, dry_run: bool) -> Result<String> {
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
        return Ok(format!("would move {source_str} to {dest_str}"));
    }

    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create parent dir for {dest_str}"))?;
        }
    }
    std::fs::rename(&source, &dest)
        .with_context(|| format!("cannot move {source_str} to {dest_str}"))?;
    if let Some(parent) = dest.parent() {
        let _ = crate::platform::fsync_dir(parent);
    }
    if let Some(parent) = source.parent() {
        let _ = crate::platform::fsync_dir(parent);
    }
    Ok(format!("moved {source_str} to {dest_str}"))
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
) -> Result<String> {
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
        return Ok(format!("would copy {source_str} to {dest_str}"));
    }

    let content = crate::file_io::read_file_bytes(&source, max_size)
        .with_context(|| format!("cannot read {}", source.display()))?;
    let opts = AtomicWriteOptions {
        backup: !no_backup && (op.backup || keep_backup || backup_explicit),
        syntax_check: false,
        retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    let result = atomic_write(&dest, &content, &opts, workspace)?;
    Ok(format!(
        "copied {source_str} to {dest_str}, checksum={}",
        result.checksum
    ))
}
