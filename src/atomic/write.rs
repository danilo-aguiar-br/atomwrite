// SPDX-License-Identifier: MIT OR Apache-2.0

//! Orchestrates the atomic write pipeline (I/O-bound, single-file ordered).

use std::fs;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::checksum;
use crate::ndjson_types::PlatformInfo;
use crate::platform;

use super::inplace::{create_backup_in, delete_backup_quietly, write_inplace_path};
use super::rename_path::write_rename_path;
use super::syntax_heuristic::syntax_heuristic_check;
use super::types::{AtomicWriteOptions, WriteResult, WriteStrategy};

/// Write content atomically via tempfile, fsync, and rename.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the target path escapes the workspace.
/// Returns `AtomwriteError::Io` if creating, writing, or renaming the tempfile fails.
/// Returns `AtomwriteError::PermissionDenied` if the target directory is not writable.
/// Returns `AtomwriteError::DiskFull` if the filesystem runs out of space during write.
/// Returns `AtomwriteError::LockTimeout` (exit 83) if another process holds the
/// advisory lock of the target for longer than `LOCK_ACQUIRE_TIMEOUT_MS`.
///
/// # Examples
///
/// ```
/// use atomwrite::atomic::{AtomicWriteOptions, atomic_write};
/// use atomwrite::checksum::hash_bytes;
///
/// let dir = tempfile::tempdir()?;
/// let target = dir.path().join("notes.txt");
/// let opts = AtomicWriteOptions {
///     backup: false,
///     ..Default::default()
/// };
///
/// let first = atomic_write(&target, b"v1\n", &opts, dir.path())?;
/// assert_eq!(first.bytes_written, 3);
/// assert_eq!(first.checksum, hash_bytes(b"v1\n"));
/// assert!(first.checksum_before.is_none());
///
/// let second = atomic_write(&target, b"v2\n", &opts, dir.path())?;
/// assert_eq!(second.checksum_before, Some(first.checksum));
/// assert_eq!(std::fs::read_to_string(&target)?, "v2\n");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn atomic_write(
    target: &Path,
    content: &[u8],
    opts: &AtomicWriteOptions,
    workspace: &Path,
) -> Result<WriteResult> {
    atomic_write_based(target, content, opts, workspace, None)
}

/// Write content atomically, refusing to overwrite a file that changed after the
/// caller read it.
///
/// For read-modify-write verbs: `expected_base` is the BLAKE3 hex checksum of the
/// content the new content was computed from. Under the advisory lock the
/// current file is hashed and compared; a different file (or a missing one)
/// fails with `AtomwriteError::StateDrift` (exit 82, retryable) and writes
/// nothing. This is the net for filesystems without `flock` and for callers that
/// did not hold the lock while reading. `None` behaves like [`atomic_write`].
///
/// # Errors
///
/// Same as [`atomic_write`], plus `AtomwriteError::StateDrift` (exit 82) when the
/// file no longer matches `expected_base`.
#[tracing::instrument(skip_all, fields(path = %target.display()))]
pub fn atomic_write_based(
    target: &Path,
    content: &[u8],
    opts: &AtomicWriteOptions,
    workspace: &Path,
    expected_base: Option<&str>,
) -> Result<WriteResult> {
    // The copy is created deep inside the write, so the path is collected here
    // and attached to any failure: a run that failed after copying would
    // otherwise leave a file on disk that no record names.
    let mut backup_made: Option<std::path::PathBuf> = None;
    match atomic_write_based_inner(
        target,
        content,
        opts,
        workspace,
        expected_base,
        &mut backup_made,
    ) {
        Ok(result) => Ok(result),
        // The path rides as an anyhow context, not as a new error root, so the
        // typed error underneath stays reachable by `downcast_ref` and the
        // exit code keeps naming what actually failed.
        Err(error) => match backup_made {
            Some(backup_path) => Err(error.context(WriteFailure { backup_path })),
            None => Err(error),
        },
    }
}

/// A failed write naming the copy it had already created.
///
/// The copy exists so the caller can undo a write, and a failure after it is
/// the moment it matters most. Carrying the path on the error keeps the caller
/// from guessing at a timestamped filename it cannot reconstruct.
#[derive(Debug)]
pub struct WriteFailure {
    /// Copy this call created before failing.
    pub backup_path: std::path::PathBuf,
}

