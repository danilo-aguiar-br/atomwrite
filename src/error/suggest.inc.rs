/// Suggest similar basenames under workspace when a path is missing (v0.1.30).
///
/// Discovery fans out via `WalkParallel` (depth ≤ 6) so monorepo `NotFound`
/// paths do not walk single-core. Bound: process-wide `--threads` pool.
fn similar_paths_for(err: &AtomwriteError, ctx: &ErrorContext) -> Option<Vec<String>> {
    let path = match err {
        AtomwriteError::NotFound { path } => path,
        _ => return None,
    };
    let wanted = path.file_name()?.to_string_lossy().to_string();
    if wanted.is_empty() {
        return None;
    }
    let root = ctx.workspace.as_ref()?;
    let mut builder = ignore::WalkBuilder::new(root);
    builder.max_depth(Some(crate::constants::SUGGEST_WALK_MAX_DEPTH));
    crate::concurrency::apply_walk_threads(&mut builder, None);
    let wanted_c = wanted;
    let mut scored: Vec<(f64, String)> =
        crate::concurrency::collect_mapped_parallel(&builder, move |entry| {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                return None;
            }
            let name = entry.file_name().to_string_lossy();
            let score = strsim::jaro_winkler(&wanted_c, &name);
            if score >= crate::constants::SUGGEST_MIN_SIMILARITY {
                Some((score, entry.path().display().to_string()))
            } else {
                None
            }
        });
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(crate::constants::SUGGEST_MAX_PATHS);
    if scored.is_empty() {
        None
    } else {
        Some(scored.into_iter().map(|(_, p)| p).collect())
    }
}

/// Diagnostic context for error reporting.
///
/// Carries information about the runtime environment that helps
/// `suggestion_for` produce actionable remediation text. The default
/// instance represents "no extra context known" and yields the same
/// suggestions as the pre-GAP-13 code path.
#[derive(Debug, Default, Clone)]
pub struct ErrorContext {
    /// Whether the user explicitly provided a workspace root via `--workspace`
    /// or CLI `--workspace`. When true, a
    /// `WorkspaceJail` error means the path escapes the *user-supplied* root,
    /// so the suggestion should be "use a path inside the workspace" rather
    /// than "set --workspace".
    pub workspace_provided: bool,
    /// Effective workspace root path, if known. Used to enrich suggestions
    /// with the actual path the user passed.
    pub workspace: Option<PathBuf>,
    /// Verb that produced the failure, when the caller knows it.
    ///
    /// `apply` and `edit` share `MatchFailed`, and the two accept different
    /// flags, so the hint must name the verb's own remedy.
    pub verb: Option<&'static str>,
}

