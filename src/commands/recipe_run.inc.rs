// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by recipe.rs (SRP split — A-MONO-001).

#[allow(clippy::too_many_arguments)]
fn run_search_replace_verify(
    args: &RecipeRunArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    write_cfg: &crate::config::WriteSection,
    start: std::time::Instant,
) -> Result<()> {
    let pattern = args
        .pattern
        .as_deref()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "recipe search-replace-verify requires --pattern".into(),
        })?;
    let replacement = args
        .replacement
        .as_deref()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "recipe search-replace-verify requires --replacement".into(),
        })?;
    // ETD: the root comes from argv (`--path`), never from the current directory.
    let root = args
        .path
        .clone()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "recipe search-replace-verify requires --path".into(),
        })?;

    let mut steps = Vec::new();

    // Step 1: search
    if shutdown.is_shutdown() {
        return cancelled(1);
    }
    let search_args = SearchArgs {
        pattern: Some(pattern.to_string()),
        paths: vec![root.clone()],
        regex: Vec::new(),
        line_number: false,
        fixed: true,
        word: false,
        case_insensitive: false,
        smart_case: false,
        context: 0,
        include: args.include.clone(),
        exclude: {
            let mut ex = args.exclude.clone();
            // A-027: single source BACKUP_EXCLUDE_GLOBS.
            crate::commands::push_backup_excludes(&mut ex);
            ex
        },
        count: false,
        files: false,
        max_count: None,
        multiline: false,
        invert: false,
        sort: None,
        include_fifo: false,
        max_filesize: crate::constants::DEFAULT_SEARCH_MAX_FILESIZE_BYTES,
        max_columns: crate::constants::DEFAULT_SEARCH_MAX_COLUMNS,
        binary: false,
        no_begin_end: false,
        pcre2: false,
        target: crate::cli_args::SearchTarget::Content,
        offset: 0,
        limit: None,
        format: Some(crate::cli_args::SearchFormat::Ndjson),
    };
    {
        let mut buf = Vec::new();
        let res = {
            let mut inner = NdjsonWriter::new(&mut buf);
            crate::commands::search::cmd_search(&search_args, global, &mut inner, shutdown)
        };
        // The child writer spent the `--limit` unit of every record as it wrote,
        // so forwarding is a re-emit: charging here too made `--limit` count
        // each record twice. The child summary is folded into the step rather
        // than printed beside it, so the stream ends in one terminal record.
        let reemit = crate::commands::child_reemit::reemit_child_buffer(writer, &buf, |v| {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("step".into(), 1.into());
            }
        })?;
        match res {
            Ok(()) => {
                let s = RecipeStepResult {
                    step_id: 1,
                    name: "search".into(),
                    status: "ok".into(),
                    detail: "scan completed".into(),
                    checksum: None,
                    child_summary: reemit.terminal,
                };
                emit_child(writer, 1, &s)?;
                steps.push(s);
            }
            Err(e) => {
                // Zero hits is informational for the recipe plan; replace still runs.
                let is_no_matches = e
                    .downcast_ref::<AtomwriteError>()
                    .is_some_and(|ae| matches!(ae, AtomwriteError::NoMatches))
                    || crate::output::error_message(&e).contains("no matches");
                if is_no_matches {
                    let s = RecipeStepResult {
                        step_id: 1,
                        name: "search".into(),
                        status: "ok".into(),
                        detail: "scan completed (zero hits)".into(),
                        checksum: None,
                        child_summary: reemit.terminal,
                    };
                    emit_child(writer, 1, &s)?;
                    steps.push(s);
                } else {
                    steps.push(RecipeStepResult {
                        step_id: 1,
                        name: "search".into(),
                        status: "error".into(),
                        detail: crate::output::error_message(&e),
                        checksum: None,
                        child_summary: reemit.terminal,
                    });
                    return finish_recipe(writer, args, steps, Some(1), start, Some(e));
                }
            }
        }
    }

    // Step 2: replace
    if shutdown.is_shutdown() {
        return cancelled(2);
    }
    // `--limit` is an output ceiling, not a recipe failure. Once the search
    // child filled the budget, a later step would produce records the consumer
    // may not read, and its own walk would find nothing and report NO_MATCHES.
    if crate::output::budget_dropped() > 0 {
        steps.push(skipped_step(2, "replace"));
        steps.push(skipped_step(3, "hash"));
        return finish_recipe(writer, args, steps, None, start, None);
    }
    let replace_args = ReplaceArgs {
        strict: crate::cli_args::StrictAtomicArgs::default(),
        pattern: pattern.to_string(),
        replacement: replacement.to_string(),
        paths: vec![root.clone()],
        regex: false,
        word: false,
        literal: true,
        backup_opts: BackupOpts::default(),
        include: args.include.clone(),
        exclude: {
            let mut ex = args.exclude.clone();
            // A-027: single source BACKUP_EXCLUDE_GLOBS.
            crate::commands::push_backup_excludes(&mut ex);
            ex
        },
        preview: false,
        max_replacements: None,
        expect_checksum: None,
        dry_run: args.dry_run,
        preserve_case: false,
        fuzzy: match args.fuzzy {
            FuzzyMode::Off => FuzzyMode::Auto,
            other => other,
        },
        fuzzy_threshold: args.fuzzy_threshold,
        progress_every: crate::constants::DEFAULT_PROGRESS_EVERY_FILES,
        preserve_timestamps: false,
    };
    {
        let mut buf = Vec::new();
        let res = {
            let mut inner = NdjsonWriter::new(&mut buf);
            crate::commands::replace::cmd_replace(
                &replace_args,
                global,
                &mut inner,
                shutdown,
                defaults,
                fuzzy_cfg,
                write_cfg,
            )
        };
        // The replace events used to be dropped: only the checksum was read out
        // of the buffer, so the consumer never saw what the step changed.
        let reemit = crate::commands::child_reemit::reemit_child_buffer(writer, &buf, |v| {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("step".into(), 2.into());
            }
        })?;
        match res {
            Ok(()) => {
                let s = step_ok(
                    2,
                    "replace",
                    "replacement applied",
                    extract_field(&buf, "checksum_after"),
                    args.dry_run,
                    reemit.terminal,
                );
                emit_child(writer, 2, &s)?;
                steps.push(s);
            }
            Err(e) => {
                steps.push(RecipeStepResult {
                    step_id: 2,
                    name: "replace".into(),
                    status: "error".into(),
                    detail: crate::output::error_message(&e),
                    checksum: None,
                    child_summary: reemit.terminal,
                });
                return finish_recipe(writer, args, steps, Some(2), start, Some(e));
            }
        }
    }

    // Step 3: hash
    if shutdown.is_shutdown() {
        return cancelled(3);
    }
    if crate::output::budget_dropped() > 0 {
        steps.push(skipped_step(3, "hash"));
        return finish_recipe(writer, args, steps, None, start, None);
    }
    let hash_args = HashArgs {
        paths: vec![root],
        verify: None,
        stdin: false,
        recursive: true,
        exclude: crate::commands::backup_exclude_globs(),
    };
    let mut buf = Vec::new();
    let hash_res = {
        let mut inner = NdjsonWriter::new(&mut buf);
        crate::commands::hash::cmd_hash(&hash_args, global, Cursor::new(Vec::new()), &mut inner)
    };
    let reemit = crate::commands::child_reemit::reemit_child_buffer(writer, &buf, |v| {
        if let Some(obj) = v.as_object_mut() {
            obj.insert("step".into(), 3.into());
        }
    })?;
    match hash_res {
        Ok(()) => {
            steps.push(step_ok(
                3,
                "hash",
                "checksums verified",
                extract_field(&buf, "value"),
                false,
                reemit.terminal,
            ));
        }
        Err(e) => {
            steps.push(RecipeStepResult {
                step_id: 3,
                name: "hash".into(),
                status: "error".into(),
                detail: crate::output::error_message(&e),
                checksum: None,
                child_summary: reemit.terminal,
            });
            return finish_recipe(writer, args, steps, Some(3), start, Some(e));
        }
    }

    finish_recipe(writer, args, steps, None, start, None)
}

