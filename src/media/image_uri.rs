// SPDX-License-Identifier: MIT OR Apache-2.0

//! Pictures for the vision channel.
//!
//! A model that receives raw PNG bytes pays for them and sees nothing. This
//! module measures the picture with the official token rule, applies the
//! camera orientation the decoder ignores, resizes only when the tier demands
//! it, and encodes the result as a `data:` URI. The URI is the whole payload:
//! one line, no progress, no filename.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use anyhow::{Context, Result};
use base64::Engine;
use fast_image_resize::FilterType as ResizeFilter;
use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::imageops::FilterType;
use image::{ColorType, DynamicImage, ImageFormat, ImageReader};

use crate::constants::{IMAGE_JPEG_QUALITY, IMAGE_MAX_MEGAPIXELS, IMAGE_ORIGINAL_MAX_BYTES};
use crate::error::AtomwriteError;
use crate::media::token_rule;

/// What a caller can ask of one picture.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RenderRequest {
    /// Use the high-resolution tier instead of the standard one.
    pub high_resolution: bool,
    /// Hard ceiling on visual tokens, overrides the tier when present.
    pub max_tokens: Option<u32>,
    /// Crop window as `(x, y, width, height)` in source pixels, before resize.
    pub crop: Option<(u32, u32, u32, u32)>,
}

/// Facts about a picture, measured without sending its pixels anywhere.
#[derive(Debug, Clone)]
pub(crate) struct ImageFacts {
    /// Width after the camera orientation is applied.
    pub width: u32,
    /// Height after the camera orientation is applied.
    pub height: u32,
    /// Stable format name, from the magic bytes.
    pub format: &'static str,
    /// Visual tokens the delivered size costs.
    pub visual_tokens: u64,
    /// Frames when the container carries an animation, `None` otherwise.
    pub frames: Option<u32>,
    /// EXIF orientation value that was applied, when the tag was present.
    pub orientation: Option<u16>,
}

/// Read the header, the EXIF orientation and the frame count of a picture.
///
/// # Errors
///
/// Returns an error when the file cannot be opened or its header cannot be
/// decoded. No pixel data is decoded here.
pub(crate) fn facts(
    path: &Path,
    format: ImageFormat,
    request: &RenderRequest,
) -> Result<ImageFacts> {
    let (source_width, source_height) = ImageReader::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("cannot sniff {}", path.display()))?
        .into_dimensions()
        .map_err(|error| {
            anyhow::Error::from(AtomwriteError::InvalidInput {
                reason: format!("corrupt image {}: {error}", path.display()),
            })
        })?;
    guard_megapixels(source_width, source_height, path)?;
    let orientation = exif_orientation(path);
    let (width, height) = apply_orientation_to_size(source_width, source_height, orientation);
    let (width, height) = crop_size(width, height, request.crop);
    let (max_edge, max_tokens) = token_rule::tier_limits(request.high_resolution);
    let max_tokens = request.max_tokens.unwrap_or(max_tokens);
    let (width, height) = token_rule::fit(width, height, max_edge, max_tokens);
    Ok(ImageFacts {
        width,
        height,
        format: crate::media::format_name(format),
        visual_tokens: token_rule::visual_tokens(width, height),
        frames: frame_count(path, format),
        orientation,
    })
}

/// Decode, orient, crop, resize and encode the picture as a `data:` URI.
///
/// # Errors
///
/// Returns an error when the picture cannot be decoded, when it would exceed
/// the megapixel guard, or when the re-encode produces more bytes than the
/// caller allows.
pub(crate) fn to_data_uri(
    path: &Path,
    format: ImageFormat,
    request: &RenderRequest,
) -> Result<(String, ImageFacts)> {
    let (source_width, source_height) = ImageReader::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("cannot sniff {}", path.display()))?
        .into_dimensions()
        .map_err(|error| {
            anyhow::Error::from(AtomwriteError::InvalidInput {
                reason: format!("corrupt image {}: {error}", path.display()),
            })
        })?;
    guard_megapixels(source_width, source_height, path)?;
    // The original bytes already fit the tier: send them unchanged and skip a
    // decode that would only reproduce the same pixels.
    if fits_as_sent(path, source_width, source_height, request)? {
        let facts = facts(path, format, request)?;
        let bytes =
            std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
        let uri = data_uri(facts.format, &bytes);
        return Ok((uri, facts));
    }
    let mut picture = ImageReader::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("cannot sniff {}", path.display()))?
        .decode()
        .map_err(|error| {
            anyhow::Error::from(AtomwriteError::InvalidInput {
                reason: format!("corrupt image {}: {error}", path.display()),
            })
        })?;
    let orientation = exif_orientation(path);
    picture = apply_orientation(picture, orientation);
    if let Some((x, y, width, height)) = request.crop {
        picture = crop(picture, x, y, width, height);
    }
    let (max_edge, max_tokens) = token_rule::tier_limits(request.high_resolution);
    let max_tokens = request.max_tokens.unwrap_or(max_tokens);
    let (width, height) = token_rule::fit(picture.width(), picture.height(), max_edge, max_tokens);
    if (width, height) != (picture.width(), picture.height()) {
        picture = resize_lanczos3(picture, width, height);
    }
    let transparent = picture.color().has_alpha();
    let (mime, bytes) = encode(&picture, transparent)?;
    let facts = ImageFacts {
        width,
        height,
        format: if transparent { "png" } else { "jpeg" },
        visual_tokens: token_rule::visual_tokens(width, height),
        frames: frame_count(path, format),
        orientation,
    };
    Ok((data_uri(mime, &bytes), facts))
}

