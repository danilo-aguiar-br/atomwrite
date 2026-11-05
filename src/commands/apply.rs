// SPDX-License-Identifier: MIT OR Apache-2.0

//! Patch application from stdin: unified diff, SEARCH/REPLACE, full file.
//! Workload: I/O-bound (stdin read + patch parse + atomic write).
//! Parallelism: none — single-file / sequential-by-contract (coordination cost ≥ work).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;

use crate::atomic::AtomicWriteOptions;
use crate::checksum;
use crate::cli::{ApplyArgs, GlobalArgs, PatchFormat};
use crate::commands::resolve_backup;
use crate::error::AtomwriteError;
use crate::ndjson_types::{ApplyResult, ApplyRollbackEvent};
use crate::output::NdjsonWriter;

thread_local! {
    /// File the running `apply` was about when it refused the patch.
    ///
    /// The standalone error envelope is written by `main`, which has the
    /// invocation line and not the patch, so the file that came from the patch
    /// is handed over here. It is set only from the request itself, never from
    /// ambient state, and cleared at the start of every run.
    static REFUSED_TARGET: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

/// Remember the file a failed `apply` was about.
fn remember_refused_target(target: &Path) {
    REFUSED_TARGET.with(|slot| *slot.borrow_mut() = Some(target.display().to_string()));
}

/// File the failed `apply` was about, for the standalone error envelope.
#[must_use]
pub fn refused_target() -> Option<String> {
    REFUSED_TARGET.with(|slot| slot.borrow().clone())
}

/// Apply a patch from stdin to a target file.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the target file does not exist.
/// Returns `AtomwriteError::InvalidInput` if the patch format is invalid.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns an I/O error if the write fails.
#[tracing::instrument(skip_all, fields(command = "apply"))]
pub fn cmd_apply(
    args: &ApplyArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    REFUSED_TARGET.with(|slot| *slot.borrow_mut() = None);
    let workspace = global.resolve_workspace()?;
    let resolved_backup = resolve_backup(&args.backup_opts, defaults);
    // The patch always arrives on standard input, so the invocation line names
    // the target only when `--file` is given. A patch that carries its own path
    // line takes the target from the stream, and the envelope says so instead
    // of crediting a line the caller never wrote.
    let target_source = if args.file.is_some() {
        crate::commands::path_resolution::TargetSource::Argv.as_str()
    } else {
        crate::commands::path_resolution::TargetSource::Stdin.as_str()
    };

    let max_stdin = global.effective_max_filesize();
    // The shared reader owns the chunk size, the cap, the reserve and the
    // cancellation token; a second `take` + `read_to_string` here would drift.
    let (patch_bytes, _) = crate::commands::read_stdin_content_cancellable(
        stdin,
        Some(max_stdin),
        crate::commands::StdinCapOrigin::GlobalMaxFilesize,
        true,
        None,
    )?;
    let patch = String::from_utf8(patch_bytes).map_err(|e| AtomwriteError::InvalidInput {
        reason: format!("apply: patch on stdin is not valid UTF-8: {e}"),
    })?;

    let format = match args.format {
        PatchFormat::Auto => detect_format(&patch).ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "apply: patch carries no recognizable marker (no ```diff fence, no ---/+++ \
                     header, no <<<<<<< SEARCH block); refusing to guess. Pass --format full to \
                     replace the whole file, or send a patch with the three markers"
                .to_string(),
        })?,
        other => other,
    };

    let targets = resolve_targets(args.file.as_deref(), &patch, format, &workspace)?;

    // Every target is prepared before the first write: a block that does not
    // match refuses the whole call, so half of a patch never lands.
    let mut prepared = Vec::with_capacity(targets.len());
    for (target, body) in &targets {
        match prepare_target(target, body, format, args, global, fuzzy_cfg, write_cfg) {
            Ok(item) => prepared.push(item),
            Err(error) => {
                // The refusal names the file it was about. A patch that names
                // its own files has no single target on the invocation line,
                // and the envelope must still say which one was refused.
                remember_refused_target(target);
                return Err(error);
            }
        }
    }

    if args.dry_run {
        for item in &prepared {
            // A patch that names its own files has one target per event, so
            // the envelope carries the file this record is about.
            writer.set_target(item.target.display().to_string(), target_source);
            writer.write_event(&crate::ndjson_types::ApplyPlan {
                r#type: "plan",
                operation: "apply",
                path: item.target.display().to_string(),
                format_detected: item.format_name.to_owned(),
                hunks: item.hunks,
            })?;
        }
        return Ok(());
    }

    let start = Instant::now();
    let mut committed: Vec<&Prepared> = Vec::with_capacity(prepared.len());
    // The writer's verdict travels with the target that produced it, so the
    // event can say which strategy ran and why a copy is missing. Dropping it
    // is what left `apply` silent about a backup the directory refused.
    let mut outcomes: Vec<crate::atomic::WriteResult> = Vec::with_capacity(prepared.len());
    for item in &prepared {
        match commit_target(item, args, &workspace, &resolved_backup) {
            Ok(outcome) => outcomes.push(outcome),
            Err(error) => {
                // A failed write may have created its copy before it failed.
                // The path travels on the error, and naming it here is what
                // keeps the caller from listing the directory to find it.
                let failed_backup = error
                    .downcast_ref::<crate::atomic::WriteFailure>()
                    .map(|failure| failure.backup_path.clone());
                return Err(rollback_after_failure(
                    &committed,
                    &outcomes,
                    failed_backup.as_deref(),
                    &workspace,
                    writer,
                    target_source,
                    error,
                ));
            }
        }
        committed.push(item);
    }

    for (item, outcome) in prepared.iter().zip(outcomes) {
        writer.set_target(item.target.display().to_string(), target_source);
        writer.write_event(&ApplyResult {
            r#type: "applied",
            path: item.target.display().to_string(),
            format_detected: item.format_name.to_owned(),
            hunks_applied: item.hunks as u64,
            line: item.first_line,
            bytes_before: item.original.len() as u64,
            bytes_after: item.result.len() as u64,
            checksum_before: item.checksum_before.clone(),
            checksum_after: item.checksum_after.clone(),
            write_strategy: outcome.write_strategy,
            backup_path: outcome.backup_path,
            backup_skipped: outcome.backup_skipped,
            elapsed_ms: start.elapsed().as_millis() as u64,
            changed: if args.show {
                changed_snippet(&item.original, &item.result)
            } else {
                None
            },
        })?;
    }

    Ok(())
}

