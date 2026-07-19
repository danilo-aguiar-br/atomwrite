impl AtomwriteError {
    /// Return the process exit code for this error variant.
    #[inline]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::NotFound { .. } => 4,
            Self::InvalidInput { .. } => 65,
            Self::MatchFailed { .. } | Self::MatchAmbiguous { .. } => 65,
            Self::Cancelled { exit, .. } => *exit,
            Self::PermissionDenied { .. } => 13,
            Self::DiskFull { .. } => 28,
            Self::QuotaExceeded { .. } => 30,
            Self::CrossDevice { .. } => 73,
            Self::Io { .. } => 74,
            Self::ConfigInvalid { .. } => 78,
            Self::StateDrift { .. } => 82,
            Self::ChecksumVerifyFailed { .. } => 81,
            Self::FileTooLarge { .. } => 65,
            Self::WorkspaceJail { .. } => 126,
            Self::SymlinkBlocked { .. } => 127,
            Self::FileImmutable { .. } => 128,
            Self::BinaryFile { .. } => 65,
            Self::FifoDetected { .. } => 85,
            Self::DeviceFile { .. } => 86,
            Self::NoMatches => 1,
            Self::BrokenPipe => 141,
            Self::InternalError { .. } => 255,
            Self::LockTimeout { .. } => 83,
            Self::SyntaxError { .. } => 88,
            Self::ExdevFallbackDisabled { .. } => 91,
            Self::CopyBackBlake3Failed { .. } => 92,
            Self::OrphanJournal { .. } => 93,
            Self::EditPairFailed { .. } => 65,
        }
    }

    /// Classify the error for retry decisions.
    #[inline]
    pub fn error_class(&self) -> ErrorClass {
        match self {
            Self::Io { source } => {
                // G-023: permanent preconditions (EISDIR, invalid input-like) are not retryable.
                if source.raw_os_error() == Some(21) // EISDIR on Unix
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
            | Self::DeviceFile { .. } => ErrorClass::PreconditionFailed,
            Self::NoMatches | Self::BrokenPipe => ErrorClass::Permanent,
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
            Self::EditPairFailed { .. } => "INVALID_INPUT",
        }
    }

    /// Return the filesystem path associated with this error, if any.
    #[inline]
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::NotFound { path }
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
            | Self::ExdevFallbackDisabled { path }
            | Self::CopyBackBlake3Failed { path }
            | Self::OrphanJournal { journal: path, .. } => Some(path),
            Self::InvalidInput { .. }
            | Self::EditPairFailed { .. }
            | Self::Io { .. }
            | Self::ConfigInvalid { .. }
            | Self::NoMatches
            | Self::BrokenPipe
            | Self::InternalError { .. }
            | Self::MatchFailed { .. }
            | Self::MatchAmbiguous { .. }
            | Self::Cancelled { .. } => None,
        }
    }
}