/// Resize, encode and wrap a picture that came from somewhere else.
///
/// A rendered PDF page arrives here with pixels already decoded, so the resize
/// rule and the encoder stay in one place instead of being copied beside it.
///
/// # Errors
///
/// Returns an error when the encoder cannot produce the picture.
pub(crate) fn from_picture(
    picture: DynamicImage,
    request: &RenderRequest,
) -> Result<(String, ImageFacts)> {
    let picture = match request.crop {
        Some((x, y, crop_width, crop_height)) => crop(picture, x, y, crop_width, crop_height),
        None => picture,
    };
    let (max_edge, max_tokens) = token_rule::tier_limits(request.high_resolution);
    let max_tokens = request.max_tokens.unwrap_or(max_tokens);
    let (width, height) = token_rule::fit(picture.width(), picture.height(), max_edge, max_tokens);
    let picture = if (width, height) != (picture.width(), picture.height()) {
        resize_lanczos3(picture, width, height)
    } else {
        picture
    };
    let transparent = picture.color().has_alpha();
    let (mime, bytes) = encode(&picture, transparent)?;
    let facts = ImageFacts {
        width,
        height,
        format: if transparent { "png" } else { "jpeg" },
        visual_tokens: token_rule::visual_tokens(width, height),
        frames: None,
        orientation: None,
    };
    Ok((data_uri(mime, &bytes), facts))
}

/// Resize a picture with the SIMD engine, keeping its own color type.
///
/// The `image` crate took 625 ms to take a 6000x4000 picture down to 1441x961
/// on this machine, and the SIMD engine took 42 ms for the same output size.
/// A color type the engine does not carry still goes through `image`, so no
/// picture loses a path it already had.
fn resize_lanczos3(picture: DynamicImage, width: u32, height: u32) -> DynamicImage {
    let (source_width, source_height) = (picture.width(), picture.height());
    let (buffer, pixel_type) = match picture.color() {
        ColorType::L8 => (picture.to_luma8().into_raw(), PixelType::U8),
        ColorType::La8 => (picture.to_luma_alpha8().into_raw(), PixelType::U8x2),
        ColorType::Rgb8 => (picture.to_rgb8().into_raw(), PixelType::U8x3),
        ColorType::Rgba8 => (picture.to_rgba8().into_raw(), PixelType::U8x4),
        _ => return picture.resize_exact(width, height, FilterType::Lanczos3),
    };
    let Ok(source) = Image::from_vec_u8(source_width, source_height, buffer, pixel_type) else {
        return picture.resize_exact(width, height, FilterType::Lanczos3);
    };
    let mut target = Image::new(width, height, pixel_type);
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(ResizeFilter::Lanczos3));
    if Resizer::new()
        .resize(&source, &mut target, &options)
        .is_err()
    {
        return picture.resize_exact(width, height, FilterType::Lanczos3);
    }
    let buffer = target.into_vec();
    let restored = match pixel_type {
        PixelType::U8 => {
            image::GrayImage::from_raw(width, height, buffer).map(DynamicImage::ImageLuma8)
        }
        PixelType::U8x2 => {
            image::GrayAlphaImage::from_raw(width, height, buffer).map(DynamicImage::ImageLumaA8)
        }
        PixelType::U8x3 => {
            image::RgbImage::from_raw(width, height, buffer).map(DynamicImage::ImageRgb8)
        }
        _ => image::RgbaImage::from_raw(width, height, buffer).map(DynamicImage::ImageRgba8),
    };
    restored.unwrap_or_else(|| picture.resize_exact(width, height, FilterType::Lanczos3))
}

