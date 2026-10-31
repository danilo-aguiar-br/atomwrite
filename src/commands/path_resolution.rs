// SPDX-License-Identifier: MIT OR Apache-2.0

//! G121 — CWD-independent path resolution for search and replace.
//!
//! Pre-v0.1.19, both `cmd_search` and `cmd_replace` validated caller-supplied
//! root paths against the workspace jail but then handed the ORIGINAL
//! (often CWD-relative) path to `ignore::WalkBuilder::new` and
//! `ignore::overrides::OverrideBuilder::new`. The walk directory engine
//! resolved those paths against the process CWD, not the workspace root.
//! With `CWD != --workspace`, a relative path like `src/` either escaped
//! the workspace silently (if CWD had a `src/` of its own) or pointed at
//! the wrong tree, producing per-file `JailViolation` events for every
//! walked entry — wasteful and violating the G118 invariant that the
//! workspace root (not the CWD) is the path resolution origin.
//!
//! This module centralizes the fix: `resolve_paths_against_workspace`
//! runs `validate_path` on every root, collects the canonical absolute
//! PathBuf, and returns a `Vec<PathBuf>` ready to hand to the walker
//! builders. Both `cmd_search` and `cmd_replace` MUST use this helper
//! before constructing their `WalkBuilder`.
//!
//! v0.1.36 — Explicit Target Designation (ETD). A verb with side effects
//! MUST receive its subject in the invocation line; resolving that subject
//! from ambient state is forbidden. `resolve_mutation_targets` is the
//! fail-closed entry point for those verbs: an empty root list is an error
//! instead of a silent drop to the current directory. The pair it returns
//! (`target_resolved`, `target_source`) is the source of truth for the
//! `target_resolved` and `target_source` fields an execution envelope must
//! carry; this module only resolves them and never emits them.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::error::AtomwriteError;

/// Operand that names standard input instead of a path (POSIX XCU §12.2).
///
/// `atomwrite` never resolves it against the workspace: the subject of the run
/// is the stream the caller piped in, so the envelope names the token itself.
pub const STDIN_TARGET: &str = "-";

/// Origin of a resolved target path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSource {
    /// The subject was named in the invocation line itself.
    Argv,
    /// The subject was inherited from the workspace slot (ambient authority).
    Workspace,
    /// The subject is standard input, named by the `-` operand.
    Stdin,
}

impl TargetSource {
    /// Wire token carried by the `target_source` field of an envelope.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Argv => "argv",
            Self::Workspace => "workspace",
            Self::Stdin => "stdin",
        }
    }
}

/// Authority policy applied when resolving the subject of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMode {
    /// Side-effect verb: the subject MUST come from the invocation line.
    Explicit,
    /// Idempotent read verb: the ambient subject is legal.
    AmbientAllowed,
}

/// A resolved subject path together with its provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    /// Canonical absolute path, already validated against the workspace jail.
    pub target_resolved: PathBuf,
    /// Where the subject came from (`argv` or `workspace`).
    pub target_source: TargetSource,
}

/// Resolve the subject roots of a command, reporting provenance per root.
///
/// An empty `paths` slice means the invocation line carried no subject. Under
/// [`TargetMode::Explicit`] this is fail-closed: the call returns
/// [`AtomwriteError::InvalidInput`] and never falls back to the workspace or
/// the current directory. Under [`TargetMode::AmbientAllowed`] the workspace
/// root becomes the single subject and its source is
/// [`TargetSource::Workspace`].
///
/// Relative roots are joined with `workspace`; absolute roots are checked for
/// jail containment. Input order is preserved, so a one-root verb can publish
/// the first entry as its `target_resolved`.
///
/// # Errors
///
/// Returns [`AtomwriteError::InvalidInput`] when `mode` is
/// [`TargetMode::Explicit`] and `paths` is empty. Returns
/// [`AtomwriteError::WorkspaceJail`] if any root escapes the workspace, and
/// [`AtomwriteError::InvalidInput`] for null bytes or reserved Windows names.
pub fn resolve_mutation_targets(
    paths: &[PathBuf],
    workspace: &Path,
    mode: TargetMode,
) -> Result<Vec<ResolvedTarget>> {
    if paths.is_empty() {
        return match mode {
            TargetMode::Explicit => Err(AtomwriteError::InvalidInput {
                reason: "target required in argv: a verb with side effects never inherits its \
                         subject from the workspace or the current directory; name the target \
                         explicitly (e.g. `src/` or `.`)"
                    .into(),
            }
            .into()),
            TargetMode::AmbientAllowed => {
                let target_resolved = crate::path_safety::validate_path(workspace, workspace)
                    .context("G121: cannot resolve the workspace root as the ambient target")?;
                Ok(vec![ResolvedTarget {
                    target_resolved,
                    target_source: TargetSource::Workspace,
                }])
            }
        };
    }
    resolve_each_root(paths, workspace)
}

/// Validate every named root against the jail, preserving input order.
fn resolve_each_root(paths: &[PathBuf], workspace: &Path) -> Result<Vec<ResolvedTarget>> {
    paths
        .iter()
        .map(|p| {
            let target_resolved =
                crate::path_safety::validate_path(p, workspace).with_context(|| {
                    format!(
                        "G121: path '{}' escapes workspace jail; use --workspace to set a different root",
                        p.display()
                    )
                })?;
            Ok(ResolvedTarget {
                target_resolved,
                target_source: TargetSource::Argv,
            })
        })
        .collect()
}

