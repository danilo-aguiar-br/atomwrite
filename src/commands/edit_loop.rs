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
//!
//! v0.1.37 — the batch is all-or-nothing by default, like `edit`: the first
//! pair that does not match aborts with exit 65 and nothing is written.
//! `--partial` writes the pairs that matched and exits 87.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use serde::Deserialize;

use crate::atomic::{AtomicWriteOptions, atomic_write_based};
use crate::cli::FuzzyMode;
use crate::cli::{EditLoopArgs, GlobalArgs};
use crate::commands::{ResolvedBackup, resolve_backup};
use crate::error::AtomwriteError;
use crate::fuzzy;
use crate::ndjson_types::{EditLoopPairResult, EditLoopSummary, PairResult};
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

/// Outcome of running the pair list against the in-memory content.
struct PairOutcome {
    content: String,
    results: Vec<EditLoopPairResult>,
    applied: usize,
    unmatched: usize,
}

fn empty_stdin_error() -> AtomwriteError {
    AtomwriteError::InvalidInput {
        reason: "stdin is empty; expected JSON array or NDJSON pairs".to_string(),
    }
}

/// Peek past a leading BOM and whitespace and return the first payload byte.
fn peek_first_byte(reader: &mut impl BufRead) -> Result<Option<u8>> {
    let filled = reader.fill_buf()?;
    let mut i = if filled.starts_with(&crate::constants::UTF8_BOM) {
        crate::constants::UTF8_BOM.len()
    } else {
        0
    };
    while i < filled.len() && matches!(filled[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    Ok(filled.get(i).copied())
}

/// Parse the whole stdin as one JSON array of pairs (size-capped).
///
/// # Errors
///
/// Returns `AtomwriteError::FileTooLarge` when the payload passes `max_size`,
/// `AtomwriteError::InvalidInput` when the payload is empty or not a JSON array
/// of edit pairs, and the typed cancel error when the shutdown signal fires.
fn parse_json_array(reader: impl Read, max_size: u64) -> Result<Vec<EditPair>> {
    // The shared reader owns the chunk size, the cap and the cancellation token.
    let (bytes, _) = crate::commands::read_stdin_content_cancellable(
        reader,
        Some(max_size),
        crate::commands::StdinCapOrigin::GlobalMaxFilesize,
        true,
        None,
    )?;
    let buf = String::from_utf8(bytes).map_err(|e| AtomwriteError::InvalidInput {
        reason: format!("edit-loop: stdin is not valid UTF-8: {e}"),
    })?;
    let trimmed = crate::output::strip_utf8_bom_str(buf.trim_start());
    if trimmed.is_empty() {
        return Err(empty_stdin_error().into());
    }
    let pairs = serde_json::from_str::<Vec<EditPair>>(trimmed).map_err(|e| {
        AtomwriteError::InvalidInput {
            reason: format!("failed to parse JSON array of edit pairs: {e}"),
        }
    })?;
    Ok(pairs)
}

/// Stream NDJSON pairs, one object per line, with a per-line size limit.
fn parse_ndjson(reader: &mut impl BufRead) -> Result<Vec<EditPair>> {
    let mut pairs = Vec::with_capacity(crate::constants::EDIT_PAIRS_CAPACITY);
    let mut line_buf = String::new();
    let mut idx = 0usize;
    loop {
        let n = crate::output::read_limited_line(
            reader,
            &mut line_buf,
            crate::constants::MAX_NDJSON_LINE_SIZE,
        )
        .map_err(|e| AtomwriteError::InvalidInput {
            reason: format!("failed to read edit-loop NDJSON line {}: {e}", idx + 1),
        })?;
        if n == 0 {
            break;
        }
        idx += 1;
        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            continue;
        }
        let pair: EditPair =
            serde_json::from_str(trimmed).map_err(|e| AtomwriteError::InvalidInput {
                reason: format!("failed to parse NDJSON pair at line {idx}: {e}"),
            })?;
        pairs.push(pair);
    }
    if pairs.is_empty() {
        return Err(empty_stdin_error().into());
    }
    Ok(pairs)
}

/// Read the pair list from stdin. Accepts a JSON array or NDJSON; the
/// dialect is chosen by the first non-whitespace byte (`[` = array).
fn read_pairs(stdin: impl Read, max_size: u64) -> Result<Vec<EditPair>> {
    let mut reader = BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
    if peek_first_byte(&mut reader)? == Some(b'[') {
        parse_json_array(reader, max_size)
    } else {
        parse_ndjson(&mut reader)
    }
}

/// Build the abort error for the first unmatched pair, mirroring `edit`:
/// a single pair surfaces the match error itself, several pairs surface
/// `EditPairFailed` with the per-pair diagnostics gathered so far.
fn abort_error(
    index: u64,
    total: u64,
    results: Vec<PairResult>,
    err: AtomwriteError,
) -> AtomwriteError {
    if total == 1 {
        return err;
    }
    let (reason, best_candidate) = match err {
        AtomwriteError::MatchFailed {
            reason,
            best_candidate,
            ..
        }
        | AtomwriteError::MatchAmbiguous {
            reason,
            best_candidate,
            ..
        } => (reason, best_candidate),
        AtomwriteError::InvalidInput { reason } => (reason, None),
        other => (other.to_string(), None),
    };
    AtomwriteError::EditPairFailed {
        index,
        total,
        reason,
        pair_results: Box::new(results),
        best_candidate,
    }
}

/// True for the errors that mean "this pair did not match".
fn is_pair_miss(err: &AtomwriteError) -> bool {
    matches!(
        err,
        AtomwriteError::MatchFailed { .. } | AtomwriteError::MatchAmbiguous { .. }
    )
}

/// Run every pair against `content` through the shared fuzzy cascade.
///
/// Without `partial` the first miss returns the abort error; with `partial`
/// misses are recorded and the remaining pairs still run.
fn apply_pairs(
    mut content: String,
    pairs: &[EditPair],
    partial: bool,
    mode: FuzzyMode,
    threshold: Option<f64>,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<PairOutcome> {
    let mut results: Vec<EditLoopPairResult> = Vec::with_capacity(pairs.len());
    let mut diagnostics: Vec<PairResult> = Vec::with_capacity(pairs.len());
    let mut applied = 0usize;
    let mut unmatched = 0usize;
    for (i, pair) in pairs.iter().enumerate() {
        let matched = match fuzzy::match_pair_cfg(
            &content, &pair.old, &pair.new, mode, threshold, fuzzy_cfg, false,
        ) {
            Ok((edited, info)) => {
                content = edited;
                applied += 1;
                diagnostics.push(PairResult {
                    index: (i + 1) as u64,
                    matched: true,
                    strategy: Some(info.strategy),
                    similarity: info.similarity,
                    source: None,
                });
                true
            }
            // Only a miss is a pair that did not match. A cancel, a refused
            // input (empty `old`, a file above the fuzzy content cap) or an
            // internal error ends the run with its own error, in both modes.
            Err(err) if !is_pair_miss(&err) => return Err(err.into()),
            Err(err) if !partial => {
                diagnostics.push(PairResult {
                    index: (i + 1) as u64,
                    matched: false,
                    strategy: None,
                    similarity: None,
                    source: None,
                });
                return Err(
                    abort_error((i + 1) as u64, pairs.len() as u64, diagnostics, err).into(),
                );
            }
            Err(_) => {
                unmatched += 1;
                false
            }
        };
        results.push(EditLoopPairResult {
            index: i + 1,
            matched,
            old: pair.old.clone(),
            new: pair.new.clone(),
        });
    }
    Ok(PairOutcome {
        content,
        results,
        applied,
        unmatched,
    })
}

/// Apply N substitution pairs from NDJSON stdin to a single file.
///
/// Reads pairs as `{"old":"...","new":"..."}` per line (or one JSON array)
/// and applies them sequentially through the fuzzy cascade, each pair on the
/// first occurrence like single `edit --old/--new`. Writes the result
/// atomically with the same options as `edit` (backup, retention, syntax
/// check, line endings).
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the target does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns a parse error if the stdin NDJSON is malformed.
/// Returns `MatchFailed` / `EditPairFailed` (exit 65, nothing written) when a
/// pair does not match and `--partial` is off.
/// Returns `EditLoopPartial` (exit 87, matching pairs written) when a pair
/// does not match under `--partial`.
#[tracing::instrument(skip_all, fields(command = "edit-loop"))]
pub fn cmd_edit_loop(
    args: &EditLoopArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let target = validate_path(&args.path, &workspace)?;
    let (fuzzy_mode, fuzzy_threshold) =
        crate::config::resolve_fuzzy(FuzzyMode::Auto, None, fuzzy_cfg)?;

    if !target.exists() {
        return Err(AtomwriteError::NotFound { path: target }.into());
    }

    let max_size = global.effective_max_filesize();
    // Read the pairs first: the lock must not be held while stdin is awaited.
    let pairs = read_pairs(stdin, max_size)?;
    // A1: lock BEFORE reading the file, hold it to the commit.
    let _update_lock = crate::lock::acquire_for_update(&target)?;
    let content = crate::file_io::read_file_string(&target, max_size)?;
    // The writer compares this base under the lock, so a file that changed
    // between the read and the commit fails as drift instead of being lost.
    let checksum_before = crate::checksum::hash_bytes(content.as_bytes());
    let outcome = apply_pairs(
        content,
        &pairs,
        args.partial,
        fuzzy_mode,
        fuzzy_threshold,
        fuzzy_cfg,
    )?;
    let (applied, unmatched) = (outcome.applied, outcome.unmatched);

    // Nothing matched under `--partial`: there is nothing to write.
    let mut risk_assessment = None;
    if applied > 0 {
        let backup = resolve_backup(&args.backup_opts, defaults);
        risk_assessment = write_result(
            args,
            &target,
            &workspace,
            &checksum_before,
            outcome.content,
            &backup,
            write_cfg,
        )?;
    }

    let summary = EditLoopSummary {
        r#type: "result",
        action: "edit_loop".to_string(),
        path: target.display().to_string(),
        pairs_total: pairs.len(),
        pairs_applied: applied,
        pairs_unmatched: unmatched,
        elapsed_ms: start.elapsed().as_millis() as u64,
        pair_results: outcome.results,
        risk_assessment,
    };

    // `--partial` with no pair matching is a plain no-match, like `edit --partial`.
    // `--partial` ran and at least one pair missed: the file IS written with
    // the matching pairs, so exit 0 would hide the miss from the caller. The
    // summary is emitted first so the counts stay available, and it says
    // `ok: false` itself when the run ends in an error envelope.
    let failure = if applied == 0 {
        Some(AtomwriteError::NoMatches)
    } else if unmatched > 0 {
        Some(AtomwriteError::EditLoopPartial {
            unmatched: unmatched as u64,
            total: pairs.len() as u64,
        })
    } else {
        None
    };
    let Some(failure) = failure else {
        writer.write_terminal(&summary)?;
        return Ok(());
    };
    Err(writer.publish_failure(&summary, failure.into()))
}

/// Normalize line endings like `edit` and write the result atomically.
fn write_result(
    args: &EditLoopArgs,
    target: &Path,
    workspace: &Path,
    checksum_before: &str,
    mut content: String,
    backup: &ResolvedBackup,
    write_cfg: &crate::config::WriteSection,
) -> Result<Option<crate::ndjson_types::WriteRiskAssessment>> {
    use crate::line_endings::{self, LineEnding};
    let target_le = match args.line_ending {
        LineEnding::Auto => line_endings::detect(content.as_bytes()),
        other => other,
    };
    content = line_endings::normalize(&content, target_le);
    // Risk only. The pairs rewrite regions of the file, so a smaller result is
    // the pairs working and not a whole-content overwrite.
    let original_bytes = std::fs::metadata(target).map_or(0, |m| m.len());
    let risk_assessment = crate::commands::write_guards::assess_content_risk(
        content.as_bytes(),
        original_bytes,
        content.len() as u64,
        write_cfg,
    );
    let opts = AtomicWriteOptions {
        backup: backup.backup,
        syntax_check: args.syntax_check.is_some(),
        retention: backup.retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: args.strict.strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup: backup.keep,
        durability: crate::platform::Durability::Auto,
    };
    atomic_write_based(
        target,
        content.as_bytes(),
        &opts,
        workspace,
        Some(checksum_before),
    )?;
    Ok(risk_assessment)
}
