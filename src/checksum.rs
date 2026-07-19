// SPDX-License-Identifier: MIT OR Apache-2.0

//! BLAKE3 checksum computation for files and byte slices.
//!
//! Workload: CPU-bound (BLAKE3) with I/O for file paths.
//! Parallelism: multi-file digests fan out via `rayon` in callers (`hash`).
//! Large single files (`>= MMAP_THRESHOLD`) use `Hasher::update_mmap_rayon`
//! (blake3 features `mmap` + `rayon`) so one multi-GB file still saturates
//! cores. Nested rayon with multi-file `par_iter` is tolerated by work-stealing;
//! operators cap cores with `--threads` / `--max-concurrency`.
//!
//! Latency notes (Rules Rust latência):
//! - Prefer [`hash_file_with_len`] when the caller also needs byte size — one
//!   `open` + one `fstat`, no second `metadata` syscall.
//! - Large files: mmap-rayon path (b3sum default) or sequential readahead fallback.
//! - Hex digests: single 64-byte `String` allocation (no intermediate heap).

use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};

use crate::constants::MMAP_THRESHOLD;

/// BLAKE3 hex digest length in ASCII characters.
const BLAKE3_HEX_LEN: usize = 64;

/// Format a BLAKE3 hash as a lowercase hex `String` with one heap allocation.
#[inline]
fn hash_to_hex_string(hash: blake3::Hash) -> String {
    // `to_hex()` is stack-resident (`HashHex`); copy once into owned String.
    let hex = hash.to_hex();
    let s = hex.as_str();
    debug_assert_eq!(s.len(), BLAKE3_HEX_LEN);
    let mut out = String::with_capacity(BLAKE3_HEX_LEN);
    out.push_str(s);
    out
}

/// Compute the BLAKE3 hash of an in-memory byte slice.
///
/// Large buffers use `update_rayon` so single-shot CPU work scales with cores.
#[inline]
pub fn hash_bytes(data: &[u8]) -> String {
    if data.len() as u64 >= MMAP_THRESHOLD {
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(data);
        hash_to_hex_string(hasher.finalize())
    } else {
        hash_to_hex_string(blake3::hash(data))
    }
}

/// Compute the BLAKE3 hash of a file, using mmap for large files.
///
/// Prefer [`hash_file_with_len`] when you also need the file size (avoids a
/// second `stat` in the caller).
///
/// Files exceeding `max_size` are rejected before any body allocation.
///
/// # Errors
///
/// Returns `AtomwriteError::FileTooLarge` if the file exceeds `max_size`.
/// Returns `AtomwriteError::Io` if the file cannot be read or memory-mapped.
pub fn hash_file(path: &Path, max_size: u64) -> Result<String> {
    hash_file_with_len(path, max_size).map(|(hash, _len)| hash)
}

/// Hash a file and return `(hex_digest, byte_len)` with a single open/stat.
///
/// # Errors
///
/// Same as [`hash_file`].
pub fn hash_file_with_len(path: &Path, max_size: u64) -> Result<(String, u64)> {
    let file = fs::File::open(path)
        .inspect_err(|e| tracing::debug!(?e, path = %path.display(), "hash_file: open failed"))
        .with_context(|| format!("cannot open {}", path.display()))?;

    let metadata = file
        .metadata()
        .inspect_err(|e| tracing::debug!(?e, path = %path.display(), "hash_file: stat failed"))
        .with_context(|| format!("cannot stat {}", path.display()))?;

    let len = metadata.len();
    if len > max_size {
        return Err(crate::error::AtomwriteError::FileTooLarge {
            path: path.to_path_buf(),
            size: len,
            max_size,
        }
        .into());
    }

    if len >= MMAP_THRESHOLD {
        Ok((hash_file_mmap_rayon(path)?, len))
    } else {
        Ok((hash_file_read(file, path, len as usize)?, len))
    }
}

/// Multi-core mmap hash for large files (blake3 `update_mmap_rayon`).
fn hash_file_mmap_rayon(path: &Path) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_mmap_rayon(path)
        .with_context(|| format!("cannot mmap-hash {}", path.display()))?;
    Ok(hash_to_hex_string(hasher.finalize()))
}

/// Read a small file from an already-open handle and hash it.
fn hash_file_read(mut file: fs::File, path: &Path, len: usize) -> Result<String> {
    use std::io::Read;
    let mut data = Vec::new();
    if let Err(e) = data.try_reserve_exact(len) {
        return Err(crate::error::AtomwriteError::InternalError {
            reason: format!(
                "allocation failed for {len} bytes hashing {}: {e}",
                path.display()
            ),
        }
        .into());
    }
    file.read_to_end(&mut data)
        .with_context(|| format!("cannot read {}", path.display()))?;
    Ok(hash_bytes(&data))
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
    Ok(hash_to_hex_string(hasher.finalize()))
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
        let file_hash = hash_file(&path, u64::MAX).unwrap();
        let bytes_hash = hash_bytes(b"file content");
        assert_eq!(file_hash, bytes_hash);
    }

    #[test]
    fn hash_file_with_len_matches_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sized.txt");
        let payload = b"twelve bytes";
        std::fs::write(&path, payload).unwrap();
        let (hash, len) = hash_file_with_len(&path, u64::MAX).unwrap();
        assert_eq!(len, payload.len() as u64);
        assert_eq!(hash, hash_bytes(payload));
        assert_eq!(hash, hash_file(&path, u64::MAX).unwrap());
    }

    #[test]
    fn hash_file_rejects_over_max() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.txt");
        std::fs::write(&path, vec![b'x'; 64]).unwrap();
        let err = hash_file_with_len(&path, 16).unwrap_err();
        let ae = err
            .downcast_ref::<crate::error::AtomwriteError>()
            .expect("AtomwriteError");
        assert!(matches!(
            ae,
            crate::error::AtomwriteError::FileTooLarge {
                size: 64,
                max_size: 16,
                ..
            }
        ));
    }
}
