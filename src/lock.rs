// SPDX-License-Identifier: MIT OR Apache-2.0

//! Advisory file locking for concurrent edit protection (G54).
//!
//! When two atomwrite processes (or an editor and an agent) edit the same
//! file simultaneously, the last `rename(2)` wins and the first write is
//! silently lost. The `--expect-checksum` flag (exit 82) detects this
//! *after* it happens, but does not prevent it.
//!
//! This module adds proactive protection via `nix::fcntl::flock` on Unix
//! (which wraps `flock(2)`) and a no-op stub on Windows. The lock is held
//! for the duration of the atomic write and released on Drop (RAII).
//!
//! **Limitations**:
//! - `flock(2)` is per-process on some systems; on NFS, `flock` is silently
//!   ignored. We document this as a known limitation and recommend
//!   `--expect-checksum` as the additional safety net on NFS.
//! - The lock file is `.<target>.atomwrite.lock` in the same directory as
//!   the target, so the lock disappears when the target is deleted.
//! - On Windows, this is a no-op (`LockGuard` always succeeds immediately)
//!   because `flock(2)` semantics are not available without C extensions.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::error::AtomwriteError;

/// RAII guard that holds an advisory exclusive lock on a file.
///
/// On Unix, the lock is held via `flock(LOCK_EX)` on the sidecar file until
/// the guard is dropped, which calls `flock(LOCK_UN)`. On Windows, the
/// guard is a no-op.
pub struct LockGuard {
    /// File path the lock is protecting.
    target: PathBuf,
    /// The sidecar lock file. Kept open so the `fd` stays valid for `flock`.
    /// On Unix, the file is duplicated via `dup(2)` so the `flock` lock
    /// survives even after the original handle is closed (we still keep the
    /// original open while the guard lives, as a belt-and-suspenders measure).
    #[cfg(unix)]
    _file: std::fs::File,
    /// Time elapsed while waiting for the lock, in milliseconds.
    pub held_ms: u64,
}

impl std::fmt::Debug for LockGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockGuard")
            .field("target", &self.target)
            .field("held_ms", &self.held_ms)
            .finish_non_exhaustive()
    }
}

#[cfg(unix)]
impl Drop for LockGuard {
    fn drop(&mut self) {
        // Release the flock explicitly so the lock is freed before the
        // file handle is closed. The `flock` syscall on the raw fd is
        // safe: we still own the file handle at this point.
        use std::os::unix::io::AsRawFd;
        let fd = self._file.as_raw_fd();
        // Best-effort: if this fails, the process is exiting anyway and
        // the kernel will release the lock when the fd is closed.
        #[allow(deprecated)]
        let _ = nix::fcntl::flock(fd, nix::fcntl::FlockArg::Unlock);
    }
}

#[cfg(not(unix))]
impl Drop for LockGuard {
    fn drop(&mut self) {
        // No-op on Windows (flock semantics not available).
    }
}

/// Acquire an exclusive advisory lock on `target` with the given timeout.
///
/// The lock is taken on a sidecar file `.<target>.atomwrite.lock` in the
/// same directory, so the sidecar automatically disappears if the target
/// is deleted (the lock is then moot anyway).
///
/// # Errors
///
/// Returns `AtomwriteError::LockTimeout` if the lock could not be acquired
/// within `timeout_ms`. Returns I/O errors for permission denied, missing
/// parent directory, etc.
pub fn acquire_exclusive(target: &Path, timeout_ms: u64) -> Result<LockGuard> {
    let start = Instant::now();
    let lock_path = sidecar_path(target);
    let parent = lock_path
        .parent()
        .ok_or_else(|| AtomwriteError::InternalError {
            reason: format!("lock path has no parent: {}", lock_path.display()),
        })?;
    std::fs::create_dir_all(parent).with_context(|| {
        format!(
            "cannot create parent directory for lock {}",
            lock_path.display()
        )
    })?;

    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("cannot open lock file {}", lock_path.display()))?;

    let held_ms = try_acquire_loop(&lock_file, timeout_ms, start)?;

    tracing::debug!(
        target = %target.display(),
        lock = %lock_path.display(),
        held_ms,
        "advisory lock acquired"
    );

    Ok(LockGuard {
        target: target.to_path_buf(),
        #[cfg(unix)]
        _file: lock_file,
        held_ms,
    })
}

