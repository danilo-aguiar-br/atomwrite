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
    let mut ops: Vec<MultiEdit> = Vec::with_capacity(8);
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
        let jd = &mut serde_json::Deserializer::from_str(trimmed);
        let op: MultiEdit = serde_path_to_error::deserialize(jd)
            .with_context(|| format!("invalid NDJSON at line {}: {}", i + 1, trimmed))?;
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
        let effective = op.effective_op();
        match effective {
            "insert-after" | "insert-before" => {
                let n = op
                    .line
                    .ok_or_else(|| anyhow::anyhow!("op '{}' requires 'line'", effective))?;
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
                    .ok_or_else(|| anyhow::anyhow!("op '{}' requires 'start'", effective))?;
                let e = op
                    .end
                    .ok_or_else(|| anyhow::anyhow!("op '{}' requires 'end'", effective))?;
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
                    .ok_or_else(|| anyhow::anyhow!("op 'exact' requires 'old'"))?;
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
                    .ok_or_else(|| anyhow::anyhow!("insert-after requires 'line' field"))?;
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
                    .ok_or_else(|| anyhow::anyhow!("insert-before requires 'line' field"))?;
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
                    .ok_or_else(|| anyhow::anyhow!("replace-range requires 'start' field"))?
                    - 1;
                let e = op
                    .end
                    .ok_or_else(|| anyhow::anyhow!("replace-range requires 'end' field"))?;
                let src = op.content.as_deref().unwrap_or("");
                let new_lines = lines_from_str(src);
                result_lines.splice(s..e, new_lines);
            }
            "delete-range" => {
                let s = op
                    .start
                    .ok_or_else(|| anyhow::anyhow!("delete-range requires 'start' field"))?
                    - 1;
                let e = op
                    .end
                    .ok_or_else(|| anyhow::anyhow!("delete-range requires 'end' field"))?;
                result_lines.drain(s..e);
            }
            _ => unreachable!("ops filtered to known variants in validation loop"),
        }
    }

    let mut edited = join_lines(&result_lines);

    for op in &mut exact_ops {
        let old = op.old.as_deref().expect("validated in op filter loop");
        let new = op.new.as_deref().unwrap_or("");
        edited = edited.replacen(old, new, 1);
    }

    let path_str = path.display().to_string();

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "edit".into(),
            path: path_str,
            would_modify: edited != original,
            details: Some(format!("mode: multi, edits: {}", ops.len())),
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
        strict_atomic: false,
        wal_policy: args.wal_policy,
        keep_backup: resolved_backup.keep,
        durability: crate::platform::Durability::Auto,
    };

    let result = atomic_write(&path, edited.as_bytes(), &opts, workspace)?;
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

