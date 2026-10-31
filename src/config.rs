// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration file loading for `.atomwrite.toml`.
//!
//! Hierarchy (highest wins): CLI flags > local config > global XDG config > defaults.
//! G-007: product knobs are never read from process environment variables.

use std::path::Path;

use crate::cli_args::FuzzyMode;
use crate::error::AtomwriteError;

/// Top-level configuration structure matching `.atomwrite.toml`.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AtomwriteConfig {
    /// Schema version of the file; absent means unversioned. A value above
    /// [`crate::constants::CONFIG_SCHEMA_VERSION`] is rejected (fail closed).
    pub version: u32,
    /// Default settings for write operations.
    pub defaults: DefaultsSection,
    /// Write guard policy (confirm / shrink / auto-rotate) — G-028.
    pub write: WriteSection,
    /// Watch one-shot bounds (B-007 / R-XDG-007).
    pub watch: WatchSection,
    /// Fuzzy matching defaults for edit operations.
    pub fuzzy: FuzzySection,
    /// Search defaults.
    pub search: SearchSection,
    /// Storage paths (XDG home override).
    pub storage: StorageSection,
    /// Opt-in diagnostics file sink (global XDG config only).
    pub log: LogSection,
    /// Worker-bound tuning.
    pub concurrency: ConcurrencySection,
    /// Advisory lock tuning.
    pub lock: LockSection,
    /// Retry backoff tuning.
    pub retry: RetrySection,
    /// `extract-artifact` bounds.
    pub extract: ExtractSection,
    /// Terminal summary bounds.
    pub summary: SummarySection,
}

/// Diagnostics file sink loaded from the global XDG config `[log]` (R26).
///
/// Off by default: with `file = false` no log file is opened and no writer
/// thread starts. Only the global XDG config can turn it on, because tracing is
/// initialised before the workspace `.atomwrite.toml` is read.
#[derive(Debug, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogSection {
    /// Tee diagnostics to a JSON-lines file under the XDG state directory.
    pub file: bool,
    /// Rotation of that file: `daily` or `hourly`.
    pub rotation: String,
}

impl Default for LogSection {
    fn default() -> Self {
        Self {
            file: false,
            rotation: crate::constants::LOG_ROTATION_DEFAULT.into(),
        }
    }
}

/// Validate `[log]` (fail closed on bad XDG config).
///
/// # Errors
///
/// Returns `AtomwriteError::ConfigInvalid` when `rotation` is neither `daily` nor `hourly`.
pub fn validate_log(cfg: &LogSection) -> Result<(), AtomwriteError> {
    match cfg.rotation.as_str() {
        "daily" | "hourly" => Ok(()),
        other => Err(AtomwriteError::ConfigInvalid {
            reason: format!(
                "config [log] rotation = \"{other}\" is invalid; use \"daily\" or \"hourly\""
            ),
        }),
    }
}

/// Log policy for tracing init, which runs before [`load_config`].
///
/// Reads both layers the rest of the CLI reads: the global XDG config and the
/// workspace `.atomwrite.toml`, merged by the same `merge_layers` the full
/// load uses. Reading only the global file silently discarded a `[log]` block
/// the owner had written in the workspace, which is the silent-discard family
/// this project treats as a defect: configuration accepted, validated and then
/// ignored.
///
/// A malformed or out-of-range `[log]` is a hard `CONFIG_INVALID` here, the same
/// answer [`load_config`] gives, so the two readers cannot disagree.
///
/// # Errors
///
/// Returns [`AtomwriteError::ConfigInvalid`] when either file is malformed or
/// when `[log].rotation` is neither `daily` nor `hourly`.
pub fn startup_log_section(
    workspace: &Path,
    explicit_path: Option<&Path>,
) -> Result<LogSection, AtomwriteError> {
    // `--config` names the whole configuration for this run, so it must also be
    // the whole log policy. Reading the layers it replaced here is what made
    // `--config /dev/null` print a workspace `[log]` error and still run.
    if let Some(path) = explicit_path {
        let log = parse_config(path)?.log;
        validate_log(&log)?;
        return Ok(log);
    }

    let global = match global_text() {
        Some(g) => match &g.text {
            Ok(text) => Some(parse_layer(&g.path, text)?),
            Err(e) => {
                return Err(AtomwriteError::ConfigInvalid {
                    reason: format!("cannot read config {}: {e}", g.path.display()),
                });
            }
        },
        None => None,
    };
    let local_path = workspace.join(crate::constants::WORKSPACE_CONFIG_FILE);
    let local = if local_path.is_file() {
        let text =
            std::fs::read_to_string(&local_path).map_err(|e| AtomwriteError::ConfigInvalid {
                reason: format!("cannot read config {}: {e}", local_path.display()),
            })?;
        Some(parse_layer(&local_path, &text)?)
    } else {
        None
    };
    let log = merge_layers(global, local)?.log;
    validate_log(&log)?;
    Ok(log)
}

