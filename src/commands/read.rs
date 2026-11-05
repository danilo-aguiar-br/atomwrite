// SPDX-License-Identifier: MIT OR Apache-2.0

//! File reading with metadata, checksum, and optional content.
//! Workload: I/O-bound (file read + NDJSON output).
//! Parallelism: none — single file.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::binary_detect;
use crate::checksum;
use crate::cli::{GlobalArgs, OutputFormat, ReadArgs};
use crate::error::AtomwriteError;
use crate::ndjson_types::{LineRange, MediaInfo, ReadOutput};
use crate::output::NdjsonWriter;

/// Read a file and emit metadata, checksum, and optional content as NDJSON.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the file does not exist.
/// Returns `AtomwriteError::StateDrift` if `--verify-checksum` fails.
///
/// `--format raw` writes the file's bytes, and refuses a binary target with a
/// hint instead of spending tokens on bytes that carry no text.
#[tracing::instrument(skip_all, fields(command = "read"))]
pub fn cmd_read(
    args: &ReadArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    verb: &str,
) -> Result<()> {
    let start = Instant::now();
    // Removing comment lines without numbers would shift every later line, and
    // the `apply` that follows would target the wrong one.
    if args.no_comments && !numbering_requested(args) {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "{verb} --no-comments {} so every line keeps the file's number; add --number",
                crate::constants::NO_COMMENTS_NEEDS_NUMBER_REASON_MARKER
            ),
        }
        .into());
    }
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

    // A picture or a PDF is not text, so the line machinery below would spend
    // the call on a header and answer nothing a model can use. The media path
    // owns the payload from here.
    if let Some(kind) = crate::media::classify(&path) {
        return read_media(kind, &path, args, writer, &metadata, &hash, bytes);
    }

    // `--ocr` reads pixels, and this file has characters. Taking the flag and
    // doing nothing would let a caller believe the text came from recognition.
    if args.ocr {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "read: {} carries text, so --ocr has nothing to recognize; it applies to a \
                 picture or to a PDF page",
                path.display()
            ),
        }
        .into());
    }

    // `--no-comments` needs a grammar that names comment markers. A plain text
    // target has none, and the filter still drops blank lines; saying so keeps
    // the flag from looking like it did nothing.
    if args.no_comments
        && !is_binary
        && crate::syntax_check::detect_language_name(&path, &[]).is_none()
    {
        crate::best_effort::best_effort(
            writeln!(
                std::io::stderr().lock(),
                "atomwrite: {} has no known language, so --no-comments drops blank lines and lines starting with //, #, --, /* or *",
                path.display()
            ),
            "warn that --no-comments has no comment syntax to drop",
        );
    }

    let permissions_str = format_permissions(&metadata);
    let modified_str = format_modified(&metadata);

    // `--stat` publishes metadata only; the line total still comes from a
    // streaming pass, so the field keeps its meaning without holding the file.
    let mut cut = None;
    let (content_str, lines_total) = if is_binary {
        (None, 0u64)
    } else if args.stat {
        (None, count_lines_streaming(&path)?)
    } else {
        let scanned = scan_lines_streaming(&path, args, max_size)?;
        cut = scanned.truncated.then(|| (true, scanned.next.clone()));
        (Some(scanned.content), scanned.total)
    };
    let (truncated, next) = cut.unwrap_or((false, None));

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
    } else if !args.lines.is_empty() {
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
        truncated: truncated.then_some(true),
        next,
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
        media: None,
    };

    writer.write_event(&output)?;

    tracing::debug!(path = %path.display(), elapsed_ms = start.elapsed().as_millis() as u64, "read complete");
    Ok(())
}
/// Answer a picture or a PDF, which the text path cannot carry.
///
/// A picture leaves as one `data:` URI line on stdout, because that is the only
/// shape the vision channel accepts. A PDF leaves as text per page, with the
/// pages that carry no text named, because silence about a scanned page is the
/// most expensive answer this verb can give.
fn read_media(
    kind: crate::media::MediaKind,
    path: &Path,
    args: &ReadArgs,
    writer: &mut NdjsonWriter<impl Write>,
    metadata: &fs::Metadata,
    hash: &str,
    bytes: u64,
) -> Result<()> {
    let request = media_request(args)?;
    match kind {
        crate::media::MediaKind::Image(format) => {
            let facts = crate::media::image_uri::facts(path, format, &request)?;
            if args.stat {
                let output = ReadOutput {
                    r#type: "read",
                    path: path.display().to_string(),
                    content: None,
                    lines: 0,
                    lines_total: None,
                    truncated: None,
                    next: None,
                    bytes,
                    checksum: hash.to_owned(),
                    permissions: format_permissions(metadata),
                    modified: format_modified(metadata),
                    kind: "image".into(),
                    binary: true,
                    range: None,
                    verified: None,
                    mode: "stat".into(),
                    media: Some(MediaInfo {
                        kind: "image".into(),
                        width: Some(facts.width),
                        height: Some(facts.height),
                        format: Some(facts.format.to_owned()),
                        visual_tokens: Some(facts.visual_tokens),
                        frames: facts.frames,
                        orientation: facts.orientation,
                        pages: None,
                        needs_vision: None,
                    }),
                };
                writer.write_event(&output)?;
                return Ok(());
            }
            let file = FileFacts {
                path,
                metadata,
                hash,
                bytes,
            };
            if args.ocr {
                let picture = crate::media::image_uri::decode_for_ocr(path, &request)?;
                return emit_recognized(&file, writer, "image", &picture, None);
            }
            // The session already paid for these exact bytes. Answering with
            // the facts keeps the caller's context free of a second copy, and
            // `--force` is how it asks for the pixels again.
            if !args.force && crate::media::image_uri::delivered_in_session(hash) {
                let output = ReadOutput {
                    r#type: "read",
                    path: path.display().to_string(),
                    content: None,
                    lines: 0,
                    lines_total: None,
                    truncated: None,
                    next: None,
                    bytes,
                    checksum: hash.to_owned(),
                    permissions: format_permissions(metadata),
                    modified: format_modified(metadata),
                    kind: "image".into(),
                    binary: true,
                    range: None,
                    verified: None,
                    mode: "delivered".into(),
                    media: Some(MediaInfo {
                        kind: "image".into(),
                        width: Some(facts.width),
                        height: Some(facts.height),
                        format: Some(facts.format.to_owned()),
                        visual_tokens: Some(facts.visual_tokens),
                        frames: facts.frames,
                        orientation: facts.orientation,
                        pages: None,
                        needs_vision: None,
                    }),
                };
                writer.write_event(&output)?;
                return Ok(());
            }
            let (uri, facts) = crate::media::image_uri::to_data_uri(path, format, &request)?;
            crate::media::image_uri::guard_size(&uri, crate::constants::IMAGE_MAX_DATA_URI_BYTES)?;
            tracing::debug!(
                path = %path.display(),
                width = facts.width,
                height = facts.height,
                visual_tokens = facts.visual_tokens,
                "picture encoded"
            );
            writer.flush()?;
            let mut out = std::io::stdout().lock();
            out.write_all(uri.as_bytes())?;
            out.write_all(b"\n")?;
            out.flush()?;
            crate::media::image_uri::mark_delivered(hash);
            Ok(())
        }
        crate::media::MediaKind::Heic => {
            let (source_width, source_height) = crate::media::image_uri::heic_size(path)?;
            let (max_edge, max_tokens) =
                crate::media::token_rule::tier_limits(request.high_resolution);
            let max_tokens = request.max_tokens.unwrap_or(max_tokens);
            let (final_width, final_height) =
                crate::media::token_rule::fit(source_width, source_height, max_edge, max_tokens);
            if args.stat {
                let output = ReadOutput {
                    r#type: "read",
                    path: path.display().to_string(),
                    content: None,
                    lines: 0,
                    lines_total: None,
                    truncated: None,
                    next: None,
                    bytes,
                    checksum: hash.to_owned(),
                    permissions: format_permissions(metadata),
                    modified: format_modified(metadata),
                    kind: "image".into(),
                    binary: true,
                    range: None,
                    verified: None,
                    mode: "stat".into(),
                    media: Some(MediaInfo {
                        kind: "image".into(),
                        width: Some(source_width),
                        height: Some(source_height),
                        format: Some("heic".into()),
                        visual_tokens: Some(crate::media::token_rule::visual_tokens(
                            final_width,
                            final_height,
                        )),
                        frames: None,
                        orientation: None,
                        pages: None,
                        needs_vision: None,
                    }),
                };
                writer.write_event(&output)?;
                return Ok(());
            }
            if args.ocr {
                let picture = crate::media::image_uri::decode_heic(path)?;
                let file = FileFacts {
                    path,
                    metadata,
                    hash,
                    bytes,
                };
                return emit_recognized(&file, writer, "image", &picture, None);
            }
            if !args.force && crate::media::image_uri::delivered_in_session(hash) {
                let output = ReadOutput {
                    r#type: "read",
                    path: path.display().to_string(),
                    content: None,
                    lines: 0,
                    lines_total: None,
                    truncated: None,
                    next: None,
                    bytes,
                    checksum: hash.to_owned(),
                    permissions: format_permissions(metadata),
                    modified: format_modified(metadata),
                    kind: "image".into(),
                    binary: true,
                    range: None,
                    verified: None,
                    mode: "delivered".into(),
                    media: Some(MediaInfo {
                        kind: "image".into(),
                        width: Some(final_width),
                        height: Some(final_height),
                        format: Some("jpeg".into()),
                        visual_tokens: Some(crate::media::token_rule::visual_tokens(
                            final_width,
                            final_height,
                        )),
                        frames: None,
                        orientation: None,
                        pages: None,
                        needs_vision: None,
                    }),
                };
                writer.write_event(&output)?;
                return Ok(());
            }
            let picture = crate::media::image_uri::decode_heic(path)?;
            let (uri, facts) = crate::media::image_uri::from_picture(picture, &request)?;
            crate::media::image_uri::guard_size(&uri, crate::constants::IMAGE_MAX_DATA_URI_BYTES)?;
            writer.flush()?;
            let mut out = std::io::stdout().lock();
            out.write_all(uri.as_bytes())?;
            out.write_all(b"\n")?;
            out.flush()?;
            crate::media::image_uri::mark_delivered(hash);
            tracing::debug!(
                path = %path.display(),
                width = facts.width,
                height = facts.height,
                visual_tokens = facts.visual_tokens,
                "heic picture encoded"
            );
            Ok(())
        }
        crate::media::MediaKind::Pdf => {
            // A scanned page carries no text, so `--pages` can only say that
            // the page needs vision. `--page` renders it instead.
            if let Some(page) = args.page {
                let file = FileFacts {
                    path,
                    metadata,
                    hash,
                    bytes,
                };
                return read_rendered_page(&file, page, args, writer, &request);
            }
            let total = crate::media::pdf_text::page_count(path)?;
            if args.stat {
                // `--stat` asks for the shape of the document, not its words:
                // decompressing every page here would pay the whole cost of a
                // text read and then throw the text away.
                let output = ReadOutput {
                    r#type: "read",
                    path: path.display().to_string(),
                    content: None,
                    lines: 0,
                    lines_total: None,
                    truncated: None,
                    next: None,
                    bytes,
                    checksum: hash.to_owned(),
                    permissions: format_permissions(metadata),
                    modified: format_modified(metadata),
                    kind: "pdf".into(),
                    binary: true,
                    range: None,
                    verified: None,
                    mode: "stat".into(),
                    media: Some(MediaInfo {
                        kind: "pdf".into(),
                        width: None,
                        height: None,
                        format: Some("pdf".into()),
                        visual_tokens: None,
                        frames: None,
                        orientation: None,
                        pages: Some((1..=total).collect()),
                        needs_vision: None,
                    }),
                };
                writer.write_event(&output)?;
                return Ok(());
            }
            let pages = requested_pages(&args.pages, total)?;
            let extracted = crate::media::pdf_text::extract_pages(
                path,
                &pages,
                crate::constants::PDF_MAX_DECOMPRESSED_BYTES,
            )?;
            let needs_vision: Vec<u32> = extracted
                .iter()
                .filter(|page| page.needs_vision())
                .map(|page| page.page)
                .collect();
            let content = join_pages(&extracted);
            let line_count = content.lines().count() as u64;
            let output = ReadOutput {
                r#type: "read",
                path: path.display().to_string(),
                content: Some(content),
                lines: line_count,
                lines_total: None,
                truncated: None,
                next: None,
                bytes,
                checksum: hash.to_owned(),
                permissions: format_permissions(metadata),
                modified: format_modified(metadata),
                kind: "pdf".into(),
                binary: true,
                range: None,
                verified: None,
                mode: if args.stat { "stat" } else { "pages" }.into(),
                media: Some(MediaInfo {
                    kind: "pdf".into(),
                    width: None,
                    height: None,
                    format: Some("pdf".into()),
                    visual_tokens: None,
                    frames: None,
                    orientation: None,
                    pages: Some(pages),
                    needs_vision: (!needs_vision.is_empty()).then_some(needs_vision),
                }),
            };
            writer.write_event(&output)?;
            Ok(())
        }
    }
}

