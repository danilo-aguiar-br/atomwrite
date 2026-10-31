// SPDX-License-Identifier: MIT OR Apache-2.0

//! `config` — read and write the persistent tuning keys (v0.1.37).
//!
//! Every tuning value of the product is a key of one typed table
//! (`config_cmd_keys.inc.rs`) that `list`, `get`, `set` and `unset` share. A key
//! is written to the config file of the `--scope` named in argv: `global` is the
//! XDG `config.toml`, `workspace` is `.atomwrite.toml` at the workspace root.
//! The effective value of a key is the workspace file, then the global file,
//! then the built-in default; a CLI flag, where the verb has one, wins over all.
//!
//! The verb runs before the config load, so a config file that other verbs
//! reject (exit 78) never stops the verb that repairs it.
//!
//! Workload: I/O-bound (one or two small files). Parallelism: none.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::cli::GlobalArgs;
use crate::config::AtomwriteConfig;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;

include!("config_cmd_keys.inc.rs");

include!("config_cmd_types.inc.rs");

impl ConfigScope {
    fn as_str(self) -> &'static str {
        Origin::from(self).as_str()
    }
}

/// Config file a key lives in for `scope`, or `None` when the platform has no
/// config directory.
fn scope_file(store: Store, scope: ConfigScope, workspace: &Path) -> Option<PathBuf> {
    match (scope, store) {
        (ConfigScope::Workspace, _) => {
            Some(workspace.join(crate::constants::WORKSPACE_CONFIG_FILE))
        }
        (ConfigScope::Global, Store::Toml) => crate::storage::global_config_path(),
        (ConfigScope::Global, Store::Bootstrap) => crate::storage::bootstrap_config_path(),
        (ConfigScope::Global, Store::LocaleFile) => crate::locale::preference_path(),
    }
}

/// File the invocation acts on, for the execution-target stamp.
///
/// `None` only for the actions that read (`list`, `get`, `doctor`): they name
/// no file, so there is nothing an operator could have displaced. A `set` or
/// `unset` names the file through `--scope` even when the key itself is
/// unknown, and that error envelope must still say which file it did not
/// touch. The TOML store is the fallback: `find_key` answers first, so
/// `storage.home` (bootstrap) and `locale` (locale file) keep their own file.
#[must_use]
pub fn target_file(args: &ConfigArgs, workspace: &Path) -> Option<PathBuf> {
    let by_key = |key: &str, scope: ConfigScope| {
        find_key(key)
            .ok()
            .and_then(|spec| scope_file(spec.store, scope, workspace))
            .or_else(|| scope_file(Store::Toml, scope, workspace))
    };
    match &args.action {
        ConfigAction::Path(a) => scope_file(Store::Toml, a.scope, workspace),
        ConfigAction::Set(a) => by_key(&a.key, a.scope),
        ConfigAction::Unset(a) => by_key(&a.key, a.scope),
        ConfigAction::List | ConfigAction::Get(_) | ConfigAction::Doctor => None,
    }
}

/// File of `scope` for `spec`, or the exit-78 error when it cannot be located.
fn resolve_file(
    spec: &KeySpec,
    scope: ConfigScope,
    workspace: &Path,
) -> Result<PathBuf, AtomwriteError> {
    scope_file(spec.store, scope, workspace).ok_or_else(|| AtomwriteError::ConfigInvalid {
        reason: "cannot resolve the XDG config directory for this platform".into(),
    })
}

/// Reject a workspace write of a key that only the global file may hold.
fn ensure_scope(spec: &KeySpec, scope: ConfigScope) -> Result<(), AtomwriteError> {
    if spec.scopes == Scopes::GlobalOnly && scope == ConfigScope::Workspace {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "config key '{}' is global-only; use --scope global",
                spec.name()
            ),
        });
    }
    Ok(())
}

