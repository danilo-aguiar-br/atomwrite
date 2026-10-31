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
//! Latency notes (Rules Rust latency):
//! - Prefer [`crate::checksum::hash_file_with_len`] when the caller also needs byte size — one
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
///
/// # Examples
///
/// ```
/// use atomwrite::checksum::hash_bytes;
///
/// let digest = hash_bytes(b"hello");
/// assert_eq!(digest.len(), 64);
/// assert_eq!(digest, hash_bytes(b"hello"));
/// assert_ne!(digest, hash_bytes(b"world"));
/// ```
#[inline]
pub fn hash_bytes(data: &[u8]) -> String {
    if data.len() as u64 >= MMAP_THRESHOLD {
        // Reaching rayon from a pool-free command (read, write, verify) must
        // still build the pool with the operator bound, not rayon's default.
        crate::concurrency::ensure_global_pool();
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
///
/// # Examples
///
/// ```
/// use atomwrite::checksum::{hash_bytes, hash_file};
///
/// let dir = tempfile::tempdir()?;
/// let path = dir.path().join("a.txt");
/// std::fs::write(&path, b"hello")?;
/// assert_eq!(hash_file(&path, 1024)?, hash_bytes(b"hello"));
/// # Ok::<(), anyhow::Error>(())
/// ```
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

/// Hash a file with a fixed buffer, touching no mapped page.
///
/// [`hash_file_with_len`] maps a large file and hashes it with every core, at
/// the cost of the file's pages entering RSS. A verb that must stay small in
/// memory (`read --stat`, which is metadata plus a digest) uses this instead:
/// the digest takes longer, and the peak stays at one buffer.
///
/// # Errors
///
/// Returns [`crate::error::AtomwriteError::FileTooLarge`] above `max_size`, and
/// an I/O error when the file cannot be opened or read.
pub fn hash_file_streaming(path: &Path, max_size: u64) -> Result<(String, u64)> {
    let file = fs::File::open(path)
        .inspect_err(|e| tracing::debug!(?e, path = %path.display(), "hash streaming: open failed"))
        .with_context(|| format!("cannot open {}", path.display()))?;
    let len = file
        .metadata()
        .inspect_err(|e| tracing::debug!(?e, path = %path.display(), "hash streaming: stat failed"))
        .with_context(|| format!("cannot stat {}", path.display()))?
        .len();
    if len > max_size {
        return Err(crate::error::AtomwriteError::FileTooLarge {
            path: path.to_path_buf(),
            size: len,
            max_size,
        }
        .into());
    }
    let mut reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, file);
    Ok((hash_reader(&mut reader)?, len))
}

/// Multi-core mmap hash for large files (blake3 `update_mmap_rayon`).
fn hash_file_mmap_rayon(path: &Path) -> Result<String> {
    crate::concurrency::ensure_global_pool();
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
///
/// # Examples
///
/// ```
/// use atomwrite::checksum::{hash_bytes, hash_reader};
///
/// let mut input: &[u8] = b"streamed content";
/// assert_eq!(hash_reader(&mut input)?, hash_bytes(b"streamed content"));
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn hash_reader(reader: &mut impl Read) -> Result<String> {
    let (hash, _len) = hash_reader_with_len(reader)?;
    Ok(hash)
}

/// Hash a reader and report how many bytes were read.
///
/// The length is counted inside the same read loop, so streaming a pipe costs
/// nothing extra: `hash --stdin` can publish `bytes` and a `total_bytes` total
/// without buffering the stream or reading it twice.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if a read error occurs during hashing.
pub fn hash_reader_with_len(reader: &mut impl Read) -> Result<(String, u64)> {
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; crate::constants::BUF_CAPACITY];
    let mut total = 0u64;
    loop {
        let n = reader.read(&mut buf).context("read error during hashing")?;
        if n == 0 {
            break;
        }
        total = total.saturating_add(n as u64);
        hasher.update(&buf[..n]);
    }
    Ok((hash_to_hex_string(hasher.finalize()), total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn large_buffer_hash_matches_blake3_and_builds_the_bounded_pool() {
        let data = vec![0xA5u8; MMAP_THRESHOLD as usize + 1];
        assert_eq!(hash_bytes(&data), blake3::hash(&data).to_hex().to_string());
        let pool = crate::concurrency::configured_pool_size().expect("pool built before rayon use");
        assert!(pool >= 1);
    }

    #[test]
    fn large_file_hash_matches_blake3() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.bin");
        let data = vec![0x5Au8; MMAP_THRESHOLD as usize + 1];
        std::fs::write(&path, &data).expect("write");
        let got = hash_file(&path, u64::MAX).expect("hash");
        assert_eq!(got, blake3::hash(&data).to_hex().to_string());
        assert!(crate::concurrency::configured_pool_size().is_some());
    }

    #[test]
    fn streaming_hash_matches_the_buffer_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("s.bin");
        let data = vec![0x11u8; 200_000];
        std::fs::write(&path, &data).expect("write");
        let (streamed, len) = hash_file_streaming(&path, u64::MAX).expect("hash");
        assert_eq!(len, data.len() as u64);
        assert_eq!(streamed, blake3::hash(&data).to_hex().to_string());
    }

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
