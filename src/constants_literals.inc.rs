// SPDX-License-Identifier: MIT OR Apache-2.0
// Named literals moved out of production code (v0.1.37, T6b).
// Included from constants.rs.

// --- Time units and the civil-calendar conversion (src/atomic/time_fmt.rs) ---
/// Seconds in one minute.
pub const SECS_PER_MINUTE: u64 = 60;
/// Seconds in one hour.
pub const SECS_PER_HOUR: u64 = 60 * SECS_PER_MINUTE;
/// Seconds in one day (UTC, no leap seconds).
pub const SECS_PER_DAY: u64 = 24 * SECS_PER_HOUR;
/// Seconds in one week.
pub const SECS_PER_WEEK: u64 = 7 * SECS_PER_DAY;
/// Milliseconds in one second, as a float for rate and ETA arithmetic.
pub const MILLIS_PER_SECOND_F64: f64 = 1000.0;
/// Days from 0000-03-01 to 1970-01-01, the shift of the days-from-civil algorithm.
pub const CIVIL_EPOCH_SHIFT_DAYS: i64 = 719_468;
/// Days in one 400-year Gregorian era.
pub const CIVIL_DAYS_PER_ERA: i64 = 146_097;
/// Days in one 100-year Gregorian century (a century year is not a leap year).
pub const CIVIL_DAYS_PER_CENTURY: u64 = 36_524;
/// Days in one 4-year leap cycle.
pub const CIVIL_DAYS_PER_FOUR_YEARS: u64 = 1460;
/// Days in a common year.
pub const CIVIL_DAYS_PER_YEAR: u64 = 365;
/// Years in one Gregorian era.
pub const CIVIL_YEARS_PER_ERA: i64 = 400;

// --- Sidecar, backup and probe names ---
/// Infix of a timestamped backup name: `<file>.bak.<YYYYMMDD_HHMMSS[_mmm]>`.
pub const BACKUP_INFIX: &str = ".bak.";
/// Substring of an exclude glob that makes `hash --recursive` skip backup files.
pub const BACKUP_NAME_MARKER: &str = "bak";
/// Digits of the date part of a backup stamp (`YYYYMMDD`).
pub const BACKUP_STAMP_DATE_DIGITS: usize = 8;
/// Digits of the time part of a backup stamp (`HHMMSS`).
pub const BACKUP_STAMP_TIME_DIGITS: usize = 6;
/// Digits of the optional millisecond part of a backup stamp (`mmm`).
pub const BACKUP_STAMP_MILLIS_DIGITS: usize = 3;
/// Byte that separates the parts of a backup stamp.
pub const BACKUP_STAMP_SEPARATOR: u8 = b'_';
/// Prefix of a WAL sidecar journal name: `.atomwrite.journal.<file>.atomwrite.journal.json`.
pub const WAL_JOURNAL_PREFIX: &str = ".atomwrite.journal.";
/// Suffix of a WAL sidecar journal name.
pub const WAL_JOURNAL_EXT: &str = ".atomwrite.journal.json";
/// Sentinel file name that KEEPS the WAL sidecars of its directory (heuristic H4).
pub const WAL_KEEP_SENTINEL: &str = ".atomwrite_keep_wal";
/// Legacy sentinel name, still honoured, that said the opposite of what it did.
///
/// A directory holding this file kept its journal all along. The name promised
/// to disable the WAL, so the reader of the file could not predict the effect.
/// Both names are accepted so an existing sentinel keeps working; the new name
/// is the one documented.
pub const WAL_LEGACY_SENTINEL: &str = ".atomwrite_no_wal";
/// Leading marker of a hidden sidecar file name.
pub const HIDDEN_FILE_PREFIX: &str = ".";
/// Suffix of the advisory lock sidecar: `.<file>.atomwrite.lock`.
pub const LOCK_SIDECAR_SUFFIX: &str = ".atomwrite.lock";
/// File `doctor` creates and removes to prove a directory is writable.
pub const DOCTOR_PROBE_FILE: &str = ".atomwrite-doctor-probe";
/// Directory the `doctor` guard counts to learn how many test targets exist.
pub const TESTS_DIR: &str = "tests";
/// Live documents whose stated test-target count the `doctor` guard checks.
///
/// Only the documents that speak about the current release: a historical
/// changelog entry states what a past run measured and must not be rewritten.
pub const DOCS_TARGET_COUNT_FILES: &[&str] = &[
    "README.md",
    "README.pt-BR.md",
    "llms.txt",
    "llms.pt-BR.txt",
    "llms-full.txt",
    "llms-full.pt-BR.txt",
    "docs/TESTING.md",
    "docs/TESTING.pt-BR.md",
    "docs/INSTALL.md",
    "docs/INSTALL.pt-BR.md",
    "docs/CROSS_PLATFORM.md",
    "docs/CROSS_PLATFORM.pt-BR.md",
];
/// Live documents whose stated config-key count the `doctor` guard checks.
pub const DOCS_KEY_COUNT_FILES: &[&str] = &[
    "README.md",
    "README.pt-BR.md",
    "llms.txt",
    "llms.pt-BR.txt",
    "llms-full.txt",
    "llms-full.pt-BR.txt",
    "docs/AGENTS.md",
    "docs/AGENTS.pt-BR.md",
    "docs/HOW_TO_USE.md",
    "docs/HOW_TO_USE.pt-BR.md",
    "docs/INSTALL.md",
    "docs/INSTALL.pt-BR.md",
    "docs/INTEGRATIONS.md",
    "docs/INTEGRATIONS.pt-BR.md",
    "docs/MIGRATION.md",
    "docs/CROSS_PLATFORM.md",
    "docs/CROSS_PLATFORM.pt-BR.md",
];
/// Phrases that state a test-target count, in both UI languages.
pub const DOCS_TARGET_PHRASES: &[&str] =
    &["test targets", "test target", "alvos de teste"];
