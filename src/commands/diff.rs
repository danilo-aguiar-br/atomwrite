// SPDX-License-Identifier: MIT OR Apache-2.0

//! File comparison with unified, stat, or changes-only output.
//! Workload: CPU-bound (text diff algorithm + I/O).

use std::io::Write;
use std::time::Instant;

use anyhow::Result;
use similar::{Algorithm, TextDiff};

use crate::cli::{DiffAlgorithm, DiffArgs, GlobalArgs};
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
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();

    let max_size = global.effective_max_filesize();
    let workspace = global.resolve_workspace()?;
    let resolved_a = crate::path_safety::validate_path(&args.file_a, &workspace)?;
    let resolved_b = crate::path_safety::validate_path(&args.file_b, &workspace)?;
    let content_a = crate::file_io::read_file_string(&resolved_a, max_size)?;
    let content_b = crate::file_io::read_file_string(&resolved_b, max_size)?;

    let algo = match args.algorithm {
        DiffAlgorithm::Myers => Algorithm::Myers,
        DiffAlgorithm::Patience => Algorithm::Patience,
        DiffAlgorithm::Lcs => Algorithm::Lcs,
    };

    let diff = TextDiff::configure()
        .algorithm(algo)
        .timeout(std::time::Duration::from_millis(500))
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
            writer.write_event(&DiffChangeOutput {
                r#type: "change",
                tag,
                line: change.old_index().or(change.new_index()).unwrap_or(0),
                text: change.value().trim_end_matches('\n'),
            })?;
        }

        writer.write_event(&DiffSummaryOutput {
            r#type: "summary",
            identical,
            file_a: path_a,
            file_b: path_b,
            lines_a: content_a.lines().count(),
            lines_b: content_b.lines().count(),
            similarity_ratio: ratio,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
    }

    Ok(())
}

fn safe_ratio(ratio: f32) -> f32 {
    if ratio.is_finite() { ratio } else { 0.0 }
}
