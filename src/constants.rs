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

/// v0.1.33: max sliding windows for `context_aware` / `rank_into`.
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

// --- WAL heuristics (G-007: named defaults for XDG config, not env) ---
/// Default seconds to keep committed WAL sidecars (0 = drop immediately).
pub const WAL_KEEP_SECS_DEFAULT: u64 = 0;
/// Max committed sidecars retained per workspace (LRU).
pub const WAL_MAX_COUNT_DEFAULT: u64 = 100;
/// Max WAL sidecar creations per 60s window (rate limit).
pub const WAL_RATE_LIMIT_DEFAULT: u64 = 10;
/// Archive committed journals older than this many days.
pub const WAL_ARCHIVE_DAYS_DEFAULT: u64 = 7;

// --- Fuzzy thresholds (G-FZZ-146/150) ---
/// Auto-mode block / dual-gate floor.
pub const FUZZY_THRESHOLD_AUTO: f64 = 0.70;
/// Aggressive-mode floor.
pub const FUZZY_THRESHOLD_AGGRESSIVE: f64 = 0.50;
/// Context-aware Damerau default.
pub const FUZZY_THRESHOLD_CONTEXT: f64 = 0.80;
/// Jaro-Winkler line match default.
pub const FUZZY_THRESHOLD_JW: f64 = 0.85;
/// Dual-gate near-exact JW bypass (G-FZZ-145 hardcode kill).
pub const FUZZY_DUAL_GATE_NEAR_EXACT: f64 = 0.99;
/// Minimum similarity to surface `best_candidate`.
pub const FUZZY_BEST_CANDIDATE_MIN: f64 = 0.50;
/// Truncate `best_candidate` text for NDJSON.
pub const FUZZY_BEST_CANDIDATE_TEXT_MAX: usize = 500;
/// Max `did_you_mean` candidates.
pub const FUZZY_MAX_CANDIDATES: usize = 3;
/// Adaptive thr: raise floor when pattern shorter than this many chars.
pub const FUZZY_ADAPTIVE_SHORT_PATTERN_CHARS: usize = 8;
/// Adaptive thr boost added for short patterns (clamped to 1.0).
pub const FUZZY_ADAPTIVE_SHORT_BOOST: f64 = 0.05;
/// Context-aware soft floor: accept near-miss within this delta below threshold.
pub const FUZZY_CONTEXT_SOFT_FLOOR_DELTA: f64 = 0.05;
/// I/O policy: ensure trailing newline on rewritten JSON (G-022).
pub const ENSURE_TRAILING_NEWLINE_JSON: bool = true;

// --- Write policy defaults (G-028: named; overridable via XDG `[write]`) ---
/// Default size above which `--confirm` requires `--ack-overwrite` (100 KiB).
pub const CONFIRM_LARGE_FILE_BYTES: u64 = 100 * 1024;
/// Default shrink block threshold percent (block when file shrinks by more than this).
pub const SHRINK_BLOCK_PERCENT: u8 = 50;
/// Default auto-rotate window for recently modified targets (24 hours).
pub const AUTO_ROTATE_MAX_AGE_SECS: u64 = 24 * 3600;

// --- One-shot / CLI policy defaults (G-039: named; clap `default_value_t`) ---
/// Global `--timeout-secs` default (one-shot agent deadline; `0` disables).
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
/// `--risk-threshold` default: 255 = off (must set e.g. 50 to enable L1 warning).
pub const DEFAULT_RISK_THRESHOLD_OFF: u8 = 255;
/// Suggested risk threshold when operators enable size-delta warnings.
pub const DEFAULT_RISK_THRESHOLD_ON: u8 = 50;

const _: () = assert!(CONFIRM_LARGE_FILE_BYTES >= 1024);
const _: () = assert!(SHRINK_BLOCK_PERCENT > 0 && SHRINK_BLOCK_PERCENT <= 99);
const _: () = assert!(AUTO_ROTATE_MAX_AGE_SECS >= 60);
const _: () = assert!(DEFAULT_TIMEOUT_SECS > 0);
const _: () = assert!(DEFAULT_RISK_THRESHOLD_ON > 0 && DEFAULT_RISK_THRESHOLD_ON < 100);

