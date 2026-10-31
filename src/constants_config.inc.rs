// SPDX-License-Identifier: MIT OR Apache-2.0
// Config verb and tuning constants (v0.1.37, T6a).
// Included from constants.rs.

// --- Config files (src/config.rs, src/storage.rs, src/commands/config_cmd.rs) ---
/// `ProjectDirs` application name that roots every XDG path of the product.
pub const STORAGE_APP_NAME: &str = "atomwrite";
/// File name of the global config inside the config directory.
pub const GLOBAL_CONFIG_FILE: &str = "config.toml";
/// File name of the workspace config, at the workspace root.
pub const WORKSPACE_CONFIG_FILE: &str = ".atomwrite.toml";
/// File name of the persisted UI locale preference inside the config directory.
pub const LOCALE_PREFERENCE_FILE: &str = "locale";
/// Unix mode of every config file the `config` verb and `locale --set` write.
pub const CONFIG_FILE_PERMISSIONS: u32 = 0o600;

// --- Reason markers of the tuning validation (src/config_tuning.inc.rs) ---
/// Words inside the `reason` of a value outside its range.
///
/// The emitter builds the reason from this constant and `error/suggest.inc.rs`
/// finds it there to pick the localized hint, so the two cannot drift apart.
pub const CONFIG_OUT_OF_RANGE_REASON_MARKER: &str = "is out of range; must be";
/// Words inside the `reason` of a key above the key it must not exceed.
pub const CONFIG_RELATION_REASON_MARKER: &str = "must not exceed";

// --- Accepted range of every tuning key (src/config_tuning.inc.rs) ---
// `config set` and the startup load share these bounds, so a value that one
// accepts the other never rejects.
/// Highest accepted percentage value.
pub const PERCENT_MAX: u64 = 100;
/// Lowest accepted `concurrency.max_threads`.
pub const CONCURRENCY_MIN_THREADS: u64 = 1;
/// Highest accepted `concurrency.max_threads`.
pub const CONCURRENCY_MAX_THREADS_CEILING: u64 = 256;
/// Lowest accepted `concurrency.ram_fraction_percent`.
pub const RAM_FRACTION_PERCENT_MIN: u64 = 1;
/// Lowest accepted `lock.acquire_timeout_ms`, in milliseconds.
pub const LOCK_MIN_ACQUIRE_TIMEOUT_MS: u64 = 100;
/// Highest accepted `lock.acquire_timeout_ms` (one hour), in milliseconds.
pub const LOCK_MAX_ACQUIRE_TIMEOUT_MS: u64 = 3_600_000;
/// Lowest accepted `retry.base_ms`, `retry.max_ms` and `retry.max_elapsed_ms`.
pub const RETRY_MIN_MS: u64 = 1;
/// Highest accepted `retry.base_ms` and `retry.max_ms` (one minute).
pub const RETRY_MAX_WAIT_CEILING_MS: u64 = 60_000;
/// Highest accepted `retry.max_elapsed_ms` (one hour).
pub const RETRY_MAX_ELAPSED_CEILING_MS: u64 = 3_600_000;
/// Lowest accepted `search.max_inflight_bytes` (4 MiB).
///
/// The `search` pipeline also raises the effective budget to what its workers
/// need (`SEARCH_BUDGET_BYTES_PER_WORKER` per worker), so a value above this
/// minimum never starves it.
pub const SEARCH_MIN_INFLIGHT_BYTES: u64 = 4 * 1024 * 1024;
/// Highest accepted `search.max_inflight_bytes` (1 GiB).
pub const SEARCH_MAX_INFLIGHT_CEILING_BYTES: u64 = 1024 * 1024 * 1024;
/// Highest accepted `extract.max_sample_rows`.
pub const EXTRACT_MAX_SAMPLE_ROWS_CEILING: u64 = 100_000;
/// Highest accepted `extract.max_text_chars`.
pub const EXTRACT_MAX_TEXT_CHARS_CEILING: u64 = 10_000_000;
/// Highest accepted `extract.max_sheets` and `extract.max_slides`.
pub const EXTRACT_MAX_PARTS_CEILING: u64 = 10_000;

const _: () = assert!(LOCK_ACQUIRE_TIMEOUT_MS >= LOCK_MIN_ACQUIRE_TIMEOUT_MS);
const _: () = assert!(LOCK_ACQUIRE_TIMEOUT_MS <= LOCK_MAX_ACQUIRE_TIMEOUT_MS);
const _: () = assert!(SEARCH_MAX_INFLIGHT_BYTES >= SEARCH_MIN_INFLIGHT_BYTES);
const _: () = assert!(SEARCH_MAX_INFLIGHT_BYTES <= SEARCH_MAX_INFLIGHT_CEILING_BYTES);
const _: () = assert!(MAX_AUTO_THREADS as u64 <= CONCURRENCY_MAX_THREADS_CEILING);
const _: () = assert!(RAM_FRACTION_PERCENT >= RAM_FRACTION_PERCENT_MIN);
const _: () =
    assert!(LOCK_RETRY_BASE_MS >= RETRY_MIN_MS && LOCK_RETRY_CAP_MS <= RETRY_MAX_WAIT_CEILING_MS);
const _: () = assert!(PERSIST_RETRY_MAX_ELAPSED_MS <= RETRY_MAX_ELAPSED_CEILING_MS);
const _: () = assert!(CONFIG_FILE_PERMISSIONS & 0o077 == 0);
