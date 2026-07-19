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

use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::checksum;
use crate::cli::{DeleteArgs, GlobalArgs};
use crate::commands::resolve_backup;
use crate::concurrency::should_parallelize;
use crate::error::AtomwriteError;
use crate::ndjson_types::{DeleteOutput, DryRunPlan, Summary};
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
                'm' => 60,
                'h' => 3600,
                'd' => 86400,
                'w' => 604800,
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

/// Outcome of processing one delete candidate (for ordered NDJSON emit).
enum DeleteItem {
    SkippedAge,
    Plan {
        path: String,
        size: u64,
    },
    Deleted {
        path: String,
        size: u64,
        hash: String,
    },
    Err(anyhow::Error),
}

/// Delete files with optional backup and dry-run support.
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
    let resolved = resolve_backup(&args.backup_opts, defaults);
    // v0.1.28 GAP-CLI-SURFACE-DRIFT: `--keep-backup` is a silent no-op for
    // delete (deletion backups are never auto-removed); surface it instead
    // of discarding the flag silently.
    let warnings: Vec<String> = if args.backup_opts.keep_backup {
        vec![
            "--keep-backup is redundant for delete: deletion backups are always preserved"
                .to_owned(),
        ]
    } else {
        Vec::new()
    };
    let mut deleted = 0u64;
    let mut planned = 0u64;
    let mut _bytes_freed = 0u64;
    let mut skipped = 0u64;

    let age_threshold = match &args.older_than {
        Some(dur_str) => Some(parse_human_duration(dur_str)?),
        None => None,
    };

    let mut visited = 0u64;
    let max_size = global.effective_max_filesize();
    // B-005 / B-014: `--confirm` is not plan and does not delete — fail closed.
    if args.confirm {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "delete --confirm is rejected (one-shot): use --plan to list targets without deleting, or omit flags to delete; write --confirm is a different large-file guard"
                .into(),
        }
        .into());
    }
    // B-015: `-y/--yes` is not a no-op — fail closed so agents do not invent confirm UX.
    if args.yes {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "delete -y/--yes is rejected (one-shot: no interactive confirm exists); use --plan to list targets, or omit flags to delete"
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

        let path = crate::path_safety::validate_path(path, &workspace)?;

        if !path.exists() {
            return Err(AtomwriteError::NotFound { path }.into());
        }

        if path.is_dir() && !args.recursive {
            return Err(AtomwriteError::InvalidInput {
                reason: format!("{} is a directory, use --recursive", path.display()),
            }
            .into());
        }

        if path.is_dir() {
            dir_roots.push(path);
        } else {
            files_to_delete.push(path);
        }
    }

    if !dir_roots.is_empty() && !crate::signal::is_global_shutdown() {
        let mut walker_builder = ignore::WalkBuilder::new(&dir_roots[0]);
        for d in dir_roots.iter().skip(1) {
            walker_builder.add(d);
        }
        walker_builder
            .hidden(!global.hidden)
            .git_ignore(!global.no_gitignore)
            .follow_links(global.follow_symlinks);
        crate::concurrency::apply_walk_threads(&mut walker_builder, global.threads);

        if !args.include.is_empty() || !args.exclude.is_empty() {
            let mut overrides = ignore::overrides::OverrideBuilder::new(&dir_roots[0]);
            for pat in &args.include {
                overrides.add(pat)?;
            }
            for pat in &args.exclude {
                overrides.add(&format!("!{pat}"))?;
            }
            walker_builder.overrides(overrides.build()?);
        }

        files_to_delete.extend(crate::concurrency::collect_files_parallel(&walker_builder));
    }

    crate::concurrency::sort_paths_parallel(&mut files_to_delete);
    files_to_delete.dedup();

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
            DeleteItem::SkippedAge => {
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
                })?;
                // G-002/G-016: plan must not count as modified.
                planned += 1;
                _bytes_freed += size;
            }
            DeleteItem::Deleted {
                path: path_str,
                size,
                hash,
            } => {
                visited += 1;
                deleted += 1;
                _bytes_freed += size;
                writer.write_event(&DeleteOutput {
                    r#type: "deleted",
                    path: path_str,
                    bytes: size,
                    checksum_before: hash,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    warnings: warnings.clone(),
                })?;
            }
            DeleteItem::Err(e) => return Err(e),
        }
    }

    // Phase 3 — remove emptied directory trees (independent roots fan out).
    // G-002 / A-004: plan never mutates directory trees either.
    if !args.dry_run && !args.plan && !crate::signal::is_global_shutdown() {
        let cleanup: Vec<Result<(), anyhow::Error>> = if should_parallelize(dir_roots.len()) {
            dir_roots
                .par_iter()
                .map(|path| cleanup_dir_root(path, &files_to_delete))
                .collect()
        } else {
            dir_roots
                .iter()
                .map(|path| cleanup_dir_root(path, &files_to_delete))
                .collect()
        };
        for r in cleanup {
            r?;
        }
    }

    // On cooperative cancel, skip summary so main can emit the shutdown
    // banner + signal exit code (same contract as search/list/count).
    if crate::signal::is_global_shutdown() {
        return Ok(());
    }

    writer.write_event(&Summary {
        r#type: "summary",
        files_visited: visited,
        files_matched: deleted + planned,
        files_modified: Some(deleted),
        files_skipped: Some(skipped + visited.saturating_sub(deleted + planned + skipped)),
        total_matches: None,
        total_replacements: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}

fn cleanup_dir_root(path: &Path, files_to_delete: &[PathBuf]) -> Result<()> {
    let files_under = files_to_delete.iter().any(|f| f.starts_with(path));
    if !files_under {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("cannot remove directory {}", path.display()))?;
    } else {
        // Parallel discovery of empty-candidate dirs; bottom-up remove stays
        // sequential (parent depends on children being gone first).
        let mut builder = ignore::WalkBuilder::new(path);
        builder.hidden(true).git_ignore(false);
        crate::concurrency::apply_walk_threads(&mut builder, None);
        let mut dirs_to_remove: Vec<PathBuf> =
            crate::concurrency::collect_mapped_parallel(&builder, |entry| {
                if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    Some(entry.path().to_path_buf())
                } else {
                    None
                }
            });
        // Deepest paths first ≈ contents_first semantics for remove_dir.
        dirs_to_remove.sort_by(|a, b| {
            let da = a.components().count();
            let db = b.components().count();
            db.cmp(&da).then_with(|| b.cmp(a))
        });
        for dir in &dirs_to_remove {
            let _ = std::fs::remove_dir(dir);
        }
    }
    Ok(())
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
        return DeleteItem::Err(crate::signal::cancelled_error("delete cancelled by signal").into());
    }

    let path_str = file_path.display().to_string();
    let meta = match std::fs::metadata(file_path) {
        Ok(m) => m,
        Err(e) => {
            return DeleteItem::Err(
                anyhow::Error::new(e).context(format!("cannot stat {path_str}")),
            );
        }
    };

    if let Some(threshold) = age_threshold {
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::ZERO);
        if age < threshold {
            return DeleteItem::SkippedAge;
        }
    }

    let hash = match checksum::hash_file(file_path, max_size) {
        Ok(h) => h,
        Err(e) => return DeleteItem::Err(e),
    };
    let size = meta.len();

    if dry_or_confirm {
        return DeleteItem::Plan {
            path: path_str,
            size,
        };
    }

    if do_backup {
        if let Err(e) = crate::atomic::create_backup(file_path, retention) {
            return DeleteItem::Err(e);
        }
    }

    if let Err(e) = std::fs::remove_file(file_path) {
        return DeleteItem::Err(
            anyhow::Error::new(e).context(format!("cannot delete {}", file_path.display())),
        );
    }

    if let Some(parent) = file_path.parent() {
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
        hash,
    }
}