/// One target of an `apply` run, prepared and locked but not yet written.
struct Prepared {
    /// Validated path of the file the patch is meant for.
    target: PathBuf,
    /// Content read under the lock, before the patch.
    original: String,
    /// Content the patch produces.
    result: String,
    /// Number of hunks or blocks the format reported.
    hunks: usize,
    /// 1-based line the first hunk landed on, when the format names one.
    first_line: Option<usize>,
    /// Format name published in the envelope.
    format_name: &'static str,
    /// BLAKE3 of `original`.
    checksum_before: String,
    /// BLAKE3 of `result`.
    checksum_after: String,
    /// The lock, held from the read to the commit.
    _lock: Option<crate::lock::LockGuard>,
}

/// Name published for a patch format.
fn format_name(format: PatchFormat) -> &'static str {
    match format {
        PatchFormat::Unified => "unified",
        PatchFormat::SearchReplace => "search-replace",
        PatchFormat::Full => "full",
        PatchFormat::Markdown => "markdown",
        PatchFormat::Auto => "auto",
    }
}

/// Resolve every file a run touches.
///
/// The target is argv when `--file` names it. Otherwise the path line inside the
/// patch decides, and that line arrived on standard input, so the envelope
/// reports the stream as the origin instead of a line the caller never wrote.
/// Paths are sorted so two runs that touch the same files take the same locks
/// in the same order.
fn resolve_targets(
    file: Option<&Path>,
    patch: &str,
    format: PatchFormat,
    workspace: &Path,
) -> Result<Vec<(PathBuf, String)>> {
    if let Some(file) = file {
        let target = crate::path_safety::validate_path(file, workspace)?;
        return Ok(vec![(target, patch.to_string())]);
    }
    let sections =
        split_patch_targets(patch, format).ok_or_else(|| AtomwriteError::InvalidInput {
            reason: if matches!(format, PatchFormat::Full) {
                "apply: --format full replaces one file, so it needs the target as an argument"
                    .to_string()
            } else {
                "apply: no target argument and no file named in the patch; pass the file, or put \
             a path line before each file's blocks (`--- a/path` and `+++ b/path` for unified, \
             or a bare path line just above `<<<<<<< SEARCH` for search-replace)"
                    .to_string()
            },
        })?;
    let mut seen = std::collections::HashSet::with_capacity(sections.len());
    let mut targets = Vec::with_capacity(sections.len());
    for (path, body) in sections {
        let target = crate::path_safety::validate_path(Path::new(&path), workspace)?;
        if !seen.insert(target.clone()) {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "apply: the patch names {} twice; one section per file",
                    target.display()
                ),
            }
            .into());
        }
        targets.push((target, body));
    }
    targets.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(targets)
}

/// Split a patch that names its own files into (path, body) sections.
fn split_patch_targets(patch: &str, format: PatchFormat) -> Option<Vec<(String, String)>> {
    match format {
        PatchFormat::Unified => split_by_header_pairs(patch),
        PatchFormat::Markdown => split_by_header_pairs(&strip_markdown_fences(patch)),
        PatchFormat::SearchReplace => split_by_path_lines(patch),
        PatchFormat::Full | PatchFormat::Auto => None,
    }
}

/// Sections delimited by a `--- a/path` line followed by a `+++ b/path` line.
fn split_by_header_pairs(patch: &str) -> Option<Vec<(String, String)>> {
    let lines: Vec<&str> = patch.lines().collect();
    let mut starts: Vec<(usize, String)> = Vec::new();
    for index in 0..lines.len().saturating_sub(1) {
        if !lines[index].starts_with("--- ") {
            continue;
        }
        let Some(new_side) = lines[index + 1].strip_prefix("+++ ") else {
            continue;
        };
        if let Some(path) = diff_header_path(new_side) {
            starts.push((index, path));
        }
    }
    if starts.is_empty() {
        return None;
    }
    let mut sections = Vec::with_capacity(starts.len());
    for (position, (start, path)) in starts.iter().enumerate() {
        let end = starts
            .get(position + 1)
            .map_or(lines.len(), |(next, _)| *next);
        sections.push((path.clone(), lines[*start..end].join("\n")));
    }
    Some(sections)
}

