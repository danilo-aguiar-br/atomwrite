// SPDX-License-Identifier: MIT OR Apache-2.0

//! Local semantic-ish search via token Jaccard similarity (v0.1.29 P3-2).
//! No network, no embeddings API, no telemetry — pure offline ranking.
//!
//! Workload: mixed I/O-bound + CPU-bound (walk + tokenize + Jaccard rank).
//! Parallelism: collect file paths (walk), then `rayon::par_iter` over files
//! for tokenize/score. Ranking is a final sort (stable contract). Bound:
//! process-wide rayon pool (`--threads` / `--max-concurrency`).

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use clap::{Args, ValueHint};
use rayon::prelude::*;

use crate::cli::GlobalArgs;
use crate::concurrency::should_parallelize;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;

/// Per-file semantic score bag (hits + optional index tokens + lines kept
/// out by `--min-score`).
type ScoredFile = (
    Vec<(f64, String, u64, String)>,
    Vec<crate::ndjson_types::SemanticIndexToken>,
    u64,
);
use crate::path_safety::validate_path;
use crate::signal::ShutdownSignal;

/// Arguments for `semantic-search`.
#[derive(Args, Debug)]
pub struct SemanticSearchArgs {
    /// Free-text query.
    pub query: String,
    /// Roots to scan.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,
    /// Maximum results.
    #[arg(long, default_value_t = crate::constants::DEFAULT_SEMANTIC_SEARCH_TOP as u64)]
    pub k: u64,
    /// Minimum Jaccard score [0.0, 1.0].
    #[arg(long, default_value_t = crate::constants::DEFAULT_SEMANTIC_SEARCH_MIN_SCORE)]
    pub min_score: f64,
    /// Optional local inverted-index directory (offline, no network).
    ///
    /// When set, builds or loads a token→paths index under this directory
    /// (`.atomwrite/semantic-index` is the recommended path). Backend becomes
    /// `inverted-index` instead of pure line Jaccard.
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub index_dir: Option<PathBuf>,

    /// Include atomwrite backup paths (`*.bak.*`) in ranking.
    /// Default: exclude backups so agents do not edit sidecars.
    #[arg(
        long,
        help = "Include *.bak.* backup files (default: exclude atomwrite backups)",
        action = clap::ArgAction::SetTrue
    )]
    pub include_backups: bool,
}

