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

use crate::atomic::AtomicWriteOptions;
use crate::cli::{CaseArgs, GlobalArgs, IdentifierCase};
use crate::commands::resolve_backup;
use crate::concurrency::should_parallelize;
use crate::output::NdjsonWriter;

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct CaseResult {
    r#type: &'static str,
    path: String,
    identifier: String,
    from_style: String,
    to_style: String,
    before: String,
    after: String,
    elapsed_ms: u64,
    /// Write strategy the atomic layer used: `rename`, `inplace` or `copyback`.
    ///
    /// The preview event reports `none`, because a dry run takes no write path.
    write_strategy: &'static str,
    /// Why the requested backup was skipped, when the directory refused it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    backup_skipped: Option<String>,
    /// Local diagnostic for a payload that looks destructive (B-013).
    #[serde(skip_serializing_if = "Option::is_none")]
    risk_assessment: Option<crate::ndjson_types::WriteRiskAssessment>,
}

/// Terminal event of `case`: identifiers renamed across the targets.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct CaseSummary {
    r#type: &'static str,
    identifiers_total: u64,
    files_modified: u64,
    /// Files a dry run would have renamed.
    ///
    /// `files_modified` stays at zero without a write, so this is the only
    /// number that tells the caller the size of the change it previewed.
    #[serde(skip_serializing_if = "Option::is_none")]
    files_planned: Option<u64>,
    /// Regular files the run actually looked at, after the directory walk.
    files_visited: u64,
    elapsed_ms: u64,
    /// Set when `--limit` cut the collected target list before the rename.
    ///
    /// `cap_collected` truncates the list and raises the same process-wide flag
    /// that `backup`, `copy`, `delete`, `hash`, `list` and `prune-backups` read,
    /// so every verb that spends the work budget reports the cut the same way.
    /// Without a limit the flag stays down and the field is omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    walker_stopped_early: Option<bool>,
    /// Targets the `--limit` budget cut before the rename.
    #[serde(skip_serializing_if = "Option::is_none")]
    files_budget_dropped: Option<u64>,
    /// Directories or files the walk could not open, so `files_visited` is not
    /// the whole tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    files_failed: Option<u64>,
    /// One entry per failed target, with `path`, `code`, `exit` and `error`.
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<crate::ndjson_types::FileFailure>>,
    /// The run answered from part of its targets, so the answer is partial.
    ///
    /// Present exactly when `files_failed` is present, so a consumer reads one
    /// flag instead of comparing two counters.
    #[serde(skip_serializing_if = "Option::is_none")]
    partial: Option<bool>,
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
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when `--subvert` is empty, carries an odd number of
/// identifiers, names an identical OLD/NEW pair, or no regular file matched the given paths.
/// Returns `AtomwriteError::InvalidInput` when an include or exclude glob does not compile.
/// Returns `AtomwriteError::WorkspaceJail` when a target path escapes the workspace.
/// Returns `AtomwriteError::SymlinkBlocked`, `AtomwriteError::FifoDetected` or
/// `AtomwriteError::DeviceFile` when a target path is a link, a FIFO or a device.
/// Returns `AtomwriteError::StateDrift` when a file changed between its read and the atomic write.
/// Returns `AtomwriteError::Io` when a target file cannot be read or rewritten,
/// after the terminal summary counts that file in `files_failed` and `errors[]`.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
/// Returns the first walk failure, published as an `ok: false` summary, when a directory under
/// the given roots could not be opened.
#[tracing::instrument(skip_all, fields(command = "case"))]
pub fn cmd_case(
    args: &CaseArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    defaults: &crate::config::DefaultsSection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let dry_run = args.dry_run;
    let resolved = resolve_backup(&args.backup_opts, defaults);
    let max_size = global.effective_max_filesize();
    let to_style = case_style_name(&args.to);
    let preserve_timestamps = args.preserve_timestamps;
    let strict_atomic = args.strict.strict_atomic;

    // G-029/G-040: clap requires `--subvert`; this is defense-in-depth only (no "yet").
    if args.subvert.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "--subvert OLD NEW is required (pair of identifiers to rename). Example: \
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
        // v0.1.36 — an identical pair is a no-op rename that silently pretends
        // to have converted something. Refuse it as invalid input; the
        // remediation text comes from `suggestion.invalid-input` (both locales).
        if from == to {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: format!("--subvert pair OLD and NEW are identical ({from:?})"),
            }
            .into());
        }
        let converted = match args.to {
            IdentifierCase::Snake => to.to_snake_case(),
            IdentifierCase::Camel => to.to_lower_camel_case(),
            IdentifierCase::Pascal => to.to_upper_camel_case(),
            IdentifierCase::Kebab => to.to_kebab_case(),
            IdentifierCase::ScreamingSnake => to.to_shouty_snake_case(),
        };
        pairs.push((from, converted));
    }

    // A glob that does not compile is a caller mistake: refuse it before any I/O.
    crate::walk_globs::validate(&args.include, &args.exclude)?;

    // Validate every root once, then expand the directories. A directory used to
    // be dropped in silence, so `case src/` exited 0 having rewritten nothing;
    // the walker makes the directory a real subject, and the empty result is
    // reported as invalid input instead of a fake success (ETD: the subject is
    // whatever argv named, file or directory).
    let mut files: Vec<PathBuf> = Vec::with_capacity(args.paths.len());
    let mut dir_roots: Vec<PathBuf> = Vec::new();
    for path in &args.paths {
        let validated = crate::path_safety::validate_path(path, &workspace)?;
        if validated.is_file() {
            files.push(validated);
        } else if validated.is_dir() {
            dir_roots.push(validated);
        }
    }
    // A directory the walk cannot open hides the files this verb was asked to
    // rewrite, so the terminal reports the hole instead of counting a short tree.
    let failures: std::sync::Arc<std::sync::Mutex<crate::ndjson_types::FailureTally>> =
        std::sync::Arc::new(std::sync::Mutex::new(
            crate::ndjson_types::FailureTally::default(),
        ));
    if !dir_roots.is_empty() {
        let walked = crate::concurrency::collect_files_from_roots_tallied(
            &dir_roots,
            global.threads,
            |builder| {
                builder
                    .hidden(!global.hidden)
                    .git_ignore(!global.no_gitignore)
                    .follow_links(global.follow_symlinks);
                // Both glob lists were validated before this point, so the
                // builder cannot receive an uncompilable override here.
                crate::best_effort::best_effort(
                    crate::walk_globs::add_overrides(
                        builder,
                        &dir_roots[0],
                        &args.include,
                        &args.exclude,
                    ),
                    "apply include/exclude globs",
                );
            },
            Some(std::sync::Arc::clone(&failures)),
        );
        files.extend(walked);
    }
    if files.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "no regular file matched the given paths; name the files directly or widen --include"
                .into(),
        }
        .into());
    }

    // `--limit` cuts the target list BEFORE the first file is touched, so
    // `case --limit 1` converts in exactly one file.
    crate::output::cap_collected(&mut files);

    let files_visited = files.len() as u64;
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
                        strict_atomic,
                        write_cfg,
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
                        strict_atomic,
                        write_cfg,
                        start,
                    )
                })
                .collect()
        };

        // `items` keeps the order of `files` in both branches, so every result
        // is paired with the file that produced it. A file the run could not
        // read or rewrite is tallied like a walk failure instead of aborting
        // the run: the summary then names it in `files_failed`, `partial` and
        // `errors[]` before the same error ends the process.
        for (path, item) in files.iter().zip(items) {
            match item {
                Ok(CaseItem::Unchanged) => {}
                Ok(CaseItem::Preview(ev)) => {
                    // Preview is dry-run: count identifiers, not disk mutations (R-002c).
                    total_identifiers += 1;
                    writer.write_event(&ev)?;
                }
                Ok(CaseItem::Done(ev)) => {
                    total_identifiers += 1;
                    files_modified += 1;
                    writer.write_event(&ev)?;
                }
                Err(err) => {
                    failures
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .record(path, err);
                }
            }
        }
    }

    let (failed_count, failed_items, first_failure) = match std::sync::Arc::try_unwrap(failures) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default().into_parts(),
        Err(arc) => {
            std::mem::take(&mut *arc.lock().unwrap_or_else(|e| e.into_inner())).into_parts()
        }
    };
    let files_failed = (failed_count > 0).then_some(failed_count);
    let summary = CaseSummary {
        r#type: "summary",
        identifiers_total: total_identifiers,
        // R-002c / R-DRY-001: dry-run never claims files_modified > 0.
        files_modified: crate::commands::summary_metrics::files_modified_count(
            files_modified,
            dry_run,
        ),
        files_planned: crate::commands::summary_metrics::files_planned_for_summary(
            files_modified,
            dry_run,
        ),
        files_visited,
        elapsed_ms: start.elapsed().as_millis() as u64,
        walker_stopped_early: crate::output::limit_reached().then_some(true),
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        files_failed,
        partial: crate::ndjson_types::partial_of(files_failed),
        errors: (!failed_items.is_empty()).then_some(failed_items),
    };
    // A rename that landed is an answer the run did produce, and the summary
    // names every file it could not read or write in `files_failed` and
    // `errors[]`. Failing the run would throw that answer away, so `case`
    // follows `search`, `replace`, `transform` and `scope` here.
    if files_modified > 0 {
        writer.write_terminal(&summary)?;
        return Ok(());
    }
    // With nothing renamed the run has no answer: the first failure explains
    // why, and a run that reached every file still ends with its summary.
    if let Some(err) = first_failure {
        return Err(writer.publish_failure(&summary, err));
    }
    writer.write_terminal(&summary)?;
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
    strict_atomic: bool,
    write_cfg: &crate::config::WriteSection,
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
    // Risk only: a rename that shortens an identifier is the operation working.
    // Assessed before the dry-run branch so the preview carries it too.
    let risk_assessment = crate::commands::write_guards::assess_content_risk(
        new_content.as_bytes(),
        content.len() as u64,
        new_content.len() as u64,
        write_cfg,
    );
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
            write_strategy: "none",
            backup_skipped: None,
            risk_assessment,
        }));
    }
    let opts = AtomicWriteOptions {
        backup,
        syntax_check: false,
        retention,
        preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup,
        durability: crate::platform::Durability::Auto,
    };
    // A1: refuse to overwrite a file that changed since it was read.
    let base = crate::checksum::hash_bytes(content.as_bytes());
    let result = crate::atomic::atomic_write_based(
        path,
        new_content.as_bytes(),
        &opts,
        workspace,
        Some(&base),
    )?;
    Ok(CaseItem::Done(CaseResult {
        r#type: "case",
        path: path_str,
        identifier,
        from_style: from_style.to_string(),
        to_style: to_style.to_string(),
        before: from.to_string(),
        after: converted.to_string(),
        elapsed_ms: start.elapsed().as_millis() as u64,
        write_strategy: result.write_strategy,
        backup_skipped: result.backup_skipped,
        risk_assessment,
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
