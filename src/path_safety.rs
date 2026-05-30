// SPDX-License-Identifier: MIT OR Apache-2.0

//! Workspace path jail validation and symlink safety checks.

use std::path::{Component, Path, PathBuf};

use anyhow::Result;

use crate::error::AtomwriteError;

/// Validate that a path is inside the workspace and is not a symlink.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::InvalidInput` if the path contains a null byte.
/// Returns `AtomwriteError::SymlinkBlocked` if the path is a symbolic link.
/// Returns `AtomwriteError::FifoDetected` if the path is a FIFO.
/// Returns `AtomwriteError::DeviceFile` if the path is a device file.
pub fn validate_path(path: &Path, workspace: &Path) -> Result<PathBuf> {
    validate_path_with_symlink(path, workspace, false)
}

/// Validate a path with configurable symlink policy.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::InvalidInput` if the path contains a null byte.
/// Returns `AtomwriteError::SymlinkBlocked` if the path is a symbolic link and symlinks are not followed.
/// Returns `AtomwriteError::FifoDetected` if the path is a FIFO.
/// Returns `AtomwriteError::DeviceFile` if the path is a device file.
pub fn validate_path_with_symlink(
    path: &Path,
    workspace: &Path,
    follow_symlinks: bool,
) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    if path_str.contains('\0') {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("path contains null byte: {}", path.display()),
        }
        .into());
    }

    if cfg!(windows) {
        check_reserved_windows(&path_str)?;
    }

    let resolved = soft_canonicalize(path);
    let workspace_resolved = soft_canonicalize(workspace);

    if !resolved.starts_with(&workspace_resolved) {
        return Err(AtomwriteError::WorkspaceJail {
            path: resolved.clone(),
        }
        .into());
    }

    if !follow_symlinks && resolved.exists() {
        check_symlink(&resolved)?;
    }

    #[cfg(unix)]
    if resolved.exists() {
        check_special_file(&resolved)?;
    }

    Ok(resolved)
}

/// Return an error if the path is a symbolic link.
///
/// # Errors
///
/// Returns `AtomwriteError::SymlinkBlocked` if the path is a symbolic link.
pub fn check_symlink(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path);
    if let Ok(meta) = metadata {
        if meta.file_type().is_symlink() {
            return Err(AtomwriteError::SymlinkBlocked {
                path: path.to_path_buf(),
            }
            .into());
        }
    }
    Ok(())
}

/// Resolve `.` and `..` components without touching the filesystem.
pub fn soft_canonicalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            other => result.push(other),
        }
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    result
}

/// Reject FIFOs and device files that would hang or corrupt the atomic pipeline.
#[cfg(unix)]
fn check_special_file(path: &Path) -> Result<()> {
    use std::os::unix::fs::FileTypeExt;
    let meta = std::fs::symlink_metadata(path);
    if let Ok(m) = meta {
        let ft = m.file_type();
        if ft.is_fifo() {
            return Err(AtomwriteError::FifoDetected {
                path: path.to_path_buf(),
            }
            .into());
        }
        if ft.is_block_device() || ft.is_char_device() {
            return Err(AtomwriteError::DeviceFile {
                path: path.to_path_buf(),
            }
            .into());
        }
    }
    Ok(())
}

fn check_reserved_windows(name: &str) -> Result<()> {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_uppercase();

    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    if RESERVED.contains(&stem.as_str()) {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("reserved Windows filename: {stem}"),
        }
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_canonicalize_resolves_dotdot() {
        let result = soft_canonicalize(Path::new("/home/user/../other/./file.txt"));
        assert_eq!(result, PathBuf::from("/home/other/file.txt"));
    }

    #[test]
    fn soft_canonicalize_empty_becomes_dot() {
        let result = soft_canonicalize(Path::new(""));
        assert_eq!(result, PathBuf::from("."));
    }

    #[test]
    fn validate_rejects_null_byte() {
        let result = validate_path(Path::new("foo\0bar"), Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[test]
    fn validate_rejects_outside_workspace() {
        let result = validate_path(Path::new("/etc/passwd"), Path::new("/home/user"));
        assert!(result.is_err());
    }

    #[test]
    fn validate_accepts_inside_workspace() {
        let result = validate_path(Path::new("/tmp/test.txt"), Path::new("/tmp"));
        assert!(result.is_ok());
    }
}
