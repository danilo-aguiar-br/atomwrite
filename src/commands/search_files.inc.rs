// SPDX-License-Identifier: MIT OR Apache-2.0
// The `--target files` branch of `search`: match on file names, not contents.
// Included from search.rs.

/// Run the file-name search when `--target` asks for it.
///
/// Returns `Ok(true)` when the run is complete (`--target files`, summary
/// already emitted) and `Ok(false)` when the caller continues with the
/// content search (`--target content`, or `both` after the name hits).
fn search_file_names(
    args: &SearchArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    canonical_paths: &[std::path::PathBuf],
    missing_roots: &[std::path::PathBuf],
    start: Instant,
) -> Result<bool> {
    use crate::cli_args::SearchTarget;
    if matches!(args.target, SearchTarget::Files | SearchTarget::Both) {
        let pat = if args.case_insensitive || args.smart_case {
            args.pattern.to_ascii_lowercase()
        } else {
            args.pattern.clone()
        };
        // WalkParallel collect + parallel name filter; sort for stable NDJSON.
        let walker = build_walker(args, canonical_paths, global)?;
        // A subtree the walk could not read must reach the summary: the answer
        // here is "every name under this root", and one lost subtree makes it
        // partial.
        let failures = Arc::new(Mutex::new(root_failures(missing_roots)));
        let files = crate::concurrency::collect_files_parallel_tallied(
            &walker,
            Some(Arc::clone(&failures)),
        );
        let casefold = args.case_insensitive || args.smart_case;
        let mut matches: Vec<(String, String)> =
            if crate::concurrency::should_parallelize(files.len()) {
                use rayon::prelude::*;
                files
                    .par_iter()
                    .filter_map(|path| {
                        if shutdown.is_shutdown() {
                            return None;
                        }
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let hay = if casefold {
                            name.to_ascii_lowercase()
                        } else {
                            name.clone()
                        };
                        if hay.contains(&pat) {
                            Some((path.display().to_string(), name))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                files
                    .iter()
                    .filter_map(|path| {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let hay = if casefold {
                            name.to_ascii_lowercase()
                        } else {
                            name.clone()
                        };
                        if hay.contains(&pat) {
                            Some((path.display().to_string(), name))
                        } else {
                            None
                        }
                    })
                    .collect()
            };
        crate::concurrency::sort_parallel(&mut matches);

        let total_name_hits = matches.len() as u64;
        let start_idx = args.offset.min(total_name_hits) as usize;
        let end_idx = match args.limit {
            Some(lim) => start_idx.saturating_add(lim as usize).min(matches.len()),
            None => matches.len(),
        };
        for (path, name) in &matches[start_idx..end_idx] {
            writer.write_event(&crate::ndjson_types::FileMatchEvent {
                r#type: "file_match",
                path: path.clone(),
                name: name.clone(),
                target: "files",
            })?;
        }
        let matched_emitted = (end_idx - start_idx) as u64;
        // `files_matched` describes files that contributed a published record,
        // so the page is what decides it: one file with two published names
        // counts once, and a file whose only names were paged out counts zero.
        let matched_files_published = {
            let mut paths: Vec<&str> = matches[start_idx..end_idx]
                .iter()
                .map(|(path, _)| path.as_str())
                .collect();
            paths.sort_unstable();
            paths.dedup();
            paths.len() as u64
        };

        if matches!(args.target, SearchTarget::Files) {
            let failures = std::mem::take(
                &mut *failures
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            );
            let base = Summary {
                r#type: "summary",
                // Every file the walk handed over was examined;
                // `files_matched` counts the files that contributed a
                // published name, and the page omitted by `--offset` and
                // `--limit` is what `offset_skipped` and `total_matches`
                // describe.
                files_visited: files.len() as u64,
                files_matched: matched_files_published,
                files_modified: None,
                files_skipped: None,
                total_matches: Some(matched_emitted),
                total_replacements: None,
                offset_skipped: Some(start_idx as u64),
                elapsed_ms: start.elapsed().as_millis() as u64,
                // This path collects every name hit and slices the page
                // afterwards, so it never cuts its own walk: reporting the
                // writer's budget here said "stopped early" on a walk that
                // read the whole tree. The omitted hits are visible in
                // `files_matched` versus `total_matches` of the full run.
                walker_stopped_early: None,
                ..Summary::default()
            };
            // A name run with no hit still answered "no name matches", so it
            // keeps its zero status; a missing root is what fails it.
            finish_search_run(
                writer,
                base,
                failures,
                missing_roots.len() as u64,
                true,
                args.format == crate::cli::SearchFormat::Text,
            )?;
            return Ok(true);
        }
    }
    Ok(false)
}
