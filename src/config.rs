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
#[serde(default)]
pub struct AtomwriteConfig {
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
}

/// Watch policy loaded from XDG / `.atomwrite.toml` `[watch]` (R-XDG-007).
#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct WatchSection {
    /// Exit after this many idle milliseconds with zero filesystem events (0 = disable idle-exit).
    pub idle_exit_ms: u64,
}

impl Default for WatchSection {
    fn default() -> Self {
        Self {
            idle_exit_ms: crate::constants::DEFAULT_WATCH_IDLE_EXIT_MS,
        }
    }
}

/// Write-guard policy loaded from XDG / `.atomwrite.toml` `[write]` (G-028).
#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct WriteSection {
    /// Bytes above which `--confirm` requires `--ack-overwrite`.
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
pub fn validate_write(cfg: &WriteSection) -> Result<(), AtomwriteError> {
    if cfg.confirm_large_bytes == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "config [write] confirm_large_bytes must be >= 1".into(),
        });
    }
    if !(1..=99).contains(&cfg.shrink_block_percent) {
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
#[serde(default)]
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
#[serde(default)]
pub struct FuzzySection {
    /// Default fuzzy mode: auto or aggressive (off rejected since v0.1.30).
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

impl Default for FuzzySection {
    fn default() -> Self {
        Self {
            mode: "auto".into(),
            threshold: crate::constants::FUZZY_THRESHOLD_AUTO,
            threshold_aggressive: crate::constants::FUZZY_THRESHOLD_AGGRESSIVE,
            threshold_context: crate::constants::FUZZY_THRESHOLD_CONTEXT,
            threshold_jw: crate::constants::FUZZY_THRESHOLD_JW,
        }
    }
}


/// Storage configuration (XDG; no process env knobs).
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct StorageSection {
    /// Optional home override path (absolute). When set, config/data/cache/state
    /// are rooted under `{home}/…` instead of `ProjectDirs`.
    pub home: Option<String>,
}

/// Search defaults (A-019: policy overridable via XDG / workspace config).
#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct SearchSection {
    /// Default context lines around matches.
    pub context: u32,
    /// Enable smart-case by default.
    pub smart_case: bool,
    /// Default `search --max-filesize` (bytes).
    pub max_filesize: u64,
    /// Default `search --max-columns`.
    pub max_columns: usize,
}

impl Default for SearchSection {
    fn default() -> Self {
        Self {
            context: 0,
            smart_case: true,
            max_filesize: crate::constants::DEFAULT_SEARCH_MAX_FILESIZE_BYTES,
            max_columns: crate::constants::DEFAULT_SEARCH_MAX_COLUMNS,
        }
    }
}

/// Validate `[search]` section (A-019).
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
    Ok(())
}

/// Load configuration from the hierarchy:
/// 1. Explicit `--config` path (strict — missing/malformed is hard error)
/// 2. `{workspace}/.atomwrite.toml` (local; soft-fail to defaults)
/// 3. Global config via [`crate::storage::global_config_path`]
///    (`set storage.home` / XDG `ProjectDirs` config path; soft-fail)
/// 4. Built-in defaults
///
/// Auto-discovered configs that fail to parse emit `tracing::warn!` and fall
/// through to defaults. An explicit `--config` path **must** exist and parse.
pub fn load_config(
    workspace: &Path,
    explicit_path: Option<&Path>,
) -> Result<AtomwriteConfig, AtomwriteError> {
    if let Some(path) = explicit_path {
        return load_from_path_strict(path);
    }

    let local = workspace.join(".atomwrite.toml");
    if local.is_file() {
        return Ok(load_from_path_soft(&local));
    }

    if let Some(global) = crate::storage::global_config_path() {
        if global.is_file() {
            return Ok(load_from_path_soft(&global));
        }
    }

    Ok(AtomwriteConfig::default())
}

/// Strict load for `--config`: missing or malformed → [`AtomwriteError::ConfigInvalid`].
fn load_from_path_strict(path: &Path) -> Result<AtomwriteConfig, AtomwriteError> {
    let content = std::fs::read_to_string(path).map_err(|e| AtomwriteError::ConfigInvalid {
        reason: format!("cannot read config {}: {e}", path.display()),
    })?;
    let config: AtomwriteConfig =
        toml::from_str(&content).map_err(|e| AtomwriteError::ConfigInvalid {
            reason: format!("malformed config {}: {e}", path.display()),
        })?;
    tracing::debug!(path = %path.display(), "loaded explicit config");
    Ok(config)
}

/// Soft load for auto-discovered configs: warn and use defaults on error.
fn load_from_path_soft(path: &Path) -> AtomwriteConfig {
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => {
                tracing::debug!(path = %path.display(), "loaded config");
                config
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "malformed config; using defaults"
                );
                AtomwriteConfig::default()
            }
        },
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "cannot read config; using defaults"
            );
            AtomwriteConfig::default()
        }
    }
}

/// Reject illegal `[fuzzy]` values (v0.1.35: off = exact-only).
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
    if !(0.0..=1.0).contains(&cfg.threshold) {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "config [fuzzy] threshold = {} is out of range; must be between 0.0 and 1.0",
                cfg.threshold
            ),
        });
    }
    Ok(())
}

/// Resolve effective fuzzy mode/threshold.
///
/// Precedence: explicit CLI `Aggressive` or `cli_threshold` wins; when CLI is
/// default `Auto` with no threshold, inherit from `.atomwrite.toml` `[fuzzy]`.
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
        None if (cfg.threshold - crate::constants::FUZZY_THRESHOLD_AUTO).abs() > f64::EPSILON => Some(cfg.threshold),
        None => None,
    };
    Ok((mode, threshold))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_returns_defaults() {
        let dir = tempdir().unwrap();
        let config = load_config(dir.path(), None).unwrap();
        assert!(config.defaults.backup);
        assert_eq!(config.defaults.retention, 5);
        assert_eq!(config.fuzzy.mode, "auto");
    }

    #[test]
    fn local_config_overrides_defaults() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(".atomwrite.toml"),
            "[defaults]\nbackup = false\nretention = 3\n",
        )
        .unwrap();
        let config = load_config(dir.path(), None).unwrap();
        assert!(!config.defaults.backup);
        assert_eq!(config.defaults.retention, 3);
    }

    #[test]
    fn malformed_toml_warns_uses_defaults() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(".atomwrite.toml"), "not valid { toml").unwrap();
        let config = load_config(dir.path(), None).unwrap();
        assert!(config.defaults.backup);
    }

    #[test]
    fn explicit_path_takes_precedence() {
        let dir = tempdir().unwrap();
        let custom = dir.path().join("custom.toml");
        std::fs::write(&custom, "[fuzzy]\nthreshold = 0.95\n").unwrap();
        let config = load_config(dir.path(), Some(&custom)).unwrap();
        assert!((config.fuzzy.threshold - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn explicit_path_missing_is_hard_error() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope.toml");
        let err = load_config(dir.path(), Some(&missing)).unwrap_err();
        assert!(
            matches!(err, AtomwriteError::ConfigInvalid { .. }),
            "expected ConfigInvalid, got {err:?}"
        );
    }
}
