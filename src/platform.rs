// SPDX-License-Identifier: MIT OR Apache-2.0

//! Platform-specific fsync, durability, and console initialization primitives.

#![allow(unsafe_code)]

use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};

/// Flush file data to persistent storage using the best platform-specific method.
///
/// # Errors
///
/// Returns an I/O error if the fsync syscall fails.
pub fn fsync_file(file: &File) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::io::AsRawFd;
        let fd = file.as_raw_fd();
        // SAFETY: F_FULLFSYNC is a macOS-specific fcntl command that flushes the
        // disk cache to physical media. The fd is valid because it comes from a
        // live File reference. This is required because macOS fsync() does NOT
        // guarantee data reaches persistent storage without F_FULLFSYNC.
        let ret = unsafe { libc::fcntl(fd, libc::F_FULLFSYNC) };
        if ret == -1 {
            file.sync_data()
                .context("fsync fallback after F_FULLFSYNC failure")?;
        }
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        file.sync_data().context("fsync file")?;
        Ok(())
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