/// Phrases that state a config-key count, in both UI languages.
pub const DOCS_KEY_PHRASES: &[&str] = &["keys", "chaves"];
/// Name of the git directory marker looked up while walking parents.
pub const GIT_DIR_NAME: &str = ".git";
/// Name of the ignore file `watch` registers for the watched root.
pub const GITIGNORE_FILE: &str = ".gitignore";
/// File-name prefix of the log files the opt-in file sink writes.
pub const LOG_FILE_PREFIX: &str = "atomwrite.";
/// Unix mode of the log files the opt-in file sink writes (owner read and write only).
pub const LOG_FILE_PERMISSIONS: u32 = 0o600;
/// Name of the thread that enforces the global `--timeout-secs` deadline.
pub const THREAD_NAME_TIMEOUT: &str = "atomwrite-timeout";
/// Name of the thread that drains the non-blocking log writer.
pub const THREAD_NAME_LOG: &str = "atomwrite-log";
/// Name of the thread that reads the real stdin for cooperative cancellation.
pub const THREAD_NAME_STDIN: &str = "atomwrite-stdin";

// --- Kernel interfaces read by `env_detect` and `concurrency` (Linux) ---
/// Control groups of PID 1, the first container hint.
pub const PROC_INIT_CGROUP_PATH: &str = "/proc/1/cgroup";
/// Control groups of this process, the fallback container hint.
pub const PROC_SELF_CGROUP_PATH: &str = "/proc/self/cgroup";
/// Kernel release string, used to spot WSL.
pub const PROC_OSRELEASE_PATH: &str = "/proc/sys/kernel/osrelease";
/// Kernel version banner, used to spot WSL.
pub const PROC_VERSION_PATH: &str = "/proc/version";
/// Mounted Kubernetes service-account directory, present only inside a pod.
pub const KUBERNETES_SECRETS_DIR: &str = "/var/run/secrets/kubernetes.io/serviceaccount";
/// Marker file Docker creates at the root of a container.
pub const DOCKERENV_PATH: &str = "/.dockerenv";
/// Marker file Flatpak creates inside a sandbox.
pub const FLATPAK_INFO_PATH: &str = "/.flatpak-info";
/// Termux user prefix on Android.
pub const TERMUX_USR_DIR: &str = "/data/data/com.termux/files/usr";
/// Link to the running executable.
pub const PROC_SELF_EXE_PATH: &str = "/proc/self/exe";
/// Substrings of a cgroup path that mean the process runs inside a container.
pub const CONTAINER_CGROUP_MARKERS: [&str; 4] = ["docker", "kubepods", "containerd", "libpod"];
/// Memory counters of the host.
pub const PROC_MEMINFO_PATH: &str = "/proc/meminfo";
/// Line of `/proc/meminfo` that reports the memory available without swapping.
pub const PROC_MEMINFO_AVAILABLE_KEY: &str = "MemAvailable:";
/// Bytes in one KiB, the unit `/proc/meminfo` reports in.
pub const BYTES_PER_KIB: u64 = 1024;

