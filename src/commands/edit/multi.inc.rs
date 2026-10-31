// ─── multi mode ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiEdit {
    #[serde(default)]
    op: Option<String>,
    #[serde(default)]
    line: Option<usize>,
    #[serde(default)]
    start: Option<usize>,
    #[serde(default)]
    end: Option<usize>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    old: Option<String>,
    #[serde(default)]
    new: Option<String>,
}

impl MultiEdit {
    fn effective_op(&self) -> &str {
        if let Some(ref op) = self.op {
            return op.as_str();
        }
        if self.old.is_some() && self.new.is_some() {
            return "exact";
        }
        "unknown"
    }
}

/// Reject a field the chosen `op` does not consume.
///
/// `deny_unknown_fields` only catches names the struct does not know; `new` is a
/// real field for `exact`, so `{"op":"replace-range",...,"new":"X"}` parsed fine
/// and the range was replaced with `content` defaulting to `""` — a silent
/// delete reported as `ok: true`. Each op therefore names the content field it
/// reads and every other content field present is an error.
fn reject_unconsumed_fields(op: &MultiEdit) -> Result<()> {
    let effective = op.effective_op();
    let (allowed, takes): (&[&str], &str) = match effective {
        "insert-after" | "insert-before" | "replace-range" => (&["content"], "content"),
        "delete-range" => (&[], "no content field"),
        "exact" => (&["old", "new"], "old and new"),
        _ => return Ok(()),
    };
    for (field, is_present) in [
        ("content", op.content.is_some()),
        ("new", op.new.is_some()),
        ("old", op.old.is_some()),
    ] {
        if is_present && !allowed.contains(&field) {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "op {effective:?} does not read the {field:?} field — it takes {takes}; \
                     nothing was written"
                ),
            }
            .into());
        }
    }
    Ok(())
}

/// Footprint of a line-based op in doubled line coordinates: line `k` is
/// `2k` and the gap right after it is `2k + 1`. Range ops cover `[2s, 2e]`,
/// inserts are a single odd point. Returns `(lo, hi, is_range)`.
fn multi_footprint(op: &MultiEdit) -> Option<(usize, usize, bool)> {
    match op.effective_op() {
        "insert-after" => op.line.map(|n| (2 * n + 1, 2 * n + 1, false)),
        "insert-before" => op
            .line
            .map(|n| ((2 * n).saturating_sub(1), (2 * n).saturating_sub(1), false)),
        "replace-range" | "delete-range" => Some((2 * op.start?, 2 * op.end?, true)),
        _ => None,
    }
}

fn describe_multi_op(op: &MultiEdit) -> String {
    match (op.line, op.start, op.end) {
        (Some(n), _, _) => format!("{} line {n}", op.effective_op()),
        (None, Some(s), Some(e)) => format!("{} {s}:{e}", op.effective_op()),
        _ => op.effective_op().to_string(),
    }
}

