// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration file loading for `.atomwrite.toml`.
//!
//! Hierarchy (highest wins): CLI flags > env vars > local config > global config > defaults.

use std::path::Path;

use crate::cli_args::FuzzyMode;
use crate::error::AtomwriteError;

/// Top-level configuration structure matching `.atomwrite.toml`.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct AtomwriteConfig {
    /// Default settings for write operations.
    pub defaults: DefaultsSection,
    /// Fuzzy matching defaults for edit operations.
    pub fuzzy: FuzzySection,
    /// Search defaults.
    pub search: SearchSection,
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
            max_filesize: 1_073_741_824,
        }
    }
}

/// Fuzzy matching configuration for edit operations.
#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct FuzzySection {
    /// Default fuzzy mode: auto or aggressive (off rejected since v0.1.30).
    pub mode: String,
    /// Default similarity threshold (0.0–1.0).
    pub threshold: f64,
}

impl Default for FuzzySection {
    fn default() -> Self {
        Self {
            mode: "auto".into(),
            threshold: 0.70,
        }
    }
}

/// Search defaults.
#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct SearchSection {
    /// Default context lines around matches.
    pub context: u32,
    /// Enable smart-case by default.
    pub smart_case: bool,
}

impl Default for SearchSection {
    fn default() -> Self {
        Self {
            context: 0,
            smart_case: true,
        }
    }
}

/// Load configuration from the hierarchy:
/// 1. Explicit `--config` path (strict — missing/malformed is hard error)
/// 2. `{workspace}/.atomwrite.toml` (local; soft-fail to defaults)
/// 3. Global config via [`crate::storage::global_config_path`]
///    (`ATOMWRITE_HOME/config/config.toml` or XDG/`ProjectDirs`; soft-fail)
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

/// Reject illegal `[fuzzy]` values (v0.1.30 product policy).
pub fn validate_fuzzy(cfg: &FuzzySection) -> Result<(), AtomwriteError> {
    let mode = cfg.mode.trim().to_ascii_lowercase();
    match mode.as_str() {
        "auto" | "aggressive" => {}
        "off" | "exact" | "disabled" | "false" | "0" => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "config [fuzzy] mode = \"{}\" is not allowed since v0.1.30; use \"auto\" or \"aggressive\"",
                    cfg.mode
                ),
            });
        }
        other => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "config [fuzzy] mode = \"{other}\" is invalid; use \"auto\" or \"aggressive\""
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
    if matches!(cli_mode, FuzzyMode::Off) {
        return Err(AtomwriteError::InvalidInput {
            reason: "fuzzy mode 'off' was removed in v0.1.30; use auto (default) or aggressive"
                .into(),
        });
    }
    let cfg_mode = match cfg.mode.trim().to_ascii_lowercase().as_str() {
        "aggressive" => FuzzyMode::Aggressive,
        _ => FuzzyMode::Auto,
    };
    let mode = match cli_mode {
        FuzzyMode::Aggressive => FuzzyMode::Aggressive,
        FuzzyMode::Auto => cfg_mode,
        FuzzyMode::Off => FuzzyMode::Auto, // unreachable after check
    };
    let threshold = match cli_threshold {
        Some(t) => Some(t),
        None if (cfg.threshold - 0.70).abs() > f64::EPSILON => Some(cfg.threshold),
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
