
/// Domain-specific errors for atomic file operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AtomwriteError {
    /// Target file does not exist.
    #[error("file not found: {path}")]
    NotFound {
        /// File path that was not found.
        path: PathBuf,
    },

    /// Caller-provided input failed validation.
    #[error("invalid input: {reason}")]
    InvalidInput {
        /// Description of the validation failure.
        reason: String,
    },

    /// Fuzzy/exact match cascade failed (v0.1.29 P0-2).
    ///
    /// Carries an optional [`BestCandidate`] so agents can correct `old`
    /// without re-reading the full file. Exit 65 / `MATCH_FAILED`.
    #[error("match failed: {reason}")]
    MatchFailed {
        /// Description of the match failure.
        reason: String,
        /// Closest near-miss region, when similarity ≥ 0.5 (boxed for clippy `result_large_err`).
        best_candidate: Option<Box<BestCandidate>>,
        /// Additional near-misses for `did_you_mean` (v0.1.30).
        candidates: Option<Vec<BestCandidate>>,
    },

    /// Multiple matches for the same `old` without `--replace-all` (v0.1.30).
    ///
    /// Exit 65 / `MATCH_AMBIGUOUS`. Agents must narrow context or pass `replace_all`.
    #[error("match ambiguous: {reason}")]
    MatchAmbiguous {
        /// Description including match count.
        reason: String,
        /// Number of matches found.
        count: u64,
        /// Optional first near-miss (unused for exact multi).
        best_candidate: Option<Box<BestCandidate>>,
    },

    /// Operation cancelled by SIGINT/SIGTERM (v0.1.29 P0-4).
    #[error("cancelled: {reason}")]
    Cancelled {
        /// Description of the cancel (signal name, path, etc.).
        reason: String,
        /// Suggested exit code (130 SIGINT, 143 SIGTERM).
        exit: u8,
    },

    /// Insufficient filesystem permissions.
    #[error("permission denied: {path}")]
    PermissionDenied {
        /// File path with insufficient permissions.
        path: PathBuf,
    },

    /// No space left on the device.
    #[error("disk full writing to {path}")]
    DiskFull {
        /// File path where the write failed.
        path: PathBuf,
    },

    /// Filesystem quota exceeded.
    #[error("quota exceeded writing to {path}")]
    QuotaExceeded {
        /// File path where quota was exceeded.
        path: PathBuf,
    },

    /// Rename attempted across different mount points.
    #[error("cross-device rename: {path}")]
    CrossDevice {
        /// File path involved in cross-device rename.
        path: PathBuf,
    },

    /// Wrapped standard I/O error.
    #[error("I/O error: {source}")]
    Io {
        /// Underlying I/O error.
        #[from]
        source: std::io::Error,
    },

    /// Invalid CLI or runtime configuration.
    #[error("invalid configuration: {reason}")]
    ConfigInvalid {
        /// Description of the configuration problem.
        reason: String,
    },

    /// File checksum changed between read and write (optimistic lock failure).
    #[error("state drift detected on {path}: expected checksum {expected}, got {actual}")]
    StateDrift {
        /// File path with checksum mismatch.
        path: PathBuf,
        /// Caller-provided expected checksum.
        expected: String,
        /// Actual checksum found on disk.
        actual: String,
    },

    /// Path resolved outside the workspace jail boundary.
    #[error("path outside workspace jail: {path} (workspace: {workspace})")]
    WorkspaceJail {
        /// Path that escaped the workspace jail.
        path: PathBuf,
        /// Workspace root used for comparison.
        workspace: PathBuf,
    },

    /// Symbolic link encountered when symlinks are disallowed.
    #[error("symlink blocked: {path}")]
    SymlinkBlocked {
        /// Symlink path that was blocked.
        path: PathBuf,
    },

    /// File has immutable attributes preventing modification.
    #[error("file is immutable: {path}")]
    FileImmutable {
        /// Immutable file path.
        path: PathBuf,
    },

    /// File detected as binary when text-only mode is required.
    #[error("binary file detected: {path}")]
    BinaryFile {
        /// Binary file path.
        path: PathBuf,
    },

    /// FIFO or named pipe detected where regular file expected.
    #[error("FIFO detected: {path}")]
    FifoDetected {
        /// FIFO path.
        path: PathBuf,
    },

    /// Block or character device file detected.
    #[error("device file detected: {path}")]
    DeviceFile {
        /// Device file path.
        path: PathBuf,
    },

    /// Checksum verification failed (hash --verify mismatch).
    #[error("checksum verification failed on {path}: expected {expected}")]
    ChecksumVerifyFailed {
        /// File path with checksum mismatch.
        path: PathBuf,
        /// Caller-provided expected checksum.
        expected: String,
    },

    /// File exceeds the configured maximum size.
    #[error("file too large: {path} is {size} bytes (max: {max_size})")]
    FileTooLarge {
        /// Path to the oversized file.
        path: PathBuf,
        /// Actual file size in bytes.
        size: u64,
        /// Configured maximum size in bytes.
        max_size: u64,
    },

    /// Search or replace found zero matches.
    #[error("no matches found")]
    NoMatches,

    /// Downstream consumer closed the output pipe.
    #[error("broken pipe")]
    BrokenPipe,

    /// Unexpected internal error.
    #[error("internal error: {reason}")]
    InternalError {
        /// Description of the internal failure.
        reason: String,
    },

    /// Advisory file lock could not be acquired within the configured timeout.
    #[error("lock timeout on {path} after {timeout_ms}ms")]
    LockTimeout {
        /// File path that could not be locked.
        path: PathBuf,
        /// Configured lock timeout in milliseconds.
        timeout_ms: u64,
    },

    /// Post-write tree-sitter syntax check found syntax errors in the output.
    #[error("syntax error detected in {path} ({count} nodes with ERROR)")]
    SyntaxError {
        /// File path that failed syntax check.
        path: PathBuf,
        /// Number of nodes with ERROR.
        count: u32,
    },

    /// `EXDEV` (cross-device rename) occurred and `--strict-atomic` was set,
    /// so copy-fallback was disabled by the caller.
    #[error("cross-device rename on {path} and --strict-atomic forbids fallback")]
    ExdevFallbackDisabled {
        /// File path involved in the cross-device rename.
        path: PathBuf,
    },

    /// Copy-back write (in-place inode-preserving strategy) failed BLAKE3
    /// verification after `ftruncate + write_all + fsync_data`.
    #[error("copy-back BLAKE3 verification failed for {path}")]
    CopyBackBlake3Failed {
        /// File path that failed post-write checksum verification.
        path: PathBuf,
    },

    /// Orphaneed `.atomwrite.journal.<target>.json` from a crashed
    /// in-place write was found and could not be recovered automatically.
    #[error("orphaned atomwrite journal at {journal} could not be recovered: {reason}")]
    OrphanJournal {
        /// Journal sidecar path left behind by a crashed write.
        journal: PathBuf,
        /// Description of why the journal could not be applied.
        reason: String,
    },

    /// A `--old`/`--new` pair in multi-pair edit mode failed to match (G117).
    ///
    /// Carries per-pair diagnostics so agents can locate the failing pair
    /// without bisecting the batch. Reuses the `INVALID_INPUT` code and
    /// exit 65 of [`Self::InvalidInput`].
    #[error("edit pair {index} of {total} failed: {reason}")]
    EditPairFailed {
        /// 1-based index of the pair that failed to match.
        index: u64,
        /// Total number of pairs in the invocation.
        total: u64,
        /// Description of why the pair did not match.
        reason: String,
        /// Per-pair results accumulated up to and including the failed pair.
        pair_results: Box<Vec<PairResult>>,
        /// Closest near-miss for the failing pair (v0.1.29 P0-2).
        best_candidate: Option<Box<BestCandidate>>,
    },
}

