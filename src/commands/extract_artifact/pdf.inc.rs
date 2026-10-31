// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by `extract_artifact/mod.rs` (E5 — PDF page and metadata signal).

use lopdf::{Document, Object};

/// Extract page count, page geometry and document information from a `PDF`.
///
/// Only the object model is read: page content streams are never decoded and
/// never forwarded. A `MediaBox` is reported for the first page, because that is
/// the figure an agent uses to decide printability.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when the file is not a parseable PDF
/// or when it is encrypted with a password this command cannot supply.
fn extract_pdf(path: &std::path::Path, _max_part: u64, output: &mut ArtifactOutput) -> Result<()> {
    let doc = Document::load(path).map_err(|e| {
        corrupt(
            path,
            format!("the PDF object model could not be parsed ({e})"),
        )
    })?;

    if doc.is_encrypted() {
        return Err(corrupt(
            path,
            "the PDF is encrypted; extract-artifact reads unencrypted documents only",
        ));
    }

    let pages = doc.get_pages();
    output.kind = "pdf";
    output.format = "pdf";
    output.pages = Some(pages.len() as u64);

    if let Some(page_id) = pages.values().next().copied()
        && let Some((width, height)) = page_media_box(&doc, page_id)
    {
        // PDF user-space units are points; pixels at 1x are not the same
        // quantity, so the field carries the point extent rounded to integers.
        output.width = Some(width.round().max(0.0) as u64);
        output.height = Some(height.round().max(0.0) as u64);
    }

    let (title, author, producer) = pdf_info(&doc);
    output.title = title;
    output.author = author;
    output.producer = producer;
    Ok(())
}

/// `MediaBox` extent for `page_id`, following `/Parent` for the inherited case.
fn page_media_box(doc: &Document, page_id: lopdf::ObjectId) -> Option<(f32, f32)> {
    let mut current = Some(page_id);
    // The page tree is shallow in practice; the bound is what stops a crafted
    // document whose `/Parent` chain is a cycle.
    for _ in 0..crate::constants::PDF_MEDIABOX_MAX_PARENT_HOPS {
        let id = current?;
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(Object::Array(bounds)) = dict.get(b"MediaBox")
            && let (Some(left), Some(bottom), Some(right), Some(top)) = (
                bounds.first().and_then(number),
                bounds.get(1).and_then(number),
                bounds.get(2).and_then(number),
                bounds.get(3).and_then(number),
            )
            && right > left
            && top > bottom
        {
            return Some((right - left, top - bottom));
        }
        current = match dict.get(b"Parent") {
            Ok(Object::Reference(parent)) => Some(*parent),
            _ => None,
        };
    }
    None
}

/// Read a numeric PDF object as `f32`, accepting both integer and real forms.
fn number(object: &Object) -> Option<f32> {
    match object {
        Object::Integer(value) => Some(*value as f32),
        Object::Real(value) => Some(*value),
        _ => None,
    }
}

/// `Title`, `Author` and `Producer` from the document information dictionary.
fn pdf_info(doc: &Document) -> (Option<String>, Option<String>, Option<String>) {
    let Some(dict) = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|object| doc.dereference(object).ok().map(|(_, resolved)| resolved))
        .and_then(|object| object.as_dict().ok())
    else {
        return (None, None, None);
    };

    let field = |key: &[u8]| {
        dict.get(key)
            .ok()
            .and_then(|value| lopdf::decode_text_string(value).ok())
            .map(|text| truncate_chars(text.trim(), CELL_CHAR_CAP))
            .filter(|text| !text.is_empty())
    };

    (field(b"Title"), field(b"Author"), field(b"Producer"))
}
