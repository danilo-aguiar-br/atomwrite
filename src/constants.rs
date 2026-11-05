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

/// Relative directory under workspace for agent patch staging (GAP-2026-019).
///
/// Content-source files for `--old-file`/`--new-file` MUST live under the
/// workspace jail. Agents stage patches at `{workspace}/{SCRATCH_DIR_REL}/…`
/// instead of OS temp (`/tmp`), which is outside the jail by default.
pub const SCRATCH_DIR_REL: &str = ".atomwrite/scratch";

/// Canonical clap `Commands` / `agent-surface` tool count (v0.1.36+ SSOT).
///
/// Kept in sync by unit test against `Cli::command().get_subcommands()`.
/// When adding a subcommand: bump this constant and agent-facing docs (45).
pub const CANONICAL_SUBCOMMAND_COUNT: usize = 47;

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

/// Exit code for `edit-loop --partial` wrote the matching pairs but others missed.
pub const EXIT_EDIT_LOOP_PARTIAL: i32 = 87;

/// Exit code when every visited file was skipped by the binary gate.
///
/// Distinct from `NO_MATCHES` (1) on purpose: "nothing was searched" and "the
/// text is not there" are opposite conclusions, and the old shared status let
/// a binary-only tree read as proof of absence.
pub const EXIT_ALL_SKIPPED_BINARY: i32 = 89;

/// Exit code for a failure no other status describes (also the `NO_MATCHES` status).
pub const EXIT_FAILURE: i32 = 1;

/// Exit code for `NO_MATCHES`: the search ran and found nothing.
pub const EXIT_NO_MATCHES: i32 = EXIT_FAILURE;

/// Exit code for a command line the parser rejected.
pub const EXIT_ARGUMENT_PARSE_ERROR: i32 = 2;

/// Exit code when the target does not exist.
pub const EXIT_NOT_FOUND: i32 = 4;

/// Exit code when the OS denied access.
pub const EXIT_PERMISSION_DENIED: i32 = 13;

/// Exit code when the device is full.
pub const EXIT_DISK_FULL: i32 = 28;

/// Exit code when a quota was exceeded.
pub const EXIT_QUOTA_EXCEEDED: i32 = 30;

/// Exit code for invalid input (also match failures, binary and oversize files).
pub const EXIT_INVALID_INPUT: i32 = 65;

/// Exit code for a rename across filesystems.
pub const EXIT_CROSS_DEVICE: i32 = 73;

/// Exit code for an I/O error.
pub const EXIT_IO_ERROR: i32 = 74;

/// Exit code for an invalid configuration.
pub const EXIT_CONFIG_INVALID: i32 = 78;

/// Exit code for optimistic-lock state drift.
pub const EXIT_STATE_DRIFT: i32 = 82;

/// Exit code when an advisory lock timed out.
pub const EXIT_LOCK_TIMEOUT: i32 = 83;

/// Exit code for `semantic-merge` conflicts.
pub const EXIT_MERGE_CONFLICT: i32 = 84;

/// Exit code when a FIFO was found where a regular file was expected.
pub const EXIT_FIFO_DETECTED: i32 = 85;

/// Exit code when a device file was found where a regular file was expected.
pub const EXIT_DEVICE_FILE: i32 = 86;

/// Exit code when the post-write syntax check failed.
pub const EXIT_SYNTAX_ERROR: i32 = 88;

/// Exit code when the cross-device copy fallback is disabled.
pub const EXIT_EXDEV_FALLBACK_DISABLED: i32 = 91;

/// Exit code when the copy-back BLAKE3 verification failed.
pub const EXIT_COPY_BACK_FAILED: i32 = 92;

/// Exit code when an orphan journal was found.
pub const EXIT_ORPHAN_JOURNAL: i32 = 93;

/// Exit code for a tree-sitter grammar this host cannot provide (v0.1.37).
///
/// `EX_UNAVAILABLE` from `sysexits.h`, which names exactly this condition: a
/// service the run needs is not available here. 69 was unused before, and the
/// alternative was 255, which reads as "the tool is broken".
pub const EXIT_GRAMMAR_UNAVAILABLE: i32 = 69;

/// Exit code for the global deadline (GNU `timeout(1)` convention).
pub const EXIT_TIMEOUT: i32 = 124;

/// Exit code when a path escapes the workspace jail.
pub const EXIT_WORKSPACE_JAIL: i32 = 126;

/// Exit code when a symlink target is blocked.
///
/// It used to be 127, which the shell reserves for "program not found": a
/// caller that reads only the status concluded the binary was missing and went
/// looking for an installation that was already there. 79 is free in this table.
pub const EXIT_SYMLINK_BLOCKED: i32 = 79;

