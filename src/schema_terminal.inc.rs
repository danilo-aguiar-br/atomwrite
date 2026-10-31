// SPDX-License-Identifier: MIT OR Apache-2.0
// Closed schemas for terminal events (summary verbs, `diff`, `config`).
// Included from schema_dispatch.inc.rs.

/// Schema names whose terminal event is a closed contract.
///
/// Every key such an event can carry is declared, `additionalProperties` is
/// `false`, and `required` lists the keys that are always present, so the
/// schema rejects an object the verb could never emit instead of validating
/// anything.
const CLOSED_TERMINAL_SCHEMAS: &[&str] = &[
    "search-terminal",
    "replace-terminal",
    "count",
    "count-by-ext",
    "count-by-size",
    "backup-terminal",
    "list-terminal",
    "config",
    "batch-terminal",
    "doctor",
    "extract",
];

/// Keys a shared summary type declares but the verb never emits.
fn never_emitted(name: &str) -> &'static [&'static str] {
    match name {
        "search-terminal" => &[
            "files_modified",
            "total_replacements",
            "files_read",
            "total_bytes",
            "binary_rejects",
        ],
        "replace-terminal" => &["total_matches", "files_read", "offset_skipped", "total_bytes"],
        _ => &[],
    }
}

/// Keys the verb always emits although the shared summary type makes them optional.
fn always_emitted(name: &str) -> &'static [&'static str] {
    match name {
        "search-terminal" => &["total_matches", "offset_skipped"],
        "replace-terminal" => &[
            "files_skipped",
            "total_replacements",
            "files_modified",
            "binary_rejects",
        ],
        "list-terminal" => &["total_bytes"],
        "count-by-ext" | "count-by-size" => &["items_omitted"],
        _ => &[],
    }
}

/// Keys of a failed terminal event: the verdict and the failure detail that
/// `NdjsonWriter::publish_failure` stamps.
const FAILURE_REQUIRED: &[&str] = &[
    "code",
    "exit",
    "error_class",
    "retryable",
    "suggestion_lang",
    "message",
];

/// Whether the verb ends a failed run with its own summary (`publish_failure`).
///
/// Such a summary says `ok: false` only when the run failed, so a failed
/// summary must carry the whole failure detail.
fn ends_failure_with_summary(name: &str) -> bool {
    matches!(
        name,
        "search-terminal" | "replace-terminal" | "batch-terminal" | "backup-terminal" | "prune-backups"
    )
}

/// Whether the terminal event can carry the failure detail `publish_failure` stamps.
///
/// `doctor` is the exception to the rule above: its report says `ok: false`
/// for a failing check without failing the run, and carries the failure
/// detail only under `--strict`, so the detail is optional there.
fn can_carry_failure_detail(name: &str) -> bool {
    ends_failure_with_summary(name) || name == "doctor"
}

/// Add `key` to the schema's `required` list when it is not there yet.
fn require_key(root: &mut serde_json::Map<String, serde_json::Value>, key: &str) {
    let required = root
        .entry("required")
        .or_insert_with(|| serde_json::json!([]));
    if let Some(list) = required.as_array_mut()
        && !list.iter().any(|k| k == key)
    {
        list.push(serde_json::Value::String(key.to_string()));
    }
}

/// Failure-detail properties and their definitions, read off the error envelope type.
fn failure_detail_schema() -> (
    serde_json::Map<String, serde_json::Value>,
    serde_json::Map<String, serde_json::Value>,
) {
    let schema = schemars::schema_for!(crate::error::ErrorJson);
    let root = schema.as_object();
    let pick = |key: &str| {
        root.and_then(|r| r.get(key))
            .and_then(serde_json::Value::as_object)
            .cloned()
            .unwrap_or_default()
    };
    let mut properties = pick("properties");
    for stamped in ["error", "ok", "code", "exit"] {
        properties.remove(stamped);
    }
    (properties, pick("$defs"))
}