// --- Platform codes ---
/// POSIX `EXDEV`: rename across filesystems.
pub const ERRNO_EXDEV: i32 = 18;
/// POSIX `EISDIR`: the operation does not apply to a directory.
pub const ERRNO_EISDIR: i32 = 21;
/// Windows code page identifier of UTF-8.
pub const WINDOWS_CODE_PAGE_UTF8: u32 = 65001;
/// Lowest printable ASCII code point; anything below it is a control character.
pub const ASCII_CONTROL_LIMIT: u32 = 32;
/// Permission bits `read` reports (rwx for user, group, other plus setuid, setgid, sticky).
pub const UNIX_MODE_PERMISSION_MASK: u32 = 0o7777;
/// Byte order mark that opens a UTF-8 file.
pub const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
/// Largest integer a JSON consumer reading `f64` represents exactly (2^53).
pub const JSON_SAFE_INTEGER_LIMIT: f64 = 9_007_199_254_740_992.0;
/// Letters in the Latin alphabet, the radix of a spreadsheet column reference.
pub const LATIN_ALPHABET_LEN: usize = 26;

// --- Allocation hints, probes and truncation limits ---
/// Smallest accepted `[write].shrink_block_percent`.
pub const SHRINK_BLOCK_PERCENT_MIN: u8 = 1;
/// Largest accepted `[write].shrink_block_percent`.
pub const SHRINK_BLOCK_PERCENT_MAX: u8 = 99;
/// Smallest accepted similarity threshold.
pub const SIMILARITY_MIN: f64 = 0.0;
/// Largest accepted similarity threshold.
pub const SIMILARITY_MAX: f64 = 1.0;
/// Unix mode of a directory the product creates for itself (owner only).
pub const PRIVATE_DIR_PERMISSIONS: u32 = 0o700;
/// Install directory and file name of the bash completion script.
pub const COMPLETION_BASH: (&str, &str) = ("bash-completion/completions", "atomwrite");
/// Install directory and file name of the zsh completion script.
pub const COMPLETION_ZSH: (&str, &str) = ("zsh/site-functions", "_atomwrite");
/// Install directory and file name of the fish completion script.
pub const COMPLETION_FISH: (&str, &str) = ("fish/vendor_completions.d", "atomwrite.fish");
/// Install directory and file name of the `PowerShell` completion script.
pub const COMPLETION_POWERSHELL: (&str, &str) = ("powershell/Completions", "_atomwrite.ps1");
/// Install directory and file name of the elvish completion script.
pub const COMPLETION_ELVISH: (&str, &str) = ("elvish/lib", "atomwrite.elv");
/// Percent scale: a whole is 100.
pub const PERCENT_SCALE: u64 = 100;
/// Size delta (percent) at or above which a write warning is `high`.
pub const WRITE_RISK_HIGH_PERCENT: u32 = 90;
/// Size delta (percent) at or above which a write warning is `medium`.
pub const WRITE_RISK_MEDIUM_PERCENT: u32 = 70;
/// Multiplier on the stdin initial capacity for the soft pre-reserve cap.
pub const STDIN_RESERVE_CAP_FACTOR: usize = 64;
/// Pseudo-path that names stdin in an error envelope.
///
/// The same spelling `diff` and `list` already use for a `-` operand, so a
/// machine consumer sees one token for "the payload came from stdin".
pub const STDIN_SUBJECT: &str = "-";
/// Bytes of a script head inspected for a shebang line.
pub const SHEBANG_PROBE_BYTES: usize = 8192;
/// Characters of source kept in a syntax-error snippet.
pub const SYNTAX_SNIPPET_CHARS: usize = 80;
/// Initial depth of the tree-sitter traversal stack in the syntax check.
pub const SYNTAX_STACK_CAPACITY: usize = 64;
/// File extensions the AST verbs map to a tree-sitter language name.
///
/// One table, because `syntax_check` resolves a language from an extension and
/// the `doctor` grammar check must ask about the same languages the verbs will
/// need. Two copies would let the doctor bless a language the verbs never use.
pub const EXTENSION_LANGUAGES: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("py", "python"),
    ("js", "javascript"),
    ("mjs", "javascript"),
    ("cjs", "javascript"),
    ("jsx", "javascript"),
    ("ts", "typescript"),
    ("tsx", "tsx"),
    ("go", "go"),
    ("c", "c"),
    ("h", "c"),
    ("cpp", "cpp"),
    ("cc", "cpp"),
    ("cxx", "cpp"),
    ("hpp", "cpp"),
    ("hxx", "cpp"),
    ("java", "java"),
    ("rb", "ruby"),
    ("php", "php"),
    ("sh", "bash"),
    ("bash", "bash"),
    ("zsh", "bash"),
    ("html", "html"),
    ("htm", "html"),
    ("css", "css"),
    ("json", "json"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("toml", "toml"),
    ("md", "markdown"),
    ("markdown", "markdown"),
    ("lua", "lua"),
    ("scala", "scala"),
    ("swift", "swift"),
    ("kt", "kotlin"),
    ("kts", "kotlin"),
    ("sql", "sql"),
];
/// Bytes of a node scanned for the one-line `outline` signature.
pub const OUTLINE_SIGNATURE_MAX_BYTES: usize = 400;
/// Characters of node text kept by `query`.
pub const QUERY_NODE_TEXT_MAX_CHARS: usize = 240;
/// Initial depth of the `query` traversal stack.
pub const QUERY_STACK_CAPACITY: usize = 64;
/// Lines of a patch inspected to detect its format.
pub const PATCH_FORMAT_PROBE_LINES: usize = 15;
/// Initial capacity of the patch-format probe buffer, in bytes.
pub const PATCH_FORMAT_PROBE_CAPACITY: usize = 256;
/// Initial capacity of the hunk list of one patch.
pub const PATCH_HUNKS_CAPACITY: usize = 4;
/// Initial capacity of the operation list of one hunk.
pub const PATCH_HUNK_OPS_CAPACITY: usize = 8;
/// Fields a unified-diff hunk header needs (`@@`, old range, new range).
pub const PATCH_HUNK_HEADER_MIN_FIELDS: usize = 3;
/// Initial capacity of the `batch` operation list.
pub const BATCH_OPS_CAPACITY: usize = 16;
/// Elapsed seconds floor that keeps the `batch` rate finite.
pub const BATCH_RATE_MIN_ELAPSED_SECS: f64 = 0.001;
/// Initial capacity of the `edit-loop` pair list.
pub const EDIT_PAIRS_CAPACITY: usize = 8;
/// Initial capacity of the `edit --multi` operation list.
pub const MULTI_EDIT_OPS_CAPACITY: usize = 8;
/// Initial capacity of the submatch list of one search line.
pub const SEARCH_SUBMATCHES_CAPACITY: usize = 4;
/// Multiplier of `search.max_filesize` that sets the heap limit of a searcher.
///
/// The line buffer starts small and grows by doubling until it holds the
/// longest single line, and `--multiline` puts the whole file on the heap when
/// there is no mmap. The limit therefore follows the largest file the gates
/// admit (`search.max_filesize`) instead of a fixed byte count: measured on a
/// 4 GiB one-line file, the growth copies cost 43, 131, 385 and 1097 ms, and
/// the cancel flag was only observed after the last one. The doubling can
/// overshoot the size it needed by one step, so the factor leaves that
/// headroom above the file the owner allowed.
pub const SEARCH_HEAP_LIMIT_FACTOR: u64 = 2;
/// Initial capacity of the candidate list of one fuzzy window scan.
pub const FUZZY_WINDOW_CANDIDATES_CAPACITY: usize = 4;
/// Slack bytes reserved when a fuzzy replacement is re-indented.
pub const FUZZY_REINDENT_SLACK_BYTES: usize = 16;
/// Slack bytes reserved for the extra newline of a line-based fuzzy replacement.
pub const FUZZY_LINES_SLACK_BYTES: usize = 4;
/// Pattern length (bytes) below which a single-line pattern also tries Jaro-Winkler.
pub const FUZZY_JW_MAX_PATTERN_BYTES: usize = 60;
/// `strategies_tried` of the `exact` match.
pub const FUZZY_TRIED_EXACT: u64 = 1;
/// `strategies_tried` of `line_trimmed`.
pub const FUZZY_TRIED_LINE_TRIMMED: u64 = 2;
/// `strategies_tried` of `whitespace_normalized`.
pub const FUZZY_TRIED_WHITESPACE: u64 = 3;
/// `strategies_tried` of `punctuation_normalized`.
pub const FUZZY_TRIED_PUNCTUATION: u64 = 4;
/// `strategies_tried` of `indent_flexible`.
pub const FUZZY_TRIED_INDENT: u64 = 5;
/// `strategies_tried` of `escape_normalized`.
pub const FUZZY_TRIED_ESCAPE: u64 = 6;
/// `strategies_tried` of `trimmed_boundary`.
pub const FUZZY_TRIED_TRIMMED_BOUNDARY: u64 = 7;
/// `strategies_tried` of `block_anchor`.
pub const FUZZY_TRIED_BLOCK_ANCHOR: u64 = 8;
/// `strategies_tried` of `context_aware` and `context_aware_jw`.
pub const FUZZY_TRIED_CONTEXT_AWARE: u64 = 9;
/// `strategies_tried` of `unicode_normalized`.
pub const FUZZY_TRIED_UNICODE: u64 = 10;
/// Directory levels the missing-file suggestion scan descends.
pub const SUGGEST_WALK_MAX_DEPTH: usize = 6;
/// Jaro-Winkler score a file name needs to be suggested for a missing path.
pub const SUGGEST_MIN_SIMILARITY: f64 = 0.75;
/// Most paths a missing-file suggestion lists.
pub const SUGGEST_MAX_PATHS: usize = 2;
/// Hex characters of the digest that names a WAL journal.
pub const WAL_JOURNAL_ID_HEX_LEN: usize = 16;
/// Parent directories the git-tracked probe climbs before it gives up.
pub const WAL_GIT_PROBE_MAX_DEPTH: u8 = 16;
/// Startup heal batches hold this many journals per pool thread.
pub const WAL_HEAL_CHUNK_PER_THREAD: usize = 4;
/// A record longer than this multiple of the line cap is not drained to its newline.
pub const OUTPUT_DRAIN_CAP_FACTOR: usize = 16;
/// Pixels in one megapixel.
pub const IMAGE_PIXELS_PER_MEGAPIXEL: u128 = 1_000_000;
/// Scale of the three decimals `megapixels` keeps.
pub const IMAGE_MEGAPIXEL_SCALE: u128 = 1_000;
/// Parent hops followed when a PDF page inherits its `MediaBox`.
pub const PDF_MEDIABOX_MAX_PARENT_HOPS: usize = 32;
/// Largest NDJSON table `--join` reads from stdin, in bytes.
///
/// The whole table is indexed in memory once, so a table past this bound is
/// refused instead of read.
pub const JOIN_TABLE_MAX_BYTES: u64 = 16 * 1024 * 1024;
/// Words every `--join` failure carries in its `reason`.
///
/// The emitters build the reason from this constant and `error/suggest.inc.rs`
/// finds it there to pick the localized hint, so the two cannot drift apart.
pub const JOIN_REASON_MARKER: &str = "--join";
/// Characters that turn a `read` operand into a pattern instead of a path.
///
/// The set is the shell's own: `*`, `?` and the `[` that opens a bracket
/// expression. `read` expands the operand itself, so no shell is involved.
pub const GLOB_METACHARS: &[char] = &['*', '?', '['];

