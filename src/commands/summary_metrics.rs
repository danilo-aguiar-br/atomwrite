// SPDX-License-Identifier: MIT OR Apache-2.0

//! DRY summary counters for dry-run / plan / preview surfaces (R-DRY-001 / B-002).
//!
//! One-shot agents must never see `files_modified > 0` when the command did not
//! mutate disk. Keep this helper free of I/O and allocation.

/// Report how many files were actually modified for an NDJSON `summary` event.
///
/// When `dry_run` (or plan/preview-equivalent) is true, always returns `Some(0)`
/// even if `matched` is positive. When mutating, returns `Some(matched)`.
#[inline]
#[must_use]
pub fn files_modified_for_summary(matched: u64, dry_run: bool) -> Option<u64> {
    if dry_run {
        Some(0)
    } else {
        Some(matched)
    }
}

/// Same as [`files_modified_for_summary`] but for non-optional `u64` summary fields
/// (e.g. `case` command).
#[inline]
#[must_use]
pub fn files_modified_count(matched: u64, dry_run: bool) -> u64 {
    if dry_run { 0 } else { matched }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_zeros_even_when_matched() {
        assert_eq!(files_modified_for_summary(7, true), Some(0));
        assert_eq!(files_modified_count(7, true), 0);
    }

    #[test]
    fn mutate_path_reports_matched() {
        assert_eq!(files_modified_for_summary(3, false), Some(3));
        assert_eq!(files_modified_count(0, false), 0);
    }
}
