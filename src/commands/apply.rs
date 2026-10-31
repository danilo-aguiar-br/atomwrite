// SPDX-License-Identifier: MIT OR Apache-2.0

//! Patch application from stdin: unified diff, SEARCH/REPLACE, full file.
//! Workload: I/O-bound (stdin read + patch parse + atomic write).
//! Parallelism: none — single-file / sequential-by-contract (coordination cost ≥ work).

use std::io::{Read, Write};
use std::time::Instant;

use anyhow::Result;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{ApplyArgs, GlobalArgs, PatchFormat};
use crate::commands::resolve_backup;
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
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let target = crate::path_safety::validate_path(&args.file, &workspace)?;
    let resolved_backup = resolve_backup(&args.backup_opts, defaults);

    let max_stdin = global.effective_max_filesize();
    // The shared reader owns the chunk size, the cap, the reserve and the
    // cancellation token; a second `take` + `read_to_string` here would drift.
    let (patch_bytes, _) = crate::commands::read_stdin_content_cancellable(
        stdin,
        Some(max_stdin),
        crate::commands::StdinCapOrigin::GlobalMaxFilesize,
        true,
        None,
    )?;
    let patch = String::from_utf8(patch_bytes).map_err(|e| AtomwriteError::InvalidInput {
        reason: format!("apply: patch on stdin is not valid UTF-8: {e}"),
    })?;

    let format = match args.format {
        PatchFormat::Auto => detect_format(&patch).ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "apply: patch carries no recognizable marker (no ```diff fence, no ---/+++ \
                     header, no <<<<<<< SEARCH block); refusing to guess. Pass --format full to \
                     replace the whole file, or send a patch with the three markers"
                .to_string(),
        })?,
        other => other,
    };

    // A1: lock BEFORE reading an existing target, hold it to the commit.
    let _update_lock = if target.exists() {
        Some(crate::lock::acquire_for_update(&target)?)
    } else {
        None
    };
    let original = if target.exists() {
        crate::file_io::read_file_string(&target, global.effective_max_filesize())?
    } else if matches!(format, PatchFormat::Full) {
        String::new()
    } else {
        return Err(AtomwriteError::NotFound { path: target }.into());
    };

    let checksum_before = checksum::hash_bytes(original.as_bytes());

    let (result_content, hunks) = match format {
        PatchFormat::Unified => apply_unified(&original, &patch)?,
        PatchFormat::SearchReplace => apply_search_replace(&original, &patch, fuzzy_cfg)?,
        PatchFormat::Full => (patch, 1),
        PatchFormat::Markdown => {
            let stripped = strip_markdown_fences(&patch);
            apply_unified(&original, &stripped)?
        }
        PatchFormat::Auto => {
            return Err(AtomwriteError::InternalError {
                reason: "apply: patch format still Auto after detection".to_string(),
            }
            .into());
        }
    };

    let checksum_after = checksum::hash_bytes(result_content.as_bytes());
    let format_name = match format {
        PatchFormat::Unified => "unified",
        PatchFormat::SearchReplace => "search-replace",
        PatchFormat::Full => "full",
        PatchFormat::Markdown => "markdown",
        PatchFormat::Auto => "auto",
    };

    // Same two barriers as `write`: a whole-file replacement is exactly the
    // case they exist for, and a patch that happens to shrink the file must
    // say so out loud too.
    crate::commands::write_guards::check_replacement_size(
        &target,
        original.len() as u64,
        result_content.len() as u64,
        write_cfg,
        args.ack_overwrite,
        args.allow_shrink,
    )?;

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
        backup: resolved_backup.backup,
        syntax_check: false,
        retention: resolved_backup.retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: args.strict.strict_atomic,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup: resolved_backup.keep,
        durability: crate::platform::Durability::Auto,
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