/// Translate the media flags into the request the image path takes.
fn media_request(args: &ReadArgs) -> Result<crate::media::image_uri::RenderRequest> {
    let crop = match args.crop.as_deref() {
        None => None,
        Some(spec) => Some(parse_crop(spec)?),
    };
    Ok(crate::media::image_uri::RenderRequest {
        high_resolution: args.high_res,
        max_tokens: args.max_tokens,
        crop,
    })
}

/// Parse `X,Y,W,H`, refusing a window that is not four positive numbers.
fn parse_crop(spec: &str) -> Result<(u32, u32, u32, u32)> {
    let parts: Vec<&str> = spec.split(',').map(str::trim).collect();
    let [x, y, width, height] = parts.as_slice() else {
        return Err(AtomwriteError::InvalidInput {
            reason: format!("--crop needs X,Y,W,H and got {spec:?}"),
        }
        .into());
    };
    let parse = |value: &str, name: &str| -> Result<u32> {
        value.parse::<u32>().map_err(|_| {
            anyhow::Error::from(AtomwriteError::InvalidInput {
                reason: format!("--crop {name} must be a non-negative integer, got {value:?}"),
            })
        })
    };
    let window = (
        parse(x, "x")?,
        parse(y, "y")?,
        parse(width, "width")?,
        parse(height, "height")?,
    );
    if window.2 == 0 || window.3 == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "--crop width and height must be greater than zero".into(),
        }
        .into());
    }
    Ok(window)
}

