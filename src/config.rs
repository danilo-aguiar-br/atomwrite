// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration file loading for `.atomwrite.toml`.
//!
//! Hierarchy (highest wins): CLI flags > env vars > local config > global config > defaults.

use std::path::Path;

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
    pub retention: u32,
    /// Line ending mode: auto, lf, crlf, cr.
    pub line_ending: String,
    /// Maximum file size in bytes.
    pub max_filesize: u64,
}

impl Default for DefaultsSection {
    fn default() -> Self {
        Self {
            backup: true,
            retention: 5,
            line_ending: "auto".into(),
            max_filesize: 1_073_741_824,
        }
    }
}

/// Fuzzy matching configuration for edit operations.
#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct FuzzySection {
    /// Default fuzzy mode: auto, off, aggressive.
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
/// 1. `{workspace}/.atomwrite.toml` (local)
/// 2. `~/.config/atomwrite/config.toml` (XDG global)
/// 3. Defaults
///
/// Parse errors emit a `tracing::warn!` and fall through to defaults.
pub fn load_config(workspace: &Path, explicit_path: Option<&Path>) -> AtomwriteConfig {
    if let Some(path) = explicit_path {
        return load_from_path(path);
    }

    let local = workspace.join(".atomwrite.toml");
    if local.is_file() {
        return load_from_path(&local);
    }

    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "atomwrite") {
        let global = proj_dirs.config_dir().join("config.toml");
        if global.is_file() {
            return load_from_path(&global);
        }
    }

    AtomwriteConfig::default()
}

fn load_from_path(path: &Path) -> AtomwriteConfig {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_returns_defaults() {
        let dir = tempdir().unwrap();
        let config = load_config(dir.path(), None);
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
        let config = load_config(dir.path(), None);
        assert!(!config.defaults.backup);
        assert_eq!(config.defaults.retention, 3);
    }

    #[test]
    fn malformed_toml_warns_uses_defaults() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(".atomwrite.toml"), "not valid { toml").unwrap();
        let config = load_config(dir.path(), None);
        assert!(config.defaults.backup);
    }

    #[test]
    fn explicit_path_takes_precedence() {
        let dir = tempdir().unwrap();
        let custom = dir.path().join("custom.toml");
        std::fs::write(&custom, "[fuzzy]\nthreshold = 0.95\n").unwrap();
        let config = load_config(dir.path(), Some(&custom));
        assert!((config.fuzzy.threshold - 0.95).abs() < f64::EPSILON);
    }
}
