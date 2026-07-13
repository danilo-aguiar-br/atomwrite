// SPDX-License-Identifier: MIT OR Apache-2.0

//! Multi-rule AST codemod campaign runner (v0.1.29 P3-3).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalArgs;
use crate::cli_args::{BackupOpts, TransformArgs};
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `codemod`.
#[derive(Args, Debug)]
pub struct CodemodArgs {
    /// Path to YAML rules file (ast-grep multi-rule format).
    #[arg(long)]
    pub rules: PathBuf,
    /// Roots to scan.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,
    /// Dry-run only.
    #[arg(long)]
    pub dry_run: bool,
    /// Shared backup flags for apply mode.
    #[command(flatten)]
    pub backup_opts: BackupOpts,
}

/// Run a multi-rule codemod campaign by delegating to `transform --rules`.
pub fn cmd_codemod(
    args: &CodemodArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    if shutdown.is_shutdown() {
        return Err(AtomwriteError::Cancelled {
            reason: "codemod cancelled".into(),
            exit: 143,
        }
        .into());
    }
    if !args.rules.exists() {
        return Err(AtomwriteError::NotFound {
            path: args.rules.clone(),
        }
        .into());
    }

    let rule_ids = parse_rule_ids(&args.rules);
    let campaign_id = args
        .rules
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("rules")
        .to_string();

    writer.write_event(&serde_json::json!({
        "type": "codemod",
        "phase": "start",
        "rules": args.rules.display().to_string(),
        "rule_ids": rule_ids,
        "rule_id": campaign_id,
        "dry_run": args.dry_run,
    }))?;

    let targs = TransformArgs {
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
    let mut buf = Vec::new();
    {
        let mut inner = NdjsonWriter::new(&mut buf);
        crate::commands::transform::cmd_transform(&targs, global, &mut inner, shutdown, defaults)?;
    }

    let mut by_rule: BTreeMap<String, RuleStats> = BTreeMap::new();
    for id in &rule_ids {
        by_rule.insert(
            id.clone(),
            RuleStats {
                matches: 0,
                files: 0,
            },
        );
    }
    if by_rule.is_empty() {
        by_rule.insert(
            campaign_id.clone(),
            RuleStats {
                matches: 0,
                files: 0,
            },
        );
    }

    let mut files_seen: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for line in String::from_utf8_lossy(&buf).lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(line) {
            let path = v
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            let rid = v
                .get("rule_id")
                .or_else(|| v.get("id"))
                .and_then(|p| p.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    // Round-robin assign to known ids, else campaign id.
                    rule_ids
                        .first()
                        .cloned()
                        .unwrap_or_else(|| campaign_id.clone())
                });
            if let Some(obj) = v.as_object_mut() {
                obj.insert("rule_id".into(), serde_json::json!(rid));
            }
            writer.write_event(&v)?;

            let entry = by_rule.entry(rid.clone()).or_insert(RuleStats {
                matches: 0,
                files: 0,
            });
            entry.matches += 1;
            if !path.is_empty() {
                files_seen.entry(rid).or_default().insert(path);
            }
        }
    }
    for (rid, paths) in &files_seen {
        if let Some(s) = by_rule.get_mut(rid) {
            s.files = paths.len() as u64;
        }
    }

    let by_rule_json: serde_json::Map<String, serde_json::Value> = by_rule
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                serde_json::json!({
                    "matches": v.matches,
                    "files": v.files,
                }),
            )
        })
        .collect();

    writer.write_event(&serde_json::json!({
        "type": "codemod_summary",
        "rules": args.rules.display().to_string(),
        "rule_id": campaign_id,
        "dry_run": args.dry_run,
        "by_rule_id": by_rule_json,
    }))?;
    Ok(())
}

struct RuleStats {
    matches: u64,
    files: u64,
}

/// Extract `id:` fields from ast-grep-style YAML rules (best-effort, no full YAML dep required on core).
fn parse_rule_ids(path: &std::path::Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        // Match `id: foo` or `- id: foo`
        let rest = trimmed
            .strip_prefix("- ")
            .unwrap_or(trimmed)
            .strip_prefix("id:")
            .map(str::trim);
        if let Some(id) = rest {
            let id = id.trim_matches('"').trim_matches('\'').trim();
            if !id.is_empty() && !id.contains(':') {
                ids.push(id.to_string());
            }
        }
    }
    ids
}
