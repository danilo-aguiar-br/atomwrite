// SPDX-License-Identifier: MIT OR Apache-2.0

//! Apply N pairs of `old`/`new` substitutions read from NDJSON on stdin.
//! Workload: I/O-bound (read file + iterate pairs + atomic write).
//! Parallelism: none — single target file; pair order is compositional.
//!
//! ADR-0039 — `edit-loop` exists because chaining many `edit --old --new`
//! invocations from an LLM agent is wasteful (per-call checksum verify
//! and atomic write). The loop variant reads the full pair list in one
//! shot, applies them sequentially in memory, then performs a single
//! atomic write at the end.

use std::io::{BufReader, Read, Write};
use std::time::Instant;

use anyhow::Result;
use serde::Deserialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::cli::FuzzyMode;
use crate::cli::{EditLoopArgs, GlobalArgs};
use crate::commands::resolve_backup;
use crate::fuzzy;
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
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let target = validate_path(&args.path, &workspace)?;
    let resolved_backup = resolve_backup(&args.backup_opts, defaults);
    let (fuzzy_mode, fuzzy_threshold) =
        crate::config::resolve_fuzzy(FuzzyMode::Auto, None, fuzzy_cfg)?;

    if !target.exists() {
        return Err(crate::error::AtomwriteError::NotFound { path: target }.into());
    }

    let max_size = global.effective_max_filesize();
    let mut content = crate::file_io::read_file_string(&target, max_size)?;

    // Parse pairs from stdin. Accepts both JSON array and NDJSON (one object
    // per line). Detection: peek first non-whitespace byte — if `[`, parse
    // as a single JSON array (size-capped by max_filesize); otherwise stream
    // NDJSON with per-line size limits (rules: limite por linha + BOM).
    let mut reader = BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
    let pairs: Vec<EditPair> = {
        use std::io::BufRead;
        // Peek past leading BOM / whitespace to choose dialect.
        let prefix = {
            let filled = reader.fill_buf()?;
            let mut i = 0usize;
            // Skip UTF-8 BOM if present at buffer start.
            if filled.starts_with(&[0xEF, 0xBB, 0xBF]) {
                i = 3;
            }
            while i < filled.len() && (filled[i] == b' ' || filled[i] == b'\t' || filled[i] == b'\n' || filled[i] == b'\r') {
                i += 1;
            }
            filled.get(i).copied()
        };

        if prefix == Some(b'[') {
            let mut buf = String::new();
            reader
                .take(max_size)
                .read_to_string(&mut buf)
                .map_err(|e| crate::error::AtomwriteError::Io { source: e })?;
            let trimmed = crate::output::strip_utf8_bom_str(buf.trim_start());
            if trimmed.is_empty() {
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: "stdin is empty; expected JSON array or NDJSON pairs".to_string(),
                }
                .into());
            }
            serde_json::from_str::<Vec<EditPair>>(trimmed).map_err(|e| {
                crate::error::AtomwriteError::InvalidInput {
                    reason: format!("failed to parse JSON array of edit pairs: {e}"),
                }
            })?
        } else {
            let mut pairs = Vec::with_capacity(8);
            let mut line_buf = String::new();
            let mut idx = 0usize;
            loop {
                let n = crate::output::read_limited_line(
                    &mut reader,
                    &mut line_buf,
                    crate::constants::MAX_NDJSON_LINE_SIZE,
                )
                .map_err(|e| crate::error::AtomwriteError::InvalidInput {
                    reason: format!("failed to read edit-loop NDJSON line {}: {e}", idx + 1),
                })?;
                if n == 0 {
                    break;
                }
                let trimmed = line_buf.trim();
                if trimmed.is_empty() {
                    idx += 1;
                    continue;
                }
                let pair: EditPair = serde_json::from_str(trimmed).map_err(|e| {
                    crate::error::AtomwriteError::InvalidInput {
                        reason: format!("failed to parse NDJSON pair at line {}: {e}", idx + 1),
                    }
                })?;
                pairs.push(pair);
                idx += 1;
            }
            if pairs.is_empty() {
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: "stdin is empty; expected JSON array or NDJSON pairs".to_string(),
                }
                .into());
            }
            pairs
        }
    };

    // Apply each pair via shared fuzzy cascade (v0.1.29 P0-1).
    let mut pair_results: Vec<EditLoopPairResult> = Vec::with_capacity(pairs.len());
    let mut applied = 0usize;
    let mut unmatched = 0usize;
    for (i, pair) in pairs.iter().enumerate() {
        match fuzzy::match_pair(
            &content,
            &pair.old,
            &pair.new,
            fuzzy_mode,
            fuzzy_threshold,
        ) {
            Ok((edited, _info)) => {
                content = edited;
                applied += 1;
                pair_results.push(EditLoopPairResult {
                    index: i + 1,
                    matched: true,
                    old: pair.old.clone(),
                    new: pair.new.clone(),
                });
            }
            Err(_) => {
                unmatched += 1;
                pair_results.push(EditLoopPairResult {
                    index: i + 1,
                    matched: false,
                    old: pair.old.clone(),
                    new: pair.new.clone(),
                });
            }
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
        backup: resolved_backup.backup,
        syntax_check: args.syntax_check.is_some(),
        retention: resolved_backup.retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup: resolved_backup.keep,
        durability: crate::platform::Durability::Auto,
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
