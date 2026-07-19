fn match_line_trimmed(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let trimmed_pat: Vec<&str> = pattern.iter().map(|l| l.trim()).collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, pat_line) in trimmed_pat.iter().enumerate() {
            if content[i + j].trim() != *pat_line {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
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

fn normalize_punctuation_whitespace(s: &str) -> String {
    // Function-local `static LazyLock` (not `const`): compiled once, stable
    // address, shared across all call sites. `const` would re-compile on each use.
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"\s*([(){}\[\]<>,;:])\s*").expect("static regex is valid")
    });
    RE.replace_all(&normalize_whitespace(s), "$1").to_string()
}

fn match_punctuation_normalized(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let norm_pat: Vec<String> = pattern
        .iter()
        .map(|l| normalize_punctuation_whitespace(l))
        .collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, norm) in norm_pat.iter().enumerate() {
            if normalize_punctuation_whitespace(content[i + j]) != *norm {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn match_whitespace_normalized(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let norm_pat: Vec<String> = pattern.iter().map(|l| normalize_whitespace(l)).collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, norm) in norm_pat.iter().enumerate() {
            if normalize_whitespace(content[i + j]) != *norm {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn match_indent_flexible(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let stripped_pat: Vec<&str> = pattern.iter().map(|l| l.trim_start()).collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, pat) in stripped_pat.iter().enumerate() {
            if content[i + j].trim_start() != *pat {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

fn normalize_escapes(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

fn match_escape_normalized(content: &str, pattern: &str) -> Option<(usize, usize)> {
    let norm_pat = normalize_escapes(pattern);
    if norm_pat == pattern {
        return None;
    }
    let norm_content = normalize_escapes(content);
    if let Some(norm_pos) = norm_content.find(&norm_pat) {
        let end_pos = norm_pos + norm_pat.len();
        if end_pos <= content.len() {
            return Some((norm_pos, end_pos));
        }
    }
    None
}

fn match_trimmed_boundary(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    let start = pattern.iter().position(|l| !l.trim().is_empty())?;
    let end = pattern.iter().rposition(|l| !l.trim().is_empty())? + 1;
    if start >= end {
        return None;
    }
    let trimmed = &pattern[start..end];
    match_line_trimmed(content, trimmed)
}

fn match_unicode_normalized(content: &[&str], pattern: &[&str]) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }
    let norm_pat: Vec<String> = pattern
        .iter()
        .map(|l| normalize_unicode_for_match(l))
        .collect();
    'outer: for i in 0..content.len().saturating_sub(pattern.len() - 1) {
        for (j, norm) in norm_pat.iter().enumerate() {
            if normalize_unicode_for_match(content[i + j]) != *norm {
                continue 'outer;
            }
        }
        return Some((i, i + pattern.len()));
    }
    None
}

