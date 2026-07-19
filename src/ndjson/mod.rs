// SPDX-License-Identifier: MIT OR Apache-2.0

//! NDJSON event types (SRP split via include fragments — shared module namespace).

use schemars::JsonSchema;
use serde::Serialize;

include!("part0.inc.rs");
include!("part1.inc.rs");
include!("part2.inc.rs");

#[cfg(test)]
include!("tests.inc.rs");
