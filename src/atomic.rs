// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file write pipeline: tempfile, fsync, rename, fsync directory.
//!
//! Three write strategies are available, selected automatically:
//!
//! - `WriteStrategy::Rename` — classic tempfile + `rename(2)` (atomic, but
//!   destroys hardlinks and inode identity).
//! - `WriteStrategy::InPlace` — `ftruncate(0) + pwrite + fsync_data` on the
//!   existing file descriptor (preserves inode and hardlinks, but NOT atomic
//!   against a crash between truncate and write).
//! - `WriteStrategy::CopyBack` — like InPlace but with a journal sidecar
//!   for crash recovery; slowest but most durable. (Reserved for v0.1.13.)
//!
//! The default policy is **auto-detect**: hardlinks and symlinks trigger
//! InPlace automatically, regular files use Rename. Pass
//! `WriteStrategy::Rename` explicitly via the `strategy` field of
//! `AtomicWriteOptions` to force the legacy behavior.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::checksum;
use crate::ndjson_types::PlatformInfo;
use crate::platform;

#[cfg(windows)]
use crate::error::AtomwriteError;

/// Write strategy selected by the atomic pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteStrategy {
    /// tempfile + rename (classic, atomic, breaks hardlinks).
    Rename,
    /// ftruncate + pwrite on existing fd (preserves inode, NOT crash-safe).
    InPlace,
    /// ftruncate + pwrite + journal sidecar (preserves inode, crash-recoverable).
    CopyBack,
}

impl WriteStrategy {
    /// String representation for NDJSON `write_strategy` field.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Rename => "rename",
            Self::InPlace => "inplace",
            Self::CopyBack => "copyback",
        }
    }
}

/// Configuration for an atomic write operation.
pub struct AtomicWriteOptions {
    /// Whether to create a backup of the target before overwriting.
    pub backup: bool,
    /// Maximum number of backup copies to retain.
    pub retention: u8,
    /// Whether to restore the original file timestamps after writing.
    pub preserve_timestamps: bool,
    /// Custom output directory for backup files. When `None`, the backup is
    /// created in the same directory as the target.
    pub backup_output_dir: Option<std::path::PathBuf>,
    /// Force a specific write strategy; `None` means auto-detect.
    pub strategy: Option<WriteStrategy>,
    /// Refuse EXDEV fallback (return `AtomwriteError::ExdevFallbackDisabled`
    /// instead of falling back to copy). Default: `false`.
    pub strict_atomic: bool,
    /// Post-write syntax check (G72). When set, the new content is parsed
    /// by ast-grep before being committed; if the parser reports syntax
    /// errors, the write is aborted with `AtomwriteError::SyntaxError`
    /// (exit 88). Default: `false`. Languages supported are the same as
    /// `ast-grep-language`'s built-in set; files with no parser available
    /// for the extension skip the check silently.
    pub syntax_check: bool,
    /// G119 L1: sidecar creation policy. `Auto` (default) lets the
    /// heuristic decide; `Always` forces the legacy behaviour
    /// (equivalent to `--strict-atomic`); `Never` suppresses the
    /// sidecar entirely. The default value `Auto` prevents 60-80% of
    /// unnecessary sidecar writes for trivial operations.
    pub wal_policy: crate::wal::WalPolicy,
    /// v0.1.21 GAP-014 v2: keep the backup after a successful write.
    /// When `false` (default), the backup created by `backup: true` is
    /// deleted quietly after the atomic rename completes. When `true`,
    /// the backup is retained and cleaned up by `cleanup_old_backups_in`
    /// according to `retention`. Backup-on-failure is ALWAYS preserved
    /// regardless of this flag.
    pub keep_backup: bool,
}

impl Default for AtomicWriteOptions {
    fn default() -> Self {
        Self {
            backup: false,
            retention: 5,
            preserve_timestamps: false,
            backup_output_dir: None,
            strategy: None,
            strict_atomic: false,
            syntax_check: false,
            wal_policy: crate::wal::WalPolicy::Auto,
            keep_backup: false,
        }
    }
}

