// SPDX-License-Identifier: MIT OR Apache-2.0

//! Advisory file locking for concurrent edit protection (G54, R23).
//!
//! Workload: I/O-bound (flock/fcntl on one path).
//! Parallelism: none — RAII lock serialises writers on a single target. Cross-
//! process concurrency is the OS lock, not rayon fan-out.
//!
//! When two atomwrite processes (or an editor and an agent) edit the same
//! file simultaneously, the last `rename(2)` wins and the first write is
//! silently lost. The `--expect-checksum` flag (exit 82) detects this
//! *after* it happens, but does not prevent it.
//!
//! This module adds proactive protection via `std::fs::File::try_lock`, which
//! wraps `flock(2)` on Unix and `LockFileEx` on Windows, so both platforms
//! serialise writers the same way with no extra crate and no `unsafe`.
//!
//! # What the lock covers
//!
//! A write that is computed from the file it replaces (edit, replace, set, del,
//! case, apply, transform, scope, `config set`, `write --append`, and the
//! `batch` ops that read) loses a concurrent update unless the lock is taken
//! **before** it reads. Those verbs call [`acquire_for_update`](crate::lock::acquire_for_update) first and hold
//! the guard until the write is committed, so concurrent writers are serialised:
//! each one reads the result of the one before it and every update survives.
//!
//! [`crate::atomic::atomic_write`] takes the same lock itself. The lock is
//! re-entrant **per thread**: while a thread holds a guard for a path, a second
//! [`acquire_exclusive`](crate::lock::acquire_exclusive) for that path on the same thread returns a nested guard
//! at once instead of blocking on its own `flock`. The real lock is released
//! when the last guard drops. A second thread of the same process asks the OS
//! and waits, as another process would.
//!
//! A verb that does not take the lock first still gets no silent loss when it
//! passes the checksum of the content it read to the writer
//! (`atomic_write_based`): the writer compares it with the file under the lock
//! and fails with `STATE_DRIFT` (exit 82, retryable)
//! instead of overwriting. That is also the net on filesystems without `flock`.
//! A write of content that does not depend on the old file (plain `write`,
//! `copy`, `move`, `rollback`) overwrites by design and takes the lock only
//! around the commit.
//!
//! **Limitations**:
//! - `flock(2)` is per-process on some systems; on NFS, `flock` is silently
//!   ignored. We document this as a known limitation and recommend
//!   `--expect-checksum` as the additional safety net on NFS. Filesystems
//!   that reject `flock` (`EOPNOTSUPP`, `ENOLCK`, `ENOSYS`) run without lock.
//! - The lock file is `.<target>.atomwrite.lock` in the same directory as
//!   the target. The guard unlinks it on release while it still holds the
//!   lock, and waiters re-check the inode after acquiring, so no sidecar
//!   survives a clean exit. A process killed by SIGKILL can leave one behind;
//!   the next writer reuses and removes it.
//! - On Windows the sidecar is reused instead of unlinked, because a handle
//!   open elsewhere would make the unlink fail anyway; the lock itself is
//!   `LockFileEx` through the same standard-library call.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;

use crate::error::AtomwriteError;
use crate::retry::{Backoff, RetryPolicy, RetryStop};

/// RAII guard that holds an advisory exclusive lock on a file.
///
/// The lock is held on the sidecar file until the last guard for the path
/// drops (guards taken again on the same thread share one lock), which
/// releases it with `File::unlock` and, on Unix, unlinks the sidecar.
///
/// Marked `#[must_use]`: dropping the guard early releases the lock; ignoring
/// the return of [`acquire_exclusive`] would drop immediately and defeat
/// concurrent-edit protection (ownership rules: resource-bearing types).
#[must_use = "holding LockGuard keeps the advisory flock; dropping releases it"]
pub struct LockGuard {
    /// File path the lock is protecting.
    target: PathBuf,
    /// The real lock, shared by every guard the thread holds for the path.
    _held: std::rc::Rc<Held>,
    /// Time elapsed while waiting for the lock, in milliseconds.
    pub held_ms: u64,
}