/// Run the `config` verb.
///
/// # Errors
///
/// `InvalidInput` (exit 65) for an unknown key, a value the key rejects, or a
/// global-only key in the workspace scope. `ConfigInvalid` (exit 78) when a file
/// cannot be read or is not valid TOML. I/O errors from the atomic write.
#[tracing::instrument(skip_all, fields(command = "config"))]
pub fn cmd_config(
    args: &ConfigArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    workspace: &Path,
) -> Result<()> {
    let start = Instant::now();
    let mut out = match &args.action {
        ConfigAction::Path(a) => path_action(a, workspace)?,
        ConfigAction::List => list_action(global, workspace)?,
        ConfigAction::Get(a) => get_action(a, global, workspace)?,
        ConfigAction::Set(a) => set_action(a, workspace)?,
        ConfigAction::Unset(a) => unset_action(a, workspace)?,
        ConfigAction::Doctor => doctor_action(global, workspace)?,
    };
    out.elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    writer.write_event(&out)?;
    Ok(())
}

fn path_action(args: &ConfigPathArgs, workspace: &Path) -> Result<ConfigOutput, AtomwriteError> {
    let path = scope_file(Store::Toml, args.scope, workspace).ok_or_else(|| {
        AtomwriteError::ConfigInvalid {
            reason: "cannot resolve the XDG config directory for this platform".into(),
        }
    })?;
    let mut out = ConfigOutput::new("path");
    out.scope = Some(args.scope.as_str());
    out.exists = Some(path.is_file());
    out.config_path = Some(path.display().to_string());
    Ok(out)
}

/// One config file read for `list` / `get`.
struct FileLayer {
    origin: Origin,
    path: PathBuf,
    table: toml::Table,
}

/// Read a file as a raw table. `Ok(None)` when it does not exist.
///
/// Unlike the startup load this is lenient about unknown keys, so a file that
/// other verbs reject can still be inspected; only broken TOML is an error.
fn read_table(path: &Path) -> Result<Option<toml::Table>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    text.parse::<toml::Table>()
        .map(Some)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Files that feed the effective value, highest priority first.
///
/// With `--config` that single file replaces the other two, exactly as it does
/// for every other verb. A file that cannot be parsed is skipped with a warning.
fn effective_layers(
    global: &GlobalArgs,
    workspace: &Path,
    warnings: &mut Vec<String>,
) -> Result<Vec<FileLayer>, AtomwriteError> {
    let mut candidates: Vec<(Origin, PathBuf)> = Vec::new();
    if let Some(explicit) = global.config.as_deref() {
        if !explicit.is_file() {
            return Err(AtomwriteError::ConfigInvalid {
                reason: format!("cannot read config {}: not a file", explicit.display()),
            });
        }
        candidates.push((Origin::Explicit, explicit.to_path_buf()));
    } else {
        candidates.push((
            Origin::Workspace,
            workspace.join(crate::constants::WORKSPACE_CONFIG_FILE),
        ));
        if let Some(path) = crate::storage::global_config_path() {
            candidates.push((Origin::Global, path));
        }
    }
    let mut layers = Vec::new();
    for (origin, path) in candidates {
        match read_table(&path) {
            Ok(Some(table)) => layers.push(FileLayer {
                origin,
                path,
                table,
            }),
            Ok(None) => {}
            Err(reason) => warnings
                .push(rust_i18n::t!("warn.config-layer-unreadable", reason = reason).to_string()),
        }
    }
    Ok(layers)
}

/// JSON form of a scalar TOML value; anything else is `null`.
fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::Integer(n) => serde_json::json!(n),
        toml::Value::Float(n) => serde_json::json!(n),
        toml::Value::Boolean(b) => serde_json::json!(b),
        toml::Value::String(s) => serde_json::json!(s),
        // A `Kind::List` key holds a TOML array. Without this arm the reader
        // answered `null` for a value the file really carries, and `config get`
        // then omitted the key as if it were absent.
        toml::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(toml_to_json).collect())
        }
        _ => serde_json::Value::Null,
    }
}

/// Value of `spec` inside a raw table.
fn lookup_in(table: &toml::Table, spec: &KeySpec) -> Option<serde_json::Value> {
    let holder = if spec.section.is_empty() {
        Some(table)
    } else {
        table.get(spec.section).and_then(toml::Value::as_table)
    };
    holder?.get(spec.field).map(toml_to_json)
}

/// `None` for JSON `null`: an envelope omits a key rather than carry a null.
fn non_null(value: serde_json::Value) -> Option<serde_json::Value> {
    (!value.is_null()).then_some(value)
}

