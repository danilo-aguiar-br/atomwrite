// SPDX-License-Identifier: MIT OR Apache-2.0
// Shaped line emission and positioned flush.
// Included from output_shape.inc.rs.

/// Write one NDJSON line to `out` after applying filters, projection and
/// every value-level operation of `shape`.
///
/// Returns `Ok(false)` when the event was filtered out, so the caller keeps an
/// exact emitted count without a second pass over the data.
///
/// # Errors
///
/// Returns an error when serialization or the write fails.
pub fn emit_shaped_line(
    out: &mut impl Write,
    value: Value,
    shape: &OutputShape,
) -> std::io::Result<bool> {
    let Some(mut shaped) = shape_value(value, shape) else {
        return Ok(false);
    };
    // The buffered path is a second door onto the data channel, so the key
    // floor runs here too: a guarantee that holds on one path only is not a
    // guarantee.
    scrub_envelope_keys(&mut shaped);
    serde_json::to_writer(&mut *out, &shaped)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(true)
}

/// Name the count the tail pass removed, on the record that closes the run.
///
/// The terminal is written after the pipeline, so it is the one record that can
/// report how many events `--dedupe`, `--aggregate` or the `--limit` budget
/// removed. A zero is left out: the absent key already says nothing was cut.
fn stamp_suppressed(value: &mut Value, dropped: u64) {
    if dropped == 0 {
        return;
    }
    if let Some(obj) = value.as_object_mut() {
        obj.entry("records_suppressed")
            .or_insert(Value::from(dropped));
    }
}

/// Write one terminal record from the tail pass.
///
/// `--select` may thin the envelope, but never below the two keys the consumer
/// reads the verdict off: a terminal projected down to `{}` says nothing. Every
/// other shaping flag is left out on purpose.
fn emit_terminal_line(
    out: &mut impl Write,
    value: Value,
    shape: &OutputShape,
    outcome: &mut FlushOutcome,
) -> std::io::Result<()> {
    let original = value.clone();
    // `--select` may thin the envelope, but never below the two keys the
    // consumer reads the verdict off: a terminal projected down to `{}` says
    // nothing, and one projected to `{"path": ...}` does not say it failed.
    let mut value = keep_terminal_keys(project_select(value, shape), &original);
    scrub_envelope_keys(&mut value);
    serde_json::to_writer(&mut *out, &value)?;
    out.write_all(b"\n")?;
    out.flush()?;
    outcome.emitted += 1;
    Ok(())
}

/// Group a positioned stream into per-file blocks, preserving arrival order.
///
/// A `search` block is `begin`, its answers, then `end`, so the block a `path`
/// names is contiguous already. Grouping keeps that block together so `--limit`
/// can decide over answers while the anchors travel or fall with them, and an
/// event with no `path` (a summary that is not terminal, a generic record) forms
/// its own block and is treated as an answer.
fn group_blocks(events: Vec<PositionedValue>) -> Vec<Vec<PositionedValue>> {
    let mut blocks: Vec<Vec<PositionedValue>> = Vec::new();
    let mut open: Option<(String, usize)> = None;
    for event in events {
        let key = event
            .value
            .as_object()
            .and_then(|o| o.get("path"))
            .and_then(|p| p.as_str())
            .map(str::to_string);
        match (&key, &open) {
            (Some(path), Some((current, index))) if path == current => {
                blocks[*index].push(event);
            }
            _ => {
                if let Some(path) = key {
                    open = Some((path, blocks.len()));
                } else {
                    open = None;
                }
                blocks.push(vec![event]);
            }
        }
    }
    blocks
}

/// What one buffered flush did, for the caller to report on the data channel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlushOutcome {
    /// Events written to the data channel.
    pub emitted: u64,
    /// Events dropped by `--filter`, `--dedupe` or the `--limit` budget.
    pub dropped: u64,
    /// Events collapsed into an aggregate summary.
    pub aggregated: u64,
    /// Stream comparison, when `--diff-against` was active.
    pub diff: Option<StreamDiff>,
    /// True when the `--limit` budget dropped at least one event.
    ///
    /// Reported, not acted on: the pass knows what it dropped, but the limit
    /// belongs to the invocation, and [`crate::output::NdjsonWriter`] is the
    /// object that owns an invocation. Marking the process-wide flag here made
    /// a free function mutate state it does not own.
    pub limit_hit: bool,
}

