// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file write pipeline: tempfile, fsync, rename, fsync directory.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::checksum;
use crate::ndjson_types::PlatformInfo;
use crate::platform;

/// Configuration for an atomic write operation.
pub struct AtomicWriteOptions {
    /// Whether to create a backup of the target before overwriting.
    pub backup: bool,
    /// Maximum number of backup copies to retain.
    pub retention: u8,
    /// Whether to restore the original file timestamps after writing.
    pub preserve_timestamps: bool,
}

impl Default for AtomicWriteOptions {
    fn default() -> Self {
        Self {
            backup: false,
            retention: 5,
            preserve_timestamps: true,
        }
    }
}

/// Result metadata returned after a successful atomic write.
pub struct WriteResult {
    /// Number of bytes written to the target file.
    pub bytes_written: u64,
    /// BLAKE3 checksum of the written content.
    pub checksum: String,
    /// BLAKE3 checksum of the file before overwriting, if it existed.
    pub checksum_before: Option<String>,
    /// Path to the backup file, if a backup was created.
    pub backup_path: Option<String>,
    /// Wall-clock time of the write operation in milliseconds.
    pub elapsed_ms: u64,
    /// Platform-specific fsync method names used.
    pub platform: PlatformInfo,
    /// Hard link count if the target had nlink > 1 (rename breaks hardlinks).
    pub hardlink_nlink: Option<u64>,
}

/// Write content atomically via tempfile, fsync, and rename.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the target path escapes the workspace.
/// Returns `AtomwriteError::Io` if creating, writing, or renaming the tempfile fails.
/// Returns `AtomwriteError::PermissionDenied` if the target directory is not writable.
/// Returns `AtomwriteError::DiskFull` if the filesystem runs out of space during write.
#[tracing::instrument(skip_all, fields(path = %target.display()))]
pub fn atomic_write(
    target: &Path,
    content: &[u8],
    opts: &AtomicWriteOptions,
    workspace: &Path,
) -> Result<WriteResult> {
    let start = Instant::now();

    // Step 1: validate path
    let target = crate::path_safety::validate_path(target, workspace)?;

    // Step 2: capture metadata of existing file
    let (checksum_before, original_meta) = if target.exists() {
        let meta =
            fs::metadata(&target).with_context(|| format!("cannot stat {}", target.display()))?;
        let hash = checksum::hash_file(&target, u64::MAX)?;
        (Some(hash), Some(meta))
    } else {
        (None, None)
    };

    // Step 2b: detect hardlinks that will be broken by rename
    #[cfg(unix)]
    let hardlink_nlink = if let Some(ref meta) = original_meta {
        use std::os::unix::fs::MetadataExt;
        let nlink = meta.nlink();
        if nlink > 1 {
            tracing::warn!(
                path = %target.display(),
                nlink = nlink,
                "atomic rename will break {} hardlink(s)",
                nlink - 1
            );
            Some(nlink)
        } else {
            None
        }
    } else {
        None
    };
    #[cfg(not(unix))]
    let hardlink_nlink: Option<u64> = None;

    // Step 3: capture timestamps for preservation
    let (mtime, atime) = if let Some(ref meta) = original_meta {
        (
            filetime::FileTime::from_last_modification_time(meta),
            filetime::FileTime::from_last_access_time(meta),
        )
    } else {
        let now = filetime::FileTime::now();
        (now, now)
    };

    // Step 4: create backup if requested
    let backup_path = if opts.backup && target.exists() {
        Some(create_backup(&target, opts.retention)?)
    } else {
        None
    };

    // Step 5: create parent directories
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create directories for {}", target.display()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(
                    parent,
                    fs::Permissions::from_mode(crate::constants::DIR_PERMISSIONS),
                );
            }
        }
    }

    // Step 6: create tempfile in same directory with identifiable prefix and restrictive permissions
    let parent = target.parent().unwrap_or(Path::new("."));
    let mut builder = tempfile::Builder::new();
    builder
        .prefix(crate::constants::TEMPFILE_PREFIX)
        .suffix(crate::constants::TEMPFILE_SUFFIX);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(
            crate::constants::TEMPFILE_PERMISSIONS,
        ));
    }
    let temp = builder
        .tempfile_in(parent)
        .with_context(|| format!("cannot create tempfile in {}", parent.display()))?;

    // Step 7: write content via BufWriter, extract File with into_inner
    {
        let mut writer = BufWriter::with_capacity(crate::constants::BUF_CAPACITY, temp.as_file());
        writer
            .write_all(content)
            .with_context(|| format!("write error for {}", target.display()))?;
        writer
            .flush()
            .with_context(|| format!("flush error for {}", target.display()))?;
        writer.into_inner().map_err(|e| {
            anyhow::anyhow!(
                "BufWriter into_inner error for {}: {}",
                target.display(),
                e.error()
            )
        })?;
    }

    // Step 8: fsync file
    platform::fsync_file(temp.as_file())
        .with_context(|| format!("fsync error for {}", target.display()))?;

    // Step 9: atomic rename
    #[cfg(windows)]
    {
        persist_with_retry(&temp, &target)?;
    }
    #[cfg(not(windows))]
    {
        temp.persist(&target)
            .inspect_err(|e| tracing::debug!(?e, path = %target.display(), "atomic rename failed"))
            .with_context(|| format!("rename error for {}", target.display()))?;
    }

    // Step 10: fsync parent directory (critical for durability)
    if let Some(parent) = target.parent() {
        if let Err(e) = platform::fsync_dir(parent) {
            tracing::warn!(
                path = %parent.display(),
                error = %e,
                "fsync_dir after persist failed"
            );
        }
    }

    // Step 11: restore permissions
    if let Some(ref meta) = original_meta {
        let _ = fs::set_permissions(&target, meta.permissions());
    }

    // Step 12: restore timestamps
    if opts.preserve_timestamps && original_meta.is_some() {
        let _ = platform::preserve_timestamps(&target, mtime, atime);
    }

    let checksum = checksum::hash_bytes(content);

    Ok(WriteResult {
        bytes_written: content.len() as u64,
        checksum,
        checksum_before,
        backup_path: backup_path.map(|p| p.display().to_string()),
        elapsed_ms: start.elapsed().as_millis() as u64,
        platform: PlatformInfo {
            fsync: platform::platform_fsync_name(),
            dir_fsync: platform::platform_dir_fsync_name(),
        },
        hardlink_nlink,
    })
}

