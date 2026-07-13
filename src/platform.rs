// SPDX-License-Identifier: MIT OR Apache-2.0

//! Platform-specific fsync, durability, and console initialization primitives.

#![allow(unsafe_code)]

use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};

/// File durability trade-off for atomic writes (v0.1.29 P2-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Durability {
    /// Full durable flush: `F_FULLFSYNC` on macOS, `sync_all` when available; always fsync dir.
    Full,
    /// Fast path: file `sync_data` only; skip directory fsync when safe enough for agent workloads.
    Fast,
    /// Auto: Full for config-like extensions, Fast for source trees (default).
    #[default]
    Auto,
}

impl Durability {
    /// Resolve Auto against a target path.
    pub fn resolve(self, target: &Path) -> Durability {
        match self {
            Durability::Auto => {
                let name = target
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let ext = target
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if matches!(
                    ext.as_str(),
                    "toml" | "json" | "yaml" | "yml" | "ini" | "conf" | "cfg" | "env"
                ) || name.starts_with('.')
                    || name == "cargo.lock"
                    || name == "dockerfile"
                {
                    Durability::Full
                } else {
                    Durability::Fast
                }
            }
            other => other,
        }
    }

    /// Stable name for NDJSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Durability::Full => "full",
            Durability::Fast => "fast",
            Durability::Auto => "auto",
        }
    }
}

/// Flush file data to persistent storage using the best platform-specific method.
///
/// # Errors
///
/// Returns an I/O error if the fsync syscall fails.
pub fn fsync_file(file: &File) -> Result<()> {
    fsync_file_with_durability(file, Durability::Full)
}

/// Flush file data according to the durability policy (v0.1.29 P2-1).
///
/// - `Full`: macOS `F_FULLFSYNC`, else `sync_all` on the file descriptor path when available
/// - `Fast` / resolved `Auto`: `sync_data` only
pub fn fsync_file_with_durability(file: &File, durability: Durability) -> Result<()> {
    match durability {
        Durability::Fast | Durability::Auto => {
            file.sync_data().context("fsync file (fast)")?;
            Ok(())
        }
        Durability::Full => {
            #[cfg(target_os = "macos")]
            {
                use std::os::unix::io::AsRawFd;
                let fd = file.as_raw_fd();
                // SAFETY: F_FULLFSYNC requires a valid fd from a live File.
                let ret = unsafe { libc::fcntl(fd, libc::F_FULLFSYNC) };
                if ret == -1 {
                    file.sync_data()
                        .context("fsync fallback after F_FULLFSYNC failure")?;
                }
                return Ok(());
            }
            #[cfg(not(target_os = "macos"))]
            {
                // Prefer sync_all for full durability of metadata+data on Linux/Windows.
                file.sync_all().context("fsync file (full)")?;
                Ok(())
            }
        }
    }
}

/// Best-effort fsync that logs a warning instead of returning an error.
///
/// Used for backup file durability where losing a sync is acceptable: the
/// backup itself has been created successfully via `fs::copy`, and the worst
/// case is that the backup metadata is not flushed to disk before the process
/// exits. The primary write path still uses the strict [`fsync_file`].
///
/// On Windows, antivirus products (Windows Defender, third-party AV) can hold
/// a transient read handle on files in the user's temp directory with
/// `FILE_SHARE_READ` but without `FILE_SHARE_WRITE`, which causes
/// `FlushFileBuffers` to return `ERROR_ACCESS_DENIED` (os error 5). The
/// tempfile crate creates backup files in `%TEMP%` during tests, which is
/// where this race was observed in CI. Logging a warning and continuing keeps
/// the user-visible operation successful.
pub fn fsync_file_best_effort(file: &File) {
    if let Err(e) = fsync_file(file) {
        tracing::warn!(error = %e, "fsync_file best-effort: continuing without durability flush");
    }
}

/// Sync the directory metadata to ensure rename durability.
///
/// # Errors
///
/// Returns an I/O error if the directory cannot be opened or synced.
/// On Windows this is a no-op and always succeeds.
pub fn fsync_dir(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let file = File::open(dir)
            .with_context(|| format!("cannot open directory {} for fsync", dir.display()))?;
        file.sync_all()
            .with_context(|| format!("fsync directory {}", dir.display()))?;
        Ok(())
    }

    #[cfg(windows)]
    {
        let _ = dir;
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = dir;
        Ok(())
    }
}

