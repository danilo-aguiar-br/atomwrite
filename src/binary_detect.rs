// SPDX-License-Identifier: MIT OR Apache-2.0

//! Binary content detection via null-byte heuristic.

use crate::constants::BINARY_DETECT_SIZE;

/// Detect whether data is binary by checking for null bytes in the first 8 KiB.
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
    let check = &data[..data.len().min(BINARY_DETECT_SIZE)];
    check.contains(&0)
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
        assert!(!is_binary(&data));
    }

    #[test]
    fn high_bytes_not_binary() {
        assert!(!is_binary(&[0xFF, 0xFE, 0xFD]));
    }
}