impl std::fmt::Display for WriteFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a copy of the file stayed on disk at {}",
            self.backup_path.display()
        )
    }
}

/// Body of [`atomic_write_based`], reporting the copy it made through `out`.
fn atomic_write_based_inner(
    target: &Path,
    content: &[u8],
    opts: &AtomicWriteOptions,
    workspace: &Path,
    expected_base: Option<&str>,
    backup_out: &mut Option<std::path::PathBuf>,
) -> Result<WriteResult> {
    let start = Instant::now();

    // Step 1: validate path
    let target = crate::path_safety::validate_path(target, workspace)?;
    let resolved_durability = opts.durability.resolve(&target);
    let mut backup_method_used: Option<&'static str> = None;
    let mut backup_skipped: Option<String> = None;

    // Step 1-lock: R23 — create the parent first (the lock sidecar lives in
    // it), then take the advisory lock BEFORE anything reads the target.
    // `_lock` lives to the end of the function: it covers the rename, the
    // directory fsync and the commit, and releases on every error path.
    ensure_parent_dir(&target)?;
    let lock =
        crate::lock::acquire_exclusive(&target, crate::config::tuning().lock_acquire_timeout_ms)?;

    // Step 1-base: the caller computed the new content from the file as it read
    // it. Under the lock, refuse to overwrite a file that has changed since.
    // Checked before any journal is opened so a refusal leaves nothing behind.
    if let Some(expected) = expected_base {
        let actual = if target.exists() {
            Some(checksum::hash_file(&target, u64::MAX)?)
        } else {
            None
        };
        if actual.as_deref() != Some(expected) {
            return Err(crate::error::AtomwriteError::StateDrift {
                path: target,
                expected: expected.to_string(),
                actual: actual
                    .unwrap_or_else(|| crate::constants::MISSING_FILE_CHECKSUM.to_string()),
            }
            .into());
        }
    }

    // Step 1a: G119 L1 — decide whether to create a sidecar at all. The
    // heuristic short-circuits for trivial writes (small file in a git
    // dir, plain write, set/del). This is the first line of defence
    // against sidecar pollution and prevents 60-80% of unnecessary
    // sidecar writes in agent LLM workloads.
    // H3 throttles creation only (and only after L1 said yes), so a flood of
    // writes in one process stops opening journals instead of leaving them
    // behind after the commit.
    let sidecar_wanted =
        crate::wal::should_create_sidecar(&target, crate::wal::JournalOp::Write, opts.wal_policy)
            && !crate::wal::creation_throttled(opts.wal_policy);

    // Step 1b: G114 — append a `Started` WAL entry. On crash, the
    // orphan journal surfaces `expected_new_checksum` for recovery.
    // G119 L2 — wrap the sidecar in a `JournalGuard` so the sidecar is
    // automatically removed on normal scope exit (after `Committed`).
    // `wal_guard.keep_on_drop` starts as `true` (safe-by-default) and is
    // flipped to `false` by `wal_guard.release()` only after the rename
    // and the `Committed` entry succeed. We swallow errors here because
    // journaling is best-effort and must never block the actual write.
    let new_checksum = blake3::hash(content);
    let (wal_op_id, mut wal_guard) = if sidecar_wanted {
        match crate::wal::journal_started_with_guard(
            &target,
            crate::wal::JournalOp::Write,
            None, // checksum_before filled in just below
            new_checksum,
        ) {
            Ok(pair) => pair,
            Err(_) => (String::new(), crate::wal::JournalGuard::inert()),
        }
    } else {
        // L1 suppression: no sidecar, no guard, no recovery metadata.
        // The write is still atomic via the tempfile+rename pipeline;
        // only the WAL layer is bypassed.
        tracing::debug!(
            path = %target.display(),
            policy = opts.wal_policy.as_str(),
            "G119 L1: sidecar suppressed by wal-policy"
        );
        (String::new(), crate::wal::JournalGuard::inert())
    };
    let wal_op_id_opt: Option<String> = if wal_op_id.is_empty() {
        None
    } else {
        Some(wal_op_id)
    };

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
        if nlink > 1 { Some(nlink) } else { None }
    } else {
        None
    };
    #[cfg(not(unix))]
    let hardlink_nlink: Option<u64> = None;

    // Step 2c: G39 — capture xattrs before any modification
    let saved_xattrs = crate::xattr_restore::save_xattrs(&target).unwrap_or_else(|e| {
        tracing::warn!(path = %target.display(), error = %e, "xattr save failed; continuing");
        Vec::new()
    });
    let xattr_count = saved_xattrs.len() as u32;

    // Step 2d: G55 — auto-detect strategy. Hardlinks and symlinks force InPlace.
    let is_symlink = {
        let sm = fs::symlink_metadata(&target);
        sm.as_ref().map(fs::Metadata::is_symlink).unwrap_or(false)
    };
    let strategy = match opts.strategy {
        Some(s) => s,
        None => {
            // A rename needs a tempfile beside the target, so a directory this
            // process cannot write to left the write with no atomic path at all
            // and answered exit 13 for a file the caller owns. The in-place
            // path needs the file's own permission and nothing else, which is
            // exactly what the lock already proved this run has.
            if hardlink_nlink.is_some_and(|n| n > 1) || is_symlink || !parent_is_writable(&target) {
                WriteStrategy::InPlace
            } else {
                WriteStrategy::Rename
            }
        }
    };
    // `--strict-atomic` promises a single atomic step. The rename path delivers
    // it, and it also refuses the EXDEV copy fallback on its own. The in-place
    // path cannot: it truncates and rewrites the same inode, so a crash between
    // the two leaves a half-written file. Refusing it here keeps the flag's
    // promise true wherever the strategy was chosen for us.
    if opts.strict_atomic && matches!(strategy, WriteStrategy::InPlace) {
        return Err(crate::error::AtomwriteError::ExdevFallbackDisabled {
            path: target,
            reason: if is_symlink {
                "the target is a symlink, so the write must keep the existing inode"
            } else {
                "the target has more than one hardlink, so the write must keep the existing inode"
            }
            .into(),
        }
        .into());
    }
    if matches!(strategy, WriteStrategy::InPlace) {
        if let Some(n) = hardlink_nlink {
            tracing::info!(
                path = %target.display(),
                nlink = n,
                "auto-switched to InPlace to preserve hardlink(s)"
            );
        } else if is_symlink {
            tracing::info!(
                path = %target.display(),
                "auto-switched to InPlace because target is a symlink"
            );
        }
    }

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
        // v0.1.30: create_backup_in ALWAYS uses reflink_or_copy (never hardlink).
        // Do not probe nlink — a hardlink target can make nlink>1 on the copy
        // path only if create failed into hardlink, which we forbid.
        match create_backup_in(&target, opts.retention, opts.backup_output_dir.as_deref()) {
            Ok(bp) => {
                backup_method_used = Some("reflink_or_copy");
                *backup_out = Some(bp.clone());
                Some(bp)
            }
            // A backup that cannot be created because the directory denies the
            // entry does not mean the file cannot be written: an in-place write
            // needs permission on the inode, not on the directory. Refusing here
            // blamed the file for a directory the caller cannot change, so the
            // write proceeds and the missing copy is stated on stderr.
            Err(e)
                if crate::runtime::find_io_error(&e)
                    == Some(std::io::ErrorKind::PermissionDenied) =>
            {
                use std::io::Write as _;
                crate::best_effort::best_effort(
                    writeln!(
                        std::io::stderr().lock(),
                        "atomwrite: backup skipped for {}: {}; writing without a backup",
                        target.display(),
                        e
                    ),
                    "report skipped backup on stderr",
                );
                tracing::warn!(
                    path = %target.display(),
                    error = %e,
                    "backup skipped by directory permission; continuing without a copy"
                );
                backup_skipped = Some(format!(
                    "the directory refused the backup file: {e}; the write proceeds without a copy"
                ));
                None
            }
            Err(e) => {
                // Nothing was swapped, so the journal has nothing to recover:
                // leaving it behind would make `wal-heal` report an orphan for
                // a run that never touched the target.
                wal_guard.discard();
                return Err(e);
            }
        }
    } else {
        None
    };

    // Step 5: parent directories already exist (see `ensure_parent_dir`).

    // Step 5.5: G72 — optional post-write syntax check.
    // v0.1.12: real tree-sitter parse via `crate::syntax_check`. Falls
    // back to a lightweight bracket-balance heuristic for unknown
    // languages. Auto-skips when no parser is available for the
    // detected extension. See `src/syntax_check.rs` for the full
    // algorithm.
    let syntax_errors: u32 = 0;
    if opts.syntax_check {
        match crate::syntax_check::syntax_check(&target, content) {
            Ok(crate::syntax_check::SyntaxCheckResult::Ok) => {}
            Ok(crate::syntax_check::SyntaxCheckResult::Skipped { .. }) => {
                // No parser for this language; keep the lightweight
                // heuristic as a final safety net.
                if let Some(reason) = syntax_heuristic_check(content) {
                    tracing::warn!(
                        path = %target.display(),
                        reason = %reason,
                        "G72 syntax heuristic (no tree-sitter parser) failed"
                    );
                    // The journal stays: this is a failure, not a cancel, and
                    // `wal-heal` is the reader that decides what a journal with
                    // no rename behind it means. `tests/cli_v0137_wal_cancel.rs`
                    // pins that contract.
                    return Err(crate::error::AtomwriteError::SyntaxError {
                        path: target.to_path_buf(),
                        count: 1,
                        line: None,
                        line_end: None,
                    }
                    .into());
                }
            }
            Ok(crate::syntax_check::SyntaxCheckResult::Errors { count, first }) => {
                tracing::warn!(
                    path = %target.display(),
                    count = count,
                    line = first.line,
                    column = first.column,
                    kind = %first.kind,
                    message = %first.message,
                    "G72 tree-sitter syntax check failed"
                );
                return Err(crate::error::AtomwriteError::SyntaxError {
                    path: target.to_path_buf(),
                    count: count as u32,
                    line: Some(first.line),
                    // The check reports the first error node only, so the
                    // refusal states the one line it can prove. A guessed end
                    // would claim a block the tree never showed.
                    line_end: Some(first.line),
                }
                .into());
            }
            Err(e) => {
                tracing::warn!(
                    path = %target.display(),
                    error = %e,
                    "G72 tree-sitter check errored; falling back to heuristic"
                );
                if let Some(_reason) = syntax_heuristic_check(content) {
                    return Err(crate::error::AtomwriteError::SyntaxError {
                        path: target.to_path_buf(),
                        count: 1,
                        line: None,
                        line_end: None,
                    }
                    .into());
                }
            }
        }
    }

    // Revalidate immediately before the swap. The lock was taken on an inode,
    // and between that moment and this one the path may have been pointed at
    // another file; swapping then would overwrite content this run never read.
    // Nothing was written yet, so the journal goes with the refusal.
    //
    // A read-modify-write caller already proved, under this same lock, that the
    // bytes on disk are the bytes it read: `expected_base` is checked before
    // any journal exists. When that check passed, a different inode carrying
    // the same content is the normal outcome of a concurrent writer that
    // renamed its result into place, and refusing it would turn a healthy
    // pipeline into a drift error.
    if expected_base.is_none() && !lock.target_unchanged() {
        wal_guard.discard();
        return Err(crate::error::AtomwriteError::StateDrift {
            path: target,
            expected: "the inode the lock was taken on".to_string(),
            actual: "another inode now answers to the same path".to_string(),
        }
        .into());
    }

    // Step 6–9: dispatch by strategy
    let (exdev_fallback, rename_method_used) = match strategy {
        WriteStrategy::Rename => match write_rename_path(
            target.as_path(),
            content,
            opts.strict_atomic,
            resolved_durability,
        ) {
            Ok(done) => done,
            Err(e) => {
                // The rename failed, so the target was never swapped: it still
                // holds the old content and the tempfile is removed on drop.
                // The journal therefore has nothing to recover, and keeping it
                // made `wal-heal` offer to recover a write that never happened.
                // The cancellations and the plain failures land here alike.
                wal_guard.discard();
                return Err(e);
            }
        },
        WriteStrategy::InPlace | WriteStrategy::CopyBack => (
            write_inplace_path(target.as_path(), content)?,
            strategy.as_str(),
        ),
    };

    // Step 10: fsync parent directory (Full durability; skip for Fast)
    if !matches!(resolved_durability, crate::platform::Durability::Fast) {
        if let Some(parent) = target.parent() {
            if let Err(e) = platform::fsync_dir(parent) {
                tracing::warn!(
                    path = %parent.display(),
                    error = %e,
                    "fsync_dir after persist failed"
                );
            }
        }
    }

    // Step 11: restore permissions
    if let Some(ref meta) = original_meta {
        if let Err(e) = fs::set_permissions(&target, meta.permissions()) {
            tracing::warn!(path = %target.display(), error = %e, "permission restore failed");
        }
    }

    // Step 12: G39 — restore xattrs on the freshly written target
    let xattr_preserved = crate::xattr_restore::restore_xattrs(&target, &saved_xattrs)
        .unwrap_or_else(|e| {
            tracing::warn!(path = %target.display(), error = %e, "xattr restore failed");
            0
        });

    // Step 13: restore timestamps
    if opts.preserve_timestamps && original_meta.is_some() {
        if let Err(e) = platform::preserve_timestamps(&target, mtime, atime) {
            tracing::warn!(path = %target.display(), error = %e, "timestamp restore failed");
        }
    }

    let checksum = checksum::hash_bytes(content);

    // G114: append a `Committed` journal entry to mark the write complete.
    // G119 L2: release the guard so the Drop runs and removes the sidecar.
    // We ignore errors here (best-effort) since the file is already on disk
    // and a recovery-time report will surface any orphan.
    if let Some(ref op_id) = wal_op_id_opt {
        crate::best_effort::best_effort(
            crate::wal::journal_committed(&target, op_id),
            "append Committed journal entry",
        );
        wal_guard.release();
    } else {
        // No WAL was created (best-effort fallback) — guard is inert.
        wal_guard.keep();
    }

    // v0.1.21 GAP-014 v2: delete backup quietly after successful write
    // when the caller did not request retention. Idempotent: NotFound is
    // treated as success. Errors other than NotFound are logged at WARN
    // level but do NOT propagate — the user's write already succeeded.
    // GAP-101: clear backup_path when the file is deleted so NDJSON
    // never reports a path that does not exist on disk.
    let backup_path = if let Some(ref bp) = backup_path {
        if !opts.keep_backup {
            delete_backup_quietly(bp);
            None
        } else {
            Some(bp.clone())
        }
    } else {
        None
    };

    Ok(WriteResult {
        bytes_written: content.len() as u64,
        checksum,
        checksum_before,
        backup_path: backup_path.map(|p| p.display().to_string()),
        elapsed_ms: start.elapsed().as_millis() as u64,
        platform: PlatformInfo {
            fsync: platform::platform_fsync_name(),
            dir_fsync: platform::platform_dir_fsync_name(),
            durability: Some(resolved_durability.as_str()),
            rename_method: Some(rename_method_used),
            backup_method: backup_method_used,
        },
        hardlink_nlink,
        write_strategy: strategy.as_str(),
        xattr_preserved,
        xattr_count,
        exdev_fallback,
        syntax_errors,
        durability: resolved_durability.as_str(),
        rename_method: rename_method_used,
        backup_method: backup_method_used,
        backup_skipped,
    })
}

/// Step 5: create missing parent directories (mode `DIR_PERMISSIONS` on Unix).
/// True when the target's directory accepts a new entry from this process.
///
/// Only the directory's own permission is asked: creating the tempfile is the
/// operation that decides the strategy. Platforms with no `access(2)` answer
/// `true`, which keeps the rename path and its atomicity as the default.
#[cfg(unix)]
fn parent_is_writable(target: &Path) -> bool {
    use nix::unistd::{AccessFlags, access};
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    access(parent, AccessFlags::W_OK).is_ok()
}

#[cfg(not(unix))]
fn parent_is_writable(_target: &Path) -> bool {
    true
}

fn ensure_parent_dir(target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create directories for {}", target.display()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                crate::best_effort::best_effort(
                    fs::set_permissions(
                        parent,
                        fs::Permissions::from_mode(crate::constants::DIR_PERMISSIONS),
                    ),
                    "set mode on created parent directory",
                );
            }
        }
    }
    Ok(())
}
