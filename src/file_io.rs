// SPDX-License-Identifier: MIT OR Apache-2.0

//! Smart file reading: one open, one `fstat`, one read into an exact-size buffer.
//!
//! Workload: I/O-bound. The size comes from the open handle (no path race between
//! stat and read) and the bytes are read straight into a `Vec` reserved for that
//! size, so a file costs one copy of itself in memory. An earlier version mapped
//! files above `MMAP_THRESHOLD` and then copied the mapping into a `Vec`, which
//! touched every page twice and held the file twice at peak.
//!
//! Zero-copy path: for pure read-only hashing, prefer
//! [`crate::checksum::hash_file`], which hashes the mmap without `to_vec()`.

use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};

#[cfg(target_os = "linux")]
use crate::constants::MMAP_THRESHOLD;
use crate::error::AtomwriteError;

/// Read a file as raw bytes.
///
/// Files exceeding `max_size` are rejected **before** any heap allocation.
/// The size is read from the open handle and capacity is reserved with
/// [`Vec::try_reserve_exact`] so allocation failure becomes a recoverable domain
/// error instead of an abort (Rules: fallible OOM / `try_reserve` for
/// input-derived sizes).
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the file does not exist.
/// Returns `AtomwriteError::FileTooLarge` if the file exceeds `max_size`.
/// Returns `AtomwriteError::InternalError` if the heap reservation fails.
/// Returns an I/O error if the file cannot be opened, stat-ed or read.
#[tracing::instrument(skip_all, fields(path = %path.display()))]
pub fn read_file_bytes(path: &Path, max_size: u64) -> Result<Vec<u8>> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(AtomwriteError::NotFound {
                path: path.to_path_buf(),
            }
            .into());
        }
        // Classified, not wrapped: a caller that reports per-file failures
        // needs `PERMISSION_DENIED`, and the typed error carries the path.
        Err(e) => return Err(AtomwriteError::from_io_at(path, e).into()),
    };

    let size = file
        .metadata()
        .inspect_err(
            |e| tracing::debug!(?e, path = %path.display(), "read_file_bytes: fstat failed"),
        )
        .with_context(|| format!("cannot stat {}", path.display()))?
        .len();

    if size > max_size {
        return Err(AtomwriteError::FileTooLarge {
            path: path.to_path_buf(),
            size,
            max_size,
        }
        .into());
    }

    let alloc_failed = |detail: String| AtomwriteError::InternalError {
        reason: format!(
            "allocation failed for {size} bytes reading {}: {detail}",
            path.display()
        ),
    };
    let len = usize::try_from(size).map_err(|e| alloc_failed(e.to_string()))?;
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|e| alloc_failed(e.to_string()))?;

    #[cfg(target_os = "linux")]
    if size >= MMAP_THRESHOLD {
        if let Err(e) = nix::fcntl::posix_fadvise(
            &file,
            0,
            0,
            nix::fcntl::PosixFadviseAdvice::POSIX_FADV_SEQUENTIAL,
        ) {
            tracing::debug!(?e, "read_file_bytes: fadvise(SEQUENTIAL) hint ignored");
        }
    }

    file.read_to_end(&mut out)
        .with_context(|| format!("cannot read {}", path.display()))?;
    Ok(out)
}

/// Check that `path` holds valid UTF-8 without holding the file.
///
/// [`read_file_string`] allocates the whole file before it can discover the
/// bytes are not text: refusing a 280 MB binary cost 300 MB of RSS for one
/// error line. This pass validates with a fixed buffer, so a verb can refuse
/// before allocating anything.
///
/// # Errors
///
/// Returns `AtomwriteError::FileTooLarge` above `max_size`, the same
/// `AtomwriteError::InvalidInput` [`read_file_string`] raises for bad UTF-8,
/// and an I/O error when the file cannot be opened or read.
pub fn validate_utf8_stream(path: &Path, max_size: u64) -> Result<()> {
    use std::io::Read;
    let file = fs::File::open(path).map_err(|e| AtomwriteError::from_io_at(path, e))?;
    let size = file
        .metadata()
        .with_context(|| format!("cannot stat {}", path.display()))?
        .len();
    if size > max_size {
        return Err(AtomwriteError::FileTooLarge {
            path: path.to_path_buf(),
            size,
            max_size,
        }
        .into());
    }
    let invalid = || {
        anyhow::Error::from(AtomwriteError::InvalidInput {
            reason: format!("file is not valid UTF-8: {}", path.display()),
        })
    };
    let mut reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, file);
    let mut buf = [0u8; crate::constants::BUF_CAPACITY];
    // A multi-byte sequence can straddle two reads, so up to three bytes carry
    // over to the next chunk before anything is declared invalid.
    let mut carry: Vec<u8> = Vec::with_capacity(4);
    loop {
        let read = reader
            .read(&mut buf)
            .with_context(|| format!("cannot read {}", path.display()))?;
        if read == 0 {
            break;
        }
        let mut chunk = std::mem::take(&mut carry);
        chunk.extend_from_slice(&buf[..read]);
        match std::str::from_utf8(&chunk) {
            Ok(_) => {}
            Err(e) if e.error_len().is_some() => return Err(invalid()),
            Err(e) => {
                let valid = e.valid_up_to();
                carry.extend_from_slice(&chunk[valid..]);
            }
        }
    }
    if !carry.is_empty() {
        return Err(invalid());
    }
    Ok(())
}

