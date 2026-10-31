/// True when this invocation emits the paired `begin` / `end` anchor events.
///
/// `--count` and `--files` replace the anchors with one `count` / `file`
/// event per file, and `--no-begin-end` removes the anchors altogether
/// (GAP-2026-010). Both `begin` and `end` consult this single predicate so
/// the two anchors can never drift apart (B5 parity: every opening an
/// invocation emits is matched by exactly one closing, and vice versa).
fn emits_anchor_events(args: &crate::cli::SearchArgs) -> bool {
    !args.count && !args.files && !args.no_begin_end
}

/// What the consumer actually wrote, as opposed to what the walker found.
///
/// The summary must report the answer the consumer can read. The producer
/// counted every file it scanned, so `search needle . --limit 1` printed one
/// `match` and a summary claiming twenty-nine — the value published was the
/// lie, not the policy. These counters rise only for records that left.
#[derive(Debug, Default)]
struct SearchTally {
    files_matched: u64,
    total_matches: u64,
    /// Match records the `--offset` page window walked past.
    offset_skipped: u64,
}

/// Emit one search event, reporting how many answer records left.
///
/// A `match`, `count` or `file` line is an answer; anchors and context lines
/// frame an answer without being one, so they return `0`. The caller sums the
/// result and stops at the `--limit` cap, which is what makes the summary and
/// the stream agree even when the writer is holding every record for a tail
/// pass.
fn emit_search_event(
    event: SearchEvent,
    writer: &mut NdjsonWriter<impl Write>,
    args: &crate::cli::SearchArgs,
    has_matches: &mut bool,
    tally: &mut SearchTally,
    text: &mut TextLayout,
) -> Result<u64> {
    // `--format text` has its own writer: the payload is lines of text, not
    // records, and the answer tallies still have to rise so `--limit`,
    // `--offset` and the exit code keep meaning what they mean.
    if args.format == crate::cli::SearchFormat::Text {
        return emit_text_event(event, writer, args, has_matches, tally, text);
    }
    match event {
        SearchEvent::Begin(path) => {
            if emits_anchor_events(args) {
                // An anchor frames a file; it is not an answer. Charging the
                // `--limit` budget for it spent the whole allowance before the
                // first `match` could be written.
                writer.write_framing_event(&SearchBegin {
                    r#type: "begin",
                    path: path.display().to_string(),
                })?;
            }
            Ok(0)
        }
        SearchEvent::Match {
            path,
            line_number,
            lines,
            byte_offset,
            submatches,
            raw: _,
        } => {
            *has_matches = true;
            if args.count || args.files || crate::output::limit_reached() {
                return Ok(0);
            }
            // `--offset N` is documented as the other half of the paging pair,
            // and it used to be accepted and ignored on the content path: the
            // run printed every match while claiming to show page two. The skip
            // happens here, on the record that would have been written, so the
            // summary never counts a match the page does not show.
            if tally.offset_skipped < args.offset {
                tally.offset_skipped = tally.offset_skipped.saturating_add(1);
                return Ok(0);
            }
            writer.write_event(&SearchMatch {
                r#type: "match",
                path: path.display().to_string(),
                line_number,
                lines,
                byte_offset,
                submatches,
            })?;
            tally.total_matches = tally.total_matches.saturating_add(1);
            Ok(1)
        }
        SearchEvent::Context {
            path,
            line_number,
            lines,
            raw_block: _,
        } => {
            // Context belongs to the match that pulled it in. While the page
            // window is still skipping, that match was paged out, so its
            // surrounding lines belong to a page the consumer did not ask for.
            let paged_out = tally.offset_skipped < args.offset;
            if !args.count && !args.files && !paged_out && !crate::output::limit_reached() {
                writer.write_event(&SearchContext {
                    r#type: "context",
                    path: path.display().to_string(),
                    line_number,
                    lines,
                })?;
            }
            Ok(0)
        }
        SearchEvent::End {
            path,
            matches,
            lines_searched,
        } => {
            *has_matches |= matches > 0;
            let path_str = path.display().to_string();
            let mut answered = 0;
            if args.files && matches > 0 && !crate::output::limit_reached() {
                writer.write_event(&SearchFile {
                    r#type: "file",
                    path: path_str.clone(),
                })?;
                tally.total_matches = tally.total_matches.saturating_add(matches);
                answered = 1;
            }
            if args.count && matches > 0 && !crate::output::limit_reached() {
                writer.write_event(&SearchCount {
                    r#type: "count",
                    path: path_str.clone(),
                    count: matches,
                })?;
                tally.total_matches = tally.total_matches.saturating_add(matches);
                answered = 1;
            }
            if emits_anchor_events(args) {
                // B5 parity: the closing anchor is emitted for every file the
                // opening anchor was emitted for, including the zero-match
                // case. `stats.matches` stays the real per-file count, so a
                // closed pair with `matches: 0` reports the truth and the
                // consumer no longer sees an opening without a closing.
                writer.write_framing_event(&SearchEnd {
                    r#type: "end",
                    path: path_str,
                    stats: FileStats {
                        matches,
                        lines_searched,
                    },
                })?;
            }
            Ok(answered)
        }
    }
}