/// Path of a diff header, without the timestamp column or the `b/` prefix.
fn diff_header_path(raw: &str) -> Option<String> {
    let token = raw.split('\t').next().unwrap_or(raw).trim();
    let token = token.strip_prefix("b/").unwrap_or(token);
    (!token.is_empty()).then(|| token.to_string())
}

/// Sections delimited by a bare path line just above each first `<<<<<<<`.
fn split_by_path_lines(patch: &str) -> Option<Vec<(String, String)>> {
    let lines: Vec<&str> = patch.lines().collect();
    let mut starts: Vec<(usize, String)> = Vec::new();
    let mut last_text: Option<usize> = None;
    for (index, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("<<<<<<<") {
            if let Some(candidate) = last_text {
                starts.push((candidate, lines[candidate].trim().to_string()));
            }
            last_text = None;
            continue;
        }
        if !line.trim().is_empty() {
            last_text = Some(index);
        }
    }
    if starts.is_empty() {
        return None;
    }
    let mut sections = Vec::with_capacity(starts.len());
    for (position, (start, path)) in starts.iter().enumerate() {
        let end = starts
            .get(position + 1)
            .map_or(lines.len(), |(next, _)| *next);
        let body = (*start + 1..end)
            .map(|line| lines[line])
            .collect::<Vec<&str>>()
            .join("\n");
        sections.push((path.clone(), body));
    }
    Some(sections)
}

/// Read, patch and validate one target without writing it.
fn prepare_target(
    target: &Path,
    patch: &str,
    format: PatchFormat,
    args: &ApplyArgs,
    global: &GlobalArgs,
    fuzzy_cfg: &crate::config::FuzzySection,
    write_cfg: &crate::config::WriteSection,
) -> Result<Prepared> {
    // A1: lock BEFORE reading an existing target, hold it to the commit.
    let lock = if target.exists() {
        Some(crate::lock::acquire_for_update(target)?)
    } else {
        None
    };
    let original = if target.exists() {
        crate::file_io::read_file_string(target, global.effective_max_filesize())?
    } else if matches!(format, PatchFormat::Full) {
        String::new()
    } else {
        return Err(AtomwriteError::NotFound {
            path: target.to_path_buf(),
        }
        .into());
    };

    let checksum_before = checksum::hash_bytes(original.as_bytes());

    let (result, hunks, first_line) = match format {
        PatchFormat::Unified => apply_unified(&original, patch)?,
        PatchFormat::SearchReplace => {
            let (result, hunks) = apply_search_replace_with(&original, patch, fuzzy_cfg, args.all)?;
            (result, hunks, None)
        }
        PatchFormat::Full => (patch.to_string(), 1, Some(1)),
        PatchFormat::Markdown => {
            let stripped = strip_markdown_fences(patch);
            apply_unified(&original, &stripped)?
        }
        PatchFormat::Auto => {
            return Err(AtomwriteError::InternalError {
                reason: "apply: patch format still Auto after detection".to_string(),
            }
            .into());
        }
    };

    // Same two barriers as `write`: a whole-file replacement is exactly the
    // case they exist for, and a patch that happens to shrink the file must
    // say so out loud too.
    // The size barrier exists for the whole-file replacement it was written
    // for. A SEARCH/REPLACE patch shrinks a file by design when it deletes
    // lines, and demanding `--allow-shrink` for that refused patches that were
    // exactly what the caller asked for.
    if matches!(format, PatchFormat::Full) {
        crate::commands::write_guards::check_replacement_size(
            target,
            original.len() as u64,
            result.len() as u64,
            write_cfg,
            args.ack_overwrite,
            args.allow_shrink,
        )?;
    }

    // A patch that leaves MORE errors than the file already had is not
    // written: the caller would learn it from a later error naming a line the
    // patch already moved. A file that arrived broken may still be edited.
    if !args.no_syntax_check && !args.dry_run {
        let before = crate::commands::outline::parse_error_count(target, original.as_bytes());
        let after = crate::commands::outline::parse_error_count(target, result.as_bytes());
        if after > before {
            // The refusal names the block that grew the error count, so the
            // caller edits that block instead of re-reading the whole file.
            let span = crate::commands::outline::first_error_span(target, result.as_bytes());
            return Err(crate::error::AtomwriteError::SyntaxError {
                path: target.to_path_buf(),
                count: after.saturating_sub(before) as u32,
                line: span.map(|(start, _)| start),
                line_end: span.map(|(_, end)| end),
            }
            .into());
        }
    }

    Ok(Prepared {
        target: target.to_path_buf(),
        checksum_after: checksum::hash_bytes(result.as_bytes()),
        original,
        result,
        hunks,
        first_line,
        format_name: format_name(format),
        checksum_before,
        _lock: lock,
    })
}

