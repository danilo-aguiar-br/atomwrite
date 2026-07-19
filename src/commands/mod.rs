// SPDX-License-Identifier: MIT OR Apache-2.0

//! Subcommand handler implementations for all atomwrite operations.

use std::io::Read as _;
use std::sync::LazyLock;

use regex::Regex;

/// Matches atomwrite backup filenames like `foo.txt.bak.20260615_035515`.
///
/// Shared process-wide `LazyLock` (not `const`): regex compilation is one-shot
/// and every call site must share the same compiled automaton. Used by
/// `list` and `count` to bucket backups under a dedicated extension key
/// rather than treating the timestamp suffix as a real extension.
pub(crate) static BACKUP_FILENAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.bak\.\d{8}_\d{6}(_\d{3})?$").expect("valid backup regex")
});

/// True when `path` looks like an atomwrite timestamped backup (`*.bak.*`).
///
/// DRY helper for recipe / semantic-search / hash excludes (A-008b / O-007).
pub(crate) fn is_backup_path(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.contains(".bak.") || BACKUP_FILENAME_RE.is_match(name))
}

/// A-027: owned exclude globs for walks that skip atomwrite backups.
#[must_use]
pub(crate) fn backup_exclude_globs() -> Vec<String> {
    crate::constants::BACKUP_EXCLUDE_GLOBS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// A-027: append backup exclude globs to an existing exclude list (idempotent-ish).
pub(crate) fn push_backup_excludes(exclude: &mut Vec<String>) {
    for g in crate::constants::BACKUP_EXCLUDE_GLOBS {
        if !exclude.iter().any(|e| e == *g) {
            exclude.push((*g).to_string());
        }
    }
}

/// v0.1.29
pub mod agent_surface;
/// Full clap command tree as JSON (`atomwrite commands`).
pub mod command_tree;
/// Environment diagnostic for agent hosts (`atomwrite doctor`).
pub mod doctor;
/// Patch application from stdin (unified diff, SEARCH/REPLACE, full file).
pub mod apply;
/// Standalone file backup with BLAKE3 checksums.
pub mod backup;
/// Batch operation execution from NDJSON manifest.
pub mod batch;
/// Math expression evaluation via fend.
pub mod calc;
/// v14 Tier 3: identifier case conversion (snake/camel/Pascal/kebab/SCREAMING).
pub mod case;
pub mod codemod;
/// Atomic file copy with checksum verification.
pub mod copy;
/// Line, match, and extension counting.
pub mod count;
/// v14 Tier 3: structured config key removal.
pub mod del;
/// File deletion with optional backup.
pub mod delete;
/// Unified diff between two files.
pub mod diff;
/// Surgical file editing by line or marker.
pub mod edit;
/// v0.1.22 ADR-0039: apply N `old`/`new` pairs from NDJSON stdin in one write.
pub mod edit_loop;
/// Field extraction from NDJSON or text.
pub mod extract;
/// v14 Tier 3: structured config value reader.
pub mod get;
/// BLAKE3 checksum computation for files.
pub mod hash;
/// Directory listing with metadata.
pub mod list;
/// Diagnose / persist UI locale preference (`atomwrite locale`).
pub mod locale_cmd;
/// Atomic file move and rename.
pub mod r#move;
/// v14 Tier 3 (v0.1.12): tree-sitter outline extraction.
#[cfg(feature = "ast")]
#[cfg_attr(docsrs, doc(cfg(feature = "ast")))]
pub mod outline;
#[cfg(not(feature = "ast"))]
#[path = "outline_stub.rs"]
pub mod outline;
/// v0.1.19 G121: workspace-relative path resolution helper for walking commands.
pub mod path_resolution;
/// v0.1.22 ADR-0040: prune `.bak.YYYYMMDD_HHMMSS` backups by age or count.
pub mod prune_backups;
/// v14 Tier 3 (v0.1.12): tree-sitter S-expression query against a file.
#[cfg(feature = "ast")]
#[cfg_attr(docsrs, doc(cfg(feature = "ast")))]
pub mod query;
#[cfg(not(feature = "ast"))]
#[path = "query_stub.rs"]
pub mod query;
/// File reading with metadata and content.
pub mod read;
/// v0.1.29
pub mod recipe;
/// Regex generation from examples via grex.
pub mod regex_gen;
/// Parallel text replacement with atomic writes.
pub mod replace;
/// File restoration from backup.
pub mod rollback;
/// Grammatical scoping with AST-based actions.
#[cfg(feature = "ast")]
#[cfg_attr(docsrs, doc(cfg(feature = "ast")))]
pub mod scope;
#[cfg(not(feature = "ast"))]
#[path = "scope_stub.rs"]
pub mod scope;
/// Parallel file content search via ripgrep.
pub mod search;
/// v0.1.29
pub mod semantic_merge;
pub mod semantic_search;
/// v14 Tier 3: structured config value setter.
pub mod set;
/// v0.1.29
pub mod sparse;
/// Structural AST code search and rewrite.
#[cfg(feature = "ast")]
#[cfg_attr(docsrs, doc(cfg(feature = "ast")))]
pub mod transform;
#[cfg(not(feature = "ast"))]
#[path = "transform_stub.rs"]
pub mod transform;
/// G119 L5 — snapshot of WAL sidecar state (read-only, no I/O side effects).
pub mod wal_stats;
pub mod watch;
/// Atomic file creation and overwrite.
pub mod write;

/// Fully resolved backup configuration for a mutating subcommand invocation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedBackup {
    /// Whether a transactional backup must be created before writing.
    pub backup: bool,
    /// Whether the backup file survives after a successful write.
    pub keep: bool,
    /// Number of backups to retain.
    pub retention: u8,
}

