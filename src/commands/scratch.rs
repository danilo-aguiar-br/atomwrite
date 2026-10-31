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
    /// Staging directory to create under the workspace (required; no default).
    ///
    /// The canonical agent location is `.atomwrite/scratch`. It must stay
    /// inside the workspace jail;
    /// absolute paths outside fail closed. A verb that creates directories
    /// never picks its target from ambient state, so omitting it is a usage error.
    #[arg(
        value_hint = ValueHint::DirPath,
        help = "Staging dir to create under the workspace (e.g. .atomwrite/scratch)"
    )]
    pub path: PathBuf,
}

/// NDJSON success event for `scratch init`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ScratchInitEvent {
    /// Event type discriminator: `"scratch_init"`.
    pub r#type: &'static str,
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
    let relative = args.path.clone();
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
        crate::best_effort::best_effort(
            std::fs::set_permissions(
                &resolved,
                std::fs::Permissions::from_mode(crate::constants::DIR_PERMISSIONS),
            ),
            "set mode on scratch directory",
        );
    }
    let rel_display = relative.to_string_lossy().replace('\\', "/");
    writer.write_event(&ScratchInitEvent {
        r#type: "scratch_init",
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