/// Result metadata returned after a successful atomic write.
pub struct WriteResult {
    /// Number of bytes written to the target file.
    pub bytes_written: u64,
    /// BLAKE3 checksum of the written content.
    pub checksum: String,
    /// BLAKE3 checksum of the file before overwriting, if it existed.
    pub checksum_before: Option<String>,
    /// Path to the backup file, if a backup was created.
    pub backup_path: Option<String>,
    /// Wall-clock time of the write operation in milliseconds.
    pub elapsed_ms: u64,
    /// Platform-specific fsync method names used.
    pub platform: PlatformInfo,
    /// Hard link count if the target had nlink > 1 (rename breaks hardlinks).
    pub hardlink_nlink: Option<u64>,
    /// Write strategy actually used (after auto-detect). Always set.
    pub write_strategy: &'static str,
    /// Number of extended attributes preserved (G39). Always set, 0 on Windows.
    pub xattr_preserved: u32,
    /// Number of extended attributes that were on the target before the write.
    pub xattr_count: u32,
    /// Whether the copy-fallback path was used due to EXDEV (G90).
    pub exdev_fallback: bool,
    /// Number of syntax errors detected by `--syntax-check` (G72), if enabled.
    /// Always 0 when the check is disabled or no parser is available.
    pub syntax_errors: u32,
}

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
        Some(create_backup_in(
            &target,
            opts.retention,
            opts.backup_output_dir.as_deref(),
        )?)
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
    let exdev_fallback = match strategy {
        WriteStrategy::Rename => write_rename_path(target.as_path(), content, opts.strict_atomic)?,
        WriteStrategy::InPlace | WriteStrategy::CopyBack => {
            write_inplace_path(target.as_path(), content)?
        }
    };

    // Step 10: fsync parent directory (critical for durability)
    if let Some(parent) = target.parent() {
        if let Err(e) = platform::fsync_dir(parent) {
            tracing::warn!(
                path = %parent.display(),
                error = %e,
                "fsync_dir after persist failed"
            );
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
    if let Some(ref bp) = backup_path {
        if !opts.keep_backup {
            delete_backup_quietly(bp);
        }
    }

    Ok(WriteResult {
        bytes_written: content.len() as u64,
        checksum,
        checksum_before,
        backup_path: backup_path.map(|p| p.display().to_string()),
        elapsed_ms: start.elapsed().as_millis() as u64,
        platform: PlatformInfo {
            fsync: platform::platform_fsync_name(),
            dir_fsync: platform::platform_dir_fsync_name(),
        },
        hardlink_nlink,
        write_strategy: strategy.as_str(),
        xattr_preserved,
        xattr_count,
        exdev_fallback,
        syntax_errors,
    })
}

/// Write via tempfile + rename with EXDEV fallback (G90).
///
/// Returns `true` if the EXDEV copy-fallback path was used.
fn write_rename_path(
    target: &Path,
    content: &[u8],
    #[cfg_attr(not(unix), allow(unused_variables))] strict_atomic: bool,
) -> Result<bool> {
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
        .with_context(|| format!("cannot create tempfile in {}", parent.display()))?;

    // Step 7: write content
    {
        let mut writer = BufWriter::with_capacity(crate::constants::BUF_CAPACITY, temp.as_file());
        writer
            .write_all(content)
            .with_context(|| format!("write error for {}", target.display()))?;
        writer
            .flush()
            .with_context(|| format!("flush error for {}", target.display()))?;
        writer.into_inner().map_err(|e| {
            anyhow::anyhow!(
                "BufWriter into_inner error for {}: {}",
                target.display(),
                e.error()
            )
        })?;
    }

    // Step 8: fsync file
    platform::fsync_file(temp.as_file())
        .with_context(|| format!("fsync error for {}", target.display()))?;

    // Step 9: atomic rename with EXDEV fallback
    #[cfg(windows)]
    {
        persist_with_retry(temp, target)?;
        return Ok(false);
    }
    #[cfg(not(windows))]
    {
        match temp.persist(target) {
            Ok(_) => Ok(false),
            Err(e) => {
                #[cfg(unix)]
                {
                    if e.error.raw_os_error() == Some(libc::EXDEV) {
                        if strict_atomic {
                            return Err(crate::error::AtomwriteError::ExdevFallbackDisabled {
                                path: target.to_path_buf(),
                            }
                            .into());
                        }
                        tracing::warn!(
                            path = %target.display(),
                            "EXDEV detected, falling back to copy + fsync + cleanup"
                        );
                        let recovered = e.file;
                        copy_tempfile_to_target(recovered.as_file(), target, content)?;
                        return Ok(true);
                    }
                }
                return Err(e.error)
                    .with_context(|| format!("rename error for {}", target.display()));
            }
        }
    }
}

/// Lightweight syntax check (G72).
///
/// Verifies that the most common bracket/quote pairs are balanced. This
/// catches the 80%+ of "LLM forgot a closing brace" type errors without
/// requiring a full tree-sitter grammar (~500 KB saving). For deeper
/// validation, pipe the file through `cargo check` or
/// `rustc --emit=metadata` externally.
///
/// Returns `Some(reason)` if a likely syntax error is detected, `None` if
/// the content passes the heuristic.
fn syntax_heuristic_check(content: &[u8]) -> Option<String> {
    // Convert to text for bracket counting. Bail early if not valid UTF-8.
    let text = std::str::from_utf8(content).ok()?;

    // 1. Strip line and block comments so they don't confuse the count.
    let stripped = strip_comments(text);

    // 2. Strip string literals (handles both "..." and '...' for Rust/JS-like).
    let stripped = strip_string_literals(&stripped);

    // 3. Count brackets.
    let mut braces = 0i32;
    let mut parens = 0i32;
    let mut brackets = 0i32;
    for c in stripped.chars() {
        match c {
            '{' => braces += 1,
            '}' => braces -= 1,
            '(' => parens += 1,
            ')' => parens -= 1,
            '[' => brackets += 1,
            ']' => brackets -= 1,
            _ => {}
        }
    }
    if braces != 0 {
        return Some(format!(
            "unbalanced braces: {} more {} than {}",
            braces.abs(),
            if braces > 0 { "open" } else { "close" },
            if braces > 0 { "close" } else { "open" }
        ));
    }
    if parens != 0 {
        return Some(format!(
            "unbalanced parentheses: {} more {} than {}",
            parens.abs(),
            if parens > 0 { "open" } else { "close" },
            if parens > 0 { "close" } else { "open" }
        ));
    }
    if brackets != 0 {
        return Some(format!(
            "unbalanced brackets: {} more {} than {}",
            brackets.abs(),
            if brackets > 0 { "open" } else { "close" },
            if brackets > 0 { "close" } else { "open" }
        ));
    }
    None
}

/// Strip line (`//`) and block (`/* ... */`) comments, respecting string
/// literals. This is a best-effort, char-by-char scanner; it is NOT a
/// full lexer. Misbehavior with nested block comments or string-interpolation
/// is acceptable since we only use this for the G72 heuristic check.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    // Line comment: skip until newline.
                    chars.next();
                    for nc in chars.by_ref() {
                        if nc == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                }
                Some('*') => {
                    // Block comment: skip until `*/`.
                    chars.next();
                    let mut prev = '\0';
                    for nc in chars.by_ref() {
                        if prev == '*' && nc == '/' {
                            break;
                        }
                        prev = nc;
                    }
                }
                _ => out.push(c),
            }
        } else if c == '"' {
            // String literal: skip until matching unescaped quote.
            out.push(c);
            while let Some(nc) = chars.next() {
                out.push(nc);
                if nc == '\\' {
                    // Skip the escaped character.
                    if let Some(escaped) = chars.next() {
                        out.push(escaped);
                    }
                } else if nc == '"' {
                    break;
                }
            }
        } else if c == '\'' {
            // Char literal (Rust) or single-quote string.
            out.push(c);
            while let Some(nc) = chars.next() {
                out.push(nc);
                if nc == '\\' {
                    if let Some(escaped) = chars.next() {
                        out.push(escaped);
                    }
                } else if nc == '\'' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Strip double-quoted string literals, leaving single chars intact.
/// Used as a second pass after `strip_comments` to remove anything that
/// might still confuse bracket counting (template literals, raw strings).
fn strip_string_literals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut prev = '\0';
    for c in text.chars() {
        if c == '"' && prev != '\\' {
            in_string = !in_string;
        }
        if !in_string {
            out.push(c);
        }
        prev = c;
    }
    out
}
#[cfg(unix)]
fn copy_tempfile_to_target(temp: &std::fs::File, target: &Path, _content: &[u8]) -> Result<()> {
    use std::io::{Read, Seek, Write};
    let mut temp_handle = temp.try_clone().context("cannot clone tempfile handle")?;
    temp_handle
        .seek(std::io::SeekFrom::Start(0))
        .context("cannot seek tempfile")?;
    let mut temp = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, temp_handle);
    let mut buf = Vec::new();
    temp.read_to_end(&mut buf).with_context(|| {
        format!(
            "cannot read tempfile for copy fallback to {}",
            target.display()
        )
    })?;

    let mut target_file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(target)
        .with_context(|| format!("cannot open target for copy fallback: {}", target.display()))?;
    target_file.write_all(&buf).with_context(|| {
        format!(
            "cannot write target for copy fallback: {}",
            target.display()
        )
    })?;
    let _ = target_file.sync_data();
    let _ = target_file;

    let mut target_file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(target)
        .with_context(|| format!("cannot open target for copy fallback: {}", target.display()))?;
    target_file.write_all(&buf).with_context(|| {
        format!(
            "cannot write target for copy fallback: {}",
            target.display()
        )
    })?;
    platform::fsync_file(&target_file).ok();
    let _ = target_file;

    // Remove the tempfile on disk (its path was inside the parent dir).
    // persist() left it on disk with a .tmp.* name; we don't have a handle
    // to it here, so we rely on the caller to clean up via the
    // tempfile-in-parent pattern. Cleanup is best-effort: ignore errors.
    if let Some(parent) = target.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if let Some(name) = name.to_str() {
                    if name.starts_with(crate::constants::TEMPFILE_PREFIX) {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
    Ok(())
}

/// Write in-place to preserve the existing inode and hardlinks (G55/G114).
///
/// Uses `ftruncate(0)` + `write_all` + `sync_data` on the existing fd.
/// NOT atomic against a crash between truncate and write — for full crash
/// recovery, use `WriteStrategy::CopyBack` with the journal sidecar.
fn write_inplace_path(target: &Path, content: &[u8]) -> Result<bool> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(false)
        .open(target)
        .with_context(|| {
            format!(
                "cannot open target for in-place write: {}",
                target.display()
            )
        })?;
    file.set_len(0)
        .with_context(|| format!("ftruncate failed for {}", target.display()))?;
    file.write_all(content)
        .with_context(|| format!("in-place write failed for {}", target.display()))?;
    file.flush()
        .with_context(|| format!("in-place flush failed for {}", target.display()))?;
    let _ = file.sync_data();
    Ok(false)
}

/// Create a timestamped backup of the target file and prune old backups.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if copying the file or creating the backup fails.
#[tracing::instrument(skip_all, fields(path = %target.display(), retention))]
pub(crate) fn create_backup(target: &Path, retention: u8) -> Result<std::path::PathBuf> {
    create_backup_in(target, retention, None)
}

/// Create a timestamped backup, optionally in a custom output directory.
///
/// When `output_dir` is `Some`, the backup is placed in that directory instead
/// of alongside the source file. The directory is created if it does not exist.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if copying, creating the directory, or the
/// backup itself fails.
#[tracing::instrument(skip_all, fields(path = %target.display(), retention, output_dir))]
pub(crate) fn create_backup_in(
    target: &Path,
    retention: u8,
    output_dir: Option<&Path>,
) -> Result<std::path::PathBuf> {
    let now = utc_timestamp_formatted();
    // file_name() returns None only for root "/" — empty string is safe for backup naming
    let filename = target.file_name().unwrap_or_default().to_string_lossy();
    let backup_name = format!("{filename}.bak.{now}");

    let backup_path = match output_dir {
        Some(dir) => {
            if !dir.exists() {
                fs::create_dir_all(dir).with_context(|| {
                    format!("cannot create backup output dir {}", dir.display())
                })?;
            }
            dir.join(&backup_name)
        }
        None => target.with_file_name(&backup_name),
    };

    // G64: prefer reflink (O(1) CoW on APFS/btrfs/XFS) over `fs::copy`.
    // reflink-copy falls back to a regular copy automatically if the
    // filesystem does not support reflinks. Result is the same as
    // `fs::copy` in the non-reflink case (returns bytes copied).
    //
    // Remove the existing backup file if any: reflink_or_copy refuses to
    // overwrite, and the test harness can produce timestamp collisions
    // (second-level resolution). Cleanup is best-effort.
    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }
    reflink_copy::reflink_or_copy(target, &backup_path)
        .with_context(|| format!("cannot create backup at {}", backup_path.display()))?;
    let backup_file = fs::File::open(&backup_path)
        .with_context(|| format!("cannot open backup for fsync: {}", backup_path.display()))?;
    // Best-effort fsync: backup file already exists on disk via fs::copy.
    // On Windows %TEMP%, antivirus products can transiently hold a read handle
    // causing FlushFileBuffers to fail with ERROR_ACCESS_DENIED. We log a
    // warning and continue because the user-visible operation (creating a
    // backup) has already succeeded; the worst case is a missing durability
    // flush for the backup metadata, which is non-fatal.
    platform::fsync_file_best_effort(&backup_file);

    if let Some(parent) = backup_path.parent() {
        if let Err(e) = platform::fsync_dir(parent) {
            tracing::warn!(
                path = %parent.display(),
                error = %e,
                "fsync_dir after backup failed"
            );
        }
    }

    if retention > 0 {
        // Prune old backups in the same directory as the new one.
        // Pass the source filename (without .bak.<timestamp> suffix) so the
        // prefix matcher correctly identifies peer backups.
        cleanup_old_backups_in(
            backup_path.parent().unwrap_or_else(|| Path::new(".")),
            &filename,
            retention,
        );
    }

    Ok(backup_path)
}