/// Resize a picture at its own resolution, oriented and cropped.
///
/// Text recognition reads glyphs, so it never sees a resized page: the resize
/// rule belongs to the vision channel, and shrinking here would cost accuracy
/// for a saving nobody asked for.
///
/// # Errors
///
/// Returns an error when the picture cannot be decoded or would exceed the
/// megapixel guard.
pub(crate) fn decode_for_ocr(path: &Path, request: &RenderRequest) -> Result<DynamicImage> {
    let (width, height) = ImageReader::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("cannot sniff {}", path.display()))?
        .into_dimensions()
        .map_err(|error| {
            anyhow::Error::from(AtomwriteError::InvalidInput {
                reason: format!("corrupt image {}: {error}", path.display()),
            })
        })?;
    guard_megapixels(width, height, path)?;
    let mut picture = ImageReader::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("cannot sniff {}", path.display()))?
        .decode()
        .map_err(|error| {
            anyhow::Error::from(AtomwriteError::InvalidInput {
                reason: format!("corrupt image {}: {error}", path.display()),
            })
        })?;
    picture = apply_orientation(picture, exif_orientation(path));
    if let Some((x, y, crop_width, crop_height)) = request.crop {
        picture = crop(picture, x, y, crop_width, crop_height);
    }
    Ok(picture)
}

/// Bytes of a re-encoded picture that the caller caps.
///
/// # Errors
///
/// Returns an error when the encoded picture exceeds `max_bytes`.
pub(crate) fn guard_size(uri: &str, max_bytes: usize) -> Result<()> {
    if uri.len() > max_bytes {
        anyhow::bail!(
            "the encoded picture is {} bytes, above the {} byte ceiling; ask for a smaller --max-tokens or a --crop",
            uri.len(),
            max_bytes
        );
    }
    Ok(())
}

/// The `data:` URI for a payload and its mime subtype.
fn data_uri(subtype: &str, bytes: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:image/{subtype};base64,{encoded}")
}

/// Refuse a picture whose pixel count would exhaust memory when decoded.
fn guard_megapixels(width: u32, height: u32, path: &Path) -> Result<()> {
    let pixels = u64::from(width) * u64::from(height);
    let ceiling = IMAGE_MAX_MEGAPIXELS * 1_000_000;
    if pixels > ceiling {
        return Err(anyhow::Error::from(AtomwriteError::InvalidInput {
            reason: format!(
                "{} is {}x{} pixels, above the {} pixel guard; crop it first",
                path.display(),
                width,
                height,
                ceiling
            ),
        }));
    }
    Ok(())
}

/// Whether the file can be delivered byte for byte without a resize.
fn fits_as_sent(path: &Path, width: u32, height: u32, request: &RenderRequest) -> Result<bool> {
    if request.crop.is_some() {
        return Ok(false);
    }
    let size = std::fs::metadata(path)
        .with_context(|| format!("cannot stat {}", path.display()))?
        .len();
    if size > IMAGE_ORIGINAL_MAX_BYTES {
        return Ok(false);
    }
    let (max_edge, max_tokens) = token_rule::tier_limits(request.high_resolution);
    let max_tokens = request.max_tokens.unwrap_or(max_tokens);
    Ok(width.max(height) <= max_edge
        && token_rule::visual_tokens(width, height) <= u64::from(max_tokens))
}

/// Frame count for an animated GIF, and `None` for every still container.
///
/// WebP is left out on purpose: the decoder in this build reads one frame, so
/// a number derived from it would claim the animation was counted when it was
/// not.
fn frame_count(path: &Path, format: ImageFormat) -> Option<u32> {
    if format != ImageFormat::Gif {
        return None;
    }
    use image::AnimationDecoder;
    let bytes = std::fs::read(path).ok()?;
    let reader = image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).ok()?;
    Some(reader.into_frames().count() as u32)
}

/// EXIF orientation tag, when the container carries one.
fn exif_orientation(path: &Path) -> Option<u16> {
    use exif::{In, Reader, Tag};
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut reader).ok()?;
    let field = exif.get_field(Tag::Orientation, In::PRIMARY)?;
    field.value.get_uint(0).map(|v| v as u16)
}

/// Size of the picture as the camera meant it to be shown.
fn apply_orientation_to_size(width: u32, height: u32, orientation: Option<u16>) -> (u32, u32) {
    match orientation {
        // 5 to 8 swap the axes: the stored picture is rotated by a quarter turn.
        Some(5..=8) => (height, width),
        _ => (width, height),
    }
}

/// Rotate and flip decoded pixels into the orientation the camera recorded.
fn apply_orientation(picture: DynamicImage, orientation: Option<u16>) -> DynamicImage {
    match orientation {
        Some(2) => picture.fliph(),
        Some(3) => picture.rotate180(),
        Some(4) => picture.flipv(),
        Some(5) => picture.rotate90().fliph(),
        Some(6) => picture.rotate90(),
        Some(7) => picture.rotate270().fliph(),
        Some(8) => picture.rotate270(),
        _ => picture,
    }
}

/// Cut the requested window out of the decoded picture, clamped to its bounds.
fn crop(picture: DynamicImage, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
    let x = x.min(picture.width().saturating_sub(1));
    let y = y.min(picture.height().saturating_sub(1));
    let width = width.min(picture.width() - x).max(1);
    let height = height.min(picture.height() - y).max(1);
    picture.crop_imm(x, y, width, height)
}

