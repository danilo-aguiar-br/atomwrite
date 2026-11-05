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
    write_cfg: &crate::config::WriteSection,
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

    // A1: take the lock BEFORE reading, hold it to the commit, so concurrent
    // edits of one file serialise instead of overwriting each other.
    let _update_lock = crate::lock::acquire_for_update(&path)?;

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

    // Above the mmap threshold the file is validated with a fixed buffer first,
    // so a binary target is refused before the read allocates it. A 280 MB file
    // used to cost 300 MB of RSS for one `not valid UTF-8` line.
    let size_on_disk = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size_on_disk >= crate::constants::MMAP_THRESHOLD {
        crate::file_io::validate_utf8_stream(&path, global.effective_max_filesize())?;
    }

    // A single pair on a file above the fuzzy content cap is decided here, with
    // the file still on disk. The cascade refuses such content whatever the file
    // holds, and the exact path only needs a yes-or-no: one streaming pass
    // answers both, so a 256 MiB file that cost 300 MB of RSS to say "not found"
    // now costs one fixed buffer. A file that DOES contain the needle falls
    // through to the full read, because the edit has to produce the new bytes.
    let (effective_old, effective_new) =
        resolve_edit_pairs(args, workspace, global.effective_max_filesize())?;
    if effective_old.len() == 1
        && size_on_disk > fuzzy_cfg.max_content_bytes
        && let Some(old_str) = effective_old.first()
    {
        let (exact_count, _) = crate::file_io::count_exact_streaming(
            &path,
            old_str.as_bytes(),
            global.effective_max_filesize(),
        )?;
        if exact_count == 0 {
            return Err(match fuzzy_mode {
                FuzzyMode::Off => AtomwriteError::MatchFailed {
                    reason: format!(
                        "old string not found (fuzzy mode off = exact-only): {old_str:?}"
                    ),
                    best_candidate: None,
                    candidates: None,
                },
                _ => AtomwriteError::InvalidInput {
                    reason: format!(
                        "content too large for the fuzzy cascade ({} bytes > {} max); raise {} in config.toml or pass --fuzzy off for exact-only matching",
                        size_on_disk,
                        fuzzy_cfg.max_content_bytes,
                        crate::constants::FUZZY_CONTENT_CAP_REASON_MARKER
                    ),
                },
            }
            .into());
        }
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
            write_cfg,
            stdin_is_tty,
        );
    }

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
        let (e, m) = edit_by_marker(&lines, args, stdin, max_size, stdin_is_tty)?;
        (e, m, None, None)
    } else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "no edit mode specified: use --old/--new, --after-line, --before-line, --range, --delete-range, --after-match, --before-match, or --between".into(),
        }
        .into());
    };

    let path_str = path.display().to_string();

    if args.dry_run {
        // The plan carries the same risk the write would report: a dry-run is
        // what an agent inspects before deciding to write at all.
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "edit".into(),
            path: path_str,
            would_modify: edited != original,
            details: Some(format!("mode: {mode}")),
            risk_assessment: crate::commands::write_guards::assess_content_risk(
                edited.as_bytes(),
                original.len() as u64,
                edited.len() as u64,
                write_cfg,
            ),
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

    // Risk assessment, and only that. The size barriers belong to `write` and
    // `apply`, which replace the whole content by contract; an edit inserts,
    // deletes or rewrites a region, so a smaller result is the operation
    // working, not data loss. Applying the shrink barrier here blocked
    // `edit --delete-range` outright.
    let risk_assessment = crate::commands::write_guards::assess_content_risk(
        edited.as_bytes(),
        original.len() as u64,
        edited.len() as u64,
        write_cfg,
    );

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
        risk_assessment,
        write_strategy: result.write_strategy,
        backup_skipped: result.backup_skipped,
    };

    writer.write_event(&output)?;
    Ok(())
}