/// Pages the caller asked for, as 1-based numbers inside the document.
fn requested_pages(specs: &[String], total: u32) -> Result<Vec<u32>> {
    if specs.is_empty() {
        return Ok((1..=total).collect());
    }
    let mut pages: Vec<u32> = Vec::new();
    for spec in specs {
        match spec.split_once(':') {
            Some((first, last)) => {
                let first =
                    first
                        .trim()
                        .parse::<u32>()
                        .map_err(|_| AtomwriteError::InvalidInput {
                            reason: format!(
                                "--pages range start must be a page number, got {first:?}"
                            ),
                        })?;
                let last =
                    last.trim()
                        .parse::<u32>()
                        .map_err(|_| AtomwriteError::InvalidInput {
                            reason: format!(
                                "--pages range end must be a page number, got {last:?}"
                            ),
                        })?;
                pages.extend(crate::media::pdf_text::page_range(first, last, total));
            }
            None => {
                let page =
                    spec.trim()
                        .parse::<u32>()
                        .map_err(|_| AtomwriteError::InvalidInput {
                            reason: format!("--pages must be N or A:B, got {spec:?}"),
                        })?;
                pages.push(page);
            }
        }
    }
    pages.sort_unstable();
    pages.dedup();
    Ok(pages)
}

/// What every media answer repeats about the file it came from.
///
/// The four values travel together in every branch of [`read_media`], so one
/// struct keeps them from turning each helper into a wall of positional
/// arguments.
struct FileFacts<'a> {
    path: &'a Path,
    metadata: &'a fs::Metadata,
    hash: &'a str,
    bytes: u64,
}