/// Flush a buffered event stream through `shape` and report what happened.
///
/// `join_tables` holds the tables already read from stdin via
/// [`parse_join_stdin`]; `shape.joins` only declares which tables the caller
/// must supply, because reading stdin is not this module's job.
///
/// The pipeline order is fixed and matters:
/// 1. `--sort` orders the events; every criterion ties back to arrival order.
/// 2. `--join` attaches the external table.
/// 3. `--dedupe` keeps the first event of each key tuple.
/// 4. `--aggregate` collapses each group into one summary.
/// 5. `--diff-against` replaces the stream with a single `stream_diff` line.
/// 6. The remaining events are written under the `--limit` budget.
///
/// Sorting before dedupe makes "keep the first" mean the ordered first, which
/// is the only reading an agent can predict. Aggregation runs after dedupe so
/// a count reflects unique events.
///
/// # Errors
///
/// Returns an error when the baseline cannot be read, an event cannot be
/// serialized, or a write fails.
pub fn flush_positioned(
    out: &mut impl Write,
    events: Vec<PositionedValue>,
    shape: &OutputShape,
    join_tables: &[JoinTable],
) -> anyhow::Result<FlushOutcome> {
    use std::collections::{HashMap, HashSet};

    let mut outcome = FlushOutcome::default();

    // The terminal record is split off before any operation runs, and rejoined
    // at the end. `--filter`, `--dedupe`, `--aggregate`, `--diff-against` and
    // `--limit` describe how to present an answer, so none of them may remove
    // the record that says the answer is complete (or that it was cut short).
    // Keeping it out of the pipeline is the only way that holds for every verb
    // without each verb remembering.
    let (terminals, mut events): (Vec<PositionedValue>, Vec<PositionedValue>) =
        events.into_iter().partition(|event| event.terminal);

    if shape.sort.is_some() {
        events.sort_by(|a, b| compare_positioned(a, b, shape));
    }

    for event in events.iter_mut() {
        for table in join_tables {
            // A miss is legitimate: the event keeps its own shape.
            apply_join(&mut event.value, table);
        }
    }

    if !shape.dedupe_keys.is_empty() {
        let mut seen: HashSet<String> = HashSet::new();
        let before = events.len();
        // A framing record is not an answer, and `search` gives its `begin`
        // anchor the same `path` the answers carry. Letting the anchor into the
        // identity set made `--dedupe path` keep the anchor and drop every
        // match of the file, which is the answer disappearing without a word.
        // The anchor travels with its block and never claims a key.
        events.retain(|event| {
            event.framing || seen.insert(dedupe_identity(&event.value, &shape.dedupe_keys))
        });
        outcome.dropped += (before - events.len()) as u64;
    }

    if let Some(first) = shape.aggregates.first() {
        let group_key = first.group.clone();
        let mut index_of: HashMap<String, usize> = HashMap::new();
        let mut grouped: Vec<(Value, Vec<PositionedValue>)> = Vec::new();
        for event in events.drain(..) {
            let Some(key) = event
                .value
                .as_object()
                .and_then(|o| o.get(&group_key))
                .cloned()
            else {
                // No group value: the event cannot be reduced, so drop it and
                // count it rather than inventing an empty group.
                outcome.dropped += 1;
                continue;
            };
            let identity = key.to_string();
            match index_of.get(&identity) {
                Some(index) => grouped[*index].1.push(event),
                None => {
                    index_of.insert(identity, grouped.len());
                    grouped.push((key, vec![event]));
                }
            }
        }
        outcome.aggregated = grouped
            .iter()
            .map(|(_, members)| members.len() as u64)
            .sum();
        events = grouped
            .into_iter()
            .enumerate()
            .map(|(index, (key, members))| PositionedValue {
                value: aggregate_group(&group_key, key, &members, &shape.aggregates),
                line_index: index as u64,
                terminal: false,
                framing: false,
            })
            .collect();
    }

    if let Some(path) = &shape.diff_against {
        let baseline = load_baseline(path)?;
        let diff = diff_streams(&baseline, &events);
        let envelope = serde_json::to_value(StreamDiffOutput::from_diff(&diff))?;
        if emit_shaped_line(out, envelope, shape)? {
            outcome.emitted += 1;
        }
        outcome.diff = Some(diff);
        // The comparison describes the data stream; the terminal still closes
        // the run, so the consumer keeps a verdict after the diff line.
        for mut terminal in terminals {
            stamp_suppressed(&mut terminal.value, outcome.dropped);
            emit_terminal_line(out, terminal.value, shape, &mut outcome)?;
        }
        return Ok(outcome);
    }

    // `--limit` counts ANSWERS. A framing record (`search` `begin` / `end`)
    // brackets a file and is not one, so it neither spends the budget nor gets
    // cut by it. Blocks are keyed by `path` so a file's anchors travel with the
    // answers they frame: a block that contributes no answer is dropped whole
    // instead of leaving an opening anchor with nothing under it.
    let mut counted: u64 = 0;
    for block in group_blocks(events) {
        let room = shape
            .limit
            .map_or(u64::MAX, |max| max.saturating_sub(counted));
        if shape.limit.is_some() && room == 0 {
            // A block with no room left is dropped whole, anchors included: an
            // opening anchor with nothing under it frames nothing.
            if block.iter().any(|event| !event.framing) {
                outcome.limit_hit = true;
            }
            outcome.dropped += block.len() as u64;
            continue;
        }
        for event in block {
            if emit_shaped_line(out, event.value, shape)? {
                outcome.emitted += 1;
                if !event.framing {
                    counted = counted.saturating_add(1);
                }
            } else {
                outcome.dropped += 1;
            }
        }
    }

    for mut terminal in terminals {
        stamp_suppressed(&mut terminal.value, outcome.dropped);
        emit_terminal_line(out, terminal.value, shape, &mut outcome)?;
    }

    Ok(outcome)
}
