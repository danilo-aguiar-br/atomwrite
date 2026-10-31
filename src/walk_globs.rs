// SPDX-License-Identifier: MIT OR Apache-2.0

//! The one place that compiles the `--include` / `--exclude` globs of every verb.
//!
//! A glob that does not compile is input the caller got wrong, so every verb
//! fails the same way: `INVALID_INPUT`, exit 65, one short message. Verbs call
//! these functions instead of building an `OverrideBuilder` or a `TypesBuilder`
//! themselves, so a new verb cannot forget to type the error (an untyped one
//! reaches the user as `INTERNAL_ERROR`, exit 255).

use std::path::Path;

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;

use crate::error::AtomwriteError;

/// Message prefix for an include glob that does not compile.
const INCLUDE_WHAT: &str = "invalid include glob";
/// Message prefix for an exclude glob that does not compile.
const EXCLUDE_WHAT: &str = "invalid exclude glob";
/// Message prefix for a glob set that does not build.
const SET_WHAT: &str = "invalid glob set";

/// Restrict `walker` with include globs (kept) and exclude globs (dropped).
///
/// Uses gitignore-style overrides rooted at `root`. Does nothing when both
/// lists are empty.
///
/// # Errors
///
/// Returns [`AtomwriteError::InvalidInput`] when a glob does not compile.
pub fn add_overrides(
    walker: &mut WalkBuilder,
    root: &Path,
    include: &[String],
    exclude: &[String],
) -> Result<(), AtomwriteError> {
    if include.is_empty() && exclude.is_empty() {
        return Ok(());
    }
    walker.overrides(compile_overrides(root, include, exclude)?);
    Ok(())
}

/// Keep only the files matching one of the include globs (file-type filter).
///
/// Used by the verbs whose `--include` selects by file name, not by path.
///
/// # Errors
///
/// Returns [`AtomwriteError::InvalidInput`] when a glob does not compile.
pub fn add_include_types(
    walker: &mut WalkBuilder,
    include: &[String],
) -> Result<(), AtomwriteError> {
    if include.is_empty() {
        return Ok(());
    }
    let mut types = TypesBuilder::new();
    for glob in include {
        types
            .add_def(&format!("custom:{glob}"))
            .map_err(|e| AtomwriteError::invalid(INCLUDE_WHAT, e))?;
    }
    types.select("custom");
    walker.types(
        types
            .build()
            .map_err(|e| AtomwriteError::invalid(INCLUDE_WHAT, e))?,
    );
    Ok(())
}

/// Check that every glob compiles, for the verbs that match names themselves.
///
/// # Errors
///
/// Returns [`AtomwriteError::InvalidInput`] when a glob does not compile.
///
/// # Examples
///
/// ```
/// use atomwrite::walk_globs::validate;
///
/// assert!(validate(&["*.rs".to_string()], &["target/**".to_string()]).is_ok());
/// assert!(validate(&["[".to_string()], &[]).is_err());
/// ```
pub fn validate(include: &[String], exclude: &[String]) -> Result<(), AtomwriteError> {
    compile_overrides(Path::new("."), include, exclude).map(|_| ())
}

fn compile_overrides(
    root: &Path,
    include: &[String],
    exclude: &[String],
) -> Result<ignore::overrides::Override, AtomwriteError> {
    let mut overrides = OverrideBuilder::new(root);
    for glob in include {
        overrides
            .add(glob)
            .map_err(|e| AtomwriteError::invalid(INCLUDE_WHAT, e))?;
    }
    for glob in exclude {
        overrides
            .add(&format!("!{glob}"))
            .map_err(|e| AtomwriteError::invalid(EXCLUDE_WHAT, e))?;
    }
    overrides
        .build()
        .map_err(|e| AtomwriteError::invalid(SET_WHAT, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn globs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn every_entry_point_rejects_a_glob_that_does_not_compile() {
        let bad = globs(&["["]);
        let mut walker = WalkBuilder::new(".");
        for result in [
            add_overrides(&mut walker, Path::new("."), &bad, &[]),
            add_overrides(&mut walker, Path::new("."), &[], &bad),
            add_include_types(&mut walker, &bad),
            validate(&bad, &[]),
            validate(&[], &bad),
        ] {
            let err = result.expect_err("the glob must be refused");
            assert_eq!(err.error_code(), "INVALID_INPUT", "{err}");
            assert_eq!(err.exit_code(), 65);
            assert!(!err.to_string().contains('\n'), "short message: {err}");
        }
    }

    #[test]
    fn valid_globs_and_empty_lists_pass() {
        let mut walker = WalkBuilder::new(".");
        assert!(add_overrides(&mut walker, Path::new("."), &[], &[]).is_ok());
        assert!(
            add_overrides(
                &mut walker,
                Path::new("."),
                &globs(&["*.rs"]),
                &globs(&["target"])
            )
            .is_ok()
        );
        assert!(add_include_types(&mut walker, &globs(&["*.rs"])).is_ok());
        assert!(add_include_types(&mut walker, &[]).is_ok());
        assert!(validate(&globs(&["*.rs"]), &globs(&["*.bak.*"])).is_ok());
    }
}
