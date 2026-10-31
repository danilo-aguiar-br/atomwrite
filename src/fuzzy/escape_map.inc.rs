// Escape-normalized match with original-byte offset mapping (GAP-2026-024).
//
// Normalizing `\n`/`\t` expands the string; byte indices in the normalized
// view MUST be mapped back to the original content before slicing.

/// Normalize common escape sequences and build a byte-offset map.
///
/// `map[norm_byte_index]` is the corresponding start byte in `original`.
/// `map` has length `normalized.len() + 1`; the last entry is `original.len()`
/// so exclusive ends map correctly.
fn normalize_escapes_with_map(
    original: &str,
) -> std::result::Result<(String, Vec<usize>), AtomwriteError> {
    let bytes = original.as_bytes();
    let mut poll = CancelPoll::lines();
    let mut out = String::with_capacity(original.len());
    let mut map: Vec<usize> = Vec::with_capacity(original.len() + 1);
    let mut i = 0usize;
    while i < bytes.len() {
        poll.tick()?;
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            let repl = match bytes[i + 1] {
                b'n' => Some('\n'),
                b't' => Some('\t'),
                b'"' => Some('"'),
                b'\\' => Some('\\'),
                _ => None,
            };
            if let Some(ch) = repl {
                let start = out.len();
                out.push(ch);
                for _ in start..out.len() {
                    map.push(i);
                }
                i += 2;
                continue;
            }
        }
        // Copy one Unicode scalar from original.
        // `get` keeps a non-boundary offset from panicking; `i` only ever
        // advances by whole scalars or by an ASCII escape pair, so it never misses.
        let Some(ch) = original.get(i..).and_then(|rest| rest.chars().next()) else {
            break;
        };
        let ch_len = ch.len_utf8();
        let start = out.len();
        out.push(ch);
        for _ in start..out.len() {
            map.push(i);
        }
        i += ch_len;
    }
    map.push(original.len());
    Ok((out, map))
}

/// Locate `pattern` after escape normalization; return **original** byte range.
fn match_escape_normalized(
    content: &str,
    pattern: &str,
) -> std::result::Result<Option<(usize, usize)>, AtomwriteError> {
    match_escape_normalized_capped(
        content,
        pattern,
        crate::constants::FUZZY_ESCAPE_MAP_MAX_CONTENT_BYTES,
    )
}

/// [`match_escape_normalized`] with an explicit content cap.
///
/// The offset map costs 8 bytes per content byte, so content above `max_content_bytes`
/// skips this strategy (`Ok(None)`, nothing allocated) and the cascade moves on.
fn match_escape_normalized_capped(
    content: &str,
    pattern: &str,
    max_content_bytes: usize,
) -> std::result::Result<Option<(usize, usize)>, AtomwriteError> {
    let (norm_pat, _) = normalize_escapes_with_map(pattern)?;
    // No escapes in pattern → nothing for this strategy to do.
    if norm_pat == pattern {
        return Ok(None);
    }
    if content.len() > max_content_bytes {
        tracing::debug!(
            content_bytes = content.len(),
            max_content_bytes,
            "escape_normalized skipped: content above the offset-map cap"
        );
        return Ok(None);
    }
    let (norm_content, cmap) = normalize_escapes_with_map(content)?;
    let Some(norm_pos) = norm_content.find(&norm_pat) else {
        return Ok(None);
    };
    let norm_end = norm_pos + norm_pat.len();
    if norm_end >= cmap.len() {
        return Ok(None);
    }
    let orig_start = cmap[norm_pos];
    let orig_end = cmap[norm_end];
    if orig_start < orig_end && orig_end <= content.len() {
        // Reject if the mapped slice is not valid UTF-8 boundaries.
        if !content.is_char_boundary(orig_start) || !content.is_char_boundary(orig_end) {
            return Ok(None);
        }
        Ok(Some((orig_start, orig_end)))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod escape_map_tests {
    use super::*;

    #[test]
    fn maps_expanded_newline_escape() {
        let content = "a\\nb\\nc";
        let pattern = "a\\nb";
        let (s, e) = match_escape_normalized(content, pattern)
            .expect("not cancelled")
            .expect("match");
        assert_eq!(&content[s..e], "a\\nb");
    }

    /// Above the cap the strategy is skipped: a match that needs the offset map is not found.
    #[test]
    fn skips_strategy_above_content_cap_without_building_the_map() {
        let content = "a\\nb\\nc";
        let pattern = "a\\nb";
        let hit = match_escape_normalized_capped(content, pattern, content.len())
            .expect("not cancelled");
        assert!(hit.is_some(), "at the cap the strategy still runs");
        let skipped = match_escape_normalized_capped(content, pattern, content.len() - 1)
            .expect("skipping is not an error");
        assert!(skipped.is_none(), "above the cap the strategy is skipped");
    }

    /// The named cap is used by the public entry point and sits under the cascade cap.
    #[test]
    fn escape_cap_constant_is_wired() {
        let big = "x".repeat(crate::constants::FUZZY_ESCAPE_MAP_MAX_CONTENT_BYTES + 1) + "\\n";
        assert!(
            match_escape_normalized(&big, "q\\nr")
                .expect("skipping is not an error")
                .is_none()
        );
    }

    #[test]
    fn rejects_when_no_escapes_in_pattern() {
        assert!(
            match_escape_normalized("hello", "hello")
                .expect("not cancelled")
                .is_none()
        );
    }
}