/// Write one prepared target.
fn commit_target(
    item: &Prepared,
    args: &ApplyArgs,
    workspace: &Path,
    resolved_backup: &crate::commands::ResolvedBackup,
) -> Result<crate::atomic::WriteResult> {
    let opts = AtomicWriteOptions {
        backup: resolved_backup.backup,
        // The parse is compared before this call, and the atomic layer's own
        // check refuses any error, which would block a file that already had
        // one forever.
        syntax_check: false,
        retention: resolved_backup.retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: args.strict.strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup: resolved_backup.keep,
        durability: crate::platform::Durability::Auto,
    };
    // The patch was applied to `original`. Passing its digest makes the writer
    // refuse a file that changed since that read, which is the safety net on
    // filesystems where the advisory lock does not exist.
    let base = item
        .target
        .exists()
        .then_some(item.checksum_before.as_str());
    crate::atomic::atomic_write_based(&item.target, item.result.as_bytes(), &opts, workspace, base)
}

/// What the rollback managed to undo.
struct RollbackOutcome {
    /// Files whose content was put back.
    restored: u64,
    /// Files the rollback could not restore, in the order it tried them.
    failed_paths: Vec<String>,
    /// Time the whole restoration took, in milliseconds.
    elapsed_ms: u64,
}

/// Restore every target this run already wrote.
///
/// A failure here never hides the failure that caused the rollback: the files
/// that come back are restored, and the ones that do not are named in the
/// returned outcome instead of only reaching the log.
fn rollback_committed(committed: &[&Prepared], workspace: &Path) -> RollbackOutcome {
    let start = Instant::now();
    let mut restored: u64 = 0;
    let mut failed_paths: Vec<String> = Vec::new();
    // The rollback writes content this run produced. Taking a backup of that
    // content would create and delete a full copy per file, so the writer runs
    // without one.
    let opts = AtomicWriteOptions {
        backup: false,
        ..AtomicWriteOptions::default()
    };
    for item in committed.iter().rev() {
        // The run still holds the lock it took before reading, and that lock is
        // on the inode the commit replaced. Passing the content digest makes the
        // writer compare bytes instead of inodes, which is what a restoration
        // needs: the file changed on purpose, and only a second writer between
        // the commit and this call can make the digest differ.
        let expected = crate::checksum::hash_file(&item.target, u64::MAX).ok();
        match crate::atomic::atomic_write_based(
            &item.target,
            item.original.as_bytes(),
            &opts,
            workspace,
            expected.as_deref(),
        ) {
            Ok(_) => restored += 1,
            Err(error) => {
                tracing::error!(
                    path = %item.target.display(),
                    %error,
                    "apply rollback could not restore the file"
                );
                failed_paths.push(item.target.display().to_string());
            }
        }
    }
    RollbackOutcome {
        restored,
        failed_paths,
        elapsed_ms: start.elapsed().as_millis() as u64,
    }
}

/// Undo the files this run already wrote and report the undo.
///
/// The record goes through `publish_failure`, so it carries `ok: false` and the
/// very `code` and `exit` of the failure that forced the rollback, and `main`
/// writes no second envelope. A run that failed before writing anything has
/// nothing to undo and publishes no record, because the error envelope alone
/// already says the call failed.
#[allow(clippy::too_many_arguments)]
fn rollback_after_failure(
    committed: &[&Prepared],
    outcomes: &[crate::atomic::WriteResult],
    failed_backup: Option<&Path>,
    workspace: &Path,
    writer: &mut NdjsonWriter<impl Write>,
    target_source: &'static str,
    failure: anyhow::Error,
) -> anyhow::Error {
    if committed.is_empty() {
        return failure;
    }
    let outcome = rollback_committed(committed, workspace);
    // A copy made for a file this run committed survives the rollback, because
    // the rollback restores content and removes nothing. The copy of the file
    // that failed survives too, and it is the one the caller needs first.
    // Naming them is what keeps the caller from listing the directory.
    let mut backups_preserved: Vec<String> = Vec::new();
    let candidates = outcomes
        .iter()
        .filter_map(|o| o.backup_path.as_deref())
        .map(str::to_owned)
        .chain(failed_backup.map(|path| path.display().to_string()));
    for backup in candidates {
        if std::path::Path::new(&backup).exists() && !backups_preserved.iter().any(|p| p == &backup)
        {
            backups_preserved.push(backup);
        }
    }
    // The rollback restores several files, and the caller has to find each one
    // in the envelope. The list is always an array, because the caller cannot
    // know how many files came back before reading the field.
    writer.set_targets(
        committed
            .iter()
            .map(|item| item.target.display().to_string())
            .collect(),
        target_source,
    );
    let err: anyhow::Error = if outcome.failed_paths.is_empty() {
        failure
    } else {
        AtomwriteError::TransactionRollbackFailed {
            reason: format!(
                "apply could not restore {}: {failure:#}",
                outcome.failed_paths.join(", ")
            ),
        }
        .into()
    };
    writer.publish_failure(
        &ApplyRollbackEvent {
            r#type: "rollback",
            operation: "apply",
            files_restored: outcome.restored,
            elapsed_ms: outcome.elapsed_ms,
            failed_paths: outcome.failed_paths,
            backups_preserved,
        },
        err,
    )
}

/// One line of a file or patch, with its terminator kept out of `text`.
struct Line<'a> {
    /// Line text without the `\n`.
    text: &'a str,
    /// Whether the source line ended with `\n`.
    terminated: bool,
}

/// Split `text` into lines that remember their terminator.
///
/// `split_inclusive` keeps the `\n`, so a range built from these lines can
/// delete a whole line instead of leaving its blank remainder behind.
fn tokenize(text: &str) -> Vec<Line<'_>> {
    text.split_inclusive('\n')
        .map(|chunk| match chunk.strip_suffix('\n') {
            Some(text) => Line {
                text,
                terminated: true,
            },
            None => Line {
                text: chunk,
                terminated: false,
            },
        })
        .collect()
}

