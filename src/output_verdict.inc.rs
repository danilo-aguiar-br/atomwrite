// SPDX-License-Identifier: MIT OR Apache-2.0
// Success-envelope verdict stamping (`ok`).
// Included from output.rs.

/// Top-level key carrying the operation result in every agent envelope.
///
/// A success envelope publishes `true` and the error envelope publishes
/// `false`, so one key decides between success and remediation across every
/// subcommand. The key is stamped once, by the writer, rather than declared
/// by each output struct: that is what stops a new subcommand from shipping
/// without it. Before v0.1.36 only `doctor` and `scratch` carried the key,
/// each one declared by hand in its own struct.
pub const RESULT_KEY: &str = "ok";

/// Terminal `type` values that carry a whole operation's outcome.
///
/// Only these are "success envelopes": one per invocation, emitted last. The
/// permit is a closed list on purpose, mirroring the error envelope's `error`
/// sentinel, and every entry is a `type` some subcommand really emits (the
/// `cli_v0137_types` test fails when an emitted `type` is in neither this list
/// nor [`STREAM_RECORD_TYPES`]). Stream records (`begin`, `match`, `progress`,
/// `plan`, `preview`, `replaced`, `end`, …) describe intermediate state and
/// MUST NOT claim a success verdict, and a `cancelled` envelope is already an
/// outcome. The `plan` and `preview` records are deliberately absent: one is
/// emitted per file, so a verdict on each would be a per-file claim rather
/// than the operation's outcome, and the terminal `summary` of the same
/// invocation already carries the verdict.
pub const SUCCESS_ENVELOPE_TYPES: &[&str] = &[
    "agent_surface",
    "applied",
    "backup",
    "calc",
    "codemod_summary",
    "commands_tree",
    "config",
    "copied",
    "count",
    "del",
    "diff",
    "doctor_report",
    "edit",
    "get",
    "hash",
    "hash_verify",
    "installed",
    "locale_report",
    "moved",
    "outline_summary",
    "query_summary",
    "read",
    "recipe",
    "recipe_result",
    "regex",
    "result",
    "rollback",
    "scratch_init",
    "semantic_merge",
    "semantic_summary",
    "set",
    "sparse_outline_budget",
    "sparse_summary",
    "summary",
    "wal_heal",
    "wal_stats",
    "watch_summary",
    "write",
];

/// Emitted `type` values that never receive the `ok` verdict from the writer.
///
/// Per-file or per-item stream records, plus the terminal records that carry
/// their own outcome elsewhere (`cancelled`). Together with
/// [`SUCCESS_ENVELOPE_TYPES`] this list is the closed set of record types the
/// product emits.
pub const STREAM_RECORD_TYPES: &[&str] = &[
    "artifact",
    "batch_op",
    "begin",
    "cancelled",
    "case",
    "case_preview",
    "change",
    "codemod",
    "context",
    "deleted",
    "end",
    "entry",
    "error",
    "file",
    "file_match",
    "match",
    "outline_item",
    "plan",
    "preview",
    "progress",
    "pruned",
    "query_kind",
    "query_match",
    "recipe_step",
    "replaced",
    "rule_begin",
    "rule_error",
    "scoped",
    "semantic_match",
    "skipped",
    "sparse_entry",
    "sparse_read",
    "stream_diff",
    "text",
    "tool",
    "transformed",
    "watch",
];

/// Whether `type` is a success envelope that must carry the `ok` verdict.
///
/// Returns `false` for non-object payloads, for values without a string `type`,
/// and for every stream record. A stream record describes intermediate state,
/// not an outcome, and a projected user payload such as the one `extract`
/// re-emits passes through untouched because it carries the user's own keys,
/// not an operation `type`. A `set` envelope is accepted by both `"set"` and
/// its `summary` fallback path because older revisions reused `summary`.
fn is_success_envelope(value: &Value) -> bool {
    let Some(kind) = value.get("type").and_then(Value::as_str) else {
        return false;
    };
    // An error envelope already carries `ok: false`; never overwrite it.
    if value.get("error").and_then(Value::as_bool) == Some(true) {
        return false;
    }
    SUCCESS_ENVELOPE_TYPES.contains(&kind)
}

