// SPDX-License-Identifier: MIT OR Apache-2.0

//! Named constants for buffer sizes, thresholds, and identifiers.
//!
//! All values here are pure compile-time `const` data (inlined at each use
//! site). Process-wide state with a stable address belongs in `static`
//! (see `signal`, command modules using `LazyLock` / `OnceLock`).

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

// Compile-time invariants: fail the build if related thresholds drift.
const _: () = assert!(BUF_CAPACITY >= 4096);
const _: () = assert!(STDIN_INITIAL_CAPACITY >= 1024);
const _: () = assert!(MMAP_THRESHOLD as usize >= BUF_CAPACITY);
const _: () = assert!(DEFAULT_MAX_FILESIZE >= MMAP_THRESHOLD);
const _: () = assert!(BINARY_DETECT_SIZE == LINE_ENDING_DETECT_SIZE);
const _: () = assert!(BINARY_DETECT_SIZE >= 1024);
const _: () = assert!(MAX_NDJSON_LINE_SIZE >= 4096);
const _: () = assert!(MAX_JSON_DEPTH >= 16);
const _: () = assert!(DEFAULT_BACKUP_RETENTION >= 1);
const _: () = assert!(DIR_PERMISSIONS & 0o111 != 0); // directories need execute bit
const _: () = assert!(TEMPFILE_PERMISSIONS & 0o077 == 0); // owner-only tempfiles
const _: () = assert!(EXIT_SUCCESS == 0);
const _: () = assert!(EXIT_BROKEN_PIPE == 128 + 13); // SIGPIPE

/// v0.1.33 one-shot: max pattern bytes for fuzzy cascade (argv / block edits).
pub const FUZZY_MAX_PATTERN_BYTES: usize = 64 * 1024;

/// v0.1.33: skip O(m×n) levenshtein when either side exceeds this char count.
pub const FUZZY_MAX_LEVENSHTEIN_CHARS: usize = 8192;

/// v0.1.33: max sliding windows for context_aware / rank_into.
pub const FUZZY_MAX_WINDOWS: usize = 4096;

/// v0.1.33: default fuzzy applies per file when `--max-replacements` omitted.
pub const FUZZY_DEFAULT_MAX_REPLACEMENTS: u64 = 1;

/// v0.1.33: absolute ceiling even if user passes a huge `--max-replacements`.
pub const FUZZY_HARD_MAX_REPLACEMENTS: u64 = 10_000;

/// v0.1.33: max growth of edited buffer vs input (anti OOM from bad loops).
pub const FUZZY_MAX_BUFFER_GROWTH_FACTOR: usize = 4;

/// v0.1.33: additional absolute growth budget on top of factor (16 MiB).
pub const FUZZY_MAX_BUFFER_GROWTH_BYTES: usize = 16 * 1024 * 1024;

const _: () = assert!(FUZZY_MAX_PATTERN_BYTES >= 1024);
const _: () = assert!(FUZZY_MAX_LEVENSHTEIN_CHARS >= 64);
const _: () = assert!(FUZZY_MAX_WINDOWS >= 16);
const _: () = assert!(FUZZY_DEFAULT_MAX_REPLACEMENTS >= 1);
const _: () = assert!(FUZZY_HARD_MAX_REPLACEMENTS >= FUZZY_DEFAULT_MAX_REPLACEMENTS);
const _: () = assert!(FUZZY_MAX_BUFFER_GROWTH_FACTOR >= 2);
