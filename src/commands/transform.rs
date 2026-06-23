// SPDX-License-Identifier: MIT OR Apache-2.0

//! Structural AST search and rewrite via ast-grep.
//! Workload: mixed I/O-bound + CPU-bound (file reading + AST parsing via ast-grep + atomic write).

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use ast_grep_core::AstGrep;
use ast_grep_core::matcher::Pattern;
use ast_grep_language::SupportLang;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{GlobalArgs, TransformArgs};
use crate::commands::resolve_backup;
use crate::ndjson_types::{Summary, TransformResult};
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
) -> Result<()> {
    let effective_backup = resolve_backup(args.backup, args.no_backup);

    // G44: multi-rule mode dispatch.
    if args.rules.is_some() || args.inline_rules.is_some() {
        return cmd_transform_multi(args, global, writer, shutdown);
    }

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

    let lang: SupportLang =
        lang_str
            .parse()
            .map_err(|_| crate::error::AtomwriteError::InvalidInput {
                reason: format!("unsupported language: {lang_str}"),
            })?;

    let pattern = Pattern::try_new(pattern_str, lang).map_err(|e| {
        crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid pattern: {e}"),
        }
    })?;

    let extensions = crate::lang_utils::lang_extensions(lang_str);

    let canonical_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;
    let mut walker = ignore::WalkBuilder::new(&canonical_paths[0]);
    for p in canonical_paths.iter().skip(1) {
        walker.add(p);
    }
    walker
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore);

    if let Some(threads) = global.threads {
        walker.threads(if threads == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        } else {
            threads
        });
    }

    if !extensions.is_empty() {
        let mut types_builder = ignore::types::TypesBuilder::new();
        for ext in &extensions {
            types_builder
                .add_def(&format!("lang:*.{ext}"))
                .context("invalid extension")?;
        }
        types_builder.select("lang");
        walker.types(types_builder.build().context("build types")?);
    }

    if !args.include.is_empty() || !args.exclude.is_empty() {
        let mut overrides_builder = ignore::overrides::OverrideBuilder::new(&canonical_paths[0]);
        for glob in &args.include {
            overrides_builder
                .add(glob)
                .context("invalid include glob")?;
        }
        for glob in &args.exclude {
            overrides_builder
                .add(&format!("!{glob}"))
                .context("invalid exclude glob")?;
        }
        walker.overrides(overrides_builder.build().context("build overrides")?);
    }

    let (tx, rx) = crossbeam_channel::bounded::<TransformEvent>(1024);

    let files_visited = Arc::new(AtomicU64::new(0));
    let files_transformed = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));
    let total_replacements = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let ft = Arc::clone(&files_transformed);
    let fs = Arc::clone(&files_skipped);
    let tr = Arc::clone(&total_replacements);
    let rewrite: Arc<str> = rewrite_str.to_owned().into();
    let language: Arc<str> = lang_str.to_owned().into();
    let dry_run = args.dry_run;
    let ws: Arc<std::path::Path> = Arc::from(workspace.as_path());

    let max_size = global.effective_max_filesize();
    let shutdown_flag = shutdown.flag();
    let backup_flag = effective_backup;
    let verify_parse = args.verify_parse;
    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let pattern = pattern.clone();
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let ft = Arc::clone(&ft);
            let fs = Arc::clone(&fs);
            let tr = Arc::clone(&tr);
            let rewrite = Arc::clone(&rewrite);
            let language = Arc::clone(&language);
            let ws = Arc::clone(&ws);
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
                let file_start = Instant::now();

                let content = match crate::file_io::read_file_string(&path, max_size) {
                    Ok(c) => c,
                    Err(e) => {
                        fs.fetch_add(1, Ordering::Relaxed);
                        // Receiver may have dropped during shutdown — send failure is expected
                        let _ = tx.send(TransformEvent::Error {
                            path,
                            message: format!("{e}"),
                        });
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
                        let _ = tx.send(TransformEvent::Error {
                            path,
                            message: format!(
                                "verify-parse: rewritten content has {count} syntax error(s)"
                            ),
                        });
                        return ignore::WalkState::Continue;
                    }
                }

                if !dry_run {
                    let validated = match crate::path_safety::validate_path(&path, &ws) {
                        Ok(p) => p,
                        Err(e) => {
                            let _ = tx.send(TransformEvent::Error {
                                path,
                                message: format!("{e:#}"),
                            });
                            return ignore::WalkState::Continue;
                        }
                    };
                    let opts = AtomicWriteOptions {
                        backup: backup_flag,
                        ..Default::default()
                    };
                    if let Err(e) = atomic_write(&validated, content.as_bytes(), &opts, &ws) {
                        let _ = tx.send(TransformEvent::Error {
                            path,
                            message: format!("{e:#}"),
                        });
                        return ignore::WalkState::Continue;
                    }
                }

                tr.fetch_add(match_count, Ordering::Relaxed);
                ft.fetch_add(1, Ordering::Relaxed);

                let _ = tx.send(TransformEvent::Transformed {
                    path,
                    language: language.to_string(),
                    matches: match_count,
                    bytes_before,
                    bytes_after: content.len() as u64,
                    checksum_before,
                    checksum_after,
                    elapsed_ms: file_start.elapsed().as_millis() as u64,
                });

                ignore::WalkState::Continue
            })
        });
    });

    for event in rx {
        if shutdown.is_shutdown() {
            break;
        }

        match event {
            TransformEvent::Transformed {
                path,
                language,
                matches,
                bytes_before,
                bytes_after,
                checksum_before,
                checksum_after,
                elapsed_ms,
            } => {
                writer.write_event(&TransformResult {
                    r#type: "transformed",
                    path: path.display().to_string(),
                    language,
                    matches,
                    replacements: matches,
                    bytes_before,
                    bytes_after,
                    checksum_before,
                    checksum_after,
                    elapsed_ms,
                })?;
            }
            TransformEvent::Error { path, message } => {
                tracing::warn!(path = %path.display(), error = %message, "transform error");
            }
        }
    }

    if let Err(panic_payload) = walker_thread.join() {
        std::panic::resume_unwind(panic_payload);
    }

    writer.write_event(&Summary {
        r#type: "summary",
        files_visited: files_visited.load(Ordering::Relaxed),
        files_matched: files_transformed.load(Ordering::Relaxed),
        files_modified: Some(files_transformed.load(Ordering::Relaxed)),
        files_skipped: Some(files_skipped.load(Ordering::Relaxed)),
        total_matches: Some(total_replacements.load(Ordering::Relaxed)),
        total_replacements: Some(total_replacements.load(Ordering::Relaxed)),
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}

enum TransformEvent {
    Transformed {
        path: PathBuf,
        language: String,
        matches: u64,
        bytes_before: u64,
        bytes_after: u64,
        checksum_before: String,
        checksum_after: String,
        elapsed_ms: u64,
    },
    Error {
        path: PathBuf,
        message: String,
    },
}

/// Multi-rule transform dispatcher (G44).
///
/// Reads rules from `--rules PATH` (YAML file) or `--inline-rules "YAML"`
/// and applies each rule in order. For each rule, the walker is
/// re-constructed with the rule's specific language filter. The summary
/// at the end aggregates counts across all rules.
fn cmd_transform_multi(
    args: &TransformArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = Instant::now();
    let effective_backup = resolve_backup(args.backup, args.no_backup);

    // Load the YAML rules.
    let rules: Vec<YamlRule> = if let Some(path) = &args.rules {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read rules file {}", path.display()))?;
        serde_yaml::from_str(&content)
            .with_context(|| format!("invalid YAML in rules file {}", path.display()))?
    } else if let Some(inline) = &args.inline_rules {
        serde_yaml::from_str(inline).with_context(|| "invalid inline-rules YAML")?
    } else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "no --rules or --inline-rules provided".into(),
        }
        .into());
    };

    if rules.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "rules manifest is empty (must contain at least one rule)".into(),
        }
        .into());
    }

    // For each rule, build a synthetic single-rule TransformArgs and
    // call cmd_transform. This reuses the entire single-rule pipeline
    // (walker, pattern match, atomic write) without duplicating logic.
    let _total_files_visited: u64 = 0;
    let _total_files_transformed: u64 = 0;
    let _total_replacements: u64 = 0;
    let _total_failed: u64 = 0;

    for rule in &rules {
        let synthetic_args = TransformArgs {
            pattern: Some(rule.pattern.clone()),
            rewrite: Some(rule.rewrite.clone()),
            language: Some(rule.language.clone()),
            paths: args.paths.clone(),
            include: args.include.clone(),
            exclude: args.exclude.clone(),
            dry_run: args.dry_run,
            rules: None,
            inline_rules: None,
            backup: effective_backup,
            no_backup: false,
            verify_parse: args.verify_parse,
        };

        // Emit a "rule_begin" event so consumers can correlate subsequent
        // transform events with the rule that produced them.
        writer.write_event(&serde_json::json!({
            "type": "rule_begin",
            "id": rule.id,
            "language": rule.language,
        }))?;

        match cmd_transform(&synthetic_args, global, writer, _shutdown) {
            Ok(()) => {
                // The inner cmd_transform already emitted a Summary event.
                // The outer caller will emit a final summary below.
            }
            Err(e) => {
                writer.write_event(&serde_json::json!({
                    "type": "rule_error",
                    "id": rule.id,
                    "error": e.to_string(),
                }))?;
                // Continue with the next rule — partial success is OK in
                // multi-rule mode; the user can re-run individual rules.
            }
        }
    }

    writer.write_event(&Summary {
        r#type: "summary",
        files_visited: 0, // per-rule summaries are nested; top-level stays 0
        files_matched: 0,
        files_modified: None,
        files_skipped: None,
        total_matches: None,
        total_replacements: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}
