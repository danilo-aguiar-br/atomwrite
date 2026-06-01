// SPDX-License-Identifier: MIT OR Apache-2.0

//! Named constants for buffer sizes, thresholds, and identifiers.

/// Buffer capacity for `BufWriter` and `BufReader` (64 KiB).
pub const BUF_CAPACITY: usize = 64 * 1024;

/// Initial allocation hint for stdin content accumulation.
pub const STDIN_INITIAL_CAPACITY: usize = 4096;

/// File size threshold above which memmap2 is used instead of heap read (1 MiB).
pub const MMAP_THRESHOLD: u64 = 1_048_576;

/// Default maximum allowed file size (1 GiB).
pub const DEFAULT_MAX_FILESIZE: u64 = 1_073_741_824;

/// Prefix for atomic write tempfiles.
pub const TEMPFILE_PREFIX: &str = ".atomwrite-";

/// Suffix for atomic write tempfiles.
pub const TEMPFILE_SUFFIX: &str = ".tmp";

/// Default number of backup copies to retain.
pub const DEFAULT_BACKUP_RETENTION: u8 = 5;

/// Directory permissions for newly created parent directories (Unix).
pub const DIR_PERMISSIONS: u32 = 0o755;

/// Restrictive permissions for tempfiles (Unix).
pub const TEMPFILE_PERMISSIONS: u32 = 0o600;

/// Detection window for binary content analysis (first 8 KiB).
pub const BINARY_DETECT_SIZE: usize = 8192;

/// Detection window for line ending analysis (first 8 KiB).
pub const LINE_ENDING_DETECT_SIZE: usize = 8192;

/// Exit code for broken pipe (128 + SIGPIPE).
pub const EXIT_BROKEN_PIPE: i32 = 141;

/// Exit code for successful operation.
pub const EXIT_SUCCESS: i32 = 0;

/// Exit code when batch transaction rollback fails.
pub const EXIT_TRANSACTION_ROLLBACK_FAILED: i32 = 80;

/// Exit code when checksum verification fails after write.
pub const EXIT_CHECKSUM_VERIFY_FAILED: i32 = 81;

/// Maximum allowed size for a single NDJSON line from stdin (256 KiB).
pub const MAX_NDJSON_LINE_SIZE: usize = 256 * 1024;

/// Maximum JSON nesting depth for dynamic Value parsing.
pub const MAX_JSON_DEPTH: usize = 128;