/// Declare the failure detail `publish_failure` stamps (and the definitions it
/// refers to) on an object schema, without overwriting a key it already has.
fn add_failure_detail(root: &mut serde_json::Map<String, serde_json::Value>) {
    let (detail, defs) = failure_detail_schema();
    if let Some(properties) = root
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
    {
        for (key, definition) in detail {
            properties.entry(key).or_insert(definition);
        }
    }
    let known = root.entry("$defs").or_insert_with(|| serde_json::json!({}));
    if let Some(known) = known.as_object_mut() {
        for (key, definition) in defs {
            known.entry(key).or_insert(definition);
        }
    }
}

/// Schema of everything `extract` writes.
///
/// `extract` is a stream of records, and which record it writes depends on the
/// input line and on the field flags:
///
/// - an NDJSON object with fields selected is narrowed to those keys (the
///   keys are the caller's, so the object stays open);
/// - an NDJSON object with no field selected is passed through whole (also
///   open: the input decides the keys);
/// - a text line with no field selected becomes `{type: "text", fields: [...]}`;
/// - a text line with fields selected becomes `{type: "text", values: [...]}`.
///
/// The union is the contract. Serving one branch alone (the schema used to
/// serve `calc`'s) described an event `extract` never writes.
fn extract_output_schema() -> Option<schemars::Schema> {
    let fields = serde_json::Value::from(schemars::schema_for!(
        ndjson_types::TextFieldsOutput<'static>
    ));
    let values = serde_json::Value::from(schemars::schema_for!(
        ndjson_types::TextValuesOutput<'static>
    ));
    // The two passthrough records leave every key to the input line, which is
    // exactly what `{"type": "object"}` says; the description names the case so
    // the schema is readable on its own.
    let json_narrowed = serde_json::json!({
        "type": "object",
        "description": "An NDJSON input object narrowed to the selected fields.",
        "additionalProperties": true,
    });
    let json_whole = serde_json::json!({
        "type": "object",
        "description": "An NDJSON input object passed through whole (no field selected).",
        "additionalProperties": true,
    });
    union_of(
        "ExtractOutput",
        "Records of `atomwrite extract`: one per input line, narrowed or whole for NDJSON, one of two shapes for text.",
        vec![json_narrowed, json_whole, fields, values],
    )
}

/// Schema of everything `scope` writes: one `result` record per rewritten
/// file, then the `summary`.
///
/// The summary is the terminal event, so it carries the keys the writer
/// stamps and, when the run failed, the failure detail. Without the `ast`
/// feature the verb fails closed and only the per-file record is described.
fn scope_output_schema() -> Option<schemars::Schema> {
    #[cfg(feature = "ast")]
    {
        union_of(
            "ScopeOutput",
            "Records of `atomwrite scope`: one `result` per rewritten file, then the `summary`.",
            vec![
                serde_json::Value::from(schemars::schema_for!(ndjson_types::ScopeResult)),
                terminal_record(
                    schemars::schema_for!(crate::commands::scope::ScopeSummary),
                    &[],
                ),
            ],
        )
    }
    #[cfg(not(feature = "ast"))]
    {
        Some(schemars::schema_for!(ndjson_types::ScopeResult))
    }
}

/// Schema of everything a list verb writes: one record per item, then the
/// terminal summary.
///
/// `hash`, `verify` and `copy` publish a record per file and always close with a
/// summary, so serving the record alone described a stream a consumer could not
/// read the verdict off. The union is the contract, the same shape `extract` and
/// `scope` already use.
fn record_with_summary_schema(
    record: schemars::Schema,
    title: &str,
    description: &str,
) -> Option<schemars::Schema> {
    record_with_own_summary(
        record,
        schemars::schema_for!(ndjson_types::FailureSummary),
        title,
        description,
    )
}

/// Same union for a verb whose summary is its own type (`backup`,
/// `prune-backups`), which still writes a record per item before the terminal.
fn record_with_own_summary(
    record: schemars::Schema,
    summary: schemars::Schema,
    title: &str,
    description: &str,
) -> Option<schemars::Schema> {
    union_of(
        title,
        description,
        vec![
            serde_json::Value::from(record),
            terminal_record(summary, &[]),
        ],
    )
}

/// The `step` tag `recipe` adds to every record it reemits from a child.
fn step_property() -> serde_json::Value {
    serde_json::json!({
        "type": "integer",
        "minimum": 1,
        "description": "1-based recipe step that emitted this record",
    })
}

