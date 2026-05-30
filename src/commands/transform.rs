// SPDX-License-Identifier: MIT OR Apache-2.0

//! Structural AST search and rewrite via ast-grep.

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
use crate::ndjson_types::{Summary, TransformResult};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Perform structural code search and rewrite via AST patterns.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::InvalidInput` if the AST pattern or language is invalid.
/// Returns `AtomwriteError::Io` if reading or writing files fails.
pub fn cmd_transform(
    args: &TransformArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let lang: SupportLang =
        args.lang
            .parse()
            .map_err(|_| crate::error::AtomwriteError::InvalidInput {
                reason: format!("unsupported language: {}", args.lang),
            })?;

    let pattern = Pattern::try_new(&args.pattern, lang).map_err(|e| {
        crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid pattern: {e}"),
        }
    })?;

    let extensions = lang_extensions(&args.lang);

    let mut walker = ignore::WalkBuilder::new(&args.paths[0]);
    for p in args.paths.iter().skip(1) {
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
        let mut overrides_builder = ignore::overrides::OverrideBuilder::new(&args.paths[0]);
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

    let (tx, rx) = crossbeam_channel::unbounded::<TransformEvent>();

    let files_visited = Arc::new(AtomicU64::new(0));
    let files_transformed = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));
    let total_replacements = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let ft = Arc::clone(&files_transformed);
    let fs = Arc::clone(&files_skipped);
    let tr = Arc::clone(&total_replacements);
    let rewrite = args.rewrite.clone();
    let language = args.lang.clone();
    let dry_run = args.dry_run;
    let backup = args.backup;
    let ws = workspace.clone();

    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let pattern = pattern.clone();
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let ft = Arc::clone(&ft);
            let fs = Arc::clone(&fs);
            let tr = Arc::clone(&tr);
            let rewrite = rewrite.clone();
            let language = language.clone();
            let ws = ws.clone();

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
                let file_start = Instant::now();

                let content = match crate::file_io::read_file_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        fs.fetch_add(1, Ordering::Relaxed);
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

                let mut new_content = content.clone();
                let mut sorted_edits = edits;
                sorted_edits.sort_by_key(|e| std::cmp::Reverse(e.position));
                for edit in &sorted_edits {
                    let start = edit.position;
                    let end = start + edit.deleted_length;
                    let replacement = String::from_utf8_lossy(&edit.inserted_text);
                    new_content.replace_range(start..end, &replacement);
                }

                let checksum_after = checksum::hash_bytes(new_content.as_bytes());

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
                        backup,
                        ..Default::default()
                    };
                    if let Err(e) = atomic_write(&validated, new_content.as_bytes(), &opts, &ws) {
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
                    language: language.clone(),
                    matches: match_count,
                    bytes_before: content.len() as u64,
                    bytes_after: new_content.len() as u64,
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

    let _ = walker_thread.join();

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

fn lang_extensions(lang: &str) -> Vec<&'static str> {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => vec!["rs"],
        "javascript" | "js" => vec!["js", "jsx", "mjs"],
        "typescript" | "ts" => vec!["ts", "tsx", "mts"],
        "python" | "py" => vec!["py", "pyi"],
        "go" => vec!["go"],
        "java" => vec!["java"],
        "c" => vec!["c", "h"],
        "cpp" | "c++" => vec!["cpp", "hpp", "cc", "hh", "cxx"],
        "csharp" | "c#" | "cs" => vec!["cs"],
        "ruby" | "rb" => vec!["rb"],
        "swift" => vec!["swift"],
        "kotlin" | "kt" => vec!["kt", "kts"],
        "lua" => vec!["lua"],
        "html" => vec!["html", "htm"],
        "css" => vec!["css"],
        "json" => vec!["json"],
        "yaml" | "yml" => vec!["yaml", "yml"],
        "toml" => vec!["toml"],
        _ => vec![],
    }
}
