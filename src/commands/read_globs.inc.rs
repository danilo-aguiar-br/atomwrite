// SPDX-License-Identifier: MIT OR Apache-2.0

// Included by `read.rs`. Operand expansion lives here so the read verb keeps
// one responsibility: turning the argv into bytes.

/// Operands of `read` resolved into concrete targets.
///
/// `empty_patterns` keeps the operand that matched nothing, in argv order, so
/// the caller reports it the way `cat` reports a missing file while the other
/// targets still reach stdout.
struct ResolvedTargets {
    paths: Vec<std::path::PathBuf>,
    empty_patterns: Vec<std::path::PathBuf>,
}

/// Whether an operand asks for expansion instead of naming one file.
fn has_glob_meta(text: &str) -> bool {
    text.chars()
        .any(|c| crate::constants::GLOB_METACHARS.contains(&c))
}

/// Split a pattern into the directory to list and the name to match.
///
/// One level is the contract: the shell `*` never descends, and a recursive
/// expansion would hand the caller bytes from a tree it did not name. Only the
/// final component may be a pattern, so the directories are matched literally
/// and a wildcard inside one of them is refused instead of expanded halfway.
fn split_pattern(pattern: &std::path::Path) -> Result<(std::path::PathBuf, String)> {
    if pattern.to_string_lossy().contains(crate::constants::RECURSIVE_GLOB) {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "read: recursive pattern '{}' is not supported; pass one level per operand",
                pattern.display()
            ),
        }
        .into());
    }
    // Only the last component may carry a metacharacter: the directories are
    // matched literally, and a wildcard in one of them would ask to descend
    // more than one level, which this expansion does not do.
    let dir_has_meta = pattern
        .parent()
        .is_some_and(|parent| has_glob_meta(&parent.to_string_lossy()));
    if dir_has_meta {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "read: only the file name may be a pattern and the directories must be literal; '{}' puts a wildcard in a directory",
                pattern.display()
            ),
        }
        .into());
    }
    let dir = match pattern.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => std::path::PathBuf::from(crate::constants::CURRENT_DIR),
    };
    let Some(name) = pattern.file_name().and_then(|n| n.to_str()) else {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "read: pattern '{}' does not end in a valid UTF-8 file name",
                pattern.display()
            ),
        }
        .into());
    };
    Ok((dir, name.to_string()))
}

/// Expand one pattern against a single level of its directory.
///
/// The order is the encoded bytes of each name, which is the `LC_ALL=C` shell
/// order on Unix and a stable order on Windows. Hidden names are skipped
/// because the shell `*` never matches a leading dot.
fn expand_one(
    pattern: &std::path::Path,
    workspace: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>> {
    let (dir, name) = split_pattern(pattern)?;
    // The directory the pattern reaches is part of the request, so it faces the
    // same jail as a named file: a pattern cannot widen the workspace.
    let listed = crate::path_safety::validate_path(&dir, workspace)?;
    let glob = globset::GlobBuilder::new(&name)
        .literal_separator(true)
        .build()
        .map_err(|e| AtomwriteError::InvalidInput {
            reason: format!("read: invalid pattern '{name}': {e}"),
        })?;
    let matcher = glob.compile_matcher();
    let entries = fs::read_dir(&listed)
        .map_err(|e| AtomwriteError::from_io_at(&listed, e))?;
    let mut hits: Vec<std::path::PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| AtomwriteError::from_io_at(&listed, e))?;
        let file_name = entry.file_name();
        let Some(text) = file_name.to_str() else {
            continue;
        };
        if text.starts_with(crate::constants::HIDDEN_FILE_PREFIX) {
            continue;
        }
        if matcher.is_match(std::path::Path::new(text)) {
            hits.push(listed.join(&file_name));
        }
    }
    hits.sort_by(|a, b| {
        a.file_name()
            .map(std::ffi::OsStr::as_encoded_bytes)
            .cmp(&b.file_name().map(std::ffi::OsStr::as_encoded_bytes))
    });
    Ok(hits)
}

/// Express an absolute operand by the part that sits under the workspace.
///
/// An operand outside the workspace keeps its absolute shape, so the jail —
/// not the shape check — is what refuses it, with the code that names the real
/// reason.
fn under_workspace(
    operand: &std::path::Path,
    workspace: &std::path::Path,
) -> std::path::PathBuf {
    match operand.strip_prefix(workspace) {
        Ok(rest) if operand.is_absolute() && !rest.as_os_str().is_empty() => rest.to_path_buf(),
        _ => operand.to_path_buf(),
    }
}

/// Turn the argv operands into the ordered list of files to read.
///
/// An operand without a glob metacharacter keeps its argv position, so
/// `read a.rs b.rs` and `read b.rs a.rs` differ exactly the way `cat` differs.
fn resolve_targets(
    operands: &[std::path::PathBuf],
    workspace: &std::path::Path,
) -> Result<ResolvedTargets> {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    let mut empty_patterns: Vec<std::path::PathBuf> = Vec::new();
    for operand in operands {
        // A metacharacter is judged on the part of the operand under the
        // workspace: the prefix says where the tree is, never how the pattern
        // is shaped.
        let judged = under_workspace(operand, workspace);
        if !has_glob_meta(&judged.to_string_lossy()) {
            paths.push(operand.clone());
            continue;
        }
        // A name with a metacharacter can still be a real file: the shell
        // `cat` opens `relatorio[1].txt` literally. Existence decides the
        // precedence, so an operand that names a file stays literal and keeps
        // its argv position; only a name that is not there is expanded. The
        // jail is the same one a named file faces.
        let exists = crate::path_safety::validate_path(operand, workspace)
            .ok()
            .is_some_and(|path| path.exists());
        if exists {
            paths.push(operand.clone());
            continue;
        }
        let hits = expand_one(&judged, workspace)?;
        if hits.is_empty() {
            empty_patterns.push(operand.clone());
        } else {
            paths.extend(hits);
        }
    }
    Ok(ResolvedTargets {
        paths,
        empty_patterns,
    })
}
