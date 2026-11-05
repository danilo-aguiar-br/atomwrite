// SPDX-License-Identifier: MIT OR Apache-2.0

//! Subcommand handler implementations for all atomwrite operations.

use std::io::{BufReader, Read};

use crate::error::AtomwriteError;

/// Backup filename policy, re-exported so callers keep the `commands::` path.
pub mod backup_names;
pub(crate) use backup_names::{
    backup_exclude_globs, is_backup_path, is_backup_timestamp_name, push_backup_excludes,
};

/// v0.1.29
pub mod agent_surface;
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
/// Forward a captured child run into the parent stream (`recipe`, `codemod`).
pub(crate) mod child_reemit;
pub mod codemod;
/// Full clap command tree as JSON (`atomwrite commands`).
pub mod command_tree;
/// Persistent tuning keys: `config path|list|get|set|unset`.
pub mod config_cmd;
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
/// Environment diagnostic for agent hosts (`atomwrite doctor`).
pub mod doctor;
/// Claim readers of the `doctor` document guards.
pub mod doctor_docs;
/// Surgical file editing by line or marker.
pub mod edit;
/// v0.1.22 ADR-0039: apply N `old`/`new` pairs from NDJSON stdin in one write.
pub mod edit_loop;
/// Field extraction from NDJSON or text.
pub mod extract;
/// v0.1.36 E5 — bounded signal extraction per artifact type (PDF, OOXML
/// document/spreadsheet/presentation, image geometry). Never forwards raw bytes.
pub mod extract_artifact;
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
/// Canonical agent staging directory (`atomwrite scratch init`).
pub mod scratch;
/// Parallel file content search via ripgrep.
pub mod search;
/// v0.1.29
pub mod semantic_merge;
pub mod semantic_search;
/// v14 Tier 3: structured config value setter.
pub mod set;

/// Byte-span locator that keeps `set` on JSON from reformatting untouched keys.
pub(crate) mod set_json_span;
/// v0.1.29
pub mod sparse;
/// DRY dry-run / plan summary counters (R-DRY-001).
pub mod summary_metrics;
/// Single dotted-key walk shared by `get`, `set` and `del`.
pub(crate) mod toml_path;
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
/// Shared size guards of every verb that replaces file content (v0.1.37 WP1).
pub mod write_guards;

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
/// `[defaults].backup` \> `true`, so an absent key keeps the rewrite default.
/// Precedence for `retention`: `--retention` \> `.atomwrite.toml`
/// `[defaults].retention` \> constant default.
pub(crate) fn resolve_backup(
    opts: &crate::cli_args::BackupOpts,
    defaults: &crate::config::DefaultsSection,
) -> ResolvedBackup {
    resolve_backup_for(opts, defaults, defaults.backup.unwrap_or(true))
}

/// Resolve the backup decision with a verb-specific default.
///
/// `silent_default` is what the verb does when the flags say nothing. `delete`
/// passes `args.backup == Some(true) || defaults.backup == Some(true)`, so the
/// verb copies only when the call asks for it, through the flag or through the
/// key written in the config: removing the file is the request, and a copy the
/// caller did not ask for keeps the bytes on disk.
pub(crate) fn resolve_backup_for(
    opts: &crate::cli_args::BackupOpts,
    defaults: &crate::config::DefaultsSection,
    silent_default: bool,
) -> ResolvedBackup {
    // clap hands back `Some(false)` for an absent tri-state flag, so only an
    // explicit `Some(true)` is a request: treating `Some(false)` as "no" made
    // every verb ignore `[defaults].backup` and stopped creating backups.
    let backup = if opts.no_backup {
        false
    } else if opts.backup == Some(true) {
        true
    } else {
        silent_default
    };

    ResolvedBackup {
        backup,
        keep: opts.keep_backup,
        retention: opts.retention.unwrap_or(defaults.retention),
    }
}

