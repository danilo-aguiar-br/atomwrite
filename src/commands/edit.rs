// SPDX-License-Identifier: MIT OR Apache-2.0

//! Surgical file editing by line number, text marker, or exact match.
//! Workload: I/O-bound (file read + fuzzy match + atomic write).
//! Parallelism: single target file (mutations ordered). Multi-pair
//! `--old-file`/`--new-file` reads fan out with `rayon` when count > 1;
//! within each pair, old∥new use `rayon::join` (two independent I/O paths).

use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
pub use crate::cli::FuzzyMode;
use crate::cli::{EditArgs, GlobalArgs};
use crate::commands::{read_stdin_text_guarded, resolve_backup};
use crate::error::AtomwriteError;
use crate::fuzzy::FuzzyInfo;
use crate::ndjson_types::{EditOutput, PairResult};
use crate::output::NdjsonWriter;

fn find_str(haystack: &str, needle: &str) -> Option<usize> {
    memchr::memmem::find(haystack.as_bytes(), needle.as_bytes())
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

fn resolve_edit_pairs(
    args: &EditArgs,
    workspace: &Path,
    max_size: u64,
) -> Result<(Vec<String>, Vec<String>)> {
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
        let pairs: Vec<(&std::path::PathBuf, &std::path::PathBuf)> = args
            .old_file
            .iter()
            .zip(args.new_file.iter())
            .collect();
        let loaded: Vec<Result<(String, String), anyhow::Error>> =
            if crate::concurrency::should_parallelize(pairs.len()) {
                use rayon::prelude::*;
                pairs
                    .par_iter()
                    .map(|(of, nf)| {
                        let of_path = crate::path_safety::validate_path(of, workspace)?;
                        let nf_path = crate::path_safety::validate_path(nf, workspace)?;
                        // Independent dual-file I/O within each pair.
                        let (old_raw, new_raw) = rayon::join(
                            || crate::file_io::read_file_string(&of_path, max_size),
                            || crate::file_io::read_file_string(&nf_path, max_size),
                        );
                        Ok((
                            strip_file_trailing_newline(old_raw?),
                            strip_file_trailing_newline(new_raw?),
                        ))
                    })
                    .collect()
            } else {
                pairs
                    .iter()
                    .map(|(of, nf)| {
                        let of_path = crate::path_safety::validate_path(of, workspace)?;
                        let nf_path = crate::path_safety::validate_path(nf, workspace)?;
                        // Even single-pair: dual independent reads via join.
                        let (old_raw, new_raw) = rayon::join(
                            || crate::file_io::read_file_string(&of_path, max_size),
                            || crate::file_io::read_file_string(&nf_path, max_size),
                        );
                        Ok((
                            strip_file_trailing_newline(old_raw?),
                            strip_file_trailing_newline(new_raw?),
                        ))
                    })
                    .collect()
            };
        let mut olds = Vec::with_capacity(loaded.len());
        let mut news = Vec::with_capacity(loaded.len());
        for item in loaded {
            let (o, n) = item?;
            olds.push(o);
            news.push(n);
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
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    stdin_is_tty: bool,
) -> Result<()> {
    let start = Instant::now();
    let (fuzzy_mode, fuzzy_threshold) =
        crate::config::resolve_fuzzy(args.fuzzy, args.fuzzy_threshold, fuzzy_cfg)?;
    let path = crate::path_safety::validate_path(&args.path, workspace)?;
    let resolved_backup = resolve_backup(&args.backup_opts, defaults);

    if !path.exists() {
        return Err(AtomwriteError::NotFound { path }.into());
    }

    let mode_count = [
        args.after_line.is_some(),
        args.before_line.is_some(),
        args.range.is_some(),
        args.delete_range.is_some(),
        !args.old.is_empty() || !args.old_file.is_empty(),
        args.after_match.is_some(),
        args.before_match.is_some(),
        args.between.is_some(),
        args.multi,
    ]
    .iter()
    .filter(|&&b| b)
    .count();
    if mode_count > 1 {
        return Err(AtomwriteError::InvalidInput {
            reason: "conflicting edit modes: specify only one of --old/--new, --after-line, \
                     --before-line, --range, --delete-range, --after-match, --before-match, \
                     --between, --multi"
                .into(),
        }
        .into());
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
                    path,
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
            defaults,
            stdin_is_tty,
        );
    }

    let (effective_old, effective_new) =
        resolve_edit_pairs(args, workspace, global.effective_max_filesize())?;

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
            fuzzy_mode,
            args.partial,
            fuzzy_threshold,
            args.replace_all,
        )?;
        (e, m, Some(fi), report)
    } else if args.after_line.is_some()
        || args.before_line.is_some()
        || args.range.is_some()
        || args.delete_range.is_some()
    {
        let max_size = global.effective_max_filesize();
        let (e, m) = edit_by_line(&lines, args, stdin, max_size, stdin_is_tty)?;
        (e, m, None, None)
    } else if args.after_match.is_some() || args.before_match.is_some() || args.between.is_some() {
        let max_size = global.effective_max_filesize();
        let (e, m) = edit_by_marker(&original, &lines, args, stdin, max_size, stdin_is_tty)?;
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
        backup: resolved_backup.backup,
        syntax_check: false,
        retention: resolved_backup.retention,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: args.wal_policy,
        keep_backup: resolved_backup.keep,
        durability: crate::platform::Durability::Auto,
    };

    let result = atomic_write(&path, edited.as_bytes(), &opts, workspace)?;
    let lines_after = edited.lines().count() as u64;

    let (fuzzy, strategy, strategies_tried, similarity, diff_preview, match_count, indent_adjusted) =
        match fuzzy_info {
            Some(fi) => (
                Some(fi.fuzzy),
                Some(fi.strategy),
                Some(fi.strategies_tried),
                fi.similarity,
                fi.diff_preview,
                Some(fi.match_count),
                Some(fi.indent_adjusted),
            ),
            None => (None, None, None, None, None, None, None),
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
        diff_preview,
        pairs_total,
        pair_results,
        mtime_preserved: Some(args.preserve_timestamps),
        match_count,
        indent_adjusted,
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
    defaults: &crate::config::DefaultsSection,
    stdin_is_tty: bool,
) -> Result<()> {
    if stdin_is_tty {
        return Err(AtomwriteError::InvalidInput {
            reason: "--multi reads content from stdin but stdin is a terminal; \
                     pipe content (cat ops.ndjson | atomwrite edit --multi ...)"
                .into(),
        }
        .into());
    }
    let resolved_backup = resolve_backup(&args.backup_opts, defaults);
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
    let mut exact_ops: Vec<&MultiEdit> =
        ops.iter().filter(|o| o.effective_op() == "exact").collect();
    let mut line_ops: Vec<&MultiEdit> =
        ops.iter().filter(|o| o.effective_op() != "exact").collect();
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
        backup: resolved_backup.backup,
        syntax_check: false,
        retention: resolved_backup.retention,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: false,
        wal_policy: args.wal_policy,
        keep_backup: resolved_backup.keep,
        durability: crate::platform::Durability::Auto,
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
        diff_preview: None,
        pairs_total: None,
        pair_results: None,
        mtime_preserved: Some(args.preserve_timestamps),
        match_count: None,
        indent_adjusted: None,
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
    custom_threshold: Option<f64>,
    replace_all: bool,
) -> Result<(String, String, FuzzyInfo, Option<MultiReport>)> {
    if old.len() > 1 {
        return edit_old_new_multi(
            original,
            old,
            new,
            fuzzy,
            partial,
            custom_threshold,
            replace_all,
        );
    }
    let old_str = &old[0];
    let new_str = new.first().map(|s| s.as_str()).unwrap_or("");
    let opts = crate::fuzzy::MatchOpts {
        mode: fuzzy,
        threshold: custom_threshold,
        replace_all,
    };
    match crate::fuzzy::match_pair_with(original, old_str, new_str, opts) {
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

// Fuzzy cascade lives in `crate::fuzzy` (v0.1.29 P0-1).
/// Re-export for property tests (GAP-086) and historical `commands::edit::match_pair`.
pub use crate::fuzzy::match_pair;

fn edit_old_new_multi(
    original: &str,
    old: &[String],
    new: &[String],
    fuzzy: FuzzyMode,
    partial: bool,
    custom_threshold: Option<f64>,
    replace_all: bool,
) -> Result<(String, String, FuzzyInfo, Option<MultiReport>)> {
    let pairs_total = old.len() as u64;
    let mut content = original.to_string();
    let mut pair_results: Vec<PairResult> = Vec::with_capacity(old.len());
    let mut applied = 0u64;
    let mut any_fuzzy = false;
    let mut max_strategies_tried = 0u64;

    for (i, (old_str, new_str)) in old.iter().zip(new.iter()).enumerate() {
        let index = (i + 1) as u64;
        let opts = crate::fuzzy::MatchOpts {
            mode: fuzzy,
            threshold: custom_threshold,
            replace_all,
        };
        match crate::fuzzy::match_pair_with(&content, old_str, new_str, opts) {
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
                let (reason, best_candidate) = match err {
                    AtomwriteError::MatchFailed {
                        reason,
                        best_candidate,
                        ..
                    } => (reason, best_candidate),
                    AtomwriteError::MatchAmbiguous {
                        reason,
                        best_candidate,
                        ..
                    } => (reason, best_candidate),
                    AtomwriteError::InvalidInput { reason } => (reason, None),
                    other => (other.to_string(), None),
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
                    pair_results: Box::new(pair_results),
                    best_candidate,
                }
                .into());
            }
        }
    }

    if applied == 0 {
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
            diff_preview: None,
            match_count: applied,
            indent_adjusted: false,
        },
        Some(MultiReport {
            pair_results,
            pairs_total,
            applied,
        }),
    ))
}