/// Rank lines by token overlap with the query (offline, no embeddings).
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when the query is empty or produces no tokens.
/// Returns `AtomwriteError::WorkspaceJail` when a root or `--index-dir` escapes the workspace.
/// Returns `AtomwriteError::Io` when the index directory cannot be created, the index file cannot
/// be written, or a root cannot be resolved.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
/// Returns the first walk failure, published as an `ok: false` summary, when a file under a root
/// could not be read for a reason other than a content rejection.
#[tracing::instrument(skip_all, fields(command = "semantic-search"))]
pub fn cmd_semantic_search(
    args: &SemanticSearchArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    _defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    if args.query.trim().is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: "query must not be empty".into(),
        }
        .into());
    }
    let workspace = global.resolve_workspace()?;
    let q_tokens = tokenize(&args.query);
    if q_tokens.is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: "query produced no tokens".into(),
        }
        .into());
    }
    let max_size = global.effective_max_filesize();
    let backend = if args.index_dir.is_some() {
        "inverted-index"
    } else {
        "jaccard"
    };

    // Optional offline inverted index: token -> list of path\tline\tsnippet
    let mut index: HashMap<String, Vec<(String, u64, String)>> = HashMap::new();
    if let Some(ref idir) = args.index_dir {
        let idir = prepare_index_dir(idir, &workspace)?;
        let idx_file = idir.join(crate::constants::SEMANTIC_INDEX_FILE);
        if idx_file.is_file() {
            if let Ok(text) = crate::file_io::read_file_string(&idx_file, max_size) {
                for line in text.lines() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
                        && let (Some(tok), Some(path), Some(ln), Some(snip)) = (
                            v.get("t").and_then(|x| x.as_str()),
                            v.get("p").and_then(|x| x.as_str()),
                            v.get("l").and_then(|x| x.as_u64()),
                            v.get("s").and_then(|x| x.as_str()),
                        )
                    {
                        index.entry(tok.to_string()).or_default().push((
                            path.to_string(),
                            ln,
                            snip.to_string(),
                        ));
                    }
                }
            }
        }
    }

    let mut hits: Vec<(f64, String, u64, String)> = Vec::new();
    // Scored lines the `--min-score` floor kept out of the ranking, so a run
    // that returns nothing says whether the corpus was empty or the floor.
    let mut below_min_score: u64 = 0;
    // Files the ranking scored. The inverted-index path reads no file, so it
    // stays `None` there instead of claiming a scan it never did.
    let mut files_scanned: Option<u64> = None;
    // Declared outside both branches: the summary reports it either way, and
    // the inverted-index path reads no file, so it stays empty there.
    let failures: std::sync::Arc<std::sync::Mutex<crate::ndjson_types::FailureTally>> =
        std::sync::Arc::new(std::sync::Mutex::new(
            crate::ndjson_types::FailureTally::default(),
        ));
    if backend == "inverted-index" && !index.is_empty() {
        let mut cand: HashMap<(String, u64), (f64, String)> = HashMap::new();
        for tok in &q_tokens {
            if let Some(entries) = index.get(tok) {
                for (path, line, snip) in entries {
                    let key = (path.clone(), *line);
                    let e = cand.entry(key).or_insert((0.0, snip.clone()));
                    e.0 += 1.0;
                }
            }
        }
        let qn = q_tokens.len() as f64;
        for ((path, line), (overlap, snip)) in cand {
            let score = if qn == 0.0 { 0.0 } else { overlap / qn };
            if score >= args.min_score {
                hits.push((score, path, line, snip));
            } else {
                below_min_score += 1;
            }
        }
    } else {
        // Stage 1 — multi-root collect (one WalkBuilder + `.add`; --threads honored).
        let mut roots = Vec::with_capacity(args.paths.len());
        for root in &args.paths {
            if shutdown.is_shutdown() {
                break;
            }
            roots.push(validate_path(root, &workspace)?);
        }
        let mut files = if roots.is_empty() {
            Vec::new()
        } else {
            // Dotfiles are searched only under `--hidden`, like every other
            // tree walk, so the shared walker honors the global flags.
            // An ignore file the walk half-reads is a hole too, and `ignore`
            // never reports it by itself.
            crate::concurrency::record_ignore_file_failures(&roots, Some(&failures));
            let wb = crate::commands::build_scoped_walker_flavored(
                &roots,
                global,
                crate::commands::WalkerFlavor::RespectGlobalFlags,
                &[],
                &[],
            )?;
            crate::concurrency::collect_files_parallel_tallied(
                &wb,
                Some(std::sync::Arc::clone(&failures)),
            )
        };
        crate::concurrency::sort_paths_parallel(&mut files);

        // A-008b: exclude atomwrite backups by default (DRY: is_backup_path).
        if !args.include_backups {
            files.retain(|p| !crate::commands::is_backup_path(p));
        }
        // `--limit` bounds the work: scoring stops at the budget instead of
        // reading the whole tree to print `k` results from the top.
        crate::output::cap_collected(&mut files);

        // Stage 2 — CPU: tokenize + Jaccard per file in parallel.
        let q_tokens = Arc::new(q_tokens);
        let min_score = args.min_score;
        let build_index = args.index_dir.is_some();
        // A file the ranking could not read is a hole in the ranking, and the
        // agent has to know that the answer is partial.
        let scored: Vec<ScoredFile> = if should_parallelize(files.len()) {
            files
                .par_iter()
                .map(|path| {
                    score_file(
                        path,
                        &q_tokens,
                        min_score,
                        max_size,
                        build_index,
                        shutdown,
                        &failures,
                    )
                })
                .collect()
        } else {
            files
                .iter()
                .map(|path| {
                    score_file(
                        path,
                        &q_tokens,
                        min_score,
                        max_size,
                        build_index,
                        shutdown,
                        &failures,
                    )
                })
                .collect()
        };

        // Captured before `scored` is consumed: it is the reach of the scan.
        files_scanned = Some(scored.len() as u64);
        let mut built: Vec<crate::ndjson_types::SemanticIndexToken> = Vec::new();
        for (file_hits, file_built, file_below) in scored {
            hits.extend(file_hits);
            built.extend(file_built);
            below_min_score += file_below;
        }

        if let Some(ref idir) = args.index_dir {
            let idir = prepare_index_dir(idir, &workspace)?;
            write_index(&idir.join(crate::constants::SEMANTIC_INDEX_FILE), &built)?;
        }
    }
    let (failed_count, failed_items, first_failure) = match std::sync::Arc::try_unwrap(failures) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default().into_parts(),
        Err(arc) => {
            std::mem::take(&mut *arc.lock().unwrap_or_else(|e| e.into_inner())).into_parts()
        }
    };
    hits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    // The ranking size before the `k` cut, so `results < hits_before_cut`
    // proves truncation and the equality proves nothing was cut.
    let hits_before_cut = hits.len() as u64;
    hits.truncate(args.k as usize);
    let total = hits.len();
    for (rank, (score, path, line, snippet)) in hits.into_iter().enumerate() {
        writer.write_event(&crate::ndjson_types::SemanticMatchEvent {
            r#type: "semantic_match",
            rank: rank + 1,
            score,
            path,
            line,
            snippet,
            backend,
        })?;
    }
    let files_failed = (failed_count > 0).then_some(failed_count);
    let summary = crate::ndjson_types::SemanticSummaryEvent {
        r#type: "semantic_summary",
        query: args.query.clone(),
        k: args.k,
        results: total,
        backend,
        files_failed,
        partial: crate::ndjson_types::partial_of(files_failed),
        errors: (!failed_items.is_empty()).then_some(failed_items),
        files_scanned,
        hits_before_cut: Some(hits_before_cut),
        below_min_score: Some(below_min_score).filter(|n| *n > 0),
        files_budget_dropped: Some(crate::output::budget_dropped()).filter(|n| *n > 0),
        // The walk collects the whole tree and only then slices the ranking,
        // so it never cuts its own walk. Reporting the writer's budget here
        // said "stopped early" on a walk that read every file; the truncation
        // is visible in `results` against `k`.
        walker_stopped_early: None,
    };
    // A run that answered publishes the answer: the hits are usable, and the
    // hole is what `partial` and `errors` are for. Failing the run here threw
    // away every hit the moment one subtree could not be read, which is the
    // rule `search` already keeps.
    if let Some(err) = first_failure
        && total == 0
    {
        return Err(writer.publish_failure(&summary, err));
    }
    writer.write_terminal(&summary)?;
    Ok(())
}

