// SPDX-License-Identifier: MIT OR Apache-2.0
// Agent-native NDJSON stdout shaping (v0.1.36).
// v0.1.36 residual: --select / --fields, --filter, --limit.
// v0.1.36 universal data operations: --sort, --dedupe, --aggregate, --join,
// --truncate-content, --redact, --normalize-encoding, --diff-against,
// --max-output-bytes.
// Included from output.rs. Zero-cost identity path when no shape flag is set.

use memchr::memmem;
use serde_json::Value;
include!("output_shape/secrets.inc.rs");
include!("output_shape/types.inc.rs");
include!("output_shape/apply.inc.rs");
include!("output_shape/ordering.inc.rs");
include!("output_shape/limits.inc.rs");
include!("output_shape/join_aggregate.inc.rs");
include!("output_shape/join_stdin.inc.rs");
include!("output_shape/stream_diff.inc.rs");
include!("output_shape/emit.inc.rs");
include!("output_shape/tests.inc.rs");
