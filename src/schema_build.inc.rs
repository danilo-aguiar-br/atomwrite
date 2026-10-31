// SPDX-License-Identifier: MIT OR Apache-2.0
// Schema builders, included by `schema_dispatch.inc.rs` (A-MONO-001).
//
// Every function here answers "which schema describes the terminal event of
// this verb" and closes that schema. The name tables live in
// `schema_names.inc.rs`.
/// Sentinel name that asks [`emit_schema_by_name`] for the name listing.
///
/// The discovery form of `--print-schema` carries no value, so the early path
/// needs one value to travel through the same channel as a named request. An
/// empty string cannot collide with a real schema name.
pub const SCHEMA_LIST_REQUEST: &str = "";

/// What the early `--print-schema` path should do, decided from the full argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintSchemaPlan {
    /// Clap should parse as usual.
    Defer,
    /// Emit the schema named `String` and exit 0.
    Print(String),
    /// Emit the schema-name listing and exit 0.
    List,
}

/// Scan argv for `--print-schema` in both of the forms its help displays.
///
/// Returns the requested name and the index of the token that carried it.
/// `--print-schema=NAME` is accepted exactly as `--print-schema NAME` is,
/// because the help renders the value as optional (`[<NAME>]`) and clap accepts
/// both; a pre-clap scan that accepted only the separated form answered a
/// correctly written invocation with a parse error.
///
/// An empty name means "list": the flag was present with no value, which is the
/// discovery form.
#[must_use]
pub fn scan_print_schema(argv: &[String]) -> Option<(String, usize)> {
    const FLAG: &str = "--print-schema";
    const FLAG_PREFIX: &str = "--print-schema=";
    for (index, arg) in argv.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(FLAG_PREFIX) {
            return Some((value.to_string(), index));
        }
        if arg == FLAG {
            let separate = argv
                .get(index + 1)
                .map(String::as_str)
                .filter(|s| !s.starts_with('-'));
            return Some((separate.unwrap_or(SCHEMA_LIST_REQUEST).to_string(), index));
        }
    }
    None
}

/// Decide what the early `--print-schema` path should do.
///
/// The command token is the first argv entry that names a subcommand and does
/// not begin with `-`. A flag placed before it is the form the help documents,
/// so it is answered here even when no subcommand was given — the whole reason
/// the path runs before clap is that discovery must work on an install that
/// cannot parse a full command line yet.
///
/// A flag placed AFTER the command, with no name, is not a discovery request:
/// the caller is running a command and the placement is clap's to judge. The
/// earlier behaviour answered the listing instead of running the command, which
/// silently replaced the operation the caller asked for.
///
/// A flag after the command WITH a name stays an explicit request, because a
/// caller that names a schema has asked for that schema from wherever it sits.
#[must_use]
pub fn plan_print_schema(argv: &[String]) -> PrintSchemaPlan {
    let Some((name, token_index)) = scan_print_schema(argv) else {
        return PrintSchemaPlan::Defer;
    };
    let command_index = argv
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, a)| !a.starts_with('-') && SUBCOMMANDS.contains(&a.as_str()))
        .map(|(index, _)| index);
    let flag_after_command = command_index.is_some_and(|command| token_index > command);
    if flag_after_command && name.is_empty() {
        return PrintSchemaPlan::Defer;
    }
    if name.is_empty() {
        PrintSchemaPlan::List
    } else {
        PrintSchemaPlan::Print(name)
    }
}

/// Subcommand tokens the early `--json-schema` prescan is allowed to match.
///
/// The prescan runs before clap, so it needs its own list. Every entry here
/// must resolve in [`emit_schema_by_name`] — the `subcommands_all_emit_schema`
/// test fails the build otherwise, which is what keeps this list from drifting
/// away from the dispatcher.
pub const SUBCOMMANDS: &[&str] = &[
    "read",
    "write",
    "edit",
    "search",
    "replace",
    "hash",
    "delete",
    "count",
    "diff",
    "move",
    "copy",
    "list",
    "extract",
    "calc",
    "regex",
    "transform",
    "scope",
    "batch",
    "backup",
    "rollback",
    "apply",
    "completions",
    "prune-backups",
    "edit-loop",
    "get",
    "set",
    "del",
    "outline",
    "query",
    "case",
    "semantic-merge",
    "sparse",
    "recipe",
    "stat",
    "agent-surface",
    "watch",
    "codemod",
    "semantic-search",
    "verify",
    "wal-stats",
    "wal-heal",
    "doctor",
    "locale",
    "config",
    "commands",
    "progress",
    "error",
    "best-candidate",
    "cancelled",
];

