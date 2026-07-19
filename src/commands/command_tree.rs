// SPDX-License-Identifier: MIT OR Apache-2.0

//! Full command tree as JSON (`atomwrite commands`).
//!
//! Machine-readable discovery surface for LLM agents — complements
//! `agent-surface` with nested clap structure (name, about, aliases, args).
//!
//! Workload: CPU-bound (clap CommandFactory walk) — tiny one-shot.
//! Parallelism: none — single schema emit; coordination cost ≫ work.

use std::io::Write;

use anyhow::Result;
use clap::{Args, CommandFactory};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::{Cli, GlobalArgs};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `commands`.
#[derive(Args, Debug, Default)]
pub struct CommandsTreeArgs {
    /// Include global flags in the tree root.
    #[arg(
        long,
        action = clap::ArgAction::SetTrue,
        help = "Include global flags under the root node"
    )]
    pub include_globals: bool,
}

#[derive(Serialize, JsonSchema)]
struct CommandsTreeReport {
    r#type: &'static str,
    version: String,
    name: String,
    about: String,
    commands: Vec<CommandNode>,
    global_args: Vec<ArgNode>,
}

#[derive(Serialize, JsonSchema)]
struct CommandNode {
    name: String,
    about: String,
    aliases: Vec<String>,
    args: Vec<ArgNode>,
    subcommands: Vec<CommandNode>,
}

#[derive(Serialize, JsonSchema)]
struct ArgNode {
    name: String,
    long: Option<String>,
    short: Option<String>,
    required: bool,
    takes_value: bool,
    help: String,
}

/// Emit the full clap command tree as one NDJSON record.
#[tracing::instrument(skip_all, fields(command = "commands-tree"))]
pub fn cmd_commands_tree(
    args: &CommandsTreeArgs,
    _global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
    _defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    let cmd = Cli::command();
    let about = cmd
        .get_about()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let mut commands: Vec<CommandNode> = cmd
        .get_subcommands()
        .map(command_node)
        .collect();
    commands.sort_by(|a, b| a.name.cmp(&b.name));

    let global_args = if args.include_globals {
        arg_nodes(&cmd)
    } else {
        Vec::new()
    };

    writer.write_event(&CommandsTreeReport {
        r#type: "commands_tree",
        version: env!("CARGO_PKG_VERSION").into(),
        name: cmd.get_name().to_string(),
        about,
        commands,
        global_args,
    })?;
    Ok(())
}

fn command_node(cmd: &clap::Command) -> CommandNode {
    let mut subcommands: Vec<CommandNode> = cmd.get_subcommands().map(command_node).collect();
    subcommands.sort_by(|a, b| a.name.cmp(&b.name));
    CommandNode {
        name: cmd.get_name().to_string(),
        about: cmd
            .get_about()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        aliases: cmd.get_all_aliases().map(|s| s.to_string()).collect(),
        args: arg_nodes(cmd),
        subcommands,
    }
}

fn arg_nodes(cmd: &clap::Command) -> Vec<ArgNode> {
    cmd.get_arguments()
        .filter(|a| !a.is_hide_set())
        .map(|a| ArgNode {
            name: a.get_id().to_string(),
            long: a.get_long().map(|s| s.to_string()),
            short: a.get_short().map(|c| c.to_string()),
            required: a.is_required_set(),
            takes_value: !matches!(
                a.get_action(),
                clap::ArgAction::SetTrue
                    | clap::ArgAction::SetFalse
                    | clap::ArgAction::Count
                    | clap::ArgAction::Help
                    | clap::ArgAction::HelpLong
                    | clap::ArgAction::HelpShort
                    | clap::ArgAction::Version
            ),
            help: a
                .get_help()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        })
        .collect()
}
