// SPDX-License-Identifier: MIT OR Apache-2.0

//! Structural AST search and rewrite via ast-grep.
//! Workload: mixed I/O-bound + CPU-bound (file reading + AST parsing via ast-grep + atomic write).
//! Parallelism: `ignore::WalkParallel` + bounded channel; bound via
//! `concurrency::apply_walk_threads` (`--threads` / `--max-concurrency`).

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use ast_grep_core::AstGrep;
use ast_grep_core::matcher::Pattern;
use ast_grep_language::SupportLang;

use crate::atomic::AtomicWriteOptions;
use crate::checksum;
use crate::cli::{GlobalArgs, TransformArgs};
use crate::commands::resolve_backup;
use crate::ndjson_types::{FailureTally, Summary, TransformResult};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// A single rule in the multi-rule YAML manifest (G44).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct YamlRule {
    /// Source language (e.g. `"rust"`, `"python"`).
    pub language: String,
    /// AST pattern to match.
    pub pattern: String,
    /// Rewrite template.
    pub rewrite: String,
    /// Optional human-readable rule id, emitted in NDJSON events.
    #[serde(default)]
    pub id: Option<String>,
}

/// One rule's run: its summary, the first failure and the named roots that did not exist.
///
/// The third field is a list instead of a count so the caller can name the
/// root in the error envelope instead of reporting a bare number.
pub struct TransformRun {
    /// Terminal summary of the rule.
    pub summary: crate::ndjson_types::FailureSummary,
    /// First failure the run recorded, if any.
    pub first_failure: Option<anyhow::Error>,
    /// Roots named in the invocation that do not exist on disk.
    pub missing_roots: Vec<std::path::PathBuf>,
}

/// Perform structural code search and rewrite via AST patterns.
///
/// In single-rule mode (default), pass `--pattern`, `--rewrite`, and
/// `--language`. In multi-rule mode (G44), pass `--rules PATH` or
/// `--inline-rules "YAML"` to apply multiple rules in sequence.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::InvalidInput` if the AST pattern or language is invalid.
/// Returns `AtomwriteError::Io` if reading or writing files fails.
#[tracing::instrument(skip_all, fields(command = "transform"))]
pub fn cmd_transform(
    args: &TransformArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    // G44: multi-rule mode dispatch.
    if args.rules.is_some() || args.inline_rules.is_some() {
        return cmd_transform_multi(args, global, writer, shutdown, defaults, write_cfg);
    }

    let run = transform_one_rule(args, global, writer, shutdown, defaults, write_cfg)?;
    let TransformRun {
        summary,
        first_failure,
        missing_roots,
    } = run;

    // A cancelled run publishes no summary; `run` writes the cancel envelope.
    if shutdown.is_shutdown() {
        return Ok(());
    }
    // A root named in the invocation that does not exist outranks a rewrite
    // that landed, so the run fails even when it matched.
    if let Some(first_missing) = missing_roots.first() {
        let err = crate::error::AtomwriteError::NotFound {
            path: first_missing.clone(),
        };
        return Err(writer.publish_failure(&summary, err.into()));
    }

    // A rewrite that landed is an answer the run did produce, and the summary
    // names every file it could not read, check or write in `files_failed` and
    // `errors[]`. Failing the run would throw that answer away.
    if summary.base.total_replacements.unwrap_or(0) > 0 {
        writer.write_terminal(&summary)?;
        return Ok(());
    }
    // With nothing rewritten the run has no answer: the first failure explains
    // why, and only a run that reached every file falls through to NO_MATCHES.
    if let Some(err) = first_failure {
        return Err(writer.publish_failure(&summary, err));
    }
    // R7: zero matches is a failed run, so the summary says `ok: false` and is
    // the one terminal event.
    Err(writer.publish_failure(&summary, crate::error::AtomwriteError::NoMatches.into()))
}

