// SPDX-License-Identifier: MIT OR Apache-2.0
// Included from search.rs (v0.1.37 R1/R3/R5).
//
// The per-file event stream between the parallel walker and the one consumer
// that owns stdout: the producer side (`FileSender`), the sink that feeds it,
// and the two consumers (streaming and `--sort`).

include!("search_budget.inc.rs");

use inflight::{Charge, budget};

/// Events a worker keeps in a local `Vec` before it opens a stream for the file.
///
/// Files that finish below this size cost one allocation and one hand-off, so
/// a tree of many small files does not pay for a channel per file.
const FILE_INLINE_EVENTS: usize = crate::constants::SEARCH_FILE_INLINE_EVENTS;

/// Bytes a worker may buffer inline before it opens a stream for the file.
///
/// Inline events are not gated by the byte budget, so this keeps their total
/// small even when a few events are huge (a large `--max-columns`).
const FILE_INLINE_MAX_BYTES: u64 = crate::constants::SEARCH_FILE_INLINE_MAX_BYTES;

/// Capacity of the stream opened for a file that outgrew its inline buffer.
///
/// The worker blocks on a full stream, so a file in flight holds at most this
/// many events no matter how many matches it has.
const FILE_STREAM_CAP: usize = crate::constants::SEARCH_FILE_STREAM_CAP;

/// Files announced to the consumer and not yet drained.
///
/// Bounds the number of files waiting for their turn. The memory they hold is
/// bounded in bytes by the global budget (`SEARCH_MAX_INFLIGHT_BYTES`).
const FILE_FEED_CAP: usize = crate::constants::SEARCH_FILE_FEED_CAP;

/// An event together with the budget bytes it holds while it travels.
type Charged = (SearchEvent, Charge);

/// Bytes an event holds in memory while it waits for the consumer.
fn event_cost(event: &SearchEvent) -> u64 {
    let heap = match event {
        SearchEvent::Begin(_) | SearchEvent::End { .. } => 0,
        SearchEvent::Context { lines, .. } => lines.len(),
        SearchEvent::Match {
            lines, submatches, ..
        } => {
            lines.len()
                + submatches
                    .iter()
                    .map(|s| std::mem::size_of::<Submatch>() + s.r#match.len())
                    .sum::<usize>()
        }
    };
    (std::mem::size_of::<Charged>() + heap) as u64
}

/// One file's events, handed to the consumer as a contiguous block.
enum FileFeed {
    /// The whole file fit in the inline buffer.
    Inline(Vec<Charged>),
    /// A larger file; the stream ends when the worker drops its sender.
    Stream {
        /// Events of the file, in order.
        rx: crossbeam_channel::Receiver<Charged>,
        /// Identifies the stream to the byte budget.
        id: u64,
        /// Bytes of this stream sent and not yet consumed.
        own: Arc<std::sync::atomic::AtomicU64>,
    },
}

/// Iterator over the events of a [`FileFeed`], whichever way it travels.
///
/// Yielding an event hands its budget bytes back, and a stream being iterated
/// is the head of the budget: while it has nothing in flight its producer is
/// not made to wait for room, so the consumer always has something to drain.
struct FeedEvents {
    source: FeedSource,
    head: Option<u64>,
}

enum FeedSource {
    Inline(std::vec::IntoIter<Charged>),
    Stream {
        events: crossbeam_channel::IntoIter<Charged>,
        own: Arc<std::sync::atomic::AtomicU64>,
    },
}

impl Iterator for FeedEvents {
    type Item = SearchEvent;

    fn next(&mut self) -> Option<SearchEvent> {
        let (event, charge) = match &mut self.source {
            FeedSource::Inline(it) => it.next()?,
            FeedSource::Stream { events, own } => {
                let item = events.next()?;
                let before = own.fetch_sub(item.1.bytes(), std::sync::atomic::Ordering::AcqRel);
                if before == item.1.bytes() {
                    budget().head_drained();
                }
                item
            }
        };
        drop(charge);
        Some(event)
    }
}

impl Drop for FeedEvents {
    fn drop(&mut self) {
        if let Some(id) = self.head {
            budget().clear_head(id);
        }
    }
}

impl IntoIterator for FileFeed {
    type Item = SearchEvent;
    type IntoIter = FeedEvents;

    fn into_iter(self) -> FeedEvents {
        match self {
            Self::Inline(events) => FeedEvents {
                source: FeedSource::Inline(events.into_iter()),
                head: None,
            },
            Self::Stream { rx, id, own } => {
                budget().set_head(id);
                FeedEvents {
                    source: FeedSource::Stream {
                        events: rx.into_iter(),
                        own,
                    },
                    head: Some(id),
                }
            }
        }
    }
}

enum FileOut {
    /// Events buffered locally, with the bytes they have already been charged.
    Inline { events: Vec<Charged>, bytes: u64 },
    Stream {
        tx: crossbeam_channel::Sender<Charged>,
        id: u64,
        own: Arc<std::sync::atomic::AtomicU64>,
    },
}

/// Producer side of one file: keeps its events together and in order.
///
/// Every `send` returns `false` once the consumer is gone or the run is
/// cancelled, so the caller can stop scanning instead of producing events
/// nobody reads. A file that outgrows its inline buffer becomes a stream whose
/// sends wait for room in the global byte budget.
struct FileSender<'a> {
    feeds: &'a crossbeam_channel::Sender<FileFeed>,
    out: FileOut,
}

