
/// Domain-specific errors for atomic file operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AtomwriteError {
    /// Target file does not exist.
    // The path is not repeated in the text: the envelope already carries it in
    // `path` and `target_resolved`, so the sentence that names it a third time
    // costs bytes on every failure without adding a fact.
    #[error("file not found")]
    NotFound {
        /// File path that was not found.
        path: PathBuf,
    },

    /// A pattern operand matched no name.
    ///
    /// Kept apart from [`Self::NotFound`] on purpose: `cat '*.rs'` that matches
    /// nothing is not a missing file, and saying "file not found: *.rs" sends
    /// the caller looking for the wrong thing. The exit code stays
    /// [`crate::constants::EXIT_NOT_FOUND`] because "nothing carries that name"
    /// is the same verdict a shell gives.
    #[error("no file matches the pattern: {pattern}")]
    GlobNoMatches {
        /// Pattern operand that matched no name under its directory.
        pattern: PathBuf,
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
        /// When true, path was a content-source (`--old-file`/`--new-file`).
        /// Selects staging-oriented suggestion (OI-SUGGEST-ROLE / v0.1.36).
        content_source: bool,
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

    /// Every visited file was skipped, so the pattern was never tested.
    #[error("every visited file was skipped ({files} files)")]
    AllSkippedBinary {
        /// How many files the binary gate skipped.
        files: u64,
    },

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
    ///
    /// `line` and `line_end` carry the 1-based span of the block that holds
    /// the first error. A refusal that named only the file left the caller to
    /// re-read the whole file to find the broken block, which is exactly the
    /// round trip this refusal exists to save.
    #[error("syntax error detected in {path} ({count} nodes with ERROR)")]
    SyntaxError {
        /// File path that failed syntax check.
        path: PathBuf,
        /// Number of nodes with ERROR.
        count: u32,
        /// 1-based line of the block that holds the first error.
        line: Option<usize>,
        /// 1-based last line of that block.
        line_end: Option<usize>,
    },

    /// The write cannot be a single atomic step and `--strict-atomic` was set.
    ///
    /// Two cases raise it. `EXDEV` (cross-device rename) is the original one:
    /// the temp file and the target sit on different filesystems. The other is a
    /// target that the auto-detected strategy can only write in place (a
    /// hardlink or a symlink target): keeping the inode means truncate + write,
    /// which is not atomic against a crash.
    #[error("--strict-atomic forbids the non-atomic write of {path}: {reason}")]
    ExdevFallbackDisabled {
        /// File path that could not be written atomically.
        path: PathBuf,
        /// Why the single atomic step is not available.
        reason: String,
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
    /// without bisecting the batch. A pair that does not match is a match
    /// failure, so it carries the `MATCH_FAILED` code of [`Self::MatchFailed`]
    /// and the exit 65 shared with [`Self::InvalidInput`].
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

    /// Three-way merge found overlapping edits it could not align by line
    /// index (v0.1.36).
    ///
    /// This is a **domain result**, not malformed input: `base`, `ours` and
    /// `theirs` are all valid files that disagree on the same lines. Exit 84
    /// / `MERGE_CONFLICT`, class permanent. The conflict count travels in
    /// `MergeConflict::count` so the caller reads it from the envelope
    /// instead of re-parsing the human message.
    #[error("merge conflict: {count} conflicting region(s) for {path}")]
    MergeConflict {
        /// Number of conflicting regions the line aligner could not resolve.
        count: u64,
        /// Output path the merge would have written.
        path: PathBuf,
    },

    /// `edit-loop --partial` wrote the pairs that matched while others did not
    /// (v0.1.36, exit changed in v0.1.37).
    ///
    /// Partial application is a **domain verdict**: the file IS written with
    /// the pairs that matched, so a zero exit would hide the miss from the
    /// caller. Exit 87 / `EDIT_LOOP_PARTIAL`, class permanent. Without
    /// `--partial` an unmatched pair aborts the batch with
    /// [`Self::EditPairFailed`] (exit 65) and nothing is written.
    #[error("edit-loop partial: {unmatched} of {total} pair(s) did not match")]
    EditLoopPartial {
        /// Number of pairs that did not match.
        unmatched: u64,
        /// Total number of pairs read from stdin.
        total: u64,
    },

    /// `doctor --strict` found a check that failed or warned (v0.1.37).
    ///
    /// A settled host verdict, not a bug: the report event already carries
    /// every check, so the error only carries the totals. Exit 1 (the generic
    /// failure status, the one the command always returned), class
    /// `precondition_failed` because the host must change before a re-run.
    #[error("doctor: {failed} check(s) failed and {warned} warned (strict mode)")]
    DoctorChecksFailed {
        /// Number of checks whose status is `fail`.
        failed: u64,
        /// Number of checks whose status is `warn`.
        warned: u64,
    },

    /// A companion flag the operation cannot proceed without was omitted
    /// (v0.1.36).
    ///
    /// Carries `flag` as its own field so the caller re-runs with the flag
    /// instead of scraping the human message. Exit 65 / `INVALID_INPUT`,
    /// matching the [`Self::InvalidInput`] family.
    #[error("missing required flag {flag}: {reason}")]
    MissingFlag {
        /// Flag literal including leading dashes (e.g. `--recursive`).
        flag: String,
        /// Description of why the flag is required.
        reason: String,
    },

    /// A `batch --transaction` rollback failed partway, leaving the tree in a
    /// state that is neither pre-transaction nor fully reverted (v0.1.36).
    ///
    /// Exit 80 / `TRANSACTION_ROLLBACK_FAILED`, class **permanent**. The
    /// condition is a disk-side failure, not malformed input, so it must not
    /// report as [`Self::InvalidInput`]. Permanent and not retryable on
    /// purpose: re-running the reversal against a half-applied tree can
    /// destroy the backups the caller still needs, so the recovery path is
    /// inspecting the WAL journal, never a blind retry.
    #[error("transaction rollback failed, tree may be partially reverted: {reason}")]
    TransactionRollbackFailed {
        /// Description of why the rollback could not finish.
        reason: String,
    },

    /// The tree-sitter grammar for a language is not on this host and the
    /// download that would fetch it did not happen (v0.1.37).
    ///
    /// Exit 69 / `GRAMMAR_UNAVAILABLE`, class `precondition_failed`: the
    /// operation needs a grammar, the grammar needs the network the first time,
    /// and neither is a defect of the file being read. Before this variant the
    /// condition arrived as [`Self::InternalError`] — exit 255 with "isto é um
    /// bug" in the suggestion — which told the agent the tool was broken when
    /// the host was merely offline.
    #[error("grammar {language} is not available: {reason}")]
    GrammarUnavailable {
        /// tree-sitter language name the verb asked for.
        language: String,
        /// Why the grammar could not be obtained.
        reason: String,
    },

    /// Two flags the caller combined cannot both hold at once (v0.1.37).
    ///
    /// Exit 2 / `ARGUMENT_PARSE_ERROR`, the code the shell and `clap` already
    /// give a malformed command line. [`Self::InvalidInput`] is 65, which the
    /// contract reserves for a target the run could not act on, so reusing it
    /// for "these flags do not go together" told an agent the file was wrong
    /// when the command was.
    #[error("{flag} cannot be combined with {other}: {reason}")]
    UnsupportedCombination {
        /// The flag that cannot be honoured in this combination.
        flag: String,
        /// The other flag in the combination.
        other: String,
        /// What the combination would have to mean, and cannot.
        reason: String,
    },
}

