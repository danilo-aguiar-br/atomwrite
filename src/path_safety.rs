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

    // Always validate Windows-hostile names so agents cannot stage bad paths
    // on Linux for later Windows consumers (and so unit tests cover the rules).
    check_windows_path_rules(path)?;

    let effective_path = if path.is_relative() {
        workspace.join(path)
    } else {
        path.to_path_buf()
    };
    let resolved = normalize_path_nfc(&soft_canonicalize(&effective_path));
    let workspace_resolved = normalize_path_nfc(&soft_canonicalize(workspace));

    if !resolved.starts_with(&workspace_resolved) {
        return Err(AtomwriteError::WorkspaceJail {
            path: resolved,
            workspace: workspace_resolved,
        }
        .into());
    }

    if !follow_symlinks {
        let real_resolved = canonicalize_existing_prefix(&resolved);
        let real_workspace = workspace
            .canonicalize()
            .unwrap_or_else(|_| workspace_resolved.clone());
        let real_workspace = normalize_path_nfc(&real_workspace);
        if !real_resolved.starts_with(&real_workspace) {
            return Err(AtomwriteError::WorkspaceJail {
                path: real_resolved,
                workspace: real_workspace,
            }
            .into());
        }

        if resolved.exists() {
            check_symlink(&resolved)?;
        }
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

fn normalize_path_nfc(path: &Path) -> PathBuf {
    use unicode_normalization::UnicodeNormalization;
    let s = path.to_string_lossy();
    let normalized: String = s.nfc().collect();
    PathBuf::from(normalized)
}

/// Canonicalize the longest existing prefix of a path, resolving symlinks.
///
/// Walks from the full path upward until an existing ancestor is found,
/// canonicalizes it (resolving all symlinks), then re-appends the
/// non-existent tail. This catches symlink-directory escapes where an
/// intermediate component is a symlink pointing outside the workspace.
fn canonicalize_existing_prefix(path: &Path) -> PathBuf {
    let mut existing = path.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();

    loop {
        if existing.exists() {
            match existing.canonicalize() {
                Ok(canon) => {
                    let mut result = canon;
                    for component in tail.into_iter().rev() {
                        result.push(component);
                    }
                    return normalize_path_nfc(&result);
                }
                Err(_) => return normalize_path_nfc(path),
            }
        }
        match existing.file_name() {
            Some(name) => {
                tail.push(name.to_owned());
                existing.pop();
            }
            None => return normalize_path_nfc(path),
        }
    }
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

/// Windows reserved device names (case-insensitive), including COM0/LPT0.
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Characters illegal in Windows file *names* (not path separators / drive colon).
#[cfg(any(windows, test))]
const WINDOWS_INVALID_NAME_CHARS: &[char] = &['<', '>', '"', '|', '?', '*'];

/// Validate path components for Windows portability.
///
/// - **All hosts:** reserved device names (`CON`, `NUL`, `COM0`…`COM9`, …)
///   so agent trees stay portable across OSes.
/// - **Windows only:** illegal filename characters and trailing space/dot
///   (Unix allows `:` and other glyphs that Win32 rejects).
fn check_windows_path_rules(path: &Path) -> Result<()> {
    for component in path.components() {
        let Component::Normal(os) = component else {
            continue;
        };
        let Some(name) = os.to_str() else {
            continue;
        };
        // Drive letter components are never `Normal` on Windows (`Prefix`/`RootDir`).
        check_reserved_windows_name(name)?;
        #[cfg(windows)]
        {
            check_windows_filename_strict(name)?;
        }
    }
    Ok(())
}

/// Reserved device stems (case-insensitive). `NUL.txt` is still `NUL`.
fn check_reserved_windows_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Ok(());
    }
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
        .to_uppercase();

    if WINDOWS_RESERVED.contains(&stem.as_str()) {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("reserved Windows filename: {stem}"),
        }
        .into());
    }
    Ok(())
}

/// Strict Win32 filename rules (illegal chars + trailing space/dot).
#[cfg(windows)]
fn check_windows_filename_strict(name: &str) -> Result<()> {
    if name.ends_with(' ') || name.ends_with('.') {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("Windows-illegal trailing space or period in filename: {name:?}"),
        }
        .into());
    }
    for ch in name.chars() {
        if WINDOWS_INVALID_NAME_CHARS.contains(&ch) || ch == ':' || (ch as u32) < 32 {
            return Err(AtomwriteError::InvalidInput {
                reason: format!("Windows-illegal character {ch:?} in filename: {name:?}"),
            }
            .into());
        }
    }
    Ok(())
}

