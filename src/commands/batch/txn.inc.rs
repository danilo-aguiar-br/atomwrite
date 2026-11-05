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

/// Paths the transaction must snapshot before its first mutation.
///
/// The source of `move` and `copy` is not rewritten: `move` reverses by
/// renaming the destination back, and `copy` never touches the source, so
/// copying it would write bytes no rollback path reads. `hash` mutates
/// nothing, and an unknown operation gets no snapshot until it declares one.
fn collect_snapshot_paths(ops: &[BatchOp], workspace: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(ops.len());
    for op in ops {
        let candidate = match op.op.as_str() {
            "write" | "replace" | "edit" | "delete" => {
                op.target.as_ref().or(op.path.as_ref())
            }
            "move" | "copy" => op.target.as_ref(),
            _ => None,
        };
        let Some(candidate) = candidate else {
            continue;
        };
        let p = std::path::Path::new(candidate.as_str());
        if let Ok(validated) = crate::path_safety::validate_path(p, workspace) {
            if validated.is_file() {
                paths.push(validated);
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Paths an operation removes or overwrites, whose snapshot is the way back.
///
/// A transaction snapshots every target before the first mutation. A file the
/// run deletes, moves or copies into keeps its snapshot because the old content
/// lives nowhere else. A rewrite keeps it only when `--keep-backup` asked for
/// the copy; without it the snapshot is a guard and becomes residue once the
/// run commits. `--no-backup` forbids every copy.
///
/// `move` contributes only its destination: the source entry is gone, but its
/// content lives on at that destination, so the snapshot of the source would
/// duplicate bytes that are still readable.
fn destructive_paths(
    ops: &[BatchOp],
    workspace: &std::path::Path,
    keep_backup: bool,
    no_backup: bool,
) -> HashSet<PathBuf> {
    let mut paths = HashSet::new();
    for op in ops {
        let candidates: [Option<&String>; 2] = match op.op.as_str() {
            "delete" => [op.target.as_ref(), op.path.as_ref()],
            "move" | "copy" => [op.target.as_ref(), None],
            "write" | "replace" | "edit" if keep_backup && !no_backup => {
                [op.target.as_ref().or(op.path.as_ref()), None]
            }
            _ => continue,
        };
        for candidate in candidates.into_iter().flatten() {
            if let Ok(validated) = crate::path_safety::validate_path(
                std::path::Path::new(candidate.as_str()),
                workspace,
            ) {
                paths.insert(validated);
            }
        }
    }
    paths
}

/// What a reversal did, including the part it managed before stopping.
#[derive(Debug, Default)]
pub(crate) struct RollbackOutcome {
    /// Pre-existing files whose content came back.
    pub restored: u64,
    /// Files the transaction created and the reversal removed.
    pub removed: u64,
    /// Path of the file the reversal could not restore, when one failed.
    pub failed_path: Option<String>,
    /// Diagnostic of that failure, empty when every file came back.
    pub reason: String,
}

/// Restore all pre-transaction backups to their original paths and remove
/// any files that were created during the transaction.
///
/// The counts travel even when the reversal stops halfway, because the caller
/// publishes them and a zero would deny the files that did come back.
fn rollback_transaction(
    backups: &[(PathBuf, PathBuf)],
    created_files: &[PathBuf],
    moves_to_reverse: &[(PathBuf, PathBuf)],
    workspace: &std::path::Path,
) -> RollbackOutcome {
    let mut outcome = RollbackOutcome::default();

    // Reverse moves first (target → source) before restoring backups.
    for (source, target) in moves_to_reverse.iter().rev() {
        if target.exists() && !source.exists() {
            if let Err(error) = std::fs::rename(target, source) {
                outcome.failed_path = Some(target.display().to_string());
                outcome.reason = format!(
                    "cannot reverse move {} → {}: {error}",
                    target.display(),
                    source.display()
                );
                return outcome;
            }
            outcome.restored += 1;
        }
    }

    for (original, backup) in backups {
        if !backup.exists() {
            continue;
        }
        let restored = (|| -> Result<()> {
            // Backups were produced by atomwrite under max_filesize; still
            // route through the fallible reader so OOM is recoverable.
            let content = crate::file_io::read_file_bytes(
                backup,
                crate::constants::DEFAULT_MAX_FILESIZE,
            )
            .with_context(|| format!("cannot read backup {}", backup.display()))?;
            let opts = AtomicWriteOptions::default();
            // The transaction still holds the lock it took before the first
            // operation, and that lock is on the inode the operation replaced.
            // Passing the content digest swaps the inode comparison for a byte
            // comparison, so a file the transaction itself rewrote is restored
            // instead of reported as drift.
            let expected = crate::checksum::hash_file(original, u64::MAX).ok();
            atomic_write_based(original, &content, &opts, workspace, expected.as_deref())
                .with_context(|| format!("cannot restore {}", original.display()))?;
            Ok(())
        })();
        if let Err(error) = restored {
            outcome.failed_path = Some(original.display().to_string());
            outcome.reason = format!("{error:#}");
            return outcome;
        }
        outcome.restored += 1;
    }

    for path in created_files {
        if !path.exists() {
            continue;
        }
        if let Err(error) = std::fs::remove_file(path) {
            outcome.failed_path = Some(path.display().to_string());
            outcome.reason = format!("cannot remove created file {}: {error}", path.display());
            return outcome;
        }
        outcome.removed += 1;
    }

    outcome
}