/// Recognize the characters on a picture and answer with the text itself.
///
/// The text leaves as `content`, the same field a text file fills, so the
/// caller reads one shape whether the page carried a text layer or not.
fn emit_recognized(
    file: &FileFacts<'_>,
    writer: &mut NdjsonWriter<impl Write>,
    kind: &str,
    picture: &image::DynamicImage,
    pages: Option<Vec<u32>>,
) -> Result<()> {
    let FileFacts {
        path,
        metadata,
        hash,
        bytes,
    } = *file;
    let text = crate::media::ocr::recognize(picture)?;
    let lines = u64::try_from(text.lines().count()).unwrap_or(u64::MAX);
    let output = ReadOutput {
        r#type: "read",
        path: path.display().to_string(),
        content: Some(text),
        lines,
        lines_total: Some(lines),
        truncated: None,
        next: None,
        bytes,
        checksum: hash.to_owned(),
        permissions: format_permissions(metadata),
        modified: format_modified(metadata),
        kind: kind.into(),
        binary: true,
        range: None,
        verified: None,
        mode: "ocr".into(),
        media: Some(MediaInfo {
            kind: kind.into(),
            width: Some(picture.width()),
            height: Some(picture.height()),
            format: None,
            visual_tokens: None,
            frames: None,
            orientation: None,
            pages,
            needs_vision: None,
        }),
    };
    writer.write_event(&output)?;
    tracing::debug!(
        path = %path.display(),
        lines,
        "characters recognized"
    );
    Ok(())
}

/// Render one PDF page and deliver it as a picture.
///
/// `--stat` reports the size of the rendering instead of sending it, which is
/// the cheap way to learn how much a page would cost. The `data:` URI leaves by
/// stdout alone, exactly as a picture does, so a caller reads one channel.
fn read_rendered_page(
    file: &FileFacts<'_>,
    page: u32,
    args: &ReadArgs,
    writer: &mut NdjsonWriter<impl Write>,
    request: &crate::media::image_uri::RenderRequest,
) -> Result<()> {
    let FileFacts {
        path,
        metadata,
        hash,
        bytes,
    } = *file;
    let dpi = args.dpi.unwrap_or(if args.ocr {
        crate::constants::PDF_OCR_RENDER_DPI
    } else {
        crate::constants::PDF_RENDER_DPI
    });
    if !crate::constants::PDF_RENDER_DPI_RANGE.contains(&dpi) {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "read: --dpi {dpi} is outside {} to {} dots per inch",
                crate::constants::PDF_RENDER_DPI_RANGE.start(),
                crate::constants::PDF_RENDER_DPI_RANGE.end()
            ),
        }
        .into());
    }
    let picture = crate::media::pdf_render::render_page(
        path,
        page,
        dpi,
        crate::constants::IMAGE_MAX_MEGAPIXELS.saturating_mul(1_000_000),
    )?;
    if args.ocr {
        return emit_recognized(file, writer, "pdf", &picture, Some(vec![page]));
    }
    let (uri, facts) = crate::media::image_uri::from_picture(picture, request)?;
    if args.stat {
        let output = ReadOutput {
            r#type: "read",
            path: path.display().to_string(),
            content: None,
            lines: 0,
            lines_total: None,
            truncated: None,
            next: None,
            bytes,
            checksum: hash.to_owned(),
            permissions: format_permissions(metadata),
            modified: format_modified(metadata),
            kind: "pdf".into(),
            binary: true,
            range: None,
            verified: None,
            mode: "stat".into(),
            media: Some(MediaInfo {
                kind: "pdf".into(),
                width: Some(facts.width),
                height: Some(facts.height),
                format: Some(facts.format.to_owned()),
                visual_tokens: Some(facts.visual_tokens),
                frames: None,
                orientation: None,
                pages: Some(vec![page]),
                needs_vision: None,
            }),
        };
        writer.write_event(&output)?;
        return Ok(());
    }
    crate::media::image_uri::guard_size(&uri, crate::constants::IMAGE_MAX_DATA_URI_BYTES)?;
    tracing::debug!(
        path = %path.display(),
        page,
        width = facts.width,
        height = facts.height,
        visual_tokens = facts.visual_tokens,
        "PDF page rendered"
    );
    writer.flush()?;
    let mut out = std::io::stdout().lock();
    out.write_all(uri.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// Join the pages, naming each one when the answer covers more than one.
fn join_pages(pages: &[crate::media::pdf_text::PageText]) -> String {
    let mut out = String::new();
    let several = pages.len() > 1;
    for page in pages {
        if several {
            out.push_str(&format!("--- page {} ---\n", page.page));
        }
        out.push_str(&page.text);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out
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
    // `head -n N`.
    let (rendered, emittable, total_lines) = render_raw_lines(data, args);
    let cap = if args.all {
        usize::MAX
    } else {
        crate::config::read_max_output_bytes()
    };
    let (payload, truncated) = cap_lines(&rendered, cap);
    let written = lock.write_all(payload);
    // The notice goes to stderr, never to stdout: raw stdout is the byte
    // channel and must stay `cat` for a pipe. The Bash tool merges both
    // streams, so the model still reads the next step.
    if truncated {
        // Only the lines whose end landed inside the payload count, so the
        // `--` separators never inflate the notice.
        let emitted = emittable
            .iter()
            .filter(|(_, end)| *end <= payload.len())
            .count()
            .max(1);
        let last = emittable
            .get(emitted.saturating_sub(1))
            .map(|(line, _)| *line)
            .unwrap_or(0);
        let next = last.saturating_add(1);
        crate::best_effort::best_effort(
            writeln!(
                std::io::stderr().lock(),
                "atomwrite: showing {emitted} of {} selected lines up to {cap} bytes; next: --lines {next}:{total_lines} (use --all for everything)",
                emittable.len()
            ),
            "write read truncation notice on stderr",
        );
    }
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
    // `--stat` answers for one file: a metadata envelope per operand would mix
    // JSON into the byte channel the caller asked to concatenate.
    if args.stat && targets.paths.len() > 1 {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "{verb} --stat takes one target per invocation, and {} were named",
                targets.paths.len()
            ),
        }
        .into());
    }
    if targets.paths.len() == 1 && targets.empty_patterns.is_empty() {
        let path = crate::path_safety::validate_path(&targets.paths[0], workspace)?;
        if !path.exists() {
            return Err(AtomwriteError::NotFound { path }.into());
        }
        // Every selector streams: a range, a head or a tail used to materialize
        // the whole file, which put the peak at the file size. Only a
        // structural selector needs the complete source for tree-sitter.
        if args.symbol.is_none() && !args.enclosing {
            return stream_one_raw(&path, args, global, writer, verb);
        }
        let raw_bytes = crate::file_io::read_file_bytes(&path, global.effective_max_filesize())?;
        // Binary bytes on the raw channel are paid tokens that carry no text.
        let head = &raw_bytes[..raw_bytes.len().min(crate::constants::BINARY_DETECT_SIZE)];
        if binary_detect::is_binary(head) {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "{verb} --format raw refuses binary {}; drop --format raw: a picture then arrives as a data: URI and a PDF as page text",
                    path.display()
                ),
            }
            .into());
        }
        // The digest is computed only when the caller asked to verify it: a
        // whole-file hash on every read was work nobody consumed.
        if let Some(ref expected) = args.verify_checksum {
            let hash = checksum::hash_bytes(&raw_bytes);
            if &hash != expected {
                return Err(AtomwriteError::ChecksumVerifyFailed {
                    path,
                    expected: expected.clone(),
                }
                .into());
            }
        }
        let mut raw_bytes = raw_bytes;
        if let Some(name) = &args.symbol {
            let spans = crate::commands::outline::symbol_spans(&path, &raw_bytes, None, name);
            let span = crate::commands::outline::resolve_symbol_span(&spans, name, &path)
                .map_err(anyhow::Error::from)?;
            raw_bytes = raw_bytes[span.start..span.end].to_vec();
        } else if args.enclosing {
            let line = args.line.ok_or_else(|| AtomwriteError::InvalidInput {
                reason: "--enclosing needs --line N".to_string(),
            })?;
            let (start, end) =
                crate::commands::outline::enclosing_span(&path, &raw_bytes, None, line)
                    .ok_or_else(|| AtomwriteError::InvalidInput {
                        reason: format!(
                            "no structural block contains line {line} in {}",
                            path.display()
                        ),
                    })?;
            raw_bytes = raw_bytes[start..end].to_vec();
        }
        // A range that starts after the last line used to answer an empty
        // success, which reads as "the file is empty". The line total is the
        // fact that lets the caller fix the request in one step.
        if !args.lines.is_empty() {
            let total = line_spans(&raw_bytes).len();
            let selected = args.lines.iter().any(|spec| {
                spec.split(',')
                    .any(|part| parse_range_str(part.trim(), total).is_some_and(|(s, e)| s < e))
            });
            if !selected && total > 0 {
                return Err(AtomwriteError::InvalidInput {
                    reason: format!(
                        "--lines selects nothing in {}: the file has {total} lines",
                        path.display()
                    ),
                }
                .into());
            }
        }
        return write_raw(writer, &raw_bytes, args);
    }
    if !has_line_filters(args) {
        return stream_raw_targets(targets, global, workspace, writer);
    }
    stream_filtered_targets(targets, args, global, workspace, writer, verb)
}

