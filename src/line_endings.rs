// SPDX-License-Identifier: MIT OR Apache-2.0

//! Line ending detection and normalization.

use crate::constants::LINE_ENDING_DETECT_SIZE;

/// Detected or desired line ending style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LineEnding {
    /// Unix-style line feed.
    Lf,
    /// Windows-style carriage return + line feed.
    CrLf,
    /// Classic Mac carriage return.
    Cr,
    /// Preserve the dominant ending of the original file.
    Auto,
}

/// Detect the dominant line ending in the data.
///
/// Returns [`LineEnding::Lf`] when no line endings are found.
///
/// # Examples
///
/// ```
/// use atomwrite::line_endings::{detect, LineEnding};
///
/// assert_eq!(detect(b"hello\nworld\n"), LineEnding::Lf);
/// assert_eq!(detect(b"hello\r\nworld\r\n"), LineEnding::CrLf);
/// assert_eq!(detect(b""), LineEnding::Lf);
/// ```
pub fn detect(data: &[u8]) -> LineEnding {
    let window = &data[..data.len().min(LINE_ENDING_DETECT_SIZE)];
    let mut crlf = 0u32;
    let mut lf = 0u32;
    let mut cr = 0u32;

    let mut i = 0;
    while i < window.len() {
        if window[i] == b'\r' {
            if i + 1 < window.len() && window[i + 1] == b'\n' {
                crlf += 1;
                i += 2;
                continue;
            }
            cr += 1;
        } else if window[i] == b'\n' {
            lf += 1;
        }
        i += 1;
    }

    if crlf == 0 && lf == 0 && cr == 0 {
        return LineEnding::Lf;
    }

    if crlf >= lf && crlf >= cr {
        LineEnding::CrLf
    } else if lf >= cr {
        LineEnding::Lf
    } else {
        LineEnding::Cr
    }
}

/// Normalize all line endings in content to the target style.
///
/// # Examples
///
/// ```
/// use atomwrite::line_endings::{normalize, LineEnding};
///
/// assert_eq!(normalize("a\r\nb\r\n", LineEnding::Lf), "a\nb\n");
/// assert_eq!(normalize("a\nb\n", LineEnding::CrLf), "a\r\nb\r\n");
/// ```
pub fn normalize(content: &str, target: LineEnding) -> String {
    let canonical = content.replace("\r\n", "\n").replace('\r', "\n");
    match target {
        LineEnding::Lf | LineEnding::Auto => canonical,
        LineEnding::CrLf => canonical.replace('\n', "\r\n"),
        LineEnding::Cr => canonical.replace('\n', "\r"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_lf_only() {
        assert_eq!(detect(b"hello\nworld\n"), LineEnding::Lf);
    }

    #[test]
    fn detect_crlf_only() {
        assert_eq!(detect(b"hello\r\nworld\r\n"), LineEnding::CrLf);
    }

    #[test]
    fn detect_cr_only() {
        assert_eq!(detect(b"hello\rworld\r"), LineEnding::Cr);
    }

    #[test]
    fn detect_mixed_crlf_dominant() {
        assert_eq!(detect(b"a\r\nb\r\nc\nd\r\n"), LineEnding::CrLf);
    }

    #[test]
    fn detect_empty_defaults_to_lf() {
        assert_eq!(detect(b""), LineEnding::Lf);
    }

    #[test]
    fn normalize_crlf_to_lf() {
        assert_eq!(normalize("a\r\nb\r\n", LineEnding::Lf), "a\nb\n");
    }

    #[test]
    fn normalize_lf_to_crlf() {
        assert_eq!(normalize("a\nb\n", LineEnding::CrLf), "a\r\nb\r\n");
    }

    #[test]
    fn normalize_mixed_to_lf() {
        assert_eq!(normalize("a\r\nb\rc\n", LineEnding::Lf), "a\nb\nc\n");
    }
}
