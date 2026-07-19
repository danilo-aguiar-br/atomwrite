// SPDX-License-Identifier: MIT OR Apache-2.0

//! Domain-specific error types with exit codes and error classification.
//!
//! Workload: mixed (error construction is CPU-light; NotFound suggestions
//! walk the workspace for similar basenames).
//! Parallelism: `similar_paths_for` uses budgeted-depth `WalkParallel` bound
//! by the process-wide thread pool, then ranks with jaro-winkler (top-5).
//! Sequential alternative rejected for monorepo NotFound paths where depth-6
//! serial walk dominates agent retry latency.

use std::path::PathBuf;

use rust_i18n::t;
use schemars::JsonSchema;
use serde::Serialize;

use crate::ndjson_types::{BestCandidate, PairResult};

include!("class.inc.rs");
include!("variants.inc.rs");
include!("impl_methods.inc.rs");
include!("error_json.inc.rs");
include!("suggest.inc.rs");

include!("tests.inc.rs");