// --- Search / progress / backup / wal-heal policy defaults (G-043) ---
/// Default `search --max-filesize` (10 MiB).
pub const DEFAULT_SEARCH_MAX_FILESIZE_BYTES: u64 = 10 * 1024 * 1024;
/// Default `search --max-columns` truncation.
pub const DEFAULT_SEARCH_MAX_COLUMNS: usize = 500;
/// Default `replace --progress-every` (0 = off).
pub const DEFAULT_PROGRESS_EVERY_FILES: u64 = 50;
// DEFAULT_BACKUP_RETENTION defined above (line ~28).
/// Default `wal-heal --threshold-secs` (1 hour; distinct from `AUTO_ROTATE_MAX_AGE_SECS` 24h).
pub const DEFAULT_WAL_HEAL_THRESHOLD_SECS: u64 = 3600;
/// Default `wal-heal --max-duration-ms` wall-clock budget.
pub const DEFAULT_WAL_HEAL_MAX_DURATION_MS: u64 = 100;
/// Default `semantic-search --top` / sparse list limits.
pub const DEFAULT_SEMANTIC_SEARCH_TOP: usize = 20;
/// Default `semantic-search` min score.
pub const DEFAULT_SEMANTIC_SEARCH_MIN_SCORE: f64 = 0.05;
/// Default `sparse` max entries.
pub const DEFAULT_SPARSE_MAX_ENTRIES: usize = 100;
/// Default `sparse` max bytes per file.
pub const DEFAULT_SPARSE_MAX_BYTES: usize = 1_048_576;
/// Default `sparse` outline depth / related limits.
pub const DEFAULT_SPARSE_OUTLINE_DEPTH: usize = 50;
/// Default `sparse` max symbols.
pub const DEFAULT_SPARSE_MAX_SYMBOLS: usize = 20;
/// Default `watch --debounce-ms`.
pub const DEFAULT_WATCH_DEBOUNCE_MS: u64 = 200;
/// Floor for watch poll sleep when debounce is smaller (A-020).
pub const WATCH_DEBOUNCE_FLOOR_MS: u64 = 50;
/// Idle exit when no FS events arrive (B-007 one-shot; overridable later via XDG).
pub const DEFAULT_WATCH_IDLE_EXIT_MS: u64 = 3_000;

// --- Write content risk patterns (B-013; not product telemetry) ---
/// Destructive shell substrings that elevate write content risk (UTF-8, case-sensitive).
pub const WRITE_CONTENT_RISK_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "rm -fr /",
    "mkfs.",
    "dd if=",
    ":(){ :|:& };:",
    "curl ",
    "| sh",
    "|sh",
    "| bash",
    "|bash",
    "wget ",
    "chmod -R 777 /",
    "chown -R",
];
/// Default `query` / outline top-k style limits.
pub const DEFAULT_QUERY_TOP: usize = 10;
/// Default `diff` context lines.
pub const DEFAULT_DIFF_CONTEXT_LINES: usize = 3;
/// Default list page size / similar.
pub const DEFAULT_LIST_LIMIT: usize = 100;

// --- WAL residual policy (A-013) ---
/// L1: target size above which Auto policy may emit a WAL sidecar (1 MiB).
pub const WAL_L1_LARGE_FILE_BYTES: u64 = 1024 * 1024;
/// Trivial file size at or below which Auto policy skips sidecars in git repos.
pub const WAL_SMALL_RECORD_BYTES: u64 = 4096;
/// Recommend auto-heal when journal count exceeds this (aligned with max count default).
pub const WAL_AUTO_HEAL_COUNT_THRESHOLD: u64 = WAL_MAX_COUNT_DEFAULT;
/// Recommend auto-heal when oldest journal age exceeds this many seconds.
pub const WAL_AUTO_HEAL_AGE_SECS: u64 = WAL_ARCHIVE_DAYS_DEFAULT * 86_400;
/// Max directories listed in wal-stats `by_directory` truncation.
pub const WAL_STATS_TOP_DIRS: usize = 10;