/// Recursive glob marker, refused because `read` expands one level only.
///
/// Accepting it would answer a recursive request with the non-recursive result
/// the shell gives without `globstar`, which is not what the caller asked for.
pub const RECURSIVE_GLOB: &str = "**";
/// Directory named by `.` when a pattern carries no directory of its own.
pub const CURRENT_DIR: &str = ".";
/// Block size of the raw concatenation in `read --format raw`.
///
/// Fixed so the peak memory of a multi-file read does not follow the sum of
/// the file sizes, and so the cancel check runs at least every 64 KiB.
pub const RAW_COPY_BUF_BYTES: usize = 64 * 1024;
/// Words the `read` reason carries when a verb takes one target but got many.
///
/// The emitter builds the reason from this constant and `error/suggest.inc.rs`
/// finds it there to pick the localized hint, so the two cannot drift apart.
pub const READ_MULTI_TARGET_REASON_MARKER: &str = "expects a single target";
/// Words the reason carries when the verb itself answers for one file only.
///
/// Distinct from `READ_MULTI_TARGET_REASON_MARKER` on purpose: that hint talks
/// about carving flags, and a call that named two files without any carving
/// flag must not be told to drop a flag it never passed. The two markers must
/// not overlap as substrings, or the first branch wins and the hint lies again.
pub const SINGLE_TARGET_REASON_MARKER: &str = "one target per invocation";
/// Marker placed in the reason when `--no-comments` came without `--number`.
pub const NO_COMMENTS_NEEDS_NUMBER_REASON_MARKER: &str = "needs --number";

