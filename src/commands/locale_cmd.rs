// SPDX-License-Identifier: MIT OR Apache-2.0

//! Diagnose and persist UI locale preference (`atomwrite locale`).
//!
//! Emits a single NDJSON report so agents and humans can inspect the resolved
//! language, OS raw tag, XDG preference path, and available MVP locales.
//!
//! Workload: I/O-bound (XDG preference read/write + OS locale probe).
//! Parallelism: none — single diagnostic/mutation; no multi-item fan-out.

use std::io::Write;

use anyhow::Result;
use clap::Args;
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::GlobalArgs;
use crate::locale::{
    Language, available_tags, clear_persisted_preference, detect_system_locale,
    negotiate_to_language, parse_cli_locale, preference_path, resolve_locale,
    write_persisted_preference,
};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `locale`.
#[derive(Args, Debug, Default)]
pub struct LocaleArgs {
    /// Persist a UI locale preference for future runs (`en` or `pt-BR`).
    #[arg(
        long,
        value_name = "TAG",
        help = "Persist locale preference (en, pt-BR)"
    )]
    pub set: Option<String>,

    /// Clear the persisted XDG locale preference.
    #[arg(
        long,
        action = clap::ArgAction::SetTrue,
        help = "Clear persisted locale preference"
    )]
    pub clear: bool,
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct LocaleReport {
    r#type: &'static str,
    resolved: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_raw: Option<String>,
    available: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    persisted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preference_path: Option<String>,
    detection_failed: bool,
    direction: String,
    /// Machine NDJSON codes/Display stay English; suggestions follow locale.
    ndjson_policy: &'static str,
    /// Present when `--set` / `--clear` mutated the preference file.
    #[serde(skip_serializing_if = "Option::is_none")]
    preference_action: Option<String>,
    /// Write strategy the atomic layer used for the preference file.
    ///
    /// Present only when this run wrote or removed the preference, so a plain
    /// report never claims a mechanism it did not use. `--clear` reports
    /// `none`, because it unlinks the file instead of writing content.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    write_strategy: Option<&'static str>,
    /// Why the requested backup was skipped, when the directory refused it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    backup_skipped: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

/// Report locale state; optionally set or clear the XDG preference.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when `--set` and `--clear` are given together or when
/// `--set` carries a tag this build does not support.
/// Returns `AtomwriteError::Io` when the preference file cannot be written or cleared because the
/// XDG config directory is unresolvable or the filesystem refuses the change.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
#[tracing::instrument(skip_all, fields(command = "locale"))]
pub fn cmd_locale(
    args: &LocaleArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
    _defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    if args.set.is_some() && args.clear {
        return Err(crate::invalid_input!(
            "--set and --clear are mutually exclusive"
        ));
    }

    let mut preference_action = None;
    let mut note = None;
    // Filled only by the two branches that touch the preference file, so a
    // plain report never claims a write path it did not take.
    let mut preference_verdict: Option<(&'static str, Option<String>)> = None;

    if args.clear {
        let path = clear_persisted_preference()?;
        preference_verdict = Some(("none", None));
        preference_action = Some("cleared".into());
        note = Some("persisted preference cleared; the report above already shows it".into());
        tracing::info!(?path, "locale preference cleared");
    } else if let Some(raw) = args.set.as_deref() {
        let tag = parse_cli_locale(raw)
            .map_err(|e| crate::error::AtomwriteError::invalid("invalid locale", e))?;
        let language = Language::from_tag(&tag).unwrap_or_else(|| negotiate_to_language(&[&tag]));
        let (path, written) = write_persisted_preference(language)?;
        preference_verdict = Some((written.write_strategy, written.backup_skipped));
        preference_action = Some(format!("set:{}", language.as_str()));
        note = Some(format!(
            "persisted {} at {}; the report above already shows it",
            language.as_str(),
            path.display()
        ));
        tracing::info!(locale = language.as_str(), path = %path.display(), "locale preference saved");
    }

    // The report must describe the state this call leaves behind, not the
    // `OnceLock` frozen at boot: after `--set` the boot state still names the
    // old preference, and a caller comparing `resolved` with `--set` would read
    // a contradiction. Re-resolving against the preference file on disk costs
    // one small read and reports the truth.
    let state = resolve_locale(
        global.lang.as_deref(),
        detect_system_locale(),
        preference_path(),
    );

    let report = LocaleReport {
        r#type: "locale_report",
        resolved: state.language.as_str().to_string(),
        source: state.source.as_str().to_string(),
        system_raw: state.system_raw.clone(),
        available: available_tags().iter().map(|s| (*s).to_string()).collect(),
        persisted: state.persisted_tag.clone(),
        preference_path: state
            .preference_path
            .as_ref()
            .map(|p| p.display().to_string()),
        detection_failed: state.detection_failed,
        direction: match state.language.direction() {
            crate::locale::TextDirection::Ltr => "ltr",
            crate::locale::TextDirection::Rtl => "rtl",
        }
        .to_string(),
        ndjson_policy: "error codes and Display messages stay English; suggestions follow resolved locale",
        preference_action,
        write_strategy: preference_verdict.as_ref().map(|verdict| verdict.0),
        backup_skipped: preference_verdict.and_then(|verdict| verdict.1),
        note,
    };

    writer.write_event(&report)?;
    Ok(())
}