/// Whether the call asks to carve lines out of the bytes.
fn has_line_filters(args: &ReadArgs) -> bool {
    args.head.is_some()
        || args.tail.is_some()
        || !args.lines.is_empty()
        || args.line.is_some()
        || args.symbol.is_some()
        || args.enclosing
        || args.grep.is_some()
        || args.number
        || args.no_comments
}

/// Open `path` and refuse a binary target before one byte reaches stdout.
fn open_raw_reader(
    path: &std::path::Path,
    verb: &str,
) -> Result<std::io::BufReader<std::fs::File>> {
    use std::io::BufRead;
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::with_capacity(crate::constants::RAW_COPY_BUF_BYTES, file);
    let head = reader.fill_buf()?;
    let len = head.len().min(crate::constants::BINARY_DETECT_SIZE);
    if binary_detect::is_binary(&head[..len]) {
        return Err(binary_raw_error(verb, path));
    }
    Ok(reader)
}

/// The verdict a raw read publishes for a binary target.
fn binary_raw_error(verb: &str, path: &std::path::Path) -> anyhow::Error {
    AtomwriteError::InvalidInput {
        reason: format!(
            "{verb} --format raw refuses binary {}; drop --format raw: a picture then arrives as a data: URI and a PDF as page text",
            path.display()
        ),
    }
    .into()
}

/// Copy one file to stdout, streaming every selector and the digest.
///
/// The binary sniff reads through `fill_buf`, and a filtered read applies the
/// selectors while the file is read, so memory stays at one buffer plus the
/// selected lines instead of following the file size.
fn stream_one_raw(
    path: &std::path::Path,
    args: &ReadArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    verb: &str,
) -> Result<()> {
    writer.flush()?;
    let total_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    // Raw streams the file, and that is exactly why the ceiling belongs here:
    // every other format refuses a target over `--max-filesize`, and this path
    // used to hand the same file back in silence.
    let max_size = global.effective_max_filesize();
    if total_bytes > max_size {
        return Err(AtomwriteError::FileTooLarge {
            path: path.to_path_buf(),
            size: total_bytes,
            max_size,
        }
        .into());
    }
    let mut reader = open_raw_reader(path, verb)?;
    // The digest runs only when the caller asked to verify it, and it streams:
    // a whole-file hash on every read was work nobody consumed.
    if let Some(ref expected) = args.verify_checksum {
        let (hash, _) = checksum::hash_file_streaming(path, global.effective_max_filesize())?;
        if &hash != expected {
            return Err(AtomwriteError::ChecksumVerifyFailed {
                path: path.to_path_buf(),
                expected: expected.clone(),
            }
            .into());
        }
    }
    let mut out = std::io::stdout().lock();
    if !has_line_filters(args) {
        return copy_plain_raw(path, args, &mut reader, &mut out, total_bytes);
    }
    let cap = if args.all {
        usize::MAX
    } else {
        crate::config::read_max_output_bytes()
    };
    let stats = stream_selected_into(path, args, &mut out, cap, verb)?;
    if cap != usize::MAX && stats.truncated {
        write_filtered_notice(path, &stats, cap);
    }
    out.flush().map_err(map_raw_io)
}

