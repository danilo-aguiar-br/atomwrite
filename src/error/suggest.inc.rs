
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
    builder.max_depth(Some(6));
    crate::concurrency::apply_walk_threads(&mut builder, None);
    let wanted_c = wanted;
    let mut scored: Vec<(f64, String)> =
        crate::concurrency::collect_mapped_parallel(&builder, move |entry| {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                return None;
            }
            let name = entry.file_name().to_string_lossy();
            let score = strsim::jaro_winkler(&wanted_c, &name);
            if score >= 0.75 {
                Some((score, entry.path().display().to_string()))
            } else {
                None
            }
        });
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(5);
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
}

#[cold]
fn suggestion_for(err: &AtomwriteError, ctx: &ErrorContext) -> Option<String> {
    // Locale-aware human suggestions via rust-i18n (`locales/{en,pt-BR}.toml`).
    // Error codes and Display messages stay English (agent machine contract).
    match err {
        AtomwriteError::NotFound { .. } => Some(t!("suggestion.not-found").to_string()),
        AtomwriteError::EditPairFailed { index, .. } => Some(
            t!("suggestion.edit-pair-failed", index = index).to_string(),
        ),
        AtomwriteError::InvalidInput { reason } => Some(
            t!("suggestion.invalid-input", reason = reason.as_str()).to_string(),
        ),
        AtomwriteError::PermissionDenied { .. } => {
            Some(t!("suggestion.permission-denied").to_string())
        }
        AtomwriteError::DiskFull { .. } => Some(t!("suggestion.disk-full").to_string()),
        AtomwriteError::QuotaExceeded { .. } => {
            Some(t!("suggestion.quota-exceeded").to_string())
        }
        AtomwriteError::CrossDevice { .. } => Some(t!("suggestion.cross-device").to_string()),
        AtomwriteError::Io { source } => {
            Some(t!("suggestion.io", source = source.to_string()).to_string())
        }
        AtomwriteError::ConfigInvalid { reason } => Some(
            t!("suggestion.config-invalid", reason = reason.as_str()).to_string(),
        ),
        AtomwriteError::StateDrift { .. } => Some(t!("suggestion.state-drift").to_string()),
        AtomwriteError::ChecksumVerifyFailed { .. } => {
            Some(t!("suggestion.checksum-verify").to_string())
        }
        AtomwriteError::FileTooLarge { .. } => Some(t!("suggestion.file-too-large").to_string()),
        AtomwriteError::WorkspaceJail { workspace, .. } => {
            if ctx.workspace_provided {
                Some(
                    t!(
                        "suggestion.workspace-jail-inside",
                        workspace = workspace.display().to_string()
                    )
                    .to_string(),
                )
            } else {
                Some(t!("suggestion.workspace-jail-set").to_string())
            }
        }
        AtomwriteError::SymlinkBlocked { .. } => {
            Some(t!("suggestion.symlink-blocked").to_string())
        }
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
        AtomwriteError::BrokenPipe => None,
        AtomwriteError::MatchAmbiguous { .. } => {
            Some(t!("suggestion.match-ambiguous").to_string())
        }
        AtomwriteError::MatchFailed { .. } => Some(t!("suggestion.match-failed").to_string()),
        AtomwriteError::Cancelled { .. } => Some(t!("suggestion.cancelled").to_string()),
        AtomwriteError::InternalError { reason } => Some(
            t!("suggestion.internal-error", reason = reason.as_str()).to_string(),
        ),
        AtomwriteError::LockTimeout { path, timeout_ms } => Some(
            t!(
                "suggestion.lock-timeout",
                path = path.display().to_string(),
                timeout_ms = timeout_ms
            )
            .to_string(),
        ),
        AtomwriteError::SyntaxError { path, count } => Some(
            t!(
                "suggestion.syntax-error",
                path = path.display().to_string(),
                count = count
            )
            .to_string(),
        ),
        AtomwriteError::ExdevFallbackDisabled { path } => Some(
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
        AtomwriteError::OrphanJournal { journal, reason } => Some(
            t!(
                "suggestion.orphan-journal",
                journal = journal.display().to_string(),
                reason = reason.as_str()
            )
            .to_string(),
        ),
    }
}

