// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parallel text replacement across files with atomic writes.
//! Workload: I/O-bound (file reading + regex matching + atomic write).

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
use crate::commands::resolve_backup;
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
#[tracing::instrument(skip_all, fields(command = "replace"))]
pub fn cmd_replace(
    args: &ReplaceArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    // G121 (CWD-relative path resolution for replace): centralize via
    // path_resolution::resolve_paths_against_workspace. This runs
    // validate_path on every caller-supplied root, COLLECTS the canonical
    // absolute PathBuf, and returns them so the WalkBuilder receives a
    // workspace-anchored root instead of the original (CWD-relative) path.
    let canonical_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;

    let pattern = compile_pattern(args)?;

    let walker = build_walker(args, &canonical_paths, global)?;

    let (tx, rx) = crossbeam_channel::bounded::<ReplaceEvent>(1024);

    let files_visited = Arc::new(AtomicU64::new(0));
    let files_modified = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));
    let total_replacements = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let fm = Arc::clone(&files_modified);
    let fs_skip = Arc::clone(&files_skipped);
    let tr = Arc::clone(&total_replacements);
    let replacement: Arc<str> = args.replacement.clone().into();
    let max_replacements = args.max_replacements;
    let dry_run = args.dry_run;
    let preview = args.preview;
    let backup = resolve_backup(args.backup, args.no_backup);
    let keep_backup = args.keep_backup;
    let ws: Arc<std::path::Path> = Arc::from(workspace.as_path());
    let expect_ck: Option<Arc<str>> = args.expect_checksum.clone().map(Into::into);

    let max_size = global.effective_max_filesize();
    let shutdown_flag = shutdown.flag();
    let preserve_timestamps = args.preserve_timestamps;
    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let pattern = pattern.clone();
            let replacement = Arc::clone(&replacement);
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let fm = Arc::clone(&fm);
            let fs_skip = Arc::clone(&fs_skip);
            let tr = Arc::clone(&tr);
            let ws = Arc::clone(&ws);
            let expect_ck = expect_ck.clone();
            let shutdown_flag = Arc::clone(&shutdown_flag);

            Box::new(move |entry| {
                if shutdown_flag.load(Ordering::Acquire) {
                    return ignore::WalkState::Quit;
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };

                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    return ignore::WalkState::Continue;
                }

                fv.fetch_add(1, Ordering::Relaxed);

                let path = entry.path().to_path_buf();
                let _span = tracing::debug_span!("process_file", path = %path.display()).entered();

                // Validate path against workspace jail BEFORE processing
                if crate::path_safety::validate_path(&path, &ws).is_err() {
                    fs_skip.fetch_add(1, Ordering::Relaxed);
                    let _ = tx.send(ReplaceEvent::Error {
                        path,
                        kind: ReplaceErrorKind::JailViolation,
                    });
                    return ignore::WalkState::Continue;
                }

                let content = match crate::file_io::read_file_string(&path, max_size) {
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
                    if checksum_before != **expected {
                        // Receiver may have dropped during shutdown — send failure is expected
                        let _ = tx.send(ReplaceEvent::Error {
                            path,
                            kind: ReplaceErrorKind::StateDrift {
                                expected: expected.to_string(),
                                actual: checksum_before,
                            },
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
                    preserve_timestamps,
                    backup_output_dir: None,
                    strategy: None,
                    strict_atomic: false,
                    syntax_check: false,
                    wal_policy: crate::wal::WalPolicy::Auto,
                    keep_backup,
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
                            mtime_preserved: preserve_timestamps,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(ReplaceEvent::Error {
                            path,
                            kind: ReplaceErrorKind::WriteFailure(format!("{e:#}")),
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
                mtime_preserved,
            } => {
                let path_str = path.display().to_string();
                writer.write_event(&ReplaceResult {
                    r#type: "replaced",
                    path: path_str,
                    replacements,
                    bytes_before,
                    bytes_after,
                    checksum_before,
                    checksum_after,
                    elapsed_ms,
                    mtime_preserved: Some(mtime_preserved),
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
                writer.write_event(&crate::ndjson_types::ReplacePreview {
                    r#type: "preview",
                    path: path.display().to_string(),
                    replacements,
                    diff,
                })?;
            }
            ReplaceEvent::Error { path, kind } => {
                let (message, error_class, retryable) = match kind {
                    ReplaceErrorKind::StateDrift { expected, actual } => (
                        format!("state drift: expected {expected}, got {actual}"),
                        crate::error::ErrorClass::Conflict.as_str(),
                        true,
                    ),
                    ReplaceErrorKind::WriteFailure(msg) => {
                        (msg, crate::error::ErrorClass::Transient.as_str(), true)
                    }
                    ReplaceErrorKind::JailViolation => (
                        "path escapes workspace jail; use --workspace to set a different root"
                            .to_string(),
                        crate::error::ErrorClass::Permanent.as_str(),
                        false,
                    ),
                };
                writer.write_event(&crate::ndjson_types::ReplaceErrorEvent {
                    status: "error",
                    path: path.display().to_string(),
                    message,
                    error_class,
                    retryable,
                })?;
            }
        }
    }

    if let Err(panic_payload) = walker_thread.join() {
        std::panic::resume_unwind(panic_payload);
    }

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
        mtime_preserved: bool,
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
        kind: ReplaceErrorKind,
    },
}

enum ReplaceErrorKind {
    StateDrift { expected: String, actual: String },
    WriteFailure(String),
    JailViolation,
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

fn apply_replacement<'a>(
    pattern: &Regex,
    content: &'a str,
    replacement: &str,
    max_replacements: Option<usize>,
) -> (Cow<'a, str>, u64) {
    let count = pattern.find_iter(content).count() as u64;

    if count == 0 {
        return (Cow::Borrowed(content), 0);
    }

    let replaced = match max_replacements {
        Some(n) => {
            let actual_count = count.min(n as u64);
            let result = pattern.replacen(content, n, replacement);
            return (Cow::Owned(result.into_owned()), actual_count);
        }
        None => pattern.replace_all(content, replacement),
    };

    match replaced {
        Cow::Borrowed(_) => (Cow::Borrowed(content), 0),
        Cow::Owned(s) => (Cow::Owned(s), count),
    }
}

fn build_walker(
    args: &ReplaceArgs,
    canonical_paths: &[std::path::PathBuf],
    global: &GlobalArgs,
) -> Result<ignore::WalkBuilder> {
    let mut builder = ignore::WalkBuilder::new(&canonical_paths[0]);

    for path in canonical_paths.iter().skip(1) {
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
        let mut overrides = ignore::overrides::OverrideBuilder::new(&canonical_paths[0]);
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
