// SPDX-License-Identifier: MIT OR Apache-2.0

//! Apply N pairs of `old`/`new` substitutions read from NDJSON on stdin.
//! Workload: I/O-bound (read file + iterate pairs + atomic write).
//!
//! ADR-0039 — `edit-loop` exists because chaining many `edit --old --new`
//! invocations from an LLM agent is wasteful (per-call checksum verify
//! and atomic write). The loop variant reads the full pair list in one
//! shot, applies them sequentially in memory, then performs a single
//! atomic write at the end.

use std::io::{Read, Write};
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::cli::{EditLoopArgs, GlobalArgs};
use crate::commands::resolve_backup;
use crate::ndjson_types::{EditLoopPairResult, EditLoopSummary};
use crate::output::NdjsonWriter;
use crate::path_safety::validate_path;

/// One NDJSON line from stdin describing a single `old`/`new` pair.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EditPair {
    /// Exact text to find in the file.
    old: String,
    /// Replacement text.
    new: String,
}

/// Apply N substitution pairs from NDJSON stdin to a single file.
///
/// Reads pairs as `{"old":"...","new":"..."}` per line, applies them
/// sequentially with `str::replacen(..., 1)` (each pair only replaces
/// the FIRST occurrence, mirroring the behaviour of single `edit
/// --old/--new`). Writes the result atomically with the same options
/// as `edit` (backup, retention, syntax check, line endings).
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the target does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns a parse error if the stdin NDJSON is malformed.
#[tracing::instrument(skip_all, fields(command = "edit-loop"))]
pub fn cmd_edit_loop(
    args: &EditLoopArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let target = validate_path(&args.path, &workspace)?;
    let effective_backup = resolve_backup(args.backup, args.no_backup);

    if !target.exists() {
        return Err(crate::error::AtomwriteError::NotFound {
            path: target.clone(),
        }
        .into());
    }

    let max_size = global.effective_max_filesize();
    let mut content = crate::file_io::read_file_string(&target, max_size)?;

    // Parse pairs from stdin. Accepts both JSON array and NDJSON (one object
    // per line). Detection: if the first non-whitespace byte is `[`, parse
    // as a JSON array; otherwise parse as NDJSON lines.
    let mut buf = String::new();
    stdin.take(max_size).read_to_string(&mut buf)?;
    let trimmed = buf.trim_start();
    let pairs: Vec<EditPair> = if trimmed.starts_with('[') {
        serde_json::from_str::<Vec<EditPair>>(trimmed)
            .with_context(|| "failed to parse JSON array of edit pairs")?
    } else {
        buf.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                serde_json::from_str::<EditPair>(l)
                    .with_context(|| format!("failed to parse NDJSON pair: {l}"))
            })
            .collect::<Result<Vec<_>>>()?
    };

    // Apply each pair in order. `replacen(.., 1)` matches the single-pair
    // `edit` semantics: only the first occurrence is touched.
    let mut pair_results: Vec<EditLoopPairResult> = Vec::with_capacity(pairs.len());
    let mut applied = 0usize;
    let mut unmatched = 0usize;
    for (i, pair) in pairs.iter().enumerate() {
        if content.contains(&pair.old) {
            content = content.replacen(&pair.old, &pair.new, 1);
            applied += 1;
            pair_results.push(EditLoopPairResult {
                index: i + 1,
                matched: true,
            });
        } else {
            unmatched += 1;
            pair_results.push(EditLoopPairResult {
                index: i + 1,
                matched: false,
            });
        }
    }

    // Normalize line endings before atomic write, mirroring `edit`.
    {
        use crate::line_endings::{self, LineEnding};
        let target_le = match args.line_ending {
            LineEnding::Auto => line_endings::detect(content.as_bytes()),
            other => other,
        };
        content = line_endings::normalize(&content, target_le);
    }

    let opts = AtomicWriteOptions {
        backup: effective_backup,
        syntax_check: args.syntax_check.is_some(),
        retention: args.retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup: args.keep_backup,
    };

    atomic_write(&target, content.as_bytes(), &opts, &workspace)?;

    writer.write_event(&EditLoopSummary {
        r#type: "result",
        action: "edit_loop".to_string(),
        path: target.display().to_string(),
        pairs_total: pairs.len(),
        pairs_applied: applied,
        pairs_unmatched: unmatched,
        elapsed_ms: start.elapsed().as_millis() as u64,
        pair_results,
    })?;

    Ok(())
}
