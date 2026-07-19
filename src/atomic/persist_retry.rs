// SPDX-License-Identifier: MIT OR Apache-2.0

//! Windows tempfile persist with retry backoff (A-022).

#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
use anyhow::Result;

#[cfg(windows)]
use tempfile::NamedTempFile;

#[cfg(windows)]
use crate::error::AtomwriteError;

#[cfg(windows)]
pub(crate) fn persist_with_retry(mut temp: NamedTempFile, target: &Path) -> Result<()> {
    // A-022: named retry backoff (constants::PERSIST_RETRY_DELAYS_MS).
    for delay_ms in crate::constants::PERSIST_RETRY_DELAYS_MS {
        match temp.persist(target) {
            Ok(_) => return Ok(()),
            Err(e) => {
                if e.error.kind() == std::io::ErrorKind::PermissionDenied {
                    std::thread::sleep(std::time::Duration::from_millis(*delay_ms));
                    temp = e.file;
                    continue;
                }
                return Err(anyhow::anyhow!(
                    "rename error for {}: {}",
                    target.display(),
                    e.error
                ));
            }
        }
    }
    Err(AtomwriteError::PermissionDenied {
        path: target.to_path_buf(),
    }
    .into())
}

