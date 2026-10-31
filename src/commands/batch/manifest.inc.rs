// SPDX-License-Identifier: MIT OR Apache-2.0
// Manifest reader, included by `run.inc.rs`.

/// Read and parse the NDJSON manifest, from a file or from stdin.
///
/// Every line is parsed before any operation runs, so a malformed manifest
/// refuses the whole batch instead of executing a prefix of it. A line past
/// [`crate::constants::MAX_NDJSON_LINE_SIZE`] is refused by `read_limited_line`
/// rather than truncated into a valid-looking op.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` when the manifest path does not exist,
/// `AtomwriteError::WorkspaceJail` when it escapes the jail,
/// `AtomwriteError::InvalidInput` for a malformed line or an empty manifest,
/// and `AtomwriteError::Io` when the source cannot be read.
fn read_manifest(
    stdin: impl Read,
    manifest_path: Option<&std::path::Path>,
    workspace: &std::path::Path,
) -> Result<Vec<BatchOp>> {
    let mut reader: Box<dyn Read> = if let Some(manifest_path) = manifest_path {
        let validated_manifest = crate::path_safety::validate_path(manifest_path, workspace)
            .with_context(|| {
                format!(
                    "manifest path escapes workspace: {}",
                    manifest_path.display()
                )
            })?;
        if !validated_manifest.is_file() {
            return Err(crate::error::AtomwriteError::NotFound {
                path: validated_manifest,
            }
            .into());
        }
        let file = std::fs::File::open(&validated_manifest)
            .with_context(|| format!("cannot open manifest {}", validated_manifest.display()))?;
        Box::new(file)
    } else {
        Box::new(stdin)
    };

    let mut buf_reader =
        std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, &mut *reader);
    let mut ops: Vec<BatchOp> = Vec::with_capacity(crate::constants::BATCH_OPS_CAPACITY);
    let mut line_buf = String::new();
    let mut idx = 0usize;

    loop {
        let n = crate::output::read_limited_line(
            &mut buf_reader,
            &mut line_buf,
            crate::constants::MAX_NDJSON_LINE_SIZE,
        )
        .with_context(|| format!("failed to read manifest line {}", idx + 1))?;
        if n == 0 {
            break;
        }
        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            idx += 1;
            continue;
        }
        let jd = &mut serde_json::Deserializer::from_str(trimmed);
        let op: BatchOp = serde_path_to_error::deserialize(jd).map_err(|e| {
            if e.inner().classify() == serde_json::error::Category::Io {
                crate::error::AtomwriteError::Io {
                    source: std::io::Error::other(e.to_string()),
                }
            } else {
                crate::error::AtomwriteError::InvalidInput {
                    reason: format!("invalid batch operation at line {}: {e}", idx + 1),
                }
            }
        })?;
        ops.push(op);
        idx += 1;
    }

    if ops.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "empty batch manifest: no operations provided".into(),
        }
        .into());
    }

    Ok(ops)
}
