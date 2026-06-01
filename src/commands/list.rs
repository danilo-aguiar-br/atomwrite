// SPDX-License-Identifier: MIT OR Apache-2.0

//! Directory listing with metadata, gitignore support, and depth control.
//! Workload: I/O-bound (directory walk + stat per entry).

use std::collections::BTreeMap;
use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result};
use ignore::WalkBuilder;

use crate::cli::{GlobalArgs, ListArgs};
use crate::ndjson_types::{ListEntry, ListSummary};
use crate::output::NdjsonWriter;

/// List project file structure with optional metadata as NDJSON.
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if traversing the directory fails.
#[tracing::instrument(skip_all, fields(command = "list"))]
pub fn cmd_list(
    args: &ListArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    let root = if args.paths.is_empty() {
        workspace.clone()
    } else {
        crate::path_safety::validate_path(&args.paths[0], &workspace)?
    };

    let mut builder = WalkBuilder::new(&root);
    builder
        .hidden(!args.all)
        .git_ignore(!global.no_gitignore)
        .sort_by_file_path(|a, b| a.cmp(b));

    if let Some(depth) = args.depth {
        builder.max_depth(Some(depth));
    }

    if !args.include.is_empty() {
        let mut types_builder = ignore::types::TypesBuilder::new();
        for pattern in &args.include {
            types_builder
                .add_def(&format!("custom:{pattern}"))
                .context("invalid include glob")?;
        }
        types_builder.select("custom");
        builder.types(types_builder.build().context("build types")?);
    }

    if !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&root);
        for pattern in &args.exclude {
            overrides.add(&format!("!{pattern}"))?;
        }
        builder.overrides(overrides.build()?);
    }

    let mut files: u64 = 0;
    let mut dirs: u64 = 0;
    let mut symlinks: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut by_ext: BTreeMap<String, u64> = BTreeMap::new();

    for entry in builder.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, "walk error");
                continue;
            }
        };

        let path = entry.path();
        let rel_path = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();

        if rel_path.is_empty() {
            continue;
        }

        let ft = entry.file_type();
        let kind = if ft.is_some_and(|t| t.is_dir()) {
            dirs += 1;
            "dir"
        } else if ft.is_some_and(|t| t.is_symlink()) {
            symlinks += 1;
            "symlink"
        } else {
            files += 1;
            "file"
        };

        let (size, modified) = if args.long {
            match entry.metadata() {
                Ok(meta) => {
                    let sz = meta.len();
                    total_bytes += sz;
                    let mod_str = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| format!("{}Z", d.as_secs()));
                    (Some(sz), mod_str)
                }
                Err(_) => (None, None),
            }
        } else {
            if let Ok(meta) = entry.metadata() {
                total_bytes += meta.len();
            }
            (None, None)
        };

        if args.count_by_ext {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                *by_ext.entry(ext.to_owned()).or_default() += 1;
            }
        }

        let output = ListEntry {
            r#type: "entry",
            path: rel_path,
            kind: kind.into(),
            size,
            modified,
        };
        writer.write_event(&output)?;
    }

    let summary = ListSummary {
        r#type: "summary",
        files,
        dirs,
        symlinks,
        total_bytes: Some(total_bytes),
        by_extension: if args.count_by_ext {
            Some(by_ext)
        } else {
            None
        },
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    writer.write_event(&summary)?;

    Ok(())
}