/// The `rule_id` tag `codemod` adds to every record it reemits from a child.
fn rule_property() -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "description": "Rule the record was written for",
    })
}

/// One record shape of a union, with the tags the parent verb adds to it.
fn tagged_record(
    schema: schemars::Schema,
    tags: &[(&str, serde_json::Value)],
) -> serde_json::Value {
    let mut value = serde_json::Value::from(schema);
    if let Some(obj) = value.as_object_mut()
        && let Some(properties) = obj
            .get_mut("properties")
            .and_then(serde_json::Value::as_object_mut)
    {
        for (key, definition) in tags {
            properties
                .entry((*key).to_string())
                .or_insert_with(|| definition.clone());
        }
    }
    value
}

/// The terminal record of a union: the type's own keys plus every key the
/// writer stamps on the last line, and the failure detail when the run failed.
fn terminal_record(
    schema: schemars::Schema,
    tags: &[(&str, serde_json::Value)],
) -> serde_json::Value {
    let mut value = tagged_record(schema, tags);
    if let Some(obj) = value.as_object_mut() {
        if let Some(properties) = obj
            .get_mut("properties")
            .and_then(serde_json::Value::as_object_mut)
        {
            for (key, definition) in writer_stamped_properties() {
                properties.entry(key).or_insert(definition);
            }
        }
        add_failure_detail(obj);
    }
    value
}

/// One terminal branch closed on its own, with the verdict keys required.
///
/// The writer stamps the verdict and the resolved target on the last line, and
/// the union root no longer carries them, so the branch adds them before it
/// closes, exactly as the root used to.
fn closed_branch(name: &str, schema: schemars::Schema) -> serde_json::Value {
    serde_json::Value::from(close_terminal_schema(name, with_envelope_keys(schema)))
}

/// The failure envelope, which any verb may publish in place of its stream.
fn error_branch() -> serde_json::Value {
    let mut value =
        serde_json::Value::from(with_envelope_keys(schemars::schema_for!(crate::error::ErrorJson)));
    if let Some(obj) = value.as_object_mut() {
        obj.remove("$schema");
    }
    value
}

/// Build the union of every record a verb writes.
///
/// One name describes one verb, so the records before the terminal, the
/// terminal and the failure envelope live in the same document; a consumer
/// validating the stream needs one schema, not one per record shape.
fn union_of(
    title: &str,
    description: &str,
    mut branches: Vec<serde_json::Value>,
) -> Option<schemars::Schema> {
    branches.push(error_branch());
    let mut defs = serde_json::Map::new();
    for branch in &mut branches {
        let Some(obj) = branch.as_object_mut() else {
            continue;
        };
        obj.remove("$schema");
        if let Some(serde_json::Value::Object(found)) = obj.remove("$defs") {
            defs.extend(found);
        }
    }
    let union = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": title,
        "description": description,
        "type": "object",
        "anyOf": branches,
        "$defs": defs,
    });
    schemars::Schema::try_from(union).ok()
}

/// Everything `recipe` writes: the listing row, every record of each child step
/// tagged with `step`, the step records and the terminal result.
fn recipe_output_schema() -> Option<schemars::Schema> {
    let step = [("step", step_property())];
    union_of(
        "RecipeOutput",
        "Records of `atomwrite recipe`: the recipe listing, every record of each step tagged with `step`, and the terminal result.",
        vec![
            tagged_record(schemars::schema_for!(ndjson_types::SearchBegin), &step),
            tagged_record(schemars::schema_for!(ndjson_types::SearchMatch), &step),
            tagged_record(schemars::schema_for!(ndjson_types::SearchEnd), &step),
            tagged_record(schemars::schema_for!(ndjson_types::DryRunPlan), &step),
            tagged_record(schemars::schema_for!(ndjson_types::ReplaceResult), &step),
            tagged_record(schemars::schema_for!(ndjson_types::ProgressEvent), &step),
            tagged_record(schemars::schema_for!(ndjson_types::HashOutput), &step),
            tagged_record(schemars::schema_for!(ndjson_types::CancelledEvent), &step),
            serde_json::Value::from(schemars::schema_for!(ndjson_types::RecipeListEvent)),
            serde_json::Value::from(schemars::schema_for!(ndjson_types::RecipeStepEvent)),
            terminal_record(
                schemars::schema_for!(crate::commands::recipe::RecipeResult),
                &[],
            ),
        ],
    )
}