/// Restore the modification and access timestamps on a file.
///
/// # Errors
///
/// Returns an I/O error if the timestamps cannot be set.
pub fn preserve_timestamps(
    path: &Path,
    mtime: filetime::FileTime,
    atime: filetime::FileTime,
) -> Result<()> {
    filetime::set_file_times(path, atime, mtime)
        .with_context(|| format!("cannot restore timestamps on {}", path.display()))
}

/// Return the name of the file fsync method used on this platform.
pub fn platform_fsync_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "F_FULLFSYNC"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "sync_data"
    }
}

/// Initialize Windows console for UTF-8 output and ANSI escape code support.
///
/// Sets code page 65001 (UTF-8) for both input and output, and enables
/// `ENABLE_VIRTUAL_TERMINAL_PROCESSING` on stdout and stderr handles so
/// ANSI escape sequences are interpreted by the Windows Console Host.
///
/// On non-Windows platforms this is a no-op.
#[cfg(windows)]
pub fn init_console() {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::*;
    // SAFETY: SetConsoleOutputCP, SetConsoleCP, GetStdHandle, GetConsoleMode
    // and SetConsoleMode are safe Win32 API calls. CP 65001 is UTF-8.
    // ENABLE_VIRTUAL_TERMINAL_PROCESSING enables ANSI escape interpretation.
    unsafe {
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);

        for handle_id in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let handle = GetStdHandle(handle_id);
            if !handle.is_null() && handle != INVALID_HANDLE_VALUE {
                let mut mode: u32 = 0;
                if GetConsoleMode(handle, &mut mode) != 0 {
                    let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
                }
            }
        }
    }
}

/// No-op on non-Windows platforms.
#[cfg(not(windows))]
pub fn init_console() {}

/// Return the name of the directory fsync method used on this platform.
pub fn platform_dir_fsync_name() -> &'static str {
    #[cfg(unix)]
    {
        "sync_all"
    }
    #[cfg(windows)]
    {
        "best_effort"
    }
    #[cfg(not(any(unix, windows)))]
    {
        "none"
    }
}

/// Perform an atomic rename and report the method used (v0.1.29 P2-2).
///
/// On Linux, prefers `renameat2(..., flags=0)` which is equivalent to
/// `renameat` for overwrite semantics (man7). Falls back to `std::fs::rename`
/// when the kernel returns `ENOSYS`.
pub fn atomic_rename(from: &Path, to: &Path) -> Result<&'static str> {
    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let from_c = CString::new(from.as_os_str().as_bytes())
            .with_context(|| format!("invalid rename source path: {}", from.display()))?;
        let to_c = CString::new(to.as_os_str().as_bytes())
            .with_context(|| format!("invalid rename dest path: {}", to.display()))?;
        // flags=0: same overwrite semantics as renameat/rename (not RENAME_NOREPLACE).
        let ret = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                from_c.as_ptr(),
                libc::AT_FDCWD,
                to_c.as_ptr(),
                0,
            )
        };
        if ret == 0 {
            return Ok("renameat2");
        }
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ENOSYS) {
            std::fs::rename(from, to)
                .with_context(|| format!("rename {} -> {}", from.display(), to.display()))?;
            return Ok("rename");
        }
        return Err(err)
            .with_context(|| format!("renameat2 {} -> {}", from.display(), to.display()));
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::fs::rename(from, to)
            .with_context(|| format!("rename {} -> {}", from.display(), to.display()))?;
        #[cfg(target_os = "macos")]
        {
            Ok("rename")
        }
        #[cfg(windows)]
        {
            Ok("MoveFileEx")
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            Ok("rename")
        }
    }
}

/// Name of the rename method that will be used (diagnostic helper).
pub fn platform_rename_method() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "renameat2"
    }
    #[cfg(target_os = "macos")]
    {
        "rename"
    }
    #[cfg(windows)]
    {
        "MoveFileEx"
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        "rename"
    }
}