/// The OS locks one guard holds, released when the last guard drops.
struct Held {
    /// Path of the sidecar lock file, unlinked on release. Unix only: the
    /// Windows sidecar is reused instead of unlinked, so the field would be
    /// dead there.
    #[cfg(unix)]
    lock_path: Option<PathBuf>,
    /// Every locked handle. The first is the target itself, which is the
    /// object being mutated; the second, when present, is the sidecar that
    /// older releases lock, kept for the migration window.
    files: Vec<std::fs::File>,
    /// Whether [`Self::files`] starts with the target's own inode.
    locked_target: bool,
}

thread_local! {
    /// Locks this thread holds, by sidecar path, so a nested acquire is a no-op.
    static HELD: std::cell::RefCell<std::collections::HashMap<PathBuf, std::rc::Weak<Held>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

impl std::fmt::Debug for LockGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockGuard")
            .field("target", &self.target)
            .field("held_ms", &self.held_ms)
            .finish_non_exhaustive()
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // Unlink while the lock is still held: a waiter that opened the old
        // inode sees the path vanish or change in `acquire_exclusive` and
        // retries on a fresh file, so two writers never hold "the" lock.
        // Best-effort: Drop must be short and panic-free.
        #[cfg(unix)]
        if let Some(lock_path) = &self.lock_path {
            crate::best_effort::best_effort(std::fs::remove_file(lock_path), "remove lock sidecar");
        }
        // If this fails, the process is exiting anyway and the kernel
        // releases the lock when the file is closed.
        for file in &self.files {
            crate::best_effort::best_effort(file.unlock(), "release advisory lock");
        }
    }
}

/// Take the lock a read-modify-write verb needs, BEFORE it reads the target.
///
/// Hold the returned guard until the write is committed. The wait budget is the
/// configured lock timeout (`lock.acquire_timeout_ms`). The `target` must be the
/// validated path the verb later hands to [`crate::atomic::atomic_write`].
///
/// # Errors
///
/// Same as [`acquire_exclusive`].
pub fn acquire_for_update(target: &Path) -> Result<LockGuard> {
    acquire_exclusive(target, crate::config::tuning().lock_acquire_timeout_ms)
}

