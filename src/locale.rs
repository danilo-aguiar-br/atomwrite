// SPDX-License-Identifier: MIT OR Apache-2.0

//! Locale detection, BCP 47 negotiation, and resolved UI language state.
//!
//! Startup order (Rules Rust i18n):
//! 1. Console UTF-8 (`platform::init_console`, before this module)
//! 2. CLI override (`--locale` flag; XDG preference via locale command)
//! 3. Persisted XDG preference (`directories::ProjectDirs`)
//! 4. OS locale via `sys-locale` (never raw `LANG` in portable code)
//! 5. Deterministic fallback [`crate::locale::Language::En`]
//!
//! Agent contract: NDJSON **codes** and **Display** error messages stay English.
//! Human-oriented **suggestions** (and future stderr copy) follow the resolved
//! locale via `rust-i18n` (`t!`).

use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::Mutex;
use std::sync::OnceLock;

use fluent_langneg::{NegotiationStrategy, negotiate_languages};
use unic_langid::LanguageIdentifier;

/// Text direction for the resolved language script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextDirection {
    /// Left-to-right (Latin, CJK, etc.).
    Ltr,
    /// Right-to-left (Arabic, Hebrew) — not shipped in the default MVP binary.
    Rtl,
}

/// Source that won the 5-layer locale precedence chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LocaleSource {
    /// Explicit `--locale` CLI flag (highest priority).
    CliFlag,
    /// User preference under XDG config (`locale` file).
    Persisted,
    /// `sys-locale` OS detection.
    System,
    /// Built-in English fallback.
    Default,
}

impl LocaleSource {
    /// Stable NDJSON / diagnostic label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CliFlag => "cli",
            Self::Persisted => "persisted",
            Self::System => "system",
            Self::Default => "default",
        }
    }
}

/// Supported UI locales compiled into the default binary (MVP: `en` + `pt-BR`).
///
/// Optional top-20 languages stay behind Cargo features (`i18n-full`, …) and
/// are **not** embedded until translations are complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Language {
    /// Neutral English (`en`) — technical logs and NDJSON machine fields.
    En,
    /// Brazilian Portuguese (`pt-BR`) — human suggestions / stderr when selected.
    PtBr,
}

impl Language {
    /// Locales available in the default (non-feature) build.
    pub const AVAILABLE: &'static [Language] = &[Language::En, Language::PtBr];

    /// BCP 47 tag used by `rust-i18n` and locale files.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::PtBr => "pt-BR",
        }
    }

    /// Ultimate fallback for any regional variant (always English in MVP).
    pub const fn fallback(self) -> Language {
        match self {
            Self::En => Self::En,
            Self::PtBr => Self::En,
        }
    }

    /// Script direction (MVP locales are LTR only).
    pub const fn direction(self) -> TextDirection {
        TextDirection::Ltr
    }

    /// Parse a canonical tag (`en`, `pt-BR`) into [`Language`].
    pub fn from_tag(tag: &str) -> Option<Language> {
        match tag {
            "en" | "en-US" | "en-GB" | "en-AU" | "en-CA" | "en-IE" | "en-IN" | "en-IL" => {
                Some(Self::En)
            }
            "pt-BR" | "pt-br" => Some(Self::PtBr),
            other => {
                // Accept underscore form after normalization.
                let n = normalize_bcp47_tag(other);
                match n.as_str() {
                    "en" | "en-US" | "en-GB" => Some(Self::En),
                    "pt-BR" => Some(Self::PtBr),
                    _ => None,
                }
            }
        }
    }

    /// Map a structured BCP 47 identifier onto a supported [`Language`].
    pub fn from_langid(id: &LanguageIdentifier) -> Option<Language> {
        let tag = id.to_string();
        Self::from_tag(&tag).or_else(|| {
            // Language-only: `pt` negotiates to `pt-BR` only via fluent-langneg;
            // here we map exact known bases for static conversion helpers.
            let lang = id.language.as_str();
            match lang {
                "en" => Some(Self::En),
                "pt" => Some(Self::PtBr),
                _ => None,
            }
        })
    }

    /// `unic-langid` identifier for this language.
    ///
    /// # Errors
    ///
    /// Returns `AtomwriteError::InternalError` if the built-in tag is not valid
    /// BCP 47 (a programming error, never user input).
    pub fn language_identifier(self) -> Result<LanguageIdentifier, crate::error::AtomwriteError> {
        self.as_str()
            .parse()
            .map_err(|e| crate::error::AtomwriteError::InternalError {
                reason: format!("built-in locale tag {:?} is not BCP 47: {e}", self.as_str()),
            })
    }
}

/// Immutable snapshot published after one-shot locale resolution.
#[derive(Debug, Clone)]
pub struct LocaleState {
    /// Negotiated UI language.
    pub language: Language,
    /// Which precedence layer selected `language`.
    pub source: LocaleSource,
    /// Raw string from `sys-locale` (if any), before normalization.
    pub system_raw: Option<String>,
    /// True when OS detection returned `None` or an unparsable tag.
    pub detection_failed: bool,
    /// Path of the XDG preference file (may not exist yet).
    pub preference_path: Option<PathBuf>,
    /// Contents of the preference file when present and valid.
    pub persisted_tag: Option<String>,
}