/// Resolve a list of root paths against the workspace jail, returning
/// the canonical absolute `PathBuf` for each entry.
///
/// Relative inputs are joined with `workspace` and soft-canonicalized;
/// absolute inputs are checked for jail containment. A single violation
/// aborts the entire resolution (G118 invariant: workspace is the path
/// origin, not the CWD).
///
/// # Errors
///
/// Returns `AtomwriteError::WorkspaceJail` if any path escapes the
/// workspace. Returns `AtomwriteError::InvalidInput` for null bytes or
/// reserved Windows filenames.
pub fn resolve_paths_against_workspace(
    paths: &[PathBuf],
    workspace: &Path,
) -> Result<Vec<PathBuf>> {
    paths
        .iter()
        .map(|p| {
            crate::path_safety::validate_path(p, workspace).with_context(|| {
                format!(
                    "G121: path '{}' escapes workspace jail; use --workspace to set a different root",
                    p.display()
                )
            })
        })
        .collect()
}

/// Split resolved roots into the ones that exist and the ones that do not.
///
/// A root named in the invocation that does not exist is a caller mistake, not
/// a scan failure, so the caller reports it before the walk and walks only the
/// rest. Any error other than `NotFound` (a permission denial, for example)
/// keeps the root in the walk, where the walker reports the real cause instead
/// of a made-up "file not found".
#[must_use]
pub fn split_missing_roots(paths: Vec<PathBuf>) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut present = Vec::with_capacity(paths.len());
    let mut missing = Vec::new();
    for path in paths {
        match std::fs::metadata(&path) {
            Ok(_) => present.push(path),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => missing.push(path),
            Err(_) => present.push(path),
        }
    }
    (present, missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolves_relative_path_against_workspace() {
        let dir = tempdir().expect("tempdir");
        let workspace = dir.path();
        let sub = workspace.join("src");
        std::fs::create_dir(&sub).expect("mkdir");
        let resolved =
            resolve_paths_against_workspace(&[PathBuf::from("src")], workspace).expect("resolve");
        assert_eq!(resolved.len(), 1);
        assert!(resolved[0].ends_with("src"));
        assert!(resolved[0].is_absolute());
    }

    #[test]
    fn resolves_absolute_path_inside_workspace() {
        let dir = tempdir().expect("tempdir");
        let workspace = dir.path();
        let target = workspace.join("data");
        std::fs::create_dir(&target).expect("mkdir");
        let resolved = resolve_paths_against_workspace(std::slice::from_ref(&target), workspace)
            .expect("resolve");
        assert_eq!(
            resolved[0],
            resolve_paths_against_workspace(std::slice::from_ref(&target), workspace).unwrap()[0]
        );
    }

    #[test]
    fn rejects_path_outside_workspace() {
        let dir = tempdir().expect("tempdir");
        let result = resolve_paths_against_workspace(&[PathBuf::from("/etc/passwd")], dir.path());
        assert!(result.is_err(), "out-of-jail path must fail");
    }

    #[test]
    fn explicit_mode_rejects_empty_targets() {
        let dir = tempdir().expect("tempdir");
        let err = resolve_mutation_targets(&[], dir.path(), TargetMode::Explicit)
            .expect_err("empty target must fail closed");
        let aw_err = err
            .downcast_ref::<AtomwriteError>()
            .expect("typed AtomwriteError");
        assert_eq!(aw_err.exit_code(), 65);
        let msg = format!("{err:#}");
        assert!(msg.contains("target required in argv"), "unexpected: {msg}");
    }

    #[test]
    fn explicit_mode_keeps_named_target_and_source() {
        let dir = tempdir().expect("tempdir");
        let named = dir.path().join("sub");
        std::fs::create_dir(&named).expect("mkdir");
        let resolved = resolve_mutation_targets(
            std::slice::from_ref(&named),
            dir.path(),
            TargetMode::Explicit,
        )
        .expect("resolve");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target_source, TargetSource::Argv);
        assert_eq!(resolved[0].target_resolved, named);
    }

    #[test]
    fn ambient_mode_uses_workspace_as_target() {
        let dir = tempdir().expect("tempdir");
        let resolved = resolve_mutation_targets(&[], dir.path(), TargetMode::AmbientAllowed)
            .expect("resolve ambient");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target_source, TargetSource::Workspace);
        assert!(resolved[0].target_resolved.is_absolute());
    }

    #[test]
    fn mutation_target_outside_workspace_is_rejected() {
        let dir = tempdir().expect("tempdir");
        let err = resolve_mutation_targets(
            &[PathBuf::from("/etc/passwd")],
            dir.path(),
            TargetMode::Explicit,
        )
        .expect_err("out-of-jail target must fail");
        assert!(err.to_string().contains("G121"), "unexpected: {err:#}");
    }

    #[test]
    fn preserves_input_count() {
        let dir = tempdir().expect("tempdir");
        let p1 = dir.path().join("a");
        let p2 = dir.path().join("b");
        std::fs::create_dir(&p1).expect("mkdir a");
        std::fs::create_dir(&p2).expect("mkdir b");
        let resolved =
            resolve_paths_against_workspace(&[PathBuf::from("a"), PathBuf::from("b")], dir.path())
                .expect("resolve");
        assert_eq!(resolved.len(), 2);
    }
}
