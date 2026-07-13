// SPDX-License-Identifier: MIT OR Apache-2.0
//! Stub when feature `ast` is disabled.
use crate::cli::GlobalArgs;
use crate::cli_args::QueryArgs;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use anyhow::Result;
use std::io::Write;

/// Tree-sitter query requires feature `ast`.
pub fn cmd_query(
    _args: &QueryArgs,
    _global: &GlobalArgs,
    _writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    Err(AtomwriteError::ConfigInvalid {
        reason: "query requires --features ast (included in default features)".into(),
    }
    .into())
}
