// SPDX-License-Identifier: MIT OR Apache-2.0

//! Surgical file editing by line number, text marker, or exact match.
//! Workload: I/O-bound (file read + fuzzy match + atomic write).

use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{EditArgs, FuzzyMode, GlobalArgs};
use crate::commands::resolve_backup;
use crate::error::AtomwriteError;

fn find_str(haystack: &str, needle: &str) -> Option<usize> {
    memchr::memmem::find(haystack.as_bytes(), needle.as_bytes())
}
use crate::ndjson_types::{EditOutput, PairResult};
use crate::output::NdjsonWriter;

struct FuzzyInfo {
    fuzzy: bool,
    strategy: String,
    strategies_tried: u64,
    similarity: Option<f64>,
}

fn strip_file_trailing_newline(s: String) -> String {
    if s.ends_with("\r\n") {
        s[..s.len() - 2].to_string()
    } else if s.ends_with('\n') {
        s[..s.len() - 1].to_string()
    } else {
        s
    }
}

fn resolve_edit_pairs(args: &EditArgs, workspace: &Path) -> Result<(Vec<String>, Vec<String>)> {
    if (!args.old.is_empty() && !args.new_file.is_empty())
        || (!args.old_file.is_empty() && !args.new.is_empty())
    {
        return Err(AtomwriteError::InvalidInput {
            reason: "cannot mix --old with --new-file or --old-file with --new; \
                     use both from the same source (--old/--new or --old-file/--new-file)"
                .into(),
        }
        .into());
    }
    if !args.old_file.is_empty() {
        if args.old_file.len() != args.new_file.len() {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "--old-file count ({}) must match --new-file count ({})",
                    args.old_file.len(),
                    args.new_file.len()
                ),
            }
            .into());
        }
        let mut olds = Vec::with_capacity(args.old_file.len());
        let mut news = Vec::with_capacity(args.new_file.len());
        for (of, nf) in args.old_file.iter().zip(args.new_file.iter()) {
            let of_path = crate::path_safety::validate_path(of, workspace)?;
            let nf_path = crate::path_safety::validate_path(nf, workspace)?;
            let old_raw =
                std::fs::read_to_string(&of_path).map_err(|_| AtomwriteError::NotFound {
                    path: of_path.clone(),
                })?;
            let new_raw =
                std::fs::read_to_string(&nf_path).map_err(|_| AtomwriteError::NotFound {
                    path: nf_path.clone(),
                })?;
            olds.push(strip_file_trailing_newline(old_raw));
            news.push(strip_file_trailing_newline(new_raw));
        }
        Ok((olds, news))
    } else {
        Ok((args.old.clone(), args.new.clone()))
    }
}

