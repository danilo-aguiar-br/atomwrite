/// Apply surgical edits to a file by line number, marker, or exact match.
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if the target file does not exist.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::InvalidInput` if the line number or range is invalid.
/// Returns `AtomwriteError::Io` if reading or writing the file fails.
#[tracing::instrument(skip_all, fields(command = "edit"))]
#[allow(clippy::too_many_arguments)] // CLI dispatch: args/global/io/workspace/config layers
pub fn cmd_edit(
    args: &EditArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    workspace: &Path,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    stdin_is_tty: bool,
) -> Result<()> {
    let start = Instant::now();
    let (fuzzy_mode, fuzzy_threshold) =
        crate::config::resolve_fuzzy(args.fuzzy, args.fuzzy_threshold, fuzzy_cfg)?;
    let path = crate::path_safety::validate_path(&args.path, workspace)?;
    let resolved_backup = resolve_backup(&args.backup_opts, defaults);

    if !path.exists() {
        return Err(AtomwriteError::NotFound { path }.into());
    }

    let mode_count = [
        args.after_line.is_some(),
        args.before_line.is_some(),
        args.range.is_some(),
        args.delete_range.is_some(),
        !args.old.is_empty() || !args.old_file.is_empty(),
        args.after_match.is_some(),
        args.before_match.is_some(),
        args.between.is_some(),
        args.multi,
    ]
    .iter()
    .filter(|&&b| b)
    .count();
    if mode_count > 1 {
        return Err(AtomwriteError::InvalidInput {
            reason: "conflicting edit modes: specify only one of --old/--new, --after-line, \
                     --before-line, --range, --delete-range, --after-match, --before-match, \
                     --between, --multi"
                .into(),
        }
        .into());
    }

    let original = crate::file_io::read_file_string(&path, global.effective_max_filesize())?;

    let checksum_before = checksum::hash_bytes(original.as_bytes());

    if let Some(ref expected) = args.expect_checksum {
        if &checksum_before != expected {
            if args.allow_sequential_drift {
                tracing::warn!(
                    expected = %expected,
                    actual = %checksum_before,
                    "drift aceito por --allow-sequential-drift"
                );
            } else {
                return Err(AtomwriteError::StateDrift {
                    path,
                    expected: expected.clone(),
                    actual: checksum_before,
                }
                .into());
            }
        }
    }

    let lines: Vec<&str> = original.lines().collect();
    let lines_before = lines.len() as u64;

    if args.multi {
        return cmd_edit_multi(
            args,
            original,
            path,
            checksum_before,
            lines_before,
            stdin,
            writer,
            workspace,
            start,
            defaults,
            stdin_is_tty,
        );
    }

    let (effective_old, effective_new) =
        resolve_edit_pairs(args, workspace, global.effective_max_filesize())?;

    if !effective_old.is_empty() && effective_old.len() != effective_new.len() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "--old/--old-file and --new/--new-file must be provided in equal pairs ({} old, {} new)",
                effective_old.len(),
                effective_new.len()
            ),
        }
        .into());
    }

    let (edited, mode, fuzzy_info, multi_report) = if !effective_old.is_empty() {
        let match_opts = crate::fuzzy::match_opts_from_section(
            fuzzy_mode,
            fuzzy_threshold,
            fuzzy_cfg,
            args.replace_all,
        );
        let (e, m, fi, report) = edit_old_new(
            &original,
            &effective_old,
            &effective_new,
            match_opts,
            args.partial,
        )?;
        (e, m, Some(fi), report)
    } else if args.after_line.is_some()
        || args.before_line.is_some()
        || args.range.is_some()
        || args.delete_range.is_some()
    {
        let max_size = global.effective_max_filesize();
        let (e, m) = edit_by_line(&lines, args, stdin, max_size, stdin_is_tty)?;
        (e, m, None, None)
    } else if args.after_match.is_some() || args.before_match.is_some() || args.between.is_some() {
        let max_size = global.effective_max_filesize();
        let (e, m) = edit_by_marker(&original, &lines, args, stdin, max_size, stdin_is_tty)?;
        (e, m, None, None)
    } else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "no edit mode specified: use --old/--new, --after-line, --before-line, --range, --delete-range, --after-match, --before-match, or --between".into(),
        }
        .into());
    };

    let path_str = path.display().to_string();

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "edit".into(),
            path: path_str,
            would_modify: edited != original,
            details: Some(format!("mode: {mode}")),
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let edited = {
        use crate::line_endings::{self, LineEnding};
        let target = match args.line_ending {
            LineEnding::Auto => line_endings::detect(original.as_bytes()),
            other => other,
        };
        line_endings::normalize(&edited, target)
    };

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

    let (fuzzy, strategy, strategies_tried, similarity, diff_preview, match_count, indent_adjusted) =
        match fuzzy_info {
            Some(fi) => (
                Some(fi.fuzzy),
                Some(fi.strategy),
                Some(fi.strategies_tried),
                fi.similarity,
                fi.diff_preview,
                Some(fi.match_count),
                Some(fi.indent_adjusted),
            ),
            None => (None, None, None, None, None, None, None),
        };

    let from_file = !args.old_file.is_empty();
    let (edits, pairs_total, pair_results) = match multi_report {
        Some(mut report) => {
            if from_file {
                for pr in &mut report.pair_results {
                    pr.source = Some("file".into());
                }
            }
            (
                report.applied,
                Some(report.pairs_total),
                Some(report.pair_results),
            )
        }
        None => (1, None, None),
    };

    let output = EditOutput {
        r#type: "edit",
        path: path_str,
        edits,
        mode,
        bytes_before: original.len() as u64,
        bytes_after: edited.len() as u64,
        checksum_before,
        checksum_after: result.checksum,
        lines_before,
        lines_after,
        elapsed_ms: start.elapsed().as_millis() as u64,
        fuzzy,
        strategy,
        strategies_tried,
        similarity,
        diff_preview,
        pairs_total,
        pair_results,
        mtime_preserved: Some(args.preserve_timestamps),
        match_count,
        indent_adjusted,
    };

    writer.write_event(&output)?;
    Ok(())
}

