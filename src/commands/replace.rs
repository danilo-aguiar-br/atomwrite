// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parallel text replacement across files with atomic writes.

use std::borrow::Cow;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use regex::Regex;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{GlobalArgs, ReplaceArgs};
use crate::ndjson_types::{DryRunPlan, ReplaceResult, Summary};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Replace text across files in parallel with atomic writes.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if reading or writing files fails.
/// Returns `AtomwriteError::NoMatches` if no replacements are found.
pub fn cmd_replace(
    args: &ReplaceArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let pattern = compile_pattern(args)?;

    let walker = build_walker(args, global)?;

    let (tx, rx) = crossbeam_channel::unbounded::<ReplaceEvent>();

    let files_visited = Arc::new(AtomicU64::new(0));
    let files_modified = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));
    let total_replacements = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let fm = Arc::clone(&files_modified);
    let fs_skip = Arc::clone(&files_skipped);
    let tr = Arc::clone(&total_replacements);
    let replacement = args.replacement.clone();
    let max_replacements = args.max_replacements;
    let dry_run = args.dry_run;
    let preview = args.preview;
    let backup = args.backup;
    let ws = workspace.clone();
    let expect_ck = args.expect_checksum.clone();

    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let pattern = pattern.clone();
            let replacement = replacement.clone();
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let fm = Arc::clone(&fm);
            let fs_skip = Arc::clone(&fs_skip);
            let tr = Arc::clone(&tr);
            let ws = ws.clone();
            let expect_ck = expect_ck.clone();

            Box::new(move |entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };

                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    return ignore::WalkState::Continue;
                }

                fv.fetch_add(1, Ordering::Relaxed);

                let path = entry.path().to_path_buf();

                let content = match crate::file_io::read_file_string(&path) {
                    Ok(c) => c,
                    Err(_) => {
                        fs_skip.fetch_add(1, Ordering::Relaxed);
                        return ignore::WalkState::Continue;
                    }
                };

                let (replaced, count) =
                    apply_replacement(&pattern, &content, &replacement, max_replacements);

                if count == 0 {
                    fs_skip.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                tr.fetch_add(count, Ordering::Relaxed);

                let checksum_before = checksum::hash_bytes(content.as_bytes());

                if let Some(ref expected) = expect_ck {
                    if &checksum_before != expected {
                        let _ = tx.send(ReplaceEvent::Error {
                            path,
                            message: format!(
                                "state drift: expected {expected}, got {checksum_before}"
                            ),
                        });
                        return ignore::WalkState::Continue;
                    }
                }

                if dry_run {
                    let _ = tx.send(ReplaceEvent::DryRun {
                        path,
                        replacements: count,
                    });
                    fm.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                if preview {
                    let diff = similar::TextDiff::from_lines(&content, &replaced);
                    let unified = diff.unified_diff().to_string();
                    let _ = tx.send(ReplaceEvent::Preview {
                        path,
                        replacements: count,
                        diff: unified,
                    });
                    fm.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                let opts = AtomicWriteOptions {
                    backup,
                    retention: 5,
                    preserve_timestamps: true,
                };

                match atomic_write(&path, replaced.as_bytes(), &opts, &ws) {
                    Ok(result) => {
                        fm.fetch_add(1, Ordering::Relaxed);
                        let _ = tx.send(ReplaceEvent::Replaced {
                            path,
                            replacements: count,
                            bytes_before: content.len() as u64,
                            bytes_after: replaced.len() as u64,
                            checksum_before,
                            checksum_after: result.checksum,
                            elapsed_ms: result.elapsed_ms,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(ReplaceEvent::Error {
                            path,
                            message: format!("{e:#}"),
                        });
                    }
                }

                ignore::WalkState::Continue
            })
        });
    });

    for event in rx {
        if shutdown.is_shutdown() {
            break;
        }

        match event {
            ReplaceEvent::Replaced {
                path,
                replacements,
                bytes_before,
                bytes_after,
                checksum_before,
                checksum_after,
                elapsed_ms,
            } => {
                writer.write_event(&ReplaceResult {
                    r#type: "replaced",
                    path: path.display().to_string(),
                    replacements,
                    bytes_before,
                    bytes_after,
                    checksum_before,
                    checksum_after,
                    elapsed_ms,
                })?;
            }
            ReplaceEvent::DryRun { path, replacements } => {
                writer.write_event(&DryRunPlan {
                    r#type: "plan",
                    operation: "replace".into(),
                    path: path.display().to_string(),
                    would_modify: true,
                    details: Some(format!("{replacements} replacements")),
                })?;
            }
            ReplaceEvent::Preview {
                path,
                replacements,
                diff,
            } => {
                writer.write_event(&serde_json::json!({
                    "type": "preview",
                    "path": path.display().to_string(),
                    "replacements": replacements,
                    "diff": diff,
                }))?;
            }
            ReplaceEvent::Error { path, message } => {
                writer.write_event(&serde_json::json!({
                    "status": "error",
                    "path": path.display().to_string(),
                    "message": message,
                    "error_class": "transient",
                    "retryable": true,
                }))?;
            }
        }
    }

    let _ = walker_thread.join();

    writer.write_event(&Summary {
        r#type: "summary",
        files_visited: files_visited.load(Ordering::Relaxed),
        files_matched: files_modified.load(Ordering::Relaxed),
        files_modified: Some(files_modified.load(Ordering::Relaxed)),
        files_skipped: Some(files_skipped.load(Ordering::Relaxed)),
        total_matches: None,
        total_replacements: Some(total_replacements.load(Ordering::Relaxed)),
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}

enum ReplaceEvent {
    Replaced {
        path: PathBuf,
        replacements: u64,
        bytes_before: u64,
        bytes_after: u64,
        checksum_before: String,
        checksum_after: String,
        elapsed_ms: u64,
    },
    DryRun {
        path: PathBuf,
        replacements: u64,
    },
    Preview {
        path: PathBuf,
        replacements: u64,
        diff: String,
    },
    Error {
        path: PathBuf,
        message: String,
    },
}

fn compile_pattern(args: &ReplaceArgs) -> Result<Regex> {
    let pattern_str = if args.literal || !args.regex {
        regex::escape(&args.pattern)
    } else {
        args.pattern.clone()
    };

    let pattern_str = if args.word {
        format!(r"\b{pattern_str}\b")
    } else {
        pattern_str
    };

    Regex::new(&pattern_str).with_context(|| format!("invalid pattern: {}", args.pattern))
}

fn apply_replacement(
    pattern: &Regex,
    content: &str,
    replacement: &str,
    max_replacements: Option<usize>,
) -> (String, u64) {
    let count = pattern.find_iter(content).count() as u64;

    if count == 0 {
        return (content.to_owned(), 0);
    }

    let replaced = match max_replacements {
        Some(n) => {
            let actual_count = count.min(n as u64);
            let result = pattern.replacen(content, n, replacement);
            return (result.into_owned(), actual_count);
        }
        None => pattern.replace_all(content, replacement),
    };

    match replaced {
        Cow::Borrowed(_) => (content.to_owned(), 0),
        Cow::Owned(s) => (s, count),
    }
}

fn build_walker(args: &ReplaceArgs, global: &GlobalArgs) -> Result<ignore::WalkBuilder> {
    let mut builder = ignore::WalkBuilder::new(&args.paths[0]);

    for path in args.paths.iter().skip(1) {
        builder.add(path);
    }

    builder
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore)
        .follow_links(global.follow_symlinks);

    if let Some(threads) = global.threads {
        builder.threads(if threads == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        } else {
            threads
        });
    }

    if !args.include.is_empty() || !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&args.paths[0]);
        for glob in &args.include {
            overrides.add(glob)?;
        }
        for glob in &args.exclude {
            overrides.add(&format!("!{glob}"))?;
        }
        builder.overrides(overrides.build()?);
    }

    Ok(builder)
}