/// Copy a plain `cat` target, cutting a large answer on a line boundary.
fn copy_plain_raw(
    path: &std::path::Path,
    args: &ReadArgs,
    reader: &mut std::io::BufReader<std::fs::File>,
    out: &mut impl Write,
    total_bytes: u64,
) -> Result<()> {
    use std::io::Read;
    if args.all {
        copy_raw_stream(reader, out)?;
        return out.flush().map_err(map_raw_io);
    }
    let cap = crate::config::read_max_output_bytes();
    let mut buf = Vec::with_capacity(cap.min(total_bytes as usize).saturating_add(1));
    reader.by_ref().take(cap as u64 + 1).read_to_end(&mut buf)?;
    let (payload, truncated) = cap_lines(&buf, cap);
    // The cut lands on a line boundary, which can still be the middle of a
    // struct. Stepping back to the last complete outline item keeps the next
    // read from starting inside one.
    let payload = if truncated {
        align_raw_payload(path, payload)
    } else {
        payload
    };
    let written = out.write_all(payload).and_then(|()| out.flush());
    if truncated {
        // The line total costs one streaming pass over the bytes that were not
        // sent, and it buys the caller a range that lands exactly on the next
        // line instead of a guess.
        let emitted = payload.iter().filter(|b| **b == b'\n').count().max(1);
        let prefixed = buf.iter().filter(|b| **b == b'\n').count();
        let mut chunk = [0u8; 65_536];
        let mut remaining = 0usize;
        loop {
            let read = reader.read(&mut chunk)?;
            if read == 0 {
                break;
            }
            remaining += memchr::memchr_iter(b'\n', &chunk[..read]).count();
        }
        crate::best_effort::best_effort(
            writeln!(
                std::io::stderr().lock(),
                "atomwrite: {} ({} bytes) has {} lines; showing {} up to {cap} bytes, next: --lines {}:{} (--all for everything)",
                path.display(),
                total_bytes,
                prefixed + remaining,
                emitted,
                emitted + 1,
                prefixed + remaining
            ),
            "write read truncation notice on stderr",
        );
    }
    written.map_err(map_raw_io)
}

/// Counts a streaming selection publishes so the cap notice can name the next range.
struct SelectedStats {
    total_lines: u64,
    selected_lines: u64,
    payload_indices: Vec<usize>,
    truncated: bool,
}

/// Stream one file's selected lines into `out` under `cap`.
///
/// `cap == usize::MAX` writes each selected line as it is chosen; any other cap
/// buffers at most `cap` bytes, cuts at the last complete line and keeps reading
/// only to count what was left out.
fn stream_selected_into(
    path: &std::path::Path,
    args: &ReadArgs,
    out: &mut impl Write,
    cap: usize,
    verb: &str,
) -> Result<SelectedStats> {
    if cap == usize::MAX && !args.lines.is_empty() {
        // Nothing can be unwritten on the byte channel, so a range that lies
        // past the end is refused before the first byte leaves.
        let total = count_lines_streaming(path)?;
        ensure_ranges_select(path, total, args)?;
    }
    drop(open_raw_reader(path, verb)?);
    let number = numbering_enabled(args);
    let mut selector = LineSelector::new(args);
    let mut payload: Vec<u8> = Vec::new();
    let mut payload_indices: Vec<usize> = Vec::new();
    let mut truncated = false;
    let mut selected_lines: u64 = 0;
    let mut io_error: Option<std::io::Error> = None;
    let mut previous: Option<usize> = None;
    let mut emit = |index: usize, raw: &[u8]| {
        if io_error.is_some() {
            return;
        }
        selected_lines += 1;
        // `rg -C` separates non-adjacent groups with `--`, so a reader that
        // asked for two ranges can tell where the first one ends.
        let gap = previous.is_some_and(|prev| index != prev + 1);
        previous = Some(index);
        if cap == usize::MAX {
            if gap && let Err(error) = out.write_all(b"--\n") {
                io_error = Some(error);
                return;
            }
            if let Err(error) = write_numbered_raw(out, index, raw, number) {
                io_error = Some(error);
            }
            return;
        }
        if truncated {
            return;
        }
        let separator = usize::from(gap) * 3;
        let mut chunk = Vec::with_capacity(separator + raw.len() + 16);
        if gap {
            chunk.extend_from_slice(b"--\n");
        }
        chunk.extend_from_slice(&numbered_chunk(index, raw, number));
        if payload.len() + chunk.len() <= cap {
            payload.extend_from_slice(&chunk);
            payload_indices.push(index);
        } else {
            let remaining = cap - payload.len();
            let cut = memchr::memrchr(b'\n', &chunk[..chunk.len().min(remaining)])
                .map(|position| position + 1)
                .unwrap_or(0);
            payload.extend_from_slice(&chunk[..cut]);
            if cut > separator {
                payload_indices.push(index);
            }
            truncated = true;
        }
    };
    let total = visit_raw_lines(path, |index, line| selector.push(index, line, &mut emit))?;
    selector.finish(&mut emit);
    // The closure's borrow ends at its last use, so the buffers it wrote are
    // readable here without an explicit drop.
    if let Some(error) = io_error {
        return Err(map_raw_io(error));
    }
    ensure_ranges_select(path, total, args)?;
    if cap != usize::MAX {
        out.write_all(&payload).map_err(map_raw_io)?;
    }
    Ok(SelectedStats {
        total_lines: total,
        selected_lines,
        payload_indices,
        truncated,
    })
}