/// Resolve effective backup configuration from CLI args and
/// `.atomwrite.toml` `[defaults]` (G-007: no product env).
///
/// Precedence for `backup`: `--no-backup`/`--backup` \> `.atomwrite.toml`
/// `[defaults].backup` \> `true`.
/// Precedence for `retention`: `--retention` \> `.atomwrite.toml`
/// `[defaults].retention` \> constant default.
pub(crate) fn resolve_backup(
    opts: &crate::cli_args::BackupOpts,
    defaults: &crate::config::DefaultsSection,
) -> ResolvedBackup {
    let backup = if opts.no_backup {
        false
    } else if opts.backup == Some(true) {
        true
    } else {
        defaults.backup
    };

    ResolvedBackup {
        backup,
        keep: opts.keep_backup,
        retention: opts.retention.unwrap_or(defaults.retention),
    }
}

/// Read new content from stdin for a subcommand mode that requires it,
/// guarding against a terminal stdin (which would otherwise hang waiting
/// for interactive input).
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when `stdin_is_tty` is true,
/// or `AtomwriteError::Io` if reading stdin fails.
pub(crate) fn read_stdin_text_guarded(
    stdin: impl std::io::Read,
    max_size: u64,
    stdin_is_tty: bool,
    mode: &str,
) -> Result<String, crate::error::AtomwriteError> {
    if stdin_is_tty {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "--{mode} reads content from stdin but stdin is a terminal; \
                 pipe content (echo 'new text' | atomwrite edit ...) or use --new-file"
            ),
        });
    }
    let mut buf = String::new();
    // Cap is known up-front; fallible reserve turns OOM into a domain error.
    let reserve = usize::try_from(max_size).unwrap_or(usize::MAX);
    buf.try_reserve(reserve).map_err(|e| {
        crate::error::AtomwriteError::InternalError {
            reason: format!("allocation failed for stdin buffer of {reserve} bytes: {e}"),
        }
    })?;
    stdin.take(max_size).read_to_string(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tty_guard_tests {
    use super::read_stdin_text_guarded;
    use crate::error::AtomwriteError;

    #[test]
    fn tty_stdin_rejected_with_invalid_input() {
        let result = read_stdin_text_guarded(std::io::empty(), 1024, true, "after-match");
        match result {
            Err(AtomwriteError::InvalidInput { reason }) => {
                assert!(reason.contains("after-match"));
                assert!(reason.contains("terminal"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn non_tty_stdin_reads_content() {
        let content = read_stdin_text_guarded("hello\n".as_bytes(), 1024, false, "range")
            .expect("should read stdin when not a tty");
        assert_eq!(content, "hello\n");
    }

    #[test]
    fn non_tty_stdin_respects_max_size() {
        let content = read_stdin_text_guarded("abcdef".as_bytes(), 3, false, "between")
            .expect("should read up to max_size");
        assert_eq!(content, "abc");
    }
}