/// Apply surgical edits to a file by line number, marker, or exact match.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the target file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::InvalidInput` if the line number or range is invalid.
/// Returns `AtomwriteError::Io` if reading or writing the file fails.
#[tracing::instrument(skip_all, fields(command = "edit"))]
pub fn cmd_edit(
    args: &EditArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    workspace: &Path,
) -> Result<()> {
    let start = Instant::now();
    let path = crate::path_safety::validate_path(&args.path, workspace)?;
    let effective_backup = resolve_backup(args.backup, args.no_backup);

    if !path.exists() {
        return Err(AtomwriteError::NotFound { path: path.clone() }.into());
    }

    let original = crate::file_io::read_file_string(&path, global.effective_max_filesize())?;

    let checksum_before = checksum::hash_bytes(original.as_bytes());

    if let Some(ref expected) = args.expect_checksum {
        if &checksum_before != expected {
            if args.allow_sequential_drift {
                tracing::warn!(
                    expected = %expected,
                    actual = %checksum_before,
                    "drift aceito por --allow-sequential-drift"
                );
            } else {
                return Err(AtomwriteError::StateDrift {
                    path: path.clone(),
                    expected: expected.clone(),
                    actual: checksum_before,
                }
                .into());
            }
        }
    }

    let lines: Vec<&str> = original.lines().collect();
    let lines_before = lines.len() as u64;

    if args.multi {
        return cmd_edit_multi(
            args,
            original,
            path,
            checksum_before,
            lines_before,
            stdin,
            writer,
            workspace,
            start,
        );
    }

    let (effective_old, effective_new) = resolve_edit_pairs(args, workspace)?;

    if !effective_old.is_empty() && effective_old.len() != effective_new.len() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "--old/--old-file and --new/--new-file must be provided in equal pairs ({} old, {} new)",
                effective_old.len(),
                effective_new.len()
            ),
        }
        .into());
    }

    let (edited, mode, fuzzy_info, multi_report) = if !effective_old.is_empty() {
        let (e, m, fi, report) = edit_old_new(
            &original,
            &effective_old,
            &effective_new,
            args.fuzzy,
            args.partial,
        )?;
        (e, m, Some(fi), report)
    } else if args.after_line.is_some()
        || args.before_line.is_some()
        || args.range.is_some()
        || args.delete_range.is_some()
    {
        let max_size = global.effective_max_filesize();
        let (e, m) = edit_by_line(&lines, args, stdin, max_size)?;
        (e, m, None, None)
    } else if args.after_match.is_some() || args.before_match.is_some() || args.between.is_some() {
        let max_size = global.effective_max_filesize();
        let (e, m) = edit_by_marker(&original, &lines, args, stdin, max_size)?;
        (e, m, None, None)
    } else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "no edit mode specified: use --old/--new, --after-line, --before-line, --range, --delete-range, --after-match, --before-match, or --between".into(),
        }
        .into());
    };

    let path_str = path.display().to_string();

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "edit".into(),
            path: path_str,
            would_modify: edited != original,
            details: Some(format!("mode: {mode}")),
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let edited = {
        use crate::line_endings::{self, LineEnding};
        let target = match args.line_ending {
            LineEnding::Auto => line_endings::detect(original.as_bytes()),
            other => other,
        };
        line_endings::normalize(&edited, target)
    };

    let opts = AtomicWriteOptions {
        backup: effective_backup,
        syntax_check: false,
        retention: args.retention,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: args.wal_policy,
        keep_backup: args.keep_backup,
    };

    let result = atomic_write(&path, edited.as_bytes(), &opts, workspace)?;
    let lines_after = edited.lines().count() as u64;

    let (fuzzy, strategy, strategies_tried, similarity) = match fuzzy_info {
        Some(fi) => (
            Some(fi.fuzzy),
            Some(fi.strategy),
            Some(fi.strategies_tried),
            fi.similarity,
        ),
        None => (None, None, None, None),
    };

    let from_file = !args.old_file.is_empty();
    let (edits, pairs_total, pair_results) = match multi_report {
        Some(mut report) => {
            if from_file {
                for pr in &mut report.pair_results {
                    pr.source = Some("file".into());
                }
            }
            (
                report.applied,
                Some(report.pairs_total),
                Some(report.pair_results),
            )
        }
        None => (1, None, None),
    };

    let output = EditOutput {
        r#type: "edit",
        path: path_str,
        edits,
        mode,
        bytes_before: original.len() as u64,
        bytes_after: edited.len() as u64,
        checksum_before,
        checksum_after: result.checksum,
        lines_before,
        lines_after,
        elapsed_ms: start.elapsed().as_millis() as u64,
        fuzzy,
        strategy,
        strategies_tried,
        similarity,
        pairs_total,
        pair_results,
        mtime_preserved: Some(args.preserve_timestamps),
    };

    writer.write_event(&output)?;
    Ok(())
}

// ─── multi mode ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiEdit {
    #[serde(default)]
    op: Option<String>,
    #[serde(default)]
    line: Option<usize>,
    #[serde(default)]
    start: Option<usize>,
    #[serde(default)]
    end: Option<usize>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    old: Option<String>,
    #[serde(default)]
    new: Option<String>,
}