/// Acquire an exclusive advisory lock on `target` with the given timeout.
///
/// Re-entrant per thread: when the calling thread already holds the lock for
/// `target`, the returned guard shares it and returns without waiting.
///
/// The lock is taken on a sidecar file `.<target>.atomwrite.lock` in the
/// same directory, which the caller must have created already. The sidecar
/// is removed when the returned guard drops.
///
/// # Errors
///
/// Returns `AtomwriteError::LockTimeout` (exit 83, carrying the real
/// `target` path) if the lock could not be acquired within `timeout_ms`.
/// Returns `AtomwriteError::Cancelled` if a shutdown signal arrives while
/// waiting, or when the global `--timeout-secs` deadline ends the wait before
/// `timeout_ms` does. Returns I/O errors for permission denied, missing parent
/// directory, etc.
///
/// Waiting follows [`crate::retry`]: exponential backoff with full jitter,
/// cut into slices that poll the shutdown flag, under a budget of `timeout_ms`
/// or the time left of the global deadline, whichever is smaller.
pub fn acquire_exclusive(target: &Path, timeout_ms: u64) -> Result<LockGuard> {
    let start = Instant::now();
    let key = lock_key(target);
    if let Some(held) = HELD.with(|m| m.borrow().get(&key).and_then(std::rc::Weak::upgrade)) {
        return Ok(LockGuard {
            target: target.to_path_buf(),
            _held: held,
            held_ms: 0,
        });
    }
    let mut backoff = Backoff::new(RetryPolicy::new(
        crate::constants::LOCK_RETRY_BASE_MS,
        crate::constants::LOCK_RETRY_CAP_MS,
        timeout_ms,
        crate::constants::LOCK_RETRY_MAX_ATTEMPTS,
    ));
    let mut files: Vec<std::fs::File> = Vec::new();
    let mut locked_target = false;

    // The subject is the file, not its name. Locking the inode the caller will
    // mutate means an in-place write needs permission on the file and nothing
    // on the directory, which is exactly what the write itself needs.
    match open_target_for_lock(target) {
        Ok(Some(file)) => {
            // A writer that renamed its result into place while this call was
            // waiting leaves the lock on an inode the path no longer reaches.
            // Holding it would exclude nobody, so the acquisition restarts on
            // whatever inode answers to the path now.
            let mut current = Some(file);
            let mut restarts = 0;
            loop {
                let Some(handle) = current.as_ref() else {
                    break;
                };
                try_acquire_loop(handle, target, timeout_ms, &mut backoff)?;
                if target_is_same_inode(handle, target)
                    || restarts >= crate::constants::LOCK_STALE_INODE_MAX_RESTARTS
                {
                    break;
                }
                restarts += 1;
                // The old handle goes when it is replaced, releasing the lock
                // on the unreachable inode.
                current = open_target_for_lock(target).ok().flatten();
            }
            if let Some(handle) = current {
                files.push(handle);
                locked_target = true;
            }
        }
        // A new file has no inode to lock yet; the sidecar carries that case.
        Ok(None) => {}
        // A file we may read but not open for writing still gets the sidecar,
        // and the write fails later on its own terms rather than here.
        Err(e) => tracing::debug!(
            path = %target.display(),
            error = %e,
            "target cannot be opened for locking; falling back to the sidecar"
        ),
    }

    // The sidecar keeps older releases and this one mutually excluded, and it
    // is best-effort: a directory this process cannot write to no longer stops
    // a write the inode lock already covers.
    let lock_path = sidecar_path(target);
    let mut sidecar_held = None;
    let mut sidecar_error = None;
    // A writer that finishes releases the sidecar by unlinking it, so a
    // handle this call already holds can name an inode no other writer can
    // reach. That handle is retaken, not trusted, and the retakes are bounded.
    for _ in 0..=crate::constants::LOCK_STALE_INODE_MAX_RESTARTS {
        let file = match std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(file) => file,
            Err(e) => {
                if locked_target {
                    tracing::debug!(
                        path = %target.display(),
                        error = %e,
                        "lock sidecar unavailable; the target inode lock carries the run"
                    );
                }
                sidecar_error = Some(e);
                break;
            }
        };
        if let Err(e) = try_acquire_loop(&file, target, timeout_ms, &mut backoff) {
            // An older release holds the sidecar while this run already owns
            // the inode. The two generations cannot write at once, so the
            // timeout is the answer instead of a silent bypass; a sidecar the
            // directory cannot create stays best-effort.
            if locked_target
                && matches!(
                    e.downcast_ref::<crate::error::AtomwriteError>(),
                    Some(crate::error::AtomwriteError::LockTimeout { .. })
                )
            {
                return Err(e);
            }
            // The lock attempt ran out of budget or was refused; the kind is
            // what the caller branches on, so it survives the flattening.
            sidecar_error = Some(
                crate::runtime::find_io_error(&e)
                    .map(std::io::Error::from)
                    .unwrap_or_else(|| std::io::Error::other(e.to_string())),
            );
            break;
        }
        if is_current_sidecar(&file, &lock_path) {
            files.push(file);
            sidecar_held = Some(lock_path.clone());
            break;
        }
    }

    let Some(_) = files.first() else {
        let error = sidecar_error
            .unwrap_or_else(|| std::io::Error::other("no lockable handle for the target"));
        // The subject is the target, because that is the file the caller named
        // and the one a retry has to reach. The sidecar is the mechanism that
        // failed, and this run creates and unlinks it, so it cannot be the path
        // the verdict reports.
        // The field is `path`, not `target`: `target` is the tracing event's
        // own key, and a second one would repeat the name in the JSON line.
        tracing::warn!(
            path = %target.display(),
            sidecar = %lock_path.display(),
            error = %error,
            "no lockable handle for the target"
        );
        return Err(AtomwriteError::from_io_at(target, error).into());
    };

    let held_ms = start.elapsed().as_millis() as u64;
    tracing::debug!(
        target = %target.display(),
        inode_locked = locked_target,
        sidecar = sidecar_held.is_some(),
        held_ms,
        "advisory lock acquired"
    );
    #[cfg(unix)]
    let held = std::rc::Rc::new(Held {
        lock_path: sidecar_held,
        files,
        locked_target,
    });
    #[cfg(not(unix))]
    let held = std::rc::Rc::new(Held {
        files,
        locked_target,
    });
    HELD.with(|m| {
        let mut map = m.borrow_mut();
        map.retain(|_, w| w.strong_count() > 0);
        map.insert(key, std::rc::Rc::downgrade(&held));
    });
    Ok(LockGuard {
        target: target.to_path_buf(),
        _held: held,
        held_ms,
    })
}

