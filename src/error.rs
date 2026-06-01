// SPDX-License-Identifier: MIT OR Apache-2.0

//! Domain-specific error types with exit codes and error classification.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Serialize;

/// Classification of error recoverability for retry decisions.
///
/// Used by callers to determine whether an operation can be retried.
/// The NDJSON output serializes this as the `error_class` string field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Transient failure that may resolve on retry (e.g., disk full, I/O).
    Transient,
    /// Conflict requiring state reload before retry (e.g., checksum mismatch).
    Conflict,
    /// Precondition not met; retry without fixing precondition will fail.
    PreconditionFailed,
    /// Permanent failure; retry will not help.
    Permanent,
}

impl ErrorClass {
    /// Return the string representation for NDJSON serialization.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::Conflict => "conflict",
            Self::PreconditionFailed => "precondition_failed",
            Self::Permanent => "permanent",
        }
    }

    /// Return true if this class indicates a retry may succeed.
    ///
    /// Both [`Transient`](Self::Transient) and [`Conflict`](Self::Conflict)
    /// are considered retryable.
    #[inline]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Transient | Self::Conflict)
    }

    /// Return true if this class indicates a permanent failure.
    ///
    /// Only [`Permanent`](Self::Permanent) errors are truly permanent.
    /// [`PreconditionFailed`](Self::PreconditionFailed) errors may succeed
    /// if the caller fixes the precondition first.
    #[inline]
    pub const fn is_permanent(&self) -> bool {
        matches!(self, Self::Permanent)
    }
}

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
}

impl AtomwriteError {
    /// Return the process exit code for this error variant.
    #[inline]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::NotFound { .. } => 4,
            Self::InvalidInput { .. } => 65,
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
        }
    }

    /// Classify the error for retry decisions.
    #[inline]
    pub const fn error_class(&self) -> ErrorClass {
        match self {
            Self::Io { .. } | Self::DiskFull { .. } | Self::QuotaExceeded { .. } => {
                ErrorClass::Transient
            }
            Self::StateDrift { .. } | Self::CrossDevice { .. } => ErrorClass::Conflict,
            Self::ChecksumVerifyFailed { .. } | Self::FileTooLarge { .. } => {
                ErrorClass::PreconditionFailed
            }
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
            | Self::DeviceFile { path } => Some(path),
            Self::InvalidInput { .. }
            | Self::Io { .. }
            | Self::ConfigInvalid { .. }
            | Self::NoMatches
            | Self::BrokenPipe
            | Self::InternalError { .. } => None,
        }
    }
}

/// Serializable error envelope emitted as a single NDJSON line.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ErrorJson {
    /// Always true, marks this line as an error event.
    pub error: bool,
    /// Machine-readable error code string.
    pub code: &'static str,
    /// Suggested process exit code.
    pub exit: u8,
    /// Human-readable error message.
    pub message: String,
    /// Filesystem path related to the error, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Error class: transient, conflict, `precondition_failed`, or permanent.
    pub error_class: &'static str,
    /// Whether a retry may resolve this error.
    pub retryable: bool,
    /// Optional actionable suggestion for the caller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Workspace root used for jail validation, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

impl ErrorJson {
    /// Build an [`ErrorJson`] from a domain error.
    #[cold]
    #[track_caller]
    pub fn from_error(err: &AtomwriteError) -> Self {
        let workspace = match err {
            AtomwriteError::WorkspaceJail { workspace, .. } => {
                Some(workspace.display().to_string())
            }
            _ => None,
        };
        Self {
            error: true,
            code: err.error_code(),
            exit: err.exit_code(),
            message: err.to_string(),
            path: err.path().map(|p| p.display().to_string()),
            error_class: err.error_class().as_str(),
            retryable: err.is_retryable(),
            suggestion: suggestion_for(err),
            workspace,
        }
    }
}

