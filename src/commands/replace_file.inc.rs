// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by replace_run.inc.rs (file-size split, v0.1.37).
//
// The per-file worker steps: reading and gating one candidate file, and applying
// the replacement to its content. Delivery and the walk live in replace_run.inc.rs.

/// Jail check, bounded read and binary rejection for one file.
///
/// Returns the text content, or `None` after counting the skip and emitting
/// the error event when the file must not be processed.
fn read_candidate(
    job: &ReplaceJob,
    counters: &ReplaceCounters,
    tx: &crossbeam_channel::Sender<ReplaceEvent>,
    path: &std::path::Path,
) -> Option<String> {
    // Validate path against workspace jail BEFORE processing
    if crate::path_safety::validate_path(path, &job.workspace).is_err() {
        counters.files_skipped.fetch_add(1, Ordering::Relaxed);
        crate::best_effort::send_to_consumer(
            tx,
            ReplaceEvent::Error {
                path: path.to_path_buf(),
                kind: ReplaceErrorKind::JailViolation,
            },
        );
        return None;
    }

    let content = match crate::file_io::read_file_string(path, job.max_size) {
        Ok(c) => c,
        Err(error) => {
            counters.files_skipped.fetch_add(1, Ordering::Relaxed);
            // A file that is too large or not text is skipped by design; one
            // that could not be accessed is a failure the summary reports.
            if !crate::output::is_content_rejection(&error) {
                crate::best_effort::send_to_consumer(
                    tx,
                    ReplaceEvent::Error {
                        path: path.to_path_buf(),
                        kind: ReplaceErrorKind::Failed(error),
                    },
                );
            }
            return None;
        }
    };

    // G-015 / G-027: refuse binary content with permanent BINARY_FILE taxonomy.
    if crate::binary_detect::is_binary(content.as_bytes()) {
        counters.files_skipped.fetch_add(1, Ordering::Relaxed);
        counters.binary_rejects.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut guard) = counters.first_binary_path.lock() {
            if guard.is_none() {
                *guard = Some(path.to_path_buf());
            }
        }
        crate::best_effort::send_to_consumer(
            tx,
            ReplaceEvent::Error {
                path: path.to_path_buf(),
                kind: ReplaceErrorKind::BinaryRejected,
            },
        );
        return None;
    }

    Some(content)
}

/// Apply the replacement to one file's content: regex, or exact then fuzzy.
fn replace_in_content<'a>(
    job: &ReplaceJob,
    content: &'a str,
    cancel: &mut crate::signal::CancelCheck,
) -> FileReplacement<'a> {
    let mut fuzzy_meta: Option<(bool, String, Option<f64>, u64)> = None;
    let mut word_ignored = false;
    let mut cancelled = false;
    let mut failure: Option<anyhow::Error> = None;
    // Matches the file still had after `--max-replacements`; the count is a
    // floor because a match inside replaced text is not re-scanned.
    let mut suppressed = 0u64;
    let (replaced, count) = if job.use_regex {
        match apply_replacement_cancellable(
            &job.pattern,
            content,
            &job.replacement,
            job.max_replacements,
            job.preserve_case,
            Some(&mut *cancel),
        ) {
            Ok((text, applied, cut)) => {
                suppressed = cut;
                (text, applied)
            }
            Err(_) => {
                cancelled = true;
                (Cow::Borrowed(content), 0)
            }
        }
    } else {
        // Fixed string: exact multi first, then fuzzy cascade if zero hits.
        let exact_result = apply_replacement_cancellable(
            &job.pattern,
            content,
            &job.replacement,
            job.max_replacements,
            job.preserve_case,
            Some(&mut *cancel),
        );
        match exact_result {
            Err(_) => {
                cancelled = true;
                (Cow::Borrowed(content), 0)
            }
            Ok((exact, exact_count, cut)) if exact_count > 0 => {
                suppressed = cut;
                (exact, exact_count)
            }
            Ok(_) => {
                // v0.1.33 one-shot: never re-scan inserted replacement text.
                // Default max applies = 1; pat⊂rep forces 1; hard ceiling applies.
                if job.word {
                    word_ignored = true;
                }
                if cancel.check().is_err() {
                    cancelled = true;
                    (Cow::Borrowed(content), 0)
                } else {
                    match fuzzy::apply_fuzzy_one_pass(
                        content,
                        &job.pattern_text,
                        &job.replacement,
                        job.fuzzy_opts,
                        job.max_replacements,
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
                                (
                                    result.edited.map_or(Cow::Borrowed(content), Cow::Owned),
                                    result.applied,
                                )
                            } else {
                                (Cow::Borrowed(content), 0)
                            }
                        }
                        Err(crate::error::AtomwriteError::Cancelled { .. }) => {
                            cancelled = true;
                            (Cow::Borrowed(content), 0)
                        }
                        // Not "no match": the cascade could not run on this file
                        // (for example above the fuzzy content cap).
                        Err(error) => {
                            failure = Some(error.into());
                            (Cow::Borrowed(content), 0)
                        }
                    }
                }
            }
        }
    };

    FileReplacement {
        replaced,
        count,
        suppressed,
        fuzzy_meta,
        word_ignored,
        cancelled,
        failure,
    }
}
