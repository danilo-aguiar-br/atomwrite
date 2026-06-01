// SPDX-License-Identifier: MIT OR Apache-2.0

//! Patch application from stdin: unified diff, SEARCH/REPLACE, full file.
//! Workload: I/O-bound (stdin read + patch parse + atomic write).

use std::io::{Read, Write};
use std::time::Instant;

use anyhow::Result;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{ApplyArgs, GlobalArgs, PatchFormat};
use crate::error::AtomwriteError;
use crate::ndjson_types::ApplyResult;
use crate::output::NdjsonWriter;

/// Apply a patch from stdin to a target file.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the target file does not exist.
/// Returns `AtomwriteError::InvalidInput` if the patch format is invalid.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns an I/O error if the write fails.
#[tracing::instrument(skip_all, fields(command = "apply"))]
pub fn cmd_apply(
    args: &ApplyArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let target = crate::path_safety::validate_path(&args.file, &workspace)?;

    let max_stdin = global.effective_max_filesize();
    let mut patch = String::new();
    stdin
        .take(max_stdin)
        .read_to_string(&mut patch)
        .map_err(|e| AtomwriteError::Io { source: e })?;

    let format = match args.format {
        PatchFormat::Auto => detect_format(&patch),
        other => other,
    };

    let original = if target.exists() {
        crate::file_io::read_file_string(&target, global.effective_max_filesize())?
    } else if matches!(format, PatchFormat::Full) {
        String::new()
    } else {
        return Err(AtomwriteError::NotFound {
            path: target.clone(),
        }
        .into());
    };

    let checksum_before = checksum::hash_bytes(original.as_bytes());

    let (result_content, hunks) = match format {
        PatchFormat::Unified => apply_unified(&original, &patch)?,
        PatchFormat::SearchReplace => apply_search_replace(&original, &patch)?,
        PatchFormat::Full => (patch.clone(), 1),
        PatchFormat::Markdown => {
            let stripped = strip_markdown_fences(&patch);
            apply_unified(&original, &stripped)?
        }
        PatchFormat::Auto => unreachable!("Auto resolved to concrete format before dispatch"),
    };

    let checksum_after = checksum::hash_bytes(result_content.as_bytes());
    let format_name = match format {
        PatchFormat::Unified => "unified",
        PatchFormat::SearchReplace => "search-replace",
        PatchFormat::Full => "full",
        PatchFormat::Markdown => "markdown",
        PatchFormat::Auto => "auto",
    };

    if args.dry_run {
        writer.write_event(&crate::ndjson_types::ApplyPlan {
            r#type: "plan",
            operation: "apply",
            path: target.display().to_string(),
            format_detected: format_name.to_owned(),
            hunks,
        })?;
        return Ok(());
    }

    let opts = AtomicWriteOptions {
        backup: args.backup,
        ..Default::default()
    };
    atomic_write(&target, result_content.as_bytes(), &opts, &workspace)?;

    writer.write_event(&ApplyResult {
        r#type: "applied",
        path: target.display().to_string(),
        format_detected: format_name.to_owned(),
        hunks_applied: hunks as u64,
        bytes_before: original.len() as u64,
        bytes_after: result_content.len() as u64,
        checksum_before,
        checksum_after,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;

    Ok(())
}

fn detect_format(patch: &str) -> PatchFormat {
    let head: String = {
        let mut buf = String::with_capacity(256);
        for (i, line) in patch.lines().take(15).enumerate() {
            if i > 0 {
                buf.push('\n');
            }
            buf.push_str(line);
        }
        buf
    };
    if head.contains("--- ") && head.contains("+++ ") && head.contains("@@ ") {
        PatchFormat::Unified
    } else if head.contains("<<<<<<< SEARCH") {
        PatchFormat::SearchReplace
    } else if head.contains("```diff") {
        PatchFormat::Markdown
    } else {
        PatchFormat::Full
    }
}

fn apply_unified(original: &str, patch: &str) -> Result<(String, usize)> {
    let mut lines: Vec<&str> = original.lines().collect();
    let mut hunks_applied = 0usize;
    let mut offset: i64 = 0;

    for hunk in parse_unified_hunks(patch) {
        let start_line = ((hunk.old_start as i64) + offset - 1).max(0) as usize;

        let mut new_lines = Vec::with_capacity(hunk.ops.len());
        let mut old_consumed = 0usize;

        for op in &hunk.ops {
            match op {
                HunkOp::Context(text) => {
                    new_lines.push(*text);
                    old_consumed += 1;
                }
                HunkOp::Delete(()) => {
                    old_consumed += 1;
                }
                HunkOp::Insert(text) => {
                    new_lines.push(*text);
                }
            }
        }

        let end = (start_line + old_consumed).min(lines.len());
        lines.splice(start_line..end, new_lines);
        offset += hunk.new_count as i64 - hunk.old_count as i64;
        hunks_applied += 1;
    }

    let mut result = lines.join("\n");
    if original.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    Ok((result, hunks_applied))
}

struct Hunk<'a> {
    old_start: usize,
    old_count: usize,
    new_count: usize,
    ops: Vec<HunkOp<'a>>,
}

enum HunkOp<'a> {
    Context(&'a str),
    Delete(()),
    Insert(&'a str),
}

fn parse_unified_hunks(patch: &str) -> Vec<Hunk<'_>> {
    let mut hunks = Vec::with_capacity(4);
    let mut current: Option<Hunk<'_>> = None;
    let lines: Vec<&str> = patch.lines().collect();

    for line in &lines {
        if let Some(header) = line.strip_prefix("@@ ") {
            if let Some(h) = current.take() {
                hunks.push(h);
            }
            if let Some((old_start, old_count, new_count)) = parse_hunk_header(header) {
                current = Some(Hunk {
                    old_start,
                    old_count,
                    new_count,
                    ops: Vec::with_capacity(8),
                });
            }
        } else if let Some(ref mut h) = current {
            if let Some(text) = line.strip_prefix('+') {
                h.ops.push(HunkOp::Insert(text));
            } else if line.starts_with('-') {
                h.ops.push(HunkOp::Delete(()));
            } else if let Some(text) = line.strip_prefix(' ') {
                h.ops.push(HunkOp::Context(text));
            } else if !line.starts_with('\\')
                && !line.starts_with("---")
                && !line.starts_with("+++")
            {
                h.ops.push(HunkOp::Context(line));
            }
        }
    }

    if let Some(h) = current {
        hunks.push(h);
    }

    hunks
}

fn parse_hunk_header(header: &str) -> Option<(usize, usize, usize)> {
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    let old_part = parts[0].trim_start_matches('-');
    let new_part = parts[1].trim_start_matches('+');

    let (old_start, old_count) = parse_range(old_part)?;
    let (_new_start, new_count) = parse_range(new_part)?;

    Some((old_start, old_count, new_count))
}

fn parse_range(range: &str) -> Option<(usize, usize)> {
    if let Some((start, count)) = range.split_once(',') {
        Some((start.parse().ok()?, count.parse().ok()?))
    } else {
        let start: usize = range.parse().ok()?;
        Some((start, 1))
    }
}

fn apply_search_replace(original: &str, patch: &str) -> Result<(String, usize)> {
    let mut result = original.to_owned();
    let mut blocks = 0usize;

    let mut i = 0;
    let lines: Vec<&str> = patch.lines().collect();

    while i < lines.len() {
        if lines[i].starts_with("<<<<<<< SEARCH") {
            let search_start = i + 1;
            let mut sep = search_start;
            while sep < lines.len() && lines[sep] != "=======" {
                sep += 1;
            }
            if sep >= lines.len() {
                break;
            }
            let replace_start = sep + 1;
            let mut replace_end = replace_start;
            while replace_end < lines.len() && !lines[replace_end].starts_with(">>>>>>> REPLACE") {
                replace_end += 1;
            }

            let search_text = lines[search_start..sep].join("\n");
            let replace_text = lines[replace_start..replace_end].join("\n");

            if let Some(pos) = result.find(&search_text) {
                result.replace_range(pos..pos + search_text.len(), &replace_text);
                blocks += 1;
            }

            i = if replace_end < lines.len() {
                replace_end + 1
            } else {
                lines.len()
            };
        } else {
            i += 1;
        }
    }

    if blocks == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "no SEARCH/REPLACE blocks found in patch".into(),
        }
        .into());
    }

    Ok((result, blocks))
}

