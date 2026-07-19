// SPDX-License-Identifier: MIT OR Apache-2.0

//! Surgical file editing by line number, text marker, or exact match.
//! Workload: I/O-bound (file read + fuzzy match + atomic write).
//! Parallelism: single target file (mutations ordered). Multi-pair
//! `--old-file`/`--new-file` reads fan out with `rayon` when count > 1;
//! within each pair, old∥new use `rayon::join` (two independent I/O paths).

use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
pub use crate::cli::FuzzyMode;
use crate::cli::{EditArgs, GlobalArgs};
use crate::commands::{read_stdin_text_guarded, resolve_backup};
use crate::error::AtomwriteError;
use crate::fuzzy::FuzzyInfo;
use crate::ndjson_types::{EditOutput, PairResult};
use crate::output::NdjsonWriter;


include!("pairs.inc.rs");
include!("single.inc.rs");
include!("multi.inc.rs");
include!("apply.inc.rs");
include!("lines.inc.rs");
