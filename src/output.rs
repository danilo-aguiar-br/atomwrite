// SPDX-License-Identifier: MIT OR Apache-2.0

//! NDJSON writer utilities for stdout with broken-pipe handling.

use std::io::{self, BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::error::{AtomwriteError, ErrorContext, ErrorJson};

/// Buffered NDJSON writer that flushes after every line.
pub struct NdjsonWriter<W: Write> {
    writer: BufWriter<W>,
}

impl<W: Write> NdjsonWriter<W> {
    /// Create a new NDJSON writer wrapping the given output.
    pub fn new(inner: W) -> Self {
        Self {
            writer: BufWriter::with_capacity(crate::constants::BUF_CAPACITY, inner),
        }
    }

    /// Serialize a value as a single NDJSON line and flush.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if serialization or writing to the underlying writer fails.
    pub fn write_event<T: Serialize>(&mut self, event: &T) -> anyhow::Result<()> {
        match serde_json::to_writer(&mut self.writer, event) {
            Ok(()) => {}
            Err(e) if is_broken_pipe(&e) => {
                return Err(crate::error::AtomwriteError::BrokenPipe.into());
            }
            Err(e) => return Err(e.into()),
        }
        match self.writer.write_all(b"\n") {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                return Err(crate::error::AtomwriteError::BrokenPipe.into());
            }
            Err(e) => return Err(e.into()),
        }
        match self.writer.flush() {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                return Err(crate::error::AtomwriteError::BrokenPipe.into());
            }
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    /// Emit a structured error as a single NDJSON line.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing the error JSON to the underlying writer fails.
    pub fn write_error(&mut self, err: &AtomwriteError, path: Option<&Path>) -> anyhow::Result<()> {
        self.write_error_with_context(err, path, &ErrorContext::default())
    }

    /// Emit a structured error as a single NDJSON line, with diagnostic context.
    ///
    /// Use this overload when the caller knows whether the workspace root was
    /// explicitly provided (e.g. via `--workspace` or `ATOMWRITE_WORKSPACE`).
    /// The context controls the suggestion text for `WorkspaceJail` errors
    /// (GAP 13 fix).
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing the error JSON to the underlying writer fails.
    pub fn write_error_with_context(
        &mut self,
        err: &AtomwriteError,
        path: Option<&Path>,
        ctx: &ErrorContext,
    ) -> anyhow::Result<()> {
        let mut json = ErrorJson::from_error_with_context(err, ctx);
        if json.path.is_none() {
            json.path = path.map(|p| p.display().to_string());
        }
        self.write_event(&json)
    }

    /// Flush the underlying buffer to the output stream.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if flushing the underlying writer fails.
    pub fn flush(&mut self) -> anyhow::Result<()> {
        self.writer.flush().map_err(|e| e.into())
    }
}

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
    let mut json = ErrorJson::from_error_with_context(err, ctx);
    if json.path.is_none() {
        json.path = path.map(|p| p.display().to_string());
    }
    serde_json::to_writer(&mut *out, &json)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// Read a single line from a buffered reader with a per-line size limit.
///
/// Reuses the provided `buf` (cleared before each call). Returns the number
/// of bytes read (0 means EOF). Returns an error if the line exceeds
/// `max_bytes` before a newline is found.
pub fn read_limited_line(
    reader: &mut impl std::io::BufRead,
    buf: &mut String,
    max_bytes: usize,
) -> std::io::Result<usize> {
    buf.clear();
    let n = reader.read_line(buf)?;
    if buf.len() > max_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "NDJSON line exceeds maximum size of {} bytes ({} bytes read)",
                max_bytes,
                buf.len()
            ),
        ));
    }
    Ok(n)
}

fn is_broken_pipe(err: &serde_json::Error) -> bool {
    if let Some(io_err) = err.io_error_kind() {
        return io_err == io::ErrorKind::BrokenPipe;
    }
    false
}