/// Reject line ops that touch the same lines before anything is applied.
///
/// Every line number refers to the ORIGINAL file, so two ranges that overlap
/// (or an insert strictly inside a range) cannot both be honoured: applying
/// them anyway mangles the file or indexes out of bounds. Two inserts at the
/// same point are fine and keep their order.
fn reject_overlapping_ops(ops: &[MultiEdit]) -> Result<()> {
    let spans: Vec<Option<(usize, usize, bool)>> = ops.iter().map(multi_footprint).collect();
    for (i, a) in spans.iter().enumerate() {
        for (j, b) in spans.iter().enumerate().skip(i + 1) {
            let (Some((a_lo, a_hi, a_range)), Some((b_lo, b_hi, b_range))) = (a, b) else {
                continue;
            };
            if (*a_range || *b_range) && a_lo <= b_hi && b_lo <= a_hi {
                return Err(AtomwriteError::InvalidInput {
                    reason: format!(
                        "overlapping --multi operations: #{} ({}) and #{} ({}); \
                         each operation must target distinct lines of the original file",
                        i + 1,
                        describe_multi_op(&ops[i]),
                        j + 1,
                        describe_multi_op(&ops[j]),
                    ),
                }
                .into());
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_edit_multi(
    args: &EditArgs,
    original: String,
    path: std::path::PathBuf,
    checksum_before: String,
    lines_before: u64,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    workspace: &Path,
    start: Instant,
    defaults: &crate::config::DefaultsSection,
    write_cfg: &crate::config::WriteSection,
    stdin_is_tty: bool,
) -> Result<()> {
    if stdin_is_tty {
        return Err(AtomwriteError::InvalidInput {
            reason: "--multi reads content from stdin but stdin is a terminal; \
                     pipe content (cat ops.ndjson | atomwrite edit --multi ...)"
                .into(),
        }
        .into());
    }
    let resolved_backup = resolve_backup(&args.backup_opts, defaults);
    let mut reader = BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
    let mut ops: Vec<MultiEdit> = Vec::with_capacity(crate::constants::MULTI_EDIT_OPS_CAPACITY);
    let mut line_buf = String::new();
    let mut i = 0usize;

    loop {
        let n = crate::output::read_limited_line(
            &mut reader,
            &mut line_buf,
            crate::constants::MAX_NDJSON_LINE_SIZE,
        )
        .context("failed to read stdin line")?;
        if n == 0 {
            break;
        }
        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        // NDJSON is one object per line; an array here is a format mistake, and
        // serde would blame the first element ("invalid type: map, expected a
        // string") instead of the real cause.
        if trimmed.starts_with('[') {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: format!(
                    "--multi reads NDJSON: one JSON object per line, not a JSON array \
                     (line {})",
                    i + 1
                ),
            }
            .into());
        }
        let jd = &mut serde_json::Deserializer::from_str(trimmed);
        let op: MultiEdit = serde_path_to_error::deserialize(jd).map_err(|e| {
            crate::error::AtomwriteError::invalid(&format!("invalid NDJSON at line {}", i + 1), e)
        })?;
        ops.push(op);
        i += 1;
    }

    if ops.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "--multi requires at least one edit operation on stdin".into(),
        }
        .into());
    }

    let content_lines = lines_from_str(&original);
    let total = content_lines.len();

    // Validate all operations before applying
    for op in &ops {
        reject_unconsumed_fields(op)?;
        let effective = op.effective_op();
        match effective {
            "insert-after" | "insert-before" => {
                let n = op
                    .line
                    .ok_or_else(|| crate::invalid_input!("op '{}' requires 'line'", effective))?;
                if n == 0 || n > total {
                    return Err(crate::error::AtomwriteError::InvalidInput {
                        reason: format!(
                            "op '{}': line {} out of range (file has {} lines)",
                            effective, n, total
                        ),
                    }
                    .into());
                }
            }
            "replace-range" | "delete-range" => {
                let s = op
                    .start
                    .ok_or_else(|| crate::invalid_input!("op '{}' requires 'start'", effective))?;
                let e = op
                    .end
                    .ok_or_else(|| crate::invalid_input!("op '{}' requires 'end'", effective))?;
                if s == 0 || e == 0 || s > total || e > total || s > e {
                    return Err(crate::error::AtomwriteError::InvalidInput {
                        reason: format!(
                            "op '{}': range {}:{} invalid (file has {} lines)",
                            effective, s, e, total
                        ),
                    }
                    .into());
                }
            }
            "exact" => {
                let old = op
                    .old
                    .as_deref()
                    .ok_or_else(|| crate::invalid_input!("op 'exact' requires 'old'"))?;
                if find_str(&original, old).is_none() {
                    return Err(crate::error::AtomwriteError::InvalidInput {
                        reason: format!("op 'exact': old string not found: {:?}", old),
                    }
                    .into());
                }
            }
            other => {
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: format!("unknown op: {:?}", other),
                }
                .into());
            }
        }
    }

    reject_overlapping_ops(&ops)?;

    // Sort line-based ops by descending line to avoid index drift
    // Separate exact ops (they work on the accumulated string, not line indices)
    let mut exact_ops: Vec<&MultiEdit> =
        ops.iter().filter(|o| o.effective_op() == "exact").collect();
    let mut line_ops: Vec<&MultiEdit> =
        ops.iter().filter(|o| o.effective_op() != "exact").collect();
    line_ops.sort_by(|a, b| {
        let la = a.line.or(a.start).unwrap_or(0);
        let lb = b.line.or(b.start).unwrap_or(0);
        lb.cmp(&la)
    });

    let mut result_lines: Vec<String> = content_lines;

    for op in &line_ops {
        match op.effective_op() {
            "insert-after" => {
                let n = op
                    .line
                    .ok_or_else(|| crate::invalid_input!("insert-after requires 'line' field"))?;
                let idx = n - 1;
                let src = op.content.as_deref().unwrap_or("");
                let new_lines = lines_from_str(src);
                for (i, line) in new_lines.into_iter().enumerate() {
                    result_lines.insert(idx + 1 + i, line);
                }
            }
            "insert-before" => {
                let n = op
                    .line
                    .ok_or_else(|| crate::invalid_input!("insert-before requires 'line' field"))?;
                let idx = n - 1;
                let src = op.content.as_deref().unwrap_or("");
                let new_lines = lines_from_str(src);
                for (i, line) in new_lines.into_iter().enumerate() {
                    result_lines.insert(idx + i, line);
                }
            }
            "replace-range" => {
                let s = op
                    .start
                    .ok_or_else(|| crate::invalid_input!("replace-range requires 'start' field"))?
                    - 1;
                let e = op
                    .end
                    .ok_or_else(|| crate::invalid_input!("replace-range requires 'end' field"))?;
                let src = op.content.as_deref().unwrap_or("");
                let new_lines = lines_from_str(src);
                result_lines.splice(s..e, new_lines);
            }
            "delete-range" => {
                let s = op
                    .start
                    .ok_or_else(|| crate::invalid_input!("delete-range requires 'start' field"))?
                    - 1;
                let e = op
                    .end
                    .ok_or_else(|| crate::invalid_input!("delete-range requires 'end' field"))?;
                result_lines.drain(s..e);
            }
            other => {
                return Err(AtomwriteError::InternalError {
                    reason: format!("--multi op {other:?} passed validation but has no handler"),
                }
                .into());
            }
        }
    }

    let mut edited = join_lines(&result_lines);

    for op in &mut exact_ops {
        let old = op
            .old
            .as_deref()
            .ok_or_else(|| AtomwriteError::InternalError {
                reason: "--multi exact op lost its `old` after validation".into(),
            })?;
        let new = op.new.as_deref().unwrap_or("");
        edited = edited.replacen(old, new, 1);
    }

    let path_str = path.display().to_string();

    // Risk only: the size barriers govern whole-content replacement.
    // Assessed before the dry-run branch so the plan carries it too.
    let risk_assessment = crate::commands::write_guards::assess_content_risk(
        edited.as_bytes(),
        original.len() as u64,
        edited.len() as u64,
        write_cfg,
    );

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "edit".into(),
            path: path_str,
            would_modify: edited != original,
            details: Some(format!("mode: multi, edits: {}", ops.len())),
            risk_assessment,
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let opts = AtomicWriteOptions {
        backup: resolved_backup.backup,
        syntax_check: false,
        retention: resolved_backup.retention,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: args.strict.strict_atomic,
        wal_policy: args.wal_policy,
        keep_backup: resolved_backup.keep,
        durability: crate::platform::Durability::Auto,
    };

    let result = crate::atomic::atomic_write_based(
        &path,
        edited.as_bytes(),
        &opts,
        workspace,
        Some(&checksum_before),
    )?;
    let lines_after = edited.lines().count() as u64;
    let output = EditOutput {
        r#type: "edit",
        path: path_str,
        edits: ops.len() as u64,
        mode: "multi".into(),
        bytes_before: original.len() as u64,
        bytes_after: edited.len() as u64,
        checksum_before,
        checksum_after: result.checksum,
        lines_before,
        lines_after,
        elapsed_ms: start.elapsed().as_millis() as u64,
        fuzzy: None,
        strategy: None,
        strategies_tried: None,
        similarity: None,
        diff_preview: None,
        pairs_total: None,
        pair_results: None,
        mtime_preserved: Some(args.preserve_timestamps),
        match_count: None,
        indent_adjusted: None,
        risk_assessment,
    };

    writer.write_event(&output)?;
    Ok(())
}

// ─── old/new with fuzzy cascade ──────────────────────────────────────────────

/// Per-pair diagnostics produced by multi-pair `--old`/`--new` editing (G117).
struct MultiReport {
    pair_results: Vec<PairResult>,
    pairs_total: u64,
    applied: u64,
}
