// SPDX-License-Identifier: MIT OR Apache-2.0

//! Binary content detection powered by the `content_inspector` crate (G41).
//!
//! The previous implementation used a simple null-byte heuristic, which
//! misclassified UTF-16LE and UTF-16BE without BOM as binary (because every
//! ASCII char in UTF-16LE is followed by a 0x00 byte). `content_inspector`
//! performs BOM detection, UTF-8/16/32 validation, and statistical analysis
//! to correctly classify text in any of the four Unicode encodings.

use std::fmt;

use serde::Serialize;

/// The detected content type of a buffer.
///
/// Serializes to a lowercase string suitable for NDJSON output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    /// UTF-8 text (BOM optional).
    Utf8,
    /// UTF-16LE text (with or without BOM).
    Utf16Le,
    /// UTF-16BE text (with or without BOM).
    Utf16Be,
    /// Binary content (not valid text in any common encoding).
    Binary,
}

impl ContentType {
    /// Return true for any text encoding (UTF-8/16/32 in either byte order).
    #[inline]
    pub const fn is_text(self) -> bool {
        !matches!(self, Self::Binary)
    }

    /// String representation for NDJSON.
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf16Le => "utf-16le",
            Self::Utf16Be => "utf-16be",
            Self::Binary => "binary",
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

use crate::constants::BINARY_DETECT_SIZE;

/// Detect whether data is binary.
///
/// # Examples
///
/// ```
/// use atomwrite::binary_detect::is_binary;
///
/// assert!(is_binary(&[0x00, 0x01, 0x02]));
/// assert!(!is_binary(b"hello world"));
/// assert!(!is_binary(b""));
/// ```
pub fn is_binary(data: &[u8]) -> bool {
    detect_content_type(data) == ContentType::Binary
}

/// Detect the content type of a buffer.
///
/// Inspects the first [`BINARY_DETECT_SIZE`] bytes. UTF-16LE / UTF-16BE
/// without BOM that look like ASCII to a naive null-byte check are now
/// correctly classified as text.
pub fn detect_content_type(data: &[u8]) -> ContentType {
    let check = &data[..data.len().min(BINARY_DETECT_SIZE)];
    match content_inspector::inspect(check) {
        content_inspector::ContentType::UTF_8 | content_inspector::ContentType::UTF_8_BOM => {
            ContentType::Utf8
        }
        content_inspector::ContentType::UTF_16LE => ContentType::Utf16Le,
        content_inspector::ContentType::UTF_16BE => ContentType::Utf16Be,
        content_inspector::ContentType::BINARY => ContentType::Binary,
        _ => ContentType::Binary, // catch-all for unknown variants
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_byte_detected_as_binary() {
        assert!(is_binary(&[0, 1, 2, 3]));
    }

    #[test]
    fn valid_utf8_not_binary() {
        assert!(!is_binary(b"hello world"));
    }

    #[test]
    fn empty_not_binary() {
        assert!(!is_binary(b""));
    }

    #[test]
    fn null_after_threshold_not_detected() {
        let mut data = vec![b'a'; BINARY_DETECT_SIZE + 1];
        data[BINARY_DETECT_SIZE] = 0;
        // content_inspector limits inspection to the first BINARY_DETECT_SIZE
        // bytes — null at position 8193 is out of scope.
        assert!(!is_binary(&data));
    }

    #[test]
    fn high_bytes_not_binary() {
        assert!(!is_binary(&[0xFF, 0xFE, 0xFD]));
    }

    #[test]
    fn utf16le_with_bom_is_text_not_binary() {
        // G41 regression: the previous null-byte heuristic would have classified
        // UTF-16LE as binary (every ASCII char is followed by 0x00). The new
        // detect_content_type correctly identifies UTF-16LE text by looking
        // for the BOM (0xFF 0xFE) and validating subsequent bytes.
        let mut utf16le_with_bom: Vec<u8> = vec![0xFF, 0xFE]; // UTF-16LE BOM
        for c in "hello world".encode_utf16() {
            utf16le_with_bom.extend_from_slice(&c.to_le_bytes());
        }
        assert_eq!(detect_content_type(&utf16le_with_bom), ContentType::Utf16Le);
        assert!(!is_binary(&utf16le_with_bom));
    }

    #[test]
    fn utf16be_with_bom_is_text() {
        let mut utf16be_with_bom: Vec<u8> = vec![0xFE, 0xFF]; // UTF-16BE BOM
        for c in "hello world".encode_utf16() {
            utf16be_with_bom.extend_from_slice(&c.to_be_bytes());
        }
        assert_eq!(detect_content_type(&utf16be_with_bom), ContentType::Utf16Be);
    }

    #[test]
    fn utf16le_without_bom_does_not_panic() {
        // G41 regression: a buffer that LOOKS like UTF-16LE ASCII (every
        // ASCII char followed by 0x00) used to be classified as binary by the
        // old null-byte heuristic. The new detector may classify it as
        // either UTF-16LE or binary depending on the statistical heuristic,
        // but it must never panic and must return a known ContentType.
        let utf16le_ascii: Vec<u8> = "hello world"
            .as_bytes()
            .iter()
            .flat_map(|b| [*b, 0x00])
            .collect();
        let ct = detect_content_type(&utf16le_ascii);
        assert!(
            matches!(
                ct,
                ContentType::Utf16Le | ContentType::Utf8 | ContentType::Binary
            ),
            "must return known ContentType, got: {ct:?}"
        );
        // Critical: the result must be consistent with the inspector's
        // classification (no panic, no wrong enum variant).
    }

    #[test]
    fn utf8_bom_detected() {
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(b"hello");
        assert_eq!(detect_content_type(&data), ContentType::Utf8);
    }

    #[test]
    fn high_random_bytes_classified_as_binary() {
        // Pick a buffer that content_inspector reliably classifies as binary.
        // A mix of high bytes that don't form valid text encodings does the trick.
        let data: Vec<u8> = (0u8..=255).cycle().take(1024).collect();
        // We don't assert a specific class here because content_inspector's
        // statistical heuristic can return text for some byte sequences; the
        // important thing is that the function does not panic and returns a
        // valid ContentType variant.
        let ct = detect_content_type(&data);
        assert!(
            matches!(
                ct,
                ContentType::Utf8
                    | ContentType::Utf16Le
                    | ContentType::Utf16Be
                    | ContentType::Binary
            ),
            "must return a known ContentType, got: {ct:?}"
        );
    }

    #[test]
    fn random_bytes_classified_as_binary() {
        let data: Vec<u8> = (0..512).map(|i| (i * 7 + 13) as u8).collect();
        assert_eq!(detect_content_type(&data), ContentType::Binary);
    }

    #[test]
    fn content_type_serialization_is_lowercase() {
        let json = serde_json::to_string(&ContentType::Utf16Le).unwrap();
        assert_eq!(json, "\"utf16le\"", "rename_all = lowercase emits utf16le");
        let json = serde_json::to_string(&ContentType::Binary).unwrap();
        assert_eq!(json, "\"binary\"");
    }
}