/// Name of the schema that describes a parsed subcommand's terminal output.
///
/// `--json-schema` after a subcommand and `--print-schema NAME` answer through
/// the same table ([`emit_schema_by_name`]), so the two forms cannot disagree.
pub(crate) fn command_schema_name(command: &Commands) -> &'static str {
    match command {
        Commands::Read(_) => "read",
        Commands::Write(_) => "write",
        Commands::Edit(_) => "edit",
        Commands::Search(_) => "search",
        Commands::Replace(_) => "replace",
        Commands::Hash(_) => "hash",
        Commands::Delete(_) => "delete",
        Commands::Count(_) => "count",
        Commands::Diff(_) => "diff",
        Commands::Move(_) => "move",
        Commands::Copy(_) => "copy",
        Commands::List(_) => "list",
        Commands::Extract(_) => "extract",
        Commands::Calc(_) => "calc",
        Commands::Regex(_) => "regex",
        Commands::Transform(_) => "transform",
        Commands::Batch(_) => "batch",
        Commands::Scope(_) => "scope",
        Commands::Backup(_) => "backup",
        Commands::Rollback(_) => "rollback",
        Commands::Apply(_) => "apply",
        Commands::Set(_) => "set",
        Commands::Get(_) => "get",
        Commands::Del(_) => "del",
        Commands::Case(_) => "case",
        Commands::Query(_) => "query",
        Commands::Outline(_) => "outline",
        Commands::WalStats(_) => "wal-stats",
        Commands::WalHeal(_) => "wal-heal",
        Commands::PruneBackups(_) => "prune-backups",
        Commands::EditLoop(_) => "edit-loop",
        Commands::Verify(_) => "verify",
        Commands::SemanticMerge(_) => "semantic-merge",
        Commands::Sparse(_) => "sparse",
        Commands::Recipe(_) => "recipe",
        Commands::Stat(_) => "stat",
        Commands::AgentSurface(_) => "agent-surface",
        Commands::Watch(_) => "watch",
        Commands::ExtractArtifact(_) => "extract-artifact",
        Commands::Codemod(_) => "codemod",
        Commands::SemanticSearch(_) => "semantic-search",
        Commands::Scratch(_) => "scratch",
        Commands::Doctor(_) => "doctor",
        Commands::Locale(_) => "locale",
        Commands::Config(_) => "config",
        Commands::CommandsTree(_) => "commands",
        Commands::Completions(_) => "completions",
    }
}

/// Emit the JSON Schema for the given subcommand's NDJSON output.
fn emit_json_schema(command: &Commands, out: impl Write) -> Result<()> {
    emit_schema_by_name(command_schema_name(command), out).map(|_| ())
}

/// Keys the writer stamps on a terminal event, as schema properties.
///
/// `ok`, `code` and `exit` come from the verdict stamp and `target_resolved`
/// and `target_source` from the resolved-target stamp. No output struct
/// declares them, because the writer is the single place that adds them, so
/// the served schema is the place that documents them.
fn writer_stamped_properties() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "ok",
            serde_json::json!({"type": "boolean", "description": "Outcome of the run, stamped by the writer"}),
        ),
        (
            "code",
            serde_json::json!({"type": "string", "description": "Machine error code, present when ok is false"}),
        ),
        (
            "exit",
            serde_json::json!({"type": "integer", "minimum": 0, "description": "Process exit status, present when ok is false"}),
        ),
        (
            "target_resolved",
            serde_json::json!({"type": "string", "description": "Absolute target the run resolved, or `-` for standard input; several subjects are joined with `,` in invocation order"}),
        ),
        (
            "target_source",
            serde_json::json!({"type": "string", "enum": ["argv", "workspace", "stdin"], "description": "Where the target came from: named in the invocation line, inherited from the workspace slot, or the standard-input stream"}),
        ),
        (
            "workspace_source",
            serde_json::json!({"type": "string", "enum": ["argv", "cwd"], "description": "Where the workspace root came from: the `--workspace` flag or the process working directory"}),
        ),
        (
            "records_suppressed",
            serde_json::json!({"type": "integer", "minimum": 0, "description": "Records the writer dropped before the terminal event: `--filter` rejections, `--select` projections that emptied the record, dedupe duplicates and `--limit` cuts. It counts records, never files, so it does not compare with `files_matched`: one file can contribute several records"}),
        ),
        (
            "files_budget_dropped",
            serde_json::json!({"type": "integer", "minimum": 0, "description": "Entries the `--limit` budget dropped, in the unit the verb produces: files for walking verbs, records for streaming ones; a floor, because workers may stop before producing the rest"}),
        ),
        (
            "files_planned",
            serde_json::json!({"type": "integer", "minimum": 0, "description": "Files the run would have modified in `--dry-run` or `--preview`; absent from a run that really wrote"}),
        ),
        (
            "truncated",
            serde_json::json!({"type": "boolean", "description": "The record itself was cut by `--truncate-content` or `--max-output-bytes`, so the fields are shorter than the source"}),
        ),
        (
            "truncated_records",
            serde_json::json!({"type": "integer", "minimum": 0, "description": "Records the writer cut this run, counted once per record"}),
        ),
    ]
}

