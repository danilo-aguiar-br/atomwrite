// SPDX-License-Identifier: MIT OR Apache-2.0

//! NDJSON writer utilities for stdout with broken-pipe handling.

use std::io::{self, BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::error::{AtomwriteError, ErrorContext, ErrorJson};

include!("output_shape.inc.rs");
include!("output_verdict.inc.rs");

include!("output_budget.inc.rs");
include!("output_lines.inc.rs");

/// How an emitted record interacts with the process-wide `--limit` budget.
///
/// `Data` is an answer record the consumer asked for and pays for. `Framing`
/// is a `begin` or `end` anchor: it never spends and never claims a dedupe key.
/// `AlreadyCharged` is a child record re-emitted by `recipe` or `codemod`, and
/// the child writer already spent that unit.
#[derive(Clone, Copy)]
enum Charge {
    Data,
    Framing,
    AlreadyCharged,
}

/// Buffered NDJSON writer that flushes after every line.
///
/// Every emitted line passes through [`Self::write_event`], which attaches the
/// [`RESULT_KEY`] verdict to a success envelope, or through the error paths, which
/// carry their own verdict. Shaping (`--filter` / `--select` / `--limit`) is
/// applied only when configured; the identity path serializes without an
/// intermediate `serde_json::Value` (memory-friendly hot path).
/// Text bytes already sent to stdout, and whether the budget was hit.
///
/// A one-shot process writes one text payload, so a module-level counter is the
/// whole state this rule needs; it lives here because every text writer passes
/// through [`NdjsonWriter::write_text_bytes`].
static TEXT_BYTES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static TEXT_CAPPED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the shared text budget already stopped a payload.
pub fn text_capped() -> bool {
    TEXT_CAPPED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Report how many entries a capped text listing left out.
///
/// The writer sees one line at a time, so it cannot know the remainder: the
/// verb that owns the listing supplies the count once its loop ends.
pub fn note_text_omitted(verb: &str, omitted: u64) {
    crate::best_effort::best_effort(
        writeln!(
            std::io::stderr().lock(),
            "atomwrite: {verb} text output stopped at {} bytes of {}; {omitted} entries left out; use --format ndjson with --limit to page the rest",
            TEXT_BYTES.load(std::sync::atomic::Ordering::Relaxed),
            crate::constants::SEARCH_TEXT_MAX_BYTES
        ),
        "write text omission notice on stderr",
    );
}

/// Buffered NDJSON writer shared by every subcommand.
pub struct NdjsonWriter<W: Write> {
    writer: BufWriter<W>,
    shape: OutputShape,
    emitted: u64,
    /// Held-back events for the tail pass (`--sort-by` / `--dedupe` /
    /// `--aggregate` / `--diff-against`).
    ///
    /// These operations cannot answer from one event: ordering needs the whole
    /// set, dedupe needs the keys already seen, and the stream comparison needs
    /// the stream. Buffering here keeps the burden in the one place every
    /// subcommand already passes through, so a subcommand author does not have
    /// to know these flags exist.
    buffered: Vec<PositionedValue>,
    /// Arrival order of the buffered events, and the stable tie-break for sort.
    line_index: u64,
    /// Set once the tail pass has run, so a second `flush` writes nothing.
    tail_done: bool,
    /// Success records `--filter` or an empty `--select` projection removed.
    ///
    /// The writer is the only place that knows a record was removed on purpose
    /// rather than never produced, so it is the only place that can say how
    /// many the consumer did not receive.
    suppressed: u64,
    /// Absolute target the run resolved, stamped on terminal envelopes.
    ///
    /// A string for a single subject and an array for several: a joined string
    /// could not be split back, because a path may itself hold the separator,
    /// and the caller has to read the target back before trusting it.
    target_resolved: Option<serde_json::Value>,
    /// Origin of the target (`argv` or `workspace`), stamped with the target.
    target_source: Option<&'static str>,
    /// Origin of the workspace root (`argv` when `--workspace` was passed,
    /// `cwd` when it was inherited from the process working directory).
    workspace_source: Option<&'static str>,
    /// Tables read from stdin for `--join`; empty when the flag is not used.
    join_tables: Vec<JoinTable>,
    /// Verb that owns the text payload, named by its cut notice.
    text_verb: &'static str,
}

impl<W: Write> NdjsonWriter<W> {
    /// Create a new NDJSON writer wrapping the given output (identity shape).
    pub fn new(inner: W) -> Self {
        Self::with_shape(inner, OutputShape::default())
    }

    /// Create a writer with agent-native stdout shaping.
    pub fn with_shape(inner: W, shape: OutputShape) -> Self {
        Self {
            writer: BufWriter::with_capacity(crate::constants::BUF_CAPACITY, inner),
            shape,
            emitted: 0,
            buffered: Vec::new(),
            line_index: 0,
            tail_done: false,
            suppressed: 0,
            target_resolved: None,
            target_source: None,
            workspace_source: None,
            join_tables: Vec::new(),
            text_verb: "search",
        }
    }

    /// Apply the output profile resolved from configuration.
    ///
    /// The config is loaded after the writer exists (the `config` verb runs
    /// before the load, so a broken file never blocks the verb that repairs it),
    /// so the profile arrives here instead of in the constructor.
    pub fn set_profile(&mut self, profile: crate::output::OutputProfile) {
        self.shape.profile = profile;
    }

    /// Output profile this writer applies.
    ///
    /// A verb that answers with one record per file reads it here to fold those
    /// records into a single line when the caller asked for `lean`.
    #[must_use]
    pub fn profile(&self) -> crate::output::OutputProfile {
        self.shape.profile
    }

    /// Read the `--join` tables from `stdin`, once, before the command runs.
    ///
    /// Join runs BEFORE `--select`, `--filter` and `--limit`, so a projection
    /// can keep the joined field and a filter can match on it. With a tail flag
    /// (`--sort-by`, `--dedupe`, `--aggregate`) the join runs in the tail pass,
    /// after the sort and before dedupe and aggregation, as [`flush_positioned`]
    /// documents. Failure and summary records never receive a join.
    ///
    /// # Errors
    ///
    /// See [`read_join_tables`].
    pub fn load_joins(
        &mut self,
        stdin: &mut impl std::io::Read,
        stdin_is_tty: bool,
        command_consumes_stdin: bool,
    ) -> anyhow::Result<()> {
        self.join_tables = read_join_tables(
            stdin,
            stdin_is_tty,
            command_consumes_stdin,
            &self.shape.joins,
        )?;
        Ok(())
    }

    /// Record the target the run resolved, so every terminal envelope names it.
    ///
    /// `source` is the wire token of where the target came from (`argv` when
    /// the caller named it, `workspace` when the run fell back to the
    /// workspace root). Terminal success and failure envelopes carry both keys
    /// once this is set, and omit them while it is not.
    pub fn set_target(&mut self, resolved: String, source: &'static str) {
        self.set_targets(vec![resolved], source);
    }

    /// Record several resolved targets in invocation order.
    ///
    /// The field is always an array, even when the run resolved a single
    /// target: the caller reads one shape and never has to know the count
    /// before choosing how to read it.
    pub fn set_targets(&mut self, resolved: Vec<String>, source: &'static str) {
        self.target_resolved = Some(serde_json::Value::Array(
            resolved
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ));
        self.target_source = Some(source);
    }

    /// Record where the workspace root came from, so terminal envelopes name it.
    ///
    /// `source` is `argv` when the caller passed `--workspace` and `cwd` when
    /// the root was inherited from the process working directory (ETD step 2).
    pub fn set_workspace(&mut self, source: &'static str) {
        self.workspace_source = Some(source);
    }

    /// Serialize a value as a single NDJSON line and flush.
    ///
    /// When shape flags are set: filter may drop the line; select projects
    /// top-level keys; limit stops further emission after N lines.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if serialization or writing to the underlying writer fails.
    pub fn write_event<T: Serialize>(&mut self, event: &T) -> anyhow::Result<()> {
        let value = serde_json::to_value(event)?;
        self.emit_stamped(value, Charge::Data)
    }

    /// Write a framing record that does not spend the `--limit` budget.
    ///
    /// `search` brackets each file with `begin` and `end` anchors. Those records
    /// frame the answer; they are not the answer. Charging the budget for them
    /// made `search --limit 1` spend its whole allowance on one `begin` and
    /// print no `match` at all, which is the opposite of what the flag promises.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if serialization or writing to the underlying
    /// writer fails.
    pub fn write_framing_event<T: Serialize>(&mut self, event: &T) -> anyhow::Result<()> {
        let value = serde_json::to_value(event)?;
        self.emit_stamped(value, Charge::Framing)
    }

    /// Re-emit a child command's record that the child already charged.
    ///
    /// `recipe` and `codemod` capture a child run into a buffer and forward the
    /// lines. The child writer spent the `--limit` unit as it wrote, so this
    /// path spends nothing: charging again made `--limit 4` print one terminal
    /// and no records at all. Shaping still applies, and the record keeps
    /// `framing: false`, so `--dedupe` and `--sort-by` treat it as an answer.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if serialization or writing fails.
    pub fn write_reemitted_event<T: Serialize>(&mut self, event: &T) -> anyhow::Result<()> {
        let value = serde_json::to_value(event)?;
        self.emit_stamped(value, Charge::AlreadyCharged)
    }

    /// Publish the summary of a run that fails, as its single terminal event.
    ///
    /// The record leaves with `ok: false`, the machine `code`, the process
    /// `exit` status and the failure detail (`error_class`, `retryable`,
    /// `suggestion`, `suggestion_lang`, `conflict_count`, `match_count`,
    /// `pairs_*`) stamped on it, so one line carries both the counts and the
    /// verdict. The returned error is `err` tagged with [`VerdictPublished`]:
    /// `main` sees the tag and writes no second envelope for the same fact.
    ///
    /// The failure path ignores `--filter` and `--limit`, like
    /// [`Self::write_event_unshaped`], because shaping describes how to present
    /// an answer and this record says there is none. `--select` and `--redact`
    /// still apply. When the record cannot be written (a closed pipe), the
    /// write error is returned instead, and it wins over `err`.
    pub fn publish_failure<T: Serialize>(
        &mut self,
        event: &T,
        err: anyhow::Error,
    ) -> anyhow::Error {
        match self.write_failure_event(event, &err) {
            Ok(()) => err.context(VerdictPublished),
            Err(write_err) => write_err,
        }
    }

    /// Stamp the failure on `event` and write it as the terminal line.
    fn write_failure_event<T: Serialize>(
        &mut self,
        event: &T,
        err: &anyhow::Error,
    ) -> anyhow::Result<()> {
        let mut value = serde_json::to_value(event)?;
        stamp_failure(&mut value, err);
        // A tail pass (`--sort-by`, `--dedupe`, ...) writes its held-back events
        // at flush time, so the failure record joins them to keep the stream order.
        // It also joins them as a TERMINAL: `--filter` and `--aggregate` must not
        // reach the record that says the run did not answer.
        if self.shape.is_tail_flush() {
            self.stamp_target(&mut value);
            self.stamp_cut_counters(&mut value);
            crate::output::scrub_envelope_keys(&mut value);
            self.buffered.push(PositionedValue {
                terminal: true,
                framing: false,
                value,
                line_index: self.line_index,
            });
            self.line_index = self.line_index.saturating_add(1);
            return Ok(());
        }
        self.stamp_target(&mut value);
        self.stamp_cut_counters(&mut value);
        let shaped = shape_error_value(value, &self.shape);
        self.write_raw_line(&shaped)?;
        self.emitted = self.emitted.saturating_add(1);
        Ok(())
    }

    /// Stamp, scrub and shape one event value, then write it.
    ///
    /// `charged` is false for a framing record: it still respects `--select` and
    /// the key floor, but it neither spends nor checks the `--limit` budget.
    fn emit_stamped(&mut self, mut value: Value, charge: Charge) -> anyhow::Result<()> {
        let charged = matches!(charge, Charge::Data);
        // B1: the success envelope carries the same `ok` verdict the error
        // envelope already carries as `ok: false`, so one key decides between
        // success and remediation across every subcommand. The verdict is
        // attached HERE — the single point every emitted record passes through —
        // so a new subcommand inherits it without its author remembering, which
        // is the cause the defect came from.
        //
        // Stamping happens BEFORE the shape runs, because `--filter` may select
        // on the verdict, and a filter that reads a key the writer has not
        // written yet would silently match nothing.
        stamp_ok(&mut value);
        self.stamp_target(&mut value);
        // Unconditional key floor: a credential-shaped top-level key is redacted
        // on EVERY envelope, with or without `--redact` and with or without any
        // other shaping flag. Forgetting the flag must fail closed, and before
        // this ran only on the shaped path, the common invocation — no flags at
        // all — was exactly the one that leaked. A floor that depends on an
        // unrelated flag is not a floor: `--limit 5` used to redact while a bare
        // call did not.
        //
        // Only the key name decides here. Redacting on value shape as well would
        // corrupt legitimate data an agent asked to read verbatim, so
        // value-pattern redaction stays behind the explicit `--redact` opt-in.
        crate::output::scrub_envelope_keys(&mut value);

        // A tail operation cannot answer from one event: order needs the whole
        // set, dedupe needs the keys already seen, and the stream comparison
        // needs the stream. Those events are held back and the tail pass in
        // `flush` writes them, so a subcommand inherits stream shaping without
        // doing anything.
        if self.shape.is_tail_flush() {
            self.buffered.push(PositionedValue {
                value,
                line_index: self.line_index,
                terminal: false,
                framing: matches!(charge, Charge::Framing),
            });
            self.line_index = self.line_index.saturating_add(1);
            return Ok(());
        }

        for table in &self.join_tables {
            apply_join(&mut value, table);
        }
        // A charged record that arrives with no budget left is work the consumer
        // may not read, so it is not written. The check asks how much budget
        // REMAINS, not whether the limit flag rose: a verb that cuts a collected
        // list with `cap_collected` raises the flag to report the truncation,
        // and that list is exactly the one whose records must still be written.
        if charged && remaining_budget() == Some(0) {
            crate::output::note_stream_cut();
            return Ok(());
        }
        if self.shape.is_identity() {
            self.write_raw_line(&value)?;
            self.emitted = self.emitted.saturating_add(1);
            if charged {
                let _filled = spend_budget();
            }
            return Ok(());
        }
        let Some(shaped) = shape_value(value, &self.shape) else {
            self.suppressed = self.suppressed.saturating_add(1);
            return Ok(());
        };
        self.write_raw_line(&shaped)?;
        self.emitted = self.emitted.saturating_add(1);
        // Charge the record that was just written, not the one that would be
        // dropped: the walk has to stop at the N-th record, so a verb that
        // already produced the whole answer keeps working on files whose
        // records the consumer may read.
        if charged {
            let _filled = spend_budget();
        }
        Ok(())
    }

    /// Emit a terminal record that shaping must not be allowed to drop.
    ///
    /// Same key floor and the same `ok` verdict as [`Self::write_event`], and
    /// no shaping at all: no `--filter`, no `--select`, no `--limit`, no tail
    /// buffering. Reserved for the cancel envelope, which reports that the run
    /// was interrupted rather than answering the question the run was asked.
    /// Shaping describes how to present an answer, so letting it remove the
    /// record that says there is no complete answer would hand the consumer a
    /// partial result wearing the shape of a finished one.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing to the underlying writer fails.
    pub fn write_event_unshaped<T: Serialize>(&mut self, event: &T) -> anyhow::Result<()> {
        let mut value = serde_json::to_value(event)?;
        stamp_ok(&mut value);
        self.stamp_target(&mut value);
        crate::output::scrub_envelope_keys(&mut value);
        self.write_raw_line(&value)?;
        self.emitted = self.emitted.saturating_add(1);
        Ok(())
    }

    /// Name the verb that owns the text payload.
    ///
    /// The cut notice says which listing stopped, so a `list` page never
    /// borrows the `search` wording and its remediation.
    pub fn set_text_verb(&mut self, verb: &'static str) {
        self.text_verb = verb;
    }

    /// Write bytes to the same stream the NDJSON records go to.
    ///
    /// Used by the modes whose payload is not a record: `read --format raw`
    /// hands the file's bytes over, and `search --format text` writes lines of
    /// text. The buffer is drained first, so nothing a record left behind can
    /// land after the raw payload and corrupt it.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the flush or the write fails.
    pub fn write_text_bytes(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        // Text payloads follow the same rule as raw reads: stop at the budget
        // on a line boundary and say so on stderr. The Bash tool cuts at 30k
        // characters, so passing that point turns the answer into a file plus a
        // second call, which costs far more than the bytes saved.
        let cap = crate::constants::SEARCH_TEXT_MAX_BYTES as u64;
        let used = TEXT_BYTES.load(std::sync::atomic::Ordering::Relaxed);
        if TEXT_CAPPED.load(std::sync::atomic::Ordering::Relaxed)
            || used.saturating_add(bytes.len() as u64) > cap
        {
            // `list` reports the omission once, with the exact count, after its
            // loop ends; the per-line notice would only repeat the first cut.
            if !TEXT_CAPPED.swap(true, std::sync::atomic::Ordering::Relaxed)
                && self.text_verb != "list"
            {
                crate::best_effort::best_effort(
                    writeln!(
                        std::io::stderr().lock(),
                        "atomwrite: {} text output stopped at {used} bytes of {cap}; narrow the pattern, or use --format ndjson with --limit",
                        self.text_verb
                    ),
                    "write text truncation notice on stderr",
                );
            }
            return Ok(());
        }
        TEXT_BYTES.store(
            used.saturating_add(bytes.len() as u64),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.writer.flush()?;
        self.writer.get_mut().write_all(bytes)?;
        self.writer.get_mut().flush()
    }

    /// Emit the terminal record of the run.
    ///
    /// `--filter`, `--select` and `--limit` describe how to present the answer,
    /// so none of them may remove the record that says the answer is complete
    /// (or that the limit cut it short). The tail pass still owns the order:
    /// with `--sort-by` the record joins the held-back events.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing to the underlying writer fails.
    pub fn write_terminal<T: Serialize>(&mut self, event: &T) -> anyhow::Result<()> {
        let mut value = serde_json::to_value(event)?;
        stamp_ok(&mut value);
        self.stamp_target(&mut value);
        self.stamp_cut_counters(&mut value);
        crate::output::scrub_envelope_keys(&mut value);
        // `--select` and the lean profile apply to the terminal record too: it
        // is the record the caller reads to decide the next step, and ignoring
        // the projection there was the defect where `delete -r --select ok`
        // still returned the whole summary. `--filter` and `--limit` stay out,
        // so presentation can thin the record but never remove the verdict.
        if !self.shape.is_identity() {
            value = crate::output::shape_terminal_value(value, &self.shape);
        }
        if self.shape.is_tail_flush() {
            self.buffered.push(PositionedValue {
                terminal: true,
                framing: false,
                value,
                line_index: self.line_index,
            });
            self.line_index = self.line_index.saturating_add(1);
            return Ok(());
        }
        self.write_raw_line(&value)?;
        self.emitted = self.emitted.saturating_add(1);
        Ok(())
    }

    /// Emit the terminal record only when `--filter` accepts it.
    ///
    /// `--filter` narrows the data stream, and the ordinary terminal path
    /// deliberately ignores it, so the record that says the run ended survives
    /// every filter. `search` is the exception the callers of this method ask
    /// for: when a filter names a key the summary does not carry, the consumer
    /// asked for a record that does not exist, and printing the summary anyway
    /// answered a question that was never posed.
    ///
    /// A summary that names a failure is not a record about the answer, it is
    /// the run's only error channel: `search --filter ok=true` over a tree with
    /// an unreadable directory printed nothing at all and still exited 0, so the
    /// caller lost the `PERMISSION_DENIED` entry `errors[]` was carrying.
    /// `terminal_names_failure` therefore wins over the filter.
    ///
    /// The same predicate as [`shape_value`] decides the clean case, so
    /// `--select` still projects and a summary that survives keeps its verdict.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the record cannot be written.
    pub fn write_terminal_if_accepted<T: Serialize>(&mut self, event: &T) -> anyhow::Result<()> {
        if !self.shape.filters.is_empty() {
            let probe = serde_json::to_value(event)?;
            if Self::terminal_names_failure(&probe) {
                return self.write_terminal(event);
            }
            if shape_value(probe, &self.shape).is_none() {
                return Ok(());
            }
            return self.write_terminal(event);
        }
        self.write_terminal(event)
    }

    /// Whether a terminal record carries a failure the caller has to see.
    ///
    /// Three shapes say so, and each one is a field the summaries already
    /// publish: `ok: false` for a run that failed, a non-zero `files_failed` for
    /// a run that answered but lost files, and a non-empty `errors[]` for the
    /// entries that name each file.
    fn terminal_names_failure(value: &serde_json::Value) -> bool {
        let refused = value.get("ok") == Some(&serde_json::Value::Bool(false));
        let lost = value
            .get("files_failed")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|count| count > 0);
        let named = value
            .get("errors")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|errors| !errors.is_empty());
        refused || lost || named
    }

    /// Records the writer accepted on the data channel.
    ///
    /// A verb that reports how many entries it produced reads this instead of
    /// counting its own attempts: `--limit` and `--filter` can drop a record
    /// after the verb already counted it, and a summary that counts attempts
    /// claims work the consumer never saw.
    #[must_use]
    pub fn emitted_count(&self) -> u64 {
        self.emitted
    }

    /// Attach the counters that say how much of the answer the run withheld.
    ///
    /// Each key enters only when the count is above zero and only when the
    /// record does not already carry it: a verb that measured its own number
    /// keeps it, and the writer's count is the fallback for every other verb.
    /// A key equal to zero is left out, because "nothing was withheld" is what
    /// an absent key already says, and a zero on every line costs tokens.
    fn stamp_cut_counters(&self, value: &mut Value) {
        let Some(obj) = value.as_object_mut() else {
            return;
        };
        if self.suppressed > 0 {
            obj.entry("records_suppressed")
                .or_insert(Value::from(self.suppressed));
        }
        let budget = budget_dropped();
        if budget > 0 {
            obj.entry("files_budget_dropped")
                .or_insert(Value::from(budget));
        }
        let truncated = truncated_records();
        if truncated > 0 {
            obj.entry("truncated_records")
                .or_insert(Value::from(truncated));
        }
    }

    /// Attach the workspace origin and the resolved target to a terminal
    /// envelope, when each was set.
    fn stamp_target(&self, value: &mut Value) {
        if let Some(source) = self.workspace_source {
            stamp_workspace_key(value, source);
        }
        let (Some(resolved), Some(source)) = (&self.target_resolved, self.target_source) else {
            return;
        };
        stamp_target_keys(value, resolved, source, false);
    }

    fn write_raw_line<T: Serialize>(&mut self, event: &T) -> anyhow::Result<()> {
        match serde_json::to_writer(&mut self.writer, event) {
            Ok(()) => {}
            Err(e) if is_broken_pipe(&e) => {
                return Err(crate::error::AtomwriteError::BrokenPipe.into());
            }
            Err(e) => return Err(e.into()),
        }
        match self.writer.write_all(b"\n") {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                return Err(crate::error::AtomwriteError::BrokenPipe.into());
            }
            Err(e) => return Err(e.into()),
        }
        match self.writer.flush() {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                return Err(crate::error::AtomwriteError::BrokenPipe.into());
            }
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    /// Emit a structured error as a single NDJSON line.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing the error JSON to the underlying writer fails.
    pub fn write_error(&mut self, err: &AtomwriteError, path: Option<&Path>) -> anyhow::Result<()> {
        self.write_error_with_context(err, path, &ErrorContext::default())
    }

    /// Emit a structured error as a single NDJSON line, with diagnostic context.
    ///
    /// Use this overload when the caller knows whether the workspace root was
    /// explicitly provided (e.g. via CLI `--workspace`).
    /// The context controls the suggestion text for `WorkspaceJail` errors
    /// (GAP 13 fix).
    ///
    /// Agent-native (v0.1.36 residual): error envelopes **always emit**.
    /// `--filter` and `--limit` apply only to success events via
    /// [`Self::write_event`]; optional `--select` may still project keys.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing the error JSON to the underlying writer fails.
    pub fn write_error_with_context(
        &mut self,
        err: &AtomwriteError,
        path: Option<&Path>,
        ctx: &ErrorContext,
    ) -> anyhow::Result<()> {
        let mut json = ErrorJson::from_error_with_context(err, ctx);
        if json.path.is_none() {
            json.path = path.map(|p| p.display().to_string());
        }
        // Bypass write_event: never filter-drop or limit-swallow errors.
        //
        // The key floor still runs, because an error envelope carries the same
        // key surface and a credential-shaped key on it leaks just as much. It
        // is applied to both branches, so the floor stops depending on whether a
        // projection flag happened to be set.
        if self.shape.select.is_none() {
            let mut value = serde_json::to_value(&json)?;
            self.stamp_target(&mut value);
            crate::output::scrub_envelope_keys(&mut value);
            return self.write_raw_line(&value);
        }
        let mut value = serde_json::to_value(&json)?;
        self.stamp_target(&mut value);
        let shaped = shape_error_value(value, &self.shape);
        self.write_raw_line(&shaped)
    }

    /// Drop the events held back for the tail pass and cancel that pass.
    ///
    /// Called when the run is cancelled: the held-back set is partial, and the
    /// tail pass would write it AFTER the terminal `cancelled` record, which
    /// must stay the last line of the stream. Events already written are not
    /// affected, and a later `flush` only drains the byte buffer.
    pub fn discard_pending(&mut self) {
        self.buffered.clear();
        self.tail_done = true;
    }

    /// Flush the underlying buffer to the output stream.
    ///
    /// When a tail operation is active (`--sort-by` / `--dedupe` /
    /// `--aggregate` / `--diff-against`), this runs the stream-wide pass first
    /// and writes its result, so the held-back events leave the process in the
    /// order and the shape the flags asked for. The pass runs once: a second
    /// `flush` only drains the byte buffer.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if flushing the underlying writer fails.
    pub fn flush(&mut self) -> anyhow::Result<()> {
        if self.shape.is_tail_flush() && !self.tail_done {
            self.tail_done = true;
            let outcome = crate::output::flush_positioned(
                &mut self.writer,
                std::mem::take(&mut self.buffered),
                &self.shape,
                &self.join_tables,
            )?;
            self.emitted = self.emitted.saturating_add(outcome.emitted);
            if outcome.limit_hit {
                mark_limit_reached();
            }
        }
        match self.writer.flush() {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                Err(crate::error::AtomwriteError::BrokenPipe.into())
            }
            Err(e) => Err(e.into()),
        }
    }
}

include!("output_error.inc.rs");

fn is_broken_pipe(err: &serde_json::Error) -> bool {
    matches!(err.io_error_kind(), Some(io::ErrorKind::BrokenPipe))
}

/// Parse one NDJSON object line and insert a field without `serde_json::json!`.
///
/// Used by recipe/codemod when re-emitting child events with an extra tag
/// (`step`, `rule_id`). Returns `None` if the line is not a JSON object.
pub fn ndjson_insert_field(
    line: &str,
    key: impl Into<String>,
    value: serde_json::Value,
) -> Option<serde_json::Value> {
    let mut v: serde_json::Value = serde_json::from_str(line).ok()?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(key.into(), value);
        Some(v)
    } else {
        None
    }
}

include!("output_tests.inc.rs");
