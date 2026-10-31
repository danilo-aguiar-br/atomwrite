impl AtomwriteError {
    /// Return the process exit code for this error variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomwrite::error::AtomwriteError;
    ///
    /// assert_eq!(AtomwriteError::NoMatches.exit_code(), 1);
    /// let err = AtomwriteError::InvalidInput { reason: "bad glob".into() };
    /// assert_eq!(err.exit_code(), 65);
    /// ```
    #[inline]
    pub const fn exit_code(&self) -> u8 {
        use crate::constants as k;
        let code = match self {
            Self::NotFound { .. } | Self::GlobNoMatches { .. } => k::EXIT_NOT_FOUND,
            Self::InvalidInput { .. } => k::EXIT_INVALID_INPUT,
            Self::MatchFailed { .. } | Self::MatchAmbiguous { .. } => k::EXIT_INVALID_INPUT,
            Self::Cancelled { exit, .. } => return *exit,
            Self::PermissionDenied { .. } => k::EXIT_PERMISSION_DENIED,
            Self::DiskFull { .. } => k::EXIT_DISK_FULL,
            Self::QuotaExceeded { .. } => k::EXIT_QUOTA_EXCEEDED,
            Self::CrossDevice { .. } => k::EXIT_CROSS_DEVICE,
            Self::Io { .. } => k::EXIT_IO_ERROR,
            Self::ConfigInvalid { .. } => k::EXIT_CONFIG_INVALID,
            Self::StateDrift { .. } => k::EXIT_STATE_DRIFT,
            Self::ChecksumVerifyFailed { .. } => k::EXIT_CHECKSUM_VERIFY_FAILED,
            Self::FileTooLarge { .. } => k::EXIT_INVALID_INPUT,
            Self::WorkspaceJail { .. } => k::EXIT_WORKSPACE_JAIL,
            Self::SymlinkBlocked { .. } => k::EXIT_SYMLINK_BLOCKED,
            Self::FileImmutable { .. } => k::EXIT_IMMUTABLE_FILE,
            Self::BinaryFile { .. } => k::EXIT_INVALID_INPUT,
            Self::FifoDetected { .. } => k::EXIT_FIFO_DETECTED,
            Self::DeviceFile { .. } => k::EXIT_DEVICE_FILE,
            Self::NoMatches => k::EXIT_NO_MATCHES,
            Self::BrokenPipe => k::EXIT_BROKEN_PIPE,
            Self::InternalError { .. } => k::EXIT_INTERNAL_ERROR,
            Self::LockTimeout { .. } => k::EXIT_LOCK_TIMEOUT,
            Self::SyntaxError { .. } => k::EXIT_SYNTAX_ERROR,
            Self::ExdevFallbackDisabled { .. } => k::EXIT_EXDEV_FALLBACK_DISABLED,
            Self::CopyBackBlake3Failed { .. } => k::EXIT_COPY_BACK_FAILED,
            Self::OrphanJournal { .. } => k::EXIT_ORPHAN_JOURNAL,
            Self::EditPairFailed { .. } => k::EXIT_INVALID_INPUT,
            // v0.1.36 — a settled domain result gets its own number instead of
            // recycling the invalid-input code (65). `MissingFlag`, by contrast,
            // IS a malformed invocation, so it keeps 65 with the InvalidInput family.
            Self::MergeConflict { .. } => k::EXIT_MERGE_CONFLICT,
            // v0.1.37 — `edit-loop --partial` writes the matching pairs, so the
            // verdict cannot share 1 with `NoMatches` (nothing found, nothing
            // written). 87 is its own number.
            Self::EditLoopPartial { .. } => k::EXIT_EDIT_LOOP_PARTIAL,
            Self::MissingFlag { .. } => k::EXIT_INVALID_INPUT,
            Self::DoctorChecksFailed { .. } => k::EXIT_FAILURE,
            // 80 is the transaction-rollback status the constants table already
            // reserved; until v0.1.36 no variant produced it.
            Self::TransactionRollbackFailed { .. } => k::EXIT_TRANSACTION_ROLLBACK_FAILED,
            Self::GrammarUnavailable { .. } => k::EXIT_GRAMMAR_UNAVAILABLE,
            Self::UnsupportedCombination { .. } => k::EXIT_ARGUMENT_PARSE_ERROR,
        };
        code as u8
    }

    /// Classify the error for retry decisions.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomwrite::error::{AtomwriteError, ErrorClass};
    ///
    /// let err = AtomwriteError::InvalidInput { reason: "bad glob".into() };
    /// assert_eq!(err.error_class(), ErrorClass::Permanent);
    /// ```
    #[inline]
    pub fn error_class(&self) -> ErrorClass {
        match self {
            Self::Io { source } => {
                // G-023: permanent preconditions (EISDIR, invalid input-like) are not retryable.
                if source.raw_os_error() == Some(crate::constants::ERRNO_EISDIR)
                    || source.kind() == std::io::ErrorKind::IsADirectory
                    || source.kind() == std::io::ErrorKind::NotADirectory
                    || source.kind() == std::io::ErrorKind::InvalidInput
                {
                    ErrorClass::PreconditionFailed
                } else {
                    ErrorClass::Transient
                }
            }
            Self::DiskFull { .. } | Self::QuotaExceeded { .. } => ErrorClass::Transient,
            Self::StateDrift { .. }
            | Self::CrossDevice { .. }
            | Self::LockTimeout { .. }
            | Self::CopyBackBlake3Failed { .. } => ErrorClass::Conflict,
            Self::ChecksumVerifyFailed { .. }
            | Self::FileTooLarge { .. }
            | Self::SyntaxError { .. }
            | Self::ExdevFallbackDisabled { .. }
            | Self::OrphanJournal { .. } => ErrorClass::PreconditionFailed,
            Self::BinaryFile { .. }
            | Self::FileImmutable { .. }
            | Self::SymlinkBlocked { .. }
            | Self::WorkspaceJail { .. }
            | Self::FifoDetected { .. }
            | Self::DeviceFile { .. }
            | Self::DoctorChecksFailed { .. } => ErrorClass::PreconditionFailed,
            Self::NoMatches | Self::BrokenPipe => ErrorClass::Permanent,
            // v0.1.36 — a merge conflict and a partial edit-loop are settled
            // domain verdicts: retrying unchanged input gives the same result.
            // `MissingFlag` is an invalid invocation, permanent like `InvalidInput`.
            // `TransactionRollbackFailed` is permanent on purpose: retrying a
            // partially applied reversal can destroy the backups still needed.
            Self::MergeConflict { .. }
            | Self::EditLoopPartial { .. }
            | Self::MissingFlag { .. }
            | Self::TransactionRollbackFailed { .. } => ErrorClass::Permanent,
            // v0.1.37 — a missing grammar is a host precondition: the same run
            // succeeds once the grammar is cached, so it is not permanent.
            Self::GrammarUnavailable { .. } => ErrorClass::PreconditionFailed,
            // The caller's own command line is malformed, so retrying the same
            // invocation can never succeed; a fixed line can.
            Self::UnsupportedCombination { .. } => ErrorClass::Permanent,
            _ => ErrorClass::Permanent,
        }
    }

    /// Return true if the error class indicates a retry may succeed.
    ///
    /// Retryable variants (transient): [`Self::DiskFull`], [`Self::QuotaExceeded`], [`Self::Io`].
    /// Retryable variants (conflict): [`Self::StateDrift`], [`Self::CrossDevice`].
    ///
    /// All other variants are non-retryable (precondition or permanent).
    #[inline]
    pub fn is_retryable(&self) -> bool {
        self.error_class().is_retryable()
    }

    /// Return true if retrying this error will never succeed.
    ///
    /// Permanent errors include: [`Self::NotFound`], [`Self::InvalidInput`],
    /// [`Self::PermissionDenied`], [`Self::ConfigInvalid`], [`Self::NoMatches`],
    /// [`Self::BrokenPipe`], and [`Self::InternalError`].
    #[inline]
    pub fn is_permanent(&self) -> bool {
        self.error_class().is_permanent()
    }

    /// Return the machine-readable error code string for NDJSON output.
    #[inline]
    pub const fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "FILE_NOT_FOUND",
            Self::GlobNoMatches { .. } => "GLOB_NO_MATCHES",
            Self::InvalidInput { .. } => "INVALID_INPUT",
            Self::MatchFailed { .. } => "MATCH_FAILED",
            Self::MatchAmbiguous { .. } => "MATCH_AMBIGUOUS",
            Self::Cancelled { .. } => "CANCELLED",
            Self::PermissionDenied { .. } => "PERMISSION_DENIED",
            Self::DiskFull { .. } => "DISK_FULL",
            Self::QuotaExceeded { .. } => "QUOTA_EXCEEDED",
            Self::CrossDevice { .. } => "CROSS_DEVICE",
            Self::Io { .. } => "IO_ERROR",
            Self::ConfigInvalid { .. } => "CONFIG_INVALID",
            Self::StateDrift { .. } => "STATE_DRIFT",
            Self::ChecksumVerifyFailed { .. } => "CHECKSUM_VERIFY_FAILED",
            Self::FileTooLarge { .. } => "FILE_TOO_LARGE",
            Self::WorkspaceJail { .. } => "WORKSPACE_JAIL",
            Self::SymlinkBlocked { .. } => "SYMLINK_BLOCKED",
            Self::FileImmutable { .. } => "IMMUTABLE_FILE",
            Self::BinaryFile { .. } => "BINARY_FILE",
            Self::FifoDetected { .. } => "FIFO_DETECTED",
            Self::DeviceFile { .. } => "DEVICE_FILE",
            Self::NoMatches => "NO_MATCHES",
            Self::BrokenPipe => "BROKEN_PIPE",
            Self::InternalError { .. } => "INTERNAL_ERROR",
            Self::LockTimeout { .. } => "LOCK_TIMEOUT",
            Self::SyntaxError { .. } => "SYNTAX_ERROR_DETECTED",
            Self::ExdevFallbackDisabled { .. } => "EXDEV_FALLBACK_DISABLED",
            Self::CopyBackBlake3Failed { .. } => "COPY_BACK_BLAKE3_FAILED",
            Self::OrphanJournal { .. } => "ORPHAN_JOURNAL",
            Self::EditPairFailed { .. } => "MATCH_FAILED",
            // v0.1.36 — own code per domain result; `MissingFlag` stays in the
            // `INVALID_INPUT` family because it is a malformed invocation.
            Self::MergeConflict { .. } => "MERGE_CONFLICT",
            Self::EditLoopPartial { .. } => "EDIT_LOOP_PARTIAL",
            Self::MissingFlag { .. } => "INVALID_INPUT",
            Self::TransactionRollbackFailed { .. } => "TRANSACTION_ROLLBACK_FAILED",
            Self::DoctorChecksFailed { .. } => "DOCTOR_CHECKS_FAILED",
            Self::GrammarUnavailable { .. } => "GRAMMAR_UNAVAILABLE",
            Self::UnsupportedCombination { .. } => "UNSUPPORTED_COMBINATION",
        }
    }

    /// Input refused for its content or syntax: `INVALID_INPUT`, exit 65.
    ///
    /// `what` names the rejected input (`invalid pattern '(...'`) and `cause` is the
    /// parser's message, cut to its last line that holds a word (a caret line
    /// points at the input, it does not say what is wrong) with a leading
    /// `error:` removed, so a multi-line diagnostic stays one short sentence.
    #[must_use]
    pub fn invalid(what: &str, cause: impl std::fmt::Display) -> Self {
        let text = cause.to_string();
        let short = text
            .lines()
            .rev()
            .map(str::trim)
            .find(|line| line.chars().any(char::is_alphabetic))
            .map_or(text.trim(), |line| line.trim_start_matches("error: "));
        Self::InvalidInput {
            reason: format!("{what}: {short}"),
        }
    }

    /// Classify an I/O error raised while reading or writing `path`.
    ///
    /// A denied access and a missing file keep their own variants and codes;
    /// anything else becomes [`Self::Io`] with the path in its message, so the
    /// failure names the file even when the OS error does not.
    #[must_use]
    pub fn from_io_at(path: &std::path::Path, source: std::io::Error) -> Self {
        match source.kind() {
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied {
                path: path.to_path_buf(),
            },
            std::io::ErrorKind::NotFound => Self::NotFound {
                path: path.to_path_buf(),
            },
            kind => Self::Io {
                source: std::io::Error::new(kind, format!("{}: {source}", path.display())),
            },
        }
    }

    /// Classify an error of the directory walker that names a path.
    ///
    /// Returns the failing path with its classified error, or `None` when the
    /// walker error has no path or no I/O cause (a malformed ignore file): that
    /// is not a failure to read the tree. A link loop is reported as
    /// [`Self::Io`] on the looping path.
    #[must_use]
    pub fn from_walk_error(err: &ignore::Error) -> Option<(PathBuf, Self)> {
        fn io_cause(err: &ignore::Error) -> Option<std::io::Error> {
            match err {
                ignore::Error::Io(source) => {
                    Some(std::io::Error::new(source.kind(), source.to_string()))
                }
                ignore::Error::WithPath { err, .. }
                | ignore::Error::WithDepth { err, .. }
                | ignore::Error::WithLineNumber { err, .. } => io_cause(err),
                _ => None,
            }
        }
        match err {
            ignore::Error::WithPath { path, err } => {
                io_cause(err).map(|source| (path.clone(), Self::from_io_at(path, source)))
            }
            ignore::Error::WithDepth { err, .. } => Self::from_walk_error(err),
            ignore::Error::Loop { child, .. } => Some((
                child.clone(),
                Self::from_io_at(child, std::io::Error::other(err.to_string())),
            )),
            _ => None,
        }
    }

    /// Return the filesystem path associated with this error, if any.
    #[inline]
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::NotFound { path }
            | Self::GlobNoMatches { pattern: path }
            | Self::PermissionDenied { path }
            | Self::DiskFull { path }
            | Self::QuotaExceeded { path }
            | Self::CrossDevice { path }
            | Self::StateDrift { path, .. }
            | Self::ChecksumVerifyFailed { path, .. }
            | Self::FileTooLarge { path, .. }
            | Self::WorkspaceJail { path, .. }
            | Self::SymlinkBlocked { path }
            | Self::FileImmutable { path }
            | Self::BinaryFile { path }
            | Self::FifoDetected { path }
            | Self::DeviceFile { path }
            | Self::LockTimeout { path, .. }
            | Self::SyntaxError { path, .. }
            | Self::ExdevFallbackDisabled { path, .. }
            | Self::CopyBackBlake3Failed { path }
            | Self::OrphanJournal { journal: path, .. }
            | Self::MergeConflict { path, .. } => Some(path),
            Self::InvalidInput { .. }
            | Self::EditPairFailed { .. }
            | Self::Io { .. }
            | Self::ConfigInvalid { .. }
            | Self::NoMatches
            | Self::BrokenPipe
            | Self::InternalError { .. }
            | Self::MatchFailed { .. }
            | Self::MatchAmbiguous { .. }
            | Self::Cancelled { .. }
            | Self::EditLoopPartial { .. }
            | Self::MissingFlag { .. }
            | Self::DoctorChecksFailed { .. }
            | Self::TransactionRollbackFailed { .. }
            | Self::GrammarUnavailable { .. }
            | Self::UnsupportedCombination { .. } => None,
        }
    }
}
