// SPDX-License-Identifier: MIT OR Apache-2.0

//! NDJSON event types (SRP split via include fragments — shared module namespace).
//!
//! # Examples
//!
//! A [`BestCandidate`] is the near miss that a failed match reports; empty
//! fields are left out of the JSON:
//!
//! ```
//! use atomwrite::ndjson::BestCandidate;
//!
//! let candidate = BestCandidate {
//!     text: Some("let total = 1;".into()),
//!     line: Some(2),
//!     column: Some(5),
//!     similarity: Some(0.92),
//!     strategy: None,
//!     diff_preview: None,
//! };
//! let json = serde_json::to_value(&candidate)?;
//! assert_eq!(json["line"], 2);
//! assert!(json.get("strategy").is_none());
//! # Ok::<(), serde_json::Error>(())
//! ```

use schemars::JsonSchema;
use serde::Serialize;

include!("part0.inc.rs");
include!("part1.inc.rs");
include!("part2.inc.rs");
include!("part3.inc.rs");

#[cfg(test)]
include!("tests.inc.rs");
