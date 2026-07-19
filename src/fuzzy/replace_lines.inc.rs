/// Apply a line-range replacement preserving trailing newline of `original`.
pub fn apply_line_replacement(
    original: &str,
    content_lines: &[&str],
    start: usize,
    end: usize,
    new: &str,
) -> String {
    // G-FZZ-037: preserve dominant line ending (CRLF vs LF) instead of always joining with LF.
    let nl = if original.contains("\r\n") { "\r\n" } else { "\n" };
    let before = content_lines[..start].join(nl);
    let after = content_lines[end..].join(nl);
    // Normalize `new` internal newlines to the file's dominant ending.
    let new_norm = if nl == "\r\n" {
        new.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        new.replace("\r\n", "\n")
    };
    let mut out = String::with_capacity(before.len() + new_norm.len() + after.len() + 4);
    if !before.is_empty() {
        out.push_str(&before);
        out.push_str(nl);
    }
    out.push_str(&new_norm);
    if !after.is_empty() {
        out.push_str(nl);
        out.push_str(&after);
    }
    let ends_nl = original.ends_with('\n');
    if ends_nl && !out.ends_with('\n') {
        out.push_str(nl);
    }
    out
}