/// Everything `codemod` writes: the start event, every record of each rule
/// tagged with `rule_id`, and the terminal summary the run really ends with.
fn codemod_output_schema() -> Option<schemars::Schema> {
    let rule = [("rule_id", rule_property())];
    union_of(
        "CodemodOutput",
        "Records of `atomwrite codemod`: the start event, every record of each rule tagged with `rule_id`, and the terminal summary.",
        vec![
            serde_json::Value::from(schemars::schema_for!(ndjson_types::CodemodStartEvent)),
            tagged_record(schemars::schema_for!(ndjson_types::TransformRuleBegin), &rule),
            tagged_record(schemars::schema_for!(ndjson_types::TransformRuleError), &rule),
            tagged_record(schemars::schema_for!(ndjson_types::TransformResult), &rule),
            tagged_record(schemars::schema_for!(ndjson_types::CancelledEvent), &rule),
            terminal_record(
                schemars::schema_for!(ndjson_types::CodemodSummaryEvent),
                &[],
            ),
            terminal_record(schemars::schema_for!(ndjson_types::Summary), &rule),
        ],
    )
}

/// The JSON Schema document `batch --input-schema` prints instead of a stream.
fn manifest_schema_branch() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "description": "The JSON Schema document `batch --input-schema` prints instead of a stream.",
        "required": ["$schema", "title", "properties"],
        "properties": {
            "$schema": {"type": "string"},
            "title": {"type": "string"},
            "type": {"type": "string"},
            "description": {"type": "string"},
            "properties": {"type": "object"},
            "required": {"type": "array"},
            "additionalProperties": {"type": "boolean"},
        },
    })
}

