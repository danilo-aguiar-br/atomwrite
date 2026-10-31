// SPDX-License-Identifier: MIT OR Apache-2.0

//! File reading with metadata, checksum, and optional content.
//! Workload: I/O-bound (file read + NDJSON output).
//! Parallelism: none — single file.

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
///
/// `--format raw` writes the file's bytes even when they look binary: the mode
/// promises the bytes, so it never refuses a file for its content.
#[tracing::instrument(skip_all, fields(command = "read"))]
pub fn cmd_read(
    args: &ReadArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    verb: &str,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let targets = resolve_targets(&args.paths, &workspace)?;

    // Raw mode IS the byte channel, so several operands concatenate right here.
    if matches!(args.format, OutputFormat::Raw) {
        return write_raw_targets(&targets, args, global, &workspace, writer, verb);
    }

    // Every other mode answers for ONE subject: a pattern that matched nothing
    // is the same failure a missing name is, and several targets are a mistake.
    if let Some(pattern) = targets.empty_patterns.first() {
        return Err(AtomwriteError::GlobNoMatches {
            pattern: pattern.clone(),
        }
        .into());
    }
    if targets.paths.len() != 1 {
        // The verb names itself (A13), and the wording stays clear of the
        // carving-flag marker: a call that named two files without `--head` or
        // `--lines` must not be told to drop a flag it never passed.
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "{verb} takes one target per invocation, and {} were named",
                targets.paths.len()
            ),
        }
        .into());
    }
    let path = crate::path_safety::validate_path(&targets.paths[0], &workspace)?;

    if !path.exists() {
        return Err(AtomwriteError::NotFound { path }.into());
    }

    // A pattern that does not compile is refused up front, not ignored while filtering.
    if let Some(pattern) = args.grep.as_deref() {
        regex::Regex::new(pattern).map_err(|e| {
            crate::error::AtomwriteError::invalid(&format!("invalid --grep pattern '{pattern}'"), e)
        })?;
    }

    let metadata =
        fs::metadata(&path).with_context(|| format!("cannot stat {}", path.display()))?;
    let max_size = global.effective_max_filesize();

    // Every other mode needs the digest, the size and the first bytes only: a
    // 280 MB file used to be read whole just to answer `--stat`, and the peak
    // matched the file size. The digest comes from the mmap path for large
    // files, and the line pass below streams with a fixed buffer.
    let is_binary = detect_binary_prefix(&path)?;
    // Streaming, not mmap: a mapped file counts its pages in RSS, and this verb
    // promises a memory ceiling it cannot keep while the whole mapping is live.
    let (hash, bytes) = checksum::hash_file_streaming(&path, max_size)?;

    if let Some(ref expected) = args.verify_checksum
        && &hash != expected
    {
        return Err(AtomwriteError::ChecksumVerifyFailed {
            path,
            expected: expected.clone(),
        }
        .into());
    }

    let permissions_str = format_permissions(&metadata);
    let modified_str = format_modified(&metadata);

    // `--stat` publishes metadata only; the line total still comes from a
    // streaming pass, so the field keeps its meaning without holding the file.
    let (content_str, lines_total) = if is_binary {
        (None, 0u64)
    } else if args.stat {
        (None, count_lines_streaming(&path)?)
    } else {
        let (content, total) = scan_lines_streaming(&path, args)?;
        (Some(content), total)
    };

    let line_count = content_str
        .as_deref()
        .map(|s| s.lines().count() as u64)
        .unwrap_or(lines_total);
    let range = if is_binary {
        None
    } else {
        parse_line_range(args, lines_total)
    };

    // FIX GAP-2026-009: emit `mode` discriminator for downstream consumers
    let mode = if args.stat {
        "stat"
    } else if args.line.is_some() {
        "line"
    } else if args.lines.is_some() {
        "lines"
    } else if args.head.is_some() {
        "head"
    } else if args.tail.is_some() {
        "tail"
    } else if args.grep.is_some() {
        "grep"
    } else {
        "full"
    };

    let output = ReadOutput {
        r#type: "read",
        path: path.display().to_string(),
        content: content_str,
        lines: line_count,
        lines_total: if lines_total != line_count {
            Some(lines_total)
        } else {
            None
        },
        bytes,
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
        mode: mode.to_string(),
    };

    writer.write_event(&output)?;

    tracing::debug!(path = %path.display(), elapsed_ms = start.elapsed().as_millis() as u64, "read complete");
    Ok(())
}