impl<'a> FileSender<'a> {
    /// Start a file. Waits for room in the byte budget first, holding nothing,
    /// so a run of many workers cannot start more files than the budget fits.
    fn new(feeds: &'a crossbeam_channel::Sender<FileFeed>) -> Self {
        budget().wait_until_room(crate::signal::is_global_shutdown);
        Self {
            feeds,
            out: FileOut::Inline {
                events: Vec::new(),
                bytes: 0,
            },
        }
    }

    fn send(&mut self, event: SearchEvent) -> bool {
        let cost = event_cost(&event);
        match &mut self.out {
            FileOut::Stream { tx, id, own } => {
                let Some(charge) = budget().acquire(
                    cost,
                    Some((*id, own.as_ref())),
                    crate::signal::is_global_shutdown,
                ) else {
                    return false;
                };
                own.fetch_add(cost, std::sync::atomic::Ordering::AcqRel);
                tx.send((event, charge)).is_ok()
            }
            FileOut::Inline { events, bytes } => {
                events.push((event, budget().charge(cost)));
                *bytes += cost;
                if events.len() <= FILE_INLINE_EVENTS && *bytes <= FILE_INLINE_MAX_BYTES {
                    return true;
                }
                self.open_stream()
            }
        }
    }

    /// Switch a file that outgrew its local buffer to a bounded stream.
    fn open_stream(&mut self) -> bool {
        let (tx, rx) = crossbeam_channel::bounded(FILE_STREAM_CAP);
        let id = budget().next_stream_id();
        let inline_bytes = match &self.out {
            FileOut::Inline { bytes, .. } => *bytes,
            FileOut::Stream { .. } => 0,
        };
        let own = Arc::new(std::sync::atomic::AtomicU64::new(inline_bytes));
        let feed = FileFeed::Stream {
            rx,
            id,
            own: Arc::clone(&own),
        };
        if self.feeds.send(feed).is_err() {
            return false;
        }
        let out = FileOut::Stream {
            tx: tx.clone(),
            id,
            own,
        };
        let FileOut::Inline { events, .. } = std::mem::replace(&mut self.out, out) else {
            return true;
        };
        // The buffer holds at most FILE_INLINE_EVENTS + 1 events, below the stream capacity.
        events.into_iter().all(|ev| tx.send(ev).is_ok())
    }

    /// Close the file: an inline block is handed over, a stream just ends.
    fn finish(self) -> bool {
        match self.out {
            FileOut::Inline { events, .. } if events.is_empty() => true,
            FileOut::Inline { events, .. } => self.feeds.send(FileFeed::Inline(events)).is_ok(),
            FileOut::Stream { .. } => true,
        }
    }
}

