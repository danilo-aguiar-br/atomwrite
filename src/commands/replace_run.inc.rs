// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by replace.rs (A-MONO-001).

fn run_replace(
    args: &ReplaceArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
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

    // v0.1.30: pre-count eligible files so progress has total/rate/eta.
    // Skip entirely when progress is off (avoids a full double-walk).
    // When needed, use WalkParallel so --threads applies (build() ignores threads).
    let progress_wanted =
        args.progress_every > 0 && !global.no_progress && global.quiet == 0;
    let precount = if progress_wanted {
        let paths = crate::concurrency::collect_files_parallel(&walker);
        paths.len() as u64
    } else {
        0
    };
    let files_total = Arc::new(AtomicU64::new(precount));
    let walker = build_walker(args, &canonical_paths, global)?;

    let (tx, rx) =
        crossbeam_channel::bounded::<ReplaceEvent>(crate::concurrency::event_channel_cap());

    // Per-run counters. Ordering::Relaxed: independent tallies; final loads
    // run after the parallel join (no data-publication barrier needed).
    let files_visited = Arc::new(AtomicU64::new(0));
    let files_modified = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));
    let total_replacements = Arc::new(AtomicU64::new(0));
    // G-027: count binary rejections separately from generic I/O failures.
    let binary_rejects = Arc::new(AtomicU64::new(0));
    let first_binary_path: Arc<std::sync::Mutex<Option<PathBuf>>> =
        Arc::new(std::sync::Mutex::new(None));

    let fv = Arc::clone(&files_visited);
    let fm = Arc::clone(&files_modified);
    let fs_skip = Arc::clone(&files_skipped);
    let tr = Arc::clone(&total_replacements);
    let binary_rejects_w = Arc::clone(&binary_rejects);
    let first_binary_w = Arc::clone(&first_binary_path);
    let ft = Arc::clone(&files_total);
    let replacement: Arc<str> = args.replacement.clone().into();
    let max_replacements = args.max_replacements;
    let dry_run = args.dry_run;
    let preview = args.preview;
    let resolved = resolve_backup(&args.backup_opts, defaults);
    let backup = resolved.backup;
    let keep_backup = resolved.keep;
    let retention = resolved.retention;
    let ws: Arc<std::path::Path> = Arc::from(workspace.as_path());
    let expect_ck: Option<Arc<str>> = args.expect_checksum.clone().map(Into::into);

    let max_size = global.effective_max_filesize();
    let shutdown_flag = shutdown.flag();
    let preserve_timestamps = args.preserve_timestamps;
    let preserve_case = args.preserve_case;
    let use_regex = args.regex;
    let (fuzzy_mode, fuzzy_threshold) =
        crate::config::resolve_fuzzy(args.fuzzy, args.fuzzy_threshold, fuzzy_cfg)?;
    let word_flag = args.word;
    let progress_every = args.progress_every;
    let quiet = global.quiet;
    let no_progress = global.no_progress;
    let pattern_owned: Arc<str> = args.pattern.clone().into();
    let fuzzy_opts = fuzzy::match_opts_from_section(
        fuzzy_mode,
        fuzzy_threshold,
        fuzzy_cfg,
        false,
    );
    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let pattern = pattern.clone();
            let pattern_owned = Arc::clone(&pattern_owned);
            let replacement = Arc::clone(&replacement);
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let fm = Arc::clone(&fm);
            let fs_skip = Arc::clone(&fs_skip);
            let tr = Arc::clone(&tr);
            let binary_rejects_w = Arc::clone(&binary_rejects_w);
            let first_binary_w = Arc::clone(&first_binary_w);
            let ft = Arc::clone(&ft);
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

                let visited = fv.fetch_add(1, Ordering::Relaxed) + 1;
                if progress_every > 0
                    && visited.is_multiple_of(progress_every)
                    && quiet == 0
                    && !no_progress
                {
                    let _ = tx.send(ReplaceEvent::Progress {
                        done: visited,
                        total: ft.load(Ordering::Relaxed),
                    });
                }

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

                // G-015 / G-027: refuse binary content with permanent BINARY_FILE taxonomy.
                if crate::binary_detect::is_binary(content.as_bytes()) {
                    fs_skip.fetch_add(1, Ordering::Relaxed);
                    binary_rejects_w.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut guard) = first_binary_w.lock() {
                        if guard.is_none() {
                            *guard = Some(path.clone());
                        }
                    }
                    let _ = tx.send(ReplaceEvent::Error {
                        path,
                        kind: ReplaceErrorKind::BinaryRejected,
                    });
                    return ignore::WalkState::Continue;
                }

                let mut fuzzy_meta: Option<(bool, String, Option<f64>, u64)> = None;
                let mut word_ignored = false;
                let (replaced, count) = if use_regex {
                    apply_replacement(
                        &pattern,
                        &content,
                        &replacement,
                        max_replacements,
                        preserve_case,
                    )
                } else {
                    // Fixed string: exact multi first, then fuzzy cascade if zero hits.
                    let (exact, exact_count) = apply_replacement(
                        &pattern,
                        &content,
                        &replacement,
                        max_replacements,
                        preserve_case,
                    );
                    if exact_count > 0 {
                        (exact, exact_count)
                    } else {
                        // v0.1.33 one-shot: never re-scan inserted replacement text.
                        // Default max applies = 1; pat⊂rep forces 1; hard ceiling applies.
                        if word_flag {
                            word_ignored = true;
                        }
                        if shutdown_flag.load(Ordering::Acquire) {
                            return ignore::WalkState::Quit;
                        }
                        match fuzzy::apply_fuzzy_one_pass(
                            &content,
                            pattern_owned.as_ref(),
                            replacement.as_ref(),
                            fuzzy_opts,
                            max_replacements,
                        ) {
                            Ok(result) => {
                                if result.applied > 0 {
                                    if let Some(info) = result.info {
                                        fuzzy_meta = Some((
                                            info.fuzzy,
                                            info.strategy,
                                            info.similarity,
                                            info.strategies_tried,
                                        ));
                                    }
                                    (std::borrow::Cow::Owned(result.edited), result.applied)
                                } else {
                                    (std::borrow::Cow::Borrowed(content.as_str()), 0)
                                }
                            }
                            Err(crate::error::AtomwriteError::Cancelled { .. }) => {
                                return ignore::WalkState::Quit;
                            }
                            Err(_) => (std::borrow::Cow::Borrowed(content.as_str()), 0),
                        }
                    }
                };

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
                    retention,
                    preserve_timestamps,
                    backup_output_dir: None,
                    strategy: None,
                    strict_atomic: false,
                    syntax_check: false,
                    wal_policy: crate::wal::WalPolicy::Auto,
                    // GAP-105: retain backup when --backup is explicitly active
                    keep_backup: keep_backup || backup,
                    durability: crate::platform::Durability::Auto,
                };

                match atomic_write(&path, replaced.as_bytes(), &opts, &ws) {
                    Ok(result) => {
                        fm.fetch_add(1, Ordering::Relaxed);
                        let (fuzzy, strategy, similarity, strategies_tried) = match fuzzy_meta {
                            Some((f, s, sim, tried)) => (Some(f), Some(s), sim, Some(tried)),
                            None => (Some(false), Some("exact".into()), None, Some(1)),
                        };
                        let _ = tx.send(ReplaceEvent::Replaced {
                            path,
                            replacements: count,
                            bytes_before: content.len() as u64,
                            bytes_after: replaced.len() as u64,
                            checksum_before,
                            checksum_after: result.checksum,
                            elapsed_ms: result.elapsed_ms,
                            mtime_preserved: preserve_timestamps,
                            fuzzy,
                            strategy,
                            similarity,
                            strategies_tried,
                            word_ignored: if word_ignored { Some(true) } else { None },
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
                fuzzy,
                strategy,
                similarity,
                strategies_tried,
                word_ignored,
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
                    fuzzy,
                    strategy,
                    similarity,
                    strategies_tried,
                    word_ignored,
                })?;
            }
            ReplaceEvent::Progress { done, total } => {
                let elapsed = start.elapsed().as_secs_f64();
                let rate = if elapsed > 0.0 {
                    done as f64 / elapsed
                } else {
                    0.0
                };
                let eta_ms = if rate > 0.0 && total > done {
                    Some(((total - done) as f64 / rate * 1000.0) as u64)
                } else {
                    None
                };
                writer.write_event(&ProgressEvent {
                    r#type: "progress",
                    done,
                    total,
                    rate_per_s: Some(rate),
                    eta_ms,
                    phase: "replace".into(),
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
                    ReplaceErrorKind::BinaryRejected => (
                        "binary file rejected for text replace (G-015/G-027)".to_string(),
                        crate::error::ErrorClass::Permanent.as_str(),
                        false,
                    ),
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

    let total_repl = total_replacements.load(Ordering::Relaxed);
    let matched = files_modified.load(Ordering::Relaxed);
    // B-002g / R-DRY-001: dry-run / preview must not claim disk mutations.
    let modified_count = crate::commands::summary_metrics::files_modified_for_summary(
        matched,
        dry_run || preview,
    );

    writer.write_event(&Summary {
        r#type: "summary",
        files_visited: files_visited.load(Ordering::Relaxed),
        files_matched: matched,
        files_modified: modified_count,
        files_skipped: Some(files_skipped.load(Ordering::Relaxed)),
        total_matches: None,
        total_replacements: Some(total_repl),
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    if total_repl == 0 {
        // G-027: all-binary / binary-only skips must not look like NO_MATCHES + retryable.
        let binaries = binary_rejects.load(Ordering::Relaxed);
        if binaries > 0 && files_modified.load(Ordering::Relaxed) == 0 {
            let path = first_binary_path
                .lock()
                .ok()
                .and_then(|g| g.clone())
                .unwrap_or_else(|| PathBuf::from("."));
            return Err(crate::error::AtomwriteError::BinaryFile { path }.into());
        }
        return Err(crate::error::AtomwriteError::NoMatches.into());
    }

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
        fuzzy: Option<bool>,
        strategy: Option<String>,
        similarity: Option<f64>,
        strategies_tried: Option<u64>,
        word_ignored: Option<bool>,
    },
    Progress {
        done: u64,
        total: u64,
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
    /// G-027: binary content is a permanent precondition, not a transient write failure.
    BinaryRejected,
    JailViolation,
}
