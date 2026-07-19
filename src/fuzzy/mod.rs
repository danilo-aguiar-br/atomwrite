// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared fuzzy match cascade for edit, replace, batch, and edit-loop (v0.1.30+).
//!
//! Workload: CPU-bound (string normalize + similarity scoring on one buffer).
//! Parallelism: multi-file at callers; intra-file window scoring uses rayon
//! (`windows::match_context_aware`) with bound fan-out and cancel polls.
//!
//! Cascade strategies (Auto/Aggressive): exact, line_trimmed, whitespace_normalized,
//! punctuation_normalized, indent_flexible, escape_normalized, trimmed_boundary,
//! block_anchor, unicode_normalized, context_aware_jw / context_aware.
//!
//! Product policy (v0.1.35): `Off` = exact-only (G-010). Guards: escape-drift,
//! match uniqueness, indent delta,
//! unicode preserve, always-on best_candidate + multi-candidates, diff_preview.
//!
//! v0.1.33 one-shot hardening:
//! - [`apply_fuzzy_one_pass`] never re-scans inserted replacement text (sed-style).
//! - Default max applies = 1; `replacement.contains(pattern)` forces 1.
//! - Pattern / levenshtein / window caps + cooperative cancel poll via
//!   [`crate::signal::is_global_shutdown`].

mod score;
mod windows;
mod normalize;
mod util;

use score::{gestalt_ratio, line_vote_ratio};
use windows::{match_block_anchor, match_context_aware};
use normalize::{maybe_unescape_new_string, normalize_unicode_for_match};
use util::{byte_offset_of_line, count_occurrences, find_str, line_col_of_offset, truncate_text};

use crate::cli_args::FuzzyMode;
use crate::error::AtomwriteError;
use crate::ndjson_types::BestCandidate;


include!("types.inc.rs");
include!("rank.inc.rs");
include!("cascade.inc.rs");
include!("strategies.inc.rs");
include!("one_pass.inc.rs");
include!("replace_lines.inc.rs");

include!("tests.inc.rs");
