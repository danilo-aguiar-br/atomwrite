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
#[tracing::instrument(skip_all, fields(path = %target.display()))]
pub fn atomic_write(
    target: &Path,
    content: &[u8],
    opts: &AtomicWriteOptions,
    workspace: &Path,
) -> Result<WriteResult> {
    let start = Instant::now();

    // Step 1: validate path
    let target = crate::path_safety::validate_path(target, workspace)?;
    let resolved_durability = opts.durability.resolve(&target);
    let mut backup_method_used: Option<&'static str> = None;

    // Step 1a: G119 L1 — decide whether to create a sidecar at all. The
    // heuristic short-circuits for trivial writes (small file in a git
    // dir, plain write, set/del). This is the first line of defence
    // against sidecar pollution and prevents 60-80% of unnecessary
    // sidecar writes in agent LLM workloads.
    let sidecar_wanted =
        crate::wal::should_create_sidecar(&target, crate::wal::JournalOp::Write, opts.wal_policy);

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
            if hardlink_nlink.is_some_and(|n| n > 1) || is_symlink {
                WriteStrategy::InPlace
            } else {
                WriteStrategy::Rename
            }
        }
    };
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
        let bp = create_backup_in(&target, opts.retention, opts.backup_output_dir.as_deref())?;
        backup_method_used = Some("reflink_or_copy");
        Some(bp)
    } else {
        None
    };

    // Step 5: create parent directories
    if let Some(parent) = target.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create directories for {}", target.display()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(
                    parent,
                    fs::Permissions::from_mode(crate::constants::DIR_PERMISSIONS),
                );
            }
        }
    }

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
                    return Err(crate::error::AtomwriteError::SyntaxError {
                        path: target.to_path_buf(),
                        count: 1,
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
                    }
                    .into());
                }
            }
        }
    }

    // Step 6–9: dispatch by strategy
    let (exdev_fallback, rename_method_used) = match strategy {
        WriteStrategy::Rename => write_rename_path(
            target.as_path(),
            content,
            opts.strict_atomic,
            resolved_durability,
        )?,
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
        let _ = fs::set_permissions(&target, meta.permissions());
    }

    // Step 12: G39 — restore xattrs on the freshly written target
    let xattr_preserved = crate::xattr_restore::restore_xattrs(&target, &saved_xattrs)
        .unwrap_or_else(|e| {
            tracing::warn!(path = %target.display(), error = %e, "xattr restore failed");
            0
        });

    // Step 13: restore timestamps
    if opts.preserve_timestamps && original_meta.is_some() {
        let _ = platform::preserve_timestamps(&target, mtime, atime);
    }

    let checksum = checksum::hash_bytes(content);

    // G114: append a `Committed` journal entry to mark the write complete.
    // G119 L2: release the guard so the Drop runs and removes the sidecar.
    // We ignore errors here (best-effort) since the file is already on disk
    // and a recovery-time report will surface any orphan.
    if let Some(ref op_id) = wal_op_id_opt {
        let _ = crate::wal::journal_committed(&target, op_id);
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
    })
}
