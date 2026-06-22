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

    for path in &args.paths {
        let path = crate::path_safety::validate_path(path, &workspace)?;

        if !path.exists() {
            return Err(crate::error::AtomwriteError::NotFound { path }.into());
        }

        if path.is_dir() && !args.recursive {
            continue;
        }

        if path.is_dir() && args.recursive {
            let walker = ignore::WalkBuilder::new(&path)
                .hidden(false)
                .git_ignore(true)
                .build();
            for entry in walker.flatten() {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    file_paths.push(entry.into_path());
                }
            }
        } else if path.is_file() {
            file_paths.push(path);
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
