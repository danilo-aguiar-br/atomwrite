// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tempfile → fsync → rename path for atomic writes.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};

use crate::platform;

#[cfg(windows)]
use super::persist_retry::persist_with_retry;
#[cfg(unix)]
use super::inplace::copy_tempfile_to_target;

pub(crate) fn write_rename_path(
    target: &Path,
    content: &[u8],
    #[cfg_attr(not(unix), allow(unused_variables))] strict_atomic: bool,
    durability: crate::platform::Durability,
) -> Result<(bool, &'static str)> {
    // Step 6: create tempfile in same directory
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

    // Step 7: write content in 64 KiB chunks with cooperative cancel (v0.1.29 P0-4).
    {
        let mut writer = BufWriter::with_capacity(crate::constants::BUF_CAPACITY, temp.as_file());
        // A-021: chunk size is BUF_CAPACITY (not a second magic 64 KiB).
        let chunk = crate::constants::BUF_CAPACITY;
        let mut offset = 0usize;
        while offset < content.len() {
            if crate::signal::is_global_shutdown() {
                // Drop tempfile by not persisting — NamedTempFile cleans on drop.
                // Exit code follows the live signal/timeout (130/143/124), not a
                // hard-coded SIGTERM — see `signal::cancelled_error`.
                return Err(crate::signal::cancelled_error(format!(
                    "atomic write cancelled for {}",
                    target.display()
                ))
                .into());
            }
            let end = (offset + chunk).min(content.len());
            writer
                .write_all(&content[offset..end])
                .with_context(|| format!("write error for {}", target.display()))?;
            offset = end;
        }
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

    // Step 8: fsync file (respect durability policy)
    platform::fsync_file_with_durability(temp.as_file(), durability)
        .with_context(|| format!("fsync error for {}", target.display()))?;

    // Step 9: atomic rename with EXDEV fallback
    #[cfg(windows)]
    {
        persist_with_retry(temp, target)?;
        return Ok((false, "MoveFileEx"));
    }
    #[cfg(not(windows))]
    {
        match temp.persist(target) {
            Ok(_) => Ok((false, platform::platform_rename_method())),
            Err(e) => {
                #[cfg(unix)]
                {
                    if e.error.raw_os_error() == Some(libc::EXDEV) {
                        if strict_atomic {
                            return Err(crate::error::AtomwriteError::ExdevFallbackDisabled {
                                path: target.to_path_buf(),
                            }
                            .into());
                        }
                        tracing::warn!(
                            path = %target.display(),
                            "EXDEV detected, falling back to copy + fsync + cleanup"
                        );
                        let recovered = e.file;
                        copy_tempfile_to_target(recovered.as_file(), target, content)?;
                        return Ok((true, "copy_exdev"));
                    }
                }
                return Err(e.error)
                    .with_context(|| format!("rename error for {}", target.display()));
            }
        }
    }
}
