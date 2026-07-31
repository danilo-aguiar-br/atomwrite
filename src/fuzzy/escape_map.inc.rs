// Escape-normalized match with original-byte offset mapping (GAP-2026-024).
//
// Normalizing `\n`/`\t` expands the string; byte indices in the normalized
// view MUST be mapped back to the original content before slicing.

/// Normalize common escape sequences and build a byte-offset map.
///
/// `map[norm_byte_index]` is the corresponding start byte in `original`.
/// `map` has length `normalized.len() + 1`; the last entry is `original.len()`
/// so exclusive ends map correctly.
fn normalize_escapes_with_map(original: &str) -> (String, Vec<usize>) {
    let bytes = original.as_bytes();
    let mut out = String::with_capacity(original.len());
    let mut map: Vec<usize> = Vec::with_capacity(original.len() + 1);
    let mut i = 0usize;
    while i < bytes.len() {
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
        let ch = original[i..].chars().next().expect("valid utf-8 slice");
        let ch_len = ch.len_utf8();
        let start = out.len();
        out.push(ch);
        for _ in start..out.len() {
            map.push(i);
        }
        i += ch_len;
    }
    map.push(original.len());
    (out, map)
}

/// Locate `pattern` after escape normalization; return **original** byte range.
fn match_escape_normalized(content: &str, pattern: &str) -> Option<(usize, usize)> {
    let (norm_pat, _) = normalize_escapes_with_map(pattern);
    // No escapes in pattern → nothing for this strategy to do.
    if norm_pat == pattern {
        return None;
    }
    let (norm_content, cmap) = normalize_escapes_with_map(content);
    let norm_pos = norm_content.find(&norm_pat)?;
    let norm_end = norm_pos + norm_pat.len();
    if norm_end >= cmap.len() {
        return None;
    }
    let orig_start = cmap[norm_pos];
    let orig_end = cmap[norm_end];
    if orig_start < orig_end && orig_end <= content.len() {
        // Reject if the mapped slice is not valid UTF-8 boundaries.
        if !content.is_char_boundary(orig_start) || !content.is_char_boundary(orig_end) {
            return None;
        }
        Some((orig_start, orig_end))
    } else {
        None
    }
}

#[cfg(test)]
mod escape_map_tests {
    use super::*;

    #[test]
    fn maps_expanded_newline_escape() {
        let content = "a\\nb\\nc";
        let pattern = "a\\nb";
        let (s, e) = match_escape_normalized(content, pattern).expect("match");
        assert_eq!(&content[s..e], "a\\nb");
    }

    #[test]
    fn rejects_when_no_escapes_in_pattern() {
        assert!(match_escape_normalized("hello", "hello").is_none());
    }
}
