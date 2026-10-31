// SPDX-License-Identifier: MIT OR Apache-2.0

//! Grammatical scoping: AST-based code selection and transformation.
//! Workload: mixed I/O-bound + CPU-bound (file reading + AST traversal via ast-grep + atomic write).
//! Parallelism: `ignore::WalkParallel` + bounded channel; bound via
//! `concurrency::apply_walk_threads` (`--threads` / `--max-concurrency`).

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::Result;
use ast_grep_core::AstGrep;
use ast_grep_core::matcher::Pattern;
use ast_grep_language::SupportLang;
use unicode_normalization::UnicodeNormalization;

use clap::ValueHint;

use crate::atomic::AtomicWriteOptions;
use crate::best_effort::send_to_consumer;
use crate::checksum;
use crate::cli::GlobalArgs;
use crate::cli_args::BackupOpts;
use crate::commands::resolve_backup;
use crate::error::AtomwriteError;
use crate::ndjson_types::{ScopeResult, Summary};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for the scope subcommand.
#[derive(clap::Args, Debug)]
pub struct ScopeArgs {
    /// Paths to search within (at least one; omitting them is a usage error).
    #[arg(required = true, value_hint = ValueHint::AnyPath)]
    pub paths: Vec<std::path::PathBuf>,

    /// Source language for AST parsing.
    /// `--lang` is an alias for `--language` (the global locale flag is
    /// `--locale`). Both `--lang rust` and `--language rust` are
    /// accepted; use the short form `-l rust` for terse scripts.
    #[arg(
        short = 'l',
        long = "language",
        alias = "lang",
        required = true,
        help = "Language (rust, py, ts, go, c, etc); accepts --lang as alias"
    )]
    pub language: String,

    /// Prepared query name (e.g. comments, strings, fn, pub-fn).
    #[arg(
        long,
        help = "Prepared query name (comments, strings, fn, pub-fn, etc)"
    )]
    pub query: Option<String>,

    /// Custom AST pattern to match (same syntax as transform).
    #[arg(long, help = "Custom AST pattern to match")]
    pub pattern: Option<String>,

    /// Delete matched content.
    #[arg(long, help = "Delete all matched content", action = clap::ArgAction::SetTrue)]
    pub delete: bool,

    /// Action to apply on matched content.
    #[arg(
        long,
        value_enum,
        help = "Transform action: upper, lower, titlecase, squeeze, symbols, normalize"
    )]
    pub action: Option<ScopeAction>,

    /// Replacement text for matched content.
    #[arg(long, help = "Replace matched content with this text")]
    pub replace_with: Option<String>,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Shared backup flags.
    #[command(flatten)]
    pub backup_opts: BackupOpts,

    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,
}

/// Available actions for the scope subcommand.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ScopeAction {
    /// Convert to uppercase.
    Upper,
    /// Convert to lowercase.
    Lower,
    /// Convert to title case.
    Titlecase,
    /// Collapse consecutive repeated whitespace.
    Squeeze,
    /// Convert ASCII symbols to Unicode equivalents.
    Symbols,
    /// NFC Unicode normalization.
    Normalize,
}

