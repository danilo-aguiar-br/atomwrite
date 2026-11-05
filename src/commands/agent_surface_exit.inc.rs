// SPDX-License-Identifier: MIT OR Apache-2.0
// Exit status list derived from the error enum.
// Included from agent_surface.rs.

/// One exit status of the process, derived from an error variant.
#[derive(Serialize, JsonSchema)]
struct ExitCodeEntry {
    /// Numeric status the process returns to the shell.
    code: u8,
    /// Machine-readable error code, or the process-level label.
    name: &'static str,
    /// Recovery class: `transient`, `conflict`, `precondition_failed`
    /// or `permanent`.
    error_class: &'static str,
    /// Whether retrying the same operation may succeed.
    retryable: bool,
    /// Provenance: `error-variant` (derived from the error enum) or
    /// `process` (status produced by the runtime, not by an error variant).
    source: &'static str,
}

/// One representative error instance per [`crate::error::AtomwriteError`] variant.
///
/// The list exists so [`exit_code_entries`] can call the public
/// `exit_code` / `error_code` / `error_class` methods on real values. It is
/// the only place that enumerates variants, because the enum carries struct
/// fields and so cannot itself be iterated. When a variant is added to
/// `crate::error`, add it here too; the deduplicated output then carries the
/// new status without any hand-written number.
fn sample_variants() -> Vec<crate::error::AtomwriteError> {
    use crate::error::AtomwriteError;
    use std::path::PathBuf;
    let p = || PathBuf::from("/probe");
    vec![
        AtomwriteError::NotFound { path: p() },
        AtomwriteError::GlobNoMatches { pattern: p() },
        AtomwriteError::InvalidInput {
            reason: String::new(),
        },
        AtomwriteError::MatchFailed {
            reason: String::new(),
            best_candidate: None,
            candidates: None,
        },
        AtomwriteError::MatchAmbiguous {
            reason: String::new(),
            count: 0,
            best_candidate: None,
        },
        AtomwriteError::Cancelled {
            reason: String::new(),
            exit: crate::signal::current_exit_code(),
        },
        AtomwriteError::PermissionDenied { path: p() },
        AtomwriteError::DiskFull { path: p() },
        AtomwriteError::QuotaExceeded { path: p() },
        AtomwriteError::CrossDevice { path: p() },
        AtomwriteError::Io {
            source: std::io::Error::other("agent-surface probe"),
        },
        AtomwriteError::ConfigInvalid {
            reason: String::new(),
        },
        AtomwriteError::StateDrift {
            path: p(),
            expected: String::new(),
            actual: String::new(),
        },
        AtomwriteError::WorkspaceJail {
            path: p(),
            workspace: p(),
            content_source: false,
        },
        AtomwriteError::SymlinkBlocked { path: p() },
        AtomwriteError::FileImmutable { path: p() },
        AtomwriteError::BinaryFile { path: p() },
        AtomwriteError::FifoDetected { path: p() },
        AtomwriteError::DeviceFile { path: p() },
        AtomwriteError::ChecksumVerifyFailed {
            path: p(),
            expected: String::new(),
        },
        AtomwriteError::FileTooLarge {
            path: p(),
            size: 0,
            max_size: 0,
        },
        AtomwriteError::NoMatches,
        AtomwriteError::BrokenPipe,
        AtomwriteError::InternalError {
            reason: String::new(),
        },
        AtomwriteError::LockTimeout {
            path: p(),
            timeout_ms: 0,
        },
        AtomwriteError::SyntaxError {
            path: p(),
            count: 0,
            line: None,
            line_end: None,
        },
        AtomwriteError::ExdevFallbackDisabled {
            path: p(),
            reason: String::new(),
        },
        AtomwriteError::CopyBackBlake3Failed { path: p() },
        AtomwriteError::OrphanJournal {
            journal: p(),
            reason: String::new(),
        },
        AtomwriteError::EditPairFailed {
            index: 1,
            total: 1,
            reason: String::new(),
            pair_results: Box::new(Vec::new()),
            best_candidate: None,
        },
        AtomwriteError::MergeConflict {
            count: 0,
            path: p(),
        },
        AtomwriteError::EditLoopPartial {
            unmatched: 0,
            total: 0,
        },
        AtomwriteError::MissingFlag {
            flag: String::new(),
            reason: String::new(),
        },
        AtomwriteError::TransactionRollbackFailed {
            reason: String::new(),
        },
        AtomwriteError::DoctorChecksFailed {
            failed: 0,
            warned: 0,
        },
        AtomwriteError::GrammarUnavailable {
            language: String::new(),
            reason: String::new(),
        },
        AtomwriteError::UnsupportedCombination {
            flag: String::new(),
            other: String::new(),
            reason: String::new(),
        },
        AtomwriteError::AllSkippedBinary { files: 0 },
    ]
}

