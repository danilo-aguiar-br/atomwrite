// SPDX-License-Identifier: MIT OR Apache-2.0

//! BLAKE3 checksum computation for files and byte slices.

#![allow(unsafe_code)]

use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};

use crate::constants::MMAP_THRESHOLD;

/// Compute the BLAKE3 hash of an in-memory byte slice.
pub fn hash_bytes(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Compute the BLAKE3 hash of a file, using mmap for large files.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if the file cannot be read or memory-mapped.
pub fn hash_file(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path).with_context(|| format!("cannot stat {}", path.display()))?;

    if metadata.len() >= MMAP_THRESHOLD {
        hash_file_mmap(path)
    } else {
        let data = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
        Ok(hash_bytes(&data))
    }
}

fn hash_file_mmap(path: &Path) -> Result<String> {
    let file = fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    // SAFETY: The file is opened read-only and we hold the File handle for the
    // duration of the mmap. The file is not modified during hashing. If an
    // external process modifies the file concurrently, we may read inconsistent
    // data, but this is acceptable for checksumming (the checksum will simply
    // reflect whatever bytes were mapped).
    let mmap = unsafe { memmap2::Mmap::map(&file) }
        .with_context(|| format!("cannot mmap {}", path.display()))?;
    Ok(hash_bytes(&mmap))
}

/// Compute the BLAKE3 hash by streaming from any reader.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if a read error occurs during hashing.
pub fn hash_reader(reader: &mut impl Read) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; crate::constants::BUF_CAPACITY];
    loop {
        let n = reader.read(&mut buf).context("read error during hashing")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn hash_bytes_empty() {
        let h = hash_bytes(b"");
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn hash_bytes_deterministic() {
        let a = hash_bytes(b"hello world");
        let b = hash_bytes(b"hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_bytes_different_inputs() {
        let a = hash_bytes(b"hello");
        let b = hash_bytes(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_reader_matches_hash_bytes() {
        let data = b"test data for hashing";
        let from_bytes = hash_bytes(data);
        let mut cursor = Cursor::new(data);
        let from_reader = hash_reader(&mut cursor).unwrap();
        assert_eq!(from_bytes, from_reader);
    }

    #[test]
    fn hash_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "file content").unwrap();
        let file_hash = hash_file(&path).unwrap();
        let bytes_hash = hash_bytes(b"file content");
        assert_eq!(file_hash, bytes_hash);
    }
}
