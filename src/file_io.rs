// SPDX-License-Identifier: MIT OR Apache-2.0

//! Smart file reading with automatic memmap2 for large files.

#![allow(unsafe_code)]

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::constants::MMAP_THRESHOLD;
use crate::error::AtomwriteError;

/// Read a file as raw bytes, using mmap for files above the threshold.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the file does not exist.
/// Returns an I/O error if the file cannot be read or mmapped.
pub fn read_file_bytes(path: &Path) -> Result<Vec<u8>> {
    if !path.exists() {
        return Err(AtomwriteError::NotFound {
            path: path.to_path_buf(),
        }
        .into());
    }

    let meta = fs::metadata(path).with_context(|| format!("cannot stat {}", path.display()))?;

    if meta.len() >= MMAP_THRESHOLD {
        let file =
            fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
        // SAFETY: File is opened read-only and the handle is held for the mmap
        // lifetime. Concurrent modification yields a stale read, not UB.
        let mmap = unsafe { memmap2::Mmap::map(&file) }
            .with_context(|| format!("cannot mmap {}", path.display()))?;
        Ok(mmap.to_vec())
    } else {
        fs::read(path).with_context(|| format!("cannot read {}", path.display()))
    }
}

/// Read a file as a UTF-8 string, using mmap for files above the threshold.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the file does not exist.
/// Returns `AtomwriteError::InvalidInput` if the file is not valid UTF-8.
/// Returns an I/O error if the file cannot be read or mmapped.
pub fn read_file_string(path: &Path) -> Result<String> {
    let bytes = read_file_bytes(path)?;
    String::from_utf8(bytes).map_err(|_| {
        AtomwriteError::InvalidInput {
            reason: format!("file is not valid UTF-8: {}", path.display()),
        }
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_file_bytes_small() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.txt");
        std::fs::write(&path, "hello").unwrap();
        let bytes = read_file_bytes(&path).unwrap();
        assert_eq!(bytes, b"hello");
    }

    #[test]
    fn read_file_string_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("utf8.txt");
        std::fs::write(&path, "hello world").unwrap();
        let s = read_file_string(&path).unwrap();
        assert_eq!(s, "hello world");
    }

    #[test]
    fn read_file_string_invalid_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("binary.bin");
        std::fs::write(&path, [0xFF, 0xFE, 0x00]).unwrap();
        let result = read_file_string(&path);
        assert!(result.is_err());
    }

    #[test]
    fn read_file_bytes_nonexistent() {
        let result = read_file_bytes(std::path::Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }
}
