// SPDX-License-Identifier: MIT OR Apache-2.0

//! Smart file reading with automatic memmap2 for large files.
//!
//! Files above `MMAP_THRESHOLD` are memory-mapped via the kernel page cache,
//! then copied to a `Vec<u8>` because callers require ownership for mutation.
//! For read-only use cases (e.g., checksumming), prefer [`crate::checksum::hash_file`]
//! which operates directly on the mmap without copying.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::constants::MMAP_THRESHOLD;
use crate::error::AtomwriteError;

/// Read a file as raw bytes, using mmap for files above the threshold.
///
/// Files exceeding `max_size` are rejected before any allocation occurs.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the file does not exist.
/// Returns `AtomwriteError::FileTooLarge` if the file exceeds `max_size`.
/// Returns an I/O error if the file cannot be read or mmapped.
#[allow(unsafe_code)]
#[tracing::instrument(skip_all, fields(path = %path.display()))]
pub fn read_file_bytes(path: &Path, max_size: u64) -> Result<Vec<u8>> {
    if !path.exists() {
        return Err(AtomwriteError::NotFound {
            path: path.to_path_buf(),
        }
        .into());
    }

    let meta = fs::metadata(path)
        .inspect_err(
            |e| tracing::debug!(?e, path = %path.display(), "read_file_bytes: stat failed"),
        )
        .with_context(|| format!("cannot stat {}", path.display()))?;

    if meta.len() > max_size {
        return Err(AtomwriteError::FileTooLarge {
            path: path.to_path_buf(),
            size: meta.len(),
            max_size,
        }
        .into());
    }

    if meta.len() >= MMAP_THRESHOLD {
        let file =
            fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let _ = nix::fcntl::posix_fadvise(
                file.as_raw_fd(),
                0,
                0,
                nix::fcntl::PosixFadviseAdvice::POSIX_FADV_SEQUENTIAL,
            );
        }
        // SAFETY: File is opened read-only and the handle is held for the mmap
        // lifetime. Concurrent modification yields a stale read, not UB.
        let mmap = unsafe { memmap2::Mmap::map(&file) }
            .with_context(|| format!("cannot mmap {}", path.display()))?;
        Ok(mmap.to_vec())
    } else {
        fs::read(path).with_context(|| format!("cannot read {}", path.display()))
    }
}

fn strip_utf8_bom(bytes: &mut Vec<u8>) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(..3);
    }
}

/// Read a file as a UTF-8 string, using mmap for files above the threshold.
///
/// Files exceeding `max_size` are rejected before any allocation occurs.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the file does not exist.
/// Returns `AtomwriteError::FileTooLarge` if the file exceeds `max_size`.
/// Returns `AtomwriteError::InvalidInput` if the file is not valid UTF-8.
/// Returns an I/O error if the file cannot be read or mmapped.
pub fn read_file_string(path: &Path, max_size: u64) -> Result<String> {
    let mut bytes = read_file_bytes(path, max_size)?;
    strip_utf8_bom(&mut bytes);
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
    fn read_file_string_invalid_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("binary.bin");
        std::fs::write(&path, [0xFF, 0xFE, 0x00]).unwrap();
        let result = read_file_string(&path, u64::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn read_file_bytes_nonexistent() {
        let result = read_file_bytes(std::path::Path::new("/nonexistent/file.txt"), u64::MAX);
        assert!(result.is_err());
    }
}