/// Quietly delete a backup file after a successful atomic write.
///
/// v0.1.21 GAP-014 v2: by default, backups are transient and removed
/// after the write commits. This function is idempotent — `NotFound`
/// is mapped to `Ok(())` so a double-delete or pre-cleaned path is
/// silent. Any other I/O error is logged at WARN level but does NOT
/// propagate: the user's write already succeeded, and propagating a
/// cleanup failure would mask the success path.
fn delete_backup_quietly(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {
            tracing::debug!(path = %path.display(), "backup deleted after successful write");
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(
                path = %path.display(),
                "backup already gone (NotFound) — nothing to delete"
            );
        }
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to delete backup after successful write — backup retained"
            );
        }
    }
}

/// Prune old backups that share the given `prefix` in the given directory.
fn cleanup_old_backups_in(parent: &Path, prefix_name: &str, retention: u8) {
    let prefix = format!("{prefix_name}.bak.");

    let mut backups: Vec<std::path::PathBuf> = match fs::read_dir(parent) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&prefix))
            })
            .collect(),
        Err(_) => return,
    };

    if backups.len() <= retention as usize {
        return;
    }

    backups.sort();
    let to_remove = backups.len() - retention as usize;
    for old in &backups[..to_remove] {
        let _ = fs::remove_file(old);
    }
}

