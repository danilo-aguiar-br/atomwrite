// SPDX-License-Identifier: MIT OR Apache-2.0

//! Agent tool surface manifesto — CLI schema bridge WITHOUT MCP (v0.1.29 P2-5).
//!
//! RULES SUPREMAS forbid MCP servers. This command emits a machine-readable
//! inventory of subcommands, flags, and exit codes so hosts wire Bash/argv
//! tools instead of JSON-RPC.
//!
//! Workload: CPU-bound (clap CommandFactory schema walk) — tiny one-shot.
//! Parallelism: none — single process inventory; coordination cost ≫ work.

use std::io::Write;

use anyhow::Result;
use clap::{Args, CommandFactory};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::{Cli, GlobalArgs};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `agent-surface`.
#[derive(Args, Debug)]
pub struct AgentSurfaceArgs {
    /// Output format.
    #[arg(long, default_value = "json")]
    pub format: String,
}

#[derive(Serialize, JsonSchema)]
struct ToolSurface {
    r#type: &'static str,
    name: String,
    summary: String,
    schema_hint: String,
    /// Whether the tool is usable in this build (A-009).
    available: bool,
    /// Optional rebuild / install hint when `available` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    install_hint: Option<String>,
    /// stdout contract for this tool (A-011).
    stdout_kind: String,
}

#[derive(Serialize, JsonSchema)]
struct SurfaceManifest {
    r#type: &'static str,
    version: String,
    integration: &'static str,
    mcp: &'static str,
    tools: Vec<ToolSurface>,
    notes: Vec<String>,
}

/// Emit the agent tool surface manifesto.
#[tracing::instrument(skip_all, fields(command = "agent-surface"))]
pub fn cmd_agent_surface(
    _args: &AgentSurfaceArgs,
    _global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
    _defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    // Prefer clap-derived inventory to avoid manual drift (v0.1.29 residual).
    let mut tools: Vec<ToolSurface> = Vec::new();
    let cmd = Cli::command();
    for sub in cmd.get_subcommands() {
        let name = sub.get_name().to_string();
        let summary = sub
            .get_about()
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.clone());
        let schema_hint = schema_hint_for(&name);
        let (available, install_hint) = availability_for(&name);
        let stdout_kind = stdout_kind_for(&name);
        tools.push(tool(
            &name,
            &summary,
            &schema_hint,
            available,
            install_hint,
            stdout_kind,
        ));
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));

    writer.write_event(&SurfaceManifest {
        r#type: "agent_surface",
        version: env!("CARGO_PKG_VERSION").into(),
        integration: "subprocess-cli-ndjson",
        mcp: "forbidden-use-cli-instead",
        tools,
        notes: vec![
            "MCP servers are forbidden by project rules; use CLI + NDJSON".into(),
            "write supports --durability full|fast|auto (default auto)".into(),
            "fuzzy: auto (default), aggressive, or off (exact-only; G-010)".into(),
            "read --format raw always emits file bytes (not NDJSON); use default format for agent NDJSON (B-006)".into(),
            "help/version/completions are human_only (G-014); agent contract is NDJSON ops".into(),
            "edit requires unique --old match unless --replace-all".into(),
            "match failures include best_candidate and optional candidates[]".into(),
            "recipe run NAME or recipe run --name NAME; excludes *.bak.*; semantic-search excludes *.bak.* unless --include-backups".into(),
            "search skips binary/NUL by default; pass --binary to opt in (A-008)".into(),
            "delete --plan is plan-only; delete --confirm and -y are rejected (B-005/B-015); omit flags to delete".into(),
            "write --confirm / --require-large-ack is large-file overwrite guard (not delete) (B-014)".into(),
            "query uses -Q/--query for S-expression; global -q is quiet (A-005)".into(),
            "watch is feature-gated; default build includes watch when default features enabled; requires --max-events and/or --timeout-secs".into(),
            "semantic-search backend is offline jaccard token overlap (no embeddings, no network) (B-010)".into(),
            "semantic-merge is line-based three-way merge (not AST/embedding) (B-010)".into(),
            "flag glossary: --plan=delete list-only; --confirm on write=large-ack; --confirm on delete=rejected".into(),
        ],
    })?;
    Ok(())
}

/// Map CLI tool name → `docs/schemas/{hint}.schema.json` basename (G-037/G-041).
fn schema_hint_for(name: &str) -> String {
    match name {
        "read" | "stat" => "read-output",
        "write" => "write-output",
        "set" => "set-result",
        "del" => "del-result",
        "case" => "case-result",
        "get" => "get-result",
        "verify" | "hash" => "hash-output",
        "edit" => "edit-output",
        "edit-loop" => "edit-loop-output",
        "search" => "search-match",
        "replace" => "replace-result",
        "batch" => "batch-summary",
        "transform" => "transform-result",
        "scope" => "scope-result",
        "progress" => "progress-event",
        "apply" => "apply-result",
        "backup" => "backup-result",
        "calc" => "calc-output",
        "copy" => "copy-output",
        "count" => "count-summary",
        "delete" => "delete-output",
        "diff" => "diff-output",
        "extract" => "extract-output",
        "list" => "list-entry",
        "move" => "move-output",
        "outline" => "outline-output",
        "prune-backups" => "prune-backups-output",
        "query" => "query-output",
        "regex" => "regex-output",
        "rollback" => "rollback-result",
        "recipe" => "recipe-result",
        "wal-stats" => "wal-stats-output",
        "wal-heal" => "wal-recovery",
        "semantic-merge" => "semantic-merge",
        "sparse" => "sparse",
        "agent-surface" => "agent-surface",
        "watch" => "watch",
        "codemod" => "codemod",
        "semantic-search" => "semantic-search",
        "commands" => "commands",
        "completions" => "completions",
        "doctor" => "doctor",
        "locale" => "locale",
        other => other,
    }
    .into()
}

fn availability_for(name: &str) -> (bool, Option<String>) {
    match name {
        "watch" => {
            if cfg!(feature = "watch") {
                (true, None)
            } else {
                (
                    false,
                    Some(
                        "cargo install atomwrite --features watch (or cargo build --features watch)"
                            .into(),
                    ),
                )
            }
        }
        _ => (true, None),
    }
}

fn stdout_kind_for(name: &str) -> String {
    match name {
        "completions" => "shell-script".into(),
        "help" => "human-text".into(),
        _ => "ndjson".into(),
    }
}

fn tool(
    name: &str,
    summary: &str,
    schema: &str,
    available: bool,
    install_hint: Option<String>,
    stdout_kind: String,
) -> ToolSurface {
    ToolSurface {
        r#type: "tool",
        name: name.into(),
        summary: summary.into(),
        schema_hint: schema.into(),
        available,
        install_hint,
        stdout_kind,
    }
}
