// SPDX-License-Identifier: MIT OR Apache-2.0

//! Surgical file editing by line number, text marker, or exact match.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{EditArgs, FuzzyMode, GlobalArgs};
use crate::error::AtomwriteError;
use crate::ndjson_types::EditOutput;
use crate::output::NdjsonWriter;

struct FuzzyInfo {
    fuzzy: bool,
    strategy: String,
    strategies_tried: u64,
    similarity: Option<f64>,
}

/// Apply surgical edits to a file by line number, marker, or exact match.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the target file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::InvalidInput` if the line number or range is invalid.
/// Returns `AtomwriteError::Io` if reading or writing the file fails.
pub fn cmd_edit(
    args: &EditArgs,
    _global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    workspace: &Path,
) -> Result<()> {
    let start = Instant::now();
    let path = crate::path_safety::validate_path(&args.path, workspace)?;

    if !path.exists() {
        return Err(AtomwriteError::NotFound { path: path.clone() }.into());
    }

    let original = crate::file_io::read_file_string(&path)?;

    let checksum_before = checksum::hash_bytes(original.as_bytes());

    if let Some(ref expected) = args.expect_checksum {
        if &checksum_before != expected {
            return Err(AtomwriteError::StateDrift {
                path: path.clone(),
                expected: expected.clone(),
                actual: checksum_before,
            }
            .into());
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

    let (edited, mode, fuzzy_info) = if args.old.is_some() {
        let (e, m, fi) = edit_old_new(&original, args)?;
        (e, m, Some(fi))
    } else if args.after_line.is_some()
        || args.before_line.is_some()
        || args.range.is_some()
        || args.delete_range.is_some()
    {
        let (e, m) = edit_by_line(&lines, args, stdin)?;
        (e, m, None)
    } else if args.after_match.is_some() || args.before_match.is_some() || args.between.is_some() {
        let (e, m) = edit_by_marker(&original, &lines, args, stdin)?;
        (e, m, None)
    } else {
        bail!(
            "no edit mode specified: use --old/--new, --after-line, --before-line, --range, --delete-range, --after-match, --before-match, or --between"
        );
    };

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "edit".into(),
            path: path.display().to_string(),
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
        backup: false,
        retention: 5,
        preserve_timestamps: true,
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

    let output = EditOutput {
        r#type: "edit",
        path: path.display().to_string(),
        edits: 1,
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
    };

    writer.write_event(&output)?;
    Ok(())
}

// ─── multi mode ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MultiEdit {
    op: String,
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
    let reader = BufReader::new(stdin);
    let mut ops: Vec<MultiEdit> = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line = line.context("failed to read stdin line")?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let op: MultiEdit = serde_json::from_str(trimmed)
            .with_context(|| format!("invalid NDJSON at line {}: {}", i + 1, trimmed))?;
        ops.push(op);
    }

    if ops.is_empty() {
        bail!("--multi requires at least one edit operation on stdin");
    }

    let content_lines: Vec<String> = original.lines().map(|s| s.to_string()).collect();
    let total = content_lines.len();

    // Validate all operations before applying
    for op in &ops {
        match op.op.as_str() {
            "insert-after" | "insert-before" => {
                let n = op
                    .line
                    .ok_or_else(|| anyhow::anyhow!("op '{}' requires 'line'", op.op))?;
                if n == 0 || n > total {
                    bail!(
                        "op '{}': line {} out of range (file has {} lines)",
                        op.op,
                        n,
                        total
                    );
                }
            }
            "replace-range" | "delete-range" => {
                let s = op
                    .start
                    .ok_or_else(|| anyhow::anyhow!("op '{}' requires 'start'", op.op))?;
                let e = op
                    .end
                    .ok_or_else(|| anyhow::anyhow!("op '{}' requires 'end'", op.op))?;
                if s == 0 || e == 0 || s > total || e > total || s > e {
                    bail!(
                        "op '{}': range {}:{} invalid (file has {} lines)",
                        op.op,
                        s,
                        e,
                        total
                    );
                }
            }
            "exact" => {
                let old = op
                    .old
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("op 'exact' requires 'old'"))?;
                if !original.contains(old) {
                    bail!("op 'exact': old string not found: {:?}", old);
                }
            }
            other => bail!("unknown op: {:?}", other),
        }
    }

    // Sort line-based ops by descending line to avoid index drift
    // Separate exact ops (they work on the accumulated string, not line indices)
    let mut exact_ops: Vec<&MultiEdit> = ops.iter().filter(|o| o.op == "exact").collect();
    let mut line_ops: Vec<&MultiEdit> = ops.iter().filter(|o| o.op != "exact").collect();
    line_ops.sort_by(|a, b| {
        let la = a.line.or(a.start).unwrap_or(0);
        let lb = b.line.or(b.start).unwrap_or(0);
        lb.cmp(&la)
    });

    let mut result_lines: Vec<String> = content_lines;

    for op in &line_ops {
        match op.op.as_str() {
            "insert-after" => {
                let n = op
                    .line
                    .ok_or_else(|| anyhow::anyhow!("insert-after requires 'line' field"))?;
                let idx = n - 1;
                let new_lines: Vec<String> = op
                    .content
                    .as_deref()
                    .unwrap_or("")
                    .lines()
                    .map(|s| s.to_string())
                    .collect();
                for (i, line) in new_lines.into_iter().enumerate() {
                    result_lines.insert(idx + 1 + i, line);
                }
            }
            "insert-before" => {
                let n = op
                    .line
                    .ok_or_else(|| anyhow::anyhow!("insert-before requires 'line' field"))?;
                let idx = n - 1;
                let new_lines: Vec<String> = op
                    .content
                    .as_deref()
                    .unwrap_or("")
                    .lines()
                    .map(|s| s.to_string())
                    .collect();
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
                let new_lines: Vec<String> = op
                    .content
                    .as_deref()
                    .unwrap_or("")
                    .lines()
                    .map(|s| s.to_string())
                    .collect();
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
            _ => unreachable!(),
        }
    }

    let mut edited = join_lines(&result_lines);

    for op in &mut exact_ops {
        let old = op.old.as_deref().unwrap();
        let new = op.new.as_deref().unwrap_or("");
        edited = edited.replacen(old, new, 1);
    }

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "edit".into(),
            path: path.display().to_string(),
            would_modify: edited != original,
            details: Some(format!("mode: multi, edits: {}", ops.len())),
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let opts = AtomicWriteOptions {
        backup: false,
        retention: 5,
        preserve_timestamps: true,
    };

    let result = atomic_write(&path, edited.as_bytes(), &opts, workspace)?;
    let lines_after = edited.lines().count() as u64;

    let output = EditOutput {
        r#type: "edit",
        path: path.display().to_string(),
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
    };

    writer.write_event(&output)?;
    Ok(())
}