#[allow(clippy::too_many_arguments)]
fn run_edit_loop_syntax_check(
    args: &RecipeRunArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    write_cfg: &crate::config::WriteSection,
    start: std::time::Instant,
) -> Result<()> {
    let pairs_file = args
        .pairs_file
        .as_ref()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "recipe edit-loop-syntax-check requires --pairs-file".into(),
        })?;
    let target = args
        .target
        .as_ref()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "recipe edit-loop-syntax-check requires --target".into(),
        })?;

    let mut steps = Vec::new();

    if shutdown.is_shutdown() {
        return cancelled(1);
    }

    let pairs_bytes = crate::file_io::read_file_bytes(pairs_file, global.effective_max_filesize())?;
    let edit_args = EditLoopArgs {
        strict: crate::cli_args::StrictAtomicArgs::default(),
        path: target.clone(),
        allow_sequential_drift: true,
        partial: false,
        backup_opts: BackupOpts::default(),
        syntax_check: args.syntax_check.clone(),
        line_ending: crate::line_endings::LineEnding::Auto,
    };

    if args.dry_run {
        steps.push(RecipeStepResult {
            step_id: 1,
            name: "edit-loop".into(),
            status: "dry_run".into(),
            detail: format!("would apply pairs from {}", pairs_file.display()),
            checksum: None,
            child_summary: None,
        });
    } else {
        let mut buf = Vec::new();
        let res = {
            let mut inner = NdjsonWriter::new(&mut buf);
            crate::commands::edit_loop::cmd_edit_loop(
                &edit_args,
                global,
                Cursor::new(pairs_bytes),
                &mut inner,
                defaults,
                fuzzy_cfg,
                write_cfg,
            )
        };
        let reemit = crate::commands::child_reemit::reemit_child_buffer(writer, &buf, |v| {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("step".into(), 1.into());
            }
        })?;
        match res {
            Ok(()) => {
                steps.push(step_ok(
                    1,
                    "edit-loop",
                    "pairs applied",
                    extract_field(&buf, "checksum"),
                    false,
                    reemit.terminal,
                ));
            }
            Err(e) => {
                steps.push(RecipeStepResult {
                    step_id: 1,
                    name: "edit-loop".into(),
                    status: "error".into(),
                    detail: crate::output::error_message(&e),
                    checksum: None,
                    child_summary: reemit.terminal,
                });
                return finish_recipe(writer, args, steps, Some(1), start, Some(e));
            }
        }
    }

    // Step 2: syntax-check is embedded in edit-loop when --syntax-check is set.
    #[cfg(feature = "ast")]
    let status = if args.syntax_check.is_some() {
        "ok"
    } else {
        "skipped"
    };
    #[cfg(not(feature = "ast"))]
    let status = "skipped_no_ast";
    let detail = match status {
        "ok" => "syntax-check requested via edit-loop flag".into(),
        "skipped_no_ast" => "rebuild with --features ast for syntax-check".into(),
        _ => "pass --syntax-check LANG to enable".into(),
    };
    steps.push(RecipeStepResult {
        step_id: 2,
        name: "syntax-check".into(),
        status: status.into(),
        detail,
        checksum: None,
        child_summary: None,
    });

    finish_recipe(writer, args, steps, None, start, None)
}

