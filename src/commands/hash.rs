// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standalone BLAKE3 checksum computation for one or more files.

use std::io::{Read, Write};
use std::time::Instant;

use anyhow::Result;

use crate::checksum;
use crate::cli::{GlobalArgs, HashArgs};
use crate::output::NdjsonWriter;

/// Compute BLAKE3 checksums for files or stdin.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if a target file does not exist.
/// Returns `AtomwriteError::Io` if reading the file or stdin fails.
pub fn cmd_hash(
    args: &HashArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    if args.stdin {
        let mut reader = std::io::BufReader::new(stdin);
        let hash = checksum::hash_reader(&mut reader)?;
        writer.write_event(&serde_json::json!({
            "type": "hash",
            "source": "stdin",
            "algorithm": "blake3",
            "value": hash,
            "elapsed_ms": start.elapsed().as_millis() as u64,
        }))?;
        return Ok(());
    }

    for path in &args.paths {
        let path = crate::path_safety::validate_path(path, &workspace)?;

        if path.is_dir() && !args.recursive {
            continue;
        }

        if path.is_file() {
            let hash = checksum::hash_file(&path)?;
            let bytes = std::fs::metadata(&path)?.len();

            if let Some(ref expected) = args.verify {
                let verified = &hash == expected;
                writer.write_event(&serde_json::json!({
                    "type": "hash",
                    "path": path.display().to_string(),
                    "algorithm": "blake3",
                    "value": hash,
                    "bytes": bytes,
                    "verified": verified,
                    "elapsed_ms": start.elapsed().as_millis() as u64,
                }))?;
                if !verified {
                    std::process::exit(crate::constants::EXIT_CHECKSUM_VERIFY_FAILED);
                }
            } else {
                writer.write_event(&serde_json::json!({
                    "type": "hash",
                    "path": path.display().to_string(),
                    "algorithm": "blake3",
                    "value": hash,
                    "bytes": bytes,
                    "elapsed_ms": start.elapsed().as_millis() as u64,
                }))?;
            }
        }
    }

    Ok(())
}
