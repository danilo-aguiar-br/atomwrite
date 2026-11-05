/// A single operation in a batch NDJSON manifest.
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchOp {
    op: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default, alias = "from", alias = "src")]
    source: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    replacement: Option<String>,
    #[serde(default)]
    backup: bool,
    #[serde(default)]
    old: Option<String>,
    #[serde(default)]
    new: Option<String>,
    /// GAP-108: allow move/copy to overwrite existing target
    #[serde(default)]
    force: Option<bool>,
}

impl BatchOp {
    fn resolve_file_path(&self) -> anyhow::Result<&str> {
        self.target
            .as_deref()
            .or(self.path.as_deref())
            .ok_or_else(|| anyhow::anyhow!("operation requires 'target' or 'path' field"))
    }
}

/// NDJSON event emitted when a transaction is rolled back.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub(crate) struct RollbackEvent {
    r#type: &'static str,
    files_restored: u64,
    files_removed: u64,
    total_reverted: u64,
    /// Files the reversal could not put back, in reverse commit order.
    ///
    /// Omitted when every file came back, so an empty list never reads as a
    /// success the run cannot vouch for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    failed_paths: Vec<String>,
    /// Snapshots the failed reversal left on disk, as absolute paths.
    ///
    /// A reversal that stops halfway cannot vouch for the tree, so the copies
    /// stay and this list names them. Omitted when nothing was preserved,
    /// because a successful reversal removes every snapshot it no longer needs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    backups_preserved: Vec<String>,
}

/// Emit the JSON Schema for the batch input manifest format.
///
/// # Errors
///
/// Returns a JSON serialization error when the generated schema cannot be turned into a value.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
pub fn emit_input_schema(writer: &mut NdjsonWriter<impl Write>) -> Result<()> {
    let schema = schemars::schema_for!(BatchOp);
    let schema_value = serde_json::to_value(&schema)?;
    writer.write_event(&schema_value)?;
    Ok(())
}

