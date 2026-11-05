// SPDX-License-Identifier: MIT OR Apache-2.0

//! File deletion with optional backup before removal.
//!
//! Workload: I/O-bound (stat + optional hash + unlink + fsync).
//! Parallelism: **all** roots (multi-path files + recursive dirs) expand into
//! one path list via `collect_files_parallel`, then a single `rayon::par_iter`.
//! Empty-dir cleanup discovers candidates via WalkParallel (bottom-up remove
//! stays ordered).
//! (hash/backup/unlink independent per file). NDJSON emits in path-sorted
//! order after the join. Single-file stays sequential (coordination cost >
//! work). Bound: `--threads` / `--max-concurrency` (WalkParallel + rayon).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use rayon::prelude::*;

use crate::checksum;
use crate::cli::{DeleteArgs, GlobalArgs};

use crate::concurrency::should_parallelize;
use crate::error::AtomwriteError;
use crate::ndjson_types::{DeleteOutput, DryRunPlan, FailureSummary, FailureTally, Summary};
use crate::output::NdjsonWriter;
use crate::platform;

fn parse_human_duration(s: &str) -> std::result::Result<Duration, AtomwriteError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: "empty duration string".into(),
        });
    }
    let mut total_secs: u64 = 0;
    let mut num_buf = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else {
            let n: u64 = num_buf.parse().map_err(|_| AtomwriteError::InvalidInput {
                reason: format!("invalid number in duration: {s:?}"),
            })?;
            num_buf.clear();
            let multiplier = match ch {
                's' => 1,
                'm' => crate::constants::SECS_PER_MINUTE,
                'h' => crate::constants::SECS_PER_HOUR,
                'd' => crate::constants::SECS_PER_DAY,
                'w' => crate::constants::SECS_PER_WEEK,
                _ => {
                    return Err(AtomwriteError::InvalidInput {
                        reason: format!("unknown duration suffix '{ch}' in {s:?}; use s/m/h/d/w"),
                    });
                }
            };
            total_secs += n * multiplier;
        }
    }
    if !num_buf.is_empty() {
        let n: u64 = num_buf.parse().map_err(|_| AtomwriteError::InvalidInput {
            reason: format!("invalid number in duration: {s:?}"),
        })?;
        total_secs += n;
    }
    if total_secs == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("duration must be > 0: {s:?}"),
        });
    }
    Ok(Duration::from_secs(total_secs))
}

/// Terminal summary that adds the leftover entry count to the shared one.
///
/// `dirs_left` says how many requested directories survived, but not what is
/// still inside them, so the reader cannot size a follow-up run from it.
#[derive(serde::Serialize)]
struct DeleteSummary<'a> {
    /// Shared terminal counters (`dirs_left`, `hint`, totals).
    #[serde(flatten)]
    summary: &'a FailureSummary,
    /// Entries that remain inside the directories the run could not remove.
    #[serde(skip_serializing_if = "Option::is_none")]
    entries_left: Option<u64>,
}

/// Count the entries still under `root`, nested ones included.
///
/// Only `file_type` is read, so a symlink counts as one entry and is never
/// followed: the number describes what the removal left behind, not what the
/// link points at.
fn count_entries_under(root: &Path) -> u64 {
    let mut total: u64 = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            total = total.saturating_add(1);
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                stack.push(entry.path());
            }
        }
    }
    total
}

/// Whether `delete` owns this directory entry.
///
/// A directory is emptied by the cleanup pass, so it is not collected here.
/// A fifo, socket or device is unlinked without being opened, and only the
/// Unix platforms have those entry kinds.
#[cfg(unix)]
fn is_removable_entry(kind: std::fs::FileType) -> bool {
    use std::os::unix::fs::FileTypeExt;
    kind.is_file()
        || kind.is_symlink()
        || kind.is_fifo()
        || kind.is_socket()
        || kind.is_block_device()
        || kind.is_char_device()
}

/// Whether `delete` owns this directory entry.
#[cfg(not(unix))]
fn is_removable_entry(kind: std::fs::FileType) -> bool {
    kind.is_file() || kind.is_symlink()
}

