// SPDX-License-Identifier: MIT OR Apache-2.0
// Strict unified-diff application for `apply`. Included from `apply.rs`.
//
// Every context line and every removed line of a hunk must match the file
// (trailing whitespace aside); a hunk that does not match fails the whole
// patch with `MATCH_FAILED` before anything is written.

/// One body line of a hunk.
enum HunkOp<'a> {
    /// Unchanged line; must be present in the file.
    Context(&'a str),
    /// Removed line; must be present in the file.
    Delete(&'a str),
    /// Added line.
    Insert(&'a str),
}

/// One `@@` section of a unified diff.
struct Hunk<'a> {
    /// 1-based first line of the old side, as written in the header.
    ///
    /// `None` when the header carries no line numbers (`@@` or
    /// `@@ ... @@`); the hunk is then located by its context lines.
    old_start: Option<usize>,
    /// Old-side line count, as written in the header.
    old_count: Option<usize>,
    ops: Vec<HunkOp<'a>>,
}

impl<'a> Hunk<'a> {
    /// Lines the file must contain, in order (context and removed lines).
    fn old_side(&self) -> Vec<&'a str> {
        self.ops
            .iter()
            .filter_map(|op| match op {
                HunkOp::Context(t) | HunkOp::Delete(t) => Some(*t),
                HunkOp::Insert(_) => None,
            })
            .collect()
    }

    /// Added lines in this hunk.
    fn inserts(&self) -> usize {
        self.ops
            .iter()
            .filter(|op| matches!(op, HunkOp::Insert(_)))
            .count()
    }

    /// Drop blank padding lines the header does not account for.
    fn trim_padding(&mut self) {
        let Some(count) = self.old_count else {
            return;
        };
        while self.old_side().len() > count && matches!(self.ops.last(), Some(HunkOp::Context(""))) {
            self.ops.pop();
        }
    }

    /// Header text quoted by an error, with or without line numbers.
    fn header_label(&self) -> String {
        self.old_start.map_or_else(
            || "@@ (no line numbers)".to_owned(),
            |start| format!("@@ -{start}"),
        )
    }
}

fn apply_unified(original: &str, patch: &str) -> Result<(String, usize, Option<usize>)> {
    let hunks = parse_unified_hunks(patch)?;
    if hunks.is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: "no `@@ -a,b +c,d @@` hunk found in unified patch".into(),
        }
        .into());
    }
    // A hunk whose old side and new side are both empty moves no bytes. GNU
    // `patch(1)` refuses such a header as malformed; accepting it would report
    // `hunks_applied` for a write that changes nothing.
    if let Some(empty) = hunks.iter().position(|h| h.old_side().is_empty() && h.inserts() == 0) {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "unified hunk {}/{} ({}) has neither context, removed nor added lines; \
                 refusing a no-op hunk",
                empty + 1,
                hunks.len(),
                hunks[empty].header_label()
            ),
        }
        .into());
    }
    let total = hunks.len();
    let mut lines: Vec<&str> = original.lines().collect();
    // Length change of the hunks applied so far plus how far the last one
    // landed from where its header said (`patch(1)` offset semantics).
    let mut drift: i64 = 0;
    let mut first_line = None;

    for (index, hunk) in hunks.iter().enumerate() {
        let old_side = hunk.old_side();
        let (pos, expected) = match hunk.old_start {
            Some(old_start) => {
                // An insert-only hunk names the line AFTER which it inserts.
                let base = if old_side.is_empty() {
                    old_start
                } else {
                    old_start.saturating_sub(1)
                };
                let expected = (base as i64 + drift).clamp(0, lines.len() as i64) as usize;
                let Some(pos) = locate_hunk(&lines, &old_side, expected) else {
                    return Err(
                        hunk_mismatch(index + 1, total, hunk, &lines, expected, &old_side).into(),
                    );
                };
                (pos, expected)
            }
            // A header with no numbers (`@@` or `@@ ... @@`) locates the hunk
            // by its context alone, so the file decides where it goes. There
            // is no header position to drift from, so `expected` is the hit.
            None => {
                let pos = locate_hunk_by_context(&lines, &old_side, index + 1, total, hunk)?;
                (pos, pos)
            }
        };
        if index == 0 {
            // The line the first hunk landed on, 1-based. The caller cannot
            // infer it: `patch(1)` may shift a hunk from its header.
            first_line = Some(pos + 1);
        }

        // Context lines keep the FILE's text; only added lines come from the patch.
        let mut new_lines: Vec<&str> = Vec::with_capacity(hunk.ops.len());
        let mut cursor = pos;
        for op in &hunk.ops {
            match op {
                HunkOp::Context(_) => {
                    new_lines.push(lines[cursor]);
                    cursor += 1;
                }
                HunkOp::Delete(_) => cursor += 1,
                HunkOp::Insert(text) => new_lines.push(text),
            }
        }
        let new_len = new_lines.len() as i64;
        lines.splice(pos..pos + old_side.len(), new_lines);
        drift += new_len - old_side.len() as i64 + (pos as i64 - expected as i64);
    }

    let mut result = lines.join("\n");
    if original.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    Ok((result, total, first_line))
}

