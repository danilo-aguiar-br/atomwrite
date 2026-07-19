// SPDX-License-Identifier: MIT OR Apache-2.0

//! Lightweight syntax heuristics (legacy; ast-grep path preferred).

pub(crate) fn syntax_heuristic_check(content: &[u8]) -> Option<String> {
    // Convert to text for bracket counting. Bail early if not valid UTF-8.
    let text = std::str::from_utf8(content).ok()?;

    // 1. Strip line and block comments so they don't confuse the count.
    let stripped = strip_comments(text);

    // 2. Strip string literals (handles both "..." and '...' for Rust/JS-like).
    let stripped = strip_string_literals(&stripped);

    // 3. Count brackets.
    let mut braces = 0i32;
    let mut parens = 0i32;
    let mut brackets = 0i32;
    for c in stripped.chars() {
        match c {
            '{' => braces += 1,
            '}' => braces -= 1,
            '(' => parens += 1,
            ')' => parens -= 1,
            '[' => brackets += 1,
            ']' => brackets -= 1,
            _ => {}
        }
    }
    if braces != 0 {
        return Some(format!(
            "unbalanced braces: {} more {} than {}",
            braces.abs(),
            if braces > 0 { "open" } else { "close" },
            if braces > 0 { "close" } else { "open" }
        ));
    }
    if parens != 0 {
        return Some(format!(
            "unbalanced parentheses: {} more {} than {}",
            parens.abs(),
            if parens > 0 { "open" } else { "close" },
            if parens > 0 { "close" } else { "open" }
        ));
    }
    if brackets != 0 {
        return Some(format!(
            "unbalanced brackets: {} more {} than {}",
            brackets.abs(),
            if brackets > 0 { "open" } else { "close" },
            if brackets > 0 { "close" } else { "open" }
        ));
    }
    None
}

/// Strip line (`//`) and block (`/* ... */`) comments, respecting string
/// literals. This is a best-effort, char-by-char scanner; it is NOT a
/// full lexer. Misbehavior with nested block comments or string-interpolation
/// is acceptable since we only use this for the G72 heuristic check.
pub(crate) fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    // Line comment: skip until newline.
                    chars.next();
                    for nc in chars.by_ref() {
                        if nc == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                }
                Some('*') => {
                    // Block comment: skip until `*/`.
                    chars.next();
                    let mut prev = '\0';
                    for nc in chars.by_ref() {
                        if prev == '*' && nc == '/' {
                            break;
                        }
                        prev = nc;
                    }
                }
                _ => out.push(c),
            }
        } else if c == '"' {
            // String literal: skip until matching unescaped quote.
            out.push(c);
            while let Some(nc) = chars.next() {
                out.push(nc);
                if nc == '\\' {
                    // Skip the escaped character.
                    if let Some(escaped) = chars.next() {
                        out.push(escaped);
                    }
                } else if nc == '"' {
                    break;
                }
            }
        } else if c == '\'' {
            // Char literal (Rust) or single-quote string.
            out.push(c);
            while let Some(nc) = chars.next() {
                out.push(nc);
                if nc == '\\' {
                    if let Some(escaped) = chars.next() {
                        out.push(escaped);
                    }
                } else if nc == '\'' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Strip double-quoted string literals, leaving single chars intact.
/// Used as a second pass after `strip_comments` to remove anything that
/// might still confuse bracket counting (template literals, raw strings).
pub(crate) fn strip_string_literals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut prev = '\0';
    for c in text.chars() {
        if c == '"' && prev != '\\' {
            in_string = !in_string;
        }
        if !in_string {
            out.push(c);
        }
        prev = c;
    }
    out
}
