// SPDX-License-Identifier: MIT OR Apache-2.0

//! Extended attribute (xattr) preservation for atomic writes.
//!
//! On Unix, `rename(2)` replaces the directory entry but extended attributes
//! are stored per-inode, so a naive atomic write destroys all xattrs that the
//! target previously had. This module saves and restores xattrs to preserve
//! macOS `com.apple.quarantine` (Gatekeeper), Linux `security.selinux` (SELinux
//! context), `security.capability` (POSIX capabilities), and arbitrary
//! `user.*` attributes.
//!
//! NO-OP on Windows (xattrs do not exist on NTFS).
//!
//! All operations are best-effort: a failure to read or write an individual
//! xattr is logged via `tracing::warn!` and the caller continues, because most
//! filesystems silently fail to read xattrs for unsupported types (FAT32,
//! tmpfs, overlayfs in user namespace) and we do not want to abort a successful
//! write just because one xattr is unreadable.

use std::path::Path;

/// A single extended attribute: `(name, value)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xattr {
    /// Attribute name (e.g. `"user.checksum"`, `"com.apple.quarantine"`).
    pub name: String,
    /// Attribute value (may be empty).
    pub value: Vec<u8>,
}

/// Read all extended attributes from `path`.
///
/// Returns an empty `Vec` on Windows, or when the filesystem does not support
/// xattrs (FAT32, tmpfs, overlayfs in user namespace), or when the caller
/// lacks permission to read the attributes.
///
/// # Errors
///
/// Returns an error only for unexpected I/O failures that are not the well-
/// known "not supported" / "operation not permitted" cases. The most common
/// failure modes are logged as warnings and return an empty `Vec`.
#[cfg(unix)]
pub fn save_xattrs(path: &Path) -> std::io::Result<Vec<Xattr>> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    // Skip xattr capture for symlinks — xattr on a symlink target modifies
    // the symlink's own xattrs (where supported), not the target's.
    if meta.file_type().is_symlink() {
        return Ok(Vec::new());
    }

    let names: Vec<_> = match xattr::list(path) {
        Ok(n) => n.collect(),
        Err(e) => {
            // ENOTSUP / EOPNOTSUPP / EPERM: filesystem or namespace does not
            // support xattrs — this is the normal case on FAT32, tmpfs, and
            // user-namespace overlayfs. Log and continue.
            tracing::debug!(path = %path.display(), error = %e, "xattr::list failed (unsupported fs?)");
            return Ok(Vec::new());
        }
    };

    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let name_str = match name.to_str() {
            Some(s) => s.to_owned(),
            None => {
                tracing::warn!(?name, "xattr name is not valid UTF-8; skipping");
                continue;
            }
        };
        match xattr::get(path, &name_str) {
            Ok(Some(value)) => out.push(Xattr {
                name: name_str,
                value,
            }),
            Ok(None) => {
                tracing::debug!(name = %name_str, "xattr::get returned None (race?)");
            }
            Err(e) => {
                tracing::warn!(name = %name_str, error = %e, "xattr::get failed; skipping");
            }
        }
    }
    Ok(out)
}

/// Read all extended attributes from `path` (Windows stub).
#[cfg(not(unix))]
pub fn save_xattrs(_path: &Path) -> std::io::Result<Vec<Xattr>> {
    Ok(Vec::new())
}

/// Restore extended attributes previously captured by [`save_xattrs`].
///
/// All writes are best-effort. A failure to write an individual xattr is
/// logged via `tracing::warn!` and the loop continues. macOS `com.apple.*`
/// attributes typically require `EPERM` handling because Gatekeeper may
/// have set special permissions; we treat that as a warning, not an error.
///
/// # Errors
///
/// Returns an error only if `xattr::set` itself fails in an unexpected way
/// that is not the well-known "not supported" / "operation not permitted"
/// cases. Individual xattr failures are logged, not propagated.
#[cfg(unix)]
pub fn restore_xattrs(path: &Path, attrs: &[Xattr]) -> std::io::Result<u32> {
    if attrs.is_empty() {
        return Ok(0);
    }
    let mut restored = 0u32;
    for attr in attrs {
        match xattr::set(path, &attr.name, &attr.value) {
            Ok(()) => {
                restored += 1;
            }
            Err(e) => {
                // EOPNOTSUPP / ENOTSUP: filesystem does not support this
                // xattr (e.g. security.selinux on a non-SELinux fs).
                // EPERM: macOS com.apple.* attributes have special
                // permissions. ENOENT: target file disappeared mid-restore
                // (very rare). All are non-fatal.
                tracing::warn!(
                    path = %path.display(),
                    name = %attr.name,
                    error = %e,
                    "xattr::set failed; continuing"
                );
            }
        }
    }
    Ok(restored)
}

/// Restore extended attributes previously captured by [`save_xattrs`]
/// (Windows stub).
#[cfg(not(unix))]
pub fn restore_xattrs(_path: &Path, _attrs: &[Xattr]) -> std::io::Result<u32> {
    Ok(0)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn save_xattrs_returns_empty_on_nonexistent_path() {
        let result = save_xattrs(Path::new("/nonexistent/atomwrite/xattr/test")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn save_and_restore_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("xattr_test.txt");
        let mut f = std::fs::File::create(&file).unwrap();
        f.write_all(b"hello world").unwrap();
        drop(f);

        // Set a user.* xattr
        xattr::set(&file, "user.atomwrite_test", b"test_value_42").unwrap();

        // Read it back
        let saved = save_xattrs(&file).unwrap();
        let entry = saved
            .iter()
            .find(|x| x.name == "user.atomwrite_test")
            .expect("user.atomwrite_test must be in save result");
        assert_eq!(entry.value, b"test_value_42");

        // Restore (re-write the same xattr)
        let restored = restore_xattrs(&file, &saved).unwrap();
        assert!(restored >= 1, "at least one xattr should be restored");

        // Verify still readable
        let again = xattr::get(&file, "user.atomwrite_test").unwrap();
        assert_eq!(again, Some(b"test_value_42".to_vec()));
    }

    #[test]
    fn save_xattrs_empty_for_tmpfs_file() {
        // tmpfs and procfs do not support user.* xattrs but list() may return
        // empty or error. Either way, save_xattrs must not panic.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("plain.txt");
        std::fs::write(&file, b"plain text").unwrap();
        let _ = save_xattrs(&file).unwrap();
    }
}