/// Write one raw line with its 1-based number in front when numbering is on.
fn write_numbered_raw(
    out: &mut impl Write,
    index: usize,
    raw: &[u8],
    number: bool,
) -> std::io::Result<()> {
    if number {
        write!(out, "{}:", index + 1)?;
    }
    out.write_all(raw)?;
    if number && !raw.ends_with(b"\n") {
        out.write_all(b"\n")?;
    }
    Ok(())
}

/// Build one raw line with its 1-based number in front when numbering is on.
fn numbered_chunk(index: usize, raw: &[u8], number: bool) -> Vec<u8> {
    let mut chunk = Vec::with_capacity(raw.len() + 16);
    if number {
        chunk.extend_from_slice(format!("{}:", index + 1).as_bytes());
    }
    chunk.extend_from_slice(raw);
    if number && !raw.ends_with(b"\n") {
        chunk.push(b'\n');
    }
    chunk
}

/// Tell stderr how much of the selection fit under the cap, and what comes next.
fn write_filtered_notice(path: &std::path::Path, stats: &SelectedStats, cap: usize) {
    let emitted = stats.payload_indices.len().max(1);
    let last = stats.payload_indices.last().copied().unwrap_or(0);
    crate::best_effort::best_effort(
        writeln!(
            std::io::stderr().lock(),
            "atomwrite: {}: showing {emitted} of {} selected lines up to {cap} bytes; next: --lines {}:{} (use --all for everything)",
            path.display(),
            stats.selected_lines,
            last.saturating_add(2),
            stats.total_lines
        ),
        "write read truncation notice on stderr",
    );
}

/// Map a stdout write failure to the crate's broken-pipe semantics.
fn map_raw_io(error: std::io::Error) -> anyhow::Error {
    if error.kind() == std::io::ErrorKind::BrokenPipe {
        crate::error::AtomwriteError::BrokenPipe.into()
    } else {
        anyhow::Error::from(error)
    }
}

/// Copy a reader to a writer in bounded chunks, converting broken pipe to 141.
fn copy_raw_stream(reader: &mut impl std::io::Read, out: &mut impl Write) -> Result<()> {
    let mut buf = vec![0u8; crate::constants::RAW_COPY_BUF_BYTES];
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            return Ok(());
        }
        out.write_all(&buf[..read]).map_err(map_raw_io)?;
    }
}