/// Position of `old_side` in `lines`: at `expected`, else the nearest exact
/// match within [`crate::constants::PATCH_MAX_HUNK_SHIFT`] lines.
fn locate_hunk(lines: &[&str], old_side: &[&str], expected: usize) -> Option<usize> {
    if old_side.is_empty() {
        return Some(expected.min(lines.len()));
    }
    let fits = |pos: usize| {
        pos + old_side.len() <= lines.len()
            && old_side
                .iter()
                .zip(&lines[pos..])
                .all(|(want, have)| want.trim_end() == have.trim_end())
    };
    if fits(expected) {
        return Some(expected);
    }
    for shift in 1..=crate::constants::PATCH_MAX_HUNK_SHIFT {
        if let Some(earlier) = expected.checked_sub(shift)
            && fits(earlier)
        {
            return Some(earlier);
        }
        if fits(expected + shift) {
            return Some(expected + shift);
        }
    }
    None
}

/// Position of a hunk whose header carries no numbers, found by its context.
///
/// The context must occur exactly once. Zero places is a miss; more than one
/// is refused with the line of each place, because picking one would be the
/// guess this verb exists to avoid.
fn locate_hunk_by_context(
    lines: &[&str],
    old_side: &[&str],
    number: usize,
    total: usize,
    hunk: &Hunk<'_>,
) -> Result<usize> {
    if old_side.is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "unified hunk {number}/{total} ({}) has neither line numbers nor context; \
                 add a `@@ -a,b +c,d @@` header or the lines around the change",
                hunk.header_label()
            ),
        }
        .into());
    }
    let mut hits = Vec::new();
    if old_side.len() <= lines.len() {
        for pos in 0..=lines.len() - old_side.len() {
            let matches = old_side
                .iter()
                .zip(&lines[pos..])
                .all(|(want, have)| want.trim_end() == have.trim_end());
            if matches {
                hits.push(pos);
            }
        }
    }
    match hits.len() {
        0 => Err(AtomwriteError::MatchFailed {
            reason: format!(
                "unified hunk {number}/{total} (@@ without line numbers) does not match any line \
                 of the file; nothing was written"
            ),
            best_candidate: None,
            candidates: None,
        }
        .into()),
        1 => Ok(hits[0]),
        _ => {
            let at = hits
                .iter()
                .map(|pos| (pos + 1).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(AtomwriteError::MatchAmbiguous {
                reason: format!(
                    "unified hunk {number}/{total} (@@ without line numbers) matches {} places, \
                     at lines {at}; add context or line numbers",
                    hits.len()
                ),
                count: hits.len() as u64,
                best_candidate: None,
            }
            .into())
        }
    }
}

