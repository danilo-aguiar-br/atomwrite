// SPDX-License-Identifier: MIT OR Apache-2.0

//! Small fuzzy text helpers (offsets, counts, truncation).

pub(crate) fn find_str(haystack: &str, needle: &str) -> Option<usize> {
    memchr::memmem::find(haystack.as_bytes(), needle.as_bytes())
}


pub(crate) fn count_occurrences(haystack: &str, needle: &str) -> u64 {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0u64;
    let mut start = 0usize;
    while let Some(pos) = find_str(&haystack[start..], needle) {
        count += 1;
        start += pos + needle.len();
        if start >= haystack.len() {
            break;
        }
    }
    count
}


pub(crate) fn truncate_text(s: &str) -> String {
    if s.chars().count() <= crate::constants::FUZZY_BEST_CANDIDATE_TEXT_MAX {
        return s.to_string();
    }
    s.chars().take(crate::constants::FUZZY_BEST_CANDIDATE_TEXT_MAX).collect()
}


pub(crate) fn line_col_of_offset(content: &str, byte_offset: usize) -> (u64, u64) {
    let mut line = 1u64;
    let mut col = 1u64;
    for (i, ch) in content.char_indices() {
        if i >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}


pub(crate) fn byte_offset_of_line(content: &str, line_idx: usize) -> usize {
    if line_idx == 0 {
        return 0;
    }
    let mut seen = 0usize;
    for (i, ch) in content.char_indices() {
        if ch == '\n' {
            seen += 1;
            if seen == line_idx {
                return i + 1;
            }
        }
    }
    content.len()
}

