// SPDX-License-Identifier: MIT OR Apache-2.0
//! Stub when feature `ast` is disabled.
use crate::cli::GlobalArgs;
use crate::cli_args::OutlineArgs;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use anyhow::Result;
use std::io::Write;

/// Outline requires feature `ast`.
///
/// # Errors
///
/// Always returns `AtomwriteError::ConfigInvalid` because this build was compiled without the
/// `ast` feature.
pub fn cmd_outline(
    _args: &OutlineArgs,
    _global: &GlobalArgs,
    _writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    Err(AtomwriteError::ConfigInvalid {
        reason: rust_i18n::t!("feature.ast-required", command = "outline").to_string(),
    }
    .into())
}