/// `MATCH_FAILED` naming the hunk, the first line that disagrees, what the
/// patch expected there and what the file has.
fn hunk_mismatch(
    number: usize,
    total: usize,
    hunk: &Hunk<'_>,
    lines: &[&str],
    expected: usize,
    old_side: &[&str],
) -> AtomwriteError {
    let at = (0..old_side.len())
        .find(|k| {
            lines
                .get(expected + k)
                .is_none_or(|have| have.trim_end() != old_side[*k].trim_end())
        })
        .unwrap_or(0);
    let line_no = expected + at + 1;
    let want = clip(old_side.get(at).copied().unwrap_or(""));
    let have = lines.get(expected + at).map(|l| clip(l));
    let found = have
        .as_deref()
        .map_or_else(|| "<end of file>".to_owned(), |l| format!("{l:?}"));
    AtomwriteError::MatchFailed {
        reason: format!(
            "unified hunk {number}/{total} ({}) does not match the file: line {line_no} \
             expected {want:?}, found {found}; nothing was written",
            hunk.header_label()
        ),
        best_candidate: Some(Box::new(crate::ndjson_types::BestCandidate {
            text: have,
            line: Some(line_no as u64),
            column: Some(1),
            similarity: None,
            strategy: Some("unified-hunk".to_owned()),
            diff_preview: None,
        })),
        candidates: None,
    }
}

/// Truncate a line quoted in an error message.
fn clip(line: &str) -> String {
    line.chars()
        .take(crate::constants::PATCH_ERROR_SNIPPET_CHARS)
        .collect()
}

fn parse_unified_hunks(patch: &str) -> Result<Vec<Hunk<'_>>> {
    let mut hunks = Vec::with_capacity(crate::constants::PATCH_HUNKS_CAPACITY);
    let mut current: Option<Hunk<'_>> = None;

    for (number, line) in patch.lines().enumerate() {
        if line == "@@" || line.starts_with("@@ ") {
            if let Some(mut done) = current.take() {
                done.trim_padding();
                hunks.push(done);
            }
            let header = line.strip_prefix("@@ ").unwrap_or("");
            let Some((old_start, old_count)) = parse_hunk_header(header) else {
                return Err(AtomwriteError::InvalidInput {
                    reason: format!(
                        "invalid hunk header at patch line {}: {:?}",
                        number + 1,
                        clip(line)
                    ),
                }
                .into());
            };
            current = Some(Hunk {
                old_start,
                old_count,
                ops: Vec::with_capacity(crate::constants::PATCH_HUNK_OPS_CAPACITY),
            });
        } else if let Some(ref mut h) = current {
            if let Some(text) = line.strip_prefix('+') {
                h.ops.push(HunkOp::Insert(text));
            } else if let Some(text) = line.strip_prefix('-') {
                h.ops.push(HunkOp::Delete(text));
            } else if let Some(text) = line.strip_prefix(' ') {
                h.ops.push(HunkOp::Context(text));
            } else if !line.starts_with('\\') {
                h.ops.push(HunkOp::Context(line));
            }
        }
    }

    if let Some(mut done) = current {
        done.trim_padding();
        hunks.push(done);
    }
    Ok(hunks)
}

/// `(old_start, old_count)` from the text after `@@ `.
///
/// `@@` and `@@ ... @@` carry no numbers and yield `(None, None)`: the hunk is
/// then located by its context. A header that names the old side without the
/// new one is malformed, so a partial parse is always `None`.
fn parse_hunk_header(header: &str) -> Option<(Option<usize>, Option<usize>)> {
    let header = header.trim();
    if header.is_empty() || header.starts_with(crate::constants::PATCH_ELLIPSIS_MARKER) {
        return Some((None, None));
    }
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let old_part = parts[0].strip_prefix('-')?;
    parts[1].strip_prefix('+')?;
    let (start, count) = parse_range(old_part)?;
    Some((Some(start), Some(count)))
}

fn parse_range(range: &str) -> Option<(usize, usize)> {
    if let Some((start, count)) = range.split_once(',') {
        Some((start.parse().ok()?, count.parse().ok()?))
    } else {
        Some((range.parse().ok()?, 1))
    }
}
