// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by `extract_artifact/mod.rs` (E5 — image geometry, header only).

/// Read image dimensions without decoding a single pixel.
///
/// The codec is identified from the leading bytes of `head`. The extent is then
/// measured straight from the file, because the offset that carries the
/// dimensions differs per codec: PNG and BMP keep them near the front, while
/// TIFF follows an IFD pointer that can point anywhere in the file. A fixed
/// head probe would therefore either miss TIFF entirely or report a bogus
/// extent, so the reader is allowed to seek. No pixel data is ever decoded.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when the header names no supported
/// codec, when the dimension fields are truncated, or when they declare a
/// zero or inconsistent extent.
fn extract_image(path: &std::path::Path, head: &[u8], output: &mut ArtifactOutput) -> Result<()> {
    let format = imagesize::image_type(head)
        .map_err(|e| corrupt(path, format!("image header not recognised ({e})")))?;
    let size = imagesize::size(path)
        .map_err(|e| corrupt(path, format!("image header cannot be measured ({e})")))?;

    if size.width == 0 || size.height == 0 {
        return Err(corrupt(
            path,
            format!(
                "image header declares an empty extent ({}x{})",
                size.width, size.height
            ),
        ));
    }

    output.kind = "image";
    output.format = image_format_name(format);
    output.width = Some(size.width as u64);
    output.height = Some(size.height as u64);
    output.megapixels = Some(megapixels(size.width as u64, size.height as u64));
    // The header extent is not what a vision call receives: the standard tier
    // resizes anything above its ceilings. Reporting both is what lets the
    // caller cost the call before making it.
    let (max_edge, max_tokens) = crate::media::token_rule::tier_limits(false);
    let (final_width, final_height) = crate::media::token_rule::fit(
        size.width as u32,
        size.height as u32,
        max_edge,
        max_tokens,
    );
    output.width_final = Some(u64::from(final_width));
    output.height_final = Some(u64::from(final_height));
    output.visual_tokens = Some(crate::media::token_rule::visual_tokens(
        final_width,
        final_height,
    ));
    Ok(())
}

/// DERIVED: `width * height / IMAGE_PIXELS_PER_MEGAPIXEL`, rounded to three decimals.
fn megapixels(width: u64, height: u64) -> f64 {
    use crate::constants::{IMAGE_MEGAPIXEL_SCALE as SCALE, IMAGE_PIXELS_PER_MEGAPIXEL as PIXELS};
    let total = u128::from(width) * u128::from(height);
    let scaled = (total * SCALE + PIXELS / 2) / PIXELS;
    scaled as f64 / SCALE as f64
}

/// Stable lowercase name for the codec detected from the magic bytes.
fn image_format_name(format: imagesize::ImageType) -> &'static str {
    match format {
        imagesize::ImageType::Png => "png",
        imagesize::ImageType::Jpeg => "jpeg",
        imagesize::ImageType::Gif => "gif",
        imagesize::ImageType::Bmp => "bmp",
        imagesize::ImageType::Webp => "webp",
        imagesize::ImageType::Tiff => "tiff",
        imagesize::ImageType::Ico => "ico",
        _ => "image",
    }
}
