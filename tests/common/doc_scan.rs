// SPDX-License-Identifier: MIT OR Apache-2.0

//! Markdown and shell scanning helpers shared by the documentation guards in
//! `tests/docs_coverage.rs`. Nothing here touches the machine: it only reads
//! text handed in by the caller.

use regex::Regex;

/// One fenced code block: the info string, the line number of its first body
/// line and the body text.
pub struct Fence {
    pub info: String,
    pub first_line: usize,
    pub body: String,
}

/// A document split into fenced blocks and prose lines.
pub struct Scan {
    pub fences: Vec<Fence>,
    /// `(1-based line number, line)` of every line outside a fence, including
    /// the fence delimiter lines' neighbours but not the delimiters themselves.
    pub prose: Vec<(usize, String)>,
}

/// Opening fence marker of `line`: the fence character, its length and the info
/// string, or `None` when the line is not a fence delimiter.
fn fence_marker(line: &str) -> Option<(char, usize, String)> {
    let trimmed = line.trim_start();
    let ch = trimmed.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let len = trimmed.chars().take_while(|c| *c == ch).count();
    (len >= 3).then(|| (ch, len, trimmed[len..].trim().to_string()))
}

pub fn scan(text: &str) -> Scan {
    let mut fences = Vec::new();
    let mut prose = Vec::new();
    let mut open: Option<(char, usize, Fence)> = None;
    for (i, line) in text.lines().enumerate() {
        let number = i + 1;
        match (&mut open, fence_marker(line)) {
            (None, Some((ch, len, info))) => {
                let fence = Fence {
                    info,
                    first_line: number + 1,
                    body: String::new(),
                };
                open = Some((ch, len, fence));
            }
            (Some((ch, len, _)), Some((c, l, info)))
                if c == *ch && l >= *len && info.is_empty() =>
            {
                if let Some((_, _, fence)) = open.take() {
                    fences.push(fence);
                }
            }
            (Some((_, _, fence)), _) => {
                fence.body.push_str(line);
                fence.body.push('\n');
            }
            (None, None) => prose.push((number, line.to_string())),
        }
    }
    if let Some((_, _, fence)) = open {
        fences.push(fence);
    }
    Scan { fences, prose }
}

/// Inline code spans of `line` and the line with those spans removed.
pub fn inline_code(line: &str) -> (Vec<String>, String) {
    let chars: Vec<char> = line.chars().collect();
    let mut spans = Vec::new();
    let mut rest = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '`' {
            rest.push(chars[i]);
            i += 1;
            continue;
        }
        let run = chars[i..].iter().take_while(|c| **c == '`').count();
        let body_start = i + run;
        let mut j = body_start;
        let mut close = None;
        while j < chars.len() {
            if chars[j] == '`' {
                let r = chars[j..].iter().take_while(|c| **c == '`').count();
                if r == run {
                    close = Some(j);
                    break;
                }
                j += r;
            } else {
                j += 1;
            }
        }
        if let Some(end) = close {
            spans.push(chars[body_start..end].iter().collect::<String>());
            i = end + run;
        } else {
            rest.extend(&chars[i..body_start]);
            i = body_start;
        }
    }
    (spans, rest)
}

/// Level (1 to 6) and text of an ATX heading line.
pub fn heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    ((1..=6).contains(&hashes) && line[hashes..].starts_with(' '))
        .then(|| (hashes, line[hashes..].trim()))
}