fn write_raw(writer: &mut NdjsonWriter<impl Write>, data: &[u8], args: &ReadArgs) -> Result<()> {
    // B-006: `--format raw` always emits file bytes (TTY and non-TTY).
    // Agent NDJSON contract uses default format; `--bytes` is a no-op alias.
    writer.flush()?;
    let inner = std::io::stdout();
    let mut lock = inner.lock();
    // Filtering runs on the BYTES, never on a lossy `String`: decoding first
    // rewrote invalid UTF-8 into `EF BF BD` and dropped the `\r` of CRLF
    // files, so `read --format raw --head N` returned bytes that differed from
    // `head -n N`. Binary files take the same path: the old branch handed the
    // whole file back and silently ignored `--head`, `--tail` and `--lines`.
    let filtered = apply_line_filters(data, args);
    let written = lock.write_all(&filtered);
    // A closed reader is a failed run (exit 141), never a silent success, and
    // a failed flush loses bytes the same way a failed write does.
    written.and_then(|()| lock.flush()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            crate::error::AtomwriteError::BrokenPipe.into()
        } else {
            anyhow::Error::from(e)
        }
    })
}

/// Write the bytes of the resolved targets, in argv order.
///
/// One target keeps the shape it had before this change: the file is materialized,
/// so `--lines`, `--head`, `--tail` and `--verify-checksum` keep answering
/// exactly as they did. Several targets stream, because the peak memory of a
/// concatenation must not follow the sum of the file sizes.
fn write_raw_targets(
    targets: &ResolvedTargets,
    args: &ReadArgs,
    global: &GlobalArgs,
    workspace: &std::path::Path,
    writer: &mut NdjsonWriter<impl Write>,
    verb: &str,
) -> Result<()> {
    if targets.paths.len() == 1 && targets.empty_patterns.is_empty() {
        let path = crate::path_safety::validate_path(&targets.paths[0], workspace)?;
        if !path.exists() {
            return Err(AtomwriteError::NotFound { path }.into());
        }
        let raw_bytes = crate::file_io::read_file_bytes(&path, global.effective_max_filesize())?;
        let hash = checksum::hash_bytes(&raw_bytes);
        if let Some(ref expected) = args.verify_checksum
            && &hash != expected
        {
            return Err(AtomwriteError::ChecksumVerifyFailed {
                path,
                expected: expected.clone(),
            }
            .into());
        }
        return write_raw(writer, &raw_bytes, args);
    }
    refuse_batch_flags(args, verb)?;
    stream_raw_targets(targets, global, workspace, writer)
}

/// Refuse the flags that carve one file when the call names several.
///
/// `cat` has no line range, so a concatenation with `--head` would silently
/// apply the range to every file or to none. Naming the flag is the only answer
/// that lets the caller fix the command in one step.
///
/// `--stat` is skipped on the `stat` verb: there the flag is set by the verb
/// itself and not asked for by the caller, and refusing it made
/// `stat --format raw a b` a contradiction — the mode was chosen and the verb's
/// own metadata flag vetoed it.
fn refuse_batch_flags(args: &ReadArgs, verb: &str) -> Result<()> {
    let flag = if args.head.is_some() {
        "--head"
    } else if args.tail.is_some() {
        "--tail"
    } else if args.lines.is_some() {
        "--lines"
    } else if args.line.is_some() {
        "--line"
    } else if args.context != 0 {
        "--context"
    } else if args.stat && verb != "stat" {
        "--stat"
    } else if args.grep.is_some() {
        "--grep"
    } else if args.verify_checksum.is_some() {
        "--verify-checksum"
    } else {
        return Ok(());
    };
    Err(AtomwriteError::InvalidInput {
        reason: format!(
            "{verb} {flag} expects a single target, and several were named; run the verb once per file"
        ),
    }
    .into())
}

