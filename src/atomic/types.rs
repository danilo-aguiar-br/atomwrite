// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic write types: strategy, options, result.

use crate::ndjson_types::PlatformInfo;

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
    /// Durability policy for fsync (v0.1.29 P2-1).
    pub durability: crate::platform::Durability,
}

impl Default for AtomicWriteOptions {
    fn default() -> Self {
        Self {
            backup: true,
            retention: crate::constants::DEFAULT_BACKUP_RETENTION,
            preserve_timestamps: false,
            backup_output_dir: None,
            strategy: None,
            strict_atomic: false,
            syntax_check: false,
            wal_policy: crate::wal::WalPolicy::Auto,
            keep_backup: false,
            durability: crate::platform::Durability::Auto,
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
    /// Resolved durability mode name (v0.1.29 P2-1).
    pub durability: &'static str,
    /// Rename method used (v0.1.29 P2-2).
    pub rename_method: &'static str,
    /// Backup method if backup was created (v0.1.29 P2-3).
    pub backup_method: Option<&'static str>,
}