impl MultiEdit {
    fn effective_op(&self) -> &str {
        if let Some(ref op) = self.op {
            return op.as_str();
        }
        if self.old.is_some() && self.new.is_some() {
            return "exact";
        }
        "unknown"
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_edit_multi(
    args: &EditArgs,
    original: String,
    path: std::path::PathBuf,
    checksum_before: String,
    lines_before: u64,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    workspace: &Path,
    start: Instant,
) -> Result<()> {
    let effective_backup = resolve_backup(args.backup, args.no_backup);
    let mut reader = BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
    let mut ops: Vec<MultiEdit> = Vec::with_capacity(8);
    let mut line_buf = String::new();
    let mut i = 0usize;

    loop {
        let n = crate::output::read_limited_line(
            &mut reader,
            &mut line_buf,
            crate::constants::MAX_NDJSON_LINE_SIZE,
        )
        .context("failed to read stdin line")?;
        if n == 0 {
            break;
        }
        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let jd = &mut serde_json::Deserializer::from_str(trimmed);
        let op: MultiEdit = serde_path_to_error::deserialize(jd)
            .with_context(|| format!("invalid NDJSON at line {}: {}", i + 1, trimmed))?;
        ops.push(op);
        i += 1;
    }

    if ops.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "--multi requires at least one edit operation on stdin".into(),
        }
        .into());
    }

    let content_lines = lines_from_str(&original);
    let total = content_lines.len();

    // Validate all operations before applying
    for op in &ops {
        let effective = op.effective_op();
        match effective {
            "insert-after" | "insert-before" => {
                let n = op
                    .line
                    .ok_or_else(|| anyhow::anyhow!("op '{}' requires 'line'", effective))?;
                if n == 0 || n > total {
                    return Err(crate::error::AtomwriteError::InvalidInput {
                        reason: format!(
                            "op '{}': line {} out of range (file has {} lines)",
                            effective, n, total
                        ),
                    }
                    .into());
                }
            }
            "replace-range" | "delete-range" => {
                let s = op
                    .start
                    .ok_or_else(|| anyhow::anyhow!("op '{}' requires 'start'", effective))?;
                let e = op
                    .end
                    .ok_or_else(|| anyhow::anyhow!("op '{}' requires 'end'", effective))?;
                if s == 0 || e == 0 || s > total || e > total || s > e {
                    return Err(crate::error::AtomwriteError::InvalidInput {
                        reason: format!(
                            "op '{}': range {}:{} invalid (file has {} lines)",
                            effective, s, e, total
                        ),
                    }
                    .into());
                }
            }
            "exact" => {
                let old = op
                    .old
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("op 'exact' requires 'old'"))?;
                if find_str(&original, old).is_none() {
                    return Err(crate::error::AtomwriteError::InvalidInput {
                        reason: format!("op 'exact': old string not found: {:?}", old),
                    }
                    .into());
                }
            }
            other => {
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: format!("unknown op: {:?}", other),
                }
                .into());
            }
        }
    }

    // Sort line-based ops by descending line to avoid index drift
    // Separate exact ops (they work on the accumulated string, not line indices)
    let mut exact_ops: Vec<&MultiEdit> = ops.iter().filter(|o| o.effective_op() == "exact").collect();
    let mut line_ops: Vec<&MultiEdit> = ops.iter().filter(|o| o.effective_op() != "exact").collect();
    line_ops.sort_by(|a, b| {
        let la = a.line.or(a.start).unwrap_or(0);
        let lb = b.line.or(b.start).unwrap_or(0);
        lb.cmp(&la)
    });

    let mut result_lines: Vec<String> = content_lines;

    for op in &line_ops {
        match op.effective_op() {
            "insert-after" => {
                let n = op
                    .line
                    .ok_or_else(|| anyhow::anyhow!("insert-after requires 'line' field"))?;
                let idx = n - 1;
                let src = op.content.as_deref().unwrap_or("");
                let new_lines = lines_from_str(src);
                for (i, line) in new_lines.into_iter().enumerate() {
                    result_lines.insert(idx + 1 + i, line);
                }
            }
            "insert-before" => {
                let n = op
                    .line
                    .ok_or_else(|| anyhow::anyhow!("insert-before requires 'line' field"))?;
                let idx = n - 1;
                let src = op.content.as_deref().unwrap_or("");
                let new_lines = lines_from_str(src);
                for (i, line) in new_lines.into_iter().enumerate() {
                    result_lines.insert(idx + i, line);
                }
            }
            "replace-range" => {
                let s = op
                    .start
                    .ok_or_else(|| anyhow::anyhow!("replace-range requires 'start' field"))?
                    - 1;
                let e = op
                    .end
                    .ok_or_else(|| anyhow::anyhow!("replace-range requires 'end' field"))?;
                let src = op.content.as_deref().unwrap_or("");
                let new_lines = lines_from_str(src);
                result_lines.splice(s..e, new_lines);
            }
            "delete-range" => {
                let s = op
                    .start
                    .ok_or_else(|| anyhow::anyhow!("delete-range requires 'start' field"))?
                    - 1;
                let e = op
                    .end
                    .ok_or_else(|| anyhow::anyhow!("delete-range requires 'end' field"))?;
                result_lines.drain(s..e);
            }
            _ => unreachable!("ops filtered to known variants in validation loop"),
        }
    }

    let mut edited = join_lines(&result_lines);

    for op in &mut exact_ops {
        let old = op.old.as_deref().expect("validated in op filter loop");
        let new = op.new.as_deref().unwrap_or("");
        edited = edited.replacen(old, new, 1);
    }

    let path_str = path.display().to_string();

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "edit".into(),
            path: path_str,
            would_modify: edited != original,
            details: Some(format!("mode: multi, edits: {}", ops.len())),
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let opts = AtomicWriteOptions {
        backup: effective_backup,
        syntax_check: false,
        retention: args.retention,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: args.wal_policy,
        keep_backup: args.keep_backup,
    };

    let result = atomic_write(&path, edited.as_bytes(), &opts, workspace)?;
    let lines_after = edited.lines().count() as u64;
    let output = EditOutput {
        r#type: "edit",
        path: path_str,
        edits: ops.len() as u64,
        mode: "multi".into(),
        bytes_before: original.len() as u64,
        bytes_after: edited.len() as u64,
        checksum_before,
        checksum_after: result.checksum,
        lines_before,
        lines_after,
        elapsed_ms: start.elapsed().as_millis() as u64,
        fuzzy: None,
        strategy: None,
        strategies_tried: None,
        similarity: None,
        pairs_total: None,
        pair_results: None,
        mtime_preserved: Some(args.preserve_timestamps),
    };

    writer.write_event(&output)?;
    Ok(())
}