/// GitHub heading slug: lower case, punctuation dropped, whitespace to `-`,
/// accents kept.
pub fn github_slug(text: &str) -> String {
    let link = Regex::new(r"\[([^\]]*)\]\([^)]*\)").expect("regex");
    let tag = Regex::new(r"<[^>]*>").expect("regex");
    let plain = link.replace_all(text, "$1");
    let plain = tag.replace_all(&plain, "");
    // Code spans keep their text in the slug; only the backticks go.
    plain
        .replace('`', "")
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            c if c.is_whitespace() => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// Anchors a document exposes: heading slugs (duplicates get `-1`, `-2`) and
/// explicit `id="…"` or `name="…"` attributes.
pub fn anchors(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, line) in scan(text).prose {
        if let Some((_, title)) = heading(&line) {
            let base = github_slug(title);
            let mut slug = base.clone();
            let mut n = 0;
            while out.contains(&slug) {
                n += 1;
                slug = format!("{base}-{n}");
            }
            out.push(slug);
        }
    }
    let attr = Regex::new(r#"(?:id|name)="([^"]+)""#).expect("regex");
    out.extend(attr.captures_iter(text).map(|c| c[1].to_string()));
    out
}

/// Decode `%XX` escapes of a link fragment.
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (
            bytes[i],
            bytes.get(i + 1).copied().and_then(hex),
            bytes.get(i + 2).copied().and_then(hex),
        ) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (b, _, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Relative link targets of `text` as `(line, target)`; code and absolute URLs
/// are left out.
pub fn relative_links(text: &str) -> Vec<(usize, String)> {
    let inline = Regex::new(r#"\]\(\s*<?([^)\s>]+)>?(?:\s+"[^"]*")?\s*\)"#).expect("regex");
    let reference = Regex::new(r"^\s*\[[^\]]+\]:\s*<?(\S+?)>?\s*$").expect("regex");
    let mut links = Vec::new();
    for (number, line) in scan(text).prose {
        let (_, plain) = inline_code(&line);
        let targets = inline
            .captures_iter(&plain)
            .map(|c| c[1].to_string())
            .chain(reference.captures(&plain).map(|c| c[1].to_string()));
        for target in targets {
            let external = target.contains("://")
                || ["mailto:", "tel:", "data:"]
                    .iter()
                    .any(|p| target.starts_with(p));
            if !external {
                links.push((number, target));
            }
        }
    }
    links
}

/// One word of a shell command line.
#[derive(Debug, PartialEq)]
pub struct Tok {
    pub text: String,
    pub line: usize,
    /// The word contained a quote character, so it is a value, never a flag.
    pub quoted: bool,
}

/// Words of the command that starts at byte `start` of `block` (just after
/// `atomwrite`), up to the first unquoted pipe, `&`, `;`, redirection, closing
/// parenthesis or unescaped newline. `first_line` is the line number of the
/// block's first line.
pub fn command_words(block: &str, start: usize, first_line: usize) -> Vec<Tok> {
    let chars: Vec<(usize, char)> = block[start..]
        .char_indices()
        .map(|(i, c)| (start + i, c))
        .collect();
    let line_at = |byte: usize| first_line + block[..byte].matches('\n').count();
    let mut toks = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut word_at = start;
    let mut quote: Option<char> = None;
    let mut depth = 0usize;
    let mut i = 0;
    let flush = |word: &mut String, quoted: &mut bool, at: usize, toks: &mut Vec<Tok>| {
        if !word.is_empty() || *quoted {
            toks.push(Tok {
                text: std::mem::take(word),
                line: line_at(at),
                quoted: *quoted,
            });
        }
        *quoted = false;
    };
    while i < chars.len() {
        let (at, c) = chars[i];
        let next = chars.get(i + 1).map(|(_, n)| *n);
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else if c == '\\' && q == '"' && next.is_some() {
                i += 1;
                word.push(chars[i].1);
            } else {
                word.push(c);
            }
            i += 1;
            continue;
        }
        match c {
            '\\' if matches!(next, Some('\n' | '\r')) => {
                i += 1;
                if next == Some('\r') && chars.get(i + 1).map(|(_, n)| *n) == Some('\n') {
                    i += 1;
                }
                flush(&mut word, &mut quoted, word_at, &mut toks);
            }
            '\\' if next.is_some() => {
                i += 1;
                word.push(chars[i].1);
            }
            '\'' | '"' => {
                if word.is_empty() && !quoted {
                    word_at = at;
                }
                quote = Some(c);
                quoted = true;
            }
            '$' if next == Some('(') => {
                depth += 1;
                word.push_str("$(");
                i += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
                word.push(')');
            }
            '#' if word.is_empty() && !quoted => {
                while i < chars.len() && chars[i].1 != '\n' {
                    i += 1;
                }
                break;
            }
            ' ' | '\t' => flush(&mut word, &mut quoted, word_at, &mut toks),
            '\n' | '|' | '&' | ';' | '>' | '<' | ')' => break,
            _ => {
                if word.is_empty() && !quoted {
                    word_at = at;
                }
                word.push(c);
            }
        }
        i += 1;
    }
    flush(&mut word, &mut quoted, word_at, &mut toks);
    toks
}

/// Words that may precede `atomwrite` and still leave it in command position.
const COMMAND_PREFIXES: &[&str] = &["sudo", "time", "then", "do", "else", "exec", "command"];

/// Is the text before `atomwrite` on its line a command position?
fn in_command_position(prefix: &str) -> bool {
    let pre = prefix.trim_end();
    if prefix.trim_start().starts_with('#') {
        return false;
    }
    if pre.is_empty() || pre.ends_with(['|', '&', ';', '(', '`', '$', '!', '{', '>']) {
        return true;
    }
    let last = pre.rsplit(char::is_whitespace).next().unwrap_or("");
    COMMAND_PREFIXES.contains(&last) || (last.contains('=') && !last.starts_with('-'))
}

/// Every `atomwrite` invocation in a shell block, as the words that follow the
/// program name.
pub fn invocations(block: &str, first_line: usize) -> Vec<Vec<Tok>> {
    let program = Regex::new(r"atomwrite(?:\.exe)?").expect("regex");
    let mut found = Vec::new();
    for m in program.find_iter(block) {
        let before = block[..m.start()].chars().next_back();
        let after = block[m.end()..].chars().next();
        let glued_before = before.is_some_and(|c| {
            c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '\\' | '$')
        });
        let glued_after = after.is_some_and(|c| {
            !(c.is_whitespace() || matches!(c, ')' | '"' | '\'' | ';' | '|' | '&'))
        });
        let line_start = block[..m.start()].rfind('\n').map_or(0, |p| p + 1);
        if glued_before || glued_after || !in_command_position(&block[line_start..m.start()]) {
            continue;
        }
        found.push(command_words(block, m.end(), first_line));
    }
    found
}

/// Info strings of fences that hold shell commands.
pub const SHELL_INFOS: &[&str] = &[
    "",
    "sh",
    "bash",
    "shell",
    "console",
    "zsh",
    "fish",
    "powershell",
    "ps1",
    "pwsh",
    "text",
    "txt",
    "cmd",
    "bat",
];

/// Counts that must match between the English and Portuguese editions.
#[derive(Debug, PartialEq, Eq)]
pub struct Shape {
    pub h1: usize,
    pub h2: usize,
    pub h3: usize,
    pub fences: usize,
    pub table_rows: usize,
}

pub fn shape(text: &str) -> Shape {
    let scanned = scan(text);
    let level = |n: usize| {
        scanned
            .prose
            .iter()
            .filter(|(_, l)| heading(l).is_some_and(|(lv, _)| lv == n))
            .count()
    };
    Shape {
        h1: level(1),
        h2: level(2),
        h3: level(3),
        fences: scanned.fences.len(),
        table_rows: scanned
            .prose
            .iter()
            .filter(|(_, l)| l.trim_start().starts_with('|'))
            .count(),
    }
}