/// Outcome of processing one delete candidate (for ordered NDJSON emit).
enum DeleteItem {
    SkippedAge,
    /// The entry is gone already: a repeated run reaches the same end state.
    SkippedGone,
    Plan {
        path: String,
        size: u64,
    },
    Deleted {
        path: String,
        size: u64,
        /// `None` for a fifo, socket or device: there is no content to hash.
        hash: Option<String>,
        /// Backup created before the unlink, when the caller asked for one.
        backup: Option<String>,
        /// Why a requested backup was skipped, when the target is itself a copy.
        backup_skipped: Option<String>,
    },
    Err(anyhow::Error),
}

/// Delete files, copying one only when the call asks, with dry-run support.
///
/// The call asks through `--backup` or through `[defaults].backup = true`, and
/// `--no-backup` wins over both. `--keep-backup` never creates the copy.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the target file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if deleting the file fails.
#[tracing::instrument(skip_all, fields(command = "delete"))]
pub fn cmd_delete(
    args: &DeleteArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    // In `lean` the verb answers once, with the totals, instead of repeating a
    // record per file the caller already named.
    let lean = writer.profile() == crate::output::OutputProfile::Lean;
    // A delete removes the file; keeping a copy is a side effect the caller
    // must ask for, through `--backup` or through `[defaults].backup = true`.
    // `--keep-backup` never creates one: it only keeps a copy that already
    // exists.
    // `[delete].backup` wins over `[defaults].backup` for this verb only.
    let silent_backup = crate::config::delete_backup().or(defaults.backup);
    let resolved = crate::commands::resolve_backup_for(
        &args.backup_opts,
        defaults,
        args.backup_opts.backup == Some(true) || silent_backup == Some(true),
    );
    // v0.1.28 GAP-CLI-SURFACE-DRIFT: `--keep-backup` is a silent no-op for
    // delete (deletion backups are never auto-removed); surface it instead
    // of discarding the flag silently.
    let mut deleted = 0u64;
    let mut planned = 0u64;
    let mut bytes_freed = 0u64;
    let mut skipped = 0u64;
    let mut backups_kept = 0u64;
    let mut backup_bytes = 0u64;

    let age_threshold = match &args.older_than {
        Some(dur_str) => Some(parse_human_duration(dur_str)?),
        None => None,
    };

    let mut visited = 0u64;
    let max_size = global.effective_max_filesize();
    // B-005 / B-014: `--confirm` is not plan and does not delete — fail closed.
    // v0.1.36 — `--plan` is named as a required flag field, not only in prose.
    if args.confirm {
        return Err(crate::error::AtomwriteError::MissingFlag {
            flag: "--plan".into(),
            reason: "delete --confirm is rejected (one-shot): list targets without deleting, or omit flags to delete; write --confirm is a different large-file guard"
                .into(),
        }
        .into());
    }
    // B-015: `-y/--yes` is not a no-op — fail closed so agents do not invent confirm UX.
    if args.yes {
        return Err(crate::error::AtomwriteError::MissingFlag {
            flag: "--plan".into(),
            reason: "delete -y/--yes is rejected (one-shot: no interactive confirm exists); list targets, or omit flags to delete"
                .into(),
        }
        .into());
    }
    // A-004: `--plan` / `--dry-run` are plan-only; never mutates.
    let dry_or_confirm = args.dry_run || args.plan;
    let do_backup = resolved.backup;
    let retention = resolved.retention;

    // Phase 1 — expand every root into one file list (fail-fast NotFound).
    // Multi-root dirs use one WalkBuilder + `.add` (docs.rs), then one par_iter.
    let mut files_to_delete: Vec<PathBuf> = Vec::new();
    let mut dir_roots: Vec<PathBuf> = Vec::new();

    for path in &args.paths {
        if crate::signal::is_global_shutdown() {
            break;
        }

        let path = crate::path_safety::validate_delete_root(path, &workspace)?;

        // A link is removed as an entry, never followed: `--recursive` on a
        // link therefore removes the link and leaves the tree it names alone.
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => files_to_delete.push(path),
            Ok(_) if path.is_dir() => {
                if !args.recursive {
                    // v0.1.36 — the required flag travels as its own field so
                    // callers re-run instead of scraping the message.
                    return Err(AtomwriteError::MissingFlag {
                        flag: "--recursive".into(),
                        reason: format!("{} is a directory", path.display()),
                    }
                    .into());
                }
                dir_roots.push(path);
            }
            Ok(_) => files_to_delete.push(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(AtomwriteError::NotFound { path }.into());
            }
            Err(error) => {
                return Err(AtomwriteError::from_io_at(&path, error).into());
            }
        }
    }

    // Overlapping roots (`a` and `a/b`) would clean the same tree twice in
    // parallel, so only the outermost root survives.
    dir_roots.sort();
    dir_roots.dedup();
    let mut outermost: Vec<PathBuf> = Vec::with_capacity(dir_roots.len());
    for root in dir_roots {
        if !outermost.iter().any(|kept| root.starts_with(kept)) {
            outermost.push(root);
        }
    }
    let dir_roots = outermost;

    // A backup copy is written next to the file, so inside a tree this verb is
    // about to remove it would outlive the removal and keep the root on disk.
    // The combination is refused before the first unlink instead of leaving a
    // partial tree behind.
    if do_backup && !dir_roots.is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: "delete --backup cannot remove a directory tree: the copy would be created inside the tree and outlive the removal; copy the tree first or pass --no-backup".into(),
        }
        .into());
    }

    if !dir_roots.is_empty() && (global.hidden || global.no_gitignore || global.follow_symlinks) {
        tracing::warn!(
            "delete owns the named tree: --hidden, --no-gitignore and --follow-symlinks have no effect"
        );
    }

    // A directory the walk cannot open hides files this verb was asked to
    // delete, so the hole is reported in the terminal instead of dropped.
    let failures: std::sync::Arc<std::sync::Mutex<FailureTally>> =
        std::sync::Arc::new(std::sync::Mutex::new(FailureTally::default()));
    if !dir_roots.is_empty() && !crate::signal::is_global_shutdown() {
        // The one shared walker: a delete that honored a different set of
        // globs than the other tree verbs would remove a different tree.
        let walker_builder = crate::commands::build_scoped_walker_flavored(
            &dir_roots,
            global,
            crate::commands::WalkerFlavor::DeleteTree,
            &args.include,
            &args.exclude,
        )?;

        // A link is a directory entry this verb owns like a file. Unfollowed,
        // it never reaches what it points at, and leaving it behind keeps the
        // directory from being removed. `--follow-symlinks` reports the target
        // type here, so a followed link still counts as what it points at.
        files_to_delete.extend(crate::concurrency::collect_mapped_parallel_tallied(
            &walker_builder,
            |entry| {
                entry
                    .file_type()
                    .is_some_and(is_removable_entry)
                    .then(|| entry.path().to_path_buf())
            },
            Some(std::sync::Arc::clone(&failures)),
        ));
    }

    crate::concurrency::sort_paths_parallel(&mut files_to_delete);
    files_to_delete.dedup();
    // `--limit` cuts the list BEFORE the first unlink, so `delete --limit 1`
    // removes exactly one file.
    crate::output::cap_collected(&mut files_to_delete);

    if crate::signal::is_global_shutdown() {
        return Ok(());
    }

    // Phase 2 — fan-out mutations (or sequential when a single file).
    let items: Vec<DeleteItem> = if should_parallelize(files_to_delete.len()) {
        files_to_delete
            .par_iter()
            .map(|file_path| {
                process_one_delete(
                    file_path,
                    age_threshold,
                    max_size,
                    dry_or_confirm,
                    do_backup,
                    retention,
                )
            })
            .collect()
    } else {
        files_to_delete
            .iter()
            .map(|file_path| {
                process_one_delete(
                    file_path,
                    age_threshold,
                    max_size,
                    dry_or_confirm,
                    do_backup,
                    retention,
                )
            })
            .collect()
    };

    for item in items {
        if crate::signal::is_global_shutdown() {
            tracing::info!(visited, deleted, "delete interrupted by signal");
            break;
        }
        match item {
            DeleteItem::SkippedAge | DeleteItem::SkippedGone => {
                visited += 1;
                skipped += 1;
            }
            DeleteItem::Plan {
                path: path_str,
                size,
            } => {
                visited += 1;
                writer.write_event(&DryRunPlan {
                    r#type: "plan",
                    operation: "delete".into(),
                    path: path_str,
                    would_modify: true,
                    details: Some(format!("{size} bytes")),
                    // A delete removes a file and writes no content to assess.
                    risk_assessment: None,
                })?;
                // G-002/G-016: plan must not count as modified.
                planned += 1;
                bytes_freed += size;
            }
            DeleteItem::Deleted {
                path: path_str,
                size,
                hash,
                backup,
                backup_skipped,
            } => {
                visited += 1;
                deleted += 1;
                bytes_freed += size;
                if backup.is_some() {
                    backups_kept += 1;
                    backup_bytes += size;
                }
                // The flag only keeps a copy that exists, so the warning is
                // per file: a target that is itself a backup, or a call that
                // never asked for the copy, has nothing for the flag to keep.
                let mut item_warnings: Vec<String> = Vec::new();
                if args.backup_opts.keep_backup && backup.is_none() {
                    item_warnings.push(
                        crate::commands::backup_names::KEEP_BACKUP_REDUNDANT_FOR_DELETE.to_owned(),
                    );
                }
                if !lean {
                    writer.write_event(&DeleteOutput {
                        r#type: "deleted",
                        path: path_str,
                        bytes: size,
                        checksum_before: hash,
                        // `delete` removes the directory entry and writes no content.
                        write_strategy: "none",
                        backup_skipped,
                        elapsed_ms: start.elapsed().as_millis() as u64,
                        backup_path: backup,
                        warnings: (!item_warnings.is_empty()).then_some(item_warnings),
                    })?;
                }
            }
            DeleteItem::Err(e) => return Err(e),
        }
    }

    // `--limit` and the directory filters narrow the target, so a leftover is
    // what the caller asked for. Anything else that survives is a real
    // failure, and it must not be reported as an argument mistake.
    let narrowed = !args.include.is_empty()
        || !args.exclude.is_empty()
        || age_threshold.is_some()
        || crate::output::limit_reached();

    // Phase 3 — remove emptied directory trees (independent roots fan out).
    // G-002 / A-004: plan never mutates directory trees either.
    let mut cleanup_failures: Vec<(PathBuf, std::io::Error)> = Vec::new();
    let mut first_leftover: Option<PathBuf> = None;
    if !args.dry_run && !args.plan && !crate::signal::is_global_shutdown() {
        let cleanup: Vec<CleanupOutcome> = if should_parallelize(dir_roots.len()) {
            dir_roots
                .par_iter()
                .map(|path| cleanup_dir_root(path, global, &args.exclude, narrowed))
                .collect()
        } else {
            dir_roots
                .iter()
                .map(|path| cleanup_dir_root(path, global, &args.exclude, narrowed))
                .collect()
        };
        for outcome in cleanup {
            cleanup_failures.extend(outcome.failures);
            if first_leftover.is_none() {
                first_leftover = outcome.first_leftover;
            }
        }
    }

    // On cooperative cancel, skip summary so main can emit the shutdown
    // banner + signal exit code (same contract as search/list/count).
    if crate::signal::is_global_shutdown() {
        return Ok(());
    }

    // Cleanup errors are real I/O failures with a path: recording them here
    // makes `errors[]` name the survivor and the exit code come from the
    // measured error instead of a synthetic verdict.
    {
        let mut tally = failures.lock().unwrap_or_else(|error| error.into_inner());
        for (path, error) in cleanup_failures {
            // The walk already named this path: recording the same failure
            // again would count one cause twice in `files_failed`.
            if tally.explains(&path) {
                continue;
            }
            tally.record(
                &path,
                crate::error::AtomwriteError::from_io_at(&path, error).into(),
            );
        }
    }

    // A plan never removed anything, so it has nothing left over to report.
    let surviving_roots: Vec<PathBuf> = if dry_or_confirm {
        Vec::new()
    } else {
        dir_roots
            .iter()
            .filter(|path| path.exists())
            .cloned()
            .collect()
    };
    let dirs_left = surviving_roots.len() as u64;
    let entries_left: u64 = surviving_roots
        .iter()
        .map(|path| count_entries_under(path))
        .sum();
    let leftover_hint = (dirs_left > 0 && narrowed).then(|| {
        "directories kept entries this run was told to skip: rerun without --include, --exclude or --older-than, or with a larger --limit".to_owned()
    });

    // An unexplained leftover in an unfiltered run is a real failure. The
    // tally carries the path, so the terminal line names what survived.
    if dirs_left > 0 && !narrowed {
        let survivor = first_leftover
            .or_else(|| surviving_roots.first().cloned())
            .unwrap_or_else(|| dir_roots[0].clone());
        let mut tally = failures.lock().unwrap_or_else(|error| error.into_inner());
        // A leftover the run already failed to remove is not a second,
        // unexplained verdict: the measured error is the answer.
        if !tally.explains(&survivor) {
            let error = std::io::Error::other(
                "directory still holds entries after the run removed every entry it owned",
            );
            tally.record(
                &survivor,
                crate::error::AtomwriteError::from_io_at(&survivor, error).into(),
            );
        }
    }
    let tally = match std::sync::Arc::try_unwrap(failures) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => std::mem::take(&mut *arc.lock().unwrap_or_else(|e| e.into_inner())),
    };
    let (summary, first) = tally.into_summary(Summary {
        r#type: "summary",
        files_visited: visited,
        files_matched: deleted + planned,
        files_modified: Some(deleted),
        files_skipped: Some(skipped),
        total_matches: None,
        total_replacements: None,
        total_bytes: Some(bytes_freed),
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        elapsed_ms: start.elapsed().as_millis() as u64,
        walker_stopped_early: crate::output::limit_reached().then_some(true),
        backups_kept: (backups_kept > 0).then_some(backups_kept),
        backup_bytes: (backups_kept > 0).then_some(backup_bytes),
        dirs_left: (dirs_left > 0).then_some(dirs_left),
        hint: leftover_hint,
        ..Summary::default()
    });
    let summary = DeleteSummary {
        summary: &summary,
        entries_left: (entries_left > 0).then_some(entries_left),
    };
    if let Some(err) = first {
        return Err(writer.publish_failure(&summary, err));
    }
    writer.write_terminal(&summary)?;

    Ok(())
}