/// Count occurrences of `needle` in a file without holding the file.
///
/// A verb that only needs to know "is this string here, and how many times"
/// should not materialize a 280 MB file to answer it. This pass reads through a
/// fixed buffer and keeps a `needle.len() - 1` byte overlap, so a match that
/// straddles two reads is still counted once.
///
/// Returns `(count, size_in_bytes)`.
///
/// # Errors
///
/// Returns `AtomwriteError::FileTooLarge` above `max_size`, the same
/// `AtomwriteError::InvalidInput` [`read_file_string`] raises for bad UTF-8,
/// and an I/O error when the file cannot be opened or read.
pub fn count_exact_streaming(path: &Path, needle: &[u8], max_size: u64) -> Result<(u64, u64)> {
    let file = fs::File::open(path).map_err(|e| AtomwriteError::from_io_at(path, e))?;
    let size = file
        .metadata()
        .with_context(|| format!("cannot stat {}", path.display()))?
        .len();
    if size > max_size {
        return Err(AtomwriteError::FileTooLarge {
            path: path.to_path_buf(),
            size,
            max_size,
        }
        .into());
    }
    if needle.is_empty() {
        return Ok((0, size));
    }
    // Validation and counting ride the same pass: a file that fails UTF-8 is
    // refused exactly as `read_file_string` would refuse it.
    let invalid = || {
        anyhow::Error::from(AtomwriteError::InvalidInput {
            reason: format!("file is not valid UTF-8: {}", path.display()),
        })
    };
    let mut reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, file);
    let mut buf = vec![0u8; crate::constants::BUF_CAPACITY];
    // Bytes carried over from the previous chunk so a straddling match is seen.
    let overlap = needle.len().saturating_sub(1);
    let mut carry: Vec<u8> = Vec::with_capacity(overlap + crate::constants::BUF_CAPACITY);
    let mut count = 0u64;
    let mut pending_utf8: Vec<u8> = Vec::with_capacity(4);
    // Absolute file offset of `carry[0]`, and the offset just past the last
    // match already counted. A carried byte is scanned twice; the second scan
    // starts inside bytes the first one already covered, so without these two
    // numbers the overlap would count the same match again.
    let mut carry_base: u64 = 0;
    let mut counted_upto: u64 = 0;
    loop {
        let read = reader
            .read(&mut buf)
            .with_context(|| format!("cannot read {}", path.display()))?;
        if read == 0 {
            break;
        }
        let mut chunk = std::mem::take(&mut pending_utf8);
        chunk.extend_from_slice(&buf[..read]);
        match std::str::from_utf8(&chunk) {
            Ok(_) => {}
            Err(e) if e.error_len().is_some() => return Err(invalid()),
            Err(e) => {
                let valid = e.valid_up_to();
                pending_utf8.extend_from_slice(&chunk[valid..]);
            }
        }
        carry.extend_from_slice(&buf[..read]);
        let mut start = 0usize;
        while let Some(pos) = memchr::memmem::find(&carry[start..], needle) {
            let absolute = carry_base
                .saturating_add(start as u64)
                .saturating_add(pos as u64);
            if absolute >= counted_upto {
                count = count.saturating_add(1);
                counted_upto = absolute.saturating_add(needle.len() as u64);
            }
            start += pos + needle.len();
            if start >= carry.len() {
                break;
            }
        }
        // Keep only the tail a future match could still need, and remember
        // where that tail begins in the file.
        if carry.len() > overlap {
            let drop = carry.len() - overlap;
            carry.drain(..drop);
            carry_base = carry_base.saturating_add(drop as u64);
        }
    }
    if !pending_utf8.is_empty() {
        return Err(invalid());
    }
    Ok((count, size))
}

