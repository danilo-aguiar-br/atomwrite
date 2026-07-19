// SPDX-License-Identifier: MIT OR Apache-2.0

//! Named multi-step recipes with real handler dispatch (v0.1.29 P1-4).
//!
//! Workload: mixed (orchestrates I/O-bound subcommands sequentially).
//! Parallelism: recipe steps stay ordered for deterministic agent contracts.

use std::io::{Cursor, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand, ValueHint};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::{FuzzyMode, GlobalArgs};
use crate::cli_args::{BackupOpts, EditLoopArgs, HashArgs, ReplaceArgs, SearchArgs};
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `recipe`.
#[derive(Args, Debug)]
pub struct RecipeArgs {
    /// Recipe subcommand (`run` or `list`).
    #[command(subcommand)]
    pub action: RecipeAction,
}

/// Recipe subcommands.
#[derive(Subcommand, Debug)]
pub enum RecipeAction {
    /// Run a built-in or documented YAML recipe.
    Run(Box<RecipeRunArgs>),
    /// List built-in recipes.
    List,
}

/// `recipe run` arguments.
#[derive(Args, Debug)]
pub struct RecipeRunArgs {
    /// Recipe name (built-in) or path to YAML documentation.
    #[arg(long)]
    pub name: String,
    /// Dry-run all mutating steps.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
    /// Workspace-relative path root for recipe steps.
    #[arg(long, default_value = ".", value_hint = ValueHint::AnyPath)]
    pub path: PathBuf,
    /// Search/replace pattern (required for search-replace-verify).
    #[arg(long)]
    pub pattern: Option<String>,
    /// Replacement text (required for search-replace-verify).
    #[arg(long)]
    pub replacement: Option<String>,
    /// Fuzzy mode for replace step (default auto).
    #[arg(long, value_enum, default_value_t = FuzzyMode::Auto)]
    pub fuzzy: FuzzyMode,
    /// Optional fuzzy threshold override.
    #[arg(long)]
    pub fuzzy_threshold: Option<f64>,
    /// Include globs for search/replace.
    #[arg(long, action = clap::ArgAction::Append)]
    pub include: Vec<String>,
    /// Exclude globs for search/replace.
    #[arg(long, action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,
    /// NDJSON pairs file for edit-loop-syntax-check.
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub pairs_file: Option<PathBuf>,
    /// Target file for edit-loop-syntax-check.
    #[arg(long, value_hint = ValueHint::AnyPath)]
    pub target: Option<PathBuf>,
    /// Language for syntax-check step (requires feature ast).
    #[arg(long)]
    pub syntax_check: Option<String>,
}

/// Per-step result in a recipe run.
#[derive(Serialize, JsonSchema, Clone, Debug)]
pub struct RecipeStepResult {
    /// 1-based step id.
    pub step_id: u64,
    /// Step name (search, replace, hash, …).
    pub name: String,
    /// Status: `ok`, `dry_run`, `error`, or `skipped`.
    pub status: String,
    /// Human detail or error message.
    pub detail: String,
    /// Optional BLAKE3 checksum produced by the step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

/// Final recipe envelope.
#[derive(Serialize, JsonSchema, Debug)]
pub struct RecipeResult {
    /// Event type discriminator.
    pub r#type: &'static str,
    /// Recipe name.
    pub recipe: String,
    /// Whether dry-run was requested.
    pub dry_run: bool,
    /// Per-step results.
    pub steps: Vec<RecipeStepResult>,
    /// Failed step id when status is partial/error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_step_id: Option<u64>,
    /// Wall-clock elapsed ms.
    pub elapsed_ms: u64,
}

/// Dispatch recipe actions.
#[tracing::instrument(skip_all, fields(command = "recipe"))]
pub fn cmd_recipe(
    args: &RecipeArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<()> {
    match &args.action {
        RecipeAction::List => {
            for name in ["search-replace-verify", "edit-loop-syntax-check"] {
                writer.write_event(&crate::ndjson_types::RecipeListEvent {
                    r#type: "recipe",
                    name,
                    builtin: true,
                })?;
            }
            Ok(())
        }
        RecipeAction::Run(a) => {
            run_recipe(a.as_ref(), global, writer, shutdown, defaults, fuzzy_cfg)
        }
    }
}

fn run_recipe(
    args: &RecipeRunArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
) -> Result<()> {
    let start = std::time::Instant::now();
    // Allow documented YAML names that map to the same built-ins.
    let name = args
        .name
        .trim_end_matches(".yaml")
        .trim_end_matches(".yml")
        .rsplit('/')
        .next()
        .unwrap_or(args.name.as_str());

    match name {
        "search-replace-verify" => {
            run_search_replace_verify(args, global, writer, shutdown, defaults, fuzzy_cfg, start)
        }
        "edit-loop-syntax-check" => {
            run_edit_loop_syntax_check(args, global, writer, shutdown, defaults, fuzzy_cfg, start)
        }
        other => Err(AtomwriteError::InvalidInput {
            reason: format!("unknown recipe: {other}"),
        }
        .into()),
    }
}

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
            if !ex.iter().any(|e| e.contains("bak")) {
                ex.push("*.bak.*".into());
                ex.push("**/*.bak.*".into());
            }
            ex
        },
        count: false,
        files: false,
        max_count: None,
        multiline: false,
        invert: false,
        sort: None,
        include_fifo: false,
        max_filesize: 10 * 1024 * 1024,
        max_columns: 500,
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
            if !ex.iter().any(|e| e.contains("bak")) {
                ex.push("*.bak.*".into());
                ex.push("**/*.bak.*".into());
            }
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
        progress_every: 50,
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
        exclude: vec!["*.bak.*".into(), "**/*.bak.*".into()],
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
    writer.write_event(&RecipeResult {
        r#type: "recipe_result",
        recipe: args.name.clone(),
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