/// Size of the crop window, clamped to the picture bounds.
fn crop_size(width: u32, height: u32, window: Option<(u32, u32, u32, u32)>) -> (u32, u32) {
    let Some((x, y, crop_width, crop_height)) = window else {
        return (width, height);
    };
    let x = x.min(width.saturating_sub(1));
    let y = y.min(height.saturating_sub(1));
    (
        crop_width.min(width - x).max(1),
        crop_height.min(height - y).max(1),
    )
}

/// Encode the picture, keeping transparency in PNG and compressing the rest as JPEG.
fn encode(picture: &DynamicImage, transparent: bool) -> Result<(&'static str, Vec<u8>)> {
    let mut bytes = Vec::new();
    if transparent {
        picture
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .context("cannot encode the picture as PNG")?;
        return Ok(("png", bytes));
    }
    // JPEG has no alpha, so a transparent pixel keeps its colour and loses its
    // transparency. Dropping the channel is honest; compositing onto white
    // would invent pixels the caller never had.
    let rgb = picture.to_rgb8();
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, IMAGE_JPEG_QUALITY);
    encoder
        .encode_image(&rgb)
        .context("cannot encode the picture as JPEG")?;
    Ok(("jpeg", bytes))
}

/// Read the size of a HEIC picture without decoding its pixels.
///
/// # Errors
///
/// Returns an error when the container is truncated or carries no valid extent.
pub(crate) fn heic_size(path: &Path) -> Result<(u32, u32)> {
    let data = std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let info = heic::ImageInfo::from_bytes(&data).map_err(|error| {
        anyhow::Error::from(AtomwriteError::InvalidInput {
            reason: format!("corrupt image {}: {error}", path.display()),
        })
    })?;
    Ok((info.width, info.height))
}

/// Decode a HEIC picture into the pixels the rest of the pipeline expects.
///
/// # Errors
///
/// Returns an error when the file cannot be read or the HEVC stream is refused.
pub(crate) fn decode_heic(path: &Path) -> Result<DynamicImage> {
    let data = std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let info = heic::ImageInfo::from_bytes(&data).map_err(|error| {
        anyhow::Error::from(AtomwriteError::InvalidInput {
            reason: format!("corrupt image {}: {error}", path.display()),
        })
    })?;
    guard_megapixels(info.width, info.height, path)?;
    let output = heic::DecoderConfig::new()
        .decode(&data, heic::PixelLayout::Rgba8)
        .map_err(|error| {
            anyhow::Error::from(AtomwriteError::InvalidInput {
                reason: format!("corrupt image {}: {error}", path.display()),
            })
        })?;
    let buffer =
        image::RgbaImage::from_raw(output.width, output.height, output.data).ok_or_else(|| {
            anyhow::Error::from(AtomwriteError::InvalidInput {
                reason: format!(
                    "corrupt image {}: decoded buffer does not match {}x{}",
                    path.display(),
                    output.width,
                    output.height
                ),
            })
        })?;
    let picture = DynamicImage::ImageRgba8(buffer);
    // The decoder always yields four channels. A picture without alpha would
    // then be encoded as PNG, which costs several times the JPEG it replaced.
    Ok(if info.has_alpha {
        picture
    } else {
        DynamicImage::ImageRgb8(picture.to_rgb8())
    })
}

/// Marker path of one delivered picture under the current agent session.
///
/// The session names a directory and the picture's BLAKE3 names the file, so
/// the same bytes read twice in one session leave the second call with the
/// facts instead of the whole `data:` URI. Without a session, or without a
/// cache directory, nothing is recorded and every read delivers the bytes.
fn session_marker(hash: &str) -> Option<std::path::PathBuf> {
    let session = std::env::var(crate::constants::SESSION_ENV_VAR).ok()?;
    if session.is_empty() {
        return None;
    }
    let safe: String = session
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Some(
        crate::storage::cache_dir()?
            .join(crate::constants::MEDIA_SESSION_DIR_NAME)
            .join(safe)
            .join(hash),
    )
}

/// Whether this session already received the picture whose bytes hash to `hash`.
pub(crate) fn delivered_in_session(hash: &str) -> bool {
    session_marker(hash).is_some_and(|path| path.is_file())
}

/// Record that this session received the picture, so the repeat is not paid.
pub(crate) fn mark_delivered(hash: &str) {
    let Some(path) = session_marker(hash) else {
        return;
    };
    if let Some(parent) = path.parent() {
        crate::best_effort::best_effort(
            std::fs::create_dir_all(parent),
            "create the picture session directory",
        );
    }
    crate::best_effort::best_effort(
        std::fs::write(path, b""),
        "mark the picture as delivered in this session",
    );
}
