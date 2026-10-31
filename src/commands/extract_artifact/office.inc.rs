// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by `extract_artifact/mod.rs` (E5 — OOXML: docx, xlsx, pptx).

use zip::ZipArchive;
use zip::result::ZipError;

/// Workbook part that lists the sheet names in workbook order.
const PART_WORKBOOK: &str = "xl/workbook.xml";
/// Shared string table part, indexed by `<c t="s"><v>N</v></c>`.
const PART_SHARED_STRINGS: &str = "xl/sharedStrings.xml";
/// `WordprocessingML` body part.
const PART_DOCUMENT: &str = "word/document.xml";
/// `PresentationML` body part that lists slides in presentation order.
const PART_PRESENTATION: &str = "ppt/presentation.xml";
/// Relationships of the presentation part, used to map `r:id` to a slide.
const PART_PRESENTATION_RELS: &str = "ppt/_rels/presentation.xml.rels";
/// `DrawingML` slide part prefix.
const PART_SLIDE_PREFIX: &str = "ppt/slides/slide";
/// `DrawingML` slide part suffix.
const PART_SLIDE_SUFFIX: &str = ".xml";
/// Default package core properties (title, author).
const PART_CORE_PROPS: &str = "docProps/core.xml";
/// Default package extended properties (producing application).
const PART_APP_PROPS: &str = "docProps/app.xml";

/// Extract signal from an OOXML container, dispatching on the body part found.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when the ZIP is malformed or when the
/// container holds no recognised body part, and `AtomwriteError::FileTooLarge`
/// when a part read from the archive exceeds `max_part`.
fn extract_office(
    path: &std::path::Path,
    max_sample_rows: usize,
    max_text_chars: usize,
    max_part: u64,
    output: &mut ArtifactOutput,
) -> Result<()> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut archive = ZipArchive::new(BufReader::new(file))
        .map_err(|e| zip_failure(path, &e, "cannot read the OOXML container"))?;
    // Members that reached the `max_part` ceiling: the XML parsed from them may
    // be a prefix, so the signal says so instead of passing it as whole.
    let mut truncated_parts = 0u64;

    // zip 9 reports an unreadable entry name as an error instead of hiding it.
    let names: Vec<String> = archive
        .file_names()
        .map(|name| name.map(std::borrow::Cow::into_owned))
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| zip_failure(path, &e, "cannot list the OOXML container parts"))?;

    // Metadata is optional in the OOXML package: a missing or malformed
    // `docProps` part degrades to `None` rather than failing the extraction.
    if names.iter().any(|n| n == PART_CORE_PROPS) {
        if let Ok((xml, cut)) = read_part(&mut archive, PART_CORE_PROPS, path, max_part) {
            let (title, author) = parse_core_props(&xml);
            output.title = title;
            output.author = author;
            if cut {
                truncated_parts += 1;
            }
        }
    }
    if names.iter().any(|n| n == PART_APP_PROPS) {
        if let Ok((xml, cut)) = read_part(&mut archive, PART_APP_PROPS, path, max_part) {
            output.producer = parse_app_props(&xml);
            if cut {
                truncated_parts += 1;
            }
        }
    }

    if names.iter().any(|n| n == PART_DOCUMENT) {
        output.kind = "document";
        output.format = "ooxml-docx";
        let (xml, cut) = read_part(&mut archive, PART_DOCUMENT, path, max_part)?;
        truncated_parts += u64::from(cut);
        let signal = parse_document(&xml, max_text_chars)?;
        output.paragraphs = Some(signal.paragraphs);
        output.text_chars = Some(signal.text_chars);
        output.text_excerpt = Some(signal.excerpt);
        output.truncated_parts = (truncated_parts > 0).then_some(truncated_parts);
        return Ok(());
    }

    if names.iter().any(|n| n == PART_WORKBOOK) {
        output.kind = "spreadsheet";
        output.format = "ooxml-xlsx";
        let shared = if names.iter().any(|n| n == PART_SHARED_STRINGS) {
            let (xml, cut) = read_part(&mut archive, PART_SHARED_STRINGS, path, max_part)?;
            truncated_parts += u64::from(cut);
            parse_shared_strings(&xml)
        } else {
            Vec::new()
        };
        let (workbook_xml, workbook_cut) =
            read_part(&mut archive, PART_WORKBOOK, path, max_part)?;
        truncated_parts += u64::from(workbook_cut);
        let sheet_names = parse_workbook(&workbook_xml);
        output.sheets = Some(read_sheets(
            &mut archive,
            &names,
            path,
            &shared,
            &sheet_names,
            &mut SheetScan {
                max_part,
                max_sample_rows,
                truncated_parts: &mut truncated_parts,
            },
        ));
        output.truncated_parts = (truncated_parts > 0).then_some(truncated_parts);
        return Ok(());
    }

    if names.iter().any(|n| n == PART_PRESENTATION) {
        output.kind = "presentation";
        output.format = "ooxml-pptx";
        let (presentation_xml, presentation_cut) =
            read_part(&mut archive, PART_PRESENTATION, path, max_part)?;
        let (rels_xml, rels_cut) =
            read_part(&mut archive, PART_PRESENTATION_RELS, path, max_part)?;
        truncated_parts += u64::from(presentation_cut) + u64::from(rels_cut);
        let order = parse_slide_order(&presentation_xml, &rels_xml);
        output.slides = Some(read_slides(
            &mut archive,
            &names,
            path,
            max_part,
            &order,
            &mut truncated_parts,
        )?);
        output.truncated_parts = (truncated_parts > 0).then_some(truncated_parts);
        return Ok(());
    }

    Err(unsupported(
        path,
        "OOXML container without a recognised body part (expected docx, xlsx, or pptx)",
    ))
}

