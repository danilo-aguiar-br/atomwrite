// SPDX-License-Identifier: MIT OR Apache-2.0

//! File deletion with optional backup before removal.
//! Workload: I/O-bound (unlink syscall + fsync).

use std::io::Write;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};

use crate::checksum;
use crate::cli::{DeleteArgs, GlobalArgs};
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
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let mut deleted = 0u64;
    let mut _bytes_freed = 0u64;
    let mut skipped = 0u64;

    let age_threshold = match &args.older_than {
        Some(dur_str) => Some(parse_human_duration(dur_str)?),
        None => None,
    };

    let mut visited = 0u64;

    for path in &args.paths {
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

        let files_to_delete: Vec<std::path::PathBuf> = if path.is_dir() {
            let mut files = Vec::new();
            let mut walker_builder = ignore::WalkBuilder::new(&path);
            walker_builder
                .hidden(!global.hidden)
                .git_ignore(!global.no_gitignore)
                .follow_links(global.follow_symlinks);

            if !args.include.is_empty() || !args.exclude.is_empty() {
                let mut overrides = ignore::overrides::OverrideBuilder::new(&path);
                for pat in &args.include {
                    overrides.add(pat)?;
                }
                for pat in &args.exclude {
                    overrides.add(&format!("!{pat}"))?;
                }
                walker_builder.overrides(overrides.build()?);
            }

            for entry in walker_builder.build().flatten() {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    files.push(entry.into_path());
                }
            }
            files
        } else {
            vec![path.clone()]
        };

        for file_path in &files_to_delete {
            visited += 1;
            let path_str = file_path.display().to_string();
            let meta =
                std::fs::metadata(file_path).with_context(|| format!("cannot stat {path_str}"))?;

            if let Some(threshold) = age_threshold {
                let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let age = SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or(Duration::ZERO);
                if age < threshold {
                    skipped += 1;
                    continue;
                }
            }

            let hash = checksum::hash_file(file_path, global.effective_max_filesize())?;
            let size = meta.len();

            if args.dry_run || args.confirm {
                writer.write_event(&DryRunPlan {
                    r#type: "plan",
                    operation: "delete".into(),
                    path: path_str,
                    would_modify: true,
                    details: Some(format!("{size} bytes")),
                })?;
                deleted += 1;
                _bytes_freed += size;
                continue;
            }

            if args.backup {
                crate::atomic::create_backup(file_path, args.retention)?;
            }

            std::fs::remove_file(file_path)
                .with_context(|| format!("cannot delete {}", file_path.display()))?;

            if let Some(parent) = file_path.parent() {
                if let Err(e) = platform::fsync_dir(parent) {
                    tracing::warn!(
                        path = %parent.display(),
                        error = %e,
                        "fsync_dir after delete failed"
                    );
                }
            }

            deleted += 1;
            _bytes_freed += size;

            writer.write_event(&DeleteOutput {
                r#type: "deleted",
                path: path_str,
                bytes: size,
                checksum_before: hash,
                elapsed_ms: start.elapsed().as_millis() as u64,
            })?;
        }

        if path.is_dir() && !args.dry_run && files_to_delete.is_empty() {
            std::fs::remove_dir_all(&path)
                .with_context(|| format!("cannot remove directory {}", path.display()))?;
        } else if path.is_dir() && !args.dry_run {
            let mut dirs_to_remove = Vec::new();
            for entry in walkdir::WalkDir::new(&path)
                .contents_first(true)
                .into_iter()
                .flatten()
            {
                if entry.file_type().is_dir() {
                    dirs_to_remove.push(entry.into_path());
                }
            }
            for dir in &dirs_to_remove {
                let _ = std::fs::remove_dir(dir);
            }
        }
    }

    writer.write_event(&Summary {
        r#type: "summary",
        files_visited: visited,
        files_matched: deleted,
        files_modified: Some(deleted),
        files_skipped: Some(skipped + visited.saturating_sub(deleted + skipped)),
        total_matches: None,
        total_replacements: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}