/// Optional byte-level BOM strip (kept for call sites that intentionally discard BOM).
#[allow(dead_code)]
fn strip_utf8_bom(bytes: &mut Vec<u8>) {
    if bytes.starts_with(&crate::constants::UTF8_BOM) {
        bytes.drain(..crate::constants::UTF8_BOM.len());
    }
}

/// Read a file as a UTF-8 string (single exact-size read, no second copy).
///
/// Files exceeding `max_size` are rejected before any allocation occurs.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the file does not exist.
/// Returns `AtomwriteError::FileTooLarge` if the file exceeds `max_size`.
/// Returns `AtomwriteError::InvalidInput` if the file is not valid UTF-8.
/// Returns an I/O error if the file cannot be opened, stat-ed or read.
///
/// # Examples
///
/// ```
/// use atomwrite::file_io::read_file_string;
///
/// let dir = tempfile::tempdir()?;
/// let path = dir.path().join("a.txt");
/// std::fs::write(&path, "line one\n")?;
/// assert_eq!(read_file_string(&path, 1024)?, "line one\n");
/// assert!(read_file_string(&path, 4).is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn read_file_string(path: &Path, max_size: u64) -> Result<String> {
    // G-FZZ-075: do not strip UTF-8 BOM here — write-back must preserve it.
    // Fuzzy match normalizes U+FEFF / ZWSP on the match path only.
    let bytes = read_file_bytes(path, max_size)?;
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
        let bytes = read_file_bytes(&path, u64::MAX).unwrap();
        assert_eq!(bytes, b"hello");
    }

    #[test]
    fn read_file_string_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("utf8.txt");
        std::fs::write(&path, "hello world").unwrap();
        let s = read_file_string(&path, u64::MAX).unwrap();
        assert_eq!(s, "hello world");
    }

    #[test]
    fn streaming_utf8_check_matches_the_reading_path() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good.txt");
        std::fs::write(&good, "ação\nlinha\n").unwrap();
        validate_utf8_stream(&good, u64::MAX).expect("valid utf-8");
        let bad = dir.path().join("bad.bin");
        std::fs::write(&bad, [0xFFu8, 0xFE, 0x00]).unwrap();
        validate_utf8_stream(&bad, u64::MAX).expect_err("invalid utf-8");
        // A multi-byte sequence crossing the buffer edge must not be refused.
        let big = dir.path().join("edge.txt");
        let mut text = "a".repeat(crate::constants::BUF_CAPACITY - 1);
        text.push('é');
        std::fs::write(&big, text).unwrap();
        validate_utf8_stream(&big, u64::MAX).expect("split codepoint is valid");
    }

    #[test]
    fn read_file_string_invalid_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("binary.bin");
        std::fs::write(&path, [0xFF, 0xFE, 0x00]).unwrap();
        let result = read_file_string(&path, u64::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn read_file_bytes_reads_file_above_mmap_threshold_exactly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.bin");
        let data: Vec<u8> = (0..(MMAP_THRESHOLD as usize + 4097))
            .map(|i| (i % 251) as u8)
            .collect();
        std::fs::write(&path, &data).unwrap();
        let bytes = read_file_bytes(&path, u64::MAX).unwrap();
        assert_eq!(bytes.len(), data.len());
        assert!(bytes == data, "bytes read must equal bytes written");
        assert_eq!(
            bytes.capacity(),
            data.len(),
            "exact-size buffer, no regrowth"
        );
    }

    #[test]
    fn read_file_bytes_nonexistent_is_typed_not_found() {
        let err =
            read_file_bytes(std::path::Path::new("/nonexistent/typed.txt"), u64::MAX).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<AtomwriteError>(),
            Some(AtomwriteError::NotFound { .. })
        ));
    }

    #[test]
    fn read_file_bytes_nonexistent() {
        let result = read_file_bytes(std::path::Path::new("/nonexistent/file.txt"), u64::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn read_file_bytes_rejects_over_max_before_alloc() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.txt");
        std::fs::write(&path, vec![b'x'; 64]).unwrap();
        let err = read_file_bytes(&path, 16).unwrap_err();
        let ae = err
            .downcast_ref::<AtomwriteError>()
            .expect("AtomwriteError");
        assert!(
            matches!(
                ae,
                AtomwriteError::FileTooLarge {
                    size: 64,
                    max_size: 16,
                    ..
                }
            ),
            "got {ae:?}"
        );
    }
}
