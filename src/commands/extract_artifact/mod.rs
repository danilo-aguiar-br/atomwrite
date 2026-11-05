// SPDX-License-Identifier: MIT OR Apache-2.0

//! Signal extraction per artifact type (`extract-artifact`, v0.1.37 E5).
//!
//! An agent never needs the raw bytes of a PDF, an Office file or an image —
//! it needs the *signal*: page count, dimensions, sheet header, slide titles,
//! document text. This command reads the container, extracts that signal and
//! emits one bounded NDJSON record. The raw stream is never forwarded.
//!
//! Workload: CPU-bound (inflate + XML/PDF tokenization) on a single artifact.
//! A single target has no independent work to fan out, so the shared rayon
//! pool is left untouched and the bound is a byte budget instead of a thread
//! count. Every part read is capped by `--max-filesize`.
//!
//! Coverage is deliberately narrow: containers whose decoder is pure Rust with
//! no C toolchain and no system package — `pdf`, `docx`, `xlsx`, `pptx`, and
//! header-only image probing for `png`, `jpeg`, `gif`, `bmp`, `webp`, `tiff`
//! and `ico`. Type detection reads magic bytes, never the file extension.

use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::cli::GlobalArgs;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;

/// Default number of data rows echoed per spreadsheet sheet.
pub(crate) const DEFAULT_MAX_SAMPLE_ROWS: usize = 10;
/// Default character budget for the document text excerpt.
pub(crate) const DEFAULT_MAX_TEXT_CHARS: usize = 400;
/// Hard ceiling on a single spreadsheet sample cell.
pub(crate) const CELL_CHAR_CAP: usize = 60;
/// Bytes read for magic-byte detection and for the image header probe.
pub(crate) const HEAD_PROBE_BYTES: usize = 64;
/// Maximum number of sheets and slides reported, so a crafted container with
/// tens of thousands of members cannot blow the payload.
pub(crate) const MAX_SHEETS: usize = 32;
/// Maximum number of slides reported.
pub(crate) const MAX_SLIDES: usize = 64;

/// Arguments for `extract-artifact`.
#[derive(clap::Args, Debug)]
pub struct ExtractArtifactArgs {
    /// Artifact to inspect (PDF, Office document, or image).
    #[arg(value_hint = clap::ValueHint::FilePath)]
    pub path: PathBuf,

    /// Maximum data rows echoed per spreadsheet sheet (0 = header only).
    ///
    /// When omitted, `extract.max_sample_rows` applies (see `config get`).
    #[arg(long, value_name = "N")]
    pub max_sample_rows: Option<usize>,

    /// Maximum characters of document text echoed back.
    ///
    /// When omitted, `extract.max_text_chars` applies (see `config get`).
    #[arg(long, value_name = "N")]
    pub max_text_chars: Option<usize>,
}

/// One bounded NDJSON record per artifact, carrying signal and never payload.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ArtifactOutput {
    /// NDJSON discriminator.
    pub r#type: &'static str,
    /// Artifact path inside the workspace jail.
    pub path: String,
    /// Coarse family: `pdf`, `document`, `spreadsheet`, `presentation`, `image`.
    pub kind: &'static str,
    /// Precise container or codec detected from the magic bytes.
    pub format: &'static str,
    /// Artifact size in bytes as reported by the filesystem.
    pub bytes: u64,
    /// Page count (PDF).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<u64>,
    /// Document title from the artifact metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Document author from the artifact metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Producing application from the artifact metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,
    /// Width in pixels (image).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,
    /// Height in pixels (image).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    /// `width * height / 1e6` (image) — the size figure a human reasons about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub megapixels: Option<f64>,
    /// Width after the standard tier resizes the picture, when it has to.
    ///
    /// The header says what the file holds; this says what a vision call
    /// would actually receive, which is the size the model is billed for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_final: Option<u64>,
    /// Height after the standard tier resizes the picture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_final: Option<u64>,
    /// Visual tokens the delivered size costs, by the 28-pixel patch rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_tokens: Option<u64>,
    /// Paragraph count (word-processing document).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraphs: Option<u64>,
    /// Total extracted text length in characters, independent of the excerpt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_chars: Option<u64>,
    /// Bounded head of the document text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_excerpt: Option<String>,
    /// Per-sheet signal (spreadsheet).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheets: Option<Vec<SheetSignal>>,
    /// Per-slide signal (presentation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slides: Option<Vec<SlideSignal>>,
    /// Container members that reached the `max_part` safety ceiling.
    ///
    /// The ceiling is the anti zip-bomb bound and it does not move; a member
    /// that reached it may have been read as a prefix, so the parsed signal
    /// comes with this warning instead of passing as complete.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated_parts: Option<u64>,
    /// Wall-clock milliseconds spent on extraction.
    pub elapsed_ms: u64,
}

