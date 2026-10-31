#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_class_transient() {
        let err = AtomwriteError::DiskFull {
            path: PathBuf::from("/tmp"),
        };
        assert_eq!(err.error_class(), ErrorClass::Transient);
        assert!(err.is_retryable());
        assert!(!err.is_permanent());
    }

    #[test]
    fn error_class_conflict() {
        let err = AtomwriteError::StateDrift {
            path: PathBuf::from("/tmp"),
            expected: "aaa".into(),
            actual: "bbb".into(),
        };
        assert_eq!(err.error_class(), ErrorClass::Conflict);
        assert!(err.is_retryable());
        assert!(!err.is_permanent());
    }

    #[test]
    fn error_class_precondition() {
        let err = AtomwriteError::BinaryFile {
            path: PathBuf::from("/tmp"),
        };
        assert_eq!(err.error_class(), ErrorClass::PreconditionFailed);
        assert!(!err.is_retryable());
        assert!(!err.is_permanent());
    }

    #[test]
    fn error_class_permanent() {
        let err = AtomwriteError::NoMatches;
        assert_eq!(err.error_class(), ErrorClass::Permanent);
        assert!(!err.is_retryable());
        assert!(err.is_permanent());
    }

    #[test]
    fn exit_code_not_found() {
        let err = AtomwriteError::NotFound {
            path: PathBuf::from("/x"),
        };
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn error_code_strings() {
        assert_eq!(
            AtomwriteError::NotFound {
                path: PathBuf::from("/x")
            }
            .error_code(),
            "FILE_NOT_FOUND"
        );
        assert_eq!(
            AtomwriteError::FifoDetected {
                path: PathBuf::from("/x")
            }
            .error_code(),
            "FIFO_DETECTED"
        );
        assert_eq!(
            AtomwriteError::DeviceFile {
                path: PathBuf::from("/x")
            }
            .error_code(),
            "DEVICE_FILE"
        );
    }

    #[test]
    fn fifo_and_device_exit_codes() {
        assert_eq!(
            AtomwriteError::FifoDetected {
                path: PathBuf::from("/x")
            }
            .exit_code(),
            85
        );
        assert_eq!(
            AtomwriteError::DeviceFile {
                path: PathBuf::from("/x")
            }
            .exit_code(),
            86
        );
    }

    #[test]
    fn error_enum_size_audit() {
        let size = std::mem::size_of::<AtomwriteError>();
        assert!(size <= 80, "AtomwriteError grew beyond 80 bytes: {size}");
    }

    #[test]
    fn all_variants_properties() {
        let p = PathBuf::from("/test");
        let variants: Vec<(AtomwriteError, u8, ErrorClass, &str, bool)> = vec![
            (
                AtomwriteError::NotFound { path: p.clone() },
                4,
                ErrorClass::Permanent,
                "FILE_NOT_FOUND",
                true,
            ),
            (
                AtomwriteError::InvalidInput { reason: "x".into() },
                65,
                ErrorClass::Permanent,
                "INVALID_INPUT",
                false,
            ),
            (
                AtomwriteError::PermissionDenied { path: p.clone() },
                13,
                ErrorClass::Permanent,
                "PERMISSION_DENIED",
                true,
            ),
            (
                AtomwriteError::DiskFull { path: p.clone() },
                28,
                ErrorClass::Transient,
                "DISK_FULL",
                true,
            ),
            (
                AtomwriteError::QuotaExceeded { path: p.clone() },
                30,
                ErrorClass::Transient,
                "QUOTA_EXCEEDED",
                true,
            ),
            (
                AtomwriteError::CrossDevice { path: p.clone() },
                73,
                ErrorClass::Conflict,
                "CROSS_DEVICE",
                true,
            ),
            (
                AtomwriteError::Io {
                    source: std::io::Error::other("x"),
                },
                74,
                ErrorClass::Transient,
                "IO_ERROR",
                false,
            ),
            (
                AtomwriteError::ConfigInvalid { reason: "x".into() },
                78,
                ErrorClass::Permanent,
                "CONFIG_INVALID",
                false,
            ),
            (
                AtomwriteError::StateDrift {
                    path: p.clone(),
                    expected: "a".into(),
                    actual: "b".into(),
                },
                82,
                ErrorClass::Conflict,
                "STATE_DRIFT",
                true,
            ),
            (
                AtomwriteError::WorkspaceJail {
                    path: p.clone(),
                    workspace: p.clone(),
                    content_source: false,
                },
                126,
                ErrorClass::PreconditionFailed,
                "WORKSPACE_JAIL",
                true,
            ),
            (
                AtomwriteError::SymlinkBlocked { path: p.clone() },
                127,
                ErrorClass::PreconditionFailed,
                "SYMLINK_BLOCKED",
                true,
            ),
            (
                AtomwriteError::FileImmutable { path: p.clone() },
                128,
                ErrorClass::PreconditionFailed,
                "IMMUTABLE_FILE",
                true,
            ),
            (
                AtomwriteError::BinaryFile { path: p.clone() },
                65,
                ErrorClass::PreconditionFailed,
                "BINARY_FILE",
                true,
            ),
            (
                AtomwriteError::FifoDetected { path: p.clone() },
                85,
                ErrorClass::PreconditionFailed,
                "FIFO_DETECTED",
                true,
            ),
            (
                AtomwriteError::DeviceFile { path: p.clone() },
                86,
                ErrorClass::PreconditionFailed,
                "DEVICE_FILE",
                true,
            ),
            (
                AtomwriteError::ChecksumVerifyFailed {
                    path: p.clone(),
                    expected: "a".into(),
                },
                81,
                ErrorClass::PreconditionFailed,
                "CHECKSUM_VERIFY_FAILED",
                true,
            ),
            (
                AtomwriteError::FileTooLarge {
                    path: p.clone(),
                    size: 100,
                    max_size: 50,
                },
                65,
                ErrorClass::PreconditionFailed,
                "FILE_TOO_LARGE",
                true,
            ),
            (
                AtomwriteError::NoMatches,
                1,
                ErrorClass::Permanent,
                "NO_MATCHES",
                false,
            ),
            (
                AtomwriteError::BrokenPipe,
                141,
                ErrorClass::Permanent,
                "BROKEN_PIPE",
                false,
            ),
            (
                AtomwriteError::InternalError { reason: "x".into() },
                255,
                ErrorClass::Permanent,
                "INTERNAL_ERROR",
                false,
            ),
            (
                AtomwriteError::LockTimeout {
                    path: p.clone(),
                    timeout_ms: 5000,
                },
                83,
                ErrorClass::Conflict,
                "LOCK_TIMEOUT",
                true,
            ),
            (
                AtomwriteError::SyntaxError {
                    path: p.clone(),
                    count: 1,
                },
                88,
                ErrorClass::PreconditionFailed,
                "SYNTAX_ERROR_DETECTED",
                true,
            ),
            (
                AtomwriteError::ExdevFallbackDisabled {
                    path: p.clone(),
                    reason: String::new(),
                },
                91,
                ErrorClass::PreconditionFailed,
                "EXDEV_FALLBACK_DISABLED",
                true,
            ),
            (
                AtomwriteError::CopyBackBlake3Failed { path: p.clone() },
                92,
                ErrorClass::Conflict,
                "COPY_BACK_BLAKE3_FAILED",
                true,
            ),
            (
                AtomwriteError::OrphanJournal {
                    journal: p,
                    reason: "x".into(),
                },
                93,
                ErrorClass::PreconditionFailed,
                "ORPHAN_JOURNAL",
                true,
            ),
        ];
        assert_eq!(variants.len(), 25, "test must cover all 25 variants");
        for (err, exit, class, code, has_path) in &variants {
            assert_eq!(err.exit_code(), *exit, "exit_code mismatch for {code}");
            assert_eq!(err.error_class(), *class, "error_class mismatch for {code}");
            assert_eq!(err.error_code(), *code, "error_code mismatch for {code}");
            assert_eq!(
                err.is_retryable(),
                class.is_retryable(),
                "retryable mismatch for {code}"
            );
            assert_eq!(err.path().is_some(), *has_path, "path mismatch for {code}");
            let json = ErrorJson::from_error(err);
            assert!(json.error);
            assert_eq!(json.exit, *exit);
            assert_eq!(json.code, *code);
            assert_eq!(json.error_class, class.as_str());
            let _ = serde_json::to_string(&json).expect("ErrorJson must serialize");
        }
    }

    #[test]
    fn error_class_as_str_roundtrip() {
        assert_eq!(ErrorClass::Transient.as_str(), "transient");
        assert_eq!(ErrorClass::Conflict.as_str(), "conflict");
        assert_eq!(
            ErrorClass::PreconditionFailed.as_str(),
            "precondition_failed"
        );
        assert_eq!(ErrorClass::Permanent.as_str(), "permanent");
    }

    #[test]
    fn error_class_is_permanent() {
        assert!(ErrorClass::Permanent.is_permanent());
        assert!(!ErrorClass::Transient.is_permanent());
        assert!(!ErrorClass::Conflict.is_permanent());
        assert!(!ErrorClass::PreconditionFailed.is_permanent());
    }

    #[test]
    fn error_json_from_error() {
        let err = AtomwriteError::NotFound {
            path: PathBuf::from("/missing"),
        };
        let json = ErrorJson::from_error(&err);
        assert!(json.error);
        assert_eq!(json.code, "FILE_NOT_FOUND");
        assert_eq!(json.exit, 4);
        assert!(!json.retryable);
    }

    // GAP 13 — context-aware suggestions

    #[test]
    fn gap13_workspace_jail_suggestion_when_workspace_not_provided() {
        // English assertion: pin UI locale for whole test (host may be pt-BR).
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Language::En);
        let err = AtomwriteError::WorkspaceJail {
            path: PathBuf::from("/etc/passwd"),
            workspace: PathBuf::from("/home/user/project"),
            content_source: false,
        };
        let ctx = ErrorContext::default();
        let json = ErrorJson::from_error_with_context(&err, &ctx);
        let s = json.suggestion.expect("must have suggestion");
        assert!(
            s.contains("--workspace"),
            "without workspace_provided, suggestion must mention --workspace, got: {s}"
        );
        assert_eq!(json.workspace.as_deref(), Some("/home/user/project"));
    }

    #[test]
    fn gap13_workspace_jail_suggestion_when_workspace_provided() {
        // Hold locale mutex for the whole assertion (parallel-test race fix).
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Language::En);
        let err = AtomwriteError::WorkspaceJail {
            path: PathBuf::from("/etc/passwd"),
            workspace: PathBuf::from("/home/user/project"),
            content_source: false,
        };
        let ctx = ErrorContext {
            workspace_provided: true,
            workspace: Some(PathBuf::from("/home/user/project")),
        };
        let json = ErrorJson::from_error_with_context(&err, &ctx);
        let s = json.suggestion.expect("must have suggestion");
        assert!(
            s.contains("inside the workspace"),
            "with workspace_provided, suggestion must say 'inside the workspace', got: {s}"
        );
        assert!(
            !s.contains("--workspace"),
            "with workspace_provided, suggestion must NOT mention --workspace flag, got: {s}"
        );
    }

    #[test]
    fn content_source_jail_uses_staging_suggestion() {
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Language::En);
        let err = AtomwriteError::WorkspaceJail {
            path: PathBuf::from("/tmp/old.txt"),
            workspace: PathBuf::from("/home/user/project"),
            content_source: true,
        };
        let ctx = ErrorContext {
            workspace_provided: true,
            workspace: Some(PathBuf::from("/home/user/project")),
        };
        let json = ErrorJson::from_error_with_context(&err, &ctx);
        let s = json.suggestion.expect("must have suggestion");
        assert!(
            s.contains("content-source") || s.contains(".atomwrite/scratch"),
            "content_source jail must mention staging/content-source, got: {s}"
        );
    }

    #[test]
    fn gap13_all_variants_have_suggestion() {
        let variants: Vec<AtomwriteError> = vec![
            AtomwriteError::NotFound {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::InvalidInput { reason: "x".into() },
            AtomwriteError::PermissionDenied {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::DiskFull {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::QuotaExceeded {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::CrossDevice {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::Io {
                source: std::io::Error::other("x"),
            },
            AtomwriteError::ConfigInvalid { reason: "x".into() },
            AtomwriteError::StateDrift {
                path: PathBuf::from("/x"),
                expected: "a".into(),
                actual: "b".into(),
            },
            AtomwriteError::ChecksumVerifyFailed {
                path: PathBuf::from("/x"),
                expected: "a".into(),
            },
            AtomwriteError::FileTooLarge {
                path: PathBuf::from("/x"),
                size: 1,
                max_size: 2,
            },
            AtomwriteError::WorkspaceJail {
                path: PathBuf::from("/x"),
                workspace: PathBuf::from("/w"),
                content_source: false,
            },
            AtomwriteError::SymlinkBlocked {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::FileImmutable {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::BinaryFile {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::FifoDetected {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::DeviceFile {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::NoMatches,
            AtomwriteError::BrokenPipe,
            AtomwriteError::InternalError { reason: "x".into() },
            AtomwriteError::LockTimeout {
                path: PathBuf::from("/x"),
                timeout_ms: 5000,
            },
            AtomwriteError::SyntaxError {
                path: PathBuf::from("/x"),
                count: 1,
            },
            AtomwriteError::ExdevFallbackDisabled {
                path: PathBuf::from("/x"),
                reason: "cross-device rename".into(),
            },
            AtomwriteError::CopyBackBlake3Failed {
                path: PathBuf::from("/x"),
            },
            AtomwriteError::OrphanJournal {
                journal: PathBuf::from("/x"),
                reason: "x".into(),
            },
            AtomwriteError::EditPairFailed {
                index: 2,
                total: 3,
                reason: "x".into(),
                pair_results: Box::new(vec![]),
                best_candidate: None,
            },
        ];
        assert_eq!(variants.len(), 26);
        for err in &variants {
            let json = ErrorJson::from_error(err);
            if matches!(err, AtomwriteError::BrokenPipe) {
                assert!(
                    json.suggestion.is_none(),
                    "BrokenPipe must remain without suggestion (SIGPIPE is not actionable)"
                );
            } else {
                assert!(
                    json.suggestion.is_some(),
                    "GAP 13: variant {err:?} must have suggestion"
                );
            }
        }
    }

    #[test]
    fn gap13_binary_file_suggestion_does_not_mention_force_text_wrong_flag() {
        let err = AtomwriteError::BinaryFile {
            path: PathBuf::from("/x"),
        };
        let json = ErrorJson::from_error(&err);
        let s = json.suggestion.expect("must have suggestion");
        assert!(
            s.contains("read --stat"),
            "BinaryFile suggestion must mention read --stat, got: {s}"
        );
    }

    #[test]
    fn gap13_file_immutable_suggestion_mentions_chattr() {
        let err = AtomwriteError::FileImmutable {
            path: PathBuf::from("/etc/shadow"),
        };
        let json = ErrorJson::from_error(&err);
        let s = json.suggestion.expect("must have suggestion");
        assert!(
            s.contains("chattr"),
            "FileImmutable suggestion must mention chattr, got: {s}"
        );
    }

    #[test]
    fn gap13_no_matches_suggestion_mentions_filters() {
        let err = AtomwriteError::NoMatches;
        let json = ErrorJson::from_error(&err);
        let s = json.suggestion.expect("must have suggestion");
        assert!(
            s.contains("--include") || s.contains("broaden"),
            "NoMatches suggestion must mention broadening or filters, got: {s}"
        );
    }

    #[test]
    fn gap13_error_context_default_matches_legacy_behavior() {
        // Default ErrorContext + default locale (`en`) must yield the English
        // suggestion from `locales/en.toml` (locale-aware suggestions).
        // A-012: RAII guard serializes locale mutations across parallel tests.
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Language::En);
        let err = AtomwriteError::NotFound {
            path: PathBuf::from("/x"),
        };
        let json = ErrorJson::from_error(&err);
        assert_eq!(
            json.suggestion.as_deref(),
            Some("verify the file path exists and is spelled correctly")
        );
    }

    #[test]
    fn suggestion_follows_pt_br_locale() {
        // A-012: hold mutex for entire assertion window (restore on drop).
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Language::PtBr);
        let err = AtomwriteError::NotFound {
            path: PathBuf::from("/x"),
        };
        let json = ErrorJson::from_error(&err);
        let s = json.suggestion.expect("suggestion");
        assert!(
            s.contains("arquivo") || s.contains("caminho"),
            "pt-BR suggestion expected, got: {s}"
        );
    }

    fn cap_error(reason: &str) -> AtomwriteError {
        AtomwriteError::InvalidInput {
            reason: reason.into(),
        }
    }

    const CONTENT_CAP_REASON: &str = "content too large for the fuzzy cascade (31 bytes > 8 max); raise [fuzzy].max_content_bytes in config.toml or pass --fuzzy off for exact-only matching";

    #[test]
    fn fuzzy_cap_hint_is_fully_localized_and_keeps_the_reason_out_of_it() {
        let err = cap_error(CONTENT_CAP_REASON);
        {
            let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Language::PtBr);
            let hint = ErrorJson::from_error(&err).suggestion.expect("hint");
            assert!(hint.contains("aumente"), "{hint}");
            assert!(hint.contains("`[fuzzy].max_content_bytes`"), "{hint}");
            assert!(hint.contains("`--fuzzy off`"), "{hint}");
            assert!(!hint.contains("raise"), "English text leaked: {hint}");
            assert!(!hint.contains("too large"), "the reason leaked: {hint}");
        }
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Language::En);
        let hint = ErrorJson::from_error(&err).suggestion.expect("hint");
        assert!(hint.contains("raise `[fuzzy].max_content_bytes`"), "{hint}");
        assert!(hint.contains("`--fuzzy off`"), "{hint}");
    }

    /// The reason the fuzzy code emits and the marker that picks the hint come
    /// from the same constants; this fails if either side stops matching.
    #[test]
    fn emitted_fuzzy_cap_reasons_select_their_dedicated_hints() {
        use crate::fuzzy::{MatchOpts, apply_fuzzy_one_pass, match_pair_with};
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Language::En);
        let hint_of = |err: AtomwriteError| {
            assert!(
                matches!(err, AtomwriteError::InvalidInput { .. }),
                "{err:?}"
            );
            ErrorJson::from_error(&err).suggestion.expect("hint")
        };
        let capped = MatchOpts {
            max_content_bytes: 8,
            ..MatchOpts::default()
        };
        let err = match_pair_with("alpha beta gamma delta", "wrongword", "x", capped).unwrap_err();
        assert!(
            hint_of(err).contains("raise `[fuzzy].max_content_bytes`"),
            "the content-cap reason no longer selects its hint"
        );
        let long = "x".repeat(crate::constants::FUZZY_MAX_PATTERN_BYTES + 1);
        let err = match_pair_with("y", &long, "z", MatchOpts::default()).unwrap_err();
        assert!(
            hint_of(err).contains("shorten the block"),
            "cascade pattern cap"
        );
        let err = apply_fuzzy_one_pass("y", &long, "z", MatchOpts::default(), None).unwrap_err();
        assert!(
            hint_of(err).contains("shorten the block"),
            "one-pass pattern cap"
        );
    }

    #[test]
    fn fuzzy_pattern_cap_and_generic_invalid_input_have_their_own_hints() {
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Language::PtBr);
        let pattern =
            cap_error("fuzzy pattern too large (70000 bytes > 65536 max); shorten the block");
        let hint = ErrorJson::from_error(&pattern).suggestion.expect("hint");
        assert!(hint.contains("encurte"), "{hint}");
        assert!(!hint.contains("shorten"), "{hint}");
        let generic = cap_error("old string must not be empty");
        let hint = ErrorJson::from_error(&generic).suggestion.expect("hint");
        assert!(hint.contains("confira os argumentos"), "{hint}");
        assert!(!hint.contains("empty"), "the reason leaked: {hint}");
    }

    #[test]
    fn edit_pair_failed_is_a_match_failure_with_the_invalid_input_exit() {
        let err = AtomwriteError::EditPairFailed {
            index: 2,
            total: 3,
            reason: "x".into(),
            pair_results: Box::new(vec![]),
            best_candidate: None,
        };
        assert_eq!(err.error_code(), "MATCH_FAILED");
        assert_eq!(err.exit_code(), 65);
        let json = ErrorJson::from_error(&err);
        assert_eq!(json.code, "MATCH_FAILED");
        assert_eq!(json.exit, 65);
    }

    #[test]
    fn from_io_at_keeps_denied_and_missing_apart_and_names_the_path() {
        let path = std::path::Path::new("/ws/f.txt");
        let denied = AtomwriteError::from_io_at(path, std::io::ErrorKind::PermissionDenied.into());
        assert!(matches!(denied, AtomwriteError::PermissionDenied { .. }));
        let missing = AtomwriteError::from_io_at(path, std::io::ErrorKind::NotFound.into());
        assert!(matches!(missing, AtomwriteError::NotFound { .. }));
        let other = AtomwriteError::from_io_at(path, std::io::Error::other("boom"));
        assert_eq!(other.error_code(), "IO_ERROR");
        assert!(other.to_string().contains("/ws/f.txt"), "{other}");
    }

    #[test]
    fn from_walk_error_needs_a_path_and_an_io_cause() {
        let denied = ignore::Error::WithPath {
            path: PathBuf::from("/ws/dir"),
            err: Box::new(ignore::Error::Io(
                std::io::ErrorKind::PermissionDenied.into(),
            )),
        };
        let (path, err) = AtomwriteError::from_walk_error(&denied).expect("classified");
        assert_eq!(path, PathBuf::from("/ws/dir"));
        assert!(matches!(err, AtomwriteError::PermissionDenied { .. }));

        let looped = ignore::Error::Loop {
            ancestor: PathBuf::from("/ws"),
            child: PathBuf::from("/ws/loop"),
        };
        let (path, err) = AtomwriteError::from_walk_error(&looped).expect("classified");
        assert_eq!(path, PathBuf::from("/ws/loop"));
        assert_eq!(err.error_code(), "IO_ERROR");

        let glob = ignore::Error::Glob {
            glob: None,
            err: "bad".into(),
        };
        assert!(AtomwriteError::from_walk_error(&glob).is_none());
    }
}
