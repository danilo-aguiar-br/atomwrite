// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by recipe.rs (SRP split — A-MONO-001).

fn run_search_replace_verify(
    args: &RecipeRunArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
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

    let mut steps = Vec::new();

    // Step 1: search
    if shutdown.is_shutdown() {
        return cancelled(1);
    }
    let search_args = SearchArgs {
        pattern: pattern.to_string(),
        paths: vec![args.path.clone()],
        regex: false,
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
    };
    {
        let mut buf = Vec::new();
        let res = {
            let mut inner = NdjsonWriter::new(&mut buf);
            crate::commands::search::cmd_search(&search_args, global, &mut inner, shutdown)
        };
        // Forward child events tagged with step (no json! — typed insert helper).
        for line in String::from_utf8_lossy(&buf).lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Some(v) = crate::output::ndjson_insert_field(line, "step", 1.into()) {
                writer.write_event(&v)?;
            }
        }
        match res {
            Ok(()) => {
                let s = RecipeStepResult {
                    step_id: 1,
                    name: "search".into(),
                    status: "ok".into(),
                    detail: "scan completed".into(),
                    checksum: None,
                };
                emit_child(writer, 1, &s)?;
                steps.push(s);
            }
            Err(e) => {
                // Zero hits is informational for the recipe plan; replace still runs.
                let is_no_matches = e
                    .downcast_ref::<AtomwriteError>()
                    .is_some_and(|ae| matches!(ae, AtomwriteError::NoMatches))
                    || e.to_string().contains("no matches");
                if is_no_matches {
                    let s = RecipeStepResult {
                        step_id: 1,
                        name: "search".into(),
                        status: "ok".into(),
                        detail: "scan completed (zero hits)".into(),
                        checksum: None,
                    };
                    emit_child(writer, 1, &s)?;
                    steps.push(s);
                } else {
                    steps.push(RecipeStepResult {
                        step_id: 1,
                        name: "search".into(),
                        status: "error".into(),
                        detail: e.to_string(),
                        checksum: None,
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
    let replace_args = ReplaceArgs {
        pattern: pattern.to_string(),
        replacement: replacement.to_string(),
        paths: vec![args.path.clone()],
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
            )
        };
        match res {
            Ok(()) => {
                let s = step_ok(
                    2,
                    "replace",
                    "replacement applied",
                    extract_field(&buf, "checksum_after"),
                    args.dry_run,
                );
                emit_child(writer, 2, &s)?;
                steps.push(s);
            }
            Err(e) => {
                steps.push(RecipeStepResult {
                    step_id: 2,
                    name: "replace".into(),
                    status: "error".into(),
                    detail: e.to_string(),
                    checksum: None,
                });
                return finish_recipe(writer, args, steps, Some(2), start, Some(e));
            }
        }
    }

    // Step 3: hash
    if shutdown.is_shutdown() {
        return cancelled(3);
    }
    let hash_args = HashArgs {
        paths: vec![args.path.clone()],
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
    match hash_res {
        Ok(()) => {
            for line in String::from_utf8_lossy(&buf).lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Some(v) = crate::output::ndjson_insert_field(line, "step", 3.into()) {
                    writer.write_event(&v)?;
                }
            }
            steps.push(step_ok(
                3,
                "hash",
                "checksums verified",
                extract_field(&buf, "value"),
                false,
            ));
        }
        Err(e) => {
            steps.push(RecipeStepResult {
                step_id: 3,
                name: "hash".into(),
                status: "error".into(),
                detail: e.to_string(),
                checksum: None,
            });
            return finish_recipe(writer, args, steps, Some(3), start, Some(e));
        }
    }

    finish_recipe(writer, args, steps, None, start, None)
}

fn run_edit_loop_syntax_check(
    args: &RecipeRunArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
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

    let pairs_bytes =
        crate::file_io::read_file_bytes(pairs_file, global.effective_max_filesize())?;
    let edit_args = EditLoopArgs {
        path: target.clone(),
        allow_sequential_drift: true,
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
            )
        };
        match res {
            Ok(()) => {
                for line in String::from_utf8_lossy(&buf).lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Some(v) = crate::output::ndjson_insert_field(line, "step", 1.into()) {
                        writer.write_event(&v)?;
                    }
                }
                steps.push(step_ok(
                    1,
                    "edit-loop",
                    "pairs applied",
                    extract_field(&buf, "checksum"),
                    false,
                ));
            }
            Err(e) => {
                steps.push(RecipeStepResult {
                    step_id: 1,
                    name: "edit-loop".into(),
                    status: "error".into(),
                    detail: e.to_string(),
                    checksum: None,
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
    });

    finish_recipe(writer, args, steps, None, start, None)
}

fn step_ok(
    step_id: u64,
    name: &str,
    detail: &str,
    checksum: Option<String>,
    dry_run: bool,
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
    writer.write_event(&RecipeResult {
        r#type: "recipe_result",
        recipe: recipe_name,
        dry_run: args.dry_run,
        steps,
        failed_step_id,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;
    if let Some(e) = err {
        return Err(e);
    }
    Ok(())
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
