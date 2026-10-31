// SPDX-License-Identifier: MIT OR Apache-2.0

//! Agent tool surface manifesto — CLI schema bridge (v0.1.29 P2-5).
//!
//! This command emits a machine-readable inventory of subcommands, flags, and
//! exit codes so hosts wire Bash/argv tools: CLI + NDJSON per subprocess.
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
    /// Output format (only `json` is produced; any other value is ignored).
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
pub(crate) struct SurfaceManifest {
    r#type: &'static str,
    version: String,
    integration: &'static str,
    tools: Vec<ToolSurface>,
    /// Every process exit status this build can return, with its machine
    /// code, recovery class and provenance. Derived from the error enum
    /// (B3) so the list cannot drift from the code that produces it.
    exit_codes: Vec<ExitCodeEntry>,
    /// Flag semantics that are contract rather than prose: what each flag
    /// means and which combinations the CLI rejects.
    flag_contract: Vec<FlagContractEntry>,
    /// How to discover the schema surface without parsing help text.
    discovery: DiscoveryEntry,
    notes: Vec<String>,
}

/// The schema-discovery door, published so an agent can find it by field.
///
/// Without this, discovering that a schema dump exists at all meant reading
/// `--help` prose and hoping the wording survived a release. The names come from
/// [`crate::SCHEMA_NAMES`], the same list the dispatcher serves, so the count
/// here cannot drift from what `--print-schema` actually answers.
#[derive(Serialize, JsonSchema)]
struct DiscoveryEntry {
    /// Global flag that answers both forms.
    flag: &'static str,
    /// What the no-argument form emits.
    list_form: &'static str,
    /// What the name-argument form emits.
    name_form: &'static str,
    /// Every schema name this build serves, alphabetically ordered.
    schema_names: Vec<&'static str>,
}

/// One flag whose semantics are part of the agent contract.
#[derive(Serialize, JsonSchema)]
struct FlagContractEntry {
    /// Flag name as accepted on the command line.
    flag: &'static str,
    /// What the flag means, including the combinations the CLI rejects.
    semantics: &'static str,
}

/// Emit the agent tool surface manifesto.
///
/// # Errors
///
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
/// Returns a serialization error when the manifest cannot be encoded as JSON.
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
        tools,
        exit_codes: exit_code_entries(),
        flag_contract: flag_contract_entries(),
        discovery: discovery_entry(),
        notes: vec!["Integration is CLI + NDJSON per subprocess".into()],
    })?;
    Ok(())
}

/// The schema-discovery door, built from the dispatcher's own name list.
fn discovery_entry() -> DiscoveryEntry {
    let mut schema_names = crate::SCHEMA_NAMES.to_vec();
    schema_names.sort_unstable();
    DiscoveryEntry {
        flag: "--print-schema",
        list_form: "no value: one NDJSON line per schema name, alphabetically ordered",
        name_form: "with NAME: that schema on the data channel; an unknown name is a typed error",
        schema_names,
    }
}

include!("agent_surface_exit.inc.rs");

/// Contract-level flag semantics lifted out of the prose notes (D3).
///
/// These were sentences inside the manifest; as structured entries a host
/// can read them by flag name instead of matching on free text.
fn flag_contract_entries() -> Vec<FlagContractEntry> {
    vec![
        FlagContractEntry {
            flag: "--durability",
            semantics: "one of full|fast|auto; default auto",
        },
        FlagContractEntry {
            flag: "--fuzzy",
            semantics: "one of auto (default) | aggressive | off (exact-only only)",
        },
        FlagContractEntry {
            flag: "--plan",
            semantics: "delete list-only preview; the run that deletes omits it",
        },
        FlagContractEntry {
            flag: "--ack-overwrite",
            semantics: "required to overwrite an existing target above the configured large-file threshold",
        },
        FlagContractEntry {
            flag: "--confirm",
            semantics: "rejected on delete; the flag cannot be combined with a live deletion",
        },
        FlagContractEntry {
            flag: "-y",
            semantics: "rejected on delete; the flag cannot be combined with a live deletion",
        },
        FlagContractEntry {
            flag: "--replace-all",
            semantics: "required when --old matches more than once; without it a multi-match edit fails",
        },
        FlagContractEntry {
            flag: "--binary",
            semantics: "search skips binary and NUL content by default; this flag opts in",
        },
        FlagContractEntry {
            flag: "--max-events",
            semantics: "watch bound; at least one of --max-events or --timeout-secs is required",
        },
        FlagContractEntry {
            flag: "--old-file",
            semantics: "content source staged inside the workspace; write the two files under --workspace",
        },
        FlagContractEntry {
            flag: "-Q",
            semantics: "query S-expression; the global -q is the quiet flag, not this one",
        },
        FlagContractEntry {
            flag: "--no-conflict-markers",
            semantics: "semantic-merge prefer-ours; conflict markers are on by default",
        },
    ]
}

