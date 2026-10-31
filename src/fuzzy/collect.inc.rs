// Non-overlapping line-range collection for fuzzy cascade strategies (GAP-2026-026).

/// Finder that returns a relative (start, end) range within a slice of lines.
///
/// `Ok(None)` is a miss; `Err` is a cancellation and must propagate.
pub(crate) type LineRangeFinder = fn(&[&str], &[&str]) -> LineMatch;

/// Collect all non-overlapping matches of `old_lines` in `content_lines` via `finder`.
///
/// Scans forward; each hit advances past its end so ranges never overlap.
///
/// # Errors
///
/// Returns [`AtomwriteError::Cancelled`] when a shutdown poll fires between
/// hits, and propagates any error `finder` returns (it uses `Err` to signal a
/// cancellation it detected while scanning).
pub(crate) fn collect_all_ranges(
    content_lines: &[&str],
    old_lines: &[&str],
    finder: LineRangeFinder,
) -> std::result::Result<Vec<(usize, usize)>, AtomwriteError> {
    let mut hits = Vec::new();
    if old_lines.is_empty() {
        return Ok(hits);
    }
    let mut start_at = 0usize;
    while start_at < content_lines.len() {
        check_cancel()?;
        if let Some((s, e)) = finder(&content_lines[start_at..], old_lines)? {
            let abs_s = start_at + s;
            let abs_e = start_at + e;
            hits.push((abs_s, abs_e));
            start_at = abs_e.max(start_at + 1);
        } else {
            break;
        }
    }
    Ok(hits)
}