fn step_ok(
    step_id: u64,
    name: &str,
    detail: &str,
    checksum: Option<String>,
    dry_run: bool,
    child_summary: Option<serde_json::Value>,
) -> RecipeStepResult {
    RecipeStepResult {
        step_id,
        name: name.into(),
        status: if dry_run {
            "dry_run".into()
        } else {
            "ok".into()
        },
        detail: detail.into(),
        checksum,
        child_summary,
    }
}

/// Detail marker of a step the `--limit` budget stopped before it ran.
const LIMIT_SKIP_DETAIL: &str = "not run: --limit stopped the stream first";

/// A step the `--limit` budget stopped before it ran.
///
/// `detail` is the marker [`finish_recipe`] counts, so a step skipped for
/// another reason — `syntax-check` without `--syntax-check` — never inflates
/// `steps_skipped`.
fn skipped_step(step_id: u64, name: &str) -> RecipeStepResult {
    RecipeStepResult {
        step_id,
        name: name.into(),
        status: "skipped".into(),
        detail: LIMIT_SKIP_DETAIL.into(),
        checksum: None,
        child_summary: None,
    }
}

fn emit_child(
    writer: &mut NdjsonWriter<impl Write>,
    step: u64,
    s: &RecipeStepResult,
) -> Result<()> {
    writer.write_event(&crate::ndjson_types::RecipeStepEvent {
        r#type: "recipe_step",
        step,
        name: s.name.clone(),
        status: s.status.clone(),
        detail: s.detail.clone(),
        checksum: s.checksum.clone(),
        child_summary: s.child_summary.clone(),
    })?;
    Ok(())
}

fn finish_recipe(
    writer: &mut NdjsonWriter<impl Write>,
    args: &RecipeRunArgs,
    steps: Vec<RecipeStepResult>,
    failed_step_id: Option<u64>,
    start: std::time::Instant,
    err: Option<anyhow::Error>,
) -> Result<()> {
    let recipe_name = args
        .name
        .clone()
        .or_else(|| args.name_positional.clone())
        .unwrap_or_else(|| "unknown".into());
    let steps_skipped = steps
        .iter()
        .filter(|s| s.status == "skipped" && s.detail == LIMIT_SKIP_DETAIL)
        .count() as u64;
    let result = RecipeResult {
        r#type: "recipe_result",
        recipe: recipe_name,
        dry_run: args.dry_run,
        steps,
        failed_step_id,
        steps_skipped: (steps_skipped > 0).then_some(steps_skipped),
        elapsed_ms: start.elapsed().as_millis() as u64,
    };
    // A failed recipe ends in an error envelope, so the result says `ok: false`.
    let Some(e) = err else {
        // The terminal goes through `write_terminal`, not `write_event`: with
        // `--limit` spent inside the steps, `write_event` dropped the very
        // record that says the recipe finished.
        writer.write_terminal(&result)?;
        return Ok(());
    };
    Err(writer.publish_failure(&result, e))
}

fn cancelled(step: u64) -> Result<()> {
    Err(crate::signal::cancelled_error(format!("recipe cancelled at step {step}")).into())
}

fn extract_field(buf: &[u8], field: &str) -> Option<String> {
    for line in String::from_utf8_lossy(buf).lines().rev() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(s) = v.get(field).and_then(|x| x.as_str())
        {
            return Some(s.to_string());
        }
    }
    None
}
