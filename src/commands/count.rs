// SPDX-License-Identifier: MIT OR Apache-2.0

//! File and line counting with optional grouping by extension.
//! Workload: I/O-bound (parallel file walk + line counting).

use std::collections::BTreeMap;
use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::cli::{CountArgs, GlobalArgs};
use crate::ndjson_types::{CountByExtOutput, CountTotalOutput, CountTotals, ExtCountOutput};
use crate::output::NdjsonWriter;

/// Count lines, pattern matches, or files grouped by extension.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if reading files fails.
#[tracing::instrument(skip_all, fields(command = "count"))]
pub fn cmd_count(
    args: &CountArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let mut walker = ignore::WalkBuilder::new(&args.paths[0]);
    for p in args.paths.iter().skip(1) {
        walker.add(p);
    }
    walker
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore);

    if !args.include.is_empty() {
        let mut types_builder = ignore::types::TypesBuilder::new();
        for pat in &args.include {
            types_builder
                .add_def(&format!("custom:{pat}"))
                .context("invalid include glob")?;
        }
        types_builder.select("custom");
        walker.types(types_builder.build().context("build types")?);
    }
    if !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&args.paths[0]);
        for pat in &args.exclude {
            overrides.add(&format!("!{pat}"))?;
        }
        walker.overrides(overrides.build()?);
    }

    let mut total_files = 0u64;
    let mut total_lines = 0u64;
    let mut total_blank = 0u64;
    let mut total_bytes = 0u64;
    let mut by_ext: BTreeMap<String, ExtCountOutput> = BTreeMap::new();

    for entry in walker.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.path();

        if let Ok(validated) = crate::path_safety::validate_path(path, &workspace) {
            let meta = match std::fs::metadata(&validated) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size = meta.len();
            total_bytes += size;
            total_files += 1;

            let ext = validated
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("(none)")
                .to_owned();

            let entry_count = by_ext.entry(ext).or_default();
            entry_count.files += 1;
            entry_count.bytes += size;

            if let Ok(content) =
                crate::file_io::read_file_string(&validated, global.effective_max_filesize())
            {
                let lines = content.lines().count() as u64;
                let blank = content.lines().filter(|l| l.trim().is_empty()).count() as u64;
                total_lines += lines;
                total_blank += blank;
                entry_count.lines += lines;
                entry_count.blank += blank;
            }
        }
    }

    if args.by_extension {
        writer.write_event(&CountByExtOutput {
            r#type: "count",
            mode: "by_extension",
            by_extension: by_ext,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
    } else {
        writer.write_event(&CountTotalOutput {
            r#type: "count",
            mode: "lines",
            total: CountTotals {
                files: total_files,
                lines: total_lines,
                blank: total_blank,
                bytes: total_bytes,
            },
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
    }

    Ok(())
}
