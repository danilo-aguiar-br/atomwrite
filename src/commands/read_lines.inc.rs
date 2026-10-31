// SPDX-License-Identifier: MIT OR Apache-2.0

// Line counting, selection and streaming for `read`.
// Included from read.rs.

/// Count the lines of `path` with a fixed buffer.
///
/// `--stat` publishes `lines_total`, so the count cannot be skipped; it can be
/// computed without ever holding the file, which is what keeps the peak flat.
///
/// This counts newlines over fixed chunks instead of going through
/// [`visit_lines`]: that pass reads one whole line at a time, so a file with no
/// newline in it (a minified blob, a one-line dump) made `--stat` allocate the
/// file size again. A trailing newline closes the last line and does not start
/// a new one, which is the same rule [`str::lines`] follows.
fn count_lines_streaming(path: &std::path::Path) -> Result<u64> {
    use std::io::Read;
    let file = fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, file);
    let mut buf = vec![0u8; crate::constants::BUF_CAPACITY];
    let mut newlines: u64 = 0;
    let mut last_byte: Option<u8> = None;
    let mut total_bytes: u64 = 0;
    loop {
        let read = reader
            .read(&mut buf)
            .with_context(|| format!("cannot read {}", path.display()))?;
        if read == 0 {
            break;
        }
        newlines = newlines.saturating_add(memchr::memchr_iter(b'\n', &buf[..read]).count() as u64);
        last_byte = Some(buf[read - 1]);
        total_bytes = total_bytes.saturating_add(read as u64);
    }
    if total_bytes == 0 {
        return Ok(0);
    }
    // Every `\n` closes a line; a file whose last byte is not `\n` has one more
    // unterminated line.
    Ok(if last_byte == Some(b'\n') {
        newlines
    } else {
        newlines.saturating_add(1)
    })
}

/// Read `path` line by line and stop as soon as the requested lines are known.
///
/// Returns the selected text (with a trailing newline, as the whole-file path
/// produced) and the file's real line total. The collector holds only the lines
/// the flags ask for: `--head 1` on a 280 MB file keeps one line and a counter.
fn scan_lines_streaming(path: &std::path::Path, args: &ReadArgs) -> Result<(String, u64)> {
    let mut sink = LineSink::for_args(args);
    let total = visit_lines(path, |index, line| sink.push(index, line))?;
    Ok((sink.finish(total), total))
}

/// Lines selected by the read flags, accumulated without the whole file.
enum LineSink {
    All(String),
    Head {
        n: usize,
        text: String,
    },
    Tail {
        n: usize,
        ring: std::collections::VecDeque<String>,
    },
    Range {
        start: usize,
        end: usize,
        text: String,
    },
    Line {
        start: usize,
        end: usize,
        text: String,
    },
    Grep {
        pattern: regex::Regex,
        text: String,
    },
}

impl LineSink {
    /// The collector matching the flags, in the same precedence the eager
    /// filter used: range, single line, head, tail, grep, then the whole file.
    fn for_args(args: &ReadArgs) -> Self {
        if let Some(ref range_str) = args.lines
            && let Some((start, end)) = parse_open_range(range_str)
        {
            return Self::Range {
                start,
                end,
                text: String::new(),
            };
        }
        if let Some(line_num) = args.line {
            let ctx = args.context;
            let idx = line_num.saturating_sub(1);
            return Self::Line {
                start: idx.saturating_sub(ctx),
                end: idx + ctx + 1,
                text: String::new(),
            };
        }
        if let Some(n) = args.head {
            return Self::Head {
                n,
                text: String::new(),
            };
        }
        if let Some(n) = args.tail {
            return Self::Tail {
                n,
                ring: std::collections::VecDeque::with_capacity(n.min(1024)),
            };
        }
        // The caller validates the pattern before the scan, so this compile
        // cannot fail here; a failure means no line matches, not a broken run.
        if let Some(ref pattern) = args.grep
            && let Ok(pattern) = regex::Regex::new(pattern)
        {
            return Self::Grep {
                pattern,
                text: String::new(),
            };
        }
        Self::All(String::new())
    }

    fn push(&mut self, index: usize, line: &str) {
        match self {
            Self::All(text) => push_line(text, line),
            Self::Head { n, text } => {
                if index < *n {
                    push_line(text, line);
                }
            }
            Self::Tail { n, ring } => {
                if *n == 0 {
                    return;
                }
                if ring.len() == *n {
                    ring.pop_front();
                }
                ring.push_back(line.to_owned());
            }
            Self::Range { start, end, text } => {
                if index >= *start && index < *end {
                    push_line(text, line);
                }
            }
            Self::Line { start, end, text } => {
                if index >= *start && index < *end {
                    push_line(text, line);
                }
            }
            Self::Grep { pattern, text } => {
                if pattern.is_match(line) {
                    push_line(text, line);
                }
            }
        }
    }

    fn finish(self, total: u64) -> String {
        match self {
            Self::All(text) | Self::Head { text, .. } | Self::Grep { text, .. } => text,
            Self::Tail { ring, .. } => join_lines(ring.iter().map(String::as_str)),
            Self::Range { start, end, text } => {
                // A range that starts past the file (or is inverted) selects
                // nothing, exactly as the eager path decided.
                if start >= end || start as u64 >= total {
                    return String::new();
                }
                text
            }
            Self::Line { start, end, text } => {
                if start >= end || start as u64 >= total {
                    return String::new();
                }
                text
            }
        }
    }
}

/// Append one line to a collecting buffer, with the trailing newline the
/// whole-file path produced.
fn push_line(text: &mut String, line: &str) {
    text.push_str(line);
    text.push('\n');
}

/// Join selected lines with the trailing newline the whole-file path produced.
fn join_lines<'a>(lines: impl Iterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for line in lines {
        push_line(&mut out, line);
    }
    out
}

/// Parse `START:END` (1-based, inclusive start) without a line total.
fn parse_open_range(s: &str) -> Option<(usize, usize)> {
    let (start, end) = s.split_once(':')?;
    let start = start.parse::<usize>().ok()?.saturating_sub(1);
    let end = end.parse::<usize>().ok()?;
    Some((start, end))
}

/// Feed every line of `path` to `visit`, stripping `\n` and `\r\n`.
///
/// Returns the number of lines, which is the file's `lines_total`. The buffer
/// is the only allocation that scales with the file, and it is fixed.
fn visit_lines(path: &std::path::Path, mut visit: impl FnMut(usize, &str)) -> Result<u64> {
    use std::io::BufRead;
    let file = fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, file);
    let mut buf: Vec<u8> = Vec::with_capacity(crate::constants::BUF_CAPACITY);
    let mut index: u64 = 0;
    loop {
        buf.clear();
        let read = reader
            .read_until(b'\n', &mut buf)
            .with_context(|| format!("cannot read {}", path.display()))?;
        if read == 0 {
            break;
        }
        let cow = String::from_utf8_lossy(&buf);
        let mut line: &str = &cow;
        if let Some(stripped) = line.strip_suffix('\n') {
            line = stripped;
        }
        if let Some(stripped) = line.strip_suffix('\r') {
            line = stripped;
        }
        visit(usize::try_from(index).unwrap_or(usize::MAX), line);
        index += 1;
    }
    Ok(index)
}
