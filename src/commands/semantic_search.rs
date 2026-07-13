// SPDX-License-Identifier: MIT OR Apache-2.0

//! Local semantic-ish search via token Jaccard similarity (v0.1.29 P3-2).
//! No network, no embeddings API, no telemetry — pure offline ranking.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalArgs;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use crate::path_safety::validate_path;
use crate::signal::ShutdownSignal;

/// Arguments for `semantic-search`.
#[derive(Args, Debug)]
pub struct SemanticSearchArgs {
    /// Free-text query.
    pub query: String,
    /// Roots to scan.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,
    /// Maximum results.
    #[arg(long, default_value_t = 20)]
    pub k: u64,
    /// Minimum Jaccard score [0.0, 1.0].
    #[arg(long, default_value_t = 0.05)]
    pub min_score: f64,
    /// Optional local inverted-index directory (offline, no network).
    ///
    /// When set, builds or loads a token→paths index under this directory
    /// (`.atomwrite/semantic-index` is the recommended path). Backend becomes
    /// `inverted-index` instead of pure line Jaccard.
    #[arg(long)]
    pub index_dir: Option<PathBuf>,
}

/// Rank lines by token overlap with the query (offline, no embeddings).
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
        let idir = validate_path(idir, &workspace).unwrap_or_else(|_| workspace.join(idir));
        let _ = std::fs::create_dir_all(&idir);
        let idx_file = idir.join("tokens.ndjson");
        if idx_file.is_file() {
            if let Ok(text) = std::fs::read_to_string(&idx_file) {
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
            }
        }
    } else {
        let mut built: Vec<serde_json::Value> = Vec::new();
        for root in &args.paths {
            if shutdown.is_shutdown() {
                break;
            }
            let root = validate_path(root, &workspace)?;
            let mut wb = ignore::WalkBuilder::new(&root);
            wb.git_ignore(true);
            for entry in wb.build() {
                if shutdown.is_shutdown() {
                    break;
                }
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                if !entry.file_type().is_some_and(|t| t.is_file()) {
                    continue;
                }
                let path = entry.path();
                let content = match crate::file_io::read_file_string(path, max_size) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                for (i, line) in content.lines().enumerate() {
                    let tokens = tokenize(line);
                    if tokens.is_empty() {
                        continue;
                    }
                    if args.index_dir.is_some() {
                        let snip: String = line.chars().take(200).collect();
                        for t in &tokens {
                            built.push(serde_json::json!({
                                "t": t,
                                "p": path.display().to_string(),
                                "l": (i as u64) + 1,
                                "s": snip,
                            }));
                        }
                    }
                    let score = jaccard(&q_tokens, &tokens);
                    if score >= args.min_score {
                        hits.push((
                            score,
                            path.display().to_string(),
                            (i as u64) + 1,
                            line.chars().take(200).collect(),
                        ));
                    }
                }
            }
        }
        if let Some(ref idir) = args.index_dir {
            let idir = validate_path(idir, &workspace).unwrap_or_else(|_| workspace.join(idir));
            let _ = std::fs::create_dir_all(&idir);
            let idx_file = idir.join("tokens.ndjson");
            if let Ok(mut f) = std::fs::File::create(&idx_file) {
                for v in built {
                    let _ = writeln!(f, "{v}");
                }
            }
        }
    }
    hits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(args.k as usize);
    let total = hits.len();
    for (rank, (score, path, line, snippet)) in hits.into_iter().enumerate() {
        writer.write_event(&serde_json::json!({
            "type": "semantic_match",
            "rank": rank + 1,
            "score": score,
            "path": path,
            "line": line,
            "snippet": snippet,
            "backend": backend,
        }))?;
    }
    writer.write_event(&serde_json::json!({
        "type": "semantic_summary",
        "query": args.query,
        "k": args.k,
        "results": total,
        "backend": backend,
    }))?;
    Ok(())
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