/// Execute the scope subcommand.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` for unknown language, query, or pattern.
#[tracing::instrument(skip_all, fields(command = "scope"))]
pub fn cmd_scope(
    args: &ScopeArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    let start = Instant::now();
    let resolved = resolve_backup(&args.backup_opts, defaults);

    let lang = parse_language(&args.language)?;
    let pattern_strs = resolve_patterns(&args.query, &args.pattern, &args.language)?;
    let patterns: Vec<Pattern> = pattern_strs
        .iter()
        .map(|ps| {
            Pattern::try_new(ps, lang).map_err(|e| {
                anyhow::Error::from(crate::error::AtomwriteError::invalid(
                    "invalid scope pattern",
                    e,
                ))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let query_name = args.query.clone().unwrap_or_else(|| "custom".to_owned());

    let workspace = global.resolve_workspace()?;

    let canonical_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;
    // A root the caller named and the run never reached is a caller mistake,
    // not a scan failure: it is counted before the walk and only the roots
    // that exist are walked.
    let (canonical_paths, missing_roots) =
        crate::commands::path_resolution::split_missing_roots(canonical_paths);
    tracing::debug!(
        present = canonical_paths.len(),
        missing = missing_roots.len(),
        "scope roots partitioned"
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
            total_replacements: None,
            files_read: Some(0),
            elapsed_ms: start.elapsed().as_millis() as u64,
            walker_stopped_early: None,
            ..Summary::default()
        };
        let mut tally = crate::ndjson_types::FailureTally::default();
        tally.record_missing_roots(&missing_roots);
        let (roots_count, roots_items, _) = tally.into_parts();
        let files_failed = Some(roots_count);
        let summary = ScopeSummary {
            base,
            files_failed,
            partial: crate::ndjson_types::partial_of(files_failed),
            errors: (!roots_items.is_empty()).then_some(roots_items),
        };
        let path = missing_roots
            .first()
            .cloned()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let err = crate::error::AtomwriteError::NotFound { path };
        return Err(writer.publish_failure(&summary, err.into()));
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
        crossbeam_channel::bounded::<ScopeEvent>(crate::concurrency::event_channel_cap());

    // Per-run counters. Ordering::Relaxed: independent tallies; final loads
    // run after the parallel join (no data-publication barrier needed).
    let files_visited = Arc::new(AtomicU64::new(0));
    let files_modified = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));
    let files_failed = Arc::new(AtomicU64::new(0));
    // Files whose content was actually read: the run produced an answer for
    // them, which is what decides whether a read failure elsewhere fails it.
    let files_read = Arc::new(AtomicU64::new(0));
    // Scopes matched across every file, which is the run's match total.
    let total_matches = Arc::new(AtomicU64::new(0));
    // Files that contributed at least one published answer record. This is the
    // summary's `files_matched`, and it never counts a failure or a skip.
    let matched_files = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let fm = Arc::clone(&files_modified);
    let fs = Arc::clone(&files_skipped);
    let ff = Arc::clone(&files_failed);
    let fr = Arc::clone(&files_read);
    let tm = Arc::clone(&total_matches);
    let mf = Arc::clone(&matched_files);
    // An ignore file the walk half-reads is a hole too, and `ignore` never
    // reports it by itself.
    for (path, err) in crate::concurrency::ignore_file_failures(&canonical_paths) {
        ff.fetch_add(1, Ordering::Relaxed);
        crate::best_effort::send_to_consumer(&tx, ScopeEvent::ReadFailed(path, err));
    }
    let delete = args.delete;
    let action = args.action;
    let replace_with: Option<Arc<str>> = args.replace_with.clone().map(Into::into);
    let dry_run = args.dry_run;
    let strict_atomic = args.strict.strict_atomic;
    let backup = resolved.backup;
    let ws: Arc<std::path::Path> = Arc::from(workspace.as_path());
    let qn: Arc<str> = query_name.into();
    let lang_name: Arc<str> = args.language.clone().into();
    let write_cfg: Arc<crate::config::WriteSection> = Arc::new(write_cfg.clone());

    let max_size = global.effective_max_filesize();
    let shutdown_flag = shutdown.flag();
    // Private stop flag for the join below: setting the shared shutdown flag
    // would make the whole process look cancelled.
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let walker_stop = Arc::clone(&stop);
    // Set at the moment a walker thread returns `WalkState::Quit`.
    //
    // `walker_stopped_early` used to be stamped from `output::limit_reached()`,
    // which records that the writer dropped a record. Those are different
    // facts: the writer can drop one record while the walk still ends on its
    // own, and a walk that quits after visiting part of the tree can still
    // have an unspent writer budget. Only the walker knows whether the walker
    // quit, so the `Quit` in this file records it here.
    let walk_quit = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let walk_quit_flag = Arc::clone(&walk_quit);
    let patterns = Arc::new(patterns);
    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let patterns = Arc::clone(&patterns);
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let fm = Arc::clone(&fm);
            let fs = Arc::clone(&fs);
            let ff = Arc::clone(&ff);
            let fr = Arc::clone(&fr);
            let tm = Arc::clone(&tm);
            let mf = Arc::clone(&mf);
            let replace_with = replace_with.clone();
            let ws = Arc::clone(&ws);
            let qn = Arc::clone(&qn);
            let lang_name = Arc::clone(&lang_name);
            let write_cfg = Arc::clone(&write_cfg);
            let shutdown_flag = Arc::clone(&shutdown_flag);
            let stop = Arc::clone(&walker_stop);
            let walk_quit_flag = Arc::clone(&walk_quit_flag);

            Box::new(move |entry| {
                if shutdown_flag.load(Ordering::Acquire)
                    || stop.load(Ordering::Acquire)
                    || crate::output::limit_reached()
                {
                    // Recorded here and only here: this is the point where an
                    // entry is dropped unvisited, which is what "stopped early"
                    // means to a reader of the summary.
                    walk_quit_flag.store(true, Ordering::Release);
                    return ignore::WalkState::Quit;
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        // Same rule the read path follows: a subtree the walk
                        // cannot open is reported, never skipped in silence.
                        let (path, err) = crate::concurrency::classify_walk_error(&e);
                        ff.fetch_add(1, Ordering::Relaxed);
                        crate::best_effort::send_to_consumer(
                            &tx,
                            ScopeEvent::ReadFailed(path, err),
                        );
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
                    Err(e) => {
                        if crate::output::is_content_rejection(&e) {
                            // Too large, binary or not text: skipped by design.
                            fs.fetch_add(1, Ordering::Relaxed);
                        } else {
                            // Could not open or read: the summary reports it.
                            ff.fetch_add(1, Ordering::Relaxed);
                            crate::best_effort::send_to_consumer(
                                &tx,
                                ScopeEvent::ReadFailed(path.clone(), e),
                            );
                        }
                        return ignore::WalkState::Continue;
                    }
                };
                fr.fetch_add(1, Ordering::Relaxed);

                let grep = AstGrep::new(&content, lang);
                let root = grep.root();
                let matches: Vec<_> = patterns.iter().flat_map(|p| root.find_all(p)).collect();

                if matches.is_empty() {
                    fs.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                let scopes_matched = matches.len() as u64;
                tm.fetch_add(scopes_matched, Ordering::Relaxed);
                mf.fetch_add(1, Ordering::Relaxed);

                let is_read_only = !delete && action.is_none() && replace_with.is_none();
                if is_read_only {
                    fm.fetch_add(1, Ordering::Relaxed);
                    let checksum = checksum::hash_bytes(content.as_bytes());
                    send_to_consumer(
                        &tx,
                        ScopeEvent::Result(Box::new(ScopeResult {
                            r#type: "scoped",
                            path: path.display().to_string(),
                            language: lang_name.to_string(),
                            query: qn.to_string(),
                            action: "none".to_owned(),
                            scopes_matched,
                            bytes_before: content.len() as u64,
                            bytes_after: content.len() as u64,
                            checksum_before: checksum.clone(),
                            checksum_after: checksum,
                            elapsed_ms: file_start.elapsed().as_millis() as u64,
                            risk_assessment: None,
                        })),
                    );
                    return ignore::WalkState::Continue;
                }

                let mut edits: Vec<(usize, usize, String)> = Vec::with_capacity(matches.len());

                for m in &matches {
                    let range = m.range();
                    let (effective_start, effective_end) = if delete {
                        expand_to_full_line(&content, range.start, range.end)
                    } else {
                        (range.start, range.end)
                    };
                    let matched_text = &content[effective_start..effective_end];
                    let replacement =
                        apply_scope_action(matched_text, delete, action, replace_with.as_deref());
                    edits.push((effective_start, effective_end, replacement.into_owned()));
                }

                edits.sort_by_key(|e| std::cmp::Reverse(e.0));

                let checksum_before = checksum::hash_bytes(content.as_bytes());
                let bytes_before = content.len() as u64;
                let mut content = content; // rebind as mut — no clone
                for (s, e, replacement) in &edits {
                    content.replace_range(*s..*e, replacement);
                }
                let checksum_after = checksum::hash_bytes(content.as_bytes());

                // Risk only: `scope` rewrites matched regions, so the
                // whole-content size barriers do not apply. Assessed before the
                // write branch so a dry-run carries the same diagnostic.
                let risk_assessment = crate::commands::write_guards::assess_content_risk(
                    content.as_bytes(),
                    bytes_before,
                    content.len() as u64,
                    write_cfg.as_ref(),
                );

                if checksum_before == checksum_after {
                    // O(1) hash comparison instead of O(n) string comparison
                    fs.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                if !dry_run {
                    let opts = AtomicWriteOptions {
                        backup,
                        strict_atomic,
                        ..Default::default()
                    };
                    // A1: refuse to overwrite a file that changed since it was read.
                    if let Err(e) = crate::atomic::atomic_write_based(
                        &path,
                        content.as_bytes(),
                        &opts,
                        &ws,
                        Some(&checksum_before),
                    ) {
                        tracing::warn!(path = %path.display(), error = %e, "scope write failed");
                        ff.fetch_add(1, Ordering::Relaxed);
                        // Receiver may have dropped during shutdown — send failure is expected
                        send_to_consumer(&tx, ScopeEvent::Failed(path, e));
                        return ignore::WalkState::Continue;
                    }
                }
                // Counted only after the write succeeded (or on dry-run).
                fm.fetch_add(1, Ordering::Relaxed);

                let action_name = if delete {
                    "delete"
                } else if replace_with.is_some() {
                    "replace"
                } else {
                    match action {
                        Some(ScopeAction::Upper) => "upper",
                        Some(ScopeAction::Lower) => "lower",
                        Some(ScopeAction::Titlecase) => "titlecase",
                        Some(ScopeAction::Squeeze) => "squeeze",
                        Some(ScopeAction::Symbols) => "symbols",
                        Some(ScopeAction::Normalize) => "normalize",
                        None => "none",
                    }
                };

                // Receiver may have dropped during shutdown — send failure is expected
                send_to_consumer(
                    &tx,
                    ScopeEvent::Result(Box::new(ScopeResult {
                        r#type: "scoped",
                        path: path.display().to_string(),
                        language: lang_name.to_string(),
                        query: qn.to_string(),
                        action: action_name.to_owned(),
                        scopes_matched,
                        bytes_before,
                        bytes_after: content.len() as u64,
                        checksum_before,
                        checksum_after,
                        elapsed_ms: file_start.elapsed().as_millis() as u64,
                        risk_assessment,
                    })),
                );

                ignore::WalkState::Continue
            })
        });
    });

    // Join on every path (v0.1.37): a failed write to stdout must not leave the
    // walker thread running behind a dropped receiver.
    let drained = drain_scope_events(&rx, writer, shutdown);
    crate::concurrency::join_walker(rx, &stop, walker_thread);
    let (failures, bytes_after_total) = drained?;
    // Named roots that do not exist are counted here, so the summary reports
    // them the same way it reports a file it could not read.
    let mut root_tally = crate::ndjson_types::FailureTally::default();
    root_tally.record_missing_roots(&missing_roots);
    let (roots_missing, roots_items, roots_first) = root_tally.into_parts();

    // A-DRY-002: use shared dry-run honesty helper (never claim mutations on dry-run).
    let modified = files_modified.load(Ordering::Relaxed);
    let failed = files_failed.load(Ordering::Relaxed);
    let failed_total = failed.saturating_add(roots_missing);
    let is_read_only = !args.delete && args.action.is_none() && args.replace_with.is_none();
    let files_modified_summary = if is_read_only {
        None
    } else {
        crate::commands::summary_metrics::files_modified_for_summary(modified, args.dry_run)
    };
    // A read-only scope changes nothing, so it has no blast radius to preview.
    let files_planned = if is_read_only {
        None
    } else {
        crate::commands::summary_metrics::files_planned_for_summary(modified, args.dry_run)
    };
    let files_failed = (failed_total > 0).then_some(failed_total);
    let summary = ScopeSummary {
        base: Summary {
            r#type: "summary",
            files_visited: files_visited.load(Ordering::Relaxed),
            files_matched: matched_files.load(Ordering::Relaxed),
            files_modified: files_modified_summary,
            files_planned,
            files_skipped: Some(files_skipped.load(Ordering::Relaxed)),
            total_matches: Some(total_matches.load(Ordering::Relaxed)),
            total_replacements: None,
            files_read: Some(files_read.load(Ordering::Relaxed)),
            total_bytes: Some(bytes_after_total),
            elapsed_ms: start.elapsed().as_millis() as u64,
            walker_stopped_early: walk_quit.load(Ordering::Acquire).then_some(true),
            ..Summary::default()
        },
        files_failed,
        partial: crate::ndjson_types::partial_of(files_failed),
        errors: if roots_items.is_empty() && failures.items.is_empty() {
            None
        } else {
            let mut items = roots_items;
            items.extend(failures.items);
            items.truncate(crate::config::summary_errors_max_entries());
            Some(items)
        },
    };

    // A rewrite that landed is an answer, so a later write error joins the
    // `errors[]` list instead of discarding the files already rewritten, which
    // is what `replace`, `transform` and `case` already do. A file that could
    // not be READ only fails the run when no file was read at all.
    let wrote_something = files_modified.load(Ordering::Relaxed) > 0;
    let nothing_done = !wrote_something
        && files_skipped.load(Ordering::Relaxed) == files_visited.load(Ordering::Relaxed);
    let any_read = files_read.load(Ordering::Relaxed) > 0;
    // A root named in the invocation that does not exist outranks a rewrite
    // that landed: the caller asked about a place the run never reached.
    let failure = roots_first
        .or_else(|| {
            if wrote_something {
                None
            } else {
                failures.first_write
            }
        })
        .or_else(|| if any_read { None } else { failures.first_read });
    let failure = failure.or_else(|| {
        nothing_done.then(|| anyhow::Error::from(crate::error::AtomwriteError::NoMatches))
    });
    // R7: a summary that precedes an error envelope says `ok: false` itself.
    let Some(err) = failure else {
        writer.write_terminal(&summary)?;
        return Ok(());
    };
    Err(writer.publish_failure(&summary, err))
}

include!("scope_actions.inc.rs");

include!("queries.inc.rs");

include!("tests.inc.rs");
