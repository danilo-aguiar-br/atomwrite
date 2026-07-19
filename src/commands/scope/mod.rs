// SPDX-License-Identifier: MIT OR Apache-2.0

//! Grammatical scoping: AST-based code selection and transformation.
//! Workload: mixed I/O-bound + CPU-bound (file reading + AST traversal via ast-grep + atomic write).
//! Parallelism: `ignore::WalkParallel` + bounded channel; bound via
//! `concurrency::apply_walk_threads` (`--threads` / `--max-concurrency`).

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use ast_grep_core::AstGrep;
use ast_grep_core::matcher::Pattern;
use ast_grep_language::SupportLang;
use unicode_normalization::UnicodeNormalization;

use clap::ValueHint;

use crate::atomic::{AtomicWriteOptions, atomic_write};
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
    /// Paths to search within.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub paths: Vec<std::path::PathBuf>,

    /// Source language for AST parsing.
    /// GAP-2026-003 — fixed in v0.1.20 via ADR-0037: the global locale
    /// flag was renamed from `--lang` to `--locale`, freeing the
    /// `--lang` namespace. `--lang` is now a working alias for
    /// `--language`. Both `--lang rust` and `--language rust` are
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
) -> Result<()> {
    let start = Instant::now();
    let resolved = resolve_backup(&args.backup_opts, defaults);

    let lang = parse_language(&args.language)?;
    let pattern_strs = resolve_patterns(&args.query, &args.pattern, &args.language)?;
    let patterns: Vec<Pattern> = pattern_strs
        .iter()
        .map(|ps| {
            Pattern::try_new(ps, lang).map_err(|e| {
                anyhow::anyhow!(AtomwriteError::InvalidInput {
                    reason: format!("invalid scope pattern: {e}"),
                })
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let query_name = args.query.clone().unwrap_or_else(|| "custom".to_owned());

    let workspace = global.resolve_workspace()?;

    let canonical_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;
    let mut walker = ignore::WalkBuilder::new(&canonical_paths[0]);
    for p in canonical_paths.iter().skip(1) {
        walker.add(p);
    }
    walker
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore)
        .follow_links(global.follow_symlinks);
    crate::concurrency::apply_walk_threads(&mut walker, global.threads);

    let extensions = crate::lang_utils::lang_extensions(&args.language);
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
        let mut overrides = ignore::overrides::OverrideBuilder::new(&canonical_paths[0]);
        for pat in &args.include {
            overrides.add(pat)?;
        }
        for pat in &args.exclude {
            overrides.add(&format!("!{pat}"))?;
        }
        walker.overrides(overrides.build()?);
    }

    let (tx, rx) =
        crossbeam_channel::bounded::<ScopeEvent>(crate::concurrency::event_channel_cap());

    // Per-run counters. Ordering::Relaxed: independent tallies; final loads
    // run after the parallel join (no data-publication barrier needed).
    let files_visited = Arc::new(AtomicU64::new(0));
    let files_modified = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let fm = Arc::clone(&files_modified);
    let fs = Arc::clone(&files_skipped);
    let delete = args.delete;
    let action = args.action;
    let replace_with: Option<Arc<str>> = args.replace_with.clone().map(Into::into);
    let dry_run = args.dry_run;
    let backup = resolved.backup;
    let ws: Arc<std::path::Path> = Arc::from(workspace.as_path());
    let qn: Arc<str> = query_name.into();
    let lang_name: Arc<str> = args.language.clone().into();

    let max_size = global.effective_max_filesize();
    let shutdown_flag = shutdown.flag();
    let patterns = Arc::new(patterns);
    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let patterns = Arc::clone(&patterns);
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let fm = Arc::clone(&fm);
            let fs = Arc::clone(&fs);
            let replace_with = replace_with.clone();
            let ws = Arc::clone(&ws);
            let qn = Arc::clone(&qn);
            let lang_name = Arc::clone(&lang_name);
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
                    Err(_) => {
                        fs.fetch_add(1, Ordering::Relaxed);
                        return ignore::WalkState::Continue;
                    }
                };

                let grep = AstGrep::new(&content, lang);
                let root = grep.root();
                let matches: Vec<_> = patterns.iter().flat_map(|p| root.find_all(p)).collect();

                if matches.is_empty() {
                    fs.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                let scopes_matched = matches.len() as u64;

                let is_read_only = !delete && action.is_none() && replace_with.is_none();
                if is_read_only {
                    fm.fetch_add(1, Ordering::Relaxed);
                    let checksum = checksum::hash_bytes(content.as_bytes());
                    let _ = tx.send(ScopeEvent::Result(ScopeResult {
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
                    }));
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

                if checksum_before == checksum_after {
                    // O(1) hash comparison instead of O(n) string comparison
                    fs.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                fm.fetch_add(1, Ordering::Relaxed);

                if !dry_run {
                    let opts = AtomicWriteOptions {
                        backup,
                        ..Default::default()
                    };
                    if let Err(e) = atomic_write(&path, content.as_bytes(), &opts, &ws) {
                        tracing::warn!(path = %path.display(), error = %e, "scope write failed");
                        return ignore::WalkState::Continue;
                    }
                }

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
                let _ = tx.send(ScopeEvent::Result(ScopeResult {
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
                }));

                ignore::WalkState::Continue
            })
        });
    });

    for event in rx {
        if shutdown.is_shutdown() {
            break;
        }
        match event {
            ScopeEvent::Result(r) => writer.write_event(&r)?,
        }
    }

    if let Err(panic_payload) = walker_thread.join() {
        std::panic::resume_unwind(panic_payload);
    }

    let summary = Summary {
        r#type: "summary",
        files_visited: files_visited.load(Ordering::Relaxed),
        files_matched: files_modified.load(Ordering::Relaxed),
        files_modified: {
            let is_read_only = !args.delete && args.action.is_none() && args.replace_with.is_none();
            if !args.dry_run && !is_read_only {
                Some(files_modified.load(Ordering::Relaxed))
            } else {
                None
            }
        },
        files_skipped: Some(files_skipped.load(Ordering::Relaxed)),
        total_matches: None,
        total_replacements: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };

    writer.write_event(&summary)?;

    if files_modified.load(Ordering::Relaxed) == 0
        && files_skipped.load(Ordering::Relaxed) == files_visited.load(Ordering::Relaxed)
    {
        return Err(crate::error::AtomwriteError::NoMatches.into());
    }

    Ok(())
}