/// Classify a patch that came without an explicit `--format`.
///
/// Returns `None` when no marker identifies the format. The caller turns that
/// `None` into a refusal that names `--format full`: guessing "replace the
/// whole file" from an unrecognized body is what turned a malformed patch into
/// silent data loss, and a patch with no markers is exactly the case where the
/// user may have meant a whole-file write.
fn detect_format(patch: &str) -> Option<PatchFormat> {
    let head: String = {
        let mut buf = String::with_capacity(crate::constants::PATCH_FORMAT_PROBE_CAPACITY);
        for (i, line) in patch
            .lines()
            .take(crate::constants::PATCH_FORMAT_PROBE_LINES)
            .enumerate()
        {
            if i > 0 {
                buf.push('\n');
            }
            buf.push_str(line);
        }
        buf
    };
    // Order matters: a fenced markdown patch also carries `---`/`+++`, so the
    // fence is tested first or every markdown diff would be read as unified.
    if head.contains("```diff") {
        Some(PatchFormat::Markdown)
    } else if head.contains("--- ") && head.contains("+++ ") {
        // A diff header means the author wrote a diff. Without a `@@` hunk
        // there is nothing to apply, and falling through to a whole-file
        // write would overwrite the target with the patch text.
        // Classified as unified so the strict parser rejects it.
        Some(PatchFormat::Unified)
    } else if head.contains("<<<<<<< SEARCH") {
        Some(PatchFormat::SearchReplace)
    } else {
        // No marker at all: refuse, and let the caller name `--format full`.
        None
    }
}

include!("apply_unified.inc.rs");