/// Watch policy loaded from XDG / `.atomwrite.toml` `[watch]` (R-XDG-007 / A-XDG-002).
#[derive(Debug, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WatchSection {
    /// Exit after this many idle milliseconds with zero filesystem events (0 = disable idle-exit).
    pub idle_exit_ms: u64,
    /// Debounce coalesce window in milliseconds (A-XDG-002).
    pub debounce_ms: u64,
}

impl Default for WatchSection {
    fn default() -> Self {
        Self {
            idle_exit_ms: crate::constants::DEFAULT_WATCH_IDLE_EXIT_MS,
            debounce_ms: crate::constants::DEFAULT_WATCH_DEBOUNCE_MS,
        }
    }
}

/// Write-guard policy loaded from XDG / `.atomwrite.toml` `[write]` (G-028).
///
/// `Clone` exists for the fan-out verbs: a worker pool reads the same policy
/// from every thread, and each worker would otherwise have to hold a borrow of
/// the owner's config alive for the whole walk.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WriteSection {
    /// Bytes above which overwriting an existing target requires `--ack-overwrite` (A-WRITE-001 default-deny).
    pub confirm_large_bytes: u64,
    /// Block writes that shrink the target by more than this percent (1–99).
    pub shrink_block_percent: u8,
    /// Auto-rotate backup window for recently modified files (seconds).
    pub auto_rotate_max_age_secs: u64,
    /// Substrings that elevate write content risk (B-013 / R-XDG-013).
    ///
    /// Empty list means “use built-in defaults” from
    /// [`crate::constants::WRITE_CONTENT_RISK_PATTERNS`]. Non-empty replaces the
    /// default list entirely (agent/XDG policy override — no hardcode lock-in).
    #[serde(default)]
    pub content_risk_patterns: Vec<String>,
}

impl Default for WriteSection {
    fn default() -> Self {
        Self {
            confirm_large_bytes: crate::constants::CONFIRM_LARGE_FILE_BYTES,
            shrink_block_percent: crate::constants::SHRINK_BLOCK_PERCENT,
            auto_rotate_max_age_secs: crate::constants::AUTO_ROTATE_MAX_AGE_SECS,
            content_risk_patterns: Vec::new(),
        }
    }
}

/// Validate `[write]` section ranges (fail closed on bad XDG config).
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when `confirm_large_bytes` or `auto_rotate_max_age_secs` is 0, or when `shrink_block_percent` falls outside `SHRINK_BLOCK_PERCENT_MIN..=SHRINK_BLOCK_PERCENT_MAX`.
pub fn validate_write(cfg: &WriteSection) -> Result<(), AtomwriteError> {
    if cfg.confirm_large_bytes == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "config [write] confirm_large_bytes must be >= 1".into(),
        });
    }
    if !(crate::constants::SHRINK_BLOCK_PERCENT_MIN..=crate::constants::SHRINK_BLOCK_PERCENT_MAX)
        .contains(&cfg.shrink_block_percent)
    {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "config [write] shrink_block_percent = {} is out of range; must be 1..=99",
                cfg.shrink_block_percent
            ),
        });
    }
    if cfg.auto_rotate_max_age_secs == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "config [write] auto_rotate_max_age_secs must be >= 1".into(),
        });
    }
    Ok(())
}