/// Bounded signal for one spreadsheet sheet.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SheetSignal {
    /// Sheet name as declared by the workbook, or the part index when absent.
    pub name: String,
    /// Number of populated rows.
    pub rows: u64,
    /// Number of columns in the first populated row.
    pub columns: u64,
    /// Column labels taken from the first populated row, blank when unlabeled.
    pub header: Vec<String>,
    /// Inferred scalar type per column over the sampled rows.
    pub types: Vec<String>,
    /// Sample rows, each holding one value per column.
    pub sample: Vec<Vec<String>>,
}

/// Bounded signal for one presentation slide.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SlideSignal {
    /// One-based slide position in presentation order.
    pub index: u64,
    /// First text run of the slide, which carries the slide heading.
    pub title: String,
    /// Total characters of text on the slide.
    pub text_chars: u64,
}

/// Container family detected from the artifact's leading bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Container {
    /// `%PDF-` header.
    Pdf,
    /// ZIP local-file header; the member list decides which Office format.
    Zip,
    /// A raster image whose dimensions live in the header.
    Image,
}

/// Extract the signal of one artifact and emit it as a single NDJSON record.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` when the path does not exist.
/// Returns `AtomwriteError::InvalidInput` when the bytes are recognised but
/// unreadable (corrupt container) or when no supported format is recognised.
/// Returns `AtomwriteError::FileTooLarge` when one container member exceeds
/// `--max-filesize`.
pub fn cmd_extract_artifact(
    args: &ExtractArtifactArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let workspace = global.resolve_workspace()?;
    let path = crate::path_safety::validate_path(&args.path, &workspace)?;
    let tuning = crate::config::tuning();
    let output = extract_signal(
        &path,
        args.max_sample_rows
            .unwrap_or(tuning.extract_max_sample_rows),
        args.max_text_chars.unwrap_or(tuning.extract_max_text_chars),
        global.effective_max_filesize(),
    )?;
    writer.write_event(&output)?;
    Ok(())
}

/// Build the signal record for one artifact path.
///
/// Split out of the command handler so every format can be exercised directly
/// by the unit tests, with no clap surface and no subprocess in the way.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` when the path does not exist.
/// Returns `AtomwriteError::InvalidInput` when the bytes are recognised but
/// unreadable (corrupt container) or when no supported format is recognised.
/// Returns `AtomwriteError::FileTooLarge` when one container member exceeds
/// `max_part`.
pub(crate) fn extract_signal(
    path: &std::path::Path,
    max_sample_rows: usize,
    max_text_chars: usize,
    max_part: u64,
) -> Result<ArtifactOutput> {
    let start = Instant::now();

    if !path.exists() {
        return Err(AtomwriteError::NotFound {
            path: path.to_path_buf(),
        }
        .into());
    }
    if path.is_dir() {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "{} is a directory; extract-artifact reads one file — \
                 use `list` for a directory inventory",
                path.display()
            ),
        }
        .into());
    }

    let bytes = std::fs::metadata(path)
        .with_context(|| format!("cannot stat {}", path.display()))?
        .len();
    let head = read_head(path)?;

    let mut output = ArtifactOutput {
        r#type: "artifact",
        path: path.display().to_string(),
        kind: "unknown",
        format: "unknown",
        bytes,
        pages: None,
        title: None,
        author: None,
        producer: None,
        width: None,
        height: None,
        megapixels: None,
        width_final: None,
        height_final: None,
        visual_tokens: None,
        paragraphs: None,
        text_chars: None,
        text_excerpt: None,
        sheets: None,
        slides: None,
        truncated_parts: None,
        elapsed_ms: 0,
    };

    match detect_container(&head) {
        Some(Container::Pdf) => extract_pdf(path, max_text_chars, max_part, &mut output)?,
        Some(Container::Zip) => {
            extract_office(path, max_sample_rows, max_text_chars, max_part, &mut output)?;
        }
        Some(Container::Image) => extract_image(path, &head, &mut output)?,
        None => {
            return Err(unsupported(
                path,
                "no supported magic bytes (expected PDF, an OOXML container, or an image header)",
            ));
        }
    }

    output.elapsed_ms = start.elapsed().as_millis() as u64;
    Ok(output)
}