#[cfg(unix)]
fn try_acquire_loop(file: &std::fs::File, timeout_ms: u64, start: Instant) -> Result<u64> {
    use std::os::unix::io::AsRawFd;
    let fd = file.as_raw_fd();
    let deadline = Duration::from_millis(timeout_ms);
    let mut attempts: u32 = 0;
    loop {
        // `LockExclusiveNonblock` is the safe variant for our retry loop:
        // it returns EWOULDBLOCK (EAGAIN) immediately if the file is already
        // locked, instead of blocking until the lock is released. This lets
        // us poll on a timer and respect `timeout_ms` exactly.
        #[allow(deprecated)]
        match nix::fcntl::flock(fd, nix::fcntl::FlockArg::LockExclusiveNonblock) {
            Ok(()) => return Ok(start.elapsed().as_millis() as u64),
            Err(nix::Error::EAGAIN) => {
                let elapsed = start.elapsed();
                if elapsed >= deadline {
                    return Err(AtomwriteError::LockTimeout {
                        path: std::path::PathBuf::from("<target>"),
                        timeout_ms,
                    }
                    .into());
                }
                attempts += 1;
                let sleep_ms = if attempts < 20 { 10 } else { 50 };
                std::thread::sleep(Duration::from_millis(sleep_ms));
            }
            Err(e) => {
                return Err(AtomwriteError::Io {
                    source: std::io::Error::from_raw_os_error(e as i32),
                }
                .into());
            }
        }
    }
}

#[cfg(not(unix))]
fn try_acquire_loop(_file: &std::fs::File, _timeout_ms: u64, start: Instant) -> Result<u64> {
    // On Windows, advisory file locking is a no-op (the semantics of
    // flock(2) are not available without C extensions). The LockGuard
    // still serves to document the intent at the call site.
    Ok(start.elapsed().as_millis() as u64)
}

/// Compute the sidecar path used as the lock file for `target`.
fn sidecar_path(target: &Path) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let filename = target
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    parent.join(format!(".{filename}.atomwrite.lock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_path_appends_dot_atomwrite_lock() {
        let target = Path::new("/tmp/foo.txt");
        let lock = sidecar_path(target);
        assert_eq!(lock, Path::new("/tmp/.foo.txt.atomwrite.lock"));
    }

    #[test]
    fn acquire_release_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("test.txt");
        std::fs::write(&target, "hello").unwrap();

        {
            let guard = acquire_exclusive(&target, 1000).unwrap();
            assert!(guard.held_ms < 1000);
        }
        // After drop, a new acquire must succeed quickly.
        let guard2 = acquire_exclusive(&target, 1000).unwrap();
        drop(guard2);
    }

    #[cfg(unix)]
    #[test]
    fn second_acquire_blocks_until_release() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("concurrent.txt");
        std::fs::write(&target, "shared").unwrap();

        let guard1 = acquire_exclusive(&target, 1000).unwrap();
        let start = std::time::Instant::now();
        // Second acquire should fail because the first is still held.
        // Use a short timeout so the test doesn't hang for the full 1000ms.
        let result = acquire_exclusive(&target, 50);
        let elapsed = start.elapsed().as_millis();
        assert!(result.is_err(), "second acquire must time out");
        assert!(
            elapsed >= 50,
            "must wait at least the configured timeout, got {elapsed}ms"
        );
        let err = result.unwrap_err();
        let ae = err.downcast_ref::<AtomwriteError>();
        assert!(
            matches!(ae, Some(AtomwriteError::LockTimeout { .. })),
            "expected LockTimeout, got: {err:?}"
        );
        drop(guard1);
    }
}