/// The scopes a key can be written in, as wire names.
fn scope_names(spec: &KeySpec) -> Vec<&'static str> {
    match spec.scopes {
        Scopes::Both => vec![
            ConfigScope::Global.as_str(),
            ConfigScope::Workspace.as_str(),
        ],
        Scopes::GlobalOnly => vec![ConfigScope::Global.as_str()],
    }
}

/// Effective value of one key, with its origin.
fn entry_for(spec: &KeySpec, layers: &[FileLayer], defaults: &AtomwriteConfig) -> KeyEntry {
    let default = (spec.default)(defaults);
    let found = match spec.store {
        Store::LocaleFile => crate::locale::preference_path().and_then(|path| {
            crate::locale::read_persisted_preference(&path)
                .map(|tag| (serde_json::json!(tag), Origin::Global, path))
        }),
        Store::Bootstrap => crate::storage::bootstrap_config_path().and_then(|path| {
            let table = read_table(&path).ok().flatten()?;
            lookup_in(&table, spec).map(|value| (value, Origin::Global, path))
        }),
        Store::Toml => layers
            .iter()
            .filter(|layer| spec.scopes == Scopes::Both || layer.origin != Origin::Workspace)
            .find_map(|layer| {
                lookup_in(&layer.table, spec).map(|v| (v, layer.origin, layer.path.clone()))
            }),
    };
    let (value, origin, source) = match found {
        Some((value, origin, path)) => (value, origin, Some(path.display().to_string())),
        None => (default.clone(), Origin::Default, None),
    };
    KeyEntry {
        key: spec.name(),
        value: non_null(value),
        origin: origin.as_str(),
        default: non_null(default),
        kind: spec.kind.label(),
        scopes: scope_names(spec),
        source,
    }
}

fn list_action(global: &GlobalArgs, workspace: &Path) -> Result<ConfigOutput, AtomwriteError> {
    let mut out = ConfigOutput::new("list");
    let layers = effective_layers(global, workspace, &mut out.warnings)?;
    let defaults = AtomwriteConfig::default();
    out.keys = Some(
        KEYS.iter()
            .map(|spec| entry_for(spec, &layers, &defaults))
            .collect(),
    );
    Ok(out)
}

fn get_action(
    args: &ConfigGetArgs,
    global: &GlobalArgs,
    workspace: &Path,
) -> Result<ConfigOutput, AtomwriteError> {
    let spec = find_key(&args.key)?;
    let mut out = ConfigOutput::new("get");
    let layers = effective_layers(global, workspace, &mut out.warnings)?;
    let entry = entry_for(spec, &layers, &AtomwriteConfig::default());
    out.key = Some(entry.key);
    out.value = entry.value;
    out.origin = Some(entry.origin);
    out.default = entry.default;
    out.config_path = entry.source;
    Ok(out)
}

/// Load the effective config exactly as the other verbs do, and report it.
///
/// The load is the measurement: a file that is unreadable, invalid TOML, or
/// carries an unknown key fails here with the same exit 78 the verb that reads
/// it would return, so `doctor` never answers `pass` over a config another verb
/// would reject.
fn doctor_action(global: &GlobalArgs, workspace: &Path) -> Result<ConfigOutput, AtomwriteError> {
    let mut out = ConfigOutput::new("doctor");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(explicit) = global.config.as_deref() {
        candidates.push(explicit.to_path_buf());
    } else {
        candidates.push(workspace.join(crate::constants::WORKSPACE_CONFIG_FILE));
        if let Some(path) = crate::storage::global_config_path() {
            candidates.push(path);
        }
    }
    let found: Vec<String> = candidates
        .iter()
        .filter(|path| path.is_file())
        .map(|path| path.display().to_string())
        .collect();
    crate::config::load_config(workspace, global.config.as_deref())?;
    out.status = Some("pass");
    out.exists = Some(!found.is_empty());
    out.config_path = found.first().cloned();
    out.config_files = Some(found);
    Ok(out)
}

/// Number of keys the `config` verb exposes.
///
/// Read from the same table `list`, `get`, `set` and `unset` use, so a guard
/// that compares a document against this number can never disagree with the
/// verb by reading a second source.
#[must_use]
pub fn config_key_count() -> usize {
    KEYS.len()
}

include!("config_cmd_write.inc.rs");
