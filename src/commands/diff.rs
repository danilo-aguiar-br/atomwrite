// SPDX-License-Identifier: MIT OR Apache-2.0

//! File comparison with unified, stat, or changes-only output.

use std::io::Write;
use std::time::Instant;

use anyhow::Result;
use similar::{Algorithm, TextDiff};

use crate::cli::{DiffAlgorithm, DiffArgs, GlobalArgs};
use crate::output::NdjsonWriter;

/// Compare two files and emit a unified diff as NDJSON.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if either file does not exist.
/// Returns `AtomwriteError::Io` if reading the files fails.
pub fn cmd_diff(
    args: &DiffArgs,
    _global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();

    let content_a = crate::file_io::read_file_string(&args.file_a)?;
    let content_b = crate::file_io::read_file_string(&args.file_b)?;

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
    let ratio = diff.ratio();

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

        writer.write_event(&serde_json::json!({
            "type": "diff",
            "identical": identical,
            "file_a": args.file_a.display().to_string(),
            "file_b": args.file_b.display().to_string(),
            "insertions": insertions,
            "deletions": deletions,
            "similarity_ratio": ratio,
            "elapsed_ms": start.elapsed().as_millis() as u64,
        }))?;
    } else if args.unified {
        let unified = diff
            .unified_diff()
            .context_radius(args.context)
            .header(
                &args.file_a.display().to_string(),
                &args.file_b.display().to_string(),
            )
            .to_string();

        writer.write_event(&serde_json::json!({
            "type": "diff",
            "identical": identical,
            "format": "unified",
            "content": unified,
            "similarity_ratio": ratio,
            "elapsed_ms": start.elapsed().as_millis() as u64,
        }))?;
    } else {
        for change in diff.iter_all_changes() {
            let tag = match change.tag() {
                similar::ChangeTag::Insert => "insert",
                similar::ChangeTag::Delete => "delete",
                similar::ChangeTag::Equal => continue,
            };
            writer.write_event(&serde_json::json!({
                "type": "change",
                "tag": tag,
                "line": change.old_index().or(change.new_index()).unwrap_or(0),
                "text": change.value().trim_end_matches('\n'),
            }))?;
        }

        writer.write_event(&serde_json::json!({
            "type": "summary",
            "identical": identical,
            "file_a": args.file_a.display().to_string(),
            "file_b": args.file_b.display().to_string(),
            "lines_a": content_a.lines().count(),
            "lines_b": content_b.lines().count(),
            "similarity_ratio": ratio,
            "elapsed_ms": start.elapsed().as_millis() as u64,
        }))?;
    }

    Ok(())
}
