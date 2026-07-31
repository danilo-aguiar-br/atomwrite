// SPDX-License-Identifier: MIT OR Apache-2.0

//! Create the canonical agent staging directory under the workspace jail.
//!
//! Workload: I/O-bound (mkdir). Parallelism: none — one-shot path.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand, ValueHint};
use serde::Serialize;

use crate::cli::GlobalArgs;
use crate::constants::SCRATCH_DIR_REL;
use crate::output::NdjsonWriter;

/// `atomwrite scratch` — staging helpers (never opens OS `/tmp`).
#[derive(Args, Debug)]
pub struct ScratchArgs {
    /// Scratch action to run (`init`).
    #[command(subcommand)]
    pub action: ScratchAction,
}

/// Scratch subcommands.
#[derive(Subcommand, Debug)]
pub enum ScratchAction {
    /// Create `{workspace}/.atomwrite/scratch` (idempotent).
    Init(ScratchInitArgs),
}

/// Arguments for `scratch init`.
#[derive(Args, Debug)]
pub struct ScratchInitArgs {
    /// Optional relative path under workspace (default: `.atomwrite/scratch`).
    ///
    /// Must stay inside the workspace jail; absolute paths outside fail closed.
    #[arg(
        long,
        value_hint = ValueHint::DirPath,
        help = "Relative staging dir under workspace (default constants::SCRATCH_DIR_REL)"
    )]
    pub path: Option<PathBuf>,
}

/// NDJSON success event for `scratch init`.
#[derive(Debug, Serialize)]
pub struct ScratchInitEvent {
    /// Always true on success.
    pub ok: bool,
    /// Agent success sentinel.
    pub status: &'static str,
    /// Subcommand name.
    pub command: &'static str,
    /// Action name.
    pub action: &'static str,
    /// Absolute path created.
    pub path: String,
    /// Relative path under workspace.
    pub relative: String,
    /// True when directory already existed.
    pub already_existed: bool,
}

/// Create the staging directory inside the workspace jail.
///
/// # Errors
///
/// Returns `WorkspaceJail` if the resolved path escapes the workspace.
/// Returns `Io` if mkdir fails.
#[tracing::instrument(skip_all, fields(command = "scratch"))]
pub fn cmd_scratch(
    args: &ScratchArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    match &args.action {
        ScratchAction::Init(init) => cmd_scratch_init(init, global, writer),
    }
}

fn cmd_scratch_init(
    args: &ScratchInitArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let workspace = global.resolve_workspace()?;
    let relative = args
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from(SCRATCH_DIR_REL));
    // Force relative staging: absolute paths are validated against jail.
    let candidate = if relative.is_absolute() {
        relative.clone()
    } else {
        workspace.join(&relative)
    };
    let resolved = crate::path_safety::validate_path(&candidate, &workspace)?;
    let already = resolved.is_dir();
    std::fs::create_dir_all(&resolved)
        .map_err(|e| crate::error::AtomwriteError::Io { source: e })?;
    // Best-effort owner-only perms on Unix (no-op on Windows).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            &resolved,
            std::fs::Permissions::from_mode(crate::constants::DIR_PERMISSIONS),
        );
    }
    let rel_display = relative.to_string_lossy().replace('\\', "/");
    writer.write_event(&ScratchInitEvent {
        ok: true,
        status: "ok",
        command: "scratch",
        action: "init",
        path: resolved.display().to_string(),
        relative: rel_display,
        already_existed: already,
    })?;
    Ok(())
}
