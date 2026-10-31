// SPDX-License-Identifier: MIT OR Apache-2.0
// Write side of the `config` verb: `set`, `unset` and the atomic commit
// (v0.1.37, T6a). Included from config_cmd.rs.

/// Text of an existing config file; empty when the file does not exist.
fn read_existing(path: &Path) -> Result<String, AtomwriteError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(AtomwriteError::ConfigInvalid {
            reason: format!("cannot read config {}: {e}", path.display()),
        }),
    }
}

/// Lock a config file BEFORE it is read, so concurrent `config set` and
/// `config unset` calls serialise and none loses a key (A1).
///
/// The sidecar lives next to the file, so the directory is created first.
fn lock_config_file(path: &Path) -> Result<crate::lock::LockGuard> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        crate::storage::ensure_dir(parent).map_err(|source| AtomwriteError::Io { source })?;
    }
    crate::lock::acquire_for_update(path)
}

/// Parse a file as TOML. A file that is not TOML is never rewritten: the edit
/// could not keep its content, so the user fixes it by hand first.
fn parse_syntax(path: &Path, text: &str) -> Result<toml::Table, AtomwriteError> {
    text.parse::<toml::Table>()
        .map_err(|e| AtomwriteError::ConfigInvalid {
            reason: format!(
                "malformed config {}: {e}; fix the TOML by hand before using `config`",
                path.display()
            ),
        })
}

/// Validate a whole config text the way the startup load does.
fn validate_text(text: &str) -> Result<(), AtomwriteError> {
    let cfg: AtomwriteConfig = toml::from_str(text).map_err(|e| AtomwriteError::ConfigInvalid {
        reason: format!("the config would be invalid: {e}"),
    })?;
    if cfg.version > crate::constants::CONFIG_SCHEMA_VERSION {
        return Err(AtomwriteError::ConfigInvalid {
            reason: format!(
                "the config declares version {}; this build supports up to {}",
                cfg.version,
                crate::constants::CONFIG_SCHEMA_VERSION
            ),
        });
    }
    crate::config::validate_all(&cfg)
}

/// Validate only the section a key belongs to.
fn validate_section(text: &str, spec: &KeySpec) -> Result<(), AtomwriteError> {
    let table = text
        .parse::<toml::Table>()
        .map_err(|e| AtomwriteError::ConfigInvalid {
            reason: format!("the config would be invalid: {e}"),
        })?;
    let mut only = toml::Table::new();
    if let Some(section) = table.get(spec.section) {
        only.insert(spec.section.to_string(), section.clone());
    }
    let cfg: AtomwriteConfig =
        toml::Value::Table(only)
            .try_into()
            .map_err(|e| AtomwriteError::ConfigInvalid {
                reason: format!("the config would be invalid: {e}"),
            })?;
    crate::config::validate_all(&cfg)
}

/// Check an edited file before it replaces the original.
///
/// The whole file must validate. When the original was already broken
/// somewhere else, that damage must not block the repair: only the touched
/// section is checked then, and the untouched problem is reported as a warning.
fn validate_edit(
    path: &Path,
    original: &str,
    updated: &str,
    spec: &KeySpec,
    warnings: &mut Vec<String>,
) -> Result<(), AtomwriteError> {
    let Err(first) = validate_text(updated) else {
        return Ok(());
    };
    let Err(before) = validate_text(original) else {
        return Err(first);
    };
    validate_section(updated, spec)?;
    warnings.push(
        rust_i18n::t!(
            "warn.config-other-problems",
            path = path.display().to_string(),
            reason = before.to_string()
        )
        .to_string(),
    );
    Ok(())
}

/// Replace `path` atomically with `content`.
///
/// The jail root is the config directory for the global scope and the
/// workspace for the workspace scope. No backup is made: the file is a few
/// lines the user can re-create with `config set`. A global file is forced to
/// owner-only permissions.
fn commit(
    path: &Path,
    scope: ConfigScope,
    workspace: &Path,
    content: &str,
) -> Result<(), AtomwriteError> {
    let parent = path.parent().ok_or_else(|| AtomwriteError::ConfigInvalid {
        reason: format!("config path {} has no parent directory", path.display()),
    })?;
    if !parent.exists() {
        crate::storage::ensure_dir(parent).map_err(|source| AtomwriteError::Io { source })?;
    }
    let jail = match scope {
        ConfigScope::Workspace => workspace,
        ConfigScope::Global => parent,
    };
    let opts = AtomicWriteOptions {
        backup: false,
        wal_policy: crate::wal::WalPolicy::Never,
        ..AtomicWriteOptions::default()
    };
    atomic_write(path, content.as_bytes(), &opts, jail).map_err(|e| {
        e.downcast::<AtomwriteError>()
            .unwrap_or_else(|other| AtomwriteError::InternalError {
                reason: other.to_string(),
            })
    })?;
    if scope == ConfigScope::Global {
        crate::storage::restrict_to_owner(path).map_err(|source| AtomwriteError::Io { source })?;
    }
    Ok(())
}

/// Persist the locale preference; returns the previous tag.
fn set_locale(parsed: &Parsed) -> Result<Option<serde_json::Value>, AtomwriteError> {
    let old = crate::locale::preference_path()
        .and_then(|path| crate::locale::read_persisted_preference(&path))
        .map(|tag| serde_json::json!(tag));
    let language = crate::locale::Language::from_tag(&parsed.toml_text).ok_or_else(|| {
        AtomwriteError::InvalidInput {
            reason: format!("'{}' is not a supported locale", parsed.toml_text),
        }
    })?;
    crate::locale::write_persisted_preference(language)
        .map_err(|source| AtomwriteError::Io { source })?;
    Ok(old)
}

