// SPDX-License-Identifier: MIT OR Apache-2.0
// `sparse outline`, included by `sparse.rs`.
//
// Split out of `sparse.rs` so no single file carries the whole verb surface:
// this stage walks under a budget, reads with `par_iter` and emits in path
// order, and it was the part that made the file outgrow the ceiling.

#[cfg(feature = "ast")]
fn sparse_outline(
    args: &SparseOutlineArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    use crate::concurrency::sort_paths_parallel;
    use std::time::Instant;
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let root = validate_path(&args.path, &workspace)?;
    // Excludes prune the walk; includes are matched by name below, but are
    // compiled here so an invalid glob is refused instead of silently ignored.
    crate::walk_globs::validate(&args.include, &[])?;
    // The walk honors `--hidden` like every other scanning verb. The tool's own
    // journal is a dotfile, so the hidden rule already keeps it out of a default
    // run; `--hidden` or the verb's own `--all` asks for every entry and gets
    // it, which is what `count --hidden` and `sparse list` already answer.
    let walker = crate::commands::build_scoped_walker_flavored(
        std::slice::from_ref(&root),
        global,
        if args.all {
            crate::commands::WalkerFlavor::AlwaysVisitHidden
        } else {
            crate::commands::WalkerFlavor::RespectGlobalFlags
        },
        &[],
        &args.exclude,
    )?;

    // Stage 1 — budgeted parallel path collect (WalkParallel + Atomic cap).
    // Workers may overshoot slightly; clamp after sort for a hard max_files.
    let include = args.include.clone();
    // Declared before the walk so a directory it cannot open is reported: an
    // outline built over a silent hole is not the tree's outline.
    let failures = std::sync::Arc::new(std::sync::Mutex::new(
        crate::ndjson_types::FailureTally::default(),
    ));
    // An ignore file the walk half-reads is a hole too, and `ignore` never
    // reports it by itself.
    crate::concurrency::record_ignore_file_failures(
        std::slice::from_ref(&root),
        Some(&failures),
    );
    // Stage-2 checks `shutdown`; stage-1 uses the process-wide signal.
    let (mut paths, hit_budget) = collect_mapped_parallel_budgeted_tallied(
        &walker,
        args.max_files,
        move |entry| {
            if crate::signal::is_global_shutdown() {
                return None;
            }
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                return None;
            }
            // An outline of a timestamped copy describes content the caller
            // already has under another name, so the copy is not an item.
            if crate::commands::is_backup_path(entry.path()) {
                return None;
            }
            let path = entry.path().to_path_buf();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !include.is_empty() {
                let ok = include.iter().any(|pat| {
                    if let Some(suf) = pat.strip_prefix("*.") {
                        name.ends_with(suf)
                            || path.extension().and_then(|e| e.to_str()) == Some(suf)
                    } else {
                        name.contains(pat) || path.to_string_lossy().contains(pat)
                    }
                });
                if !ok {
                    return None;
                }
            }
            Some(path)
        },
        Some(std::sync::Arc::clone(&failures)),
    );
    sort_paths_parallel(&mut paths);
    let mut truncated = hit_budget || paths.len() as u64 > args.max_files;
    if paths.len() as u64 > args.max_files {
        paths.truncate(args.max_files as usize);
    }
    // `--limit` bounds the work on top of `--max-files`, which bounds discovery.
    if crate::output::cap_collected(&mut paths) {
        truncated = true;
    }

    let files_seen = paths.len() as u64;
    let max = global.effective_max_filesize();

    // Stage 2 — independent read+AST (rayon-safe via collect_outline_for_path).
    // A file that cannot be read is a hole in the outline, so it is recorded
    // instead of being dropped as if it had no items.
    let record_read = |path: &PathBuf, e: &anyhow::Error| {
        if let Ok(mut tally) = failures.lock() {
            tally.record(
                path,
                anyhow::Error::new(crate::error::AtomwriteError::Io {
                    source: std::io::Error::other(e.to_string()),
                }),
            );
        }
    };
    // A file the parser could not handle is not a file without symbols: the
    // status flag keeps the two apart, and the summary counts the holes.
    let outlined: Vec<(PathBuf, Vec<crate::commands::outline::OutlineItem>, bool)> =
        if should_parallelize(paths.len()) {
            paths
                .par_iter()
                .filter_map(|path| {
                    if shutdown.is_shutdown() {
                        return None;
                    }
                    let content = match crate::file_io::read_file_bytes(path, max) {
                        Ok(content) => content,
                        Err(e) => {
                            record_read(path, &e);
                            return None;
                        }
                    };
                    let (items, parsed) = crate::commands::outline::collect_outline_for_path_status(
                        path, &content, None,
                    );
                    Some((path.clone(), items, parsed))
                })
                .collect()
        } else {
            paths
                .iter()
                .filter_map(|path| {
                    if shutdown.is_shutdown() {
                        return None;
                    }
                    let content = match crate::file_io::read_file_bytes(path, max) {
                        Ok(content) => content,
                        Err(e) => {
                            record_read(path, &e);
                            return None;
                        }
                    };
                    let (items, parsed) = crate::commands::outline::collect_outline_for_path_status(
                        path, &content, None,
                    );
                    Some((path.clone(), items, parsed))
                })
                .collect()
        };

    // Stage 3 — emit in path order for stable NDJSON.
    let mut outlined = outlined;
    outlined.sort_by(|a, b| a.0.cmp(&b.0));
    let mut items_emitted = 0u64;
    let mut items_truncated = 0u64;
    let mut files_unparsed = 0u64;
    for (_, items, parsed) in outlined {
        if !parsed {
            files_unparsed += 1;
        }
        items_emitted += items.len() as u64;
        items_truncated += items
            .iter()
            .filter(|i| i.signature_truncated == Some(true))
            .count() as u64;
        for item in items {
            writer.write_event(&item)?;
        }
    }

    let tally = match std::sync::Arc::try_unwrap(failures) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => std::mem::take(&mut *arc.lock().unwrap_or_else(|e| e.into_inner())),
    };
    let (count, items, first) = tally.into_parts();
    // `emitted` counts records that reached the consumer, not records the verb
    // attempted: `files_seen` is the discovery side and belongs to the budget
    // event below, which says so in its own field.
    let files_failed = (count > 0).then_some(count);
    let summary = SparseSummary {
        r#type: "sparse_summary",
        emitted: writer.emitted_count(),
        truncated,
        elapsed_ms: start.elapsed().as_millis() as u64,
        files_failed,
        partial: crate::ndjson_types::partial_of(files_failed),
        errors: (!items.is_empty()).then_some(items),
        walker_stopped_early: crate::output::limit_reached().then_some(true),
        files_scanned: Some(files_seen),
        path_bytes: None,
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        files_unparsed: (files_unparsed > 0).then_some(files_unparsed),
        items_truncated: (items_truncated > 0).then_some(items_truncated),
    };
    if let Some(err) = first {
        return Err(writer.publish_failure(&summary, err));
    }
    writer.write_terminal(&summary)?;
    writer.write_event(&crate::ndjson_types::SparseOutlineBudget {
        r#type: "sparse_outline_budget",
        files_seen,
        items: items_emitted,
        truncated,
        max_files: args.max_files,
    })?;
    Ok(())
}

