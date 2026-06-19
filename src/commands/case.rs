// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand: `case` — convert identifier case (snake_case,
//! camelCase, PascalCase, kebab-case, SCREAMING_SNAKE_CASE) in source files.

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use heck::{ToKebabCase, ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use serde::Serialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::cli::{CaseArgs, GlobalArgs, IdentifierCase};
use crate::commands::resolve_backup;
use crate::output::NdjsonWriter;

#[derive(Debug, Serialize)]
struct CaseResult {
    r#type: &'static str,
    path: String,
    identifier: String,
    from_style: String,
    to_style: String,
    before: String,
    after: String,
    elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
struct CaseSummary {
    r#type: &'static str,
    identifiers_total: u64,
    files_modified: u64,
    elapsed_ms: u64,
}

/// Execute the `case` subcommand.
///
/// Renames identifiers in source files by computing the new identifier
/// in the requested case style (`snake_case`, `camelCase`, `PascalCase`,
/// `kebab-case`, `SCREAMING_SNAKE_CASE`) via the `heck` crate and
/// replacing occurrences in each target file.
pub fn cmd_case(
    args: &CaseArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let dry_run = args.dry_run;
    let effective_backup = resolve_backup(args.backup, args.no_backup);

    // Apply each requested identifier rename.
    let mut total_identifiers = 0u64;
    let mut files_modified = 0u64;

    for pair in args.subvert.chunks(2) {
        if pair.len() != 2 {
            bail!("--subvert expects an even number of identifiers (old new pairs); got odd count");
        }
        let from = &pair[0];
        let to = &pair[1];

        // Compute the new identifier in every requested target style.
        let converted = match args.to {
            IdentifierCase::Snake => to.to_snake_case(),
            IdentifierCase::Camel => to.to_lower_camel_case(),
            IdentifierCase::Pascal => to.to_upper_camel_case(),
            IdentifierCase::Kebab => to.to_kebab_case(),
            IdentifierCase::ScreamingSnake => to.to_shouty_snake_case(),
        };

        for path in &args.paths {
            let validated = crate::path_safety::validate_path(path, &workspace)?;
            if !validated.is_file() {
                continue;
            }
            let content = std::fs::read_to_string(&validated)
                .with_context(|| format!("cannot read {}", validated.display()))?;
            // Word-boundary replace of the from identifier with `converted`.
            // Use plain `replace` for now (case-sensitive, no word boundary
            // for camel/Pascal); a smarter word-boundary matcher is a
            // future enhancement.
            let new_content = content.replace(from, &converted);
            if new_content == content {
                continue;
            }
            let before = from.clone();
            let after = converted.clone();
            total_identifiers += 1;
            files_modified += 1;
            if dry_run {
                writer.write_event(&CaseResult {
                    r#type: "case_preview",
                    path: validated.display().to_string(),
                    identifier: format!("{from} -> {converted}"),
                    from_style: detect_case_style(from),
                    to_style: format!("{to:?}"),
                    before,
                    after,
                    elapsed_ms: 0,
                })?;
                continue;
            }
            let opts = AtomicWriteOptions {
                backup: effective_backup,
                syntax_check: false,
                retention: 5,
                preserve_timestamps: args.preserve_timestamps,
                backup_output_dir: None,
                strategy: None,
                strict_atomic: false,
                wal_policy: crate::wal::WalPolicy::Auto,
                keep_backup: false,
            };
            let _ = atomic_write(&validated, new_content.as_bytes(), &opts, &workspace)?;
            writer.write_event(&CaseResult {
                r#type: "case",
                path: validated.display().to_string(),
                identifier: format!("{from} -> {converted}"),
                from_style: detect_case_style(from),
                to_style: format!("{to:?}"),
                before,
                after,
                elapsed_ms: start.elapsed().as_millis() as u64,
            })?;
        }
    }

    writer.write_event(&CaseSummary {
        r#type: "summary",
        identifiers_total: total_identifiers,
        files_modified,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;
    Ok(())
}

/// Best-effort detection of the input identifier's case style for
/// reporting purposes. Not authoritative — `user_id` could be either
/// `snake_case` or kebab-case-with-underscores; we just report what looks
/// most likely.
fn detect_case_style(s: &str) -> String {
    if s.contains('_') && s.chars().all(|c| !c.is_uppercase() || c == '_') {
        if s.chars().any(|c| c.is_ascii_uppercase()) {
            "SCREAMING_SNAKE".into()
        } else {
            "snake_case".into()
        }
    } else if s.contains('-') {
        "kebab-case".into()
    } else if s.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        "PascalCase".into()
    } else {
        "camelCase".into()
    }
}

#[allow(dead_code)]
fn _path_buf_marker() -> PathBuf {
    PathBuf::new()
}
