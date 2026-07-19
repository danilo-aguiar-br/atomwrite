// SPDX-License-Identifier: MIT OR Apache-2.0

//! Batch execution of multiple operations from an NDJSON manifest.
//!
//! Workload: I/O-bound (NDJSON parse + multi-file atomic writes).
//! Parallelism: non-transactional batches with unique target paths fan out
//! via `rayon::par_iter` (independent mutations). Transaction **ops** stay
//! sequential so rollback order stays correct; transaction **pre-backup**
//! snapshots fan out with `par_iter` (independent I/O). Bound: process-wide
//! rayon pool.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::GlobalArgs;
use crate::concurrency::should_parallelize;
use crate::ndjson_types::{BatchOpResult, BatchSummary, ProgressEvent};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

include!("schema.inc.rs");
include!("run.inc.rs");
include!("txn.inc.rs");
include!("ops.inc.rs");
