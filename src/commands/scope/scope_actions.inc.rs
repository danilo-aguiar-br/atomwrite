// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by scope/mod.rs (A-MONO-001).

fn expand_to_full_line(content: &str, start: usize, end: usize) -> (usize, usize) {
    let bytes = content.as_bytes();
    let line_start = bytes[..start]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |pos| pos + 1);
    let line_end = bytes[end..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(content.len(), |pos| end + pos + 1);

    let before_match = &content[line_start..start];

    if before_match.trim().is_empty() {
        (line_start, line_end)
    } else {
        let content_end = if line_end > 0 && bytes[line_end - 1] == b'\n' {
            line_end - 1
        } else {
            line_end
        };
        let trim_start = content[line_start..start]
            .rfind(|c: char| !c.is_whitespace())
            .map_or(start, |pos| line_start + pos + 1);
        (trim_start, content_end)
    }
}

fn apply_scope_action<'a>(
    text: &'a str,
    delete: bool,
    action: Option<ScopeAction>,
    replace_with: Option<&str>,
) -> std::borrow::Cow<'a, str> {
    if delete {
        return std::borrow::Cow::Owned(String::new());
    }
    if let Some(replacement) = replace_with {
        return std::borrow::Cow::Owned(replacement.to_owned());
    }
    match action {
        Some(ScopeAction::Upper) => std::borrow::Cow::Owned(text.to_uppercase()),
        Some(ScopeAction::Lower) => std::borrow::Cow::Owned(text.to_lowercase()),
        Some(ScopeAction::Titlecase) => std::borrow::Cow::Owned(titlecase(text)),
        Some(ScopeAction::Squeeze) => std::borrow::Cow::Owned(squeeze(text)),
        Some(ScopeAction::Symbols) => std::borrow::Cow::Owned(symbolize(text)),
        Some(ScopeAction::Normalize) => std::borrow::Cow::Owned(text.nfc().collect::<String>()),
        None => std::borrow::Cow::Borrowed(text),
    }
}

fn titlecase(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for c in s.chars() {
        if capitalize_next && c.is_alphabetic() {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
            if c.is_whitespace() || c == '_' || c == '-' {
                capitalize_next = true;
            }
        }
    }
    result
}

fn squeeze(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if Some(c) != prev || !c.is_whitespace() {
            result.push(c);
        }
        prev = Some(c);
    }
    result
}

fn symbolize(s: &str) -> String {
    s.replace("=>", "⇒")
        .replace("->", "→")
        .replace("<-", "←")
        .replace("!=", "≠")
        .replace(">=", "≥")
        .replace("<=", "≤")
        .replace("...", "…")
        .replace("--", "—")
}

fn resolve_patterns(
    query_name: &Option<String>,
    custom_pattern: &Option<String>,
    lang_str: &str,
) -> Result<Vec<String>> {
    if let Some(p) = custom_pattern {
        return Ok(vec![p.clone()]);
    }

    let name = query_name
        .as_deref()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "either --query or --pattern is required".into(),
        })?;

    lookup_prepared_queries(name, lang_str)
}

fn parse_language(lang_str: &str) -> Result<SupportLang> {
    lang_str.parse().map_err(|_| {
        AtomwriteError::InvalidInput {
            reason: format!("unsupported language: {lang_str}"),
        }
        .into()
    })
}

enum ScopeEvent {
    Result(ScopeResult),
}