/// Read at most [`HEAD_PROBE_BYTES`] from the artifact head.
fn read_head(path: &std::path::Path) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut head = vec![0u8; HEAD_PROBE_BYTES];
    let mut filled = 0usize;
    let mut reader = BufReader::new(file);
    while filled < head.len() {
        let n = reader
            .read(&mut head[filled..])
            .with_context(|| format!("cannot read {}", path.display()))?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    head.truncate(filled);
    Ok(head)
}

/// Classify the artifact by its leading bytes. Extensions are never consulted.
fn detect_container(head: &[u8]) -> Option<Container> {
    if head.starts_with(b"%PDF-") {
        return Some(Container::Pdf);
    }
    // Local file header, central directory, or spanned marker: all OOXML
    // containers are ZIP archives, so any of the three means "try to unzip".
    if head.starts_with(b"PK\x03\x04")
        || head.starts_with(b"PK\x05\x06")
        || head.starts_with(b"PK\x07\x08")
    {
        return Some(Container::Zip);
    }
    if crate::binary_detect::is_binary(head) && imagesize::image_type(head).is_ok() {
        return Some(Container::Image);
    }
    None
}

/// Build the error for bytes that are recognised but cannot be parsed.
pub(crate) fn corrupt(path: &std::path::Path, reason: impl std::fmt::Display) -> anyhow::Error {
    AtomwriteError::InvalidInput {
        reason: format!("corrupt artifact {}: {reason}", path.display()),
    }
    .into()
}

/// Build the error for bytes whose format is outside the supported set.
pub(crate) fn unsupported(path: &std::path::Path, reason: &str) -> anyhow::Error {
    AtomwriteError::InvalidInput {
        reason: format!(
            "unsupported artifact {}: {reason}. Supported: pdf, docx, xlsx, pptx, \
             png, jpeg, gif, bmp, webp, tiff, ico",
            path.display()
        ),
    }
    .into()
}

/// Truncate `text` to `cap` characters on a character boundary.
pub(crate) fn truncate_chars(text: &str, cap: usize) -> String {
    if text.chars().count() <= cap {
        return text.to_string();
    }
    text.chars().take(cap).collect()
}

use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::Event;

/// A reader with text trimming on; entities resolve with the default set.
fn xml_reader(xml: &str) -> Reader<&[u8]> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    reader
}

/// Lowercase local name of a start or empty element.
fn local_name(name: quick_xml::name::QName<'_>) -> String {
    // Since quick-xml 0.42 the reader is str-based, so `LocalName` derefs to
    // `&str` instead of `&[u8]`.
    name.local_name().as_ref().to_ascii_lowercase()
}

/// Decoded, unescaped text of a `Text` or `CData` event.
fn text_of(event: &Event<'_>) -> Option<String> {
    match event {
        // 0.42 removed `BytesText::unescape`; `xml_content` is the replacement
        // and resolves the predefined entities under the declared XML version.
        Event::Text(t) => Some(t.xml_content(XmlVersion::Implicit1_0).into_owned()),
        Event::CData(c) => Some(c.as_ref().to_string()),
        _ => None,
    }
}

/// Value of attribute `key` on a start element, if present.
fn attr_value(element: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<String> {
    let key = std::str::from_utf8(key).ok()?;
    for attribute in element.attributes().flatten() {
        if attribute.key.as_ref() == key {
            // `value` is already `Cow<str>` in 0.42; `normalized_value`
            // applies the XML whitespace and predefined-entity rules.
            // `unescape_value` is deprecated in favour of it. An error falls
            // back to the raw text so a malformed attribute never loses the
            // whole document.
            return Some(
                attribute
                    .normalized_value(XmlVersion::Implicit1_0)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| attribute.value.to_string()),
            );
        }
    }
    None
}

include!("office.inc.rs");
include!("pdf.inc.rs");
include!("image.inc.rs");
include!("tests.inc.rs");