// ─── line-based and marker-based edits (unchanged) ───────────────────────────

fn edit_by_line(
    lines: &[&str],
    args: &EditArgs,
    stdin: impl Read,
    max_size: u64,
    stdin_is_tty: bool,
) -> Result<(String, String)> {
    let mut result_lines = lines_to_owned(lines);

    if let Some(n) = args.after_line {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "after-line")?;
        let idx = validate_line_num(n, lines.len())?;
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result_lines.insert(idx + i + 1, line);
        }
        return Ok((join_lines(&result_lines), "after_line".into()));
    }

    if let Some(n) = args.before_line {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "before-line")?;
        let idx = validate_line_num(n, lines.len())?;
        let insert_at = if idx == 0 { 0 } else { idx };
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result_lines.insert(insert_at + i, line);
        }
        return Ok((join_lines(&result_lines), "before_line".into()));
    }

    if let Some(ref range_str) = args.range {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "range")?;
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
    stdin_is_tty: bool,
) -> Result<(String, String)> {
    if let Some(ref marker) = args.after_match {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "after-match")?;
        let idx = find_line_with(lines, marker)?;
        let mut result = lines_to_owned(lines);
        let new_lines = lines_from_str(&content);
        for (i, line) in new_lines.into_iter().enumerate() {
            result.insert(idx + 1 + i, line);
        }
        return Ok((join_lines(&result), "after_match".into()));
    }

    if let Some(ref marker) = args.before_match {
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "before-match")?;
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
        let content = read_stdin_text_guarded(stdin, max_size, stdin_is_tty, "between")?;
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

fn lines_from_str(s: &str) -> Vec<String> {
    s.lines().map(String::from).collect()
}

fn lines_to_owned(lines: &[&str]) -> Vec<String> {
    let mut v = Vec::with_capacity(lines.len());
    v.extend(lines.iter().map(|s| String::from(*s)));
    v
}

fn join_lines(lines: &[String]) -> String {
    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}
