// SPDX-License-Identifier: MIT OR Apache-2.0

// Line counting, selection and streaming for `read`.
// Included from read.rs.

/// Count the lines of `path` with a fixed buffer.
///
/// `--stat` publishes `lines_total`, so the count cannot be skipped; it can be
/// computed without ever holding the file, which is what keeps the peak flat.
///
/// This counts newlines over fixed chunks instead of going through
/// [`visit_raw_lines`]: that pass reads one whole line at a time, so a file with no
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
///
/// Every selector -- ranges, single line, head, tail, grep, `--number` and
/// `--no-comments` -- is composed here, the same way raw composes it. Only a
/// structural selector needs the whole source, because tree-sitter resolves it
/// over the complete bytes.
/// One streaming text scan, with the byte ceiling already applied.
struct ScannedText {
    /// Selected lines, cut at the ceiling when it was hit.
    content: String,
    /// Total lines in the file, before selection and before the cut.
    total: u64,
    /// Whether the ceiling cut the selection.
    truncated: bool,
    /// Ready-made command that reads the rest, when the cut allows one.
    next: Option<String>,
}

/// Whether the call already narrowed the selection to a range.
///
/// A narrowed call keeps the newline cut, because an outline boundary would
/// drop lines the caller named one by one.
fn has_line_filter(args: &ReadArgs) -> bool {
    !args.lines.is_empty()
        || args.line.is_some()
        || args.head.is_some()
        || args.tail.is_some()
        || args.grep.is_some()
        || args.no_comments
}

fn scan_lines_streaming(
    path: &std::path::Path,
    args: &ReadArgs,
    max_size: u64,
) -> Result<ScannedText> {
    let cap = if args.all {
        usize::MAX
    } else {
        crate::config::read_max_output_bytes()
    };
    if args.symbol.is_some() || args.enclosing {
        return scan_structural_span(path, args, max_size, cap);
    }
    let number = numbering_enabled(args);
    let mut selector = LineSelector::new(args);
    let mut out = String::new();
    let mut ends: Vec<(usize, usize)> = Vec::new();
    let mut truncated = false;
    let mut last_line = 0usize;
    let total = {
        let mut emit = |index: usize, line: &[u8]| {
            if truncated {
                return;
            }
            let before = out.len();
            push_text_line(&mut out, index, line, number);
            if out.len() > cap {
                out.truncate(before);
                truncated = true;
                return;
            }
            ends.push((index + 1, out.len()));
            last_line = index + 1;
        };
        let total = visit_raw_lines(path, |index, line| selector.push(index, line, &mut emit))?;
        // `emit` is a no-op after the cut, so finishing the selector costs
        // nothing and the borrow ends with the block.
        selector.finish(&mut emit);
        total
    };
    ensure_ranges_select(path, total, args)?;
    // Align the cut to the end of the last complete outline item, so the next
    // read starts at a symbol boundary instead of inside one. Numbering and
    // line filters change the text, so the source prefix is not on hand there.
    if truncated
        && !has_line_filter(args)
        && !number
        && let Some(end_line) = align_cut_to_outline(path, &out, last_line)
    {
        if let Some(&(_, offset)) = ends.iter().find(|(line, _)| *line == end_line) {
            out.truncate(offset);
            last_line = end_line;
        }
    }
    let next = (truncated && last_line > 0)
        .then(|| format!("--lines {}:{total} (use --all for everything)", last_line + 1));
    Ok(ScannedText {
        content: out,
        total,
        truncated,
        next,
    })
}

/// Cut `cut_line` back to the end of the last complete outline item.
///
/// Code uses the same tree-sitter outline the `outline` verb publishes. A
/// Markdown file has no tree here, so the last heading found in the prefix
/// marks the start of the section that was cut, and the line before it ends the
/// previous one.
fn align_cut_to_outline(
    path: &std::path::Path,
    prefix: &str,
    cut_line: usize,
) -> Option<usize> {
    if cut_line < 2 {
        return None;
    }
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if ext == "md" || ext == "markdown" {
        let mut last_heading = None;
        for (index, line) in prefix.lines().enumerate() {
            if line.starts_with('#') {
                last_heading = Some(index + 1);
            }
        }
        return (last_heading.filter(|line| *line > 1)).map(|line| line - 1);
    }
    crate::commands::outline::collect_outline_for_path(path, prefix.as_bytes(), None)
        .iter()
        .map(|item| item.end_line)
        .filter(|end| *end < cut_line)
        .max()
}