/// Compare two line texts, ignoring the `\r` of a CRLF file.
fn same_text(left: &str, right: &str) -> bool {
    left.strip_suffix('\r').unwrap_or(left) == right.strip_suffix('\r').unwrap_or(right)
}

/// Byte offset of the first byte of a tokenized line.
fn line_start(lines: &[Line<'_>], index: usize) -> usize {
    lines[..index]
        .iter()
        .map(|line| line.text.len() + usize::from(line.terminated))
        .sum()
}

/// Byte offset just past the terminator of a tokenized line.
fn line_end(lines: &[Line<'_>], index: usize) -> usize {
    line_start(lines, index) + lines[index].text.len() + usize::from(lines[index].terminated)
}

/// Byte ranges where `needle` occupies whole lines of `haystack`.
///
/// The comparison is per line: a needle that is a fragment of a longer line
/// produces no range at all, which is the whole point of the rule. A needle
/// line that is not the last one must be terminated in the file too, because
/// two terminated lines are what makes the match a whole-line match.
fn whole_line_ranges(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    let have = tokenize(haystack);
    let want = tokenize(needle);
    if want.is_empty() || want.len() > have.len() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for start in 0..=have.len() - want.len() {
        let matches = want.iter().enumerate().all(|(offset, wanted)| {
            let line = &have[start + offset];
            same_text(line.text, wanted.text)
                && (offset + 1 == want.len() || line.terminated == wanted.terminated)
        });
        if matches {
            let end = line_end(&have, start + want.len() - 1);
            hits.push((line_start(&have, start), end));
        }
    }
    hits
}

/// Byte ranges a SEARCH block with a `...` wildcard covers.
///
/// Every anchor line must occur exactly once in the file, in the pattern's
/// order, and the gap a wildcard stands for may not exceed `max_skipped`
/// lines. `Ok` with no range means an anchor is absent, which the caller
/// reports as a miss for the whole patch.
fn ellipsis_ranges(
    haystack: &str,
    needle: &str,
    max_skipped: usize,
) -> Result<Vec<(usize, usize)>, AtomwriteError> {
    let marker = crate::constants::PATCH_ELLIPSIS_MARKER;
    let have = tokenize(haystack);
    let want: Vec<&str> = needle.split('\n').collect();
    let first_is_anchor = want.first().is_some_and(|line| line.trim() != marker);
    let last_is_anchor = want.last().is_some_and(|line| line.trim() != marker);
    if !first_is_anchor || !last_is_anchor {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "a `{marker}` wildcard needs a SEARCH line above and below it; \
                 name the anchoring line on both sides"
            ),
        });
    }
    // (pattern line index, file line index) of every anchor.
    let mut positions: Vec<(usize, usize)> = Vec::new();
    for (pattern_index, anchor) in want.iter().enumerate() {
        if anchor.trim() == marker {
            continue;
        }
        let found: Vec<usize> = have
            .iter()
            .enumerate()
            .filter(|(_, line)| same_text(line.text, anchor))
            .map(|(index, _)| index)
            .collect();
        if found.is_empty() {
            return Ok(Vec::new());
        }
        if found.len() > 1 {
            let lines = found
                .iter()
                .map(|index| (index + 1).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(AtomwriteError::MatchAmbiguous {
                reason: format!(
                    "anchor {anchor:?} matches {} lines ({lines}); name a longer anchor",
                    found.len()
                ),
                count: found.len() as u64,
                best_candidate: None,
            });
        }
        positions.push((pattern_index, found[0]));
    }
    let mut skipped = 0usize;
    for pair in positions.windows(2) {
        let (left_pattern, left_line) = pair[0];
        let (right_pattern, right_line) = pair[1];
        if right_line <= left_line {
            return Ok(Vec::new());
        }
        if right_pattern == left_pattern + 1 {
            // No wildcard between these anchors: they must be adjacent.
            if right_line != left_line + 1 {
                return Ok(Vec::new());
            }
        } else {
            skipped += right_line - left_line - 1;
        }
    }
    if skipped > max_skipped {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "the `{marker}` wildcard would skip {skipped} lines, above the cap of \
                 {max_skipped} (apply.max_ellipsis_lines); nothing was written"
            ),
        });
    }
    let Some((_, first_line)) = positions.first().copied() else {
        return Ok(Vec::new());
    };
    let Some((_, last_line)) = positions.last().copied() else {
        return Ok(Vec::new());
    };
    Ok(vec![(
        line_start(&have, first_line),
        line_end(&have, last_line),
    )])
}

/// The lines a patch changed, numbered, for `--show`.
///
/// The first differing line anchors the window, and the window is short on
/// purpose: the answer is the evidence that the swap happened, not a copy of
/// the file. `None` means the patch changed nothing, which is worth knowing.
fn changed_snippet(original: &str, result: &str) -> Option<String> {
    let old: Vec<&str> = original.lines().collect();
    let new: Vec<&str> = result.lines().collect();
    let mut first = 0usize;
    while first < old.len() && first < new.len() && old[first] == new[first] {
        first += 1;
    }
    if first >= old.len() && first >= new.len() {
        return None;
    }
    let start = first.saturating_sub(1);
    let end = (first + 7).min(new.len());
    let mut out = String::new();
    for (offset, line) in new[start..end].iter().enumerate() {
        out.push_str(&format!("{}:{line}\n", start + offset + 1));
    }
    Some(out)
}