/// Where the next `--format text` line stands relative to the previous one.
///
/// `rg` prints `--` between two groups that are not contiguous in the file,
/// and never inside a group. Tracking the last line number printed, and
/// whether context is on at all, is what reproduces that: without context a
/// match at line 9 straight after line 1 gets no separator, which is exactly
/// what `rg` does.
#[derive(Debug, Default)]
struct TextLayout {
    /// Path whose group is open, so a new file starts a new group.
    current_path: Option<std::path::PathBuf>,
    /// Last line number written for `current_path`.
    last_line: Option<u64>,
    /// A group is open, so a separator is only owed when lines are skipped.
    in_group: bool,
    /// Workspace root, resolved once, that every printed path is relative to.
    workspace: std::path::PathBuf,
    /// The targets as they were typed, so the prefix matches `rg` exactly.
    targets: Vec<std::path::PathBuf>,
    /// Whether the file name goes in front of every line.
    show_path: bool,
}

/// Roots this run searches, with the omitted case made explicit.
///
/// `search` keeps the `rg` label of a caller who typed nothing, so the omitted
/// list becomes `.` here instead of an empty walk.
pub(crate) fn search_roots(args: &SearchArgs) -> Vec<std::path::PathBuf> {
    if args.paths.is_empty() {
        vec![std::path::PathBuf::from(".")]
    } else {
        args.paths.clone()
    }
}