/// Where the byte cap handed to [`read_stdin_content_cancellable`] came from.
///
/// The origin decides the error a caller sees on overflow: a verb-local
/// `--max-size` is an argument mistake (`InvalidInput`), while the global
/// `--max-filesize` is a size limit and reports `FileTooLarge`, exactly like
/// `read`, `edit` and `hash` do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StdinCapOrigin {
    /// The verb-local `--max-size` flag.
    LocalMaxSize,
    /// The global `--max-filesize` flag (or its default ceiling).
    GlobalMaxFilesize,
}

/// The one stdin reader: 64 KiB chunks, a fallible buffer, the optional cap and
/// cooperative cancellation.
///
/// Every verb that reads a payload from stdin goes through here, so reserve,
/// cap and cancel behave the same and a new verb cannot reimplement them
/// slightly differently.
///
/// # Errors
///
/// Returns the typed error of the cap that was exceeded, `InvalidInput` on an
/// empty payload that did not opt in, the cancellation error when the shutdown
/// signal fires, and `InternalError` when the buffer cannot grow.
pub(crate) fn read_stdin_content_cancellable(
    stdin: impl Read,
    max_size: Option<u64>,
    cap_origin: StdinCapOrigin,
    allow_empty: bool,
    shutdown: Option<&crate::signal::ShutdownSignal>,
) -> anyhow::Result<(Vec<u8>, u64)> {
    let mut reader = BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
    let mut buf = Vec::with_capacity(crate::constants::STDIN_INITIAL_CAPACITY);
    if let Some(max) = max_size {
        let reserve = usize::try_from(max.min(u64::from(u32::MAX))).unwrap_or(usize::MAX);
        // Soft pre-reserve up to the cap so growth is one-shot when the
        // payload is near max_size; failure is non-fatal (we still grow).
        if let Err(e) = buf.try_reserve(reserve.min(
            crate::constants::STDIN_INITIAL_CAPACITY * crate::constants::STDIN_RESERVE_CAP_FACTOR,
        )) {
            tracing::debug!(error = %e, "stdin pre-reserve failed; buffer will grow on demand");
        }
    }
    // A-025: stdin read chunk aligned with BUF_CAPACITY.
    let mut chunk = [0u8; crate::constants::BUF_CAPACITY];
    loop {
        if shutdown.is_some_and(|s| s.is_shutdown()) {
            return Err(
                crate::signal::cancelled_error(crate::signal::current_cancel_reason()).into(),
            );
        }
        // A cancelled read arrives as an `io::Error` wrapping the typed
        // `Cancelled`, and the `context` below would hide it from the
        // `downcast` every caller performs. `surface_cancel` restores the
        // typed error so a deadline exits 124 instead of 255.
        let n = reader.read(&mut chunk).map_err(|e| {
            crate::stdin_cancel::surface_cancel(
                anyhow::Error::new(e).context("failed to read stdin"),
            )
        })?;
        if n == 0 {
            break;
        }
        buf.try_reserve(n)
            .map_err(|e| AtomwriteError::InternalError {
                reason: format!("allocation failed while reading stdin (+{n} bytes): {e}"),
            })?;
        buf.extend_from_slice(&chunk[..n]);
        if let Some(max) = max_size {
            if buf.len() as u64 > max {
                return Err(match cap_origin {
                    StdinCapOrigin::LocalMaxSize => AtomwriteError::InvalidInput {
                        reason: format!(
                            "stdin exceeds max size {} bytes (got {} bytes)",
                            max,
                            buf.len()
                        ),
                    },
                    StdinCapOrigin::GlobalMaxFilesize => AtomwriteError::FileTooLarge {
                        path: std::path::PathBuf::from(crate::constants::STDIN_SUBJECT),
                        size: buf.len() as u64,
                        max_size: max,
                    },
                }
                .into());
            }
        }
    }
    let n = buf.len();

    if !allow_empty && n == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "stdin produced 0 bytes; pass --allow-empty-stdin to confirm an empty write is intentional".into(),
        }
        .into());
    }

    Ok((buf, n as u64))
}