/// Exit code when the target file is immutable.
pub const EXIT_IMMUTABLE_FILE: i32 = 128;

/// Exit code for SIGHUP (128 + 1).
pub const EXIT_SIGHUP: i32 = 129;

/// Exit code for SIGINT (128 + 2).
pub const EXIT_SIGINT: i32 = 130;

/// Exit code for SIGTERM (128 + 15).
pub const EXIT_SIGTERM: i32 = 143;

/// Exit code for an internal error.
pub const EXIT_INTERNAL_ERROR: i32 = 255;

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

// --- Fuzzy cancel poll and content cap (T1, v0.1.37) ---
/// Max content bytes the fuzzy cascade scans (256 MiB); `[fuzzy].max_content_bytes` overrides.
pub const FUZZY_MAX_CONTENT_BYTES: u64 = 256 * 1024 * 1024;
/// Cancel poll mask for cheap per-line loops: check shutdown every 256 iterations.
pub const FUZZY_CANCEL_POLL_MASK: usize = 0xFF;
/// Cancel poll mask for expensive per-iteration work (diff, Damerau): check every iteration.
pub const FUZZY_CANCEL_POLL_MASK_HEAVY: usize = 0;
/// Punctuation whose adjacent spaces `punctuation_normalized` removes.
pub const FUZZY_PUNCTUATION_CHARS: &str = "(){}[]<>,;:";

/// Max content bytes for `escape_normalized`, which builds an 8-byte-per-byte offset map
/// (32 MiB of content = 256 MiB of map); above it the strategy is skipped, not failed.
pub const FUZZY_ESCAPE_MAP_MAX_CONTENT_BYTES: usize = 32 * 1024 * 1024;

const _: () = assert!((FUZZY_CANCEL_POLL_MASK + 1).is_power_of_two());
const _: () = assert!(FUZZY_ESCAPE_MAP_MAX_CONTENT_BYTES as u64 <= FUZZY_MAX_CONTENT_BYTES);
const _: () = assert!(FUZZY_MAX_CONTENT_BYTES >= FUZZY_MAX_PATTERN_BYTES as u64);

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
/// Default size above which large overwrite requires `--ack-overwrite` (100 KiB; A-WRITE-001).
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

// --- Read cap and compact output budgets (v0.1.38, token economy) ---
/// Default byte ceiling of `read` before it stops at a line boundary.
///
/// Kept below the Bash tool's 30k-character cut, so a capped read never
/// triggers the "output written to file" path and the extra round trip.
pub const READ_DEFAULT_MAX_BYTES: usize = 24_000;
/// Byte ceiling of `search --format text` before it stops with a notice.
pub const SEARCH_TEXT_MAX_BYTES: usize = 20_000;
/// Byte ceiling of a compact `outline` answer before items are omitted.
pub const OUTLINE_MAX_BYTES: usize = 6_000;
/// Byte ceiling of `list --format text` before it stops with a notice.
pub const LIST_TEXT_MAX_BYTES: usize = 20_000;