/// Test helper: full Windows filename rules (reserved + strict chars).
#[cfg(test)]
fn check_windows_filename(name: &str) -> Result<()> {
    check_reserved_windows_name(name)?;
    // Exercise strict rules on every host in unit tests.
    if name.ends_with(' ') || name.ends_with('.') {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("Windows-illegal trailing space or period in filename: {name:?}"),
        }
        .into());
    }
    for ch in name.chars() {
        if WINDOWS_INVALID_NAME_CHARS.contains(&ch) || ch == ':' || (ch as u32) < 32 {
            return Err(AtomwriteError::InvalidInput {
                reason: format!("Windows-illegal character {ch:?} in filename: {name:?}"),
            }
            .into());
        }
    }
    Ok(())
}

/// Windows classic `MAX_PATH` limit (without long-path opt-in).
pub const WINDOWS_MAX_PATH: usize = 260;

/// Threshold at which we apply the Win32 long-path prefix (`\\?\`).
/// Slightly under 260 to leave room for temp suffixes during atomic rename.
pub const WINDOWS_LONG_PATH_THRESHOLD: usize = 240;

/// Prepare a path for filesystem APIs that may hit Windows `MAX_PATH`.
///
/// On Windows, absolute paths whose UTF-16-ish length exceeds
/// [`WINDOWS_LONG_PATH_THRESHOLD`] receive the `\\?\` (or `\\?\UNC\`) prefix
/// when not already present. On other platforms this is identity.
///
/// Prefer this helper before open/create/rename when the path may be long.
pub fn prepare_fs_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        return prepare_fs_path_windows(path);
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

/// Whether a path string length is at risk of Windows `MAX_PATH` issues.
pub fn exceeds_windows_max_path(path: &Path) -> bool {
    path.to_string_lossy().chars().count() >= WINDOWS_MAX_PATH
}

#[cfg(windows)]
fn prepare_fs_path_windows(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with(r"\\?\") || s.starts_with("//?/") {
        return path.to_path_buf();
    }
    if s.chars().count() < WINDOWS_LONG_PATH_THRESHOLD {
        return path.to_path_buf();
    }
    // Absolute with drive letter: C:\foo → \\?\C:\foo
    if path.is_absolute() {
        if s.starts_with(r"\\") || s.starts_with("//") {
            // UNC: \\server\share\path → \\?\UNC\server\share\path
            let trimmed = s.trim_start_matches(['\\', '/']);
            return PathBuf::from(format!(r"\\?\UNC\{trimmed}"));
        }
        return PathBuf::from(format!(r"\\?\{s}"));
    }
    path.to_path_buf()
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

    #[test]
    fn rejects_reserved_con_and_nul_txt() {
        assert!(check_windows_filename("CON").is_err());
        assert!(check_windows_filename("nul.txt").is_err());
        assert!(check_windows_filename("COM0").is_err());
        assert!(check_windows_filename("lpt0.log").is_err());
        assert!(check_windows_filename("LPT9").is_err());
        assert!(check_windows_filename("readme.txt").is_ok());
    }

    #[test]
    fn rejects_trailing_space_or_dot() {
        assert!(check_windows_filename("file.").is_err());
        assert!(check_windows_filename("file ").is_err());
    }

    #[test]
    fn rejects_invalid_windows_chars() {
        assert!(check_windows_filename("a<b").is_err());
        assert!(check_windows_filename("a|b").is_err());
        assert!(check_windows_filename("a:b").is_err());
        assert!(check_windows_filename("ok-name_1.txt").is_ok());
    }

    #[test]
    fn validate_path_rejects_reserved_component() {
        let ws = Path::new("/tmp");
        let bad = Path::new("/tmp/CON/out.txt");
        let err = validate_path(bad, ws).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("reserved Windows") || msg.contains("CON"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn prepare_fs_path_identity_on_unix() {
        let p = Path::new("/tmp/short.txt");
        assert_eq!(prepare_fs_path(p), p);
    }

    #[test]
    fn exceeds_windows_max_path_detects_long() {
        let long = "a".repeat(WINDOWS_MAX_PATH);
        assert!(exceeds_windows_max_path(Path::new(&long)));
        assert!(!exceeds_windows_max_path(Path::new("short.txt")));
    }
}
