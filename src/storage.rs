// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cross-platform storage paths (XDG / ProjectDirs / `ATOMWRITE_HOME`).
//!
//! Rules Rust multiplataforma: config, cache, data and state live under
//! platform conventions (`directories::ProjectDirs`) unless the operator
//! overrides the base with `ATOMWRITE_HOME`. Never hardcode `/home/`,
//! `C:\Users\`, or `/tmp` for durable state.

use std::path::{Path, PathBuf};

/// Environment variable that overrides the atomwrite home base directory.
///
/// When set to a non-empty path, config / data / cache / state resolve under:
/// - `{ATOMWRITE_HOME}/config`
/// - `{ATOMWRITE_HOME}/data`
/// - `{ATOMWRITE_HOME}/cache`
/// - `{ATOMWRITE_HOME}/state`
///
/// Empty or unset falls back to [`directories::ProjectDirs`].
pub const HOME_ENV: &str = "ATOMWRITE_HOME";

/// Resolve the optional `ATOMWRITE_HOME` override (non-empty only).
pub fn home_override() -> Option<PathBuf> {
    home_override_from(std::env::var_os(HOME_ENV))
}

/// Pure form of [`home_override`] for tests.
pub fn home_override_from(raw: Option<std::ffi::OsString>) -> Option<PathBuf> {
    raw.filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Config directory (`…/atomwrite` under XDG/AppData, or `{ATOMWRITE_HOME}/config`).
pub fn config_dir() -> Option<PathBuf> {
    config_dir_from(home_override())
}

/// Pure config dir resolution given an optional home override.
pub fn config_dir_from(home: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(home) = home {
        return Some(home.join("config"));
    }
    directories::ProjectDirs::from("", "", "atomwrite").map(|p| p.config_dir().to_path_buf())
}

/// Data directory for durable application data.
pub fn data_dir() -> Option<PathBuf> {
    data_dir_from(home_override())
}

/// Pure data dir resolution given an optional home override.
pub fn data_dir_from(home: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(home) = home {
        return Some(home.join("data"));
    }
    directories::ProjectDirs::from("", "", "atomwrite").map(|p| p.data_dir().to_path_buf())
}

/// Cache directory for disposable artifacts.
pub fn cache_dir() -> Option<PathBuf> {
    cache_dir_from(home_override())
}

/// Pure cache dir resolution given an optional home override.
pub fn cache_dir_from(home: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(home) = home {
        return Some(home.join("cache"));
    }
    directories::ProjectDirs::from("", "", "atomwrite").map(|p| p.cache_dir().to_path_buf())
}

/// State directory when available (Linux XDG state); falls back to data dir.
pub fn state_dir() -> Option<PathBuf> {
    state_dir_from(home_override())
}

/// Pure state dir resolution given an optional home override.
pub fn state_dir_from(home: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(home) = home {
        return Some(home.join("state"));
    }
    directories::ProjectDirs::from("", "", "atomwrite").map(|p| {
        p.state_dir()
            .map(|s| s.to_path_buf())
            .unwrap_or_else(|| p.data_dir().to_path_buf())
    })
}

/// Path to the global config file (`config.toml` under [`config_dir`]).
pub fn global_config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

/// Ensure a storage directory exists with owner-only permissions on Unix.
///
/// # Errors
///
/// Returns I/O errors from `create_dir_all` / `set_permissions`.
pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn empty_home_override_ignored() {
        assert!(home_override_from(None).is_none());
        assert!(home_override_from(Some(OsString::from(""))).is_none());
    }

    #[test]
    fn home_override_redirects_all_dirs() {
        let base = PathBuf::from("/tmp/aw-home-test");
        let home = Some(base.clone());
        assert_eq!(
            config_dir_from(home.clone()).as_deref(),
            Some(base.join("config").as_path())
        );
        assert_eq!(
            data_dir_from(home.clone()).as_deref(),
            Some(base.join("data").as_path())
        );
        assert_eq!(
            cache_dir_from(home.clone()).as_deref(),
            Some(base.join("cache").as_path())
        );
        assert_eq!(
            state_dir_from(home).as_deref(),
            Some(base.join("state").as_path())
        );
    }

    #[test]
    fn project_dirs_fallback_does_not_panic() {
        // Without override: may be Some or None depending on host HOME.
        let _ = config_dir_from(None);
        let _ = data_dir_from(None);
        let _ = cache_dir_from(None);
        let _ = state_dir_from(None);
    }
}