/// Line text for the NDJSON `lines` field: lossy UTF-8, cut at `max_columns`.
///
/// One allocation per line. A trailing newline is kept so submatch offsets
/// match the scanned bytes; [`trim_line_end`] removes it afterwards.
fn line_text(bytes: &[u8], max_columns: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    if text.len() <= max_columns {
        return text.into_owned();
    }
    // G68: long lines (minified bundles) are cut at a char boundary.
    let mut cut = max_columns;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}...[truncated]", &text[..cut])
}

/// The line's own bytes, with only the `\n` terminator removed.
///
/// `rg` prints the line content and then a newline, so a CRLF file shows
/// `text\r` on the line: the `\r` belongs to the content, not to the
/// terminator. Invalid UTF-8 survives untouched, which a `String` cannot do.
fn trimmed_line_bytes(bytes: &[u8]) -> Vec<u8> {
    let end = bytes.iter().rposition(|b| *b != b'\n').map_or(0, |i| i + 1);
    bytes[..end].to_vec()
}

fn trim_line_end(line: &mut String) {
    let keep = line.trim_end_matches('\n').len();
    line.truncate(keep);
}

struct SearchSink<'a, 'b> {
    matcher: &'a grep_regex::RegexMatcher,
    path: Arc<std::path::Path>,
    out: &'a mut FileSender<'b>,
    cancel: crate::signal::CancelCheck,
    max_columns: usize,
    /// `--count` / `--files`: only the tallies are used, no event per match.
    tally_only: bool,
    /// Per-file ceiling requested by `--max-count`.
    ///
    /// The searcher is asked for one match above the ceiling, so the sink learns
    /// that the file had more without a second pass: the extra match is never
    /// delivered, and `suppressed` records that the file was cut.
    max_count: Option<u64>,
    /// The file produced a match past `--max-count`, so the answer is a slice.
    suppressed: bool,
    matches: u64,
    lines: u64,
    /// The search ended early (cancelled or consumer gone); the file is partial.
    stopped: bool,
}

impl SearchSink<'_, '_> {
    /// Stop the search quietly: `Ok(false)` ends it without a search error.
    fn stop(&mut self) -> Result<bool, std::io::Error> {
        self.stopped = true;
        Ok(false)
    }

    fn deliver(&mut self, event: SearchEvent) -> Result<bool, std::io::Error> {
        if self.out.send(event) {
            Ok(true)
        } else {
            self.stop()
        }
    }
}

impl Sink for SearchSink<'_, '_> {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        // One match above the ceiling is the only cheap proof that the file had
        // more. It stops the file quietly instead of ending the run, because the
        // other files still answer.
        if self.max_count.is_some_and(|max| self.matches >= max) {
            self.suppressed = true;
            return Ok(false);
        }
        self.lines += 1;
        self.matches += 1;
        if self.cancel.advance(1).is_err() {
            return self.stop();
        }
        if self.tally_only {
            return Ok(true);
        }

        let mut lines = line_text(mat.bytes(), self.max_columns);
        let submatches = extract_submatches(self.matcher, &lines);
        trim_line_end(&mut lines);
        self.deliver(SearchEvent::Match {
            path: Arc::clone(&self.path),
            line_number: mat.line_number().unwrap_or(0),
            lines,
            byte_offset: mat.absolute_byte_offset(),
            submatches,
            raw: trimmed_line_bytes(mat.bytes()),
        })
    }

    fn context(
        &mut self,
        _searcher: &grep_searcher::Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        if self.cancel.advance(1).is_err() {
            return self.stop();
        }
        if self.tally_only {
            return Ok(true);
        }

        let mut lines = line_text(ctx.bytes(), self.max_columns);
        trim_line_end(&mut lines);
        self.deliver(SearchEvent::Context {
            path: Arc::clone(&self.path),
            line_number: ctx.line_number().unwrap_or(0),
            lines,
            raw_block: ctx.bytes().to_vec(),
        })
    }
}