/// What one requested directory left behind after the removal pass.
struct CleanupOutcome {
    failures: Vec<(PathBuf, std::io::Error)>,
    first_leftover: Option<PathBuf>,
}

/// Remove the directories this run emptied, deepest first.
///
/// Only `remove_dir` is used, never `remove_dir_all`: the same walk policy that
/// chose the files also chooses the directories, and a brute-force removal
/// would destroy something `--exclude` kept out while no counter saw it. A
/// directory that still holds an entry stays, and the caller decides whether
/// that leftover was requested.
///
/// `narrowed` says the run was told to skip entries. When it is false, a
/// non-empty directory on another device is a mounted file system, which this
/// verb never crosses and reports instead of emptying.
fn cleanup_dir_root(
    path: &Path,
    global: &crate::cli::GlobalArgs,
    exclude: &[String],
    #[cfg_attr(not(unix), allow(unused_variables))] narrowed: bool,
) -> CleanupOutcome {
    let mut outcome = CleanupOutcome {
        failures: Vec::new(),
        first_leftover: None,
    };
    let roots = [path.to_path_buf()];
    let builder = match crate::commands::build_scoped_walker_flavored(
        &roots,
        global,
        crate::commands::WalkerFlavor::DeleteTree,
        &[],
        exclude,
    ) {
        Ok(builder) => builder,
        Err(error) => {
            outcome
                .failures
                .push((path.to_path_buf(), std::io::Error::other(error.to_string())));
            return outcome;
        }
    };
    let mut dirs_to_remove: Vec<PathBuf> =
        crate::concurrency::collect_mapped_parallel(&builder, |entry| {
            entry
                .file_type()
                .is_some_and(|kind| kind.is_dir())
                .then(|| entry.path().to_path_buf())
        });
    // Deepest paths first ≈ contents_first semantics for remove_dir.
    dirs_to_remove.sort_by(|a, b| {
        let da = a.components().count();
        let db = b.components().count();
        db.cmp(&da).then_with(|| b.cmp(a))
    });
    for dir in &dirs_to_remove {
        match std::fs::read_dir(dir) {
            Ok(mut entries) => {
                if let Some(first) = entries.next() {
                    // Name the surviving entry, not only the directory that
                    // holds it, so the failure points at what is on disk.
                    let survivor = first.map_or_else(|_| dir.clone(), |entry| entry.path());
                    outcome.first_leftover.get_or_insert(survivor);
                    #[cfg(unix)]
                    {
                        if !narrowed && is_other_file_system(path, dir) {
                            outcome.failures.push((
                                dir.clone(),
                                std::io::Error::other(
                                    "directory is a mount point on another file system",
                                ),
                            ));
                        }
                    }
                    continue;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                outcome.failures.push((dir.clone(), error));
                continue;
            }
        }
        match std::fs::remove_dir(dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => outcome.failures.push((dir.clone(), error)),
        }
    }
    outcome
}

/// Whether `dir` lives on another device than `root`.
#[cfg(unix)]
fn is_other_file_system(root: &Path, dir: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (
        std::fs::symlink_metadata(root),
        std::fs::symlink_metadata(dir),
    ) {
        (Ok(root_meta), Ok(dir_meta)) => root_meta.dev() != dir_meta.dev(),
        _ => false,
    }
}

/// Remove a link, never what it points at. Windows keeps a link to a directory
/// as a directory entry, which only `remove_dir` accepts.
fn remove_link(link: &Path) -> std::io::Result<()> {
    std::fs::remove_file(link).or_else(|error| std::fs::remove_dir(link).map_err(|_| error))
}

/// Delete one unfollowed link: its size and checksum describe the link itself,
/// and a broken link has no target to read or to copy.
fn process_one_link(
    link: &Path,
    meta: &std::fs::Metadata,
    age_threshold: Option<Duration>,
    dry_or_confirm: bool,
    do_backup: bool,
) -> DeleteItem {
    let path_str = link.display().to_string();
    if let Some(threshold) = age_threshold {
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::ZERO);
        if age < threshold {
            return DeleteItem::SkippedAge;
        }
    }
    let size = meta.len();
    if dry_or_confirm {
        return DeleteItem::Plan {
            path: path_str,
            size,
        };
    }
    let target = match std::fs::read_link(link) {
        Ok(target) => target,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return DeleteItem::SkippedGone;
        }
        Err(error) => {
            return DeleteItem::Err(
                anyhow::Error::new(error).context(format!("cannot read link {path_str}")),
            );
        }
    };
    if let Err(error) = remove_link(link) {
        if error.kind() == std::io::ErrorKind::NotFound {
            return DeleteItem::SkippedGone;
        }
        return DeleteItem::Err(
            anyhow::Error::new(error).context(format!("cannot delete {path_str}")),
        );
    }
    if let Some(parent) = link.parent() {
        if let Err(e) = platform::fsync_dir(parent) {
            tracing::warn!(
                path = %parent.display(),
                error = %e,
                "fsync_dir after delete failed"
            );
        }
    }
    DeleteItem::Deleted {
        path: path_str,
        size,
        hash: Some(checksum::hash_bytes(target.as_os_str().as_encoded_bytes())),
        backup: None,
        backup_skipped: do_backup
            .then(|| crate::commands::backup_names::BACKUP_OF_LINK_SKIPPED.to_string()),
    }
}