/// Build the exit status list from the error enum itself (B3).
///
/// Every entry with `source: "error-variant"` is read off a real
/// [`crate::error::AtomwriteError`] through its public methods, so the
/// numbers, the machine codes and the recovery classes cannot drift from
/// the code that produces them. The remaining entries are process statuses
/// that no error variant owns.
fn exit_code_entries() -> Vec<ExitCodeEntry> {
    let mut codes: Vec<ExitCodeEntry> = Vec::new();
    for err in sample_variants() {
        let code = err.exit_code();
        let name = err.error_code();
        if codes.iter().any(|e| e.code == code && e.name == name) {
            continue;
        }
        codes.push(ExitCodeEntry {
            code,
            name,
            error_class: err.error_class().as_str(),
            retryable: err.is_retryable(),
            source: "error-variant",
        });
    }
    // Process statuses with no error variant behind them.
    codes.push(ExitCodeEntry {
        code: crate::constants::EXIT_SUCCESS as u8,
        name: "SUCCESS",
        error_class: "none",
        retryable: false,
        source: "process",
    });
    codes.push(ExitCodeEntry {
        code: crate::constants::EXIT_ARGUMENT_PARSE_ERROR as u8,
        name: "ARGUMENT_PARSE_ERROR",
        error_class: "permanent",
        retryable: false,
        source: "process",
    });
    codes.push(ExitCodeEntry {
        code: crate::constants::EXIT_TIMEOUT as u8,
        name: "TIMEOUT",
        error_class: "precondition_failed",
        retryable: false,
        source: "process",
    });
    codes.push(ExitCodeEntry {
        code: crate::constants::EXIT_SIGHUP as u8,
        name: "SIGHUP",
        error_class: "none",
        retryable: false,
        source: "process",
    });
    codes.push(ExitCodeEntry {
        code: crate::constants::EXIT_SIGINT as u8,
        name: "SIGINT",
        error_class: "none",
        retryable: false,
        source: "process",
    });
    codes.push(ExitCodeEntry {
        code: crate::constants::EXIT_SIGTERM as u8,
        name: "SIGTERM",
        error_class: "none",
        retryable: false,
        source: "process",
    });
    codes.sort_by_key(|e| e.code);
    codes.dedup_by(|a, b| a.code == b.code && a.name == b.name);
    codes
}

/// Exit status table for `--help`, derived from `exit_code_entries`.
///
/// One line per `(code, name)` pair, in the order of the agent-surface
/// manifest, so the help text, the manifest and the README table cannot
/// disagree about which statuses the process can return.
pub fn exit_codes_help() -> String {
    let rows: Vec<String> = exit_code_entries()
        .iter()
        .map(|entry| format!("  {:>3}  {}", entry.code, entry.name))
        .collect();
    format!("Exit codes:\n{}", rows.join("\n"))
}

/// Stable ordinal of one error variant, used for coverage checks.
///
/// The match is deliberately exhaustive and has no wildcard arm: adding a
/// variant to [`crate::error::AtomwriteError`] stops the build here until
/// [`sample_variants`] also learns about it, which is what keeps the derived
/// exit status list truthful (B3). The number of variants is the length of
/// [`sample_variants`], never a hand-kept constant.
#[cfg(test)]
fn variant_ordinal(err: &crate::error::AtomwriteError) -> u32 {
    use crate::error::AtomwriteError as E;
    match err {
        E::NotFound { .. } => 0,
        E::GlobNoMatches { .. } => 1,
        E::InvalidInput { .. } => 2,
        E::MatchFailed { .. } => 3,
        E::MatchAmbiguous { .. } => 4,
        E::Cancelled { .. } => 5,
        E::PermissionDenied { .. } => 6,
        E::DiskFull { .. } => 7,
        E::QuotaExceeded { .. } => 8,
        E::CrossDevice { .. } => 9,
        E::Io { .. } => 10,
        E::ConfigInvalid { .. } => 11,
        E::StateDrift { .. } => 12,
        E::ChecksumVerifyFailed { .. } => 13,
        E::FileTooLarge { .. } => 14,
        E::WorkspaceJail { .. } => 15,
        E::SymlinkBlocked { .. } => 16,
        E::FileImmutable { .. } => 17,
        E::BinaryFile { .. } => 18,
        E::FifoDetected { .. } => 19,
        E::DeviceFile { .. } => 20,
        E::NoMatches => 21,
        E::BrokenPipe => 22,
        E::InternalError { .. } => 23,
        E::LockTimeout { .. } => 24,
        E::SyntaxError { .. } => 25,
        E::ExdevFallbackDisabled { .. } => 26,
        E::CopyBackBlake3Failed { .. } => 27,
        E::OrphanJournal { .. } => 28,
        E::EditPairFailed { .. } => 29,
        E::MergeConflict { .. } => 30,
        E::EditLoopPartial { .. } => 31,
        E::MissingFlag { .. } => 32,
        E::TransactionRollbackFailed { .. } => 33,
        E::DoctorChecksFailed { .. } => 34,
        E::GrammarUnavailable { .. } => 35,
        E::UnsupportedCombination { .. } => 36,
        E::AllSkippedBinary { .. } => 37,
    }
}
