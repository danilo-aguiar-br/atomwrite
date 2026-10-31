// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tempfile → fsync → rename path for atomic writes.

#[cfg(unix)]
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::Result;

use crate::platform;

/// Turn an I/O failure at `target` into the typed error, keeping its exit code.
///
/// `with_context` would lift a `std::io::Error` into `anyhow::Error` with no
/// type, and the catch-all in `main` can only read that as `INTERNAL_ERROR`
/// (exit 255). Classifying here keeps `PermissionDenied` (13) and `Io` (74)
/// distinguishable, which is what the exit contract promises.
fn io_at(target: &Path, what: &str) -> impl Fn(std::io::Error) -> anyhow::Error {
    let path = target.to_path_buf();
    let what = what.to_string();
    move |source| {
        let reason = format!("{what} for {}: {source}", path.display());
        crate::error::AtomwriteError::from_io_at(&path, std::io::Error::new(source.kind(), reason))
            .into()
    }
}

#[cfg(unix)]
use super::inplace::copy_content_to_target;
#[cfg(windows)]
use super::persist_retry::persist_with_retry;

/// Write `content` to `target` through tempfile, fsync and rename.
///
/// Returns `(fell_back, method)`: `true` only on the EXDEV copy fallback, and
/// `method` names the rename primitive that actually ran.
///
/// # Errors
///
/// Returns an I/O error when the tempfile cannot be created in the target's
/// directory, when a chunk write or the flush fails, when the durability fsync
/// fails, or when the final rename fails. Returns
/// [`crate::error::AtomwriteError::ExdevFallbackDisabled`] (exit 91) when the
/// tempfile and the target land on different filesystems and `strict_atomic`
/// is set. Returns [`crate::signal::cancelled_error`] (130/143/124) when a hard
/// shutdown arrives between chunks.
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
        .map_err(io_at(parent, "cannot create tempfile"))?;

    // Step 7: write content in 64 KiB chunks with cooperative cancel (v0.1.29 P0-4).
    {
        let mut writer = BufWriter::with_capacity(crate::constants::BUF_CAPACITY, temp.as_file());
        // A-021: chunk size is BUF_CAPACITY (not a second magic 64 KiB).
        let chunk = crate::constants::BUF_CAPACITY;
        let mut offset = 0usize;
        while offset < content.len() {
            // A closed reader (SIGPIPE) does not abort a write already under way:
            // it is local and short, and aborting it would leave its journal behind.
            // SIGINT / SIGTERM / SIGHUP / the deadline still abort it here.
            if crate::signal::is_global_hard_shutdown() {
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
                .map_err(io_at(target, "write error"))?;
            offset = end;
        }
        writer.flush().map_err(io_at(target, "flush error"))?;
        writer.into_inner().map_err(|e| {
            io_at(target, "BufWriter into_inner error")(std::io::Error::new(
                e.error().kind(),
                format!(
                    "BufWriter into_inner error for {}: {}",
                    target.display(),
                    e.error()
                ),
            ))
        })?;
    }

    // Step 8: fsync file (respect durability policy)
    platform::fsync_file_with_durability(temp.as_file(), durability)
        .map_err(io_at(target, "fsync error"))?;

    // Step 9: atomic rename. The tempfile is created in the target's own
    // directory, so a cross-device rename cannot originate here; EXDEV stays
    // reachable through the Windows persist retry, which classifies its own
    // error instead of letting `anyhow` drop the type.
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
                                reason: "the temp file and the target are on different filesystems"
                                    .into(),
                            }
                            .into());
                        }
                        tracing::warn!(
                            path = %target.display(),
                            "EXDEV detected, falling back to copy + fsync + cleanup"
                        );
                        // Held until the end of the arm: its drop removes exactly this tempfile.
                        let _recovered = e.file;
                        copy_content_to_target(target, content)?;
                        return Ok((true, "copy_exdev"));
                    }
                }
                return Err(io_at(target, "rename error")(e.error));
            }
        }
    }
}
