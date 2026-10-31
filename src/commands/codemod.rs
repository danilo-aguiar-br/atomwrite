// SPDX-License-Identifier: MIT OR Apache-2.0

//! Multi-rule AST codemod campaign runner (v0.1.29 P3-3).
//!
//! Workload: mixed I/O + AST CPU (delegates to `transform`).
//! Parallelism: inherits `transform`'s `WalkParallel` + bounded channel;
//! bound via `--threads` / `--max-concurrency`. No separate fan-out here.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, ValueHint};

use crate::cli::GlobalArgs;
use crate::cli_args::{BackupOpts, TransformArgs};
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `codemod`.
#[derive(Args, Debug)]
pub struct CodemodArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    /// Path to a YAML rules file: a sequence of rules, each with the four keys
    /// `id` (optional label), `language`, `pattern` and `rewrite`.
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub rules: PathBuf,
    /// Roots to rewrite (at least one; omitting them is a usage error).
    #[arg(required = true, value_hint = ValueHint::AnyPath)]
    pub paths: Vec<PathBuf>,
    /// Dry-run only.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
    /// Shared backup flags for apply mode.
    #[command(flatten)]
    pub backup_opts: BackupOpts,
}

/// Run a multi-rule codemod campaign by delegating to `transform --rules`.
///
/// # Errors
///
/// Returns `AtomwriteError::Cancelled` when shutdown was already requested before the run starts.
/// Returns `AtomwriteError::NotFound` when the `--rules` file does not exist.
/// Returns every error the delegated `transform` run raises, including
/// `AtomwriteError::InvalidInput` for malformed rules YAML, `AtomwriteError::WorkspaceJail` for a
/// path outside the workspace and `AtomwriteError::NoMatches` when no rule matched.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
#[tracing::instrument(skip_all, fields(command = "codemod"))]
pub fn cmd_codemod(
    args: &CodemodArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    if shutdown.is_shutdown() {
        return Err(crate::signal::cancelled_error("codemod cancelled").into());
    }
    if !args.rules.exists() {
        return Err(AtomwriteError::NotFound {
            path: args.rules.clone(),
        }
        .into());
    }

    // The ids come from the same manifest parser `transform` uses, so the
    // labels of the start event and the rules actually applied cannot drift.
    // A malformed manifest falls back to an empty list here: the delegated
    // `transform` run reports the real error, with the field that is wrong.
    #[cfg(feature = "ast")]
    let rule_ids = crate::commands::transform::rule_ids_from_file(
        &args.rules,
        global.effective_max_filesize(),
    )
    .unwrap_or_default();
    #[cfg(not(feature = "ast"))]
    let rule_ids: Vec<String> = Vec::new();
    let campaign_id = args
        .rules
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("rules")
        .to_string();

    writer.write_event(&crate::ndjson_types::CodemodStartEvent {
        r#type: "codemod",
        phase: "start",
        rules: args.rules.display().to_string(),
        rule_ids: rule_ids.clone(),
        rule_id: campaign_id.clone(),
        dry_run: args.dry_run,
    })?;

    let targs = TransformArgs {
        strict: args.strict,
        paths: args.paths.clone(),
        pattern: None,
        rewrite: None,
        language: None,
        include: vec![],
        exclude: vec![],
        dry_run: args.dry_run,
        rules: Some(args.rules.clone()),
        inline_rules: None,
        backup_opts: args.backup_opts.clone(),
        verify_parse: false,
    };

    // Capture transform NDJSON, forward with rule tagging, aggregate by rule_id.
    //
    // The child's verdict is kept instead of propagated: `?` here threw the
    // buffer away, and the buffer holds the child's own failure envelope.
    let mut buf = Vec::new();
    let res = {
        let mut inner = NdjsonWriter::new(&mut buf);
        crate::commands::transform::cmd_transform(
            &targs, global, &mut inner, shutdown, defaults, write_cfg,
        )
    };

    use crate::ndjson_types::CodemodRuleStats;

    let mut by_rule: BTreeMap<String, CodemodRuleStats> = BTreeMap::new();
    for id in &rule_ids {
        by_rule.insert(
            id.clone(),
            CodemodRuleStats {
                matches: 0,
                files: 0,
            },
        );
    }
    if by_rule.is_empty() {
        by_rule.insert(
            campaign_id.clone(),
            CodemodRuleStats {
                matches: 0,
                files: 0,
            },
        );
    }

    // Attribution follows the `rule_begin` stream instead of assuming an order:
    // `transform` emits one `rule_begin` before each rule's events, so the last
    // one seen is the rule that produced the next `transformed`. Assuming
    // `rule_ids.first()` tagged every rule's work with the first id.
    //
    // `matches` is the event's own count of AST matches, never the number of
    // NDJSON lines, because one `transformed` line reports N matches and the
    // summary must add up to the `total_matches` beside it.
    let mut current_rule: Option<String> = None;
    let mut files_seen: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    // The child charged every record as it wrote, so forwarding is a re-emit.
    // The child summary is folded into this verb's terminal instead of being
    // printed beside it, so the campaign ends in exactly one terminal record.
    let reemit = crate::commands::child_reemit::reemit_child_buffer(writer, &buf, |v| {
        // Owned, because the object is mutated below and a borrow would pin it.
        let event_type = v
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string();
        let rid = match event_type.as_str() {
            "rule_begin" => {
                let id = v
                    .get("id")
                    .and_then(|p| p.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| campaign_id.clone());
                current_rule = Some(id.clone());
                id
            }
            "rule_error" => v
                .get("id")
                .and_then(|p| p.as_str())
                .map(str::to_string)
                .or_else(|| current_rule.clone())
                .unwrap_or_else(|| campaign_id.clone()),
            _ => current_rule.clone().unwrap_or_else(|| campaign_id.clone()),
        };
        if let Some(obj) = v.as_object_mut() {
            obj.insert("rule_id".into(), serde_json::Value::String(rid.clone()));
        }

        // Only `transformed` describes a rule's work; `rule_begin`, `rule_error`
        // and `summary` are framing and would inflate the count.
        if event_type != "transformed" {
            return;
        }
        let matches = v
            .get("matches")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let entry = by_rule.entry(rid.clone()).or_insert(CodemodRuleStats {
            matches: 0,
            files: 0,
        });
        entry.matches = entry.matches.saturating_add(matches);
        if let Some(path) = v.get("path").and_then(|p| p.as_str())
            && !path.is_empty()
        {
            files_seen.entry(rid).or_default().insert(path.to_string());
        }
    })?;
    for (rid, paths) in &files_seen {
        if let Some(s) = by_rule.get_mut(rid) {
            s.files = paths.len() as u64;
        }
    }

    // The child failed, and its failure envelope is the campaign's terminal:
    // publishing it keeps the one-line verdict the consumer already watches for.
    // `publish_failure` returns the error tagged `VerdictPublished`, so `main`
    // does not write a second envelope for the same fact.
    if let Err(e) = res {
        if reemit.failed
            && let Some(mut terminal) = reemit.terminal
        {
            if let Some(obj) = terminal.as_object_mut() {
                obj.insert(
                    "rule_id".into(),
                    serde_json::Value::String(campaign_id.clone()),
                );
            }
            return Err(writer.publish_failure(&terminal, e));
        }
        return Err(e);
    }

    writer.write_terminal(&crate::ndjson_types::CodemodSummaryEvent {
        r#type: "codemod_summary",
        rules: args.rules.display().to_string(),
        rule_id: campaign_id,
        dry_run: args.dry_run,
        by_rule_id: by_rule,
        transform_summary: reemit.terminal,
    })?;
    Ok(())
}
