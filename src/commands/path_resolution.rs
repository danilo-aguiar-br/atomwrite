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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

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
            resolve_paths_against_workspace(std::slice::from_ref(&target), workspace)
                .unwrap()[0]
        );
    }

    #[test]
    fn rejects_path_outside_workspace() {
        let dir = tempdir().expect("tempdir");
        let result = resolve_paths_against_workspace(&[PathBuf::from("/etc/passwd")], dir.path());
        assert!(result.is_err(), "out-of-jail path must fail");
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