fn apply_search_replace(
    original: &str,
    patch: &str,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<(String, usize)> {
    let mut result = original.to_owned();
    let mut blocks = 0usize;
    // 1-based numbers of the blocks whose SEARCH text is not in the file, and
    // the near-miss of the first one.
    let mut unmatched: Vec<usize> = Vec::new();
    let mut first_miss: Option<Box<crate::ndjson_types::BestCandidate>> = None;

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

            // Exact first; on miss use fuzzy cascade (G-FZZ-059/073).
            blocks += 1;
            if let Some(pos) = result.find(&search_text) {
                result.replace_range(pos..pos + search_text.len(), &replace_text);
            } else {
                match crate::fuzzy::match_pair_cfg(
                    &result,
                    &search_text,
                    &replace_text,
                    crate::cli_args::FuzzyMode::Auto,
                    None,
                    fuzzy_cfg,
                    false,
                ) {
                    Ok((edited, _info)) => result = edited,
                    // Only "not found" makes a block missing; a cancel, an
                    // oversize refusal or an ambiguous match reaches the caller.
                    Err(AtomwriteError::MatchFailed { best_candidate, .. }) => {
                        unmatched.push(blocks);
                        if first_miss.is_none() {
                            first_miss = best_candidate;
                        }
                    }
                    Err(other) => return Err(other.into()),
                }
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

    if !unmatched.is_empty() {
        return Err(AtomwriteError::MatchFailed {
            reason: format!(
                "{} of {blocks} SEARCH/REPLACE blocks did not match the file (block {:?}); \
                 nothing was written",
                unmatched.len(),
                unmatched
            ),
            best_candidate: first_miss,
            candidates: None,
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
        assert!(matches!(detect_format(patch), Some(PatchFormat::Unified)));
    }

    #[test]
    fn detect_format_search_replace() {
        let patch = "<<<<<<< SEARCH\nold text\n=======\nnew text\n>>>>>>> REPLACE\n";
        assert!(matches!(
            detect_format(patch),
            Some(PatchFormat::SearchReplace)
        ));
    }

    #[test]
    fn detect_format_markdown() {
        let patch = "```diff\n--- a/file.txt\n+++ b/file.txt\n```\n";
        assert!(matches!(detect_format(patch), Some(PatchFormat::Markdown)));
    }

    #[test]
    fn detect_format_refuses_a_body_without_markers() {
        // A marker-less body used to classify as `Full`, which replaced the
        // target with the patch text. It now yields `None`, and the caller
        // refuses unless the user passed `--format full`.
        let patch = "this is just plain text content\nno markers at all\n";
        assert!(detect_format(patch).is_none());
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
        let (result, blocks) =
            apply_search_replace(original, patch, &crate::config::FuzzySection::default()).unwrap();
        assert_eq!(blocks, 1);
        assert!(result.contains("hello universe"));
    }

    #[test]
    fn apply_search_replace_no_blocks_fails() {
        let original = "hello world\n";
        let patch = "no search replace blocks here\n";
        let result = apply_search_replace(original, patch, &crate::config::FuzzySection::default());
        assert!(result.is_err());
    }

    fn mismatch_reason(result: Result<(String, usize)>) -> String {
        match result.unwrap_err().downcast::<AtomwriteError>().unwrap() {
            AtomwriteError::MatchFailed { reason, .. } => reason,
            other => panic!("expected MatchFailed, got {other:?}"),
        }
    }

    #[test]
    fn unified_rejects_wrong_removed_line() {
        let patch = "--- a\n+++ b\n@@ -1 +1 @@\n-zzz\n+yyy\n";
        let reason = mismatch_reason(apply_unified("alpha\nbeta\ngamma\n", patch));
        assert!(reason.contains("hunk 1/1"), "{reason}");
        assert!(reason.contains("zzz"), "{reason}");
        assert!(reason.contains("alpha"), "{reason}");
    }

    #[test]
    fn unified_rejects_wrong_context() {
        let patch = "--- a\n+++ b\n@@ -1,3 +1,3 @@\n qqq\n-beta\n+yyy\n rrr\n";
        let reason = mismatch_reason(apply_unified("alpha\nbeta\ngamma\n", patch));
        assert!(reason.contains("qqq"), "{reason}");
    }

    #[test]
    fn unified_second_hunk_failure_applies_nothing() {
        let patch = "--- a\n+++ b\n@@ -1 +1 @@\n-alpha\n+ALPHA\n@@ -3 +3 @@\n-nope\n+x\n";
        let reason = mismatch_reason(apply_unified("alpha\nbeta\ngamma\n", patch));
        assert!(reason.contains("hunk 2/2"), "{reason}");
    }

    #[test]
    fn unified_tolerates_shifted_header_and_trailing_space() {
        let original = "one\ntwo\nbeta  \nfour\n";
        let patch = "--- a\n+++ b\n@@ -9 +9 @@\n-beta\n+BETA\n";
        let (out, hunks) = apply_unified(original, patch).unwrap();
        assert_eq!(hunks, 1);
        assert_eq!(out, "one\ntwo\nBETA\nfour\n");
    }

    #[test]
    fn unified_rejects_a_hunk_beyond_the_shift_window() {
        let far = crate::constants::PATCH_MAX_HUNK_SHIFT + 5;
        let mut original = String::new();
        for i in 0..far {
            original.push_str(&format!("l{i}\n"));
        }
        original.push_str("target\n");
        let patch = "--- a\n+++ b\n@@ -1 +1 @@\n-target\n+x\n";
        assert!(mismatch_reason(apply_unified(&original, patch)).contains("hunk 1/1"));
    }

    #[test]
    fn unified_insert_only_hunk_goes_after_the_named_line() {
        let patch = "--- a\n+++ b\n@@ -1,0 +2,1 @@\n+inserted\n";
        let (out, _) = apply_unified("alpha\nbeta\n", patch).unwrap();
        assert_eq!(out, "alpha\ninserted\nbeta\n");
    }

    #[test]
    fn unified_without_hunks_is_invalid_input() {
        let err = apply_unified("a\n", "--- a\n+++ b\n").unwrap_err();
        assert!(matches!(
            err.downcast_ref::<AtomwriteError>(),
            Some(AtomwriteError::InvalidInput { .. })
        ));
    }

    #[test]
    fn search_replace_with_a_missing_block_fails_the_whole_patch() {
        let patch = "<<<<<<< SEARCH\nalpha\n=======\nALPHA\n>>>>>>> REPLACE\n\
                     <<<<<<< SEARCH\nCOMPLETELY ABSENT 12345\n=======\nX\n>>>>>>> REPLACE\n";
        let reason = mismatch_reason(apply_search_replace(
            "alpha\nbeta\n",
            patch,
            &crate::config::FuzzySection::default(),
        ));
        assert!(reason.contains("1 of 2"), "{reason}");
    }

    #[test]
    fn search_replace_lets_a_fuzzy_refusal_through() {
        let fuzzy = crate::config::FuzzySection {
            max_content_bytes: 4,
            ..crate::config::FuzzySection::default()
        };
        let patch = "<<<<<<< SEARCH\nnot in the file\n=======\nX\n>>>>>>> REPLACE\n";
        let err = apply_search_replace("alpha beta gamma\n", patch, &fuzzy).unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<AtomwriteError>(),
                Some(AtomwriteError::InvalidInput { .. })
            ),
            "{err:#}"
        );
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