/// Stamp the roots this run resolved, so every terminal envelope names them.
///
/// An omitted list is ambient authority and says `workspace`; a named root is
/// the caller's and says `argv`. Several roots are joined with `,` in argv
/// order, which is the shape `replace` already publishes.
pub(crate) fn stamp_search_target<W: std::io::Write>(
    writer: &mut crate::output::NdjsonWriter<W>,
    args: &SearchArgs,
    resolved: &[std::path::PathBuf],
    workspace: &std::path::Path,
) {
    let source = if args.paths.is_empty() {
        crate::commands::path_resolution::TargetSource::Workspace
    } else {
        crate::commands::path_resolution::TargetSource::Argv
    };
    let joined = if args.paths.is_empty() {
        workspace.display().to_string()
    } else {
        resolved
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    writer.set_target(joined, source.as_str());
}

impl TextLayout {
    /// Build the layout for one run.
    ///
    /// `rg` decides the path prefix from the command line, not from what the
    /// walk found: a single file prints `LINE:TEXT`, and anything else prints
    /// the target as typed followed by the file below it (`.` yields `./a.rs`,
    /// `src` yields `src/a.rs`).
    fn new(args: &SearchArgs, workspace: &std::path::Path) -> Self {
        Self {
            workspace: workspace.to_path_buf(),
            targets: search_roots(args),
            show_path: text_needs_path(args, workspace),
            ..Self::default()
        }
    }

    /// Ordering key, reproducing how `rg` sequences two files.
    ///
    /// `rg` walks the targets in the order they were typed and, inside a
    /// directory, by path. Sorting on the printed label alone would put
    /// `sub/b.rs` before `a.rs` when both were named on the line, which is not
    /// what the command asked for; keying on the target index first keeps the
    /// caller's order and falls back to the path inside a directory, where
    /// every file shares the same index.
    fn order_key(&self, path: &std::path::Path) -> (usize, String) {
        let relative = path.strip_prefix(&self.workspace).unwrap_or(path);
        let relative = relative.to_string_lossy().replace('\\', "/");
        let index = self
            .targets
            .iter()
            .enumerate()
            .filter(|(_, target)| !target.is_file())
            .find_map(|(index, target)| {
                let base = target.to_string_lossy().replace('\\', "/");
                let base = base.trim_end_matches('/').to_string();
                (base.is_empty() || base == "." || relative.starts_with(&format!("{base}/")))
                    .then_some(index)
            })
            .unwrap_or(usize::MAX);
        (index, relative)
    }

    /// The path as `rg` prints it: target prefix plus the file below it.
    fn path_label(&self, path: &std::path::Path) -> String {
        let relative = path.strip_prefix(&self.workspace).unwrap_or(path);
        let relative = relative.to_string_lossy().replace('\\', "/");
        for target in &self.targets {
            if target.is_file() {
                continue;
            }
            let base = target.to_string_lossy().replace('\\', "/");
            let base = base.trim_end_matches('/');
            if base.is_empty() || base == "." {
                return format!("./{relative}");
            }
            if let Some(rest) = relative.strip_prefix(&format!("{base}/")) {
                return format!("{base}/{rest}");
            }
        }
        relative
    }
}

/// Emit one event as plain text, `rg -n` shaped.
///
/// Returns the same answer count the NDJSON path returns, so the two formats
/// agree on what `--limit` spent and what the summary counted.
fn emit_text_event(
    event: SearchEvent,
    writer: &mut NdjsonWriter<impl Write>,
    args: &SearchArgs,
    has_matches: &mut bool,
    tally: &mut SearchTally,
    text: &mut TextLayout,
) -> Result<u64> {
    match event {
        SearchEvent::Begin(_) => Ok(0),
        SearchEvent::Match {
            path,
            line_number,
            raw,
            ..
        } => {
            *has_matches = true;
            if args.count || args.files || crate::output::limit_reached() {
                return Ok(0);
            }
            if tally.offset_skipped < args.offset {
                tally.offset_skipped = tally.offset_skipped.saturating_add(1);
                return Ok(0);
            }
            text.write_line(writer, args, &path, line_number, &raw, b':')?;
            tally.total_matches = tally.total_matches.saturating_add(1);
            Ok(1)
        }
        SearchEvent::Context {
            path,
            line_number,
            raw_block,
            ..
        } => {
            let paged_out = tally.offset_skipped < args.offset;
            if args.count || args.files || paged_out || crate::output::limit_reached() {
                return Ok(0);
            }
            // One `SinkContext` carries a WHOLE block, and its `line_number` is
            // the first line of that block, so the remaining lines are numbered
            // from it rather than from a per-line field the searcher never sends.
            let mut number = line_number;
            for line in raw_block.split(|b| *b == b'\n') {
                if line.is_empty() && number > line_number {
                    // The trailing element of a `\n`-terminated block.
                    continue;
                }
                text.write_line(writer, args, &path, number, line, b'-')?;
                number = number.saturating_add(1);
            }
            Ok(0)
        }
        SearchEvent::End { path, matches, .. } => {
            *has_matches |= matches > 0;
            if args.files && matches > 0 && !crate::output::limit_reached() {
                let line = format!("{}\n", text.path_label(&path));
                writer.write_text_bytes(line.as_bytes())?;
                tally.total_matches = tally.total_matches.saturating_add(matches);
                return Ok(1);
            }
            if args.count && matches > 0 && !crate::output::limit_reached() {
                let line = if text.show_path {
                    format!("{}:{matches}\n", text.path_label(&path))
                } else {
                    format!("{matches}\n")
                };
                writer.write_text_bytes(line.as_bytes())?;
                tally.total_matches = tally.total_matches.saturating_add(matches);
                return Ok(1);
            }
            Ok(0)
        }
    }
}

impl TextLayout {
    /// Whether a line needs the file name in front of it.
    ///
    /// `rg` omits it when the search was given exactly one file to read. The
    /// decision is per run, so it is asked of the arguments, never of how many
    /// files the walk happened to reach.
    fn write_line(
        &mut self,
        writer: &mut NdjsonWriter<impl Write>,
        args: &SearchArgs,
        path: &std::path::Path,
        line_number: u64,
        bytes: &[u8],
        separator: u8,
    ) -> Result<()> {
        let new_path = self.current_path.as_deref() != Some(path);
        let skipped = self
            .last_line
            .is_none_or(|last| line_number > last.saturating_add(1));
        // `rg` separates groups only when context is on; without it every
        // match is its own line and a separator would be noise.
        if args.context > 0 && self.in_group && (new_path || skipped) {
            writer.write_text_bytes(b"--\n")?;
        }
        let mut line: Vec<u8> = Vec::with_capacity(bytes.len() + 32);
        if self.show_path {
            line.extend_from_slice(self.path_label(path).as_bytes());
            line.push(separator);
        }
        line.extend_from_slice(line_number.to_string().as_bytes());
        line.push(separator);
        line.extend_from_slice(bytes);
        line.push(b'\n');
        writer.write_text_bytes(&line)?;
        self.current_path = Some(path.to_path_buf());
        self.last_line = Some(line_number);
        self.in_group = true;
        Ok(())
    }
}

/// Whether every printed line carries the file name, the way `rg` decides it.
///
/// A single file named on the command line prints `LINE:TEXT`; a directory or
/// several files print the path too. Only the targets on the command line
/// decide this: `--include` narrows which files are read and never turns a
/// named file into a set, which is why `rg -F pat a.rs -g '*.rs'` still prints
/// a bare `7:text`.
fn text_needs_path(args: &SearchArgs, workspace: &std::path::Path) -> bool {
    if args.paths.len() != 1 {
        return true;
    }
    let only = args.paths[0].clone();
    let resolved = if only.is_absolute() {
        only
    } else {
        workspace.join(only)
    };
    !resolved.is_file()
}

fn build_matcher(args: &SearchArgs) -> Result<grep_regex::RegexMatcher> {
    let mut builder = RegexMatcherBuilder::new();

    if args.case_insensitive {
        builder.case_insensitive(true);
    }

    if args.smart_case {
        builder.case_smart(true);
    }

    if args.word {
        builder.word(true);
    }

    if args.multiline {
        builder.multi_line(true);
    }

    if args.fixed {
        builder.fixed_strings(true);
    }

    builder.build(&args.pattern).map_err(|e| {
        crate::error::AtomwriteError::invalid(
            &format!("invalid search pattern '{}'", args.pattern),
            e,
        )
        .into()
    })
}

fn build_walker(
    args: &SearchArgs,
    canonical_paths: &[std::path::PathBuf],
    global: &GlobalArgs,
) -> Result<ignore::WalkBuilder> {
    let mut builder =
        crate::commands::build_scoped_walker(canonical_paths, global, &args.include, &args.exclude)?;

    if let Some(SortBy::Path) = &args.sort {
        builder.sort_by_file_path(|a, b| a.cmp(b));
    }

    Ok(builder)
}

fn extract_submatches(matcher: &grep_regex::RegexMatcher, line: &str) -> Vec<Submatch> {
    let mut subs = Vec::with_capacity(crate::constants::SEARCH_SUBMATCHES_CAPACITY);
    // The line already matched; a matcher error here only costs the
    // per-match offsets, so the line is still reported without them.
    crate::best_effort::best_effort(
        matcher.find_iter(line.as_bytes(), |m| {
            let matched_text = &line[m.start()..m.end()];
            subs.push(Submatch {
                r#match: matched_text.to_owned(),
                start: m.start(),
                end: m.end(),
            });
            true
        }),
        "collect submatch offsets",
    );
    subs
}

/// One `NotFound` per root named in the invocation that does not exist.
///
/// The tally is rebuilt where it is needed instead of being threaded through
/// every branch: it holds one entry per named root, so rebuilding it costs the
/// same as carrying it.
fn root_failures(missing: &[std::path::PathBuf]) -> FailureTally {
    let mut tally = FailureTally::default();
    tally.record_missing_roots(missing);
    tally
}

/// The single exit of a `search` run: one place decides the status.
///
/// A root named in the invocation that does not exist outranks everything,
/// even matches: the caller asked about a place the run never reached (Bug 2).
/// A file that failed DURING the walk keeps the run at zero when it also
/// answered, and the summary says `partial: true` so nobody reads that answer
/// as complete. Text mode has no terminal record, so every failure goes to
/// stderr with its path and reason, and the status still follows the same
/// rules.
///
/// # Errors
///
/// Returns the first failure tagged with [`crate::output::VerdictPublished`]
/// when a root is missing or when nothing matched and something failed;
/// returns `AtomwriteError::NoMatches` when nothing failed and nothing matched.
fn finish_search_run(
    writer: &mut NdjsonWriter<impl Write>,
    base: Summary,
    failures: FailureTally,
    roots_missing: u64,
    answered: bool,
    text_mode: bool,
) -> Result<()> {
    let (summary, first_failure) = failures.into_summary(base);
    if text_mode {
        if let Some(errors) = &summary.errors {
            for failure in errors {
                crate::best_effort::best_effort(
                    writeln!(
                        std::io::stderr().lock(),
                        "atomwrite: {}: {}",
                        failure.path,
                        failure.message
                    ),
                    "report search failure on stderr",
                );
            }
        }
        if roots_missing > 0
            && let Some(err) = first_failure
        {
            return Err(err.context(crate::output::VerdictPublished));
        }
        if answered {
            return Ok(());
        }
        if let Some(err) = first_failure {
            return Err(err.context(crate::output::VerdictPublished));
        }
        return Err(crate::error::AtomwriteError::NoMatches.into());
    }
    // A named root that does not exist is a failed invocation, matches or not.
    if roots_missing > 0
        && let Some(err) = first_failure
    {
        return Err(writer.publish_failure(&summary, err));
    }
    if answered {
        // `if_accepted`, not `write_terminal`: `--filter` must not swallow the
        // record that says part of the answer is missing.
        writer.write_terminal_if_accepted(&summary)?;
        return Ok(());
    }
    if let Some(err) = first_failure {
        return Err(writer.publish_failure(&summary, err));
    }
    Err(writer.publish_failure(&summary, crate::error::AtomwriteError::NoMatches.into()))
}