/// Run one rule and return its summary WITHOUT writing a terminal event.
///
/// The caller owns the terminal record. Single-rule mode publishes it right
/// away; multi-rule mode sums every rule's summary into one, so a run of five
/// rules does not end with five `summary` records that each claim to describe
/// the whole invocation.
///
/// # Errors
///
/// Returns the same errors as [`cmd_transform`] for a language, pattern or
/// path that cannot be used; a per-file failure travels in the second tuple
/// element instead, because the summary still describes what did run.
fn transform_one_rule(
    args: &TransformArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    write_cfg: &crate::config::WriteSection,
) -> Result<TransformRun> {
    let resolved_backup = resolve_backup(&args.backup_opts, defaults);

    // Single-rule mode: validate the three required flags.
    let pattern_str =
        args.pattern
            .as_deref()
            .ok_or_else(|| crate::error::AtomwriteError::InvalidInput {
                reason:
                    "--pattern is required (or use --rules / --inline-rules for multi-rule mode)"
                        .into(),
            })?;
    let rewrite_str =
        args.rewrite
            .as_deref()
            .ok_or_else(|| crate::error::AtomwriteError::InvalidInput {
                reason:
                    "--rewrite is required (or use --rules / --inline-rules for multi-rule mode)"
                        .into(),
            })?;
    let lang_str =
        args.language
            .as_deref()
            .ok_or_else(|| crate::error::AtomwriteError::InvalidInput {
                reason:
                    "--language is required (or use --rules / --inline-rules for multi-rule mode)"
                        .into(),
            })?;

    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let lang: SupportLang = crate::lang_utils::parse_language(lang_str)?;

    let pattern = Pattern::try_new(pattern_str, lang)
        .map_err(|e| crate::error::AtomwriteError::invalid("invalid pattern", e))?;

    let canonical_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;

    // A root the caller named and the run never reached is a caller mistake,
    // not a scan failure, so it is counted before the walk.
    let (canonical_paths, missing_roots) =
        crate::commands::path_resolution::split_missing_roots(canonical_paths);
    tracing::debug!(
        present = canonical_paths.len(),
        missing = missing_roots.len(),
        "transform roots partitioned"
    );
    if canonical_paths.is_empty() {
        // The walker indexes the first root, so an empty list is not walked.
        let base = Summary {
            r#type: "summary",
            files_visited: 0,
            files_matched: 0,
            files_modified: None,
            files_skipped: Some(0),
            total_matches: Some(0),
            total_replacements: Some(0),
            elapsed_ms: start.elapsed().as_millis() as u64,
            walker_stopped_early: None,
            ..Summary::default()
        };
        let mut tally = crate::ndjson_types::FailureTally::default();
        tally.record_missing_roots(&missing_roots);
        let (summary, first_failure) = tally.into_summary(base);
        return Ok(TransformRun {
            summary,
            first_failure,
            missing_roots,
        });
    }

    let mut walker = crate::commands::build_scoped_walker(
        &canonical_paths,
        global,
        &args.include,
        &args.exclude,
    )?;

    walker.types(crate::lang_utils::file_types(lang));

    crate::lang_utils::ensure_targets_match(&canonical_paths, lang)?;

    let (tx, rx) =
        crossbeam_channel::bounded::<TransformEvent>(crate::concurrency::event_channel_cap());

    // Per-run counters. Ordering::Relaxed: independent tallies; final loads
    // run after the parallel join (no data-publication barrier needed).
    let files_visited = Arc::new(AtomicU64::new(0));
    let files_transformed = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));
    let total_replacements = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let ft = Arc::clone(&files_transformed);
    let tb = Arc::clone(&total_bytes);
    let fs = Arc::clone(&files_skipped);
    let tr = Arc::clone(&total_replacements);
    let rewrite: Arc<str> = rewrite_str.to_owned().into();
    let language: Arc<str> = lang_str.to_owned().into();
    let dry_run = args.dry_run;
    let strict_atomic = args.strict.strict_atomic;
    let ws: Arc<std::path::Path> = Arc::from(workspace.as_path());
    let write_cfg: Arc<crate::config::WriteSection> = Arc::new(write_cfg.clone());

    let max_size = global.effective_max_filesize();
    let shutdown_flag = shutdown.flag();
    let stop = Arc::new(AtomicBool::new(false)); // R4: set when the consumer exits
    let walker_stop = Arc::clone(&stop);
    let backup_flag = resolved_backup.backup;
    let verify_parse = args.verify_parse;
    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let pattern = pattern.clone();
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let ft = Arc::clone(&ft);
            let fs = Arc::clone(&fs);
            let tr = Arc::clone(&tr);
            let tb = Arc::clone(&tb);
            let rewrite = Arc::clone(&rewrite);
            let language = Arc::clone(&language);
            let ws = Arc::clone(&ws);
            let write_cfg = Arc::clone(&write_cfg);
            let shutdown_flag = Arc::clone(&shutdown_flag);
            let stop = Arc::clone(&walker_stop);

            Box::new(move |entry| {
                if shutdown_flag.load(Ordering::Acquire)
                    || stop.load(Ordering::Acquire)
                    || crate::output::limit_reached()
                {
                    return ignore::WalkState::Quit;
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(walk_error) => {
                        if let Some((path, error)) =
                            crate::error::AtomwriteError::from_walk_error(&walk_error)
                        {
                            crate::best_effort::send_to_consumer(
                                &tx,
                                TransformEvent::Error {
                                    path,
                                    error: error.into(),
                                },
                            );
                        }
                        return ignore::WalkState::Continue;
                    }
                };

                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    return ignore::WalkState::Continue;
                }

                fv.fetch_add(1, Ordering::Relaxed);

                let path = entry.path().to_path_buf();
                let _span = tracing::debug_span!("process_file", path = %path.display()).entered();
                let file_start = Instant::now();

                let content = match crate::file_io::read_file_string(&path, max_size) {
                    Ok(c) => c,
                    Err(error) => {
                        fs.fetch_add(1, Ordering::Relaxed);
                        // A file that is too large or not text is skipped by
                        // design; one that could not be accessed is a failure.
                        if !crate::output::is_content_rejection(&error) {
                            // Receiver may have dropped during shutdown — send failure is expected
                            crate::best_effort::send_to_consumer(
                                &tx,
                                TransformEvent::Error { path, error },
                            );
                        }
                        return ignore::WalkState::Continue;
                    }
                };

                let grep = AstGrep::new(&content, lang);
                let root = grep.root();
                let edits = root.replace_all(&pattern, &*rewrite);

                if edits.is_empty() {
                    fs.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                let match_count = edits.len() as u64;
                // `--limit` is a work budget: the claim comes BEFORE the rewrite
                // (and before a dry-run record), so the run reports and touches
                // exactly what it was allowed.
                if !crate::output::claim_work() {
                    return ignore::WalkState::Quit;
                }
                let checksum_before = checksum::hash_bytes(content.as_bytes());
                let bytes_before = content.len() as u64;

                let mut content = content; // rebind as mut — no clone
                let mut sorted_edits = edits;
                sorted_edits.sort_by_key(|e| std::cmp::Reverse(e.position));
                for edit in &sorted_edits {
                    let start = edit.position;
                    let end = start + edit.deleted_length;
                    let replacement = String::from_utf8_lossy(&edit.inserted_text);
                    content.replace_range(start..end, &replacement);
                }

                let checksum_after = checksum::hash_bytes(content.as_bytes());

                if verify_parse {
                    if let Ok(crate::syntax_check::SyntaxCheckResult::Errors { count, .. }) =
                        crate::syntax_check::syntax_check(&path, content.as_bytes())
                    {
                        let error = crate::error::AtomwriteError::SyntaxError {
                            path: path.clone(),
                            count: u32::try_from(count).unwrap_or(u32::MAX),
                        };
                        crate::best_effort::send_to_consumer(
                            &tx,
                            TransformEvent::Error {
                                path,
                                error: error.into(),
                            },
                        );
                        return ignore::WalkState::Continue;
                    }
                }

                // Assessed before the write branch on purpose: a dry-run is the
                // run an agent inspects to decide whether to write at all, so it
                // is the one that most needs the diagnostic. The AST rewrite
                // changes only the matched regions, so the whole-content size
                // barriers do not apply and this is risk only.
                let risk_assessment = crate::commands::write_guards::assess_content_risk(
                    content.as_bytes(),
                    bytes_before,
                    content.len() as u64,
                    write_cfg.as_ref(),
                );

                if !dry_run {
                    if stop.load(Ordering::Acquire) {
                        return ignore::WalkState::Quit;
                    }
                    let validated = match crate::path_safety::validate_path(&path, &ws) {
                        Ok(p) => p,
                        Err(error) => {
                            crate::best_effort::send_to_consumer(
                                &tx,
                                TransformEvent::Error { path, error },
                            );
                            return ignore::WalkState::Continue;
                        }
                    };
                    let opts = AtomicWriteOptions {
                        backup: backup_flag,
                        strict_atomic,
                        ..Default::default()
                    };
                    // A1: refuse to overwrite a file that changed since it was read.
                    if let Err(error) = crate::atomic::atomic_write_based(
                        &validated,
                        content.as_bytes(),
                        &opts,
                        &ws,
                        Some(&checksum_before),
                    ) {
                        crate::best_effort::send_to_consumer(
                            &tx,
                            TransformEvent::Error { path, error },
                        );
                        return ignore::WalkState::Continue;
                    }
                }

                tr.fetch_add(match_count, Ordering::Relaxed);
                ft.fetch_add(1, Ordering::Relaxed);
                tb.fetch_add(content.len() as u64, Ordering::Relaxed);

                crate::best_effort::send_to_consumer(
                    &tx,
                    TransformEvent::Transformed {
                        path,
                        language: language.to_string(),
                        matches: match_count,
                        bytes_before,
                        bytes_after: content.len() as u64,
                        checksum_before,
                        checksum_after,
                        elapsed_ms: file_start.elapsed().as_millis() as u64,
                        risk_assessment,
                    },
                );

                ignore::WalkState::Continue
            })
        });
    });

    let drained = drain_transform_events(&rx, writer, shutdown);
    crate::concurrency::join_walker(rx, &stop, walker_thread);
    let failures = drained?;

    let total_repl = total_replacements.load(Ordering::Relaxed);
    let matched = files_transformed.load(Ordering::Relaxed);
    // B-002 / R-DRY-001: dry-run must not claim disk mutations.
    let files_modified =
        crate::commands::summary_metrics::files_modified_for_summary(matched, dry_run);
    let files_planned =
        crate::commands::summary_metrics::files_planned_for_summary(matched, dry_run);

    let base = Summary {
        r#type: "summary",
        files_visited: files_visited.load(Ordering::Relaxed),
        files_matched: matched,
        files_modified,
        files_planned,
        files_skipped: Some(files_skipped.load(Ordering::Relaxed)),
        total_matches: Some(total_repl),
        total_replacements: Some(total_repl),
        total_bytes: Some(total_bytes.load(Ordering::Relaxed)),
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        elapsed_ms: start.elapsed().as_millis() as u64,
        walker_stopped_early: crate::output::limit_reached().then_some(true),
        ..Summary::default()
    };
    // A file that could not be read, checked or written is reported in the
    // summary instead of staying in the log.
    let mut failures = failures;
    // An ignore file the walk half-reads is a hole too, and `ignore` never
    // reports it by itself.
    crate::concurrency::record_ignore_file_failures_into(&canonical_paths, &mut failures);
    failures.record_missing_roots(&missing_roots);
    let (summary, first_failure) = failures.into_summary(base);
    Ok(TransformRun {
        summary,
        first_failure,
        missing_roots,
    })
}

include!("transform_multi.inc.rs");
