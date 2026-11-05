// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by locale.rs (responsibility split, v0.1.37 G2).
//
// Persisting the UI locale preference: where the file lives, how it is
// read back, how it is written and how it is cleared. Kept apart from
// detection and resolution so the part that touches the filesystem is
// separate from the part that decides a language.

/// XDG config path for the persisted locale preference.
pub fn preference_path() -> Option<PathBuf> {
    crate::storage::config_dir().map(|d| d.join(crate::constants::LOCALE_PREFERENCE_FILE))
}

/// Read a validated persisted preference (`en` or `pt-BR`), if any.
pub fn read_persisted_preference(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let tag = text.lines().next()?.trim();
    if tag.is_empty() {
        return None;
    }
    let language = negotiate_to_language(&[tag]);
    // Only accept if the tag actually maps to a supported locale cleanly.
    Language::from_tag(tag)
        .or({
            // Allow `pt` / `pt_BR.UTF-8` style values in the file via negotiation.
            Some(language)
        })
        .map(|i| i.as_str().to_string())
}

/// Persist a UI locale preference (owner-only file when possible).
///
/// The returned tuple carries the path and the verdict of the write, so a
/// caller that publishes `write_strategy` never has to guess the mechanism.
///
/// # Errors
///
/// Returns `std::io::ErrorKind::NotFound` when the XDG config directory cannot be resolved or the preference path has no parent directory.
/// Returns the I/O error of `ensure_dir`, `atomic_write` or `restrict_to_owner` when creating the directory, persisting the file or tightening its permissions fails.
pub fn write_persisted_preference(
    language: Language,
) -> std::io::Result<(PathBuf, crate::atomic::WriteResult)> {
    let path = preference_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "cannot resolve XDG config directory for atomwrite",
        )
    })?;
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "locale preference path has no parent directory",
        )
    })?;
    crate::storage::ensure_dir(parent)?;
    // Tempfile, fsync and rename: a crash never leaves a half-written preference.
    let opts = crate::atomic::AtomicWriteOptions {
        backup: false,
        wal_policy: crate::wal::WalPolicy::Never,
        ..crate::atomic::AtomicWriteOptions::default()
    };
    let content = format!("{}\n", language.as_str());
    let result = crate::atomic::atomic_write(&path, content.as_bytes(), &opts, parent)
        .map_err(std::io::Error::other)?;
    crate::storage::restrict_to_owner(&path)?;
    Ok((path, result))
}

/// Remove the persisted preference file (no error if missing).
///
/// # Errors
///
/// Returns the `std::io::Error` of `fs::remove_file` when the removal fails for a reason other than a missing file.
pub fn clear_persisted_preference() -> std::io::Result<Option<PathBuf>> {
    let Some(path) = preference_path() else {
        return Ok(None);
    };
    match fs::remove_file(&path) {
        Ok(()) => Ok(Some(path)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Some(path)),
        Err(e) => Err(e),
    }
}