/// Policy differences between verbs that walk a tree.
///
/// The roots, the worker bound and the glob compilation are always shared. What
/// legitimately differs is whether `--include` filters by path or by file name,
/// and whether the walk ignores the ignore files it finds. Hidden files follow
/// `--hidden` in every flavor: a tree walk that always saw dotfiles made the
/// flag inert, while a path named on the command line is still read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalkerFlavor {
    /// Honor `--hidden`, `--no-gitignore` and `--follow-symlinks`; `--include` is a path glob.
    RespectGlobalFlags,
    /// Same as [`Self::RespectGlobalFlags`], but `--include` selects file names.
    RespectGlobalFlagsByType,
    /// Visit hidden files always; gitignore still honors `--no-gitignore`.
    ///
    /// Only a verb whose own flag asks for the whole tree uses this, such as
    /// `list --all`.
    AlwaysVisitHidden,
    /// Ignore nothing: a tree copy that honored a gitignore would not copy the
    /// tree the caller named. Hidden files still follow `--hidden`.
    CopyTree,
    /// The named tree belongs to the verb: no ignore file, no hidden filter,
    /// no symlink following and no crossing into another file system.
    ///
    /// A recursive delete that honored any filter would leave entries behind
    /// and then report a directory it never finished as an argument mistake.
    DeleteTree,
}

impl WalkerFlavor {
    /// Hidden files are skipped unless the flavor always visits them or
    /// `--hidden` came.
    const fn skips_hidden(self, global_hidden: bool) -> bool {
        match self {
            Self::RespectGlobalFlags | Self::RespectGlobalFlagsByType | Self::CopyTree => {
                !global_hidden
            }
            Self::AlwaysVisitHidden | Self::DeleteTree => false,
        }
    }

    /// gitignore is honored unless the flavor ignores nothing or `--no-gitignore` came.
    const fn honors_gitignore(self, no_gitignore: bool) -> bool {
        match self {
            Self::CopyTree | Self::DeleteTree => false,
            _ => !no_gitignore,
        }
    }

    /// Whether `.ignore`, parent ignore files, the global gitignore and
    /// `.git/info/exclude` still filter this walk.
    ///
    /// A tree copy and a recursive delete own every entry the caller named, so
    /// they read no ignore file at all. The remaining flavors keep the shared
    /// defaults, and `--no-gitignore` still turns off only the gitignore half.
    const fn honors_ignore_files(self) -> bool {
        !matches!(self, Self::CopyTree | Self::DeleteTree)
    }

    /// Symlinks are followed only where the verb reads through them.
    ///
    /// A delete removes the link entry and never what it points at, so the
    /// global flag cannot widen the tree it is allowed to touch.
    const fn follows_links(self, global_follow: bool) -> bool {
        match self {
            Self::DeleteTree => false,
            _ => global_follow,
        }
    }

    /// A recursive delete never crosses into a mounted file system.
    const fn stays_on_one_file_system(self) -> bool {
        matches!(self, Self::DeleteTree)
    }
}

/// The one walker builder for the verbs that scan a tree.
///
/// Roots, hidden files, gitignore, symlinks, the shared worker bound and the
/// include/exclude globs are decided once, so two scanning verbs cannot drift
/// into two different notions of "which files this run visits".
///
/// # Errors
///
/// Returns the error of an include or exclude glob that does not compile.
pub(crate) fn build_scoped_walker_flavored(
    canonical_paths: &[std::path::PathBuf],
    global: &crate::cli::GlobalArgs,
    flavor: WalkerFlavor,
    include: &[String],
    exclude: &[String],
) -> anyhow::Result<ignore::WalkBuilder> {
    let mut builder = ignore::WalkBuilder::new(&canonical_paths[0]);

    for path in canonical_paths.iter().skip(1) {
        builder.add(path);
    }

    builder
        .hidden(flavor.skips_hidden(global.hidden))
        .ignore(flavor.honors_ignore_files())
        .parents(flavor.honors_ignore_files())
        .git_ignore(flavor.honors_gitignore(global.no_gitignore))
        .git_global(flavor.honors_ignore_files())
        .git_exclude(flavor.honors_ignore_files())
        .follow_links(flavor.follows_links(global.follow_symlinks))
        .same_file_system(flavor.stays_on_one_file_system());

    // Always apply the shared bound (default = all cores, RAM-capped).
    crate::concurrency::apply_walk_threads(&mut builder, global.threads);

    if flavor == WalkerFlavor::RespectGlobalFlagsByType {
        // `--include` names files here, and `--exclude` still prunes paths.
        crate::walk_globs::add_include_types(&mut builder, include)?;
        crate::walk_globs::add_overrides(&mut builder, &canonical_paths[0], &[], exclude)?;
    } else {
        crate::walk_globs::add_overrides(&mut builder, &canonical_paths[0], include, exclude)?;
    }

    Ok(builder)
}

