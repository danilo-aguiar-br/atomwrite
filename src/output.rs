// SPDX-License-Identifier: MIT OR Apache-2.0

//! NDJSON writer utilities for stdout with broken-pipe handling.

use std::io::{self, BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::error::{AtomwriteError, ErrorJson};

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
            Err(e) if is_broken_pipe(&e) => std::process::exit(crate::constants::EXIT_BROKEN_PIPE),
            Err(e) => return Err(e.into()),
        }
        match self.writer.write_all(b"\n") {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                std::process::exit(crate::constants::EXIT_BROKEN_PIPE)
            }
            Err(e) => return Err(e.into()),
        }
        match self.writer.flush() {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                std::process::exit(crate::constants::EXIT_BROKEN_PIPE)
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
        let mut json = ErrorJson::from_error(err);
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
pub fn write_error_json(
    out: &mut impl Write,
    err: &AtomwriteError,
    path: Option<&Path>,
) -> anyhow::Result<()> {
    let mut json = ErrorJson::from_error(err);
    if json.path.is_none() {
        json.path = path.map(|p| p.display().to_string());
    }
    serde_json::to_writer(&mut *out, &json)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

fn is_broken_pipe(err: &serde_json::Error) -> bool {
    if let Some(io_err) = err.io_error_kind() {
        return io_err == io::ErrorKind::BrokenPipe;
    }
    false
}
