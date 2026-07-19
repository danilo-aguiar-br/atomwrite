// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file write pipeline: tempfile, fsync, rename, fsync directory.
//!
//! Workload: I/O-bound (single-file durable write + optional backup/journal).
//! Parallelism: none inside one write (ordered rename/fsync contract). Multi-file
//! callers (`backup`, `batch`, `delete`) fan out with rayon around this module.
//!
//! Three write strategies are available, selected automatically:
//!
//! - `WriteStrategy::Rename` — classic tempfile + `rename(2)` (atomic, but
//!   destroys hardlinks and inode identity).
//! - `WriteStrategy::InPlace` — `ftruncate(0) + pwrite + fsync_data` on the
//!   existing file descriptor (preserves inode and hardlinks, but NOT atomic
//!   against a crash between truncate and write).
//! - `WriteStrategy::CopyBack` — like `InPlace` but with a journal sidecar
//!   for crash recovery; slowest but most durable. (Reserved for v0.1.13.)
//!
//! The default policy is **auto-detect**: hardlinks and symlinks trigger
//! `InPlace` automatically, regular files use Rename. Pass
//! `WriteStrategy::Rename` explicitly via the `strategy` field of
//! `AtomicWriteOptions` to force the legacy behavior.

mod types;
mod write;
mod rename_path;
mod syntax_heuristic;
mod inplace;
mod time_fmt;
#[cfg(windows)]
mod persist_retry;

pub use types::{AtomicWriteOptions, WriteResult, WriteStrategy};
pub use write::atomic_write;
pub use time_fmt::{epoch_to_utc, rfc3339_now};
pub(crate) use inplace::{create_backup, create_backup_in};

include!("tests.inc.rs");
