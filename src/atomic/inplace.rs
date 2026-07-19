// SPDX-License-Identifier: MIT OR Apache-2.0

//! In-place write, EXDEV fallback, and timestamped backups.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::platform;

use super::time_fmt::utc_timestamp_formatted;

/// EXDEV fallback: write the in-memory payload already produced for the
/// tempfile pipeline. Reuses `content` (no second heap buffer / no re-read
/// of the temp file) and performs a single truncate+write+fsync.
#[cfg(unix)]
pub(crate) fn copy_tempfile_to_target(_temp: &std::fs::File, target: &Path, content: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut target_file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(target)
        .with_context(|| format!("cannot open target for copy fallback: {}", target.display()))?;
    target_file.write_all(content).with_context(|| {
        format!(
            "cannot write target for copy fallback: {}",
            target.display()
        )
    })?;
    platform::fsync_file(&target_file).ok();
    let _ = target_file;

    // Remove the tempfile on disk (its path was inside the parent dir).
    // persist() left it on disk with a .tmp.* name; we don't have a handle
    // to it here, so we rely on the caller to clean up via the
    // tempfile-in-parent pattern. Cleanup is best-effort: ignore errors.
    if let Some(parent) = target.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if let Some(name) = name.to_str() {
                    if name.starts_with(crate::constants::TEMPFILE_PREFIX) {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
    Ok(())
}

/// Write in-place to preserve the existing inode and hardlinks (G55/G114).
///
/// Uses `ftruncate(0)` + `write_all` + `sync_data` on the existing fd.
/// NOT atomic against a crash between truncate and write — for full crash
/// recovery, use `WriteStrategy::CopyBack` with the journal sidecar.
pub(crate) fn write_inplace_path(target: &Path, content: &[u8]) -> Result<bool> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(false)
        .open(target)
        .with_context(|| {
            format!(
                "cannot open target for in-place write: {}",
                target.display()
            )
        })?;
    file.set_len(0)
        .with_context(|| format!("ftruncate failed for {}", target.display()))?;
    file.write_all(content)
        .with_context(|| format!("in-place write failed for {}", target.display()))?;
    file.flush()
        .with_context(|| format!("in-place flush failed for {}", target.display()))?;
    let _ = file.sync_data();
    Ok(false)
}

/// Create a timestamped backup of the target file and prune old backups.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if copying the file or creating the backup fails.
#[tracing::instrument(skip_all, fields(path = %target.display(), retention))]
pub(crate) fn create_backup(target: &Path, retention: u8) -> Result<std::path::PathBuf> {
    create_backup_in(target, retention, None)
}

/// Create a timestamped backup, optionally in a custom output directory.
///
/// When `output_dir` is `Some`, the backup is placed in that directory instead
/// of alongside the source file. The directory is created if it does not exist.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if copying, creating the directory, or the
/// backup itself fails.
#[tracing::instrument(skip_all, fields(path = %target.display(), retention, output_dir))]
pub(crate) fn create_backup_in(
    target: &Path,
    retention: u8,
    output_dir: Option<&Path>,
) -> Result<std::path::PathBuf> {
    let now = utc_timestamp_formatted();
    // file_name() returns None only for root "/" — empty string is safe for backup naming
    let filename = target.file_name().unwrap_or_default().to_string_lossy();
    let backup_name = format!("{filename}.bak.{now}");

    let backup_path = match output_dir {
        Some(dir) => {
            if !dir.exists() {
                fs::create_dir_all(dir).with_context(|| {
                    format!("cannot create backup output dir {}", dir.display())
                })?;
            }
            dir.join(&backup_name)
        }
        None => target.with_file_name(&backup_name),
    };

    // v0.1.29 P2-3 / audit fix: prefer reflink → copy. NEVER hardlink.
    // Hardlink shares the inode with the live file. A later write that
    // auto-switches to InPlace (nlink > 1) mutates the backup in place and
    // makes `rollback` a silent no-op (checksum_before == checksum_after).
    // G64: reflink is O(1) CoW on APFS/btrfs/XFS; falls back to full copy.
    //
    // Remove the existing backup file if any: reflink_or_copy refuses to
    // overwrite, and the test harness can produce timestamp collisions
    // (second-level resolution). Cleanup is best-effort.
    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }
    reflink_copy::reflink_or_copy(target, &backup_path)
        .with_context(|| format!("cannot create backup at {}", backup_path.display()))?;
    let backup_file = fs::File::open(&backup_path)
        .with_context(|| format!("cannot open backup for fsync: {}", backup_path.display()))?;
    // Best-effort fsync: backup file already exists on disk via fs::copy.
    // On Windows %TEMP%, antivirus products can transiently hold a read handle
    // causing FlushFileBuffers to fail with ERROR_ACCESS_DENIED. We log a
    // warning and continue because the user-visible operation (creating a
    // backup) has already succeeded; the worst case is a missing durability
    // flush for the backup metadata, which is non-fatal.
    platform::fsync_file_best_effort(&backup_file);

    if let Some(parent) = backup_path.parent() {
        if let Err(e) = platform::fsync_dir(parent) {
            tracing::warn!(
                path = %parent.display(),
                error = %e,
                "fsync_dir after backup failed"
            );
        }
    }

    if retention > 0 {
        // Prune old backups in the same directory as the new one.
        // Pass the source filename (without .bak.<timestamp> suffix) so the
        // prefix matcher correctly identifies peer backups.
        cleanup_old_backups_in(
            backup_path.parent().unwrap_or_else(|| Path::new(".")),
            &filename,
            retention,
        );
    }

    Ok(backup_path)
}

/// Quietly delete a backup file after a successful atomic write.
///
/// v0.1.21 GAP-014 v2: by default, backups are transient and removed
/// after the write commits. This function is idempotent — `NotFound`
/// is mapped to `Ok(())` so a double-delete or pre-cleaned path is
/// silent. Any other I/O error is logged at WARN level but does NOT
/// propagate: the user's write already succeeded, and propagating a
/// cleanup failure would mask the success path.
pub(crate) fn delete_backup_quietly(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {
            tracing::debug!(path = %path.display(), "backup deleted after successful write");
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(
                path = %path.display(),
                "backup already gone (NotFound) — nothing to delete"
            );
        }
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to delete backup after successful write — backup retained"
            );
        }
    }
}

/// Prune old backups that share the given `prefix` in the given directory.
pub(crate) fn cleanup_old_backups_in(parent: &Path, prefix_name: &str, retention: u8) {
    let prefix = format!("{prefix_name}.bak.");

    let mut backups: Vec<std::path::PathBuf> = match fs::read_dir(parent) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&prefix))
            })
            .collect(),
        Err(_) => return,
    };

    if backups.len() <= retention as usize {
        return;
    }

    backups.sort();
    let to_remove = backups.len() - retention as usize;
    for old in &backups[..to_remove] {
        let _ = fs::remove_file(old);
    }
}