// --- Image delivery limits (v0.1.38, official resize rule) ---
/// Side of one visual-token patch in pixels.
pub const IMAGE_PATCH_PX: u32 = 28;
/// Standard resolution tier: maximum long edge in pixels.
pub const IMAGE_TIER_STANDARD_MAX_EDGE: u32 = 1_568;
/// Standard resolution tier: maximum visual tokens.
pub const IMAGE_TIER_STANDARD_MAX_TOKENS: u32 = 1_568;
/// High-resolution tier: maximum long edge in pixels.
pub const IMAGE_TIER_HIGH_MAX_EDGE: u32 = 2_576;
/// High-resolution tier: maximum visual tokens.
pub const IMAGE_TIER_HIGH_MAX_TOKENS: u32 = 4_784;
/// Refuse to decode an image above this many megapixels (decompression bomb guard).
pub const IMAGE_MAX_MEGAPIXELS: u64 = 100;
/// Send the original bytes up to this size when they already fit the tier.
pub const IMAGE_ORIGINAL_MAX_BYTES: u64 = 3_750_000;
/// JPEG quality for the re-encoded image.
pub const IMAGE_JPEG_QUALITY: u8 = 80;
/// Ceiling of a `data:` URI before the reader refuses it and asks for less.
pub const IMAGE_MAX_DATA_URI_BYTES: usize = 5_000_000;
/// Cache subdirectory that remembers the pictures an agent session received.
pub const MEDIA_SESSION_DIR_NAME: &str = "media-sessions";
/// Environment variable that names the agent session, when one is running.
pub const SESSION_ENV_VAR: &str = "CLAUDE_CODE_SESSION_ID";
/// Decompressed ceiling of one PDF page, so a crafted stream cannot inflate.
pub const PDF_MAX_DECOMPRESSED_BYTES: usize = 32 * 1024 * 1024;
/// Pages `extract-artifact` decodes text from, before the excerpt ceiling cuts.
pub const PDF_TEXT_MAX_PAGES: usize = 50;
/// Resolution a PDF page is rendered at, in dots per inch.
///
/// The PDF unit is 1/72 inch, so this is also the scale factor. 150 dpi makes
/// an A4 page about 1240 x 1754 pixels, which is under the standard image tier
/// and still legible for a scan.
pub const PDF_RENDER_DPI: f32 = 150.0;
/// Resolution a page is rendered at when the characters are being recognized.
///
/// Recognition reads glyphs one at a time, and 150 dpi leaves small print too
/// coarse to separate letters. 300 dpi is the resolution a scanner uses for
/// the same reason, and it costs only the time of one page.
pub const PDF_OCR_RENDER_DPI: f32 = 300.0;
/// Range `--dpi` accepts on a rendered page.
///
/// Below 36 the page is a thumbnail no print survives, and above 600 the
/// pixels grow four times per doubling while the eye gains nothing.
pub const PDF_RENDER_DPI_RANGE: std::ops::RangeInclusive<f32> = 36.0..=600.0;
/// Directory, under the cache directory, that keeps the OCR model files.
pub const OCR_MODEL_DIR_NAME: &str = "models";
/// Base URL the two OCR model files are fetched from.
///
/// The files are versioned by name, so the base never moves on its own. The
/// models are the ones the engine was trained with, published by their author.
pub const OCR_MODEL_BASE_URL: &str = "https://huggingface.co/robertknight/ocrs/resolve/main";
/// Text detection model file, which draws a box around every word.
pub const OCR_DETECTION_MODEL_FILE: &str = "text-detection-ssfbcj81.rten";
/// Text recognition model file, which turns each box into characters.
pub const OCR_RECOGNITION_MODEL_FILE: &str = "text-rec-checkpoint-s52qdbqt.rten";
/// Largest model file the fetch accepts, so a wrong URL cannot fill the disk.
pub const OCR_MODEL_MAX_BYTES: u64 = 64 * 1024 * 1024;
/// Time the model fetch may take before it gives up.
pub const OCR_MODEL_DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
/// Times a lock acquisition restarts when a rename swapped the inode under it.
pub const LOCK_STALE_INODE_MAX_RESTARTS: u32 = 8;

const _: () = assert!(READ_DEFAULT_MAX_BYTES < 30_000);
const _: () = assert!(IMAGE_TIER_STANDARD_MAX_EDGE > IMAGE_PATCH_PX);
/// Fragment of the error `grep-searcher` raises when a single line exceeds
/// the searcher's heap limit.
///
/// The library has no typed error for it, only this text, so the translation to
/// the crate's own vocabulary keys on it. If the wording ever changes, the match
/// simply stops and the raw message is reported unchanged instead of turning a
/// valid run into a failure.
pub const SEARCH_HEAP_LIMIT_MARKER: &str = "allocation limit";
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
/// Maximum number of per-file failures echoed in `errors[]` of a scan summary.
///
/// The failure counter stays exact; only the reported list is bounded so that a
/// tree full of unreadable files cannot grow memory without limit.
pub const MAX_REPORTED_FILE_FAILURES: usize = 50;
/// Files scored per batch by `semantic-search`; bounds resident memory and sets the
/// granularity at which the global deadline is polled between batches.
pub const SEMANTIC_SEARCH_BATCH_FILES: usize = 64;
/// Maximum characters of the source line kept as the `semantic-search` snippet.
pub const SEMANTIC_SNIPPET_CHARS: usize = 200;
/// Minimum byte length of a `semantic-search` token.
pub const SEMANTIC_TOKEN_MIN_LEN: usize = 2;
/// File name of the `semantic-search` inverted index inside `--index-dir`.
pub const SEMANTIC_INDEX_FILE: &str = "tokens.ndjson";
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
/// Default idle-exit bound for `watch` (A-ONESHOT-001: prefer short one-shot loops).
/// Overridable via CLI `--idle-exit-ms` or XDG `[watch].idle_exit_ms`.
pub const DEFAULT_WATCH_IDLE_EXIT_MS: u64 = 500;

