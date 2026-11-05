// SPDX-License-Identifier: MIT OR Apache-2.0

//! The visual-token rule the vision models bill by.
//!
//! A picture costs one patch per 28 by 28 pixel cell, and each patch is one
//! visual token. The rule is arithmetic, so it lives here once and every verb
//! that reports or enforces a picture budget calls it instead of repeating it.

use crate::constants::{
    IMAGE_PATCH_PX, IMAGE_TIER_HIGH_MAX_EDGE, IMAGE_TIER_HIGH_MAX_TOKENS,
    IMAGE_TIER_STANDARD_MAX_EDGE, IMAGE_TIER_STANDARD_MAX_TOKENS,
};

/// Visual tokens a picture of this size costs, rounded up per axis.
pub(crate) fn visual_tokens(width: u32, height: u32) -> u64 {
    let patch = u64::from(IMAGE_PATCH_PX);
    let cols = u64::from(width).div_ceil(patch);
    let rows = u64::from(height).div_ceil(patch);
    cols * rows
}

/// Long-edge and token ceiling of the tier.
///
/// The caller picks the tier because the high-resolution one quadruples the
/// cost of the same picture, and that choice belongs to the request.
pub(crate) fn tier_limits(high_resolution: bool) -> (u32, u32) {
    if high_resolution {
        (IMAGE_TIER_HIGH_MAX_EDGE, IMAGE_TIER_HIGH_MAX_TOKENS)
    } else {
        (IMAGE_TIER_STANDARD_MAX_EDGE, IMAGE_TIER_STANDARD_MAX_TOKENS)
    }
}

/// Largest size with the same aspect ratio that fits both ceilings.
///
/// A picture already inside both ceilings keeps its size, so the common case
/// re-encodes the original pixels and never resamples them.
pub(crate) fn fit(width: u32, height: u32, max_edge: u32, max_tokens: u32) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (width, height);
    }
    let long_edge = width.max(height);
    let edge_scale = f64::from(max_edge) / f64::from(long_edge);
    let current = visual_tokens(width, height).max(1);
    let token_scale = (f64::from(max_tokens) / current as f64).sqrt();
    let scale = 1.0_f64.min(edge_scale).min(token_scale);
    if scale >= 1.0 {
        return (width, height);
    }
    let mut scaled_width = (f64::from(width) * scale).round().max(1.0) as u32;
    let mut scaled_height = (f64::from(height) * scale).round().max(1.0) as u32;
    // The token count rounds each axis UP to a whole patch, so one pass can
    // land a patch over the ceiling. Shrink until the rounded count fits; the
    // loop is bounded because each step removes at least one patch.
    for _ in 0..MAX_FIT_STEPS {
        let tokens = visual_tokens(scaled_width, scaled_height);
        if tokens <= u64::from(max_tokens) {
            break;
        }
        let factor = (f64::from(max_tokens) / tokens as f64).sqrt() * FIT_MARGIN;
        scaled_width = f64::from(scaled_width)
            .mul_add(factor, 0.0)
            .floor()
            .max(1.0) as u32;
        scaled_height = f64::from(scaled_height)
            .mul_add(factor, 0.0)
            .floor()
            .max(1.0) as u32;
    }
    (scaled_width, scaled_height)
}

/// Steps the token ceiling may take before the size is accepted as is.
const MAX_FIT_STEPS: usize = 8;
/// Safety margin of each correction step, so rounding never lands on the ceiling.
const FIT_MARGIN: f64 = 0.99;