static RESOLVED: OnceLock<LocaleState> = OnceLock::new();

/// Normalize OS / user tags: `_` → `-`, strip encoding (`.UTF-8`), drop `@variant` modifiers.
///
/// Does **not** invent English for `C` / `POSIX` / `C.UTF-8` — those yield empty language
/// after strip and must fall through to the default language.
pub fn normalize_bcp47_tag(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // Drop encoding / modifier: `pt_BR.UTF-8@euro` → `pt_BR`
    let base = trimmed.split(['.', '@']).next().unwrap_or(trimmed).trim();
    // POSIX "C" / "POSIX" are not user language preferences.
    if base.eq_ignore_ascii_case("C") || base.eq_ignore_ascii_case("POSIX") {
        return String::new();
    }
    base.replace('_', "-")
}

/// Parse a raw locale string into a [`LanguageIdentifier`] via `unic-langid`.
pub fn parse_langid(raw: &str) -> Option<LanguageIdentifier> {
    let tag = normalize_bcp47_tag(raw);
    if tag.is_empty() {
        return None;
    }
    tag.parse().ok()
}

/// Available tags as static strings for clap / diagnostics.
pub fn available_tags() -> &'static [&'static str] {
    &["en", "pt-BR"]
}

/// Negotiate requested BCP 47 tags against the MVP available set.
///
/// Uses `fluent-langneg` [`NegotiationStrategy::Lookup`] so exactly one locale
/// is returned (with English default).
pub fn negotiate_to_language(requested_tags: &[&str]) -> Language {
    let requested: Vec<fluent_langneg::LanguageIdentifier> = requested_tags
        .iter()
        .filter_map(|t| {
            let n = normalize_bcp47_tag(t);
            if n.is_empty() {
                None
            } else {
                // fluent-langneg 0.14 re-exports icu_locid LanguageIdentifier.
                n.parse().ok()
            }
        })
        .collect();

    let available: Vec<fluent_langneg::LanguageIdentifier> = available_tags()
        .iter()
        .filter_map(|t| t.parse().ok())
        .collect();

    // `en` is the built-in default; if even it cannot parse, answer it anyway.
    let Ok(default) = "en".parse::<fluent_langneg::LanguageIdentifier>() else {
        return Language::En;
    };

    if requested.is_empty() {
        return Language::En;
    }

    let supported = negotiate_languages(
        &requested,
        &available,
        Some(&default),
        NegotiationStrategy::Lookup,
    );

    supported
        .first()
        .and_then(|id| {
            let tag = id.to_string();
            Language::from_tag(&tag).or_else(|| {
                // Lookup may return `pt` when only `pt` was requested and `pt-BR` is available
                // depending on strategy — map language subtag.
                match id.language.as_str() {
                    "pt" => Some(Language::PtBr),
                    "en" => Some(Language::En),
                    _ => None,
                }
            })
        })
        .unwrap_or(Language::En)
}

include!("locale_persist.inc.rs");
/// Pure resolution used by [`init_locale`] and unit tests (no global side effects).
pub fn resolve_locale(
    cli_override: Option<&str>,
    system_raw: Option<String>,
    pref_path: Option<PathBuf>,
) -> LocaleState {
    let persisted_tag = pref_path
        .as_ref()
        .and_then(|p| read_persisted_preference(p));

    // Layer 1 — explicit CLI `--locale`.
    if let Some(raw) = cli_override.map(str::trim).filter(|s| !s.is_empty()) {
        let language = negotiate_to_language(&[raw]);
        return LocaleState {
            language,
            source: LocaleSource::CliFlag,
            system_raw,
            detection_failed: false,
            preference_path: pref_path,
            persisted_tag,
        };
    }

    // Layer 3 — persisted preference (layer 2 env is already folded into cli_override by clap).
    if let Some(ref tag) = persisted_tag {
        let language = negotiate_to_language(&[tag.as_str()]);
        return LocaleState {
            language,
            source: LocaleSource::Persisted,
            system_raw,
            detection_failed: false,
            preference_path: pref_path,
            persisted_tag,
        };
    }

    // Layer 4 — OS locale via sys-locale abstraction.
    if let Some(ref raw) = system_raw {
        if let Some(id) = parse_langid(raw) {
            let tag = id.to_string();
            let language = negotiate_to_language(&[tag.as_str()]);
            return LocaleState {
                language,
                source: LocaleSource::System,
                system_raw,
                detection_failed: false,
                preference_path: pref_path,
                persisted_tag: None,
            };
        }
        // Unparsable / C / POSIX → fall through with failure flag.
        return LocaleState {
            language: Language::En,
            source: LocaleSource::Default,
            system_raw,
            detection_failed: true,
            preference_path: pref_path,
            persisted_tag: None,
        };
    }

    // Layer 5 — default English.
    LocaleState {
        language: Language::En,
        source: LocaleSource::Default,
        system_raw: None,
        detection_failed: true,
        preference_path: pref_path,
        persisted_tag: None,
    }
}