/// Default settings applied to write/edit/replace operations.
#[derive(Debug, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DefaultsSection {
    /// Enable backup by default.
    pub backup: bool,
    /// Number of backups to retain.
    pub retention: u8,
    /// Line ending mode: auto, lf, crlf, cr.
    pub line_ending: String,
    /// Maximum file size in bytes.
    pub max_filesize: u64,
}

impl Default for DefaultsSection {
    fn default() -> Self {
        Self {
            backup: true,
            retention: crate::constants::DEFAULT_BACKUP_RETENTION,
            line_ending: "auto".into(),
            max_filesize: crate::constants::DEFAULT_MAX_FILESIZE,
        }
    }
}

/// Fuzzy matching configuration for edit operations.
#[derive(Debug, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FuzzySection {
    /// Default fuzzy mode: `auto`, `aggressive`, or `off` (exact-only, G-010).
    pub mode: String,
    /// Default similarity threshold (0.0–1.0) for Auto block/dual-gate.
    pub threshold: f64,
    /// Aggressive-mode floor (G-FZZ-150 XDG matrix).
    #[serde(default = "default_fuzzy_aggressive")]
    pub threshold_aggressive: f64,
    /// Context-aware Damerau threshold.
    #[serde(default = "default_fuzzy_context")]
    pub threshold_context: f64,
    /// Jaro-Winkler line threshold.
    #[serde(default = "default_fuzzy_jw")]
    pub threshold_jw: f64,
    /// Context-aware soft-floor delta below threshold (GAP-2026-022; XDG `[fuzzy]`).
    #[serde(default = "default_fuzzy_soft_floor")]
    pub soft_floor_delta: f64,
    /// Max content bytes the fuzzy cascade scans; larger files fail instead of running it.
    #[serde(default = "default_fuzzy_max_content_bytes")]
    pub max_content_bytes: u64,
}

fn default_fuzzy_aggressive() -> f64 {
    crate::constants::FUZZY_THRESHOLD_AGGRESSIVE
}
fn default_fuzzy_context() -> f64 {
    crate::constants::FUZZY_THRESHOLD_CONTEXT
}
fn default_fuzzy_jw() -> f64 {
    crate::constants::FUZZY_THRESHOLD_JW
}
fn default_fuzzy_soft_floor() -> f64 {
    crate::constants::FUZZY_CONTEXT_SOFT_FLOOR_DELTA
}
fn default_fuzzy_max_content_bytes() -> u64 {
    crate::constants::FUZZY_MAX_CONTENT_BYTES
}

impl Default for FuzzySection {
    fn default() -> Self {
        Self {
            mode: "auto".into(),
            threshold: crate::constants::FUZZY_THRESHOLD_AUTO,
            threshold_aggressive: crate::constants::FUZZY_THRESHOLD_AGGRESSIVE,
            threshold_context: crate::constants::FUZZY_THRESHOLD_CONTEXT,
            threshold_jw: crate::constants::FUZZY_THRESHOLD_JW,
            soft_floor_delta: crate::constants::FUZZY_CONTEXT_SOFT_FLOOR_DELTA,
            max_content_bytes: crate::constants::FUZZY_MAX_CONTENT_BYTES,
        }
    }
}

/// Storage configuration (XDG; no process env knobs).
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageSection {
    /// Optional home override path (absolute). When set, config/data/cache/state
    /// are rooted under `{home}/…` instead of `ProjectDirs`.
    pub home: Option<String>,
}

/// Search defaults (A-019: policy overridable via XDG / workspace config).
#[derive(Debug, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchSection {
    /// Default context lines around matches.
    pub context: u32,
    /// Enable smart-case by default.
    pub smart_case: bool,
    /// Default `search --max-filesize` (bytes).
    pub max_filesize: u64,
    /// Default `search --max-columns`.
    pub max_columns: usize,
    /// Total bytes of search events in flight between walker workers and stdout.
    pub max_inflight_bytes: u64,
}

impl Default for SearchSection {
    fn default() -> Self {
        Self {
            context: 0,
            smart_case: true,
            max_filesize: crate::constants::DEFAULT_SEARCH_MAX_FILESIZE_BYTES,
            max_columns: crate::constants::DEFAULT_SEARCH_MAX_COLUMNS,
            max_inflight_bytes: crate::constants::SEARCH_MAX_INFLIGHT_BYTES,
        }
    }
}