/// Classify a patch that came without an explicit `--format`.
///
/// Returns `None` when no marker identifies the format. The caller turns that
/// `None` into a refusal that names `--format full`: guessing "replace the
/// whole file" from an unrecognized body is what turned a malformed patch into
/// silent data loss, and a patch with no markers is exactly the case where the
/// user may have meant a whole-file write.
fn detect_format(patch: &str) -> Option<PatchFormat> {
    let head: String = {
        let mut buf = String::with_capacity(crate::constants::PATCH_FORMAT_PROBE_CAPACITY);
        for (i, line) in patch
            .lines()
            .take(crate::constants::PATCH_FORMAT_PROBE_LINES)
            .enumerate()
        {
            if i > 0 {
                buf.push('\n');
            }
            buf.push_str(line);
        }
        buf
    };
    // Order matters: a fenced markdown patch also carries `---`/`+++`, so the
    // fence is tested first or every markdown diff would be read as unified.
    if head.contains("```diff") {
        Some(PatchFormat::Markdown)
    } else if head.contains("--- ") && head.contains("+++ ") {
        // A diff header means the author wrote a diff. Without a `@@` hunk
        // there is nothing to apply, and falling through to a whole-file
        // write would overwrite the target with the patch text.
        // Classified as unified so the strict parser rejects it.
        Some(PatchFormat::Unified)
    } else if head.contains("<<<<<<< SEARCH") {
        Some(PatchFormat::SearchReplace)
    } else {
        // No marker at all: refuse, and let the caller name `--format full`.
        None
    }
}

include!("apply_unified.inc.rs");

#[cfg(test)]
fn apply_search_replace(
    original: &str,
    patch: &str,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<(String, usize)> {
    apply_search_replace_with(original, patch, fuzzy_cfg, false)
}

/// Apply a SEARCH/REPLACE patch, optionally replacing every occurrence.
///
/// # Errors
///
/// Returns [`AtomwriteError::InvalidInput`] for a block without its separator
/// or terminator, an empty SEARCH, or a SEARCH that only matches inside a
/// larger line. Returns [`AtomwriteError::MatchAmbiguous`] when a block matches
/// more than one whole line and `replace_all` is false.
fn apply_search_replace_with(
    original: &str,
    patch: &str,
    fuzzy_cfg: &crate::config::FuzzySection,
    replace_all: bool,
) -> Result<(String, usize)> {
    let mut result = original.to_owned();
    let mut blocks = 0usize;
    // 1-based numbers of the blocks whose SEARCH text is not in the file, and
    // the near-miss of the first one.
    let mut unmatched: Vec<usize> = Vec::new();
    let mut first_miss: Option<Box<crate::ndjson_types::BestCandidate>> = None;

    let mut i = 0;
    let lines: Vec<&str> = patch.lines().collect();

    while i < lines.len() {
        if lines[i].starts_with("<<<<<<< SEARCH") {
            let search_start = i + 1;
            let mut sep = search_start;
            while sep < lines.len() && lines[sep] != "=======" {
                sep += 1;
            }
            // A block that is cut short used to be dropped in silence, so the
            // run said "applied" while one of its blocks never ran. The block
            // number is the fact the caller needs to fix the patch.
            if sep >= lines.len() {
                return Err(AtomwriteError::InvalidInput {
                    reason: format!(
                        "block {} has no `=======` separator; nothing was written",
                        blocks + 1
                    ),
                }
                .into());
            }
            let replace_start = sep + 1;
            let mut replace_end = replace_start;
            while replace_end < lines.len() && !lines[replace_end].starts_with(">>>>>>> REPLACE") {
                replace_end += 1;
            }
            if replace_end >= lines.len() {
                return Err(AtomwriteError::InvalidInput {
                    reason: format!(
                        "block {} has no `>>>>>>> REPLACE` terminator; nothing was written",
                        blocks + 1
                    ),
                }
                .into());
            }

            let search_text = lines[search_start..sep].join("\n");
            let replace_text = lines[replace_start..replace_end].join("\n");
            // An empty SEARCH matches at offset zero in every file, which is
            // never what a patch means. Whole-file replacement has its own
            // verb, and the hint names it.
            if search_text.is_empty() {
                return Err(AtomwriteError::InvalidInput {
                    reason: format!(
                        "block {} has an empty SEARCH; use `write` to replace the whole file",
                        blocks + 1
                    ),
                }
                .into());
            }

            blocks += 1;
            // `...` alone in the REPLACE is not a wildcard: the wildcard says
            // "any lines here" in a SEARCH, and in a REPLACE it would write
            // three literal dots where the caller meant to keep the file.
            if replace_text.trim() == crate::constants::PATCH_ELLIPSIS_MARKER {
                return Err(AtomwriteError::InvalidInput {
                    reason: format!(
                        "block {blocks} has a bare `{}` in its REPLACE; the wildcard only stands \
                         in a SEARCH",
                        crate::constants::PATCH_ELLIPSIS_MARKER
                    ),
                }
                .into());
            }
            let has_ellipsis = search_text
                .split('\n')
                .any(|line| line.trim() == crate::constants::PATCH_ELLIPSIS_MARKER);
            let hits = if has_ellipsis {
                ellipsis_ranges(
                    &result,
                    &search_text,
                    crate::config::apply_max_ellipsis_lines(),
                )?
            } else {
                whole_line_ranges(&result, &search_text)
            };
            if !hits.is_empty() {
                if hits.len() > 1 && !replace_all {
                    return Err(AtomwriteError::MatchAmbiguous {
                        reason: format!(
                            "block {blocks} matches {} whole lines; name one line more, or pass --all",
                            hits.len()
                        ),
                        count: hits.len() as u64,
                        best_candidate: None,
                    }
                    .into());
                }
                // Replace from the end so earlier offsets stay valid.
                for (start, end) in hits.iter().rev() {
                    let mut insert = replace_text.clone();
                    // An empty REPLACE deletes the whole matched lines. Adding
                    // the separator back would leave a blank line where the
                    // caller asked for nothing.
                    if !insert.is_empty()
                        && *end > *start
                        && result.as_bytes().get(*end - 1) == Some(&b'\n')
                        && !insert.ends_with('\n')
                    {
                        insert.push('\n');
                    }
                    result.replace_range(*start..*end, &insert);
                }
                i = replace_end + 1;
                continue;
            }
            // A wildcard pattern that found no anchor is a miss; the fuzzy
            // cascade cannot help because `...` is not text of the file.
            if has_ellipsis {
                unmatched.push(blocks);
                i = replace_end + 1;
                continue;
            }
            // Text that exists only as part of a longer line is the wrong
            // match: `old` would swap the middle of `older_name`. Saying so is
            // cheaper than writing and letting the caller discover it.
            if result.contains(&search_text) {
                return Err(AtomwriteError::InvalidInput {
                    reason: format!(
                        "block {blocks} matches text inside a larger line; write the complete line"
                    ),
                }
                .into());
            }
            // Near-miss (whitespace, indentation): the fuzzy cascade decides.
            match crate::fuzzy::match_pair_cfg(
                &result,
                &search_text,
                &replace_text,
                crate::cli_args::FuzzyMode::Auto,
                None,
                fuzzy_cfg,
                false,
            ) {
                Ok((edited, _info)) => result = edited,
                // Only "not found" makes a block missing; a cancel, an
                // oversize refusal or an ambiguous match reaches the caller.
                Err(AtomwriteError::MatchFailed { best_candidate, .. }) => {
                    unmatched.push(blocks);
                    if first_miss.is_none() {
                        first_miss = best_candidate;
                    }
                }
                Err(other) => return Err(other.into()),
            }

            i = if replace_end < lines.len() {
                replace_end + 1
            } else {
                lines.len()
            };
        } else {
            i += 1;
        }
    }

    if blocks == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "no SEARCH/REPLACE blocks found in patch".into(),
        }
        .into());
    }

    if !unmatched.is_empty() {
        return Err(AtomwriteError::MatchFailed {
            reason: format!(
                "{} of {blocks} SEARCH/REPLACE blocks did not match the file (block {:?}); \
                 nothing was written",
                unmatched.len(),
                unmatched
            ),
            best_candidate: first_miss,
            candidates: None,
        }
        .into());
    }

    Ok((result, blocks))
}

