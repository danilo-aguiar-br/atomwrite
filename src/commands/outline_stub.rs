// SPDX-License-Identifier: MIT OR Apache-2.0
//! Stub when feature `ast` is disabled.
use crate::cli::GlobalArgs;
use crate::cli_args::OutlineArgs;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use anyhow::Result;
use std::io::Write;

/// Outline requires feature `ast`.
pub fn cmd_outline(
    _args: &OutlineArgs,
    _global: &GlobalArgs,
    _writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    Err(AtomwriteError::ConfigInvalid {
        reason: "outline requires --features ast (included in default features)".into(),
    }
    .into())
}
