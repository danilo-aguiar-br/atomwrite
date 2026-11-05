// SPDX-License-Identifier: MIT OR Apache-2.0

//! Media the text channel cannot carry.
//!
//! The verbs that read files answer in text. A picture has no text, and a PDF
//! has text on pages, so this module does the decoding work the model would
//! otherwise do badly: it names the type from the magic bytes, it measures the
//! picture with the official token rule, it turns the picture into a data URI
//! the vision channel accepts, and it pulls the text out of a PDF page by page.
//! Nothing here writes to stdout; the caller owns the channel.

pub(crate) mod image_uri;
pub(crate) mod ocr;
pub(crate) mod pdf_render;
pub(crate) mod pdf_text;
pub(crate) mod token_rule;

use std::path::Path;

/// What the magic bytes of a file say it is.
///
/// The extension is never consulted: a PNG named `.txt` is a picture, and a
/// text file named `.png` is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MediaKind {
    /// A bitmap this binary can decode and re-encode.
    Image(image::ImageFormat),
    /// A HEIC/HEIF picture, decoded by the `heic` crate.
    Heic,
    /// A PDF document.
    Pdf,
}

/// Classify a path by its leading bytes.
///
/// Returns `None` for text, for an empty file, and for a binary this module
/// does not decode. A read error is reported as `None` here because the caller
/// already answers for a missing or unreadable path with its own error.
pub(crate) fn classify(path: &Path) -> Option<MediaKind> {
    let kind = infer::get_from_path(path).ok()??;
    match kind.mime_type() {
        "application/pdf" => Some(MediaKind::Pdf),
        // The `image` crate carries no HEVC decoder, so the container gets its
        // own variant instead of being refused as an unknown image.
        "image/heic" | "image/heif" | "image/heic-sequence" | "image/heif-sequence" => {
            Some(MediaKind::Heic)
        }
        mime if mime.starts_with("image/") => {
            image_format_of(kind.extension()).map(MediaKind::Image)
        }
        _ => None,
    }
}

/// Translate the sniffed extension into the decoder that will open it.
fn image_format_of(extension: &str) -> Option<image::ImageFormat> {
    Some(match extension {
        "png" => image::ImageFormat::Png,
        "jpg" | "jpeg" => image::ImageFormat::Jpeg,
        "webp" => image::ImageFormat::WebP,
        "gif" => image::ImageFormat::Gif,
        "bmp" => image::ImageFormat::Bmp,
        "tif" | "tiff" => image::ImageFormat::Tiff,
        "ico" => image::ImageFormat::Ico,
        _ => return None,
    })
}

/// Stable name of a decoded format, for the envelope.
pub(crate) fn format_name(format: image::ImageFormat) -> &'static str {
    match format {
        image::ImageFormat::Png => "png",
        image::ImageFormat::Jpeg => "jpeg",
        image::ImageFormat::WebP => "webp",
        image::ImageFormat::Gif => "gif",
        image::ImageFormat::Bmp => "bmp",
        image::ImageFormat::Tiff => "tiff",
        image::ImageFormat::Ico => "ico",
        _ => "unknown",
    }
}
