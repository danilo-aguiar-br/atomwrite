/// Outcome of a line-range finder: `Ok(None)` is a miss, `Err` is a cancel.
type LineMatch = std::result::Result<Option<(usize, usize)>, AtomwriteError>;

fn match_line_trimmed(content: &[&str], pattern: &[&str]) -> LineMatch {
    if pattern.is_empty() {
        return Ok(None);
    }
    let trimmed_pat: Vec<&str> = pattern.iter().map(|l| l.trim()).collect();
    let mut poll = CancelPoll::lines();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        poll.tick()?;
        for (j, pat_line) in trimmed_pat.iter().enumerate() {
            if content[i + j].trim() != *pat_line {
                continue 'outer;
            }
        }
        return Ok(Some((i, i + pattern.len())));
    }
    Ok(None)
}

fn normalize_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut first = true;
    for word in s.split_whitespace() {
        if !first {
            result.push(' ');
        }
        result.push_str(word);
        first = false;
    }
    result
}

/// Collapse whitespace, then drop the spaces next to `FUZZY_PUNCTUATION_CHARS`.
fn normalize_punctuation_whitespace(s: &str) -> String {
    let normalized = normalize_whitespace(s);
    let is_punct = |c: char| crate::constants::FUZZY_PUNCTUATION_CHARS.contains(c);
    let mut out = String::with_capacity(normalized.len());
    let mut chars = normalized.chars().peekable();
    let mut prev_punct = false;
    while let Some(c) = chars.next() {
        if c == ' ' && (prev_punct || chars.peek().copied().is_some_and(is_punct)) {
            continue;
        }
        prev_punct = is_punct(c);
        out.push(c);
    }
    out
}

/// Find `pattern` in `content` comparing both sides through `normalize`.
fn match_normalized(
    content: &[&str],
    pattern: &[&str],
    normalize: fn(&str) -> String,
) -> LineMatch {
    if pattern.is_empty() {
        return Ok(None);
    }
    let norm_pat: Vec<String> = pattern.iter().map(|l| normalize(l)).collect();
    // Normalizing allocates per line, so one flag read per iteration is noise.
    let mut poll = CancelPoll::heavy();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        poll.tick()?;
        for (j, norm) in norm_pat.iter().enumerate() {
            if normalize(content[i + j]) != *norm {
                continue 'outer;
            }
        }
        return Ok(Some((i, i + pattern.len())));
    }
    Ok(None)
}

fn match_punctuation_normalized(content: &[&str], pattern: &[&str]) -> LineMatch {
    match_normalized(content, pattern, normalize_punctuation_whitespace)
}

fn match_whitespace_normalized(content: &[&str], pattern: &[&str]) -> LineMatch {
    match_normalized(content, pattern, normalize_whitespace)
}

fn match_indent_flexible(content: &[&str], pattern: &[&str]) -> LineMatch {
    if pattern.is_empty() {
        return Ok(None);
    }
    let stripped_pat: Vec<&str> = pattern.iter().map(|l| l.trim_start()).collect();
    let mut poll = CancelPoll::lines();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        poll.tick()?;
        for (j, pat) in stripped_pat.iter().enumerate() {
            if content[i + j].trim_start() != *pat {
                continue 'outer;
            }
        }
        return Ok(Some((i, i + pattern.len())));
    }
    Ok(None)
}

// escape normalize + map: see escape_map.inc.rs (GAP-2026-024)

fn match_trimmed_boundary(content: &[&str], pattern: &[&str]) -> LineMatch {
    let Some(start) = pattern.iter().position(|l| !l.trim().is_empty()) else {
        return Ok(None);
    };
    let Some(end) = pattern.iter().rposition(|l| !l.trim().is_empty()) else {
        return Ok(None);
    };
    let end = end + 1;
    if start >= end {
        return Ok(None);
    }
    match_line_trimmed(content, &pattern[start..end])
}

fn match_unicode_normalized(content: &[&str], pattern: &[&str]) -> LineMatch {
    match_normalized(content, pattern, normalize_unicode_for_match)
}