/// Map CLI tool name → `docs/schemas/{hint}.schema.json` basename (G-037/G-041).
fn schema_hint_for(name: &str) -> String {
    match name {
        "read" | "stat" => "read-output",
        "write" => "write-output",
        "set" => "set-result",
        "del" => "del-result",
        "case" => "case-output",
        "get" => "get-result",
        "verify" | "hash" => "hash-output",
        "edit" => "edit-output",
        "edit-loop" => "edit-loop-output",
        "search" => "search-output",
        "replace" => "replace-output",
        "batch" => "batch-output",
        "transform" => "transform-result",
        "scope" => "scope-result",
        "progress" => "progress-event",
        "apply" => "apply-result",
        "backup" => "backup-output",
        "calc" => "calc-output",
        "copy" => "copy-output",
        "count" => "count-summary",
        "delete" => "delete-output",
        "diff" => "diff-output",
        "extract" => "extract-output",
        "list" => "list-output",
        "move" => "move-output",
        "outline" => "outline-output",
        "prune-backups" => "prune-backups-output",
        "query" => "query-output",
        "regex" => "regex-output",
        "rollback" => "rollback-result",
        "recipe" => "recipe-output",
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
        "config" => "config",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::CANONICAL_SUBCOMMAND_COUNT;

    /// B3: the exit status list is not empty and every entry is well formed.
    #[test]
    fn exit_code_entries_are_well_formed() {
        let codes = exit_code_entries();
        assert!(!codes.is_empty(), "the surface must publish exit statuses");
        for entry in &codes {
            assert!(
                matches!(entry.source, "error-variant" | "process"),
                "unknown provenance {:?} for status {}",
                entry.source,
                entry.code
            );
            assert!(!entry.name.is_empty(), "status {} has no name", entry.code);
        }
    }

    /// SSOT: docs/agent-facing counts must match clap inventory (v0.1.36 residual).
    #[test]
    fn clap_subcommand_count_matches_canonical_ssot() {
        let n = Cli::command().get_subcommands().count();
        assert_eq!(
            n, CANONICAL_SUBCOMMAND_COUNT,
            "update CANONICAL_SUBCOMMAND_COUNT and agent-facing docs when adding a subcommand (got {n})"
        );
    }

    /// B3: the exit status list is derived, and `FIFO_DETECTED` is in it.
    ///
    /// The regression this guards is exit 85 existing in the binary without
    /// ever appearing on the agent surface.
    #[test]
    fn exit_codes_are_derived_and_include_fifo_detected() {
        let codes = exit_code_entries();
        let fifo = codes
            .iter()
            .find(|e| e.name == "FIFO_DETECTED")
            .expect("FIFO_DETECTED must be on the agent surface");
        assert_eq!(fifo.code, 85, "FIFO_DETECTED exit code must be 85");
        assert_eq!(fifo.source, "error-variant");
        // Derivation, not transcription: the entry agrees with the enum.
        let probe = crate::error::AtomwriteError::FifoDetected {
            path: std::path::PathBuf::from("/probe"),
        };
        assert_eq!(fifo.code, probe.exit_code());
        assert_eq!(fifo.error_class, probe.error_class().as_str());
        assert_eq!(fifo.retryable, probe.is_retryable());
    }

    /// B3: every variant reaches the list, and each variant's own status is
    /// the one the list reports for its machine code.
    #[test]
    fn every_error_variant_is_covered_by_the_exit_code_list() {
        let variants = sample_variants();
        let mut seen = vec![false; variants.len()];
        for err in &variants {
            let idx = variant_ordinal(err) as usize;
            assert!(
                idx < seen.len(),
                "variant ordinal {idx} is outside the {} sampled variants",
                seen.len()
            );
            assert!(!seen[idx], "duplicate instance for variant ordinal {idx}");
            seen[idx] = true;
        }
        assert!(
            seen.iter().all(|s| *s),
            "sample_variants is missing one or more variants"
        );

        let codes = exit_code_entries();
        for err in &variants {
            let hit = codes
                .iter()
                .any(|e| e.code == err.exit_code() && e.name == err.error_code());
            assert!(
                hit,
                "variant {} (exit {}) missing from the agent surface list",
                err.error_code(),
                err.exit_code()
            );
        }
    }

    /// B3: the list is sorted and free of duplicate (code, name) pairs.
    #[test]
    fn exit_code_list_is_sorted_and_deduplicated() {
        let codes = exit_code_entries();
        let mut sorted = codes.iter().map(|e| e.code).collect::<Vec<_>>();
        let before = sorted.clone();
        sorted.sort_unstable();
        assert_eq!(before, sorted, "exit code list must be ordered by code");
        let mut pairs: Vec<(u8, &str)> = codes.iter().map(|e| (e.code, e.name)).collect();
        let len_before = pairs.len();
        pairs.sort_unstable();
        pairs.dedup();
        assert_eq!(len_before, pairs.len(), "duplicate (code, name) entry");
    }
}
