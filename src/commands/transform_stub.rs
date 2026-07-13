// SPDX-License-Identifier: MIT OR Apache-2.0
//! Stub when feature `ast` is disabled.
use crate::cli::GlobalArgs;
use crate::cli_args::TransformArgs;
use crate::config::DefaultsSection;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;
use anyhow::Result;
use std::io::Write;

/// AST transform requires feature `ast`.
pub fn cmd_transform(
    _args: &TransformArgs,
    _global: &GlobalArgs,
    _writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
    _defaults: &DefaultsSection,
) -> Result<()> {
    Err(AtomwriteError::ConfigInvalid {
        reason: "transform requires --features ast (included in default features)".into(),
    }
    .into())
}