/// Concatenate several targets, filtering each one, preserving argv order.
///
/// A header names the file whenever more than one target is in play, because a
/// concatenated answer with no owner is a fact the caller cannot attribute.
/// One target may fail and the rest still go out: the status is the first
/// failure's, the same rule the unfiltered path already follows.
fn stream_filtered_targets(
    targets: &ResolvedTargets,
    args: &ReadArgs,
    _global: &GlobalArgs,
    workspace: &std::path::Path,
    writer: &mut NdjsonWriter<impl Write>,
    verb: &str,
) -> Result<()> {
    writer.flush()?;
    let mut out = std::io::stdout().lock();
    let header = targets.paths.len() > 1;
    let cap = if args.all {
        usize::MAX
    } else {
        crate::config::read_max_output_bytes()
    };
    let mut first_error: Option<anyhow::Error> = None;
    for raw_path in &targets.paths {
        let path = match crate::path_safety::validate_path(raw_path, workspace) {
            Ok(path) => path,
            Err(error) => {
                crate::best_effort::best_effort(
                    writeln!(
                        std::io::stderr().lock(),
                        "atomwrite: {}: {error}",
                        raw_path.display()
                    ),
                    "report rejected target on stderr",
                );
                first_error.get_or_insert(error);
                continue;
            }
        };
        if !path.exists() {
            let error = AtomwriteError::NotFound { path: path.clone() };
            crate::best_effort::best_effort(
                writeln!(
                    std::io::stderr().lock(),
                    "atomwrite: {}: not found",
                    path.display()
                ),
                "report missing target on stderr",
            );
            first_error.get_or_insert(error.into());
            continue;
        }
        // The binary sniff runs before the header, so a refused target leaves
        // no half-written attribution on stdout.
        open_raw_reader(&path, verb)?;
        if header {
            out.write_all(format!("==> {} <==\n", path.display()).as_bytes())
                .map_err(map_raw_io)?;
        }
        match stream_selected_into(&path, args, &mut out, cap, verb) {
            Ok(stats) => {
                if cap != usize::MAX && stats.truncated {
                    write_filtered_notice(&path, &stats, cap);
                }
            }
            Err(error) => {
                // A binary target and a checksum mismatch end the run; an
                // unreadable target lets the other files still answer.
                let fatal = error
                    .downcast_ref::<AtomwriteError>()
                    .is_some_and(|failure| {
                        matches!(
                            failure,
                            AtomwriteError::InvalidInput { .. }
                                | AtomwriteError::ChecksumVerifyFailed { .. }
                        )
                    });
                if fatal {
                    return Err(error);
                }
                crate::best_effort::best_effort(
                    writeln!(
                        std::io::stderr().lock(),
                        "atomwrite: {}: {error}",
                        path.display()
                    ),
                    "report unreadable target on stderr",
                );
                first_error.get_or_insert(error);
            }
        }
    }
    out.flush().map_err(map_raw_io)?;
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
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
/// Render the selected raw lines, with optional numbering.
///
/// Returns the payload, the original 0-based index of every emitted line and
/// the file's total line count, so the cap notice can name the exact next
/// range instead of guessing from a byte count.
fn render_raw_lines(data: &[u8], args: &ReadArgs) -> (Vec<u8>, Vec<(usize, usize)>, usize) {
    let ranges = line_spans(data);
    let total = ranges.len();
    let indices = select_line_indices(&ranges, data, args, total);
    let numbered = numbering_enabled(args);
    let mut out = Vec::new();
    let mut emitted = Vec::with_capacity(indices.len());
    let mut previous: Option<usize> = None;
    for &index in &indices {
        // `rg -C` separates non-adjacent groups with `--`, so a reader that
        // asked for two ranges can tell where the first one ends.
        if let Some(prev) = previous
            && index != prev + 1
        {
            out.extend_from_slice(b"--\n");
        }
        let (start, end) = ranges[index];
        if numbered {
            out.extend_from_slice(format!("{}:", index + 1).as_bytes());
        }
        out.extend_from_slice(&data[start..end]);
        if numbered && !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        emitted.push((index + 1, out.len()));
        previous = Some(index);
    }
    (out, emitted, total)
}

/// Pick the 0-based line indices the request selects, in file order.
///
/// Every selector composes on the ORIGINAL line numbers, so `--no-comments`
/// removes lines from the answer without renumbering the rest.
fn select_line_indices(
    ranges: &[(usize, usize)],
    data: &[u8],
    args: &ReadArgs,
    total: usize,
) -> Vec<usize> {
    let mut keep = vec![false; total];
    let mut filtered = false;
    if args.enclosing {
        // The enclosing block is the answer; a line selector inside it would
        // carve the block back down to the single line it came from.
    } else if !args.lines.is_empty() {
        filtered = true;
        for raw in &args.lines {
            for part in raw.split(',') {
                if let Some((start, end)) = parse_range_str(part.trim(), total) {
                    for slot in keep.iter_mut().take(end).skip(start) {
                        *slot = true;
                    }
                }
            }
        }
    } else if let Some(line_num) = args.line {
        filtered = true;
        let idx = line_num.saturating_sub(1);
        if idx < total {
            let ctx = args.context;
            let start = idx.saturating_sub(ctx);
            let end = (idx + ctx + 1).min(total);
            for slot in keep.iter_mut().take(end).skip(start) {
                *slot = true;
            }
        }
    } else if let Some(n) = args.head {
        filtered = true;
        for slot in keep.iter_mut().take(n.min(total)) {
            *slot = true;
        }
    } else if let Some(n) = args.tail {
        filtered = true;
        for slot in keep.iter_mut().skip(total.saturating_sub(n)) {
            *slot = true;
        }
    } else if let Some(pattern) = &args.grep {
        filtered = true;
        if let Ok(re) = regex::Regex::new(pattern) {
            for (index, (start, end)) in ranges.iter().enumerate() {
                if re.is_match(&String::from_utf8_lossy(trim_line_ending(
                    &data[*start..*end],
                ))) {
                    let lo = index.saturating_sub(args.context);
                    let hi = (index + args.context + 1).min(total);
                    for slot in keep.iter_mut().take(hi).skip(lo) {
                        *slot = true;
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    for (index, (start, end)) in ranges.iter().enumerate() {
        if filtered && !keep[index] {
            continue;
        }
        if args.no_comments && is_comment_or_blank(&data[*start..*end]) {
            continue;
        }
        out.push(index);
    }
    out
}

/// Whether a line is blank or a comment in the languages the CLI edits.
///
/// Opt-in through `--no-comments`: a filter guessed wrong on an exotic grammar
/// would silence real content, so the list stays to the markers every language
/// in the project writes the same way.
fn is_comment_or_blank(line: &[u8]) -> bool {
    let text = String::from_utf8_lossy(line);
    let trimmed = text.trim();
    trimmed.is_empty()
        || trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("--")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
}

/// Cut `payload` back to the end of the last complete outline item.
///
/// Only a byte cut on a plain `cat` reaches here, because a caller-named range
/// must return the lines it asked for. Invalid UTF-8 and an unknown grammar
/// keep the line cut, since no outline can be trusted there.
fn align_raw_payload<'a>(path: &std::path::Path, payload: &'a [u8]) -> &'a [u8] {
    let Ok(prefix) = std::str::from_utf8(payload) else {
        return payload;
    };
    let last_line = payload.iter().filter(|b| **b == b'\n').count();
    let Some(end_line) = align_cut_to_outline(path, prefix, last_line) else {
        return payload;
    };
    if end_line == 0 || end_line >= last_line {
        return payload;
    }
    let mut seen = 0usize;
    for (index, byte) in payload.iter().enumerate() {
        if *byte == b'\n' {
            seen += 1;
            if seen == end_line {
                return &payload[..=index];
            }
        }
    }
    payload
}

/// Cut `payload` at the last complete line that fits `cap` bytes.
fn cap_lines(payload: &[u8], cap: usize) -> (&[u8], bool) {
    if payload.len() <= cap {
        return (payload, false);
    }
    let cut = memchr::memrchr(b'\n', &payload[..cap])
        .map(|index| index + 1)
        .unwrap_or(0);
    (&payload[..cut], true)
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
    let mut merged: Option<(usize, usize)> = None;
    for raw in &args.lines {
        for part in raw.split(',') {
            if let Some((start, end)) = parse_range_str(part.trim(), total as usize) {
                merged = Some(match merged {
                    Some((lo, hi)) => (lo.min(start), hi.max(end)),
                    None => (start, end),
                });
            }
        }
    }
    if let Some((start, end)) = merged {
        return Some(LineRange {
            start: start + 1,
            end,
        });
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
