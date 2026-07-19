// SPDX-License-Identifier: MIT OR Apache-2.0

//! v14 Tier 3 subcommand: `case` — convert identifier case (snake_case,
//! camelCase, PascalCase, kebab-case, SCREAMING_SNAKE_CASE) in source files.
//!
//! Workload: I/O-bound (read + token rewrite + atomic write).
//! Parallelism: multi-file `paths` fan-out via `rayon::par_iter` (independent
//! atomic writes per file). Subvert pairs stay sequential so renames on the
//! same file compose in declaration order. Bound: process-wide rayon pool
//! (`--threads` / `--max-concurrency`). Single-file stays sequential
//! (coordination cost > work).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use heck::{ToKebabCase, ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use rayon::prelude::*;
use serde::Serialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::cli::{CaseArgs, GlobalArgs, IdentifierCase};
use crate::commands::resolve_backup;
use crate::concurrency::should_parallelize;
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

enum CaseItem {
    Unchanged,
    Preview(CaseResult),
    Done(CaseResult),
}

/// Execute the `case` subcommand.
///
/// Renames identifiers in source files by computing the new identifier
/// in the requested case style (`snake_case`, `camelCase`, `PascalCase`,
/// `kebab-case`, `SCREAMING_SNAKE_CASE`) via the `heck` crate and
/// replacing occurrences in each target file.
#[tracing::instrument(skip_all, fields(command = "case"))]
pub fn cmd_case(
    args: &CaseArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let dry_run = args.dry_run;
    let resolved = resolve_backup(&args.backup_opts, defaults);
    let max_size = global.effective_max_filesize();
    let to_style = case_style_name(&args.to);
    let preserve_timestamps = args.preserve_timestamps;

    // G-029/G-040: clap requires `--subvert`; this is defense-in-depth only (no "yet").
    if args.subvert.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason:
                "--subvert OLD NEW is required (pair of identifiers to rename). Example: \
                     atomwrite case --to snake --subvert myVar my_var src/"
                    .into(),
        }
        .into());
    }

    // Validate pairs first so odd-count fails before any I/O.
    let mut pairs: Vec<(String, String)> = Vec::new();
    for pair in args.subvert.chunks(2) {
        if pair.len() != 2 {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason:
                    "--subvert expects an even number of identifiers (old new pairs); got odd count"
                        .into(),
            }
            .into());
        }
        let from = pair[0].clone();
        let to = pair[1].clone();
        let converted = match args.to {
            IdentifierCase::Snake => to.to_snake_case(),
            IdentifierCase::Camel => to.to_lower_camel_case(),
            IdentifierCase::Pascal => to.to_upper_camel_case(),
            IdentifierCase::Kebab => to.to_kebab_case(),
            IdentifierCase::ScreamingSnake => to.to_shouty_snake_case(),
        };
        pairs.push((from, converted));
    }

    // Validate paths once; skip non-files (same contract as before).
    let mut files: Vec<PathBuf> = Vec::with_capacity(args.paths.len());
    for path in &args.paths {
        let validated = crate::path_safety::validate_path(path, &workspace)?;
        if validated.is_file() {
            files.push(validated);
        }
    }

    let mut total_identifiers = 0u64;
    let mut files_modified = 0u64;

    // Pairs are sequential (compose renames on the same file). Paths fan out.
    for (from, converted) in &pairs {
        if crate::signal::is_global_shutdown() {
            break;
        }
        let from_style = detect_case_style(from);
        let items: Vec<Result<CaseItem, anyhow::Error>> = if should_parallelize(files.len()) {
            files
                .par_iter()
                .map(|path| {
                    process_one_case(
                        path,
                        &workspace,
                        from,
                        converted,
                        &from_style,
                        &to_style,
                        dry_run,
                        max_size,
                        resolved.backup,
                        resolved.retention,
                        resolved.keep,
                        preserve_timestamps,
                        start,
                    )
                })
                .collect()
        } else {
            files
                .iter()
                .map(|path| {
                    process_one_case(
                        path,
                        &workspace,
                        from,
                        converted,
                        &from_style,
                        &to_style,
                        dry_run,
                        max_size,
                        resolved.backup,
                        resolved.retention,
                        resolved.keep,
                        preserve_timestamps,
                        start,
                    )
                })
                .collect()
        };

        for item in items {
            match item? {
                CaseItem::Unchanged => {}
                CaseItem::Preview(ev) => {
                    // Preview is dry-run: count identifiers, not disk mutations (R-002c).
                    total_identifiers += 1;
                    writer.write_event(&ev)?;
                }
                CaseItem::Done(ev) => {
                    total_identifiers += 1;
                    files_modified += 1;
                    writer.write_event(&ev)?;
                }
            }
        }
    }

    writer.write_event(&CaseSummary {
        r#type: "summary",
        identifiers_total: total_identifiers,
        // R-002c / R-DRY-001: dry-run never claims files_modified > 0.
        files_modified: crate::commands::summary_metrics::files_modified_count(
            files_modified,
            dry_run,
        ),
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_one_case(
    path: &Path,
    workspace: &Path,
    from: &str,
    converted: &str,
    from_style: &str,
    to_style: &str,
    dry_run: bool,
    max_size: u64,
    backup: bool,
    retention: u8,
    keep_backup: bool,
    preserve_timestamps: bool,
    start: Instant,
) -> Result<CaseItem> {
    if crate::signal::is_global_shutdown() {
        return Ok(CaseItem::Unchanged);
    }
    let content = crate::file_io::read_file_string(path, max_size)?;
    // Word-boundary replace of the from identifier with `converted`.
    // Use plain `replace` for now (case-sensitive, no word boundary
    // for camel/Pascal); a smarter word-boundary matcher is a
    // future enhancement.
    let new_content = content.replace(from, converted);
    if new_content == content {
        return Ok(CaseItem::Unchanged);
    }
    let path_str = path.display().to_string();
    let identifier = format!("{from} -> {converted}");
    if dry_run {
        return Ok(CaseItem::Preview(CaseResult {
            r#type: "case_preview",
            path: path_str,
            identifier,
            from_style: from_style.to_string(),
            to_style: to_style.to_string(),
            before: from.to_string(),
            after: converted.to_string(),
            elapsed_ms: 0,
        }));
    }
    let opts = AtomicWriteOptions {
        backup,
        syntax_check: false,
        retention,
        preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    let _ = atomic_write(path, new_content.as_bytes(), &opts, workspace)?;
    Ok(CaseItem::Done(CaseResult {
        r#type: "case",
        path: path_str,
        identifier,
        from_style: from_style.to_string(),
        to_style: to_style.to_string(),
        before: from.to_string(),
        after: converted.to_string(),
        elapsed_ms: start.elapsed().as_millis() as u64,
    }))
}

fn case_style_name(style: &IdentifierCase) -> String {
    match style {
        IdentifierCase::Snake => "snake_case".into(),
        IdentifierCase::Camel => "camelCase".into(),
        IdentifierCase::Pascal => "PascalCase".into(),
        IdentifierCase::Kebab => "kebab-case".into(),
        IdentifierCase::ScreamingSnake => "SCREAMING_SNAKE".into(),
    }
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