fn expand_to_full_line(content: &str, start: usize, end: usize) -> (usize, usize) {
    let bytes = content.as_bytes();
    let line_start = bytes[..start]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |pos| pos + 1);
    let line_end = bytes[end..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(content.len(), |pos| end + pos + 1);

    let before_match = &content[line_start..start];

    if before_match.trim().is_empty() {
        (line_start, line_end)
    } else {
        let content_end = if line_end > 0 && bytes[line_end - 1] == b'\n' {
            line_end - 1
        } else {
            line_end
        };
        let trim_start = content[line_start..start]
            .rfind(|c: char| !c.is_whitespace())
            .map_or(start, |pos| line_start + pos + 1);
        (trim_start, content_end)
    }
}

fn apply_scope_action<'a>(
    text: &'a str,
    delete: bool,
    action: Option<ScopeAction>,
    replace_with: Option<&str>,
) -> std::borrow::Cow<'a, str> {
    if delete {
        return std::borrow::Cow::Owned(String::new());
    }
    if let Some(replacement) = replace_with {
        return std::borrow::Cow::Owned(replacement.to_owned());
    }
    match action {
        Some(ScopeAction::Upper) => std::borrow::Cow::Owned(text.to_uppercase()),
        Some(ScopeAction::Lower) => std::borrow::Cow::Owned(text.to_lowercase()),
        Some(ScopeAction::Titlecase) => std::borrow::Cow::Owned(titlecase(text)),
        Some(ScopeAction::Squeeze) => std::borrow::Cow::Owned(squeeze(text)),
        Some(ScopeAction::Symbols) => std::borrow::Cow::Owned(symbolize(text)),
        Some(ScopeAction::Normalize) => std::borrow::Cow::Owned(text.nfc().collect::<String>()),
        None => std::borrow::Cow::Borrowed(text),
    }
}

fn titlecase(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for c in s.chars() {
        if capitalize_next && c.is_alphabetic() {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
            if c.is_whitespace() || c == '_' || c == '-' {
                capitalize_next = true;
            }
        }
    }
    result
}

fn squeeze(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if Some(c) != prev || !c.is_whitespace() {
            result.push(c);
        }
        prev = Some(c);
    }
    result
}

fn symbolize(s: &str) -> String {
    s.replace("=>", "⇒")
        .replace("->", "→")
        .replace("<-", "←")
        .replace("!=", "≠")
        .replace(">=", "≥")
        .replace("<=", "≤")
        .replace("...", "…")
        .replace("--", "—")
}

fn resolve_patterns(
    query_name: &Option<String>,
    custom_pattern: &Option<String>,
    lang_str: &str,
) -> Result<Vec<String>> {
    if let Some(p) = custom_pattern {
        return Ok(vec![p.clone()]);
    }

    let name = query_name
        .as_deref()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "either --query or --pattern is required".into(),
        })?;

    lookup_prepared_queries(name, lang_str)
}

fn parse_language(lang_str: &str) -> Result<SupportLang> {
    lang_str.parse().map_err(|_| {
        AtomwriteError::InvalidInput {
            reason: format!("unsupported language: {lang_str}"),
        }
        .into()
    })
}

enum ScopeEvent {
    Result(ScopeResult),
}

include!("queries.inc.rs");

include!("tests.inc.rs");