/// Add the writer-stamped keys to the root `properties` of a terminal schema.
///
/// A key the output struct already declares (for example `ok` on `doctor`)
/// keeps its own definition.
fn with_envelope_keys(mut schema: schemars::Schema) -> schemars::Schema {
    if let Some(root) = schema.as_object_mut() {
        let properties = root
            .entry("properties")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(map) = properties.as_object_mut() {
            for (key, definition) in writer_stamped_properties() {
                map.entry(key).or_insert(definition);
            }
        }
    }
    schema
}

/// Schema of the terminal event of the file-operation verbs.
fn file_verb_schema(name: &str) -> Option<schemars::Schema> {
    Some(match name {
        "read" | "stat" => schemars::schema_for!(ndjson_types::ReadOutput),
        "write" => schemars::schema_for!(ndjson_types::WriteOutput),
        "write-risk" => schemars::schema_for!(ndjson_types::WriteRiskAssessment),
        "edit" => schemars::schema_for!(ndjson_types::EditOutput),
        "search" => return search_output_schema(),
        "replace" => return replace_output_schema(),
        "hash" => {
            return record_with_summary_schema(
                schemars::schema_for!(ndjson_types::HashOutput),
                "HashOutput",
                "Records of `hash`: one digest per file, then the terminal summary.",
            );
        }
        "verify" => {
            return record_with_summary_schema(
                schemars::schema_for!(ndjson_types::HashVerifyOutput),
                "HashVerifyOutput",
                "Records of `verify`: one verdict per file, then the terminal summary.",
            );
        }
        "delete" => {
            return record_with_summary_schema(
                schemars::schema_for!(ndjson_types::DeleteOutput),
                "DeleteOutput",
                "Records of `delete`: one deleted file, then the terminal summary.",
            );
        }
        "count" => schemars::schema_for!(ndjson_types::CountTotalOutput),
        "count-by-ext" => schemars::schema_for!(ndjson_types::CountByExtOutput),
        "count-by-size" => schemars::schema_for!(ndjson_types::CountBySizeOutput),
        "diff" => schemars::schema_for!(ndjson_types::DiffOutput),
        "move" => schemars::schema_for!(ndjson_types::MoveOutput),
        "copy" => {
            return record_with_summary_schema(
                schemars::schema_for!(ndjson_types::CopyOutput),
                "CopyOutput",
                "Records of `copy`: one copied file, then the terminal summary.",
            );
        }
        "list" => return list_output_schema(),
        "extract" => return extract_output_schema(),
        "calc" => schemars::schema_for!(ndjson_types::CalcOutput),
        "regex" => schemars::schema_for!(ndjson_types::RegexOutput),
        "transform" => {
            return record_with_summary_schema(
                schemars::schema_for!(ndjson_types::TransformResult),
                "TransformOutput",
                "Records of `transform`: one transformed file, then the terminal summary.",
            );
        }
        "batch" => return batch_output_schema(),
        "scope" => return scope_output_schema(),
        "backup" => return backup_output_schema(),
        "rollback" => schemars::schema_for!(ndjson_types::RollbackResult),
        "apply" => schemars::schema_for!(ndjson_types::ApplyResult),
        "prune-backups" => {
            return record_with_own_summary(
                schemars::schema_for!(ndjson_types::PruneBackupEntry),
                schemars::schema_for!(ndjson_types::PruneBackupSummary),
                "PruneBackupsOutput",
                "Records of `prune-backups`: one entry per backup handled, then the terminal summary.",
            );
        }
        "edit-loop" => schemars::schema_for!(ndjson_types::EditLoopSummary),
        "completions" => schemars::schema_for!(CompletionsInstalled),
        "extract-artifact" => {
            schemars::schema_for!(crate::commands::extract_artifact::ArtifactOutput)
        }
        "recipe" => return recipe_output_schema(),
        "watch" => return watch_output_schema(),
        _ => return None,
    })
}