fn strip_markdown_fences(patch: &str) -> String {
    let mut in_block = false;
    let mut result = Vec::with_capacity(patch.lines().count());

    for line in patch.lines() {
        if line.starts_with("```diff") {
            in_block = true;
            continue;
        }
        if in_block && line.starts_with("```") {
            in_block = false;
            continue;
        }
        if in_block {
            result.push(line);
        }
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_format_unified() {
        let patch = "--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,3 @@\n";
        assert!(matches!(detect_format(patch), PatchFormat::Unified));
    }

    #[test]
    fn detect_format_search_replace() {
        let patch = "<<<<<<< SEARCH\nold text\n=======\nnew text\n>>>>>>> REPLACE\n";
        assert!(matches!(detect_format(patch), PatchFormat::SearchReplace));
    }

    #[test]
    fn detect_format_markdown() {
        let patch = "```diff\n--- a/file.txt\n+++ b/file.txt\n```\n";
        assert!(matches!(detect_format(patch), PatchFormat::Markdown));
    }

    #[test]
    fn detect_format_full() {
        let patch = "this is just plain text content\nno markers at all\n";
        assert!(matches!(detect_format(patch), PatchFormat::Full));
    }

    #[test]
    fn strip_markdown_fences_extracts_content() {
        let input = "some preamble\n```diff\n--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new\n```\nafter";
        let result = strip_markdown_fences(input);
        assert!(result.contains("--- a"));
        assert!(!result.contains("```"));
    }

    #[test]
    fn apply_search_replace_basic() {
        let original = "hello world\nfoo bar\n";
        let patch = "<<<<<<< SEARCH\nhello world\n=======\nhello universe\n>>>>>>> REPLACE\n";
        let (result, blocks) = apply_search_replace(original, patch).unwrap();
        assert_eq!(blocks, 1);
        assert!(result.contains("hello universe"));
    }

    #[test]
    fn apply_search_replace_no_blocks_fails() {
        let original = "hello world\n";
        let patch = "no search replace blocks here\n";
        let result = apply_search_replace(original, patch);
        assert!(result.is_err());
    }

    #[test]
    fn apply_unified_simple() {
        let original = "line1\nline2\nline3\n";
        let patch = "--- a/file\n+++ b/file\n@@ -1,3 +1,3 @@\n line1\n-line2\n+modified\n line3\n";
        let (result, hunks) = apply_unified(original, patch).unwrap();
        assert_eq!(hunks, 1);
        assert!(result.contains("modified"));
        assert!(!result.contains("line2"));
    }
}