/// Validate `--index-dir` against the workspace jail and create it.
///
/// A validation failure (jail violation, symlink escape) is returned as is:
/// it must never be turned into permission to use the directory.
fn prepare_index_dir(idir: &Path, workspace: &Path) -> Result<std::path::PathBuf> {
    let idir = validate_path(idir, workspace)?;
    std::fs::create_dir_all(&idir).map_err(|source| AtomwriteError::Io { source })?;
    Ok(idir)
}

/// Rewrite the inverted index file with every token record built this run.
fn write_index(idx_file: &Path, built: &[crate::ndjson_types::SemanticIndexToken]) -> Result<()> {
    let mut f = std::fs::File::create(idx_file).map_err(|source| AtomwriteError::Io { source })?;
    for rec in built {
        let line = serde_json::to_string(rec)?;
        writeln!(f, "{line}").map_err(|source| AtomwriteError::Io { source })?;
    }
    Ok(())
}

fn score_file(
    path: &Path,
    q_tokens: &HashSet<String>,
    min_score: f64,
    max_size: u64,
    build_index: bool,
    shutdown: &ShutdownSignal,
    failures: &std::sync::Mutex<crate::ndjson_types::FailureTally>,
) -> ScoredFile {
    let mut hits = Vec::new();
    let mut built = Vec::new();
    let mut below = 0u64;
    if shutdown.is_shutdown() {
        return (hits, built, below);
    }
    let content = match crate::file_io::read_file_string(path, max_size) {
        Ok(c) => c,
        Err(e) => {
            // Too large, binary or not UTF-8 is a file the caller told the
            // ranking to leave alone; anything else is a hole.
            if !crate::output::is_content_rejection(&e)
                && let Ok(mut tally) = failures.lock()
            {
                tally.record(path, e);
            }
            return (hits, built, below);
        }
    };
    let path_str = path.display().to_string();
    for (i, line) in content.lines().enumerate() {
        let tokens = tokenize(line);
        if tokens.is_empty() {
            continue;
        }
        if build_index {
            let snip: String = line
                .chars()
                .take(crate::constants::SEMANTIC_SNIPPET_CHARS)
                .collect();
            for t in &tokens {
                built.push(crate::ndjson_types::SemanticIndexToken {
                    t: t.clone(),
                    p: path_str.clone(),
                    l: (i as u64) + 1,
                    s: snip.clone(),
                });
            }
        }
        let score = jaccard(q_tokens, &tokens);
        if score >= min_score {
            hits.push((
                score,
                path_str.clone(),
                (i as u64) + 1,
                line.chars()
                    .take(crate::constants::SEMANTIC_SNIPPET_CHARS)
                    .collect(),
            ));
        } else {
            below += 1;
        }
    }
    (hits, built, below)
}

fn tokenize(s: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    // Split on non-alnum except underscore; then split snake and camel.
    for raw in s.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if raw.is_empty() {
            continue;
        }
        let lower = raw.to_ascii_lowercase();
        if lower.len() >= 2 {
            out.insert(lower.clone());
        }
        // snake_case subtokens
        for part in raw.split('_') {
            let p = part.to_ascii_lowercase();
            if p.len() >= 2 {
                out.insert(p);
            }
        }
        // camelCase / PascalCase subtokens
        let mut cur = String::new();
        for ch in raw.chars() {
            if ch.is_uppercase() && !cur.is_empty() {
                let p = cur.to_ascii_lowercase();
                if p.len() >= 2 {
                    out.insert(p);
                }
                cur.clear();
            }
            cur.push(ch);
        }
        if !cur.is_empty() {
            let p = cur.to_ascii_lowercase();
            if p.len() >= 2 {
                out.insert(p);
            }
        }
    }
    out
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f64;
    let uni = a.union(b).count() as f64;
    if uni == 0.0 { 0.0 } else { inter / uni }
}