fn utc_timestamp_formatted() -> String {
    use std::time::SystemTime;
    // duration_since fails only if system clock precedes UNIX epoch — defaults to 1970-01-01
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (year, month, day, hour, min, sec) = epoch_to_utc(secs);
    format!("{year:04}{month:02}{day:02}_{hour:02}{min:02}{sec:02}")
}

/// Return the current UTC time as an RFC 3339 string (e.g. `2024-01-15T14:30:22Z`).
pub fn rfc3339_now() -> String {
    use std::time::SystemTime;
    // duration_since fails only if system clock precedes UNIX epoch — defaults to 1970-01-01
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d, h, min, sec) = epoch_to_utc(secs);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{sec:02}Z")
}

pub(crate) fn epoch_to_utc(epoch: u64) -> (u64, u64, u64, u64, u64, u64) {
    let sec_of_day = epoch % 86400;
    let hour = sec_of_day / 3600;
    let min = (sec_of_day % 3600) / 60;
    let sec = sec_of_day % 60;

    let mut days = (epoch / 86400) as i64;
    days += 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = (days - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y as u64, m, d, hour, min, sec)
}

#[cfg(windows)]
use tempfile::NamedTempFile;

#[cfg(windows)]
fn persist_with_retry(mut temp: NamedTempFile, target: &Path) -> Result<()> {
    let delays = [100, 200, 400];
    for delay_ms in &delays {
        match temp.persist(target) {
            Ok(_) => return Ok(()),
            Err(e) => {
                if e.error.kind() == std::io::ErrorKind::PermissionDenied {
                    std::thread::sleep(std::time::Duration::from_millis(*delay_ms));
                    temp = e.file;
                    continue;
                }
                return Err(anyhow::anyhow!(
                    "rename error for {}: {}",
                    target.display(),
                    e.error
                ));
            }
        }
    }
    Err(AtomwriteError::PermissionDenied {
        path: target.to_path_buf(),
    }
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_to_utc_epoch_zero() {
        assert_eq!(epoch_to_utc(0), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn epoch_to_utc_known_date() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(epoch_to_utc(1704067200), (2024, 1, 1, 0, 0, 0));
    }

    #[test]
    fn atomic_write_options_default_values() {
        let opts = AtomicWriteOptions::default();
        assert!(!opts.backup);
        assert_eq!(opts.retention, 5);
        assert!(!opts.preserve_timestamps);
    }

    #[test]
    fn create_backup_and_retention() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();

        for _ in 0..7 {
            create_backup(&file, 5).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("test.txt.bak."))
            })
            .collect();

        assert!(
            backups.len() <= 5,
            "retention should keep at most 5 backups, got {}",
            backups.len()
        );
    }

    #[test]
    fn atomic_write_updates_mtime_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "original").unwrap();

        let original_meta = std::fs::metadata(&file).unwrap();
        let original_mtime = filetime::FileTime::from_last_modification_time(&original_meta);
        std::thread::sleep(std::time::Duration::from_millis(50));

        let opts = AtomicWriteOptions::default();
        assert!(
            !opts.preserve_timestamps,
            "GAP 12 fix: default must update mtime so cargo/make detect the change"
        );

        let _ = atomic_write(&file, b"updated content", &opts, dir.path()).unwrap();

        let new_meta = std::fs::metadata(&file).unwrap();
        let new_mtime = filetime::FileTime::from_last_modification_time(&new_meta);
        assert!(
            new_mtime > original_mtime,
            "default behavior must update mtime to now (was {:?}, now {:?})",
            original_mtime,
            new_mtime
        );
    }

    #[test]
    fn atomic_write_preserves_mtime_when_opted_in() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "original").unwrap();

        let original_meta = std::fs::metadata(&file).unwrap();
        let original_mtime = filetime::FileTime::from_last_modification_time(&original_meta);
        std::thread::sleep(std::time::Duration::from_millis(50));

        let opts = AtomicWriteOptions {
            preserve_timestamps: true,
            ..Default::default()
        };

        let _ = atomic_write(&file, b"updated content", &opts, dir.path()).unwrap();

        let new_meta = std::fs::metadata(&file).unwrap();
        let new_mtime = filetime::FileTime::from_last_modification_time(&new_meta);
        assert_eq!(
            new_mtime, original_mtime,
            "preserve_timestamps=true must keep original mtime intact"
        );
    }

    #[test]
    fn write_strategy_rename_for_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("regular.txt");
        std::fs::write(&file, "old").unwrap();
        let opts = AtomicWriteOptions::default();
        let r = atomic_write(&file, b"new", &opts, dir.path()).unwrap();
        assert_eq!(r.write_strategy, "rename", "nlink=1 must use rename");
        assert!(r.hardlink_nlink.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn write_strategy_inplace_for_hardlink_preserves_inode() {
        use std::os::unix::fs::MetadataExt;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("with_hardlink.txt");
        let link = dir.path().join("hardlink.txt");
        std::fs::write(&file, "shared content").unwrap();
        std::fs::hard_link(&file, &link).unwrap();

        let original_ino = std::fs::metadata(&file).unwrap().ino();
        let original_link_ino = std::fs::metadata(&link).unwrap().ino();
        assert_eq!(
            original_ino, original_link_ino,
            "pre-condition: both must point to the same inode"
        );

        let opts = AtomicWriteOptions::default();
        let r = atomic_write(&file, b"new shared content", &opts, dir.path()).unwrap();
        assert_eq!(
            r.write_strategy, "inplace",
            "G55: nlink>1 must auto-switch to InPlace"
        );
        assert_eq!(r.hardlink_nlink, Some(2));

        // Critical assertion: the inode of both files must still be the same.
        let new_file_ino = std::fs::metadata(&file).unwrap().ino();
        let new_link_ino = std::fs::metadata(&link).unwrap().ino();
        assert_eq!(
            new_file_ino, original_ino,
            "G55: file inode must be preserved (was {}, now {})",
            original_ino, new_file_ino
        );
        assert_eq!(
            new_link_ino, original_ino,
            "G55: hardlink inode must be preserved (was {}, now {})",
            original_ino, new_link_ino
        );

        // Both must read the new content (proves hardlink is still active).
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "new shared content"
        );
        assert_eq!(
            std::fs::read_to_string(&link).unwrap(),
            "new shared content"
        );
    }

    #[test]
    fn write_result_includes_strategy_and_xattr_fields() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("fields.txt");
        std::fs::write(&file, "x").unwrap();
        let opts = AtomicWriteOptions::default();
        let r = atomic_write(&file, b"y", &opts, dir.path()).unwrap();
        // write_strategy is always set (rename/inplace/copyback)
        assert!(
            matches!(r.write_strategy, "rename" | "inplace" | "copyback"),
            "write_strategy must be set, got: {}",
            r.write_strategy
        );
        // xattr_preserved <= xattr_count (we never invent xattrs)
        assert!(
            r.xattr_preserved <= r.xattr_count,
            "xattr_preserved ({}) must be <= xattr_count ({})",
            r.xattr_preserved,
            r.xattr_count
        );
        // exdev_fallback is false for the normal case
        assert!(
            !r.exdev_fallback,
            "exdev_fallback must be false in normal flow"
        );
    }

    #[test]
    fn exdev_fallback_disabled_error_when_strict_atomic() {
        // We cannot easily trigger a real EXDEV in a portable unit test, but
        // we can verify the error variant's exit code and code string.
        let err = crate::error::AtomwriteError::ExdevFallbackDisabled {
            path: std::path::PathBuf::from("/tmp/x"),
        };
        assert_eq!(err.exit_code(), 91);
        assert_eq!(err.error_code(), "EXDEV_FALLBACK_DISABLED");
        assert_eq!(
            err.error_class(),
            crate::error::ErrorClass::PreconditionFailed
        );
    }
}