/// Concatenate every target byte for byte, the way `cat` does.
///
/// No header, no separator and no added newline: a file that ends without `\n`
/// is followed by the next file's first byte. A target that fails prints one
/// line on stderr and the run keeps going, because the other targets still owe
/// their bytes to stdout. The status is the first failure's, which is the
/// semantic code the owner chose over `cat`'s flat 1.
fn stream_raw_targets(
    targets: &ResolvedTargets,
    global: &GlobalArgs,
    workspace: &std::path::Path,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    writer.flush()?;
    let max_size = global.effective_max_filesize();
    let shutdown = crate::signal::get_or_install_handlers()?;
    let mut cancel = crate::signal::CancelCheck::for_bytes(shutdown.flag());
    let mut out = std::io::stdout().lock();
    let mut buf = vec![0u8; crate::constants::RAW_COPY_BUF_BYTES];
    let mut first_error: Option<AtomwriteError> = None;
    let mut bytes_total: u64 = 0;
    for path in &targets.paths {
        // Every target faces the crate's one jail check, and expansion does not
        // skip it: a pattern that matches a symbolic link pointing outside the
        // workspace would otherwise stream the target's bytes, while the same
        // link named explicitly is refused with 126 (measured on the build
        // before this change).
        let checked = match crate::path_safety::validate_path(path, workspace) {
            Ok(checked) => checked,
            Err(e) => {
                let failure = e.downcast::<AtomwriteError>().unwrap_or_else(|other| {
                    AtomwriteError::InternalError {
                        reason: format!("{other:#}"),
                    }
                });
                record_failure(failure, &mut first_error);
                continue;
            }
        };
        match copy_one_target(
            &checked,
            &mut out,
            &mut buf,
            max_size,
            &mut cancel,
            &mut bytes_total,
        ) {
            Ok(()) => {}
            // A closed reader and an elapsed deadline end the run: the bytes
            // after them were never requested in full by anyone.
            Err(e) if is_fatal(&e) => return Err(e.into()),
            Err(e) => record_failure(e, &mut first_error),
        }
    }
    for pattern in &targets.empty_patterns {
        record_failure(
            AtomwriteError::GlobNoMatches {
                pattern: pattern.clone(),
            },
            &mut first_error,
        );
    }
    out.flush()
        .map_err(|e| write_error_at(std::path::Path::new(""), e))?;
    tracing::debug!(
        targets = targets.paths.len(),
        bytes = bytes_total,
        "read raw concatenation complete"
    );
    match first_error {
        // The line for every failure already went to stderr, in argv order, so
        // the status travels alone: the marker tells the dispatcher that the
        // verdict was published and only the exit code is still owed.
        Some(e) => Err(anyhow::Error::from(e).context(crate::output::VerdictPublished)),
        None => Ok(()),
    }
}

/// Report every failure where it happens and keep the FIRST for the status.
///
/// Printing only the later ones and leaving the first to the caller put the
/// first failure LAST on stderr, so the lines ran against argv order (A14,
/// measured with two oversized targets: `b.rs` printed before `a.rs`). The
/// status still comes from the first failure, and the returned error carries
/// [`crate::output::VerdictPublished`] so the caller does not print it twice.
fn record_failure(e: AtomwriteError, first: &mut Option<AtomwriteError>) {
    report_target_error(&e);
    if first.is_none() {
        *first = Some(e);
    }
}

/// Whether a per-target failure ends the whole run instead of skipping a file.
fn is_fatal(e: &AtomwriteError) -> bool {
    matches!(e, AtomwriteError::BrokenPipe) || crate::signal::is_global_shutdown()
}

/// Copy one file to stdout through the cancel check.
fn copy_one_target(
    path: &std::path::Path,
    out: &mut impl Write,
    buf: &mut [u8],
    max_size: u64,
    cancel: &mut crate::signal::CancelCheck,
    bytes_total: &mut u64,
) -> std::result::Result<(), AtomwriteError> {
    use std::io::Read;
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(AtomwriteError::NotFound {
                path: path.to_path_buf(),
            });
        }
        Err(e) => return Err(AtomwriteError::from_io_at(path, e)),
    };
    let size = file
        .metadata()
        .map_err(|e| AtomwriteError::from_io_at(path, e))?
        .len();
    if max_size > 0 && size > max_size {
        return Err(AtomwriteError::FileTooLarge {
            path: path.to_path_buf(),
            size,
            max_size,
        });
    }
    loop {
        let read = file
            .read(buf)
            .map_err(|e| AtomwriteError::from_io_at(path, e))?;
        if read == 0 {
            return Ok(());
        }
        // The deadline has to cut a 196 MB file in the middle, not after it.
        cancel.advance(read as u64)?;
        out.write_all(&buf[..read])
            .map_err(|e| write_error_at(path, e))?;
        *bytes_total += read as u64;
    }
}

/// Map a write failure, keeping the broken-pipe verdict separate.
fn write_error_at(path: &std::path::Path, e: std::io::Error) -> AtomwriteError {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        AtomwriteError::BrokenPipe
    } else {
        AtomwriteError::from_io_at(path, e)
    }
}

