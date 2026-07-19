// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by lib.rs (schema dispatch — A-MONO-001).

/// Emit the JSON Schema for the given subcommand's NDJSON output.
fn emit_json_schema(command: &Commands, mut out: impl Write) -> Result<()> {
    let schema = match command {
        Commands::Read(_) => schemars::schema_for!(ndjson_types::ReadOutput),
        Commands::Write(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Edit(_) => schemars::schema_for!(ndjson_types::EditOutput),
        Commands::Search(_) => schemars::schema_for!(ndjson_types::SearchMatch),
        Commands::Replace(_) => schemars::schema_for!(ndjson_types::ReplaceResult),
        Commands::Hash(_) => schemars::schema_for!(ndjson_types::HashOutput),
        Commands::Delete(_) => schemars::schema_for!(ndjson_types::DeleteOutput),
        Commands::Count(_) => schemars::schema_for!(ndjson_types::Summary),
        Commands::Diff(_) => schemars::schema_for!(ndjson_types::DryRunPlan),
        Commands::Move(_) => schemars::schema_for!(ndjson_types::MoveOutput),
        Commands::Copy(_) => schemars::schema_for!(ndjson_types::CopyOutput),
        Commands::List(_) => schemars::schema_for!(ndjson_types::ListEntry),
        Commands::Extract(_) => schemars::schema_for!(ndjson_types::CalcOutput),
        Commands::Calc(_) => schemars::schema_for!(ndjson_types::CalcOutput),
        Commands::Regex(_) => schemars::schema_for!(ndjson_types::RegexOutput),
        Commands::Transform(_) => schemars::schema_for!(ndjson_types::TransformResult),
        Commands::Batch(_) => schemars::schema_for!(ndjson_types::BatchSummary),
        Commands::Scope(_) => schemars::schema_for!(ndjson_types::ScopeResult),
        Commands::Backup(_) => schemars::schema_for!(ndjson_types::BackupResult),
        Commands::Rollback(_) => schemars::schema_for!(ndjson_types::RollbackResult),
        Commands::Apply(_) => schemars::schema_for!(ndjson_types::ApplyResult),
        Commands::Set(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Get(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Del(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Case(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Query(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Outline(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::WalStats(_) => schemars::schema_for!(ndjson_types::WalStats),
        Commands::WalHeal(_) => schemars::schema_for!(ndjson_types::AutoHealReport),
        Commands::PruneBackups(_) => schemars::schema_for!(ndjson_types::PruneBackupSummary),
        Commands::EditLoop(_) => schemars::schema_for!(ndjson_types::EditLoopSummary),
        Commands::Verify(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::SemanticMerge(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Sparse(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Recipe(_) => schemars::schema_for!(crate::commands::recipe::RecipeResult),
        Commands::Stat(_) => schemars::schema_for!(ndjson_types::ReadOutput),
        Commands::AgentSurface(_) => schemars::schema_for!(ndjson_types::WriteOutput),
        Commands::Watch(_) => schemars::schema_for!(ndjson_types::WatchSummary),
        Commands::Codemod(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::SemanticSearch(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::Doctor(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::Locale(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::CommandsTree(_) => schemars::schema_for!(ndjson_types::ProgressEvent),
        Commands::Completions(_) => schemars::schema_for!(ndjson_types::CalcOutput),
    };
    serde_json::to_writer_pretty(&mut out, &schema)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// Emit the JSON Schema for a subcommand by name, without requiring parsed args.
///
/// Returns `Ok(true)` if the schema was emitted, `Ok(false)` if the name is unknown.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn emit_schema_by_name(name: &str, mut out: impl Write) -> Result<bool> {
    let schema = match name {
        "read" => schemars::schema_for!(ndjson_types::ReadOutput),
        "write" => schemars::schema_for!(ndjson_types::WriteOutput),
        "edit" => schemars::schema_for!(ndjson_types::EditOutput),
        "search" => schemars::schema_for!(ndjson_types::SearchMatch),
        "replace" => schemars::schema_for!(ndjson_types::ReplaceResult),
        "hash" => schemars::schema_for!(ndjson_types::HashOutput),
        "delete" => schemars::schema_for!(ndjson_types::DeleteOutput),
        "count" => schemars::schema_for!(ndjson_types::Summary),
        "diff" => schemars::schema_for!(ndjson_types::DryRunPlan),
        "move" => schemars::schema_for!(ndjson_types::MoveOutput),
        "copy" => schemars::schema_for!(ndjson_types::CopyOutput),
        "list" => schemars::schema_for!(ndjson_types::ListEntry),
        "extract" => schemars::schema_for!(ndjson_types::CalcOutput),
        "calc" => schemars::schema_for!(ndjson_types::CalcOutput),
        "regex" => schemars::schema_for!(ndjson_types::RegexOutput),
        "transform" => schemars::schema_for!(ndjson_types::TransformResult),
        "batch" => schemars::schema_for!(ndjson_types::BatchSummary),
        "scope" => schemars::schema_for!(ndjson_types::ScopeResult),
        "backup" => schemars::schema_for!(ndjson_types::BackupResult),
        "rollback" => schemars::schema_for!(ndjson_types::RollbackResult),
        "apply" => schemars::schema_for!(ndjson_types::ApplyResult),
        "set" => schemars::schema_for!(ndjson_types::WriteOutput),
        "get" => schemars::schema_for!(ndjson_types::WriteOutput),
        "del" => schemars::schema_for!(ndjson_types::WriteOutput),
        "case" => schemars::schema_for!(ndjson_types::WriteOutput),
        "query" => schemars::schema_for!(ndjson_types::WriteOutput),
        "outline" => schemars::schema_for!(ndjson_types::WriteOutput),
        "prune-backups" => schemars::schema_for!(ndjson_types::PruneBackupSummary),
        "edit-loop" => schemars::schema_for!(ndjson_types::EditLoopSummary),
        "completions" => schemars::schema_for!(ndjson_types::WriteOutput),
        "recipe" => schemars::schema_for!(crate::commands::recipe::RecipeResult),
        "progress" => schemars::schema_for!(ndjson_types::ProgressEvent),
        "error" => schemars::schema_for!(crate::error::ErrorJson),
        "best-candidate" => schemars::schema_for!(ndjson_types::BestCandidate),
        "cancelled" => schemars::schema_for!(ndjson_types::CancelledEvent),
        "watch" => schemars::schema_for!(ndjson_types::WatchSummary),
        "semantic-merge" | "sparse" | "agent-surface" | "codemod" | "semantic-search"
        | "doctor" | "locale" | "commands" => {
            schemars::schema_for!(ndjson_types::ProgressEvent)
        }
        _ => return Ok(false),
    };
    serde_json::to_writer_pretty(&mut out, &schema)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(true)
}
