// SPDX-License-Identifier: MIT OR Apache-2.0

//! Agent tool surface manifesto — CLI schema bridge WITHOUT MCP (v0.1.29 P2-5).
//!
//! RULES SUPREMAS forbid MCP servers. This command emits a machine-readable
//! inventory of subcommands, flags, and exit codes so hosts wire Bash/argv
//! tools instead of JSON-RPC.

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
        tools.push(tool(&name, &summary, &schema_hint));
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
            "fuzzy is mandatory: auto (default) or aggressive; off is rejected since v0.1.30".into(),
            "edit requires unique --old match unless --replace-all".into(),
            "match failures include best_candidate and optional candidates[]".into(),
            "recipe run excludes *.bak.* by default; progress_every 50 on replace".into(),
            "watch requires: cargo install atomwrite --features watch".into(),
            "semantic-search backend is offline jaccard (no embeddings)".into(),
            "semantic-merge is line-based three-way merge (not AST/embedding)".into(),
        ],
    })?;
    Ok(())
}

fn schema_hint_for(name: &str) -> String {
    match name {
        "read" | "stat" => "read-output",
        "write" | "set" | "del" | "case" | "verify" => "write-output",
        "edit" => "edit-output",
        "search" => "search-match",
        "replace" => "replace-result",
        "hash" => "hash-output",
        "batch" => "batch-summary",
        "semantic-merge" => "semantic-merge",
        "sparse" => "sparse",
        "recipe" => "recipe",
        "agent-surface" => "agent-surface",
        "watch" => "watch",
        "codemod" => "codemod",
        "semantic-search" => "semantic-search",
        "transform" => "transform-result",
        "scope" => "scope-result",
        "progress" => "progress-event",
        other => other,
    }
    .into()
}

fn tool(name: &str, summary: &str, schema: &str) -> ToolSurface {
    ToolSurface {
        r#type: "tool",
        name: name.into(),
        summary: summary.into(),
        schema_hint: schema.into(),
    }
}