/// Create a timestamped backup of the target file and prune old backups.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if copying the file or creating the backup fails.
#[tracing::instrument(skip_all, fields(path = %target.display(), retention))]
pub(crate) fn create_backup(target: &Path, retention: u8) -> Result<std::path::PathBuf> {
    let now = utc_timestamp_formatted();
    // file_name() returns None only for root "/" — empty string is safe for backup naming
    let filename = target.file_name().unwrap_or_default().to_string_lossy();
    let backup_name = format!("{filename}.bak.{now}");
    let backup_path = target.with_file_name(&backup_name);

    fs::copy(target, &backup_path)
        .with_context(|| format!("cannot create backup at {}", backup_path.display()))?;
    let backup_file = fs::File::open(&backup_path)
        .with_context(|| format!("cannot open backup for fsync: {}", backup_path.display()))?;
    platform::fsync_file(&backup_file)
        .with_context(|| format!("cannot fsync backup: {}", backup_path.display()))?;

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
        cleanup_old_backups(target, retention);
    }

    Ok(backup_path)
}

fn cleanup_old_backups(target: &Path, retention: u8) {
    let parent = match target.parent() {
        Some(p) => p,
        None => return,
    };
    let filename = match target.file_name() {
        Some(f) => f.to_string_lossy(),
        None => return,
    };
    let prefix = format!("{filename}.bak.");

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

fn utc_timestamp_formatted() -> String {
    use std::time::SystemTime;
    // duration_since fails only if system clock precedes UNIX epoch — defaults to 1970-01-01
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (year, month, day, hour, min, sec) = epoch_to_utc(secs);
    format!("{year:04}{month:02}{day:02}_{hour:02}{min:02}{sec:02}")
}

/// Return the current UTC time as an RFC 3339 string (e.g. `2024-01-15T14:30:22Z`).
pub fn rfc3339_now() -> String {
    use std::time::SystemTime;
    // duration_since fails only if system clock precedes UNIX epoch — defaults to 1970-01-01
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d, h, min, sec) = epoch_to_utc(secs);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{sec:02}Z")
}

pub(crate) fn epoch_to_utc(epoch: u64) -> (u64, u64, u64, u64, u64, u64) {
    let sec_of_day = epoch % 86400;
    let hour = sec_of_day / 3600;
    let min = (sec_of_day % 3600) / 60;
    let sec = sec_of_day % 60;

    let mut days = (epoch / 86400) as i64;
    days += 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = (days - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y as u64, m, d, hour, min, sec)
}

#[cfg(windows)]
use tempfile::NamedTempFile;

#[cfg(windows)]
fn persist_with_retry(temp: &NamedTempFile, target: &Path) -> Result<()> {
    let delays = [100, 200, 400];
    for delay_ms in &delays {
        match temp.persist(target) {
            Ok(_) => return Ok(()),
            Err(e) => {
                if e.error.kind() == std::io::ErrorKind::PermissionDenied {
                    std::thread::sleep(std::time::Duration::from_millis(*delay_ms));
                    continue;
                }
                return Err(anyhow::anyhow!(
                    "rename error for {}: {}",
                    target.display(),
                    e.error
                ));
            }
        }
    }
    Err(AtomwriteError::PermissionDenied {
        path: target.to_path_buf(),
    }
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_to_utc_epoch_zero() {
        assert_eq!(epoch_to_utc(0), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn epoch_to_utc_known_date() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(epoch_to_utc(1704067200), (2024, 1, 1, 0, 0, 0));
    }

    #[test]
    fn atomic_write_options_default_values() {
        let opts = AtomicWriteOptions::default();
        assert!(!opts.backup);
        assert_eq!(opts.retention, 5);
        assert!(opts.preserve_timestamps);
    }

    #[test]
    fn create_backup_and_retention() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();

        for _ in 0..7 {
            create_backup(&file, 5).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("test.txt.bak."))
            })
            .collect();

        assert!(
            backups.len() <= 5,
            "retention should keep at most 5 backups, got {}",
            backups.len()
        );
    }
}