/// Translate a ZIP-layer failure into a domain error with a clear cause.
fn zip_failure(path: &std::path::Path, err: &ZipError, context: &str) -> anyhow::Error {
    match err {
        ZipError::InvalidArchive(detail) => corrupt(path, format!("{context}: {detail}")),
        ZipError::UnsupportedArchive(detail) => {
            unsupported(path, &format!("container variant not supported ({detail})"))
        }
        ZipError::InvalidPassword => corrupt(path, format!("{context}: entry is encrypted")),
        ZipError::FileNotFound => corrupt(path, format!("{context}: part not found in container")),
        ZipError::Io(io) => corrupt(path, format!("{context}: {io}")),
        // `ZipError` is `#[non_exhaustive]`: a future variant is reported as a
        // corrupt container rather than being silently guessed at.
        _ => corrupt(path, context),
    }
}

/// Read one archive member into memory, bounded by `max_part`.
///
/// The second value says the member reached the ceiling, so the text parsed
/// from it may be a prefix of the declared part. The bound itself does not
/// change: `declared` is attacker-controlled metadata.
fn read_part(
    archive: &mut ZipArchive<BufReader<File>>,
    name: &str,
    path: &std::path::Path,
    max_part: u64,
) -> Result<(String, bool)> {
    let entry = archive
        .by_name(name)
        .map_err(|e| zip_failure(path, &e, &format!("cannot open part {name}")))?;

    let declared = entry.size();
    if declared > max_part {
        return Err(AtomwriteError::FileTooLarge {
            path: path.to_path_buf(),
            size: declared,
            max_size: max_part,
        }
        .into());
    }

    let mut buf: Vec<u8> = Vec::new();
    // `take` is the real bound: `declared` is attacker-controlled metadata,
    // and a zip bomb lies about it.
    entry
        .take(max_part)
        .read_to_end(&mut buf)
        .map_err(|e| corrupt(path, format!("cannot inflate part {name}: {e}")))?;

    let reached_ceiling = buf.len() as u64 >= max_part;
    let text = String::from_utf8(buf)
        .map_err(|e| corrupt(path, format!("part {name} is not UTF-8 ({e})")))?;
    Ok((text, reached_ceiling))
}

/// Read `dc:title` and `dc:creator` from the package core properties.
fn parse_core_props(xml: &str) -> (Option<String>, Option<String>) {
    let mut reader = xml_reader(xml);
    let mut buffer = Vec::new();
    let mut title = None;
    let mut author = None;
    let mut current: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(e)) => {
                let name = local_name(e.name());
                current = if name == "title" || name == "creator" {
                    Some(name)
                } else {
                    None
                };
            }
            Ok(Event::Text(t)) => {
                if let Some(field) = current.as_deref()
                    && let Some(text) = text_of(&Event::Text(t))
                    && !text.is_empty()
                {
                    match field {
                        "title" => title = Some(text),
                        "creator" => author = Some(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => current = None,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    (title, author)
}

/// Read `Application` from the package extended properties.
fn parse_app_props(xml: &str) -> Option<String> {
    let mut reader = xml_reader(xml);
    let mut buffer = Vec::new();
    let mut inside = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(e)) => inside = local_name(e.name()) == "application",
            Ok(Event::Text(t)) if inside => {
                if let Some(text) = text_of(&Event::Text(t))
                    && !text.is_empty()
                {
                    return Some(text);
                }
            }
            Ok(Event::End(_)) => inside = false,
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
        buffer.clear();
    }
}

include!("office_docx.inc.rs");
include!("office_xlsx.inc.rs");
include!("office_pptx.inc.rs");
