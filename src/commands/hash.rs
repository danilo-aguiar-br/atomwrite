// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standalone BLAKE3 checksum computation for one or more files.
//! Workload: CPU-bound (BLAKE3 hashing).

use std::io::{Read, Write};
use std::time::Instant;

use anyhow::Result;

use crate::checksum;
use crate::cli::{GlobalArgs, HashArgs};
use crate::ndjson_types::HashOutput;
use crate::output::NdjsonWriter;

/// Compute BLAKE3 checksums for files or stdin.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if a target file does not exist.
/// Returns `AtomwriteError::Io` if reading the file or stdin fails.
#[tracing::instrument(skip_all, fields(command = "hash"))]
pub fn cmd_hash(
    args: &HashArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    if args.stdin {
        let mut reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
        let hash = checksum::hash_reader(&mut reader)?;
        writer.write_event(&HashOutput {
            r#type: "hash",
            path: None,
            source: Some("stdin"),
            algorithm: "blake3",
            value: hash,
            bytes: None,
            verified: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?;
        return Ok(());
    }

    let mut file_paths: Vec<std::path::PathBuf> = Vec::new();
    // Recipe and agents pass exclude; recursive walks always skip `*.bak.*`.
    let excludes = if args.exclude.is_empty() && args.recursive {
        vec!["*.bak.*".into(), "**/*.bak.*".into()]
    } else {
        args.exclude.clone()
    };
    let skip_bak_in_walk = args.recursive
        || excludes.iter().any(|e| e.contains("bak"));

    for path in &args.paths {
        let path = crate::path_safety::validate_path(path, &workspace)?;

        if !path.exists() {
            return Err(crate::error::AtomwriteError::NotFound { path }.into());
        }

        if path.is_dir() && !args.recursive {
            continue;
        }

        if path.is_dir() && args.recursive {
            let mut builder = ignore::WalkBuilder::new(&path);
            builder.hidden(false);
            builder.git_ignore(true);
            if skip_bak_in_walk {
                builder.filter_entry(|entry| !path_looks_like_bak(entry.path()));
            }
            for entry in builder.build().flatten() {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    let p = entry.into_path();
                    if !is_excluded_path(&p, &excludes) {
                        file_paths.push(p);
                    }
                }
            }
        } else if path.is_file() {
            // Explicit single-file paths are hashed even if they look like backups,
            // unless an exclude pattern matches.
            if excludes.is_empty() || !is_excluded_path(&path, &excludes) {
                file_paths.push(path);
            }
        }
    }

    file_paths.sort();

    for path in &file_paths {
        let path_str = path.display().to_string();
        let hash = checksum::hash_file(path, global.effective_max_filesize())?;
        let bytes = std::fs::metadata(path)?.len();

        if let Some(ref expected) = args.verify {
            let verified = &hash == expected;
            writer.write_event(&HashOutput {
                r#type: "hash",
                path: Some(path_str.clone()),
                source: None,
                algorithm: "blake3",
                value: hash,
                bytes: Some(bytes),
                verified: Some(verified),
                elapsed_ms: start.elapsed().as_millis() as u64,
            })?;
            if !verified {
                return Err(crate::error::AtomwriteError::ChecksumVerifyFailed {
                    path: path.clone(),
                    expected: expected.clone(),
                }
                .into());
            }
        } else {
            writer.write_event(&HashOutput {
                r#type: "hash",
                path: Some(path_str),
                source: None,
                algorithm: "blake3",
                value: hash,
                bytes: Some(bytes),
                verified: None,
                elapsed_ms: start.elapsed().as_millis() as u64,
            })?;
        }
    }

    Ok(())
}

fn path_looks_like_bak(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.contains(".bak."))
}

fn is_excluded_path(path: &std::path::Path, excludes: &[String]) -> bool {
    if excludes.is_empty() {
        return false;
    }
    let s = path.to_string_lossy();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    for pat in excludes {
        if pat == "*.bak.*" || pat == "**/*.bak.*" {
            if name.contains(".bak.") || s.contains(".bak.") {
                return true;
            }
            continue;
        }
        if let Some(stripped) = pat.strip_prefix("**/") {
            if s.contains(stripped.trim_start_matches('*')) {
                return true;
            }
        }
        if let Some(stripped) = pat.strip_prefix('*') {
            if name.ends_with(stripped) || s.ends_with(stripped) {
                return true;
            }
        }
        if s.contains(pat) || name == pat {
            return true;
        }
    }
    false
}