/// Open the target for the exclusive lock, or report that it does not exist.
fn open_target_for_lock(target: &Path) -> std::io::Result<Option<std::fs::File>> {
    match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(target)
    {
        Ok(file) => Ok(Some(file)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

impl LockGuard {
    /// True when the path still resolves to the inode this guard locked.
    ///
    /// The check is what makes a rename safe to perform: between the moment
    /// the lock was taken and the moment the swap runs, the path may have been
    /// pointed at another file, and swapping then would overwrite content this
    /// run never read. A guard that locked no inode (a new file) has nothing to
    /// compare and reports `true`.
    #[must_use]
    pub fn target_unchanged(&self) -> bool {
        if !self._held.locked_target {
            return true;
        }
        let Some(file) = self._held.files.first() else {
            return true;
        };
        let Ok(clone) = file.try_clone() else {
            return true;
        };
        match (
            same_file::Handle::from_file(clone),
            same_file::Handle::from_path(&self.target),
        ) {
            (Ok(held), Ok(current)) => held == current,
            // Unreadable metadata is not evidence of divergence; the write
            // itself will report what it cannot do.
            _ => true,
        }
    }
}

/// Whether the locked handle still answers to the same path it was opened from.
///
/// The comparison is by inode, because a rename swaps the inode under a path
/// without touching the file the caller already holds. An unreadable path is
/// reported as unchanged: the write itself will say what it cannot do, and
/// turning a metadata error into a retry loop would hide it.
fn target_is_same_inode(file: &std::fs::File, target: &Path) -> bool {
    let Ok(clone) = file.try_clone() else {
        return true;
    };
    match (
        same_file::Handle::from_file(clone),
        same_file::Handle::from_path(target),
    ) {
        (Ok(held), Ok(current)) => held == current,
        _ => true,
    }
}

/// Key of the per-thread table: the sidecar path with its directory resolved,
/// so two spellings of the same directory share one lock.
fn lock_key(lock_path: &Path) -> PathBuf {
    let resolved = lock_path
        .parent()
        .and_then(|dir| dir.canonicalize().ok())
        .zip(lock_path.file_name());
    match resolved {
        Some((dir, name)) => dir.join(name),
        None => lock_path.to_path_buf(),
    }
}

/// On Windows the sidecar is never unlinked, so the open file is always
/// the current one.
#[cfg(not(unix))]
fn is_current_sidecar(_file: &std::fs::File, _lock_path: &Path) -> bool {
    true
}

/// True when `lock_path` still names the inode behind `file`.
#[cfg(unix)]
fn is_current_sidecar(file: &std::fs::File, lock_path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Ok(held) = file.metadata() else {
        return true;
    };
    match std::fs::metadata(lock_path) {
        Ok(current) => held.dev() == current.dev() && held.ino() == current.ino(),
        Err(e) => e.kind() != std::io::ErrorKind::NotFound,
    }
}

/// True when the error means the filesystem has no advisory lock at all.
#[cfg(unix)]
fn is_lock_unsupported(e: &std::io::Error) -> bool {
    use nix::errno::Errno;
    matches!(
        e.raw_os_error(),
        Some(code)
            if code == Errno::EOPNOTSUPP as i32
                || code == Errno::ENOLCK as i32
                || code == Errno::ENOSYS as i32
    )
}

fn try_acquire_loop(
    file: &std::fs::File,
    target: &Path,
    timeout_ms: u64,
    backoff: &mut Backoff,
) -> Result<()> {
    loop {
        // `try_lock` is the non-blocking variant: it reports `WouldBlock`
        // immediately if another holder owns the lock, instead of blocking
        // until release. This lets the retry module wait with jitter and
        // respect `timeout_ms` and the global deadline.
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => match backoff.wait() {
                Ok(()) => {}
                Err(RetryStop::Cancelled) => {
                    return Err(crate::signal::cancelled_error(format!(
                        "lock wait cancelled for {}",
                        target.display()
                    ))
                    .into());
                }
                Err(RetryStop::DeadlineElapsed | RetryStop::AttemptsExhausted) => {
                    return Err(AtomwriteError::LockTimeout {
                        path: target.to_path_buf(),
                        timeout_ms,
                    }
                    .into());
                }
            },
            Err(std::fs::TryLockError::Error(e)) => {
                #[cfg(unix)]
                if is_lock_unsupported(&e) {
                    // The filesystem has no flock: the lock is advisory, so
                    // the write proceeds and `--expect-checksum` stays the
                    // safety net.
                    tracing::warn!(
                        path = %target.display(),
                        "filesystem does not support flock; continuing without advisory lock"
                    );
                    return Ok(());
                }
                return Err(AtomwriteError::Io { source: e }.into());
            }
        }
    }
}

/// Compute the sidecar path used as the lock file for `target`.
fn sidecar_path(target: &Path) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let filename = target
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let prefix = crate::constants::HIDDEN_FILE_PREFIX;
    let suffix = crate::constants::LOCK_SIDECAR_SUFFIX;
    // The sidecar lives beside the target, so its name is bounded by the same
    // filesystem limit. Past 255 bytes the stem becomes a short hash of the
    // full name: still unique per target and stable across runs.
    let name = if prefix.len() + filename.len() + suffix.len() > 255 {
        let digest = crate::checksum::hash_bytes(filename.as_bytes());
        format!("{prefix}{}", &digest[..16])
    } else {
        format!("{prefix}{filename}")
    };
    parent.join(format!("{name}{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
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
    fn release_removes_the_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("clean.txt");
        std::fs::write(&target, "x").unwrap();

        let guard = acquire_exclusive(&target, 1000).unwrap();
        assert!(sidecar_path(&target).exists(), "sidecar exists while held");
        drop(guard);
        assert!(!sidecar_path(&target).exists(), "sidecar must not survive");
    }

    #[cfg(unix)]
    #[test]
    fn second_acquire_blocks_until_release() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("concurrent.txt");
        std::fs::write(&target, "shared").unwrap();

        let guard1 = acquire_exclusive(&target, 1000).unwrap();
        let start = std::time::Instant::now();
        // A second THREAD must fail while the first holds the lock (the same
        // thread would share it). A short timeout keeps the test fast.
        let other = target.clone();
        let result = std::thread::spawn(move || acquire_exclusive(&other, 50).map(drop))
            .join()
            .expect("acquire thread");
        let elapsed = start.elapsed().as_millis();
        assert!(result.is_err(), "second acquire must time out");
        assert!(
            elapsed >= 50,
            "must wait at least the configured timeout, got {elapsed}ms"
        );
        assert!(
            elapsed < 50 + 300,
            "the last wait is cut at the deadline, got {elapsed}ms"
        );
        let err = result.unwrap_err();
        let ae = err.downcast_ref::<AtomwriteError>();
        assert!(
            matches!(
                ae,
                Some(AtomwriteError::LockTimeout { path, .. }) if path == &target
            ),
            "expected LockTimeout carrying the target path, got: {err:?}"
        );
        drop(guard1);
    }

    #[cfg(unix)]
    #[test]
    fn nested_acquire_on_the_same_thread_shares_the_lock() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested.txt");
        std::fs::write(&target, "x").unwrap();

        let outer = acquire_for_update(&target).unwrap();
        let begun = std::time::Instant::now();
        let inner = acquire_exclusive(&target, 50).expect("nested acquire must not block");
        assert!(begun.elapsed().as_millis() < 40, "nested acquire waited");
        drop(outer);
        assert!(
            sidecar_path(&target).exists(),
            "the real lock lives until the last guard drops"
        );
        drop(inner);
        assert!(!sidecar_path(&target).exists(), "sidecar must not survive");
        // After the last drop the lock is free for a fresh acquire.
        drop(acquire_exclusive(&target, 1000).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn a_held_lock_serialises_threads_of_one_process() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("serial.txt");
        std::fs::write(&target, "0").unwrap();
        let target = std::sync::Arc::new(target);
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let target = std::sync::Arc::clone(&target);
                std::thread::spawn(move || {
                    let _lock = acquire_for_update(&target).expect("lock");
                    let n: u32 = std::fs::read_to_string(&*target).unwrap().parse().unwrap();
                    // Widen the read-write window so an unserialised run would lose updates.
                    for _ in 0..200 {
                        std::thread::yield_now();
                    }
                    std::fs::write(&*target, (n + 1).to_string()).unwrap();
                })
            })
            .collect();
        for w in workers {
            w.join().expect("worker");
        }
        assert_eq!(std::fs::read_to_string(&*target).unwrap(), "8");
    }
}