#[cold]
fn suggestion_for(err: &AtomwriteError, ctx: &ErrorContext) -> Option<String> {
    // Locale-aware human suggestions via rust-i18n (`locales/{en,pt-BR}.toml`).
    // Error codes and Display messages stay English (agent machine contract).
    match err {
        AtomwriteError::NotFound { .. } | AtomwriteError::GlobNoMatches { .. } => {
            Some(t!("suggestion.not-found").to_string())
        }
        AtomwriteError::EditPairFailed { index, .. } => {
            Some(t!("suggestion.edit-pair-failed", index = index).to_string())
        }
        // The `reason` of these errors is English by contract and stays in
        // `message`; the hint is localized on its own, so it never mixes the two.
        AtomwriteError::InvalidInput { reason } => {
            let key = if reason.contains(crate::constants::JOIN_REASON_MARKER) {
                t!("suggestion.join-input")
            } else if reason.contains(crate::constants::FUZZY_CONTENT_CAP_REASON_MARKER) {
                t!("suggestion.fuzzy-content-cap")
            } else if reason.contains(crate::constants::FUZZY_PATTERN_CAP_REASON_MARKER) {
                t!("suggestion.fuzzy-pattern-cap")
            } else if reason.contains(crate::constants::CONFIG_RELATION_REASON_MARKER) {
                t!("suggestion.config-relation")
            } else if reason.contains(crate::constants::CONFIG_OUT_OF_RANGE_REASON_MARKER) {
                t!("suggestion.config-out-of-range")
            } else if reason.contains(crate::constants::PRUNE_DIRECTORY_REASON_MARKER) {
                t!("suggestion.prune-backups-directory")
            } else if reason.contains(crate::constants::SIDE_EFFECT_NO_SIDE_EFFECT_REASON_MARKER) {
                t!("suggestion.confirm-side-effect-without-side-effect")
            } else if reason.contains(crate::constants::LANGUAGE_MISMATCH_REASON_MARKER) {
                t!("suggestion.language-extension-mismatch")
            } else if reason.contains(crate::constants::SINGLE_TARGET_REASON_MARKER) {
                t!("suggestion.single-target")
            } else if reason.contains(crate::constants::READ_MULTI_TARGET_REASON_MARKER) {
                t!("suggestion.read-multi-target")
            } else if reason.contains(crate::constants::NO_COMMENTS_NEEDS_NUMBER_REASON_MARKER) {
                t!("suggestion.read-no-comments-number")
            } else {
                t!("suggestion.invalid-input")
            };
            Some(key.to_string())
        }
        AtomwriteError::PermissionDenied { .. } => {
            Some(t!("suggestion.permission-denied").to_string())
        }
        AtomwriteError::DiskFull { .. } => Some(t!("suggestion.disk-full").to_string()),
        AtomwriteError::QuotaExceeded { .. } => Some(t!("suggestion.quota-exceeded").to_string()),
        AtomwriteError::CrossDevice { .. } => Some(t!("suggestion.cross-device").to_string()),
        AtomwriteError::Io { source } => {
            Some(t!("suggestion.io", source = source.to_string()).to_string())
        }
        AtomwriteError::ConfigInvalid { reason } => {
            // A verb this build does not carry is not a bad config key: the
            // operator cannot fix it in `config.toml`, only by rebuilding.
            let key = if reason.contains(crate::constants::FEATURE_MISSING_REASON_MARKER) {
                t!("suggestion.feature-missing")
            } else {
                t!("suggestion.config-invalid")
            };
            Some(key.to_string())
        }
        AtomwriteError::StateDrift { .. } => Some(t!("suggestion.state-drift").to_string()),
        AtomwriteError::ChecksumVerifyFailed { .. } => {
            Some(t!("suggestion.checksum-verify").to_string())
        }
        AtomwriteError::FileTooLarge { .. } => Some(t!("suggestion.file-too-large").to_string()),
        AtomwriteError::WorkspaceJail {
            path,
            workspace,
            content_source,
        } => {
            if *content_source && ctx.workspace_provided {
                Some(
                    t!(
                        "suggestion.workspace-jail-content-source",
                        workspace = workspace.display().to_string()
                    )
                    .to_string(),
                )
            } else if ctx.workspace_provided {
                Some(
                    t!(
                        "suggestion.workspace-jail-inside",
                        workspace = workspace.display().to_string()
                    )
                    .to_string(),
                )
            } else {
                // Without a workspace of its own, the caller gets the exact
                // command that would reach this file instead of a placeholder.
                let root = path
                    .parent()
                    .map(|parent| parent.display().to_string())
                    .unwrap_or_else(|| workspace.display().to_string());
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Some(
                    t!(
                        "suggestion.workspace-jail-set",
                        root = root,
                        name = name
                    )
                    .to_string(),
                )
            }
        }
        AtomwriteError::SymlinkBlocked { .. } => Some(t!("suggestion.symlink-blocked").to_string()),
        AtomwriteError::FileImmutable { path } => Some(
            t!(
                "suggestion.immutable-path",
                path = path.display().to_string()
            )
            .to_string(),
        ),
        AtomwriteError::BinaryFile { .. } => Some(t!("suggestion.binary-file").to_string()),
        AtomwriteError::FifoDetected { .. } => Some(t!("suggestion.skip-special").to_string()),
        AtomwriteError::DeviceFile { .. } => Some(t!("suggestion.skip-special").to_string()),
        AtomwriteError::NoMatches => Some(t!("suggestion.no-matches").to_string()),
        AtomwriteError::AllSkippedBinary { .. } => {
            Some(t!("suggestion.all-skipped-binary").to_string())
        }
        AtomwriteError::BrokenPipe => None,
        AtomwriteError::MatchAmbiguous { .. } => Some(t!("suggestion.match-ambiguous").to_string()),
        AtomwriteError::MatchFailed { .. } => {
            let key = if ctx.verb == Some("apply") {
                t!("suggestion.match-failed-apply")
            } else {
                t!("suggestion.match-failed")
            };
            Some(key.to_string())
        }
        AtomwriteError::Cancelled { .. } => Some(t!("suggestion.cancelled").to_string()),
        AtomwriteError::InternalError { .. } => Some(t!("suggestion.internal-error").to_string()),
        AtomwriteError::LockTimeout { path, timeout_ms } => Some(
            t!(
                "suggestion.lock-timeout",
                path = path.display().to_string(),
                timeout_ms = timeout_ms
            )
            .to_string(),
        ),
        AtomwriteError::SyntaxError { path, count, .. } => Some(
            t!(
                "suggestion.syntax-error",
                path = path.display().to_string(),
                count = count
            )
            .to_string(),
        ),
        AtomwriteError::ExdevFallbackDisabled { path, .. } => Some(
            t!(
                "suggestion.exdev-fallback-disabled",
                path = path.display().to_string()
            )
            .to_string(),
        ),
        AtomwriteError::CopyBackBlake3Failed { path } => Some(
            t!(
                "suggestion.copy-back-blake3-failed",
                path = path.display().to_string()
            )
            .to_string(),
        ),
        AtomwriteError::OrphanJournal { journal, .. } => Some(
            t!(
                "suggestion.orphan-journal",
                journal = journal.display().to_string()
            )
            .to_string(),
        ),
        AtomwriteError::MergeConflict { count, path } => Some(
            t!(
                "suggestion.merge-conflict",
                count = count,
                path = path.display().to_string()
            )
            .to_string(),
        ),
        AtomwriteError::EditLoopPartial { unmatched, total } => Some(
            t!(
                "suggestion.edit-loop-partial",
                unmatched = unmatched,
                total = total
            )
            .to_string(),
        ),
        AtomwriteError::DoctorChecksFailed { failed, warned } => Some(
            t!(
                "suggestion.doctor-checks-failed",
                failed = failed,
                warned = warned
            )
            .to_string(),
        ),
        AtomwriteError::MissingFlag { flag, .. } => {
            Some(t!("suggestion.missing-flag", flag = flag.as_str()).to_string())
        }
        AtomwriteError::TransactionRollbackFailed { .. } => {
            Some(t!("suggestion.transaction-rollback-failed").to_string())
        }
        AtomwriteError::GrammarUnavailable { language, .. } => Some(
            t!("suggestion.grammar-unavailable", language = language.as_str()).to_string(),
        ),
        AtomwriteError::UnsupportedCombination { flag, other, .. } => Some(
            t!(
                "suggestion.unsupported-combination",
                flag = flag.as_str(),
                other = other.as_str()
            )
            .to_string(),
        ),
    }
}