/// Attach the [`RESULT_KEY`] verdict to a success envelope lacking it.
///
/// An envelope that already carries the key keeps its own value, so the error
/// envelope's `false` and a payload the user projected keep theirs.
fn stamp_ok(value: &mut Value) {
    if !is_success_envelope(value) {
        return;
    }
    if let Some(obj) = value.as_object_mut() {
        obj.entry(RESULT_KEY).or_insert(Value::Bool(true));
    }
}

/// Mark an event as the failed outcome of its run.
///
/// Overwrites `ok` with `false` (the caller decided the run failed, so any
/// earlier `true` would contradict the error envelope that follows) and adds
/// the machine `code` and process `exit` unless the event already carries them.
fn stamp_failed(value: &mut Value, code: &str, exit: u8) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    obj.insert(RESULT_KEY.to_string(), Value::Bool(false));
    obj.entry("code")
        .or_insert_with(|| Value::String(code.to_string()));
    obj.entry("exit").or_insert_with(|| Value::from(exit));
}

/// Display text of [`VerdictPublished`], also used to skip it in a cause chain.
const VERDICT_PUBLISHED_MESSAGE: &str = "verdict already published on the data channel";

/// Marker attached to an error whose verdict is already on the data channel.
///
/// A terminal event that states the failure itself (`hash --verify`, or any
/// summary written through [`NdjsonWriter::publish_failure`]) carries `ok`,
/// `code` and `exit`, so the generic error envelope would state the same fact
/// twice. `main` suppresses the envelope only when this marker is in the error
/// chain; an error raised anywhere else has published nothing and still gets
/// its envelope.
#[derive(Debug, Clone, Copy)]
pub struct VerdictPublished;

impl std::fmt::Display for VerdictPublished {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(VERDICT_PUBLISHED_MESSAGE)
    }
}

/// Human message of an error, without the [`VerdictPublished`] marker.
///
/// `anyhow` shows only the outermost context, so `to_string()` on a published
/// failure would print the marker instead of the cause.
pub fn error_message(err: &anyhow::Error) -> String {
    err.chain()
        .map(ToString::to_string)
        .find(|message| message != VERDICT_PUBLISHED_MESSAGE)
        .unwrap_or_else(|| err.to_string())
}

/// Keys of the error envelope that a terminal event must not copy.
///
/// `ok`, `code` and `exit` are stamped by [`stamp_failed`], and `error` is the
/// sentinel of the standalone error envelope, which this event is not.
const FAILURE_STAMPED_KEYS: [&str; 4] = ["error", "ok", "code", "exit"];

/// Run `read` on the typed error behind `err`.
///
/// A typed [`AtomwriteError`] in the chain is used as is, an I/O error is
/// classified like the binary's top-level handler does, and anything else is
/// the internal error.
fn with_typed_failure<R>(err: &anyhow::Error, read: impl FnOnce(&AtomwriteError) -> R) -> R {
    if let Some(known) = err.downcast_ref::<AtomwriteError>() {
        return read(known);
    }
    let mapped = match crate::runtime::find_io_error(err) {
        Some(kind) => crate::runtime::io_to_atomwrite_error(kind, err),
        None => AtomwriteError::InternalError {
            reason: error_message(err),
        },
    };
    read(&mapped)
}

/// Error class and retry hint of a failure, as the error envelope states them.
pub fn failure_class(err: &anyhow::Error) -> (&'static str, bool) {
    with_typed_failure(err, |known| {
        (known.error_class().as_str(), known.is_retryable())
    })
}