fn strip_markdown_fences(patch: &str) -> String {
    let mut in_block = false;
    let mut result = Vec::with_capacity(patch.lines().count());

    for line in patch.lines() {
        if line.starts_with("```diff") {
            in_block = true;
            continue;
        }
        if in_block && line.starts_with("```") {
            in_block = false;
            continue;
        }
        if in_block {
            result.push(line);
        }
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_format_unified() {
        let patch = "--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,3 @@\n";
        assert!(matches!(detect_format(patch), Some(PatchFormat::Unified)));
    }

    #[test]
    fn detect_format_search_replace() {
        let patch = "<<<<<<< SEARCH\nold text\n=======\nnew text\n>>>>>>> REPLACE\n";
        assert!(matches!(
            detect_format(patch),
            Some(PatchFormat::SearchReplace)
        ));
    }

    #[test]
    fn detect_format_markdown() {
        let patch = "```diff\n--- a/file.txt\n+++ b/file.txt\n```\n";
        assert!(matches!(detect_format(patch), Some(PatchFormat::Markdown)));
    }

    #[test]
    fn detect_format_refuses_a_body_without_markers() {
        // A marker-less body used to classify as `Full`, which replaced the
        // target with the patch text. It now yields `None`, and the caller
        // refuses unless the user passed `--format full`.
        let patch = "this is just plain text content\nno markers at all\n";
        assert!(detect_format(patch).is_none());
    }

    #[test]
    fn strip_markdown_fences_extracts_content() {
        let input = "some preamble\n```diff\n--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new\n```\nafter";
        let result = strip_markdown_fences(input);
        assert!(result.contains("--- a"));
        assert!(!result.contains("```"));
    }

    #[test]
    fn apply_search_replace_basic() {
        let original = "hello world\nfoo bar\n";
        let patch = "<<<<<<< SEARCH\nhello world\n=======\nhello universe\n>>>>>>> REPLACE\n";
        let (result, blocks) =
            apply_search_replace(original, patch, &crate::config::FuzzySection::default()).unwrap();
        assert_eq!(blocks, 1);
        assert!(result.contains("hello universe"));
    }

    #[test]
    fn apply_search_replace_no_blocks_fails() {
        let original = "hello world\n";
        let patch = "no search replace blocks here\n";
        let result = apply_search_replace(original, patch, &crate::config::FuzzySection::default());
        assert!(result.is_err());
    }

    fn mismatch_reason(result: Result<(String, usize, Option<usize>)>) -> String {
        match result.unwrap_err().downcast::<AtomwriteError>().unwrap() {
            AtomwriteError::MatchFailed { reason, .. } => reason,
            other => panic!("expected MatchFailed, got {other:?}"),
        }
    }

    #[test]
    fn unified_rejects_wrong_removed_line() {
        let patch = "--- a\n+++ b\n@@ -1 +1 @@\n-zzz\n+yyy\n";
        let reason = mismatch_reason(apply_unified("alpha\nbeta\ngamma\n", patch));
        assert!(reason.contains("hunk 1/1"), "{reason}");
        assert!(reason.contains("zzz"), "{reason}");
        assert!(reason.contains("alpha"), "{reason}");
    }

    #[test]
    fn unified_rejects_wrong_context() {
        let patch = "--- a\n+++ b\n@@ -1,3 +1,3 @@\n qqq\n-beta\n+yyy\n rrr\n";
        let reason = mismatch_reason(apply_unified("alpha\nbeta\ngamma\n", patch));
        assert!(reason.contains("qqq"), "{reason}");
    }

    #[test]
    fn unified_second_hunk_failure_applies_nothing() {
        let patch = "--- a\n+++ b\n@@ -1 +1 @@\n-alpha\n+ALPHA\n@@ -3 +3 @@\n-nope\n+x\n";
        let reason = mismatch_reason(apply_unified("alpha\nbeta\ngamma\n", patch));
        assert!(reason.contains("hunk 2/2"), "{reason}");
    }

    #[test]
    fn unified_tolerates_shifted_header_and_trailing_space() {
        let original = "one\ntwo\nbeta  \nfour\n";
        let patch = "--- a\n+++ b\n@@ -9 +9 @@\n-beta\n+BETA\n";
        let (out, hunks, _) = apply_unified(original, patch).unwrap();
        assert_eq!(hunks, 1);
        assert_eq!(out, "one\ntwo\nBETA\nfour\n");
    }

    #[test]
    fn unified_rejects_a_hunk_beyond_the_shift_window() {
        let far = crate::constants::PATCH_MAX_HUNK_SHIFT + 5;
        let mut original = String::new();
        for i in 0..far {
            original.push_str(&format!("l{i}\n"));
        }
        original.push_str("target\n");
        let patch = "--- a\n+++ b\n@@ -1 +1 @@\n-target\n+x\n";
        assert!(mismatch_reason(apply_unified(&original, patch)).contains("hunk 1/1"));
    }

    #[test]
    fn unified_insert_only_hunk_goes_after_the_named_line() {
        let patch = "--- a\n+++ b\n@@ -1,0 +2,1 @@\n+inserted\n";
        let (out, _, _) = apply_unified("alpha\nbeta\n", patch).unwrap();
        assert_eq!(out, "alpha\ninserted\nbeta\n");
    }

    #[test]
    fn unified_without_hunks_is_invalid_input() {
        let err = apply_unified("a\n", "--- a\n+++ b\n").unwrap_err();
        assert!(matches!(
            err.downcast_ref::<AtomwriteError>(),
            Some(AtomwriteError::InvalidInput { .. })
        ));
    }

    #[test]
    fn search_replace_with_a_missing_block_fails_the_whole_patch() {
        let patch = "<<<<<<< SEARCH\nalpha\n=======\nALPHA\n>>>>>>> REPLACE\n\
                     <<<<<<< SEARCH\nCOMPLETELY ABSENT 12345\n=======\nX\n>>>>>>> REPLACE\n";
        let reason = mismatch_reason(
            apply_search_replace(
                "alpha\nbeta\n",
                patch,
                &crate::config::FuzzySection::default(),
            )
            .map(|(result, hunks)| (result, hunks, None)),
        );
        assert!(reason.contains("1 of 2"), "{reason}");
    }

    #[test]
    fn search_replace_lets_a_fuzzy_refusal_through() {
        let fuzzy = crate::config::FuzzySection {
            max_content_bytes: 4,
            ..crate::config::FuzzySection::default()
        };
        let patch = "<<<<<<< SEARCH\nnot in the file\n=======\nX\n>>>>>>> REPLACE\n";
        let err = apply_search_replace("alpha beta gamma\n", patch, &fuzzy).unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<AtomwriteError>(),
                Some(AtomwriteError::InvalidInput { .. })
            ),
            "{err:#}"
        );
    }

    #[test]
    fn apply_unified_simple() {
        let original = "line1\nline2\nline3\n";
        let patch = "--- a/file\n+++ b/file\n@@ -1,3 +1,3 @@\n line1\n-line2\n+modified\n line3\n";
        let (result, hunks, _) = apply_unified(original, patch).unwrap();
        assert_eq!(hunks, 1);
        assert!(result.contains("modified"));
        assert!(!result.contains("line2"));
    }
}
