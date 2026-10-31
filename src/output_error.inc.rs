// SPDX-License-Identifier: MIT OR Apache-2.0

// Error-envelope emitters, split out of `output.rs` so the writer stays below
// the 500-line ceiling.

/// Write a structured error as NDJSON directly to a raw writer.
///
/// # Errors
///
/// Returns an I/O error if writing the error JSON to the underlying writer fails.
#[cold]
pub fn write_error_json(
    out: &mut impl Write,
    err: &AtomwriteError,
    path: Option<&Path>,
) -> anyhow::Result<()> {
    write_error_json_with_context(out, err, path, &ErrorContext::default())
}

/// Write a structured error as NDJSON directly to a raw writer, with context.
///
/// # Errors
///
/// Returns an I/O error if writing the error JSON to the underlying writer fails.
#[cold]
pub fn write_error_json_with_context(
    out: &mut impl Write,
    err: &AtomwriteError,
    path: Option<&Path>,
    ctx: &ErrorContext,
) -> anyhow::Result<()> {
    write_error_json_shaped(out, err, path, ctx, &OutputShape::default())
}

/// Write a structured error applying agent-native optional `--select` projection.
///
/// Used by `main` so failure envelopes stay on the same stdout contract as
/// success events. **`--filter` never drops errors** (agent-native always-emit);
/// **`--limit` does not apply** on this path. Only `--select`/`--fields` may
/// thin top-level keys.
///
/// # Errors
///
/// Returns an I/O error if writing fails.
#[cold]
pub fn write_error_json_shaped(
    out: &mut impl Write,
    err: &AtomwriteError,
    path: Option<&Path>,
    ctx: &ErrorContext,
    shape: &OutputShape,
) -> anyhow::Result<()> {
    write_error_json_shaped_with_target(out, err, path, ctx, shape, None)
}

/// Like [`write_error_json_shaped`], also naming the subject the run did not reach.
///
/// A verb with side effects stamps `target_resolved` and `target_source` on its
/// terminal record. When the run fails before writing one, the failure envelope
/// IS the terminal record, so the same two keys belong on it: otherwise the
/// caller cannot tell which subject was left untouched.
///
/// # Errors
///
/// Returns an I/O error if writing fails.
#[cold]
pub fn write_error_json_shaped_with_target(
    out: &mut impl Write,
    err: &AtomwriteError,
    path: Option<&Path>,
    ctx: &ErrorContext,
    shape: &OutputShape,
    target: Option<(&str, &str)>,
) -> anyhow::Result<()> {
    let mut json = ErrorJson::from_error_with_context(err, ctx);
    if json.path.is_none() {
        json.path = path.map(|p| p.display().to_string());
    }
    // The error channel carries the same credential surface as the success
    // channel, and it reaches stdout through THIS function — not through the
    // writer's `write_error_with_context`. Both branches therefore run the
    // unconditional key floor and the opt-in content redaction, so a credential
    // in a path or a message is scrubbed whether or not a projection flag is
    // set. Before this, the branch without `--select` serialized the envelope
    // straight from the struct, so `--redact` was inert here while the same
    // credential in a success payload was redacted.
    let mut value = serde_json::to_value(&json)?;
    if let Some((resolved, source)) = target {
        stamp_target_keys(&mut value, resolved, source);
    }
    let shaped = shape_error_value(value, shape);
    serde_json::to_writer(&mut *out, &shaped).map_err(io::Error::from)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}
