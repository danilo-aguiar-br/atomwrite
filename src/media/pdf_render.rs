// SPDX-License-Identifier: MIT OR Apache-2.0

//! Rasterise one PDF page, so a scanned document reaches the vision channel.
//!
//! A page whose text layer is empty has nothing to extract, and answering
//! `needs_vision` sends the caller back with no way to see it. This module
//! renders the page in process, and the pixel buffer then follows the same
//! resize and encode path a picture follows.

use std::path::Path;

use anyhow::Result;
use image::DynamicImage;

use crate::error::AtomwriteError;

/// Render page `page` (1-based) of `path` into a picture.
///
/// `dpi` decides the scale, because the PDF unit is 1/72 inch. The page is
/// refused before rendering when its pixel count would pass `max_pixels`, so a
/// hostile document cannot cost the machine its memory.
///
/// # Errors
///
/// Returns an error when the file is not a readable PDF, when the page does not
/// exist, and when the rendered size passes `max_pixels`.
pub(crate) fn render_page(
    path: &Path,
    page: u32,
    dpi: f32,
    max_pixels: u64,
) -> Result<DynamicImage> {
    let bytes = std::fs::read(path).map_err(|source| AtomwriteError::Io { source })?;
    let pdf = hayro_syntax::Pdf::new(bytes).map_err(|error| AtomwriteError::InvalidInput {
        reason: format!("read: {} is not a readable PDF: {error:?}", path.display()),
    })?;
    let pages = pdf.pages();
    let index = page.saturating_sub(1) as usize;
    let Some(selected) = pages.get(index) else {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "read: {} has {} page(s), so --page {page} does not exist",
                path.display(),
                pages.len()
            ),
        }
        .into());
    };
    let (base_width, base_height) = selected.render_dimensions();
    let scale = (dpi / 72.0).max(0.01);
    let width = (base_width * scale).round().max(1.0);
    let height = (base_height * scale).round().max(1.0);
    let pixels = u64::from(width as u32).saturating_mul(u64::from(height as u32));
    if pixels > max_pixels {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "read: page {page} of {} would render at {width} x {height} pixels, \
                 above the {max_pixels} pixel ceiling; lower --dpi",
                path.display()
            ),
        }
        .into());
    }

    let pixmap = hayro::render(
        selected,
        &hayro::RenderCache::new(),
        &hayro_interpret::InterpreterSettings::default(),
        &hayro::RenderSettings::default(),
        &hayro::PixmapSettings {
            x_scale: scale,
            y_scale: scale,
            ..hayro::PixmapSettings::default()
        },
    );
    let (width, height) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
    let mut rgb = Vec::with_capacity(pixmap.data_as_u8_slice().len() / 4 * 3);
    composite_over_white(pixmap.data_as_u8_slice(), &mut rgb);
    let buffer = image::RgbImage::from_raw(width, height, rgb).ok_or_else(|| {
        AtomwriteError::InternalError {
            reason: format!(
                "read: rendered page {page} of {} did not produce {width} x {height} pixels",
                path.display()
            ),
        }
    })?;
    Ok(DynamicImage::ImageRgb8(buffer))
}

/// Flatten a premultiplied RGBA8 page onto white.
///
/// A page has no viewer behind it, so transparency has to become a colour
/// before the picture leaves as a `data:` URI. White is what every PDF viewer
/// paints, and the missing alpha channel lets the page encode as JPEG instead
/// of a PNG several times larger.
fn composite_over_white(rgba: &[u8], rgb: &mut Vec<u8>) {
    rgb.reserve(rgba.len() / 4 * 3);
    for pixel in rgba.chunks_exact(4) {
        let rest = 255 - u32::from(pixel[3]);
        for channel in &pixel[..3] {
            let mixed = u32::from(*channel) + rest;
            rgb.push(mixed.min(255) as u8);
        }
    }
}