#[cold]
fn suggestion_for(err: &AtomwriteError) -> Option<String> {
    match err {
        AtomwriteError::NotFound { .. } => Some("verify the file path exists".into()),
        AtomwriteError::PermissionDenied { .. } => Some("check file permissions".into()),
        AtomwriteError::DiskFull { .. } => Some("free disk space and retry".into()),
        AtomwriteError::QuotaExceeded { .. } => Some("check disk quota and free space".into()),
        AtomwriteError::CrossDevice { .. } => {
            Some("ensure source and destination are on the same filesystem".into())
        }
        AtomwriteError::StateDrift { .. } => {
            Some("re-read the file to get current checksum, then retry".into())
        }
        AtomwriteError::ChecksumVerifyFailed { .. } => {
            Some("re-read the file to get current checksum".into())
        }
        AtomwriteError::FileTooLarge { .. } => {
            Some("use --max-filesize to increase the limit or process smaller files".into())
        }
        AtomwriteError::WorkspaceJail { .. } => {
            Some("use an absolute path for the target file".into())
        }
        AtomwriteError::SymlinkBlocked { .. } => {
            Some("use --follow-symlinks to allow symbolic links".into())
        }
        AtomwriteError::BinaryFile { .. } => Some("use read --stat for metadata only".into()),
        AtomwriteError::FifoDetected { .. } => {
            Some("skip this file or use stdin redirection instead".into())
        }
        AtomwriteError::DeviceFile { .. } => {
            Some("skip this file or use stdin redirection instead".into())
        }
        AtomwriteError::BrokenPipe => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_class_transient() {
        let err = AtomwriteError::DiskFull {
            path: PathBuf::from("/tmp"),
        };
        assert_eq!(err.error_class(), ErrorClass::Transient);
        assert!(err.is_retryable());
        assert!(!err.is_permanent());
    }

    #[test]
    fn error_class_conflict() {
        let err = AtomwriteError::StateDrift {
            path: PathBuf::from("/tmp"),
            expected: "aaa".into(),
            actual: "bbb".into(),
        };
        assert_eq!(err.error_class(), ErrorClass::Conflict);
        assert!(err.is_retryable());
        assert!(!err.is_permanent());
    }

    #[test]
    fn error_class_precondition() {
        let err = AtomwriteError::BinaryFile {
            path: PathBuf::from("/tmp"),
        };
        assert_eq!(err.error_class(), ErrorClass::PreconditionFailed);
        assert!(!err.is_retryable());
        assert!(!err.is_permanent());
    }

    #[test]
    fn error_class_permanent() {
        let err = AtomwriteError::NoMatches;
        assert_eq!(err.error_class(), ErrorClass::Permanent);
        assert!(!err.is_retryable());
        assert!(err.is_permanent());
    }

    #[test]
    fn exit_code_not_found() {
        let err = AtomwriteError::NotFound {
            path: PathBuf::from("/x"),
        };
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn error_code_strings() {
        assert_eq!(
            AtomwriteError::NotFound {
                path: PathBuf::from("/x")
            }
            .error_code(),
            "FILE_NOT_FOUND"
        );
        assert_eq!(
            AtomwriteError::FifoDetected {
                path: PathBuf::from("/x")
            }
            .error_code(),
            "FIFO_DETECTED"
        );
        assert_eq!(
            AtomwriteError::DeviceFile {
                path: PathBuf::from("/x")
            }
            .error_code(),
            "DEVICE_FILE"
        );
    }

    #[test]
    fn fifo_and_device_exit_codes() {
        assert_eq!(
            AtomwriteError::FifoDetected {
                path: PathBuf::from("/x")
            }
            .exit_code(),
            85
        );
        assert_eq!(
            AtomwriteError::DeviceFile {
                path: PathBuf::from("/x")
            }
            .exit_code(),
            86
        );
    }

    #[test]
    fn error_enum_size_audit() {
        let size = std::mem::size_of::<AtomwriteError>();
        assert!(size <= 80, "AtomwriteError grew beyond 80 bytes: {size}");
    }

    #[test]
    fn all_variants_properties() {
        let p = PathBuf::from("/test");
        let variants: Vec<(AtomwriteError, u8, ErrorClass, &str, bool)> = vec![
            (
                AtomwriteError::NotFound { path: p.clone() },
                4,
                ErrorClass::Permanent,
                "FILE_NOT_FOUND",
                true,
            ),
            (
                AtomwriteError::InvalidInput { reason: "x".into() },
                65,
                ErrorClass::Permanent,
                "INVALID_INPUT",
                false,
            ),
            (
                AtomwriteError::PermissionDenied { path: p.clone() },
                13,
                ErrorClass::Permanent,
                "PERMISSION_DENIED",
                true,
            ),
            (
                AtomwriteError::DiskFull { path: p.clone() },
                28,
                ErrorClass::Transient,
                "DISK_FULL",
                true,
            ),
            (
                AtomwriteError::QuotaExceeded { path: p.clone() },
                30,
                ErrorClass::Transient,
                "QUOTA_EXCEEDED",
                true,
            ),
            (
                AtomwriteError::CrossDevice { path: p.clone() },
                73,
                ErrorClass::Conflict,
                "CROSS_DEVICE",
                true,
            ),
            (
                AtomwriteError::Io {
                    source: std::io::Error::other("x"),
                },
                74,
                ErrorClass::Transient,
                "IO_ERROR",
                false,
            ),
            (
                AtomwriteError::ConfigInvalid { reason: "x".into() },
                78,
                ErrorClass::Permanent,
                "CONFIG_INVALID",
                false,
            ),
            (
                AtomwriteError::StateDrift {
                    path: p.clone(),
                    expected: "a".into(),
                    actual: "b".into(),
                },
                82,
                ErrorClass::Conflict,
                "STATE_DRIFT",
                true,
            ),
            (
                AtomwriteError::WorkspaceJail {
                    path: p.clone(),
                    workspace: p.clone(),
                },
                126,
                ErrorClass::PreconditionFailed,
                "WORKSPACE_JAIL",
                true,
            ),
            (
                AtomwriteError::SymlinkBlocked { path: p.clone() },
                127,
                ErrorClass::PreconditionFailed,
                "SYMLINK_BLOCKED",
                true,
            ),
            (
                AtomwriteError::FileImmutable { path: p.clone() },
                128,
                ErrorClass::PreconditionFailed,
                "IMMUTABLE_FILE",
                true,
            ),
            (
                AtomwriteError::BinaryFile { path: p.clone() },
                65,
                ErrorClass::PreconditionFailed,
                "BINARY_FILE",
                true,
            ),
            (
                AtomwriteError::FifoDetected { path: p.clone() },
                85,
                ErrorClass::PreconditionFailed,
                "FIFO_DETECTED",
                true,
            ),
            (
                AtomwriteError::DeviceFile { path: p.clone() },
                86,
                ErrorClass::PreconditionFailed,
                "DEVICE_FILE",
                true,
            ),
            (
                AtomwriteError::ChecksumVerifyFailed {
                    path: p.clone(),
                    expected: "a".into(),
                },
                81,
                ErrorClass::PreconditionFailed,
                "CHECKSUM_VERIFY_FAILED",
                true,
            ),
            (
                AtomwriteError::FileTooLarge {
                    path: p.clone(),
                    size: 100,
                    max_size: 50,
                },
                65,
                ErrorClass::PreconditionFailed,
                "FILE_TOO_LARGE",
                true,
            ),
            (
                AtomwriteError::NoMatches,
                1,
                ErrorClass::Permanent,
                "NO_MATCHES",
                false,
            ),
            (
                AtomwriteError::BrokenPipe,
                141,
                ErrorClass::Permanent,
                "BROKEN_PIPE",
                false,
            ),
            (
                AtomwriteError::InternalError { reason: "x".into() },
                255,
                ErrorClass::Permanent,
                "INTERNAL_ERROR",
                false,
            ),
        ];
        assert_eq!(variants.len(), 20, "test must cover all 20 variants");
        for (err, exit, class, code, has_path) in &variants {
            assert_eq!(err.exit_code(), *exit, "exit_code mismatch for {code}");
            assert_eq!(err.error_class(), *class, "error_class mismatch for {code}");
            assert_eq!(err.error_code(), *code, "error_code mismatch for {code}");
            assert_eq!(
                err.is_retryable(),
                class.is_retryable(),
                "retryable mismatch for {code}"
            );
            assert_eq!(err.path().is_some(), *has_path, "path mismatch for {code}");
            let json = ErrorJson::from_error(err);
            assert!(json.error);
            assert_eq!(json.exit, *exit);
            assert_eq!(json.code, *code);
            assert_eq!(json.error_class, class.as_str());
            let _ = serde_json::to_string(&json).expect("ErrorJson must serialize");
        }
    }

    #[test]
    fn error_class_as_str_roundtrip() {
        assert_eq!(ErrorClass::Transient.as_str(), "transient");
        assert_eq!(ErrorClass::Conflict.as_str(), "conflict");
        assert_eq!(
            ErrorClass::PreconditionFailed.as_str(),
            "precondition_failed"
        );
        assert_eq!(ErrorClass::Permanent.as_str(), "permanent");
    }

    #[test]
    fn error_class_is_permanent() {
        assert!(ErrorClass::Permanent.is_permanent());
        assert!(!ErrorClass::Transient.is_permanent());
        assert!(!ErrorClass::Conflict.is_permanent());
        assert!(!ErrorClass::PreconditionFailed.is_permanent());
    }

    #[test]
    fn error_json_from_error() {
        let err = AtomwriteError::NotFound {
            path: PathBuf::from("/missing"),
        };
        let json = ErrorJson::from_error(&err);
        assert!(json.error);
        assert_eq!(json.code, "FILE_NOT_FOUND");
        assert_eq!(json.exit, 4);
        assert!(!json.retryable);
    }
}
