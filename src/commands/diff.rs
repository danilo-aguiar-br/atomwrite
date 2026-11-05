// SPDX-License-Identifier: MIT OR Apache-2.0

//! File comparison with unified, stat, or changes-only output.
//! Workload: CPU-bound (text diff algorithm + I/O).
//! Parallelism: dual-file reads via `rayon::join` (two independent I/O paths);
//! diff algorithm itself is single-threaded (similar crate). Bound: global
//! rayon pool (`--threads` / `--max-concurrency`).

use std::io::{Read, Write};
use std::time::Instant;

use anyhow::Result;
use similar::{Algorithm, TextDiff};

use crate::cli::{DiffAlgorithm, DiffArgs, GlobalArgs};
use crate::error::AtomwriteError;
use crate::ndjson_types::{DiffChangeOutput, DiffStatOutput, DiffSummaryOutput, DiffUnifiedOutput};
use crate::output::NdjsonWriter;

/// Compare two files and emit a unified diff as NDJSON.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if either file does not exist.
/// Returns `AtomwriteError::Io` if reading the files fails.
#[tracing::instrument(skip_all, fields(command = "diff"))]
pub fn cmd_diff(
    args: &DiffArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();

    let max_size = global.effective_max_filesize();
    let workspace = global.resolve_workspace()?;
    // G-003 / B-001: either FILE_A or FILE_B may be "-" (stdin). Not both.
    // Must use the already-locked `stdin` from `run()` — never `std::io::stdin()`
    // again or we deadlock on the outer lock (same bug class as edit-loop).
    let stdin_a = args.file_a.as_os_str() == "-";
    let stdin_b = args.file_b.as_os_str() == "-";
    if stdin_a && stdin_b {
        return Err(AtomwriteError::InvalidInput {
            reason:
                "diff: only one of FILE_A or FILE_B may be '-' (stdin); both sides cannot be stdin"
                    .into(),
        }
        .into());
    }

    let content_a;
    let content_b;
    if stdin_a {
        let resolved_b = crate::path_safety::validate_path(&args.file_b, &workspace)?;
        // The shared reader owns the chunk size, the cap and the cancellation
        // token, so `diff` cannot drift into its own `take` + `read_to_string`.
        content_a = read_diff_stdin(stdin, max_size)?;
        content_b = crate::file_io::read_file_string(&resolved_b, max_size)?;
    } else if stdin_b {
        let resolved_a = crate::path_safety::validate_path(&args.file_a, &workspace)?;
        content_a = crate::file_io::read_file_string(&resolved_a, max_size)?;
        content_b = read_diff_stdin(stdin, max_size)?;
    } else {
        let resolved_a = crate::path_safety::validate_path(&args.file_a, &workspace)?;
        let resolved_b = crate::path_safety::validate_path(&args.file_b, &workspace)?;
        // Independent reads — fan out on the process-wide rayon pool.
        let (a, b) = rayon::join(
            || crate::file_io::read_file_string(&resolved_a, max_size),
            || crate::file_io::read_file_string(&resolved_b, max_size),
        );
        content_a = a?;
        content_b = b?;
    }

    let algo = match args.algorithm {
        DiffAlgorithm::Myers => Algorithm::Myers,
        DiffAlgorithm::Patience => Algorithm::Patience,
        DiffAlgorithm::Lcs => Algorithm::Lcs,
    };

    let diff = TextDiff::configure()
        .algorithm(algo)
        .timeout(std::time::Duration::from_millis(
            crate::constants::DIFF_SIMILARITY_TIMEOUT_MS,
        ))
        .diff_lines(&content_a, &content_b);

    let identical = content_a == content_b;
    let ratio = safe_ratio(diff.ratio());

    let path_a = args.file_a.display().to_string();
    let path_b = args.file_b.display().to_string();

    if args.stat {
        let mut insertions = 0u64;
        let mut deletions = 0u64;
        for change in diff.iter_all_changes() {
            match change.tag() {
                similar::ChangeTag::Insert => insertions += 1,
                similar::ChangeTag::Delete => deletions += 1,
                similar::ChangeTag::Equal => {}
            }
        }

        writer.write_event(&DiffStatOutput {
            r#type: "diff",
            identical,
            file_a: path_a,
            file_b: path_b,
            insertions,
            deletions,
            similarity_ratio: ratio,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
    } else if args.unified {
        let unified = diff
            .unified_diff()
            .context_radius(args.context)
            .header(&path_a, &path_b)
            .to_string();

        writer.write_event(&DiffUnifiedOutput {
            r#type: "diff",
            identical,
            format: "unified",
            content: unified,
            similarity_ratio: ratio,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
    } else {
        for change in diff.iter_all_changes() {
            let tag = match change.tag() {
                similar::ChangeTag::Insert => "insert",
                similar::ChangeTag::Delete => "delete",
                similar::ChangeTag::Equal => continue,
            };
            // `similar` indexes from zero, so publishing the index directly
            // named the line above the change: the caller then edited the
            // neighbour and blamed the content. Each side keeps its own number,
            // because an insert has no old line and a delete has no new one.
            let old_line = change.old_index().map(|index| index + 1);
            let new_line = change.new_index().map(|index| index + 1);
            writer.write_event(&DiffChangeOutput {
                r#type: "change",
                tag,
                line: old_line.or(new_line).unwrap_or(1),
                old_line,
                new_line,
                text: change.value().trim_end_matches('\n'),
            })?;
        }

        // The engine never says whether it aborted, so the signal is the
        // budget itself: a run that spent the whole deadline may have come
        // back with a coarser diff, and the consumer reads this to know the
        // ratio is a floor.
        let timed_out = start.elapsed()
            >= std::time::Duration::from_millis(crate::constants::DIFF_SIMILARITY_TIMEOUT_MS);
        writer.write_terminal(&DiffSummaryOutput {
            r#type: "summary",
            identical,
            file_a: path_a,
            file_b: path_b,
            lines_a: content_a.lines().count(),
            lines_b: content_b.lines().count(),
            similarity_ratio: ratio,
            elapsed_ms: start.elapsed().as_millis() as u64,
            timed_out: timed_out.then_some(true),
        })?;
    }

    Ok(())
}

/// Read one `-` side of a diff from stdin through the shared reader.
///
/// # Errors
///
/// Returns `AtomwriteError::FileTooLarge` when the payload passes `max_size`,
/// the cancellation error when the shutdown signal fires, and
/// `AtomwriteError::InvalidInput` when the bytes are not UTF-8.
fn read_diff_stdin(stdin: impl Read, max_size: u64) -> Result<String> {
    let (bytes, _) = crate::commands::read_stdin_content_cancellable(
        stdin,
        Some(max_size),
        crate::commands::StdinCapOrigin::GlobalMaxFilesize,
        true,
        None,
    )?;
    String::from_utf8(bytes).map_err(|e| {
        AtomwriteError::InvalidInput {
            reason: format!("diff: stdin is not valid UTF-8: {e}"),
        }
        .into()
    })
}

fn safe_ratio(ratio: f32) -> f32 {
    if ratio.is_finite() { ratio } else { 0.0 }
}