/// The canonical walker: every global flag is honored and `--include` is a path glob.
///
/// # Errors
///
/// Returns the error of an include or exclude glob that does not compile.
pub(crate) fn build_scoped_walker(
    canonical_paths: &[std::path::PathBuf],
    global: &crate::cli::GlobalArgs,
    include: &[String],
    exclude: &[String],
) -> anyhow::Result<ignore::WalkBuilder> {
    build_scoped_walker_flavored(
        canonical_paths,
        global,
        WalkerFlavor::RespectGlobalFlags,
        include,
        exclude,
    )
}

/// Read new content from stdin for a subcommand mode that requires it,
/// guarding against a terminal stdin (which would otherwise hang waiting
/// for interactive input).
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when `stdin_is_tty` is true, the
/// typed error of [`read_stdin_content_cancellable`] when the cap is exceeded,
/// and an error when the payload is not UTF-8.
pub(crate) fn read_stdin_text_guarded(
    stdin: impl std::io::Read,
    max_size: u64,
    stdin_is_tty: bool,
    mode: &str,
) -> Result<String, crate::error::AtomwriteError> {
    if stdin_is_tty {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "--{mode} reads content from stdin but stdin is a terminal; \
                 pipe content (echo 'new text' | atomwrite edit ...) or use --new-file"
            ),
        });
    }
    let (bytes, _) = read_stdin_content_cancellable(
        stdin,
        Some(max_size),
        StdinCapOrigin::GlobalMaxFilesize,
        true,
        None,
    )
    .map_err(|e| match e.downcast::<AtomwriteError>() {
        Ok(err) => err,
        Err(other) => AtomwriteError::InternalError {
            reason: format!("cannot read stdin: {other:#}"),
        },
    })?;
    // The cap passed every caller is the global `--max-filesize`, so overflow
    // is a size limit, not an argument mistake. Truncating here would write a
    // prefix of what the operator piped and call it success.
    String::from_utf8(bytes).map_err(|e| AtomwriteError::InvalidInput {
        reason: format!("stdin is not valid UTF-8: {e}"),
    })
}

#[cfg(test)]
mod backup_name_tests {
    use super::is_backup_timestamp_name;

    #[test]
    fn accepts_both_timestamp_shapes() {
        assert!(is_backup_timestamp_name("foo.txt.bak.20260615_035515"));
        assert!(is_backup_timestamp_name("foo.txt.bak.20260615_035515_042"));
        assert!(is_backup_timestamp_name("a.bak.b.bak.20260615_035515"));
    }

    #[test]
    fn rejects_other_names() {
        assert!(!is_backup_timestamp_name("foo.txt"));
        assert!(!is_backup_timestamp_name("foo.txt.bak."));
        assert!(!is_backup_timestamp_name("foo.txt.bak.20260615_035515_42"));
        assert!(!is_backup_timestamp_name("foo.txt.bak.2026061_5035515"));
        assert!(!is_backup_timestamp_name(
            "foo.txt.bak.20260615_035515.orig"
        ));
        assert!(!is_backup_timestamp_name("foo.txt.bak.2026061a_035515"));
    }
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
    fn non_tty_stdin_refuses_a_payload_above_max_size() {
        let err = read_stdin_text_guarded("abcdef".as_bytes(), 3, false, "between")
            .expect_err("a payload above the cap is refused, not truncated");
        match err {
            AtomwriteError::FileTooLarge { size, max_size, .. } => {
                assert_eq!((size, max_size), (6, 3));
            }
            other => panic!("expected FileTooLarge, got {other:?}"),
        }
    }
}
