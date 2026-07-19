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
    /// Recipe name (built-in) or path to YAML documentation (`--name`).
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
    /// Positional recipe name (B-004 discoverability: `recipe run NAME`).
    #[arg(value_name = "NAME")]
    pub name_positional: Option<String>,
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
    // B-004: accept `--name` or positional NAME.
    let raw_name = args
        .name
        .as_deref()
        .or(args.name_positional.as_deref())
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "recipe run requires --name NAME or positional NAME".into(),
        })?;
    // Allow documented YAML names that map to the same built-ins.
    let name = raw_name
        .trim_end_matches(".yaml")
        .trim_end_matches(".yml")
        .rsplit('/')
        .next()
        .unwrap_or(raw_name);

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

include!("recipe_run.inc.rs");