/// Terminal summary bounds shared by every verb that reports failures.
#[derive(Debug, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SummarySection {
    /// Most `errors[]` entries a terminal summary lists.
    ///
    /// `files_failed` keeps the full count, so a small cap costs detail and
    /// never the size of the hole. The ceiling keeps one record bounded.
    pub errors_max_entries: usize,
}

impl Default for SummarySection {
    fn default() -> Self {
        Self {
            errors_max_entries: crate::constants::SUMMARY_ERRORS_MAX_ENTRIES,
        }
    }
}

/// Validate `[search]` section (A-019).
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when `max_filesize` or `max_columns` is 0, or when `max_inflight_bytes` falls outside `SEARCH_MIN_INFLIGHT_BYTES..=SEARCH_MAX_INFLIGHT_CEILING_BYTES`.
pub fn validate_search_section(cfg: &SearchSection) -> Result<(), AtomwriteError> {
    if cfg.max_filesize == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "config [search] max_filesize must be >= 1".into(),
        });
    }
    if cfg.max_columns == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "config [search] max_columns must be >= 1".into(),
        });
    }
    require_in_range(
        "search",
        "max_inflight_bytes",
        cfg.max_inflight_bytes,
        crate::constants::SEARCH_MIN_INFLIGHT_BYTES,
        crate::constants::SEARCH_MAX_INFLIGHT_CEILING_BYTES,
    )?;
    Ok(())
}

/// Validate `[summary]` section.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when `errors_max_entries` falls
/// outside `SUMMARY_ERRORS_MIN_ENTRIES..=SUMMARY_ERRORS_CEILING`.
pub fn validate_summary_section(cfg: &SummarySection) -> Result<(), AtomwriteError> {
    require_in_range(
        "summary",
        "errors_max_entries",
        cfg.errors_max_entries as u64,
        crate::constants::SUMMARY_ERRORS_MIN_ENTRIES as u64,
        crate::constants::SUMMARY_ERRORS_CEILING as u64,
    )?;
    Ok(())
}

/// Load configuration from the hierarchy:
/// 1. Explicit `--config` path (alone: it replaces the two files below)
/// 2. `{workspace}/.atomwrite.toml` (workspace layer, wins key by key)
/// 3. Global config via [`crate::storage::global_config_path`]
///    (`set storage.home` / XDG `ProjectDirs` config path)
/// 4. Built-in defaults
///
/// The workspace and global files are merged key by key, so a workspace file
/// that sets one key keeps every other key of the global file.
///
/// Every file that is found must be readable, valid TOML, free of unknown keys
/// and not newer than [`crate::constants::CONFIG_SCHEMA_VERSION`]; otherwise
/// the load fails with [`AtomwriteError::ConfigInvalid`] (exit 78). A broken
/// config is never swapped for defaults in silence.
///
/// # Errors
///
/// Returns `AtomwriteError::ConfigInvalid` when the explicit path, the global XDG file or the workspace `.atomwrite.toml` cannot be read, is not valid TOML, carries an unknown key, declares a version above `CONFIG_SCHEMA_VERSION`, or yields a merged table that no longer type-checks.
pub fn load_config(
    workspace: &Path,
    explicit_path: Option<&Path>,
) -> Result<AtomwriteConfig, AtomwriteError> {
    if let Some(path) = explicit_path {
        return parse_config(path);
    }

    let global = match global_text() {
        Some(g) => match &g.text {
            Ok(text) => Some(parse_layer(&g.path, text)?),
            Err(e) => {
                return Err(AtomwriteError::ConfigInvalid {
                    reason: format!("cannot read config {}: {e}", g.path.display()),
                });
            }
        },
        None => None,
    };
    let local_path = workspace.join(crate::constants::WORKSPACE_CONFIG_FILE);
    let local = if local_path.is_file() {
        Some(read_layer(&local_path)?)
    } else {
        None
    };
    merge_layers(global, local)
}