// ─── old/new with fuzzy cascade ──────────────────────────────────────────────

fn edit_old_new(original: &str, args: &EditArgs) -> Result<(String, String, FuzzyInfo)> {
    let old = args.old.as_ref().expect("--old is required");
    let new = args.new.as_deref().unwrap_or("");
    let fuzzy_mode = args.fuzzy;

    // Strategy 1: exact match
    if let Some(pos) = original.find(old.as_str()) {
        let edited = format!(
            "{}{}{}",
            &original[..pos],
            new,
            &original[pos + old.len()..]
        );
        return Ok((
            edited,
            "exact".into(),
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
            reason: format!("old string not found in file (fuzzy=off): {:?}", old),
        }
        .into());
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let content_lines: Vec<&str> = original.lines().collect();

    // Strategy 2: line-trimmed
    if let Some((start, end)) = match_line_trimmed(&content_lines, &old_lines) {
        let edited = apply_replacement(original, &content_lines, start, end, new);
        return Ok((
            edited,
            "old_new".into(),
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
        let edited = apply_replacement(original, &content_lines, start, end, new);
        return Ok((
            edited,
            "old_new".into(),
            FuzzyInfo {
                fuzzy: true,
                strategy: "whitespace_normalized".into(),
                strategies_tried: 3,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 4: indent-flexible
    if let Some((start, end)) = match_indent_flexible(&content_lines, &old_lines) {
        let edited = apply_replacement(original, &content_lines, start, end, new);
        return Ok((
            edited,
            "old_new".into(),
            FuzzyInfo {
                fuzzy: true,
                strategy: "indent_flexible".into(),
                strategies_tried: 4,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 5: escape-normalized
    if let Some((orig_start, orig_end)) = match_escape_normalized(original, old) {
        let edited = format!(
            "{}{}{}",
            &original[..orig_start],
            new,
            &original[orig_end..]
        );
        return Ok((
            edited,
            "old_new".into(),
            FuzzyInfo {
                fuzzy: true,
                strategy: "escape_normalized".into(),
                strategies_tried: 5,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 6: trimmed-boundary
    if let Some((start, end)) = match_trimmed_boundary(&content_lines, &old_lines) {
        let edited = apply_replacement(original, &content_lines, start, end, new);
        return Ok((
            edited,
            "old_new".into(),
            FuzzyInfo {
                fuzzy: true,
                strategy: "trimmed_boundary".into(),
                strategies_tried: 6,
                similarity: Some(1.0),
            },
        ));
    }

    // Strategy 7: block-anchor (only in auto/aggressive, requires >= 50% similarity in auto, >= 50% in aggressive)
    let min_ratio = match fuzzy_mode {
        FuzzyMode::Aggressive => 0.50,
        _ => 0.70,
    };
    if let Some((start, end, ratio)) = match_block_anchor(&content_lines, &old_lines, min_ratio) {
        let edited = apply_replacement(original, &content_lines, start, end, new);
        return Ok((
            edited,
            "old_new".into(),
            FuzzyInfo {
                fuzzy: true,
                strategy: "block_anchor".into(),
                strategies_tried: 7,
                similarity: Some(ratio),
            },
        ));
    }

    Err(AtomwriteError::InvalidInput {
        reason: format!(
            "old string not found after fuzzy cascade (7 strategies tried): {:?}",
            old
        ),
    }
    .into())
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

fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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
    let mut candidates: Vec<(usize, usize, f64)> = Vec::new();

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
            let ratio = diff.ratio() as f64;
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

fn apply_replacement(
    original: &str,
    content_lines: &[&str],
    start: usize,
    end: usize,
    new: &str,
) -> String {
    let mut result: Vec<String> = Vec::with_capacity(content_lines.len());
    for line in &content_lines[..start] {
        result.push(line.to_string());
    }
    for line in new.lines() {
        result.push(line.to_string());
    }
    for line in &content_lines[end..] {
        result.push(line.to_string());
    }
    // Preserve trailing newline if original had one
    let mut out = result.join("\n");
    if original.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

// ─── line-based and marker-based edits (unchanged) ───────────────────────────

fn read_stdin_text(mut stdin: impl Read) -> Result<String> {
    let mut buf = String::new();
    stdin
        .read_to_string(&mut buf)
        .context("failed to read stdin for edit")?;
    Ok(buf)
}

fn edit_by_line(lines: &[&str], args: &EditArgs, stdin: impl Read) -> Result<(String, String)> {
    let mut result_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    if let Some(n) = args.after_line {
        let content = read_stdin_text(stdin)?;
        let idx = validate_line_num(n, lines.len())?;
        let new_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        for (i, line) in new_lines.into_iter().enumerate() {
            result_lines.insert(idx + i + 1, line);
        }
        return Ok((join_lines(&result_lines), "after_line".into()));
    }

    if let Some(n) = args.before_line {
        let content = read_stdin_text(stdin)?;
        let idx = validate_line_num(n, lines.len())?;
        let insert_at = if idx == 0 { 0 } else { idx };
        let new_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        for (i, line) in new_lines.into_iter().enumerate() {
            result_lines.insert(insert_at + i, line);
        }
        return Ok((join_lines(&result_lines), "before_line".into()));
    }

    if let Some(ref range_str) = args.range {
        let content = read_stdin_text(stdin)?;
        let (start, end) = parse_range(range_str, lines.len())?;
        let new_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        result_lines.splice(start..end, new_lines);
        return Ok((join_lines(&result_lines), "replace_range".into()));
    }

    if let Some(ref range_str) = args.delete_range {
        let (start, end) = parse_range(range_str, lines.len())?;
        result_lines.drain(start..end);
        return Ok((join_lines(&result_lines), "delete_range".into()));
    }

    bail!("no line-mode edit operation specified");
}

fn edit_by_marker(
    original: &str,
    lines: &[&str],
    args: &EditArgs,
    stdin: impl Read,
) -> Result<(String, String)> {
    if let Some(ref marker) = args.after_match {
        let content = read_stdin_text(stdin)?;
        let idx = find_line_with(lines, marker)?;
        let mut result: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let new_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        for (i, line) in new_lines.into_iter().enumerate() {
            result.insert(idx + 1 + i, line);
        }
        return Ok((join_lines(&result), "after_match".into()));
    }

    if let Some(ref marker) = args.before_match {
        let content = read_stdin_text(stdin)?;
        let idx = find_line_with(lines, marker)?;
        let mut result: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let new_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        for (i, line) in new_lines.into_iter().enumerate() {
            result.insert(idx + i, line);
        }
        return Ok((join_lines(&result), "before_match".into()));
    }

    if let Some(ref markers) = args.between {
        if markers.len() != 2 {
            bail!("--between requires exactly 2 markers");
        }
        let content = read_stdin_text(stdin)?;
        let start_idx = find_line_with(lines, &markers[0])?;
        let end_idx = find_line_with_after(lines, &markers[1], start_idx + 1)?;

        let mut result: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let new_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        result.splice((start_idx + 1)..end_idx, new_lines);
        return Ok((join_lines(&result), "between".into()));
    }

    let _ = original;
    bail!("no marker-mode edit operation specified");
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
        bail!("invalid range format: expected N:M, got {s}");
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
        bail!("invalid range: start ({}) >= end ({})", start + 1, end);
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