/// A reader that stops the scan of a file as soon as the run is cancelled.
///
/// The searcher pulls the file through `read`, one buffer at a time, so checking
/// the shutdown flag there lets a signal or `--timeout-secs` cut a scan inside a
/// file instead of at the next file boundary. The check is one `Acquire` load
/// per buffer, so it costs nothing measurable.
///
/// The flag is only consulted when the searcher asks for bytes, so the distance
/// between two checkpoints is whatever it does in between. Measured on a 4 GiB
/// file that is one unterminated line: grep grew its line buffer by copying
/// everything already read, the copies cost 43, 131, 385 and 1097 ms, and the
/// deadline fired 990 ms before the last one ended. Capping the slice here
/// changed nothing (five paired runs inside the noise) because the copy is not a
/// read. What bounds it is the searcher's heap limit, derived from
/// `search.max_filesize`, which keeps the buffer — and therefore the copy —
/// small. The two debug lines below preserve that measurement, so a future
/// overshoot names its own phase instead of being guessed at.
struct CancelRead<R> {
    inner: R,
    flag: Arc<AtomicBool>,
    last_call: std::time::Instant,
}

impl<R: std::io::Read> std::io::Read for CancelRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Measure the gap the searcher spends between two reads: the flag can
        // only be observed here, so a long gap is time the cancel cannot cut.
        let gap = self.last_call.elapsed();
        self.last_call = std::time::Instant::now();
        if gap.as_millis() >= u128::from(crate::constants::SEARCH_READ_GAP_LOG_MS) {
            tracing::debug!(
                gap_ms = gap.as_millis() as u64,
                requested_bytes = buf.len(),
                raised_ago_ms = crate::signal::shutdown_raised_ago()
                    .map_or(0, |d| d.as_millis() as u64),
                "searcher went this long without asking for bytes"
            );
        }
        if self.flag.load(Ordering::Acquire) {
            // How long the scan ran past the moment the process decided to
            // stop, and how much the searcher asked for in the call that saw
            // it. The pair separates "the loop checks too seldom" from "the
            // searcher stops asking for bytes"; A7 needs exactly this.
            tracing::debug!(
                requested_bytes = buf.len(),
                raised_ago_ms = crate::signal::shutdown_raised_ago()
                    .map_or(0, |d| d.as_millis() as u64),
                "cancel observed at read boundary"
            );
            return Err(std::io::Error::other(crate::signal::current_cancel_reason()));
        }
        self.inner.read(buf)
    }
}

/// Search the file at `path` through [`CancelRead`].
///
/// Equivalent to `Searcher::search_path` (open the file, read it through the
/// searcher), with the cancel check added. A read cut short by the cancel is
/// not an error of the file: the caller sees the run as cancelled instead.
fn search_file_cancellable(
    searcher: &mut grep_searcher::Searcher,
    matcher: &grep_regex::RegexMatcher,
    path: &std::path::Path,
    flag: &Arc<AtomicBool>,
    sink: &mut SearchSink<'_, '_>,
) -> std::io::Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = CancelRead {
        inner: file,
        flag: Arc::clone(flag),
        last_call: std::time::Instant::now(),
    };
    match searcher.search_reader(matcher, reader, sink) {
        Err(_) if flag.load(Ordering::Acquire) => Ok(()),
        other => other,
    }
}

fn event_path(event: &SearchEvent) -> &std::path::Path {
    match event {
        SearchEvent::Begin(path)
        | SearchEvent::Match { path, .. }
        | SearchEvent::Context { path, .. }
        | SearchEvent::End { path, .. } => path,
    }
}

