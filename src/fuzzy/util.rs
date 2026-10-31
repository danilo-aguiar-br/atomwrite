// SPDX-License-Identifier: MIT OR Apache-2.0

//! Small fuzzy text helpers (offsets, counts, truncation, cancel polling).

use crate::error::AtomwriteError;

pub(crate) fn find_str(haystack: &str, needle: &str) -> Option<usize> {
    memchr::memmem::find(haystack.as_bytes(), needle.as_bytes())
}

/// Count non-overlapping occurrences of `needle`, polling for cancellation.
///
/// # Errors
///
/// Returns [`AtomwriteError::Cancelled`] when the global shutdown flag is set.
pub(crate) fn count_occurrences(haystack: &str, needle: &str) -> Result<u64, AtomwriteError> {
    if needle.is_empty() {
        return Ok(0);
    }
    let mut poll = CancelPoll::lines();
    let mut count = 0u64;
    let mut start = 0usize;
    while let Some(pos) = find_str(&haystack[start..], needle) {
        poll.tick()?;
        count += 1;
        start += pos + needle.len();
        if start >= haystack.len() {
            break;
        }
    }
    Ok(count)
}

pub(crate) fn truncate_text(s: &str) -> String {
    if s.chars().count() <= crate::constants::FUZZY_BEST_CANDIDATE_TEXT_MAX {
        return s.to_string();
    }
    s.chars()
        .take(crate::constants::FUZZY_BEST_CANDIDATE_TEXT_MAX)
        .collect()
}

/// 1-based `(line, column)` of `byte_offset`, counted in chars like an editor.
pub(crate) fn line_col_of_offset(content: &str, byte_offset: usize) -> (u64, u64) {
    let end = byte_offset.min(content.len());
    let head = &content.as_bytes()[..end];
    let line = memchr::memchr_iter(b'\n', head).count() as u64 + 1;
    let line_start = memchr::memrchr(b'\n', head).map_or(0, |p| p + 1);
    let col = content[line_start..]
        .char_indices()
        .take_while(|(i, _)| line_start + i < end)
        .count() as u64
        + 1;
    (line, col)
}

/// `(line, column)` of the first column of the 0-based line `line_idx`.
///
/// Equivalent to `line_col_of_offset(content, <offset of that line>)` without
/// scanning the buffer, so ranking many candidates stays linear.
pub(crate) fn line_start_position(line_idx: usize) -> (u64, u64) {
    (line_idx as u64 + 1, 1)
}

/// Fail with [`AtomwriteError::Cancelled`] when the global shutdown flag is set
/// (signal or `--timeout-secs`).
///
/// # Errors
///
/// Returns [`AtomwriteError::Cancelled`] when the global shutdown flag is set,
/// carrying the exit code of the live signal or the timeout (130/143/124).
#[inline]
pub(crate) fn check_cancel() -> Result<(), AtomwriteError> {
    if crate::signal::is_global_shutdown() {
        return Err(crate::signal::cancelled_error(
            "fuzzy match cancelled by signal or --timeout-secs",
        ));
    }
    Ok(())
}

/// Cooperative cancel poll for fuzzy loops: a local counter plus a mask, so the
/// shutdown flag is read once per `mask + 1` iterations (and on the first one).
pub(crate) struct CancelPoll {
    count: usize,
    mask: usize,
}

impl CancelPoll {
    /// Poll for cheap per-line loops ([`crate::constants::FUZZY_CANCEL_POLL_MASK`]).
    pub(crate) const fn lines() -> Self {
        Self {
            count: 0,
            mask: crate::constants::FUZZY_CANCEL_POLL_MASK,
        }
    }

    /// Poll on every iteration, for expensive work such as diffs or Damerau windows.
    pub(crate) const fn heavy() -> Self {
        Self {
            count: 0,
            mask: crate::constants::FUZZY_CANCEL_POLL_MASK_HEAVY,
        }
    }

    /// Count one iteration; check the shutdown flag when the mask says so.
    ///
    /// # Errors
    ///
    /// Returns whatever [`check_cancel`] returns, and only on the iterations
    /// where the mask makes the flag due: [`AtomwriteError::Cancelled`] with
    /// the exit code of the live signal or the timeout (130/143/124).
    #[inline]
    pub(crate) fn tick(&mut self) -> Result<(), AtomwriteError> {
        let due = self.count & self.mask == 0;
        self.count = self.count.wrapping_add(1);
        if due { check_cancel() } else { Ok(()) }
    }
}

/// Split `content` into lines (like [`str::lines`]) with a single exact
/// reservation and cancel polling.
///
/// # Errors
///
/// Returns [`AtomwriteError::Cancelled`] on shutdown, or
/// [`AtomwriteError::InternalError`] when the line table cannot be allocated.
pub(crate) fn collect_lines(content: &str) -> Result<Vec<&str>, AtomwriteError> {
    let capacity = memchr::memchr_iter(b'\n', content.as_bytes())
        .count()
        .saturating_add(1);
    let mut lines: Vec<&str> = Vec::new();
    lines
        .try_reserve_exact(capacity)
        .map_err(|e| AtomwriteError::InternalError {
            reason: format!("allocation failed for {capacity} fuzzy line slots: {e}"),
        })?;
    let mut poll = CancelPoll::lines();
    for line in content.lines() {
        poll.tick()?;
        lines.push(line);
    }
    Ok(lines)
}