// ─── old/new with fuzzy cascade ──────────────────────────────────────────────

/// Per-pair diagnostics produced by multi-pair `--old`/`--new` editing (G117).
struct MultiReport {
    pair_results: Vec<PairResult>,
    pairs_total: u64,
    applied: u64,
}

fn edit_old_new(
    original: &str,
    old: &[String],
    new: &[String],
    fuzzy: FuzzyMode,
    partial: bool,
) -> Result<(String, String, FuzzyInfo, Option<MultiReport>)> {
    if old.len() > 1 {
        return edit_old_new_multi(original, old, new, fuzzy, partial);
    }
    let old_str = &old[0];
    let new_str = new.first().map(|s| s.as_str()).unwrap_or("");
    match match_pair(original, old_str, new_str, fuzzy) {
        Ok((edited, info)) => {
            let mode = if info.strategy == "exact" {
                "exact".into()
            } else {
                "old_new".into()
            };
            Ok((edited, mode, info, None))
        }
        Err(_) if partial => Err(AtomwriteError::NoMatches.into()),
        Err(err) => Err(err.into()),
    }
}

/// Match a single `old` string in `content` via the 9-strategy fuzzy cascade
/// and return the edited content with `new` substituted.
///
/// Shared by the single-pair and multi-pair `--old`/`--new` paths so both
/// have identical fuzzy behavior (G117 fix: the multi path previously used
/// exact matching only).
fn match_pair(
    content: &str,
    old: &str,
    new: &str,
    fuzzy_mode: FuzzyMode,
) -> std::result::Result<(String, FuzzyInfo), AtomwriteError> {
    // Strategy 1: exact match
    if let Some(pos) = find_str(content, old) {
        let edited = format!("{}{}{}", &content[..pos], new, &content[pos + old.len()..]);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: false,
                strategy: "exact".into(),
                strategies_tried: 1,
                similarity: None,
            },
        ));
    }

    if matches!(fuzzy_mode, FuzzyMode::Off) {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("old string not found in file (fuzzy=off): {old:?}"),
        });
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let content_lines: Vec<&str> = content.lines().collect();

    // Strategy 2: line-trimmed
    if let Some((start, end)) = match_line_trimmed(&content_lines, &old_lines) {
        let edited = apply_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "line_trimmed".into(),
                strategies_tried: 2,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 3: whitespace-normalized
    if let Some((start, end)) = match_whitespace_normalized(&content_lines, &old_lines) {
        let edited = apply_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "whitespace_normalized".into(),
                strategies_tried: 3,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 3.5: punctuation-whitespace-normalized
    if let Some((start, end)) = match_punctuation_normalized(&content_lines, &old_lines) {
        let edited = apply_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "punctuation_normalized".into(),
                strategies_tried: 4,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 4: indent-flexible
    if let Some((start, end)) = match_indent_flexible(&content_lines, &old_lines) {
        let edited = apply_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "indent_flexible".into(),
                strategies_tried: 5,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 5: escape-normalized
    if let Some((orig_start, orig_end)) = match_escape_normalized(content, old) {
        let edited = format!("{}{}{}", &content[..orig_start], new, &content[orig_end..]);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "escape_normalized".into(),
                strategies_tried: 6,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 7: trimmed-boundary
    if let Some((start, end)) = match_trimmed_boundary(&content_lines, &old_lines) {
        let edited = apply_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "trimmed_boundary".into(),
                strategies_tried: 8,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 8: block-anchor (only in auto/aggressive, requires >= 50% similarity in auto, >= 50% in aggressive)
    let min_ratio = match fuzzy_mode {
        FuzzyMode::Aggressive => 0.50,
        _ => 0.70,
    };
    if let Some((start, end, ratio)) = match_block_anchor(&content_lines, &old_lines, min_ratio) {
        let edited = apply_replacement(content, &content_lines, start, end, new);
        return Ok((
            edited,
            FuzzyInfo {
                fuzzy: true,
                strategy: "block_anchor".into(),
                strategies_tried: 8,
                similarity: Some(ratio),
            },
        ));
    }

    // Strategy 9: context-aware (G116, opt-in via aggressive or auto).
    // Uses `strsim::normalized_levenshtein` to find a window of `old_lines`
    // in `content_lines` with similarity >= 0.80. More expensive than
    // block-anchor but catches edits where leading/trailing context is
    // also wrong (e.g. when an LLM adds comments near the match).
    if matches!(fuzzy_mode, FuzzyMode::Aggressive | FuzzyMode::Auto) {
        if let Some((start, end, similarity)) =
            match_context_aware(&content_lines, &old_lines, 0.80)
        {
            let edited = apply_replacement(content, &content_lines, start, end, new);
            return Ok((
                edited,
                FuzzyInfo {
                    fuzzy: true,
                    strategy: "context_aware".into(),
                    strategies_tried: 9,
                    similarity: Some(similarity),
                },
            ));
        }
    }

    Err(AtomwriteError::InvalidInput {
        reason: format!("old string not found after fuzzy cascade (9 strategies tried): {old:?}"),
    })
}

// ─── matching strategies ─────────────────────────────────────────────────────

fn match_line_trimmed(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let trimmed_pat: Vec<&str> = pattern.iter().map(|l| l.trim()).collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, pat_line) in trimmed_pat.iter().enumerate() {
            if content[i + j].trim() != *pat_line {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn edit_old_new_multi(
    original: &str,
    old: &[String],
    new: &[String],
    fuzzy: FuzzyMode,
    partial: bool,
) -> Result<(String, String, FuzzyInfo, Option<MultiReport>)> {
    let pairs_total = old.len() as u64;
    let mut content = original.to_string();
    let mut pair_results: Vec<PairResult> = Vec::with_capacity(old.len());
    let mut applied = 0u64;
    let mut any_fuzzy = false;
    let mut max_strategies_tried = 0u64;

    for (i, (old_str, new_str)) in old.iter().zip(new.iter()).enumerate() {
        let index = (i + 1) as u64;
        match match_pair(&content, old_str, new_str, fuzzy) {
            Ok((edited, info)) => {
                content = edited;
                applied += 1;
                any_fuzzy |= info.fuzzy;
                max_strategies_tried = max_strategies_tried.max(info.strategies_tried);
                pair_results.push(PairResult {
                    index,
                    matched: true,
                    strategy: Some(info.strategy),
                    similarity: info.similarity,
                    source: None,
                });
            }
            Err(_) if partial => {
                pair_results.push(PairResult {
                    index,
                    matched: false,
                    strategy: None,
                    similarity: None,
                    source: None,
                });
            }
            Err(err) => {
                let reason = match err {
                    AtomwriteError::InvalidInput { reason } => reason,
                    other => other.to_string(),
                };
                pair_results.push(PairResult {
                    index,
                    matched: false,
                    strategy: None,
                    similarity: None,
                    source: None,
                });
                return Err(AtomwriteError::EditPairFailed {
                    index,
                    total: pairs_total,
                    reason,
                    pair_results,
                }
                .into());
            }
        }
    }

    if applied == 0 {
        // --partial with zero applicable pairs: no write, same exit semantics
        // as `replace` with zero matches.
        return Err(AtomwriteError::NoMatches.into());
    }

    let (mode, strategy) = if any_fuzzy {
        (format!("fuzzy-multi({applied})"), "fuzzy-multi")
    } else {
        (format!("exact-multi({applied})"), "exact-multi")
    };
    Ok((
        content,
        mode,
        FuzzyInfo {
            fuzzy: any_fuzzy,
            strategy: strategy.into(),
            strategies_tried: max_strategies_tried,
            similarity: None,
        },
        Some(MultiReport {
            pair_results,
            pairs_total,
            applied,
        }),
    ))
}

fn normalize_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut first = true;
    for word in s.split_whitespace() {
        if !first {
            result.push(' ');
        }
        result.push_str(word);
        first = false;
    }
    result
}

fn normalize_punctuation_whitespace(s: &str) -> String {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"\s*([(){}\[\]<>,;:])\s*").expect("static regex is valid")
    });
    RE.replace_all(&normalize_whitespace(s), "$1").to_string()
}

fn match_punctuation_normalized(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let norm_pat: Vec<String> = pattern
        .iter()
        .map(|l| normalize_punctuation_whitespace(l))
        .collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, norm) in norm_pat.iter().enumerate() {
            if normalize_punctuation_whitespace(content[i + j]) != *norm {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn match_whitespace_normalized(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let norm_pat: Vec<String> = pattern.iter().map(|l| normalize_whitespace(l)).collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, norm) in norm_pat.iter().enumerate() {
            if normalize_whitespace(content[i + j]) != *norm {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn match_indent_flexible(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let stripped_pat: Vec<&str> = pattern.iter().map(|l| l.trim_start()).collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, pat) in stripped_pat.iter().enumerate() {
            if content[i + j].trim_start() != *pat {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn normalize_escapes(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

fn match_escape_normalized(content: &str, pattern: &str) -> Option<(usize, usize)> {
    let norm_pat = normalize_escapes(pattern);
    if norm_pat == pattern {
        return None; // nothing to normalize, already tried exact
    }
    let norm_content = normalize_escapes(content);
    if let Some(norm_pos) = norm_content.find(&norm_pat) {
        // Map back: use byte position in original via ratio of normalized content
        // This is approximate; we use the norm_pos as-is since it's a best-effort strategy
        let end_pos = norm_pos + norm_pat.len();
        if end_pos <= content.len() {
            return Some((norm_pos, end_pos));
        }
    }
    None
}

fn match_trimmed_boundary(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    let start = pattern.iter().position(|l| !l.trim().is_empty())?;
    let end = pattern.iter().rposition(|l| !l.trim().is_empty())? + 1;
    if start >= end {
        return None;
    }
    let trimmed = &pattern[start..end];
    match_line_trimmed(content, trimmed)
}

fn match_block_anchor(
    content: &[&str],
    pattern: &[&str],
    min_ratio: f64,
) -> Option<(usize, usize, f64)> {
    if pattern.len() < 2 {
        return None;
    }
    let first = pattern.first()?.trim();
    let last = pattern.last()?.trim();
    let plen = pattern.len();
    let mut candidates: Vec<(usize, usize, f64)> = Vec::with_capacity(4);

    for (i, line) in content.iter().enumerate() {
        if line.trim() != first {
            continue;
        }
        let search_end = (i + plen * 2).min(content.len());
        for j in (i + 1)..search_end {
            if content[j].trim() != last {
                continue;
            }
            let block = content[i..=j].join("\n");
            let pat = pattern.join("\n");
            let diff = similar::TextDiff::from_lines(&pat, &block);
            let raw_ratio = diff.ratio() as f64;
            let ratio = if raw_ratio.is_finite() {
                raw_ratio
            } else {
                0.0
            };
            if ratio >= min_ratio {
                candidates.push((i, j + 1, ratio));
            }
        }
    }

    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    // Multiple candidates: keep only high-confidence ones
    candidates.retain(|c| c.2 >= 0.70);
    candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    candidates.first().copied()
}

/// Strategy 9: context-aware similarity via normalized Levenshtein (G116).
///
/// Slides a window of `pattern.len()` lines over `content` and computes
/// `strsim::normalized_levenshtein` between the joined window and the
/// pattern. Returns the first window whose similarity >= `threshold`.
/// More expensive than block-anchor but tolerates edits where leading
/// AND trailing context lines are also wrong (e.g. when an LLM rewrites
/// a few lines around the target match).
fn match_context_aware(
    content: &[&str],
    pattern: &[&str],
    threshold: f64,
) -> Option<(usize, usize, f64)> {
    if pattern.is_empty() || pattern.len() > content.len() {
        return None;
    }
    let pat_joined = pattern.join("\n");
    let plen = pattern.len();
    let mut best: Option<(usize, usize, f64)> = None;
    for i in 0..=(content.len() - plen) {
        let window_joined = content[i..i + plen].join("\n");
        let sim = strsim::normalized_levenshtein(&pat_joined, &window_joined);
        if sim >= threshold {
            return Some((i, i + plen, sim));
        }
        // Track the best match in case no candidate crosses the threshold.
        if best.is_none_or(|(_, _, b)| sim > b) {
            best = Some((i, i + plen, sim));
        }
    }
    // If nothing crossed the threshold, return the best one anyway if it's
    // within 0.05 of the threshold. This handles near-misses (e.g. 0.79 vs
    // 0.80) that are clearly the intended target.
    if let Some((i, j, sim)) = best {
        if sim >= threshold - 0.05 {
            return Some((i, j, sim));
        }
    }
    None
}

fn lines_from_str(s: &str) -> Vec<String> {
    s.lines().map(String::from).collect()
}

fn lines_to_owned(lines: &[&str]) -> Vec<String> {
    let mut v = Vec::with_capacity(lines.len());
    v.extend(lines.iter().map(|s| String::from(*s)));
    v
}

fn apply_replacement(
    original: &str,
    content_lines: &[&str],
    start: usize,
    end: usize,
    new: &str,
) -> String {
    let before = content_lines[..start].join("\n");
    let after = content_lines[end..].join("\n");
    let mut out = String::with_capacity(before.len() + new.len() + after.len() + 2);
    if !before.is_empty() {
        out.push_str(&before);
        out.push('\n');
    }
    out.push_str(new);
    if !after.is_empty() {
        out.push('\n');
        out.push_str(&after);
    }
    // Preserve trailing newline if original had one
    if original.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

// ─── line-based and marker-based edits (unchanged) ───────────────────────────

fn read_stdin_text(stdin: impl Read, max_size: u64) -> Result<String> {
    let mut buf = String::new();
    stdin
        .take(max_size)
        .read_to_string(&mut buf)
        .context("failed to read stdin for edit")?;
    Ok(buf)
}

fn edit_by_line(
    lines: &[&str],
    args: &EditArgs,
    stdin: impl Read,
    max_size: u64,
) -> Result<(String, String)> {
    let mut result_lines = lines_to_owned(lines);

    if let Some(n) = args.after_line {
        let content = read_stdin_text(stdin, max_size)?;
        let idx = validate_line_num(n, lines.len())?;
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result_lines.insert(idx + i + 1, line);
        }
        return Ok((join_lines(&result_lines), "after_line".into()));
    }

    if let Some(n) = args.before_line {
        let content = read_stdin_text(stdin, max_size)?;
        let idx = validate_line_num(n, lines.len())?;
        let insert_at = if idx == 0 { 0 } else { idx };
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result_lines.insert(insert_at + i, line);
        }
        return Ok((join_lines(&result_lines), "before_line".into()));
    }

    if let Some(ref range_str) = args.range {
        let content = read_stdin_text(stdin, max_size)?;
        let (start, end) = parse_range(range_str, lines.len())?;
        let new_lines = lines_from_str(&content);
        result_lines.splice(start..end, new_lines);
        return Ok((join_lines(&result_lines), "replace_range".into()));
    }

    if let Some(ref range_str) = args.delete_range {
        let (start, end) = parse_range(range_str, lines.len())?;
        result_lines.drain(start..end);
        return Ok((join_lines(&result_lines), "delete_range".into()));
    }

    Err(crate::error::AtomwriteError::InvalidInput {
        reason: "no line-mode edit operation specified".into(),
    }
    .into())
}

fn edit_by_marker(
    original: &str,
    lines: &[&str],
    args: &EditArgs,
    stdin: impl Read,
    max_size: u64,
) -> Result<(String, String)> {
    if let Some(ref marker) = args.after_match {
        let content = read_stdin_text(stdin, max_size)?;
        let idx = find_line_with(lines, marker)?;
        let mut result = lines_to_owned(lines);
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result.insert(idx + 1 + i, line);
        }
        return Ok((join_lines(&result), "after_match".into()));
    }

    if let Some(ref marker) = args.before_match {
        let content = read_stdin_text(stdin, max_size)?;
        let idx = find_line_with(lines, marker)?;
        let mut result = lines_to_owned(lines);
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result.insert(idx + i, line);
        }
        return Ok((join_lines(&result), "before_match".into()));
    }

    if let Some(ref markers) = args.between {
        if markers.len() != 2 {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: "--between requires exactly 2 markers".into(),
            }
            .into());
        }
        let content = read_stdin_text(stdin, max_size)?;
        let start_idx = find_line_with(lines, &markers[0])?;
        let end_idx = find_line_with_after(lines, &markers[1], start_idx + 1)?;

        let mut result = lines_to_owned(lines);
        let new_lines = lines_from_str(&content);
        result.splice((start_idx + 1)..end_idx, new_lines);
        return Ok((join_lines(&result), "between".into()));
    }

    let _ = original;
    Err(crate::error::AtomwriteError::InvalidInput {
        reason: "no marker-mode edit operation specified".into(),
    }
    .into())
}

fn validate_line_num(n: usize, total: usize) -> Result<usize> {
    if n == 0 || n > total {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("line {n} out of range (file has {total} lines)"),
        }
        .into());
    }
    Ok(n - 1)
}

fn parse_range(s: &str, total: usize) -> Result<(usize, usize)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid range format: expected N:M, got {s}"),
        }
        .into());
    }
    let start = parts[0]
        .parse::<usize>()
        .context("invalid range start")?
        .saturating_sub(1);
    let end = parts[1]
        .parse::<usize>()
        .context("invalid range end")?
        .min(total);

    if start >= end {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid range: start ({}) >= end ({})", start + 1, end),
        }
        .into());
    }

    Ok((start, end))
}

fn find_line_with(lines: &[&str], marker: &str) -> Result<usize> {
    for (i, line) in lines.iter().enumerate() {
        if line.contains(marker) {
            return Ok(i);
        }
    }
    Err(AtomwriteError::InvalidInput {
        reason: format!("marker not found: {marker:?}"),
    }
    .into())
}

fn find_line_with_after(lines: &[&str], marker: &str, after: usize) -> Result<usize> {
    for (i, line) in lines.iter().enumerate().skip(after) {
        if line.contains(marker) {
            return Ok(i);
        }
    }
    Err(AtomwriteError::InvalidInput {
        reason: format!("end marker not found after line {after}: {marker:?}"),
    }
    .into())
}

fn join_lines(lines: &[String]) -> String {
    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}