// --- Batch progress policy (A-018) ---
/// Divisor for adaptive batch progress cadence (`total / DIVISOR`).
pub const BATCH_PROGRESS_DIVISOR: u64 = 20;
/// Minimum `progress_every` when batch has ops.
pub const BATCH_PROGRESS_MIN: u64 = 1;
/// Maximum `progress_every` clamp for batch.
pub const BATCH_PROGRESS_MAX: u64 = 50;

// --- Residual policy (A-021…A-030) ---
/// Max chars in fuzzy `diff_preview` mini unified diff (A-023).
pub const FUZZY_MINI_DIFF_MAX_CHARS: usize = 800;
/// Windows atomic `persist` retry backoff delays in ms (A-022).
pub const PERSIST_RETRY_DELAYS_MS: &[u64] = &[100, 200, 400];
/// `diff` similar/timeout budget for expensive similarity path (A-026).
pub const DIFF_SIMILARITY_TIMEOUT_MS: u64 = 500;
/// Lock acquire: fast poll interval (A-030).
pub const LOCK_POLL_FAST_MS: u64 = 10;
/// Lock acquire: slow poll interval after fast attempts exhausted (A-030).
pub const LOCK_POLL_SLOW_MS: u64 = 50;
/// Lock acquire: number of fast polls before switching to slow (A-030).
pub const LOCK_POLL_FAST_ATTEMPTS: u32 = 20;
/// Glob patterns that exclude atomwrite timestamped backups (A-027 DRY).
pub const BACKUP_EXCLUDE_GLOBS: &[&str] = &["*.bak.*", "**/*.bak.*"];
/// Bounded channel capacity walker → NDJSON (A-031: policy of record).
pub const EVENT_CHANNEL_CAP: usize = 1024;
/// Conservative RSS budget per concurrent file task (A-031).
pub const RAM_PER_TASK_BYTES: u64 = 16 * 1024 * 1024;

const _: () = assert!(DEFAULT_SEARCH_MAX_FILESIZE_BYTES >= 1024);
const _: () = assert!(DEFAULT_SEARCH_MAX_COLUMNS >= 16);
const _: () = assert!(DEFAULT_BACKUP_RETENTION >= 1);
const _: () = assert!(DEFAULT_WAL_HEAL_THRESHOLD_SECS >= 1);
const _: () = assert!(DEFAULT_WAL_HEAL_MAX_DURATION_MS >= 1);
const _: () = assert!(WAL_L1_LARGE_FILE_BYTES >= 4096);
const _: () = assert!(WAL_SMALL_RECORD_BYTES >= 64);
const _: () = assert!(WAL_AUTO_HEAL_COUNT_THRESHOLD >= 1);
const _: () = assert!(WAL_AUTO_HEAL_AGE_SECS >= 86_400);
const _: () = assert!(BATCH_PROGRESS_DIVISOR >= 1);
const _: () = assert!(BATCH_PROGRESS_MAX >= BATCH_PROGRESS_MIN);
const _: () = assert!(WATCH_DEBOUNCE_FLOOR_MS >= 1);
const _: () = assert!(FUZZY_MINI_DIFF_MAX_CHARS >= 64);
const _: () = assert!(!PERSIST_RETRY_DELAYS_MS.is_empty());
const _: () = assert!(DIFF_SIMILARITY_TIMEOUT_MS >= 1);
const _: () = assert!(LOCK_POLL_FAST_MS >= 1);
const _: () = assert!(LOCK_POLL_SLOW_MS >= LOCK_POLL_FAST_MS);
const _: () = assert!(LOCK_POLL_FAST_ATTEMPTS >= 1);
const _: () = assert!(!BACKUP_EXCLUDE_GLOBS.is_empty());
const _: () = assert!(EVENT_CHANNEL_CAP >= 16);
const _: () = assert!(RAM_PER_TASK_BYTES >= 1024 * 1024);