/// Report one target's failure on stderr, where diagnostics belong.
fn report_target_error(e: &AtomwriteError) {
    crate::best_effort::best_effort(
        writeln!(std::io::stderr().lock(), "atomwrite: {e}"),
        "report read batch error on stderr",
    );
}

/// Whether the file starts with bytes that mark it as binary.
///
/// Only the detection window is read, so deciding the `kind` of a 280 MB file
/// costs one small read instead of the whole file.
fn detect_binary_prefix(path: &std::path::Path) -> Result<bool> {
    use std::io::Read;
    let mut file =
        fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut head = [0u8; crate::constants::BINARY_DETECT_SIZE];
    let read = file
        .read(&mut head)
        .with_context(|| format!("cannot read {}", path.display()))?;
    Ok(binary_detect::is_binary(&head[..read]))
}

/// Select the requested lines from `data`, returning their ORIGINAL bytes.
///
/// `raw` promises the file's bytes, so this runs on `&[u8]` and copies whole
/// line slices, terminator included. The previous version decoded with
/// `from_utf8_lossy` and rebuilt the result with `join("\n") + "\n"`, which
/// rewrote invalid UTF-8 into `EF BF BD`, dropped the `\r` of CRLF files, and
/// appended a newline to a file whose last line had none. A slice of the
/// original buffer cannot do any of those three things.
///
/// A file that does not end in a newline keeps that shape: the final line is
/// copied without a terminator, exactly as `head` and `tail` would print it.
fn apply_line_filters(data: &[u8], args: &ReadArgs) -> Vec<u8> {
    let ranges = line_spans(data);
    let total = ranges.len();

    if let Some(ref range_str) = args.lines {
        if let Some((start, end)) = parse_range_str(range_str, total) {
            if start >= end {
                return Vec::new();
            }
            return concat_spans(data, &ranges[start..end]);
        }
    }

    if let Some(line_num) = args.line {
        let idx = line_num.saturating_sub(1);
        if idx >= total {
            return Vec::new();
        }
        let ctx = args.context;
        let start = idx.saturating_sub(ctx);
        let end = (idx + ctx + 1).min(total);
        return concat_spans(data, &ranges[start..end]);
    }

    if let Some(n) = args.head {
        let end = n.min(total);
        if end == 0 {
            return Vec::new();
        }
        return concat_spans(data, &ranges[..end]);
    }

    if let Some(n) = args.tail {
        if n == 0 {
            return Vec::new();
        }
        let start = total.saturating_sub(n);
        return concat_spans(data, &ranges[start..]);
    }

    if let Some(ref pattern) = args.grep {
        if let Ok(re) = regex::Regex::new(pattern) {
            let matched: Vec<(usize, usize)> = ranges
                .iter()
                .copied()
                .filter(|span| re.is_match(&String::from_utf8_lossy(&data[span.0..span.1])))
                .collect();
            return concat_spans(data, &matched);
        }
    }

    data.to_vec()
}

/// Byte span of every line in `data`, terminator included when present.
///
/// Splitting on the `\n` byte keeps the `\r` inside the span, so concatenating
/// spans reproduces CRLF files exactly, and the last line keeps its absent
/// terminator instead of gaining one.
fn line_spans(data: &[u8]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start = 0;
    for (index, byte) in data.iter().enumerate() {
        if *byte == b'\n' {
            spans.push((start, index + 1));
            start = index + 1;
        }
    }
    if start < data.len() {
        spans.push((start, data.len()));
    }
    spans
}

/// Concatenate the chosen line spans, byte for byte, with no separator added.
fn concat_spans(data: &[u8], spans: &[(usize, usize)]) -> Vec<u8> {
    let mut out = Vec::with_capacity(spans.iter().map(|(a, b)| b - a).sum());
    for (start, end) in spans {
        out.extend_from_slice(&data[*start..*end]);
    }
    out
}

fn parse_range_str(s: &str, total: usize) -> Option<(usize, usize)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let start = parts[0].parse::<usize>().ok()?.saturating_sub(1).min(total);
    let end = parts[1].parse::<usize>().ok()?.min(total);
    if start >= end {
        return Some((0, 0));
    }
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
        format!(
            "0o{:o}",
            metadata.permissions().mode() & crate::constants::UNIX_MODE_PERMISSION_MASK
        )
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

include!("read_globs.inc.rs");
include!("read_lines.inc.rs");