/// Answer `--symbol` and `--enclosing` from the same byte slice raw carves.
fn scan_structural_span(
    path: &std::path::Path,
    args: &ReadArgs,
    max_size: u64,
    cap: usize,
) -> Result<ScannedText> {
    let data = crate::file_io::read_file_bytes(path, max_size)?;
    let (start, end) = if let Some(name) = &args.symbol {
        let spans = crate::commands::outline::symbol_spans(path, &data, None, name);
        let span = crate::commands::outline::resolve_symbol_span(&spans, name, path)
            .map_err(anyhow::Error::from)?;
        (span.start, span.end)
    } else {
        let line = args.line.ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "--enclosing needs --line N".to_string(),
        })?;
        crate::commands::outline::enclosing_span(path, &data, None, line).ok_or_else(|| {
            AtomwriteError::InvalidInput {
                reason: format!(
                    "no structural block contains line {line} in {}",
                    path.display()
                ),
            }
        })?
    };
    let file_total = line_spans(&data).len() as u64;
    let slice = &data[start..end];
    // A range inside a symbol is counted from the symbol's first line, the
    // same slice raw carves; `lines_total` still reports the file it came from.
    let slice_total = line_spans(slice).len();
    ensure_ranges_select(path, slice_total as u64, args)?;
    let mut rendered = render_selected_text(slice, args);
    let truncated = rendered.len() > cap;
    if truncated {
        let cut = memchr::memrchr(b'\n', &rendered.as_bytes()[..cap])
            .map(|index| index + 1)
            .unwrap_or(0);
        rendered.truncate(cut);
    }
    Ok(ScannedText {
        content: rendered,
        total: file_total,
        truncated,
        next: None,
    })
}

/// Render the selected lines of an in-memory slice as default text.
fn render_selected_text(data: &[u8], args: &ReadArgs) -> String {
    let spans = line_spans(data);
    let total = spans.len();
    let indices = select_line_indices(&spans, data, args, total);
    let number = numbering_enabled(args);
    let mut out = String::new();
    for &index in &indices {
        let (start, end) = spans[index];
        push_text_line(&mut out, index, &data[start..end], number);
    }
    out
}

/// Append one selected line as `N:line` when numbering is on.
fn push_text_line(out: &mut String, index: usize, raw: &[u8], number: bool) {
    let text = String::from_utf8_lossy(raw);
    let line = text.strip_suffix('\n').unwrap_or(&text);
    let line = line.strip_suffix('\r').unwrap_or(line);
    if number {
        out.push_str(&format!("{}:", index + 1));
    }
    out.push_str(line);
    out.push('\n');
}

/// Refuse a `--lines` window that lies past the last line of the file.
///
/// An empty success reads as "the file is empty", so the line total is the
/// fact that lets the caller fix the request in one step.
fn ensure_ranges_select(path: &std::path::Path, total: u64, args: &ReadArgs) -> Result<()> {
    if args.lines.is_empty() {
        return Ok(());
    }
    let selected = parse_line_range(args, total).is_some_and(|range| range.start <= range.end);
    if !selected && total > 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "--lines selects nothing in {}: the file has {total} lines",
                path.display()
            ),
        }
        .into());
    }
    Ok(())
}

/// Whether a line carries its 1-based number in front.
///
/// `--grep` and `--no-comments` always number, because both cut lines out of
/// the answer: a bare `fn a() {}` cannot be turned into a follow-up `apply`,
/// and the number of the line in the file is what names the target.
/// `--number` covers every other selector.
fn numbering_enabled(args: &ReadArgs) -> bool {
    numbering_requested(args) || args.no_comments
}

/// Whether the call asked for numbers without `--no-comments`.
///
/// `--no-comments` removes lines, so it may only run when the surviving lines
/// still carry their original number. This is the condition the `read` verb
/// checks before it refuses a bare `--no-comments`.
fn numbering_requested(args: &ReadArgs) -> bool {
    args.number || crate::config::read_number_default() || args.grep.is_some()
}