/// Marker placed in the `prune-backups` reason when the subject is a directory.
pub const PRUNE_DIRECTORY_REASON_MARKER: &str = "prune-backups takes files";
/// Marker placed in the reason when `--confirm-side-effect` names a read-only verb.
pub const SIDE_EFFECT_NO_SIDE_EFFECT_REASON_MARKER: &str = "writes nothing";
/// Marker placed in the reason when a named file's extension is another language.
pub const LANGUAGE_MISMATCH_REASON_MARKER: &str = "extension decides the grammar";
/// Marker placed in the reason when the verb needs a feature this build lacks.
///
/// The stub of an optional verb reports `ConfigInvalid` with this phrase, and
/// the suggestion layer reads it to answer "rebuild with the feature" instead
/// of sending the operator to the configuration keys.
pub const FEATURE_MISSING_REASON_MARKER: &str = "requires rebuild";
/// Largest retry budget accepted, in seconds; keeps `Instant` arithmetic from overflowing.
pub const RETRY_MAX_BUDGET_SECS: u64 = u32::MAX as u64;
/// Minimum length for parallel path sort (`par_sort_unstable`); below it a sequential sort is cheaper.
pub const PAR_SORT_THRESHOLD: usize = 4096;

const _: () = assert!(SECS_PER_DAY == 86_400);
const _: () = assert!(SECS_PER_WEEK == 604_800);
const _: () = assert!(CIVIL_DAYS_PER_ERA as u64 == 400 * CIVIL_DAYS_PER_YEAR + 97);
const _: () = assert!(WRITE_RISK_HIGH_PERCENT > WRITE_RISK_MEDIUM_PERCENT);
const _: () = assert!(PERCENT_SCALE == 100);
const _: () = assert!(FUZZY_TRIED_UNICODE > FUZZY_TRIED_CONTEXT_AWARE);
const _: () = assert!(SUGGEST_MAX_PATHS >= 1);
const _: () = assert!(WAL_JOURNAL_ID_HEX_LEN >= 8 && WAL_JOURNAL_ID_HEX_LEN <= 64);
const _: () = assert!(UTF8_BOM.len() == 3);