// --- `config` list keys (`Kind::List`) ---
/// Separator of a `Kind::List` value typed on the command line.
pub const CONFIG_LIST_SEPARATOR: char = ',';
/// Most items one `Kind::List` key accepts.
pub const CONFIG_LIST_MAX_ITEMS: usize = 256;
/// Longest single item a `Kind::List` key accepts, in bytes.
pub const CONFIG_LIST_ITEM_MAX_BYTES: usize = 4096;

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
// --- T3a: strict `apply` (v0.1.37) ---
/// Lines a unified hunk may sit away from where its `@@` header says.
pub const PATCH_MAX_HUNK_SHIFT: usize = 64;
/// Characters of a file or patch line quoted in an `apply` error message.
pub const PATCH_ERROR_SNIPPET_CHARS: usize = 120;
/// Marker that stands for an unanchored run of lines in a SEARCH block.
pub const PATCH_ELLIPSIS_MARKER: &str = "...";
/// Most lines a `...` wildcard may skip between its two anchors.
pub const PATCH_ELLIPSIS_MAX_LINES: usize = 200;
// --- T3a: startup WAL heal scope and H3 window (v0.1.37) ---
/// Length of the H3 creation rate-limit window, in seconds.
pub const WAL_RATE_WINDOW_SECS: u64 = 60;
/// Directory names the startup heal never enters, even without a `.gitignore`.
pub const WAL_STARTUP_HEAL_SKIP_DIRS: &[&str] = &[".git", "target", "node_modules"];
/// Walk depth of the startup heal when the target is a file: its siblings only.
pub const WAL_STARTUP_HEAL_FILE_DEPTH: usize = 1;

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
/// `diff` similar/timeout budget for expensive similarity path (A-026).
pub const DIFF_SIMILARITY_TIMEOUT_MS: u64 = 500;
/// Max time `atomic_write` waits for the advisory lock of a target before it
/// fails with `LOCK_TIMEOUT` (exit 83), in milliseconds.
// Tunable: moves to XDG config in a later step
pub const LOCK_ACQUIRE_TIMEOUT_MS: u64 = 5_000;
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
const _: () = assert!(DIFF_SIMILARITY_TIMEOUT_MS >= 1);
const _: () = assert!(LOCK_ACQUIRE_TIMEOUT_MS >= 100);
const _: () = assert!(!BACKUP_EXCLUDE_GLOBS.is_empty());
const _: () = assert!(EVENT_CHANNEL_CAP >= 16);
const _: () = assert!(RAM_PER_TASK_BYTES >= 1024 * 1024);

// --- Terminal summary failure report (v0.1.37, T2) ---
/// Default for `summary.errors_max_entries`: most `errors[]` entries a terminal
/// summary lists, while `files_failed` keeps the full count.
pub const SUMMARY_ERRORS_MAX_ENTRIES: usize = 50;
/// Smallest `summary.errors_max_entries` the config accepts.
pub const SUMMARY_ERRORS_MIN_ENTRIES: usize = 1;
/// Largest `summary.errors_max_entries` the config accepts.
///
/// An unbounded list would let one broken tree push megabytes of failure text
/// into a single record, which is the cost the cap exists to avoid.
pub const SUMMARY_ERRORS_CEILING: usize = 1000;
const _: () = assert!(SUMMARY_ERRORS_MAX_ENTRIES >= SUMMARY_ERRORS_MIN_ENTRIES);
const _: () = assert!(SUMMARY_ERRORS_MAX_ENTRIES <= SUMMARY_ERRORS_CEILING);
/// Config key named by the fuzzy content-cap `InvalidInput` reason.
///
/// The reason is built from this constant and the localized hint is chosen by
/// finding it in the reason, so the two cannot drift apart; a test in
/// `src/error/tests.inc.rs` fails if the emitted reason stops matching.
pub const FUZZY_CONTENT_CAP_REASON_MARKER: &str = "[fuzzy].max_content_bytes";
/// Opening words of the fuzzy pattern-cap `InvalidInput` reason (same contract as above).
pub const FUZZY_PATTERN_CAP_REASON_MARKER: &str = "fuzzy pattern too large";

// --- Config schema and log sink (v0.1.37, R25/R26) ---
/// Highest `version` value a config file may declare; a larger one fails closed.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;
/// Rotated log files kept by the opt-in file sink (`[log] file = true`).
pub const LOG_MAX_FILES: usize = 7;
/// Default rotation of the opt-in file sink.
pub const LOG_ROTATION_DEFAULT: &str = "daily";
const _: () = assert!(LOG_MAX_FILES >= 1);

// --- Retry, thread caps and in-flight memory (T3b) ---
include!("constants_concurrency.inc.rs");

// --- Named literals moved out of production code (T6b) ---
include!("constants_literals.inc.rs");

// --- Config verb and tuning keys (T6a) ---
include!("constants_config.inc.rs");
