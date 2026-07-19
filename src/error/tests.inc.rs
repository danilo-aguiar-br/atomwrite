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
                AtomwriteError::ExdevFallbackDisabled { path: p.clone() },
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
        // English assertion: pin UI locale (host may be pt-BR).
        crate::locale::set_locale_for_test(crate::locale::Idioma::En);
        let err = AtomwriteError::WorkspaceJail {
            path: PathBuf::from("/etc/passwd"),
            workspace: PathBuf::from("/home/user/project"),
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
        crate::locale::set_locale_for_test(crate::locale::Idioma::En);
        let err = AtomwriteError::WorkspaceJail {
            path: PathBuf::from("/etc/passwd"),
            workspace: PathBuf::from("/home/user/project"),
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
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Idioma::En);
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
        let _guard = crate::locale::LocaleTestGuard::set(crate::locale::Idioma::PtBr);
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
}