/// Everything `batch` writes, plus the manifest schema `--input-schema` answers.
fn batch_output_schema() -> Option<schemars::Schema> {
    union_of(
        "BatchOutput",
        "Records of `atomwrite batch`: one result per operation, progress, the rollback event and the terminal summary; `--input-schema` answers with the manifest schema instead.",
        vec![
            serde_json::Value::from(schemars::schema_for!(ndjson_types::BatchOpResult<'static>)),
            serde_json::Value::from(schemars::schema_for!(ndjson_types::ProgressEvent)),
            terminal_record(
                schemars::schema_for!(crate::commands::batch::RollbackEvent),
                &[],
            ),
            closed_branch(
                "batch-terminal",
                schemars::schema_for!(ndjson_types::BatchSummary),
            ),
            manifest_schema_branch(),
        ],
    )
}

/// Everything `search` writes: begin, context, end, file and match records,
/// then the terminal summary.
fn search_output_schema() -> Option<schemars::Schema> {
    union_of(
        "SearchOutput",
        "Records of `atomwrite search`: `begin`, `context`, `file`, `match` and `end` records, then the terminal summary.",
        vec![
            serde_json::Value::from(schemars::schema_for!(ndjson_types::SearchBegin)),
            serde_json::Value::from(schemars::schema_for!(ndjson_types::SearchContext)),
            serde_json::Value::from(schemars::schema_for!(ndjson_types::SearchEnd)),
            serde_json::Value::from(schemars::schema_for!(ndjson_types::SearchFile)),
            serde_json::Value::from(schemars::schema_for!(ndjson_types::SearchMatch)),
            closed_branch(
                "search-terminal",
                schemars::schema_for!(ndjson_types::FailureSummary),
            ),
        ],
    )
}

/// Everything `replace` writes: the dry-run plan or the per-file result, then
/// the terminal summary.
fn replace_output_schema() -> Option<schemars::Schema> {
    union_of(
        "ReplaceOutput",
        "Records of `atomwrite replace`: a `plan` record in `--dry-run`, one `replace` record per file, then the terminal summary.",
        vec![
            serde_json::Value::from(schemars::schema_for!(ndjson_types::DryRunPlan)),
            serde_json::Value::from(schemars::schema_for!(ndjson_types::ReplaceResult)),
            closed_branch(
                "replace-terminal",
                schemars::schema_for!(ndjson_types::FailureSummary),
            ),
        ],
    )
}

/// Everything `list` writes: one entry per item, then the terminal summary.
fn list_output_schema() -> Option<schemars::Schema> {
    union_of(
        "ListOutput",
        "Records of `atomwrite list`: one `entry` per item, then the terminal summary.",
        vec![
            serde_json::Value::from(schemars::schema_for!(ndjson_types::ListEntry)),
            closed_branch(
                "list-terminal",
                schemars::schema_for!(ndjson_types::ListSummary),
            ),
        ],
    )
}

/// Everything `backup` writes: one record per source, then the terminal
/// summary.
fn backup_output_schema() -> Option<schemars::Schema> {
    union_of(
        "BackupOutput",
        "Records of `atomwrite backup`: one `backup` or `plan` record per source, then the terminal summary.",
        vec![
            serde_json::Value::from(schemars::schema_for!(ndjson_types::BackupResult)),
            closed_branch(
                "backup-terminal",
                schemars::schema_for!(ndjson_types::BackupSummary),
            ),
        ],
    )
}

/// Everything `semantic-search` writes: one match per hit, then the summary.
fn semantic_search_output_schema() -> Option<schemars::Schema> {
    union_of(
        "SemanticSearchOutput",
        "Records of `atomwrite semantic-search`: one `semantic_match` per hit, then the terminal summary.",
        vec![
            serde_json::Value::from(schemars::schema_for!(ndjson_types::SemanticMatchEvent)),
            terminal_record(
                schemars::schema_for!(ndjson_types::SemanticSummaryEvent),
                &[],
            ),
        ],
    )
}

/// Everything `sparse` writes: the entry listing, the read records, then the
/// summary.
fn sparse_output_schema() -> Option<schemars::Schema> {
    union_of(
        "SparseOutput",
        "Records of `atomwrite sparse`: `sparse_entry` records on the listing, `sparse_read` records on a read, then the terminal summary.",
        vec![
            serde_json::Value::from(schemars::schema_for!(crate::commands::sparse::SparseEntry)),
            serde_json::Value::from(schemars::schema_for!(ndjson_types::SparseReadEvent)),
            terminal_record(
                schemars::schema_for!(crate::commands::sparse::SparseSummary),
                &[],
            ),
        ],
    )
}

/// Everything `semantic-merge` writes: the dry-run plan, then the result.
fn semantic_merge_output_schema() -> Option<schemars::Schema> {
    union_of(
        "SemanticMergeOutput",
        "Records of `atomwrite semantic-merge`: the `plan` record in `--dry-run`, then the terminal result.",
        vec![
            serde_json::Value::from(schemars::schema_for!(ndjson_types::DryRunPlan)),
            terminal_record(
                schemars::schema_for!(crate::commands::semantic_merge::SemanticMergeResult),
                &[],
            ),
        ],
    )
}

/// Everything `case` writes: one `case` or `case_preview` record per file,
/// then the summary.
fn case_output_schema() -> Option<schemars::Schema> {
    union_of(
        "CaseOutput",
        "Records of `atomwrite case`: one `case` or `case_preview` record per file, then the terminal summary.",
        vec![
            serde_json::Value::from(schemars::schema_for!(crate::commands::case::CaseResult)),
            terminal_record(
                schemars::schema_for!(crate::commands::case::CaseSummary),
                &[],
            ),
        ],
    )
}

/// Everything `watch` writes: one event per change, then the summary.
fn watch_output_schema() -> Option<schemars::Schema> {
    union_of(
        "WatchOutput",
        "Records of `atomwrite watch`: one `watch` event per change, then the terminal summary.",
        vec![
            serde_json::Value::from(schemars::schema_for!(ndjson_types::WatchEvent)),
            terminal_record(schemars::schema_for!(ndjson_types::WatchSummary), &[]),
        ],
    )
}

/// Terminal event of `completions --install`.
///
/// Without `--install` the command writes the shell script as plain text
/// (`stdout_kind: shell-script` on the agent surface) and emits no NDJSON, so
/// the installed-path event is the only record this schema can describe.
#[derive(serde::Serialize, schemars::JsonSchema)]
pub struct CompletionsInstalled {
    /// Event type discriminator, always `installed`.
    pub r#type: &'static str,
    /// Absolute path of the completion file that was written.
    pub path: String,
}

/// Close the schema of a terminal event; other schemas pass through untouched.
fn close_terminal_schema(name: &str, mut schema: schemars::Schema) -> schemars::Schema {
    if name == "diff" {
        return close_diff_schema(schema);
    }
    // `extract` writes a stream of records, not a terminal event: the caller's
    // own keys decide the shape of a passthrough line, so the envelope keys
    // would describe keys the verb never writes and a closed root would reject
    // every line of a plain object.
    if name == "extract" {
        return schema;
    }
    // A union root carries no `properties` of its own, so closing it would put
    // `additionalProperties: false` beside the `anyOf` and reject every record.
    // Each union closes its own terminal branch instead.
    if schema
        .as_object()
        .is_some_and(|root| root.contains_key("anyOf") || root.contains_key("oneOf"))
    {
        return schema;
    }
    if !CLOSED_TERMINAL_SCHEMAS.contains(&name) {
        return schema;
    }
    let Some(root) = schema.as_object_mut() else {
        return schema;
    };
    if let Some(properties) = root
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
    {
        for key in never_emitted(name) {
            properties.remove(*key);
        }
    }
    // The two verbs share one summary type; each schema carries its own name.
    match name {
        "search-terminal" => {
            root.insert("title".to_string(), "SearchSummary".into());
        }
        "replace-terminal" => {
            root.insert("title".to_string(), "ReplaceSummary".into());
        }
        _ => {}
    }
    for key in always_emitted(name) {
        require_key(root, key);
    }
    require_key(root, crate::output::RESULT_KEY);
    if can_carry_failure_detail(name) {
        add_failure_detail(root);
    }
    if ends_failure_with_summary(name) {
        root.insert(
            "if".to_string(),
            serde_json::json!({"properties": {"ok": {"const": false}}, "required": ["ok"]}),
        );
        root.insert(
            "then".to_string(),
            serde_json::json!({"required": FAILURE_REQUIRED}),
        );
    }
    root.insert(
        "additionalProperties".to_string(),
        serde_json::Value::Bool(false),
    );
    schema
}

/// Close each record shape of `diff`.
///
/// The schema is a union of five inlined shapes: the four records the verb
/// writes and the failure envelope, which any run may publish instead. The two
/// stream-level shapes
/// that end the run (`summary`, `diff`) carry the verdict stamped by the
/// writer; the `change` record does not.
fn close_diff_schema(mut schema: schemars::Schema) -> schemars::Schema {
    let stamped = writer_stamped_properties();
    let Some(branches) = schema
        .as_object_mut()
        .and_then(|root| root.get_mut("anyOf"))
        .and_then(serde_json::Value::as_array_mut)
    else {
        return schema;
    };
    for branch in branches {
        let Some(obj) = branch.as_object_mut() else {
            continue;
        };
        let terminal = obj
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|p| p.contains_key("elapsed_ms"));
        if terminal {
            if let Some(properties) = obj
                .get_mut("properties")
                .and_then(serde_json::Value::as_object_mut)
            {
                for (key, definition) in &stamped {
                    properties
                        .entry((*key).to_string())
                        .or_insert(definition.clone());
                }
            }
            require_key(obj, crate::output::RESULT_KEY);
        }
        obj.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
    }
    // Every record matches exactly one shape (the shapes differ by `type` and by
    // closed key set), so the union is a `oneOf` under an object root.
    if let Some(root) = schema.as_object_mut()
        && let Some(serde_json::Value::Array(mut branches)) = root.remove("anyOf")
    {
        let mut error = error_branch();
        if let Some(obj) = error.as_object_mut()
            && let Some(serde_json::Value::Object(found)) = obj.remove("$defs")
        {
            let defs = root
                .entry("$defs")
                .or_insert_with(|| serde_json::json!({}));
            if let Some(map) = defs.as_object_mut() {
                map.extend(found);
            }
        }
        branches.push(error);
        root.insert("type".to_string(), "object".into());
        root.insert("oneOf".to_string(), serde_json::Value::Array(branches));
    }
    schema
}