/// Write one TOML key; returns the value the scope file held before.
fn set_toml_key(
    spec: &KeySpec,
    parsed: &Parsed,
    path: &Path,
    scope: ConfigScope,
    workspace: &Path,
    warnings: &mut Vec<String>,
) -> Result<Option<serde_json::Value>> {
    let _update_lock = lock_config_file(path)?;
    let original = read_existing(path)?;
    let old = lookup_in(&parse_syntax(path, &original)?, spec);
    let key_path = format!("{}.{}", spec.section, spec.field);
    let (updated, _, _) = crate::commands::set::toml_set(&original, &key_path, &parsed.toml_text)?;
    validate_edit(path, &original, &updated, spec, warnings)?;
    if updated != original {
        commit(path, scope, workspace, &updated)?;
    }
    Ok(old)
}

fn set_action(args: &ConfigSetArgs, workspace: &Path) -> Result<ConfigOutput> {
    let spec = find_key(&args.key)?;
    ensure_scope(spec, args.scope)?;
    let parsed = spec.kind.parse(&args.key, &args.value)?;
    let path = resolve_file(spec, args.scope, workspace)?;
    let mut out = ConfigOutput::new("set");
    out.scope = Some(args.scope.as_str());
    out.key = Some(spec.name());
    out.origin = Some(Origin::from(args.scope).as_str());
    out.config_path = Some(path.display().to_string());
    if args.dry_run {
        // Read what the scope file holds today and stop there: computing the
        // change is enough to answer, and the file keeps its bytes.
        let old = match spec.store {
            Store::LocaleFile => crate::locale::read_persisted_preference(&path)
                .map(|tag| serde_json::json!(tag)),
            Store::Toml | Store::Bootstrap => {
                let original = read_existing(&path)?;
                lookup_in(&parse_syntax(&path, &original)?, spec)
            }
        };
        out.dry_run = Some(true);
        out.changed = Some(old.as_ref() != Some(&parsed.json));
        out.old_value = old;
        out.value = Some(parsed.json);
        return Ok(out);
    }
    let old = match spec.store {
        Store::LocaleFile => set_locale(&parsed)?,
        Store::Toml | Store::Bootstrap => set_toml_key(
            spec,
            &parsed,
            &path,
            args.scope,
            workspace,
            &mut out.warnings,
        )?,
    };
    if spec.store == Store::Bootstrap {
        let relocated = crate::storage::config_dir_from(Some(PathBuf::from(&parsed.toml_text)))
            .map(|dir| dir.join(crate::constants::GLOBAL_CONFIG_FILE));
        out.warnings.push(
            rust_i18n::t!(
                "warn.config-storage-home",
                path = relocated
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            )
            .to_string(),
        );
    }
    out.changed = Some(old.as_ref() != Some(&parsed.json));
    out.old_value = old;
    out.value = Some(parsed.json);
    Ok(out)
}

/// Remove one TOML key; returns the value the scope file held before.
fn unset_toml_key(
    spec: &KeySpec,
    path: &Path,
    scope: ConfigScope,
    workspace: &Path,
    warnings: &mut Vec<String>,
) -> Result<Option<serde_json::Value>> {
    let _update_lock = lock_config_file(path)?;
    let original = read_existing(path)?;
    let old = lookup_in(&parse_syntax(path, &original)?, spec);
    if old.is_none() {
        return Ok(None);
    }
    let mut doc: toml_edit::DocumentMut =
        original
            .parse()
            .map_err(|e| AtomwriteError::ConfigInvalid {
                reason: format!("malformed config {}: {e}", path.display()),
            })?;
    let removed = doc
        .get_mut(spec.section)
        .and_then(toml_edit::Item::as_table_like_mut)
        .and_then(|table| table.remove(spec.field))
        .is_some();
    if removed {
        let emptied = doc
            .get(spec.section)
            .and_then(toml_edit::Item::as_table_like)
            .is_some_and(toml_edit::TableLike::is_empty);
        if emptied {
            doc.remove(spec.section);
        }
        let updated = doc.to_string();
        validate_edit(path, &original, &updated, spec, warnings)?;
        commit(path, scope, workspace, &updated)?;
    }
    Ok(old)
}

fn unset_action(args: &ConfigUnsetArgs, workspace: &Path) -> Result<ConfigOutput> {
    let spec = find_key(&args.key)?;
    ensure_scope(spec, args.scope)?;
    let path = resolve_file(spec, args.scope, workspace)?;
    let mut out = ConfigOutput::new("unset");
    out.scope = Some(args.scope.as_str());
    out.key = Some(spec.name());
    out.config_path = Some(path.display().to_string());
    if args.dry_run {
        let old = match spec.store {
            Store::LocaleFile => crate::locale::read_persisted_preference(&path)
                .map(|tag| serde_json::json!(tag)),
            Store::Toml | Store::Bootstrap => {
                let original = read_existing(&path)?;
                lookup_in(&parse_syntax(&path, &original)?, spec)
            }
        };
        out.dry_run = Some(true);
        out.changed = Some(old.is_some());
        out.old_value = old;
        return Ok(out);
    }
    let old = match spec.store {
        Store::LocaleFile => {
            let old =
                crate::locale::read_persisted_preference(&path).map(|tag| serde_json::json!(tag));
            crate::locale::clear_persisted_preference()
                .map_err(|source| AtomwriteError::Io { source })?;
            old
        }
        Store::Toml | Store::Bootstrap => {
            unset_toml_key(spec, &path, args.scope, workspace, &mut out.warnings)?
        }
    };
    out.changed = Some(old.is_some());
    out.old_value = old;
    Ok(out)
}
