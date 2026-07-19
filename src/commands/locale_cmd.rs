// SPDX-License-Identifier: MIT OR Apache-2.0

//! Diagnose and persist UI locale preference (`atomwrite locale`).
//!
//! Emits a single NDJSON report so agents and humans can inspect the resolved
//! language, OS raw tag, XDG preference path, and available MVP locales.
//!
//! Workload: I/O-bound (XDG preference read/write + OS locale probe).
//! Parallelism: none — single diagnostic/mutation; no multi-item fan-out.

use std::io::Write;

use anyhow::{bail, Result};
use clap::Args;
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::GlobalArgs;
use crate::locale::{
    available_tags, clear_persisted_preference, negotiate_to_idioma, parse_cli_locale,
    preference_path, read_persisted_preference, resolved_state, write_persisted_preference, Idioma,
};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `locale`.
#[derive(Args, Debug, Default)]
pub struct LocaleArgs {
    /// Persist a UI locale preference for future runs (`en` or `pt-BR`).
    #[arg(long, value_name = "TAG", help = "Persist locale preference (en, pt-BR)")]
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
struct LocaleReport {
    r#type: &'static str,
    resolved: String,
    source: String,
    system_raw: Option<String>,
    available: Vec<String>,
    persisted: Option<String>,
    preference_path: Option<String>,
    detection_failed: bool,
    direction: String,
    /// Machine NDJSON codes/Display stay English; suggestions follow locale.
    ndjson_policy: &'static str,
    /// Present when `--set` / `--clear` mutated the preference file.
    preference_action: Option<String>,
    note: Option<String>,
}

/// Report locale state; optionally set or clear the XDG preference.
#[tracing::instrument(skip_all, fields(command = "locale"))]
pub fn cmd_locale(
    args: &LocaleArgs,
    _global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
    _defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    if args.set.is_some() && args.clear {
        bail!("--set and --clear are mutually exclusive");
    }

    let mut preference_action = None;
    let mut note = None;

    if args.clear {
        let path = clear_persisted_preference()?;
        preference_action = Some("cleared".into());
        note = Some(
            "persisted preference cleared; takes effect on the next invocation".into(),
        );
        tracing::info!(?path, "locale preference cleared");
    } else if let Some(raw) = args.set.as_deref() {
        let tag = parse_cli_locale(raw).map_err(|e| anyhow::anyhow!(e))?;
        let idioma = Idioma::from_tag(&tag).unwrap_or_else(|| negotiate_to_idioma(&[&tag]));
        let path = write_persisted_preference(idioma)?;
        preference_action = Some(format!("set:{}", idioma.as_str()));
        note = Some(format!(
            "persisted {} at {}; takes effect on the next invocation (current process already resolved)",
            idioma.as_str(),
            path.display()
        ));
        tracing::info!(locale = idioma.as_str(), path = %path.display(), "locale preference saved");
    }

    let state = resolved_state();
    let pref_path = preference_path();
    let persisted_now = pref_path
        .as_ref()
        .and_then(|p| read_persisted_preference(p));

    let report = LocaleReport {
        r#type: "locale_report",
        resolved: state
            .map(|s| s.idioma.as_str().to_string())
            .unwrap_or_else(|| "en".into()),
        source: state
            .map(|s| s.source.as_str().to_string())
            .unwrap_or_else(|| "default".into()),
        system_raw: state.and_then(|s| s.system_raw.clone()),
        available: available_tags().iter().map(|s| (*s).to_string()).collect(),
        persisted: persisted_now,
        preference_path: pref_path.map(|p| p.display().to_string()),
        detection_failed: state.map(|s| s.detection_failed).unwrap_or(true),
        direction: state
            .map(|s| match s.idioma.direcao() {
                crate::locale::TextDirection::Ltr => "ltr",
                crate::locale::TextDirection::Rtl => "rtl",
            })
            .unwrap_or("ltr")
            .to_string(),
        ndjson_policy: "error codes and Display messages stay English; suggestions follow resolved locale",
        preference_action,
        note,
    };

    writer.write_event(&report)?;
    Ok(())
}