/// Write each file's block to stdout as it arrives, one file at a time.
///
/// Memory is bounded in bytes by the global budget (`SEARCH_MAX_INFLIGHT_BYTES`),
/// not by an event count. Returns whether any match was seen. On cancel it
/// returns at once and drops the receivers: pending events are discarded and
/// the `cancelled` envelope is the last record (emitted by the caller's caller).
fn consume_streaming(
    feeds: &crossbeam_channel::Receiver<FileFeed>,
    writer: &mut NdjsonWriter<impl Write>,
    args: &SearchArgs,
    shutdown: &ShutdownSignal,
    tally: &mut SearchTally,
    text: &mut TextLayout,
) -> Result<bool> {
    let mut has_matches = false;
    let cap = crate::output::limit_cap();
    let mut accepted: u64 = 0;
    for feed in feeds {
        // Once the `--limit` budget is full, the file that filled it has been
        // written whole (its `begin` and `end` pair stays intact). Every later
        // block would contribute only framing, so the whole block is skipped
        // instead of printing a run of empty `begin` / `end` pairs.
        if crate::output::limit_reached() {
            break;
        }
        let mut answered = 0u64;
        for event in feed {
            if shutdown.is_shutdown() {
                return Ok(has_matches);
            }
            let took =
                emit_search_event(event, writer, args, &mut has_matches, tally, text)?;
            answered = answered.saturating_add(took);
            accepted = accepted.saturating_add(took);
            // The cap is counted on the RECORDS, not on the writer's spend:
            // with a tail pass holding every event, the writer only spends at
            // flush time, after the walker is done. Marking here stops the next
            // record of this file and the next file, so the summary and the
            // flush agree on what the cap allowed.
            if took > 0 && cap.is_some_and(|max| accepted >= max) {
                crate::output::mark_limit_reached();
            }
        }
        if answered > 0 {
            tally.files_matched = tally.files_matched.saturating_add(1);
        }
    }
    Ok(has_matches)
}

/// Buffer every file, then write them in path order (`--sort path`).
///
/// Sorting needs every event before the first can be written, so memory grows
/// with the total number of events, O(matches). Use it for small result sets;
/// the unsorted mode streams in bounded memory. Blocks are keyed by the path of
/// their own events, so concurrent workers can never mix two files.
fn consume_sorted(
    feeds: &crossbeam_channel::Receiver<FileFeed>,
    writer: &mut NdjsonWriter<impl Write>,
    args: &SearchArgs,
    shutdown: &ShutdownSignal,
    tally: &mut SearchTally,
    text: &mut TextLayout,
) -> Result<bool> {
    let mut buffer: std::collections::BTreeMap<std::path::PathBuf, Vec<SearchEvent>> =
        std::collections::BTreeMap::new();
    for feed in feeds {
        let mut block = Vec::new();
        for event in feed {
            if shutdown.is_shutdown() {
                return Ok(false);
            }
            block.push(event);
        }
        if let Some(first) = block.first() {
            buffer
                .entry(event_path(first).to_path_buf())
                .or_default()
                .append(&mut block);
        }
    }

    // `--sort` implies the writer's tail pass, and a buffered record is not
    // charged as it is written: the budget used to be spent at flush time, so
    // the walker never saw the limit and the summary counted every file it had
    // scanned. The consumer applies the same cap here, over the path-ordered
    // buffer, and what it accepts is exactly what the flush writes.
    let cap = crate::output::limit_cap();
    let mut accepted: u64 = 0;
    let mut has_matches = false;
    // Text mode has no `--sort` of its own and must still match `rg`, whose
    // order is "target as typed, then path inside it"; the map alone sorts by
    // absolute path, which puts `sub/b.rs` before a named `a.rs`.
    let mut blocks: Vec<(std::path::PathBuf, Vec<SearchEvent>)> = buffer.into_iter().collect();
    if args.format == crate::cli::SearchFormat::Text {
        blocks.sort_by_cached_key(|(path, _)| text.order_key(path));
    }
    for (_, block) in blocks {
        // Same rule as the streaming path: once the budget filled, a further
        // file's block could only add framing, and its records would be
        // discarded by the writer anyway.
        if crate::output::limit_reached() {
            break;
        }
        let mut answered = 0u64;
        for event in block {
            if shutdown.is_shutdown() {
                return Ok(has_matches);
            }
            let took =
                emit_search_event(event, writer, args, &mut has_matches, tally, text)?;
            answered = answered.saturating_add(took);
            accepted = accepted.saturating_add(took);
            if took > 0 && cap.is_some_and(|max| accepted >= max) {
                crate::output::mark_limit_reached();
            }
        }
        if answered > 0 {
            tally.files_matched = tally.files_matched.saturating_add(1);
        }
    }
    Ok(has_matches)
}