/// Serializable detail (`error_class`, `retryable`, `suggestion`, ...) of a failure.
fn failure_detail(err: &anyhow::Error) -> Value {
    with_typed_failure(err, |known| {
        serde_json::to_value(ErrorJson::from_error(known)).unwrap_or(Value::Null)
    })
}

/// Turn a terminal event into the failed outcome of its run.
///
/// Stamps `ok: false`, `code` and `exit`, then adds the detail an agent needs
/// to branch (`error_class`, `retryable`, `suggestion`, `suggestion_lang`,
/// `conflict_count`, `match_count`, `pairs_*`, ...) without overwriting any key
/// the event already carries.
fn stamp_failure(value: &mut Value, err: &anyhow::Error) {
    let (code, exit) = failure_identity(err);
    stamp_failed(value, code, exit);
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    let Value::Object(detail) = failure_detail(err) else {
        return;
    };
    for (key, field) in detail {
        if !FAILURE_STAMPED_KEYS.contains(&key.as_str()) {
            obj.entry(key).or_insert(field);
        }
    }
}

/// Whether a failed file read was refused for what the file holds, not for access.
///
/// A file that is too large, not valid text or binary is skipped by design by
/// the scanning verbs (`files_skipped`); only a file that could not be opened
/// or read is a failure their summary reports.
pub fn is_content_rejection(err: &anyhow::Error) -> bool {
    matches!(
        err.downcast_ref::<AtomwriteError>(),
        Some(
            AtomwriteError::InvalidInput { .. }
                | AtomwriteError::FileTooLarge { .. }
                | AtomwriteError::BinaryFile { .. }
        )
    )
}

/// Machine code and process exit status of the error that ends a run.
///
/// Feeds [`NdjsonWriter::publish_failure`], which holds the failure as an
/// `anyhow::Error`. The mapping mirrors the binary's top-level
/// handler: a typed [`AtomwriteError`] keeps its own code and status, an I/O
/// error in the chain is classified like `main` classifies it, and anything
/// else is the internal error that exits 1.
pub fn failure_identity(err: &anyhow::Error) -> (&'static str, u8) {
    if let Some(known) = err.downcast_ref::<AtomwriteError>() {
        return (known.error_code(), known.exit_code());
    }
    if let Some(kind) = crate::runtime::find_io_error(err) {
        let mapped = crate::runtime::io_to_atomwrite_error(kind, err);
        return (mapped.error_code(), mapped.exit_code());
    }
    ("INTERNAL_ERROR", 1)
}

/// Whether the event closes a run, as a success, a failure or a cancel.
///
/// These are the only events that name the resolved target: a stream record
/// describes one item and would repeat the same path on every line.
fn is_terminal_envelope(value: &Value) -> bool {
    if value.get("error").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    let Some(kind) = value.get("type").and_then(Value::as_str) else {
        return false;
    };
    kind == "cancelled" || SUCCESS_ENVELOPE_TYPES.contains(&kind)
}

/// Attach `workspace_source` to a terminal envelope (ETD step 2).
///
/// An envelope that already carries the key keeps its own value.
fn stamp_workspace_key(value: &mut Value, source: &str) {
    if !is_terminal_envelope(value) {
        return;
    }
    if let Some(obj) = value.as_object_mut() {
        obj.entry("workspace_source")
            .or_insert_with(|| Value::String(source.to_string()));
    }
}

/// Attach `target_resolved` and `target_source` to a terminal envelope.
///
/// An envelope that already carries either key keeps its own value.
fn stamp_target_keys(value: &mut Value, resolved: &str, source: &str) {
    if !is_terminal_envelope(value) {
        return;
    }
    if let Some(obj) = value.as_object_mut() {
        obj.entry("target_resolved")
            .or_insert_with(|| Value::String(resolved.to_string()));
        obj.entry("target_source")
            .or_insert_with(|| Value::String(source.to_string()));
    }
}
