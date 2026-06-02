// SPDX-License-Identifier: MIT OR Apache-2.0

//! File reading with metadata, checksum, and optional content.
//! Workload: I/O-bound (file read + NDJSON output).

use std::fs;
use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::binary_detect;
use crate::checksum;
use crate::cli::{GlobalArgs, OutputFormat, ReadArgs};
use crate::error::AtomwriteError;
use crate::ndjson_types::{LineRange, ReadOutput};
use crate::output::NdjsonWriter;

/// Read a file and emit metadata, checksum, and optional content as NDJSON.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the file does not exist.
/// Returns `AtomwriteError::StateDrift` if `--verify-checksum` fails.
/// Returns `AtomwriteError::BinaryFile` if `--format raw` is used on a binary file.
#[tracing::instrument(skip_all, fields(command = "read"))]
pub fn cmd_read(
    args: &ReadArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let path = crate::path_safety::validate_path(&args.path, &workspace)?;

    if !path.exists() {
        return Err(AtomwriteError::NotFound { path: path.clone() }.into());
    }

    let metadata =
        fs::metadata(&path).with_context(|| format!("cannot stat {}", path.display()))?;

    let raw_bytes = crate::file_io::read_file_bytes(&path, global.effective_max_filesize())?;

    let is_binary = binary_detect::is_binary(&raw_bytes);
    let hash = checksum::hash_bytes(&raw_bytes);

    if let Some(ref expected) = args.verify_checksum {
        let verified = &hash == expected;
        if !verified {
            return Err(AtomwriteError::StateDrift {
                path: path.clone(),
                expected: expected.clone(),
                actual: hash,
            }
            .into());
        }
    }

    let permissions_str = format_permissions(&metadata);
    let modified_str = format_modified(&metadata);

    if matches!(args.format, OutputFormat::Raw) {
        return write_raw(writer, &raw_bytes, args, is_binary);
    }

    let content_str = if is_binary || args.stat {
        None
    } else {
        let text = String::from_utf8_lossy(&raw_bytes);
        Some(apply_line_filters(&text, args))
    };

    let (line_count, range) = if is_binary {
        (0, None)
    } else {
        let text = String::from_utf8_lossy(&raw_bytes);
        let total_lines = text.lines().count() as u64;
        let range = parse_line_range(args, total_lines);
        (total_lines, range)
    };

    let output = ReadOutput {
        r#type: "read",
        path: path.display().to_string(),
        content: content_str,
        lines: line_count,
        bytes: raw_bytes.len() as u64,
        checksum: hash,
        permissions: permissions_str,
        modified: modified_str,
        kind: if is_binary {
            "binary".into()
        } else {
            "text".into()
        },
        binary: is_binary,
        range,
        verified: args.verify_checksum.as_ref().map(|_| true),
    };

    writer.write_event(&output)?;

    tracing::debug!(path = %path.display(), elapsed_ms = start.elapsed().as_millis() as u64, "read complete");
    Ok(())
}

fn write_raw(
    writer: &mut NdjsonWriter<impl Write>,
    data: &[u8],
    args: &ReadArgs,
    is_binary: bool,
) -> Result<()> {
    if is_binary {
        return Err(AtomwriteError::BinaryFile {
            path: args.path.clone(),
        }
        .into());
    }

    let text = String::from_utf8_lossy(data);
    let filtered = apply_line_filters(&text, args);

    writer.flush()?;
    let inner = std::io::stdout();
    let mut lock = inner.lock();
    match lock.write_all(filtered.as_bytes()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => return Ok(()),
        Err(e) => return Err(e.into()),
    }
    let _ = lock.flush();
    Ok(())
}

fn apply_line_filters(text: &str, args: &ReadArgs) -> String {
    let all_lines: Vec<&str> = text.lines().collect();
    let total = all_lines.len();

    if let Some(ref range_str) = args.lines {
        if let Some((start, end)) = parse_range_str(range_str, total) {
            return all_lines[start..end].join("\n") + "\n";
        }
    }

    if let Some(line_num) = args.line {
        let idx = line_num.saturating_sub(1);
        let ctx = args.context;
        let start = idx.saturating_sub(ctx);
        let end = (idx + ctx + 1).min(total);
        return all_lines[start..end].join("\n") + "\n";
    }

    if let Some(n) = args.head {
        let end = n.min(total);
        return all_lines[..end].join("\n") + "\n";
    }

    if let Some(n) = args.tail {
        let start = total.saturating_sub(n);
        return all_lines[start..].join("\n") + "\n";
    }

    if let Some(ref pattern) = args.grep {
        if let Ok(re) = regex::Regex::new(pattern) {
            let matched: Vec<&str> = all_lines
                .iter()
                .copied()
                .filter(|l| re.is_match(l))
                .collect();
            return matched.join("\n") + "\n";
        }
    }

    text.to_owned()
}

fn parse_range_str(s: &str, total: usize) -> Option<(usize, usize)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let start = parts[0].parse::<usize>().ok()?.saturating_sub(1);
    let end = parts[1].parse::<usize>().ok()?.min(total);
    Some((start, end))
}

fn parse_line_range(args: &ReadArgs, total: u64) -> Option<LineRange> {
    if let Some(ref range_str) = args.lines {
        if let Some((start, end)) = parse_range_str(range_str, total as usize) {
            return Some(LineRange {
                start: start + 1,
                end,
            });
        }
    }
    if let Some(line_num) = args.line {
        let ctx = args.context;
        let start = line_num.saturating_sub(ctx);
        let end = (line_num + ctx).min(total as usize);
        return Some(LineRange { start, end });
    }
    None
}

fn format_permissions(metadata: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        format!("0o{:o}", metadata.permissions().mode() & 0o7777)
    }
    #[cfg(not(unix))]
    {
        if metadata.permissions().readonly() {
            "readonly".into()
        } else {
            "readwrite".into()
        }
    }
}

fn format_modified(metadata: &fs::Metadata) -> String {
    match metadata.modified() {
        Ok(time) => {
            // defaults to epoch if file mtime precedes UNIX epoch — display-only
            let secs = time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let (year, month, day, hour, min, sec) = crate::atomic::epoch_to_utc(secs);
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
        }
        Err(_) => "unknown".into(),
    }
}