/// Merge the two layers of the hierarchy into one config.
///
/// Both layers arrive already materialized, so this function touches no
/// filesystem and reads no global state: unit tests drive the exact merge the
/// binary runs, without inheriting the operator's config directory.
fn merge_layers(
    global: Option<Layer>,
    local: Option<Layer>,
) -> Result<AtomwriteConfig, AtomwriteError> {
    match (global, local) {
        (None, None) => Ok(AtomwriteConfig::default()),
        (Some(only), None) | (None, Some(only)) => Ok(only.config),
        (Some(global), Some(local)) => {
            let merged = merge_tables(global.table, local.table);
            toml::Value::Table(merged)
                .try_into()
                .map_err(|e| AtomwriteError::ConfigInvalid {
                    reason: format!(
                        "merged config of {} and {} is invalid: {e}",
                        crate::constants::GLOBAL_CONFIG_FILE,
                        crate::constants::WORKSPACE_CONFIG_FILE
                    ),
                })
        }
    }
}

/// Read and parse one config file; every failure names the file in the reason.
fn parse_config(path: &Path) -> Result<AtomwriteConfig, AtomwriteError> {
    read_layer(path).map(|layer| layer.config)
}

/// Reject illegal `[fuzzy]` values (v0.1.35: off = exact-only).
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when `[fuzzy]` mode is an unsupported value (including the retired `disabled`, `false` and `0` aliases), when `threshold` falls outside 0.0..=1.0, or when `max_content_bytes` is 0.
pub fn validate_fuzzy(cfg: &FuzzySection) -> Result<(), AtomwriteError> {
    let mode = cfg.mode.trim().to_ascii_lowercase();
    match mode.as_str() {
        "auto" | "aggressive" | "off" | "exact" => {}
        "disabled" | "false" | "0" => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "config [fuzzy] mode = \"{}\" is invalid; use \"auto\", \"aggressive\", or \"off\" (exact-only)",
                    cfg.mode
                ),
            });
        }
        other => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "config [fuzzy] mode = \"{other}\" is invalid; use \"auto\", \"aggressive\", or \"off\""
                ),
            });
        }
    }
    if !(crate::constants::SIMILARITY_MIN..=crate::constants::SIMILARITY_MAX)
        .contains(&cfg.threshold)
    {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "config [fuzzy] threshold = {} is out of range; must be between 0.0 and 1.0",
                cfg.threshold
            ),
        });
    }
    if cfg.max_content_bytes == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "config [fuzzy] max_content_bytes = 0 is invalid; must be greater than 0"
                .into(),
        });
    }
    Ok(())
}

/// Resolve effective fuzzy mode/threshold.
///
/// Precedence: explicit CLI `Aggressive` or `cli_threshold` wins; when CLI is
/// default `Auto` with no threshold, inherit from `.atomwrite.toml` `[fuzzy]`.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when `[fuzzy]` violates any rule checked by [`validate_fuzzy`]: an unknown mode, a threshold outside 0.0..=1.0, or `max_content_bytes` equal to 0.
pub fn resolve_fuzzy(
    cli_mode: FuzzyMode,
    cli_threshold: Option<f64>,
    cfg: &FuzzySection,
) -> Result<(FuzzyMode, Option<f64>), AtomwriteError> {
    validate_fuzzy(cfg)?;
    // G-010: Off is exact-only (no cascade) — honest API for agents.
    let cfg_mode = match cfg.mode.trim().to_ascii_lowercase().as_str() {
        "aggressive" => FuzzyMode::Aggressive,
        "off" => FuzzyMode::Off,
        _ => FuzzyMode::Auto,
    };
    let mode = match cli_mode {
        FuzzyMode::Aggressive => FuzzyMode::Aggressive,
        FuzzyMode::Off => FuzzyMode::Off,
        FuzzyMode::Auto => cfg_mode,
    };
    let threshold = match cli_threshold {
        Some(t) => Some(t),
        None if (cfg.threshold - crate::constants::FUZZY_THRESHOLD_AUTO).abs() > f64::EPSILON => {
            Some(cfg.threshold)
        }
        None => None,
    };
    Ok((mode, threshold))
}

include!("config_tuning.inc.rs");
include!("config_tests.inc.rs");