fn process_one_delete(
    file_path: &Path,
    age_threshold: Option<Duration>,
    max_size: u64,
    dry_or_confirm: bool,
    do_backup: bool,
    retention: u8,
) -> DeleteItem {
    if crate::signal::is_global_shutdown() {
        // The cause comes from the live signal, never from a fixed sentence.
        // "cancelled by signal" was stated rather than observed, so a run
        // stopped by the global `--timeout-secs` deadline reported a signal
        // that never arrived.
        return DeleteItem::Err(
            crate::signal::cancelled_error(crate::signal::current_cancel_reason()).into(),
        );
    }

    let path_str = file_path.display().to_string();
    // `symlink_metadata` never follows the final entry, so a link is seen as a
    // link and a fifo is seen without being opened.
    let meta = match std::fs::symlink_metadata(file_path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return DeleteItem::SkippedGone;
        }
        Err(error) => {
            return DeleteItem::Err(
                anyhow::Error::new(error).context(format!("cannot stat {path_str}")),
            );
        }
    };

    if meta.file_type().is_symlink() {
        return process_one_link(file_path, &meta, age_threshold, dry_or_confirm, do_backup);
    }

    if let Some(threshold) = age_threshold {
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::ZERO);
        if age < threshold {
            return DeleteItem::SkippedAge;
        }
    }

    let size = meta.len();
    if dry_or_confirm {
        return DeleteItem::Plan {
            path: path_str,
            size,
        };
    }

    // A fifo, socket or device has no content to hash and no bytes to copy:
    // opening it would block or read a device. The entry is unlinked instead.
    let is_regular = meta.file_type().is_file();
    // A delete publishes the bytes it removed, so it must not race a writer
    // that swaps the inode under the same path: the hash, the copy and the
    // unlink describe one inode only while that writer is excluded. The lock is
    // taken on the regular file the caller named; a fifo, socket or device is
    // still unlinked without one, because opening such an entry for a lock can
    // block or touch a device.
    let _lock = if is_regular {
        match crate::lock::acquire_for_update(file_path) {
            Ok(guard) => Some(guard),
            Err(error) => return DeleteItem::Err(error),
        }
    } else {
        None
    };
    let hash = if is_regular {
        match checksum::hash_file(file_path, max_size) {
            Ok(hash) => Some(hash),
            Err(error) => return DeleteItem::Err(error),
        }
    } else {
        None
    };

    // A file that is itself a backup gets no backup of its own: the copy
    // doubles the bytes and no verb ever prunes the copies of copies. The
    // decision belongs here, per target, so naming such a file explicitly
    // still deletes it.
    let target_is_backup =
        do_backup && is_regular && crate::commands::backup_names::is_backup_path(file_path);
    let backup = if do_backup && is_regular && !target_is_backup {
        match crate::atomic::create_backup(file_path, retention) {
            Ok(path) => Some(path.display().to_string()),
            Err(error) => return DeleteItem::Err(error),
        }
    } else {
        None
    };
    let backup_skipped = if do_backup && !is_regular {
        Some(crate::commands::backup_names::BACKUP_OF_SPECIAL_SKIPPED.to_string())
    } else {
        target_is_backup
            .then(|| crate::commands::backup_names::BACKUP_OF_BACKUP_SKIPPED.to_string())
    };

    if let Err(error) = std::fs::remove_file(file_path) {
        if error.kind() == std::io::ErrorKind::NotFound {
            return DeleteItem::SkippedGone;
        }
        return DeleteItem::Err(
            anyhow::Error::new(error).context(format!("cannot delete {}", file_path.display())),
        );
    }

    if let Some(parent) = file_path.parent() {
        if let Err(error) = platform::fsync_dir(parent) {
            tracing::warn!(
                path = %parent.display(),
                error = %error,
                "fsync_dir after delete failed"
            );
        }
    }

    DeleteItem::Deleted {
        path: path_str,
        size,
        hash,
        backup,
        backup_skipped,
    }
}
