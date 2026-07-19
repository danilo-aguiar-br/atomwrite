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
#[derive(Debug, Serialize)]
struct RollbackEvent {
    r#type: &'static str,
    files_restored: u64,
    files_removed: u64,
    total_reverted: u64,
}

/// Emit the JSON Schema for the batch input manifest format.
pub fn emit_input_schema(writer: &mut NdjsonWriter<impl Write>) -> Result<()> {
    let schema = schemars::schema_for!(BatchOp);
    let schema_value = serde_json::to_value(&schema)?;
    writer.write_event(&schema_value)?;
    Ok(())
}

