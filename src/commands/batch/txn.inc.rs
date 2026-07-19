/// True when every op has a distinct primary path key (safe for parallel mutate).
fn batch_targets_unique(ops: &[BatchOp]) -> bool {
    let mut seen = HashSet::with_capacity(ops.len());
    for op in ops {
        let key = op
            .target
            .as_deref()
            .or(op.path.as_deref())
            .or(op.source.as_deref())
            .unwrap_or("");
        if key.is_empty() {
            return false;
        }
        if !seen.insert(key.to_string()) {
            return false;
        }
        // Moves/copies also touch source — treat as second key when present.
        if matches!(op.op.as_str(), "move" | "copy") {
            if let Some(src) = op.source.as_deref().or(op.path.as_deref()) {
                if !seen.insert(src.to_string()) {
                    return false;
                }
            }
        }
    }
    true
}

/// Collect validated paths of existing files that operations will mutate.
fn collect_target_paths(ops: &[BatchOp], workspace: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(ops.len());
    for op in ops {
        for candidate in [op.target.as_ref(), op.path.as_ref(), op.source.as_ref()]
            .iter()
            .flatten()
        {
            let p = std::path::Path::new(candidate.as_str());
            if let Ok(validated) = crate::path_safety::validate_path(p, workspace) {
                if validated.is_file() {
                    paths.push(validated);
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Restore all pre-transaction backups to their original paths and remove
/// any files that were created during the transaction.
///
/// Returns `(restored, removed)` where `restored` is the count of pre-existing
/// files whose content was rolled back, and `removed` is the count of new
/// files that were created and then deleted.
fn rollback_transaction(
    backups: &[(PathBuf, PathBuf)],
    created_files: &[PathBuf],
    moves_to_reverse: &[(PathBuf, PathBuf)],
    workspace: &std::path::Path,
) -> Result<(u64, u64)> {
    let mut restored = 0u64;

    // Reverse moves first (target → source) before restoring backups.
    for (source, target) in moves_to_reverse.iter().rev() {
        if target.exists() && !source.exists() {
            std::fs::rename(target, source).with_context(|| {
                format!(
                    "cannot reverse move {} → {}",
                    target.display(),
                    source.display()
                )
            })?;
            restored += 1;
        }
    }

    for (original, backup) in backups {
        if backup.exists() {
            // Backups were produced by atomwrite under max_filesize; still
            // route through the fallible reader so OOM is recoverable.
            let content = crate::file_io::read_file_bytes(
                backup,
                crate::constants::DEFAULT_MAX_FILESIZE,
            )
            .with_context(|| format!("cannot read backup {}", backup.display()))?;
            let opts = AtomicWriteOptions::default();
            atomic_write(original, &content, &opts, workspace)
                .with_context(|| format!("cannot restore {}", original.display()))?;
            restored += 1;
        }
    }

    let mut removed = 0u64;
    for path in created_files {
        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("cannot remove created file {}", path.display()))?;
            removed += 1;
        }
    }

    Ok((restored, removed))
}