/// Schema of the terminal event of the structured and reporting verbs.
///
/// Each arm names the struct the verb really serializes, so the served schema
/// and the wire cannot drift apart without the type checker noticing.
fn report_verb_schema(name: &str) -> Option<schemars::Schema> {
    Some(match name {
        "set" => schemars::schema_for!(crate::commands::set::SetResult),
        "get" => schemars::schema_for!(crate::commands::get::GetResult),
        "del" => schemars::schema_for!(crate::commands::del::DelResult),
        "case" => return case_output_schema(),
        #[cfg(feature = "ast")]
        "query" => schemars::schema_for!(crate::commands::query::QuerySummary),
        #[cfg(feature = "ast")]
        "outline" => schemars::schema_for!(crate::commands::outline::OutlineSummary),
        // Without `ast` the verbs fail closed and emit no event of their own.
        #[cfg(not(feature = "ast"))]
        "query" | "outline" => schemars::schema_for!(ndjson_types::ProgressEvent),
        "locale" => schemars::schema_for!(crate::commands::locale_cmd::LocaleReport),
        "config" => schemars::schema_for!(crate::commands::config_cmd::ConfigOutput),
        "scratch" => schemars::schema_for!(crate::commands::scratch::ScratchInitEvent),
        "wal-stats" => schemars::schema_for!(ndjson_types::WalStats),
        "wal-heal" => schemars::schema_for!(ndjson_types::AutoHealReport),
        "semantic-merge" => return semantic_merge_output_schema(),
        "sparse" => return sparse_output_schema(),
        "agent-surface" => schemars::schema_for!(crate::commands::agent_surface::SurfaceManifest),
        "codemod" => return codemod_output_schema(),
        "semantic-search" => return semantic_search_output_schema(),
        "doctor" => schemars::schema_for!(crate::commands::doctor::DoctorReport),
        "commands" => schemars::schema_for!(crate::commands::command_tree::CommandsTreeReport),
        _ => return None,
    })
}

/// Schema of the records that are not the terminal event of one verb.
fn event_schema(name: &str) -> Option<schemars::Schema> {
    Some(match name {
        "progress" => schemars::schema_for!(ndjson_types::ProgressEvent),
        // The writer stamps the resolved target on the error envelope too, so
        // the served schema declares the same two keys the success path has.
        "error" => with_envelope_keys(schemars::schema_for!(crate::error::ErrorJson)),
        "best-candidate" => schemars::schema_for!(ndjson_types::BestCandidate),
        "cancelled" => schemars::schema_for!(ndjson_types::CancelledEvent),
        "stream_diff" => schemars::schema_for!(crate::output::StreamDiffOutput),
        _ => return None,
    })
}

/// Emit the JSON Schema for a subcommand by name, without requiring parsed args.
///
/// The empty name ([`SCHEMA_LIST_REQUEST`]) emits the sorted name listing
/// instead of a schema, so discovery and emission share one entry point.
///
/// Returns `Ok(true)` if something was emitted, `Ok(false)` if the name is
/// unknown, so a typo fails closed instead of writing an empty document.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn emit_schema_by_name(name: &str, mut out: impl Write) -> Result<bool> {
    if name == SCHEMA_LIST_REQUEST {
        let mut names: Vec<&str> = SCHEMA_NAMES.to_vec();
        names.sort_unstable();
        for name in names {
            serde_json::to_writer(&mut out, &name).map_err(std::io::Error::from)?;
            out.write_all(b"\n")?;
        }
        out.flush()?;
        return Ok(true);
    }
    let verb = file_verb_schema(name).or_else(|| report_verb_schema(name));
    let closed = verb
        .map(|schema| {
            // A union carries its envelope keys on the terminal branch, never on
            // the root: the root declares no keys of its own.
            let union_root = schema
                .as_object()
                .is_some_and(|root| root.contains_key("anyOf") || root.contains_key("oneOf"));
            if name == "scope" || union_root {
                schema
            } else {
                with_envelope_keys(schema)
            }
        })
        .map(|schema| close_terminal_schema(name, schema));
    let Some(schema) = closed.or_else(|| event_schema(name)) else {
        return Ok(false);
    };
    // `io::Error::from` keeps the kind of a failed write, so a closed reader
    // stays recognisable as a broken pipe further up.
    serde_json::to_writer_pretty(&mut out, &schema).map_err(std::io::Error::from)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(true)
}
