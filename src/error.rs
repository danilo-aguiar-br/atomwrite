// SPDX-License-Identifier: MIT OR Apache-2.0

//! Domain-specific error types with exit codes and error classification.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Serialize;

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
    #[error("path outside workspace jail: {path}")]
    WorkspaceJail {
        /// Path that escaped the workspace jail.
        path: PathBuf,
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

    /// Search or replace found zero matches.
    #[error("no matches found")]
    NoMatches,

    /// Unexpected internal error.
    #[error("internal error: {reason}")]
    InternalError {
        /// Description of the internal failure.
        reason: String,
    },
}

impl AtomwriteError {
    /// Return the process exit code for this error variant.
    pub fn exit_code(&self) -> u8 {
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
            Self::WorkspaceJail { .. } => 126,
            Self::SymlinkBlocked { .. } => 127,
            Self::FileImmutable { .. } => 128,
            Self::BinaryFile { .. } => 65,
            Self::FifoDetected { .. } => 85,
            Self::DeviceFile { .. } => 86,
            Self::NoMatches => 1,
            Self::InternalError { .. } => 255,
        }
    }

    /// Classify the error as transient, conflict, `precondition_failed`, or permanent.
    pub fn error_class(&self) -> &str {
        match self {
            Self::Io { .. } | Self::DiskFull { .. } | Self::QuotaExceeded { .. } => "transient",
            Self::StateDrift { .. } | Self::CrossDevice { .. } => "conflict",
            Self::BinaryFile { .. }
            | Self::FileImmutable { .. }
            | Self::SymlinkBlocked { .. }
            | Self::WorkspaceJail { .. }
            | Self::FifoDetected { .. }
            | Self::DeviceFile { .. } => "precondition_failed",
            Self::NoMatches => "permanent",
            _ => "permanent",
        }
    }

    /// Return true if the error class indicates a retry may succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(self.error_class(), "transient" | "conflict")
    }

    /// Return the machine-readable error code string for NDJSON output.
    pub fn error_code(&self) -> &str {
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
            Self::WorkspaceJail { .. } => "WORKSPACE_JAIL",
            Self::SymlinkBlocked { .. } => "SYMLINK_BLOCKED",
            Self::FileImmutable { .. } => "IMMUTABLE_FILE",
            Self::BinaryFile { .. } => "BINARY_FILE",
            Self::FifoDetected { .. } => "FIFO_DETECTED",
            Self::DeviceFile { .. } => "DEVICE_FILE",
            Self::NoMatches => "NO_MATCHES",
            Self::InternalError { .. } => "INTERNAL_ERROR",
        }
    }

    /// Return the filesystem path associated with this error, if any.
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::NotFound { path }
            | Self::PermissionDenied { path }
            | Self::DiskFull { path }
            | Self::QuotaExceeded { path }
            | Self::CrossDevice { path }
            | Self::StateDrift { path, .. }
            | Self::WorkspaceJail { path }
            | Self::SymlinkBlocked { path }
            | Self::FileImmutable { path }
            | Self::BinaryFile { path }
            | Self::FifoDetected { path }
            | Self::DeviceFile { path } => Some(path),
            Self::InvalidInput { .. }
            | Self::Io { .. }
            | Self::ConfigInvalid { .. }
            | Self::NoMatches
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
    pub code: String,
    /// Suggested process exit code.
    pub exit: u8,
    /// Human-readable error message.
    pub message: String,
    /// Filesystem path related to the error, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Error class: transient, conflict, `precondition_failed`, or permanent.
    pub error_class: String,
    /// Whether a retry may resolve this error.
    pub retryable: bool,
    /// Optional actionable suggestion for the caller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl ErrorJson {
    /// Build an [`ErrorJson`] from a domain error.
    pub fn from_error(err: &AtomwriteError) -> Self {
        Self {
            error: true,
            code: err.error_code().to_owned(),
            exit: err.exit_code(),
            message: err.to_string(),
            path: err.path().map(|p| p.display().to_string()),
            error_class: err.error_class().to_owned(),
            retryable: err.is_retryable(),
            suggestion: suggestion_for(err),
        }
    }
}

fn suggestion_for(err: &AtomwriteError) -> Option<String> {
    match err {
        AtomwriteError::NotFound { .. } => Some("verify the file path exists".into()),
        AtomwriteError::PermissionDenied { .. } => Some("check file permissions".into()),
        AtomwriteError::DiskFull { .. } => Some("free disk space and retry".into()),
        AtomwriteError::StateDrift { .. } => {
            Some("re-read the file to get current checksum, then retry".into())
        }
        AtomwriteError::WorkspaceJail { .. } => {
            Some("use --workspace to set the correct workspace root".into())
        }
        AtomwriteError::BinaryFile { .. } => {
            Some("use read --stat for metadata or --force-text to bypass".into())
        }
        AtomwriteError::FifoDetected { .. } => {
            Some("skip this file or use stdin redirection instead".into())
        }
        AtomwriteError::DeviceFile { .. } => {
            Some("skip this file or use stdin redirection instead".into())
        }
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
        assert_eq!(err.error_class(), "transient");
        assert!(err.is_retryable());
    }

    #[test]
    fn error_class_conflict() {
        let err = AtomwriteError::StateDrift {
            path: PathBuf::from("/tmp"),
            expected: "aaa".into(),
            actual: "bbb".into(),
        };
        assert_eq!(err.error_class(), "conflict");
        assert!(err.is_retryable());
    }

    #[test]
    fn error_class_precondition() {
        let err = AtomwriteError::BinaryFile {
            path: PathBuf::from("/tmp"),
        };
        assert_eq!(err.error_class(), "precondition_failed");
        assert!(!err.is_retryable());
    }

    #[test]
    fn error_class_permanent() {
        let err = AtomwriteError::NoMatches;
        assert_eq!(err.error_class(), "permanent");
        assert!(!err.is_retryable());
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