/// Detect OS locale once via `sys-locale` (cross-platform; no direct `LANG` reads).
pub fn detect_system_locale() -> Option<String> {
    sys_locale::get_locale()
}

/// Initialize process locale: resolve, publish [`OnceLock`], set `rust-i18n`.
///
/// Safe to call once from `main`. Subsequent calls still update `rust-i18n`
/// (for tests) but do not replace the published [`LocaleState`].
pub fn init_locale(cli_override: Option<&str>) -> Language {
    let system_raw = detect_system_locale();
    let pref = preference_path();
    let state = resolve_locale(cli_override, system_raw, pref);
    let language = state.language;
    rust_i18n::set_locale(language.as_str());
    // The first resolved state wins; a later call (tests, embedding) keeps it.
    if RESOLVED.set(state).is_err() {
        tracing::debug!("locale already resolved; keeping the first state");
    }
    language
}

/// Resolved locale state after [`init_locale`], if initialization ran.
pub fn resolved_state() -> Option<&'static LocaleState> {
    RESOLVED.get()
}

/// Resolved language (defaults to English when init has not run — e.g. unit tests).
pub fn resolved_language() -> Language {
    RESOLVED.get().map(|s| s.language).unwrap_or(Language::En)
}

/// Process-wide lock for locale-mutating unit tests (A-012).
///
/// `rust_i18n::set_locale` is global; parallel tests must serialize.
#[cfg(test)]
static LOCALE_TEST_MUTEX: Mutex<()> = Mutex::new(());

/// RAII guard: sets locale for the duration of a test and restores English on drop.
///
/// Holds the locale test mutex so concurrent tests cannot race (A-012 flake fix).
#[cfg(test)]
pub struct LocaleTestGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prev: &'static str,
}

#[cfg(test)]
impl LocaleTestGuard {
    /// Lock the locale test mutex, set `language`, restore `en` on drop.
    pub fn set(language: Language) -> Self {
        let lock = LOCALE_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // rust-i18n has no getter for the active locale, so the guard always
        // restores `en` on drop.
        rust_i18n::set_locale(language.as_str());
        Self {
            _lock: lock,
            prev: "en",
        }
    }
}

#[cfg(test)]
impl Drop for LocaleTestGuard {
    fn drop(&mut self) {
        rust_i18n::set_locale(self.prev);
    }
}

/// Apply an language for tests without requiring OS detection.
///
/// Prefer [`LocaleTestGuard::set`] so the locale is restored and tests serialize.
#[cfg(test)]
pub fn set_locale_for_test(language: Language) {
    // Backward-compatible: still sets locale, but callers that need isolation
    // should use LocaleTestGuard. Holding a brief lock reduces race windows.
    let _g = LOCALE_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    rust_i18n::set_locale(language.as_str());
}

/// Clap `value_parser` for `--locale`: accept aliases, return canonical tag.
///
/// # Examples
///
/// ```
/// use atomwrite::locale::parse_cli_locale;
///
/// assert_eq!(parse_cli_locale("pt-br"), Ok("pt-BR".to_string()));
/// assert_eq!(parse_cli_locale("en"), Ok("en".to_string()));
/// assert!(parse_cli_locale("ja").is_err());
/// ```
///
/// # Errors
///
/// Returns `Err` with the formatted message listing `available_tags()` when the normalized tag is empty, when `parse_langid` rejects it as malformed BCP 47, or when its language is neither `en` nor `pt` (for example `ja`).
pub fn parse_cli_locale(raw: &str) -> Result<String, String> {
    let tag = normalize_bcp47_tag(raw);
    if tag.is_empty() {
        return Err(format!(
            "invalid locale {raw:?}; available: {}",
            available_tags().join(", ")
        ));
    }
    // Reject completely unknown language families (do not silently map `zh` → en).
    let parsed = parse_langid(&tag);
    if parsed.is_none() {
        return Err(format!(
            "invalid locale {raw:?}; available: {}",
            available_tags().join(", ")
        ));
    }
    // If user asked for an unsupported language (e.g. `ja`), negotiation yields `en`
    // — surface a clear error instead of silent English.
    if let Some(id) = parsed {
        let lang = id.language.as_str();
        if !matches!(lang, "en" | "pt") {
            return Err(format!(
                "unsupported locale {raw:?} (language {lang}); available: {}",
                available_tags().join(", ")
            ));
        }
    }
    Ok(negotiate_to_language(&[&tag]).as_str().to_string())
}

/// Emit a tracing event for detection failure / resolved locale (call after tracing init).
pub fn log_resolved_locale() {
    match resolved_state() {
        Some(state) => {
            if state.detection_failed {
                tracing::debug!(
                    system_raw = ?state.system_raw,
                    fallback = state.language.as_str(),
                    "locale OS detection missing or unparsable; using fallback"
                );
            }
            tracing::debug!(
                locale = state.language.as_str(),
                source = state.source.as_str(),
                system_raw = ?state.system_raw,
                persisted = ?state.persisted_tag,
                "locale resolved"
            );
        }
        None => {
            tracing::debug!("locale state not initialized");
        }
    }
}

#[cfg(test)]
include!("locale_tests.inc.rs");