/// Drop a trailing `\n` and the `\r` before it, for matching comparisons.
fn trim_line_ending(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

/// Which lines the flags select, resolved before the file is read.
enum Selection {
    All,
    Ranges(Vec<(usize, usize)>),
    Line { start: usize, end: usize },
    Head { n: usize },
    Tail { n: usize },
    Grep { pattern: regex::Regex },
}

/// Stream line selection shared by the default and the raw readers.
///
/// The decisions mirror [`select_line_indices`], which applies the same flags
/// to a whole slice, so both formats answer the same lines for the same flags.
struct LineSelector {
    selection: Selection,
    context: usize,
    no_comments: bool,
    after: usize,
    pending: std::collections::VecDeque<(usize, Vec<u8>)>,
    tail: std::collections::VecDeque<(usize, Vec<u8>)>,
    last_emitted: Option<usize>,
}

impl LineSelector {
    /// Resolve the flags with the same precedence `select_line_indices` uses:
    /// ranges, single line, head, tail, grep, then the whole file.
    fn new(args: &ReadArgs) -> Self {
        let selection = if !args.lines.is_empty() {
            let mut ranges = Vec::new();
            for raw in &args.lines {
                for part in raw.split(',') {
                    if let Some(range) = parse_open_range(part.trim()) {
                        ranges.push(range);
                    }
                }
            }
            Selection::Ranges(ranges)
        } else if let Some(line_num) = args.line {
            let idx = line_num.saturating_sub(1);
            Selection::Line {
                start: idx.saturating_sub(args.context),
                end: idx + args.context + 1,
            }
        } else if let Some(n) = args.head {
            Selection::Head { n }
        } else if let Some(n) = args.tail {
            Selection::Tail { n }
        } else if let Some(ref pattern) = args.grep
            && let Ok(pattern) = regex::Regex::new(pattern)
        {
            Selection::Grep { pattern }
        } else {
            Selection::All
        };
        Self {
            selection,
            context: args.context,
            no_comments: args.no_comments,
            after: 0,
            pending: std::collections::VecDeque::new(),
            tail: std::collections::VecDeque::new(),
            last_emitted: None,
        }
    }

    /// Feed one line, terminator included, and emit whatever is ready.
    fn push(&mut self, index: usize, line: &[u8], emit: &mut impl FnMut(usize, &[u8])) {
        if let Selection::Tail { n } = &self.selection {
            let n = *n;
            if n == 0 {
                return;
            }
            if self.tail.len() == n {
                self.tail.pop_front();
            }
            self.tail.push_back((index, line.to_vec()));
            return;
        }
        if let Selection::Grep { pattern } = &self.selection {
            let matched = pattern.is_match(&String::from_utf8_lossy(trim_line_ending(line)));
            if matched {
                let pending = std::mem::take(&mut self.pending);
                for (pending_index, pending_line) in pending {
                    emit_selected(
                        &mut self.last_emitted,
                        self.no_comments,
                        pending_index,
                        &pending_line,
                        emit,
                    );
                }
                emit_selected(&mut self.last_emitted, self.no_comments, index, line, emit);
                self.after = self.context;
            } else if self.after > 0 {
                self.after -= 1;
                emit_selected(&mut self.last_emitted, self.no_comments, index, line, emit);
            } else if self.context > 0 {
                if self.pending.len() == self.context {
                    self.pending.pop_front();
                }
                self.pending.push_back((index, line.to_vec()));
            }
            return;
        }
        let keep = match &self.selection {
            Selection::All => true,
            Selection::Ranges(ranges) => ranges
                .iter()
                .any(|(start, end)| index >= *start && index < *end),
            Selection::Line { start, end } => index >= *start && index < *end,
            Selection::Head { n } => index < *n,
            Selection::Tail { .. } | Selection::Grep { .. } => false,
        };
        if keep {
            emit_selected(&mut self.last_emitted, self.no_comments, index, line, emit);
        }
    }

    /// Emit what a tail kept back; a grep pre-context with no match is dropped.
    fn finish(&mut self, emit: &mut impl FnMut(usize, &[u8])) {
        let tail = std::mem::take(&mut self.tail);
        for (index, line) in tail {
            emit_selected(&mut self.last_emitted, self.no_comments, index, &line, emit);
        }
    }
}

/// Emit one selected line unless `--no-comments` drops it or it was already out.
fn emit_selected(
    last_emitted: &mut Option<usize>,
    no_comments: bool,
    index: usize,
    line: &[u8],
    emit: &mut impl FnMut(usize, &[u8]),
) {
    if last_emitted.is_some_and(|last| index <= last) {
        return;
    }
    if no_comments && is_comment_or_blank(line) {
        return;
    }
    *last_emitted = Some(index);
    emit(index, line);
}

/// Parse `START:END` (1-based, inclusive start) without a line total.
fn parse_open_range(s: &str) -> Option<(usize, usize)> {
    let (start, end) = s.split_once(':')?;
    let start = start.parse::<usize>().ok()?.saturating_sub(1);
    let end = end.parse::<usize>().ok()?;
    Some((start, end))
}

/// Feed every line of `path`, terminator included, to `visit`.
///
/// Returns the number of lines, which is the file's `lines_total`. The line
/// buffer is the only allocation that scales with the file, and it is fixed.
fn visit_raw_lines(path: &std::path::Path, mut visit: impl FnMut(usize, &[u8])) -> Result<u64> {
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
        visit(usize::try_from(index).unwrap_or(usize::MAX), &buf);
        index += 1;
    }
    Ok(index)
}
