#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_returns_defaults() {
        let config = merge_layers(None, None).unwrap();
        assert!(config.defaults.backup);
        assert_eq!(config.defaults.retention, 5);
        assert_eq!(config.fuzzy.mode, "auto");
    }

    #[test]
    fn local_config_overrides_defaults() {
        let config = load_local("[defaults]\nbackup = false\nretention = 3\n").unwrap();
        assert!(!config.defaults.backup);
        assert_eq!(config.defaults.retention, 3);
    }

    /// Drive the exact layer merge the binary runs, with no global layer.
    ///
    /// The unit tests must not inherit the operator's `~/.config/atomwrite`
    /// file: pass `None` for the global layer and parse only the body under
    /// test.
    fn load_local(body: &str) -> Result<AtomwriteConfig, AtomwriteError> {
        let path = Path::new(crate::constants::WORKSPACE_CONFIG_FILE);
        merge_layers(None, Some(parse_layer(path, body)?))
    }

    #[test]
    fn malformed_toml_fails_closed() {
        let err = load_local("not valid { toml").unwrap_err();
        assert!(
            matches!(err, AtomwriteError::ConfigInvalid { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn unknown_key_fails_closed_and_names_the_file() {
        let err = load_local("[defaults]\nbogus = 1\n").unwrap_err();
        let AtomwriteError::ConfigInvalid { reason } = err else {
            panic!("expected ConfigInvalid");
        };
        assert!(reason.contains(".atomwrite.toml"), "{reason}");
        assert!(reason.contains("bogus"), "{reason}");
    }

    #[test]
    fn unknown_top_level_section_fails_closed() {
        assert!(load_local("[nope]\nx = 1\n").is_err());
    }

    #[test]
    fn future_version_fails_closed() {
        let future = crate::constants::CONFIG_SCHEMA_VERSION + 1;
        let err = load_local(&format!("version = {future}\n")).unwrap_err();
        assert!(
            matches!(err, AtomwriteError::ConfigInvalid { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn supported_and_absent_version_load() {
        let current = crate::constants::CONFIG_SCHEMA_VERSION;
        assert_eq!(
            load_local(&format!("version = {current}\n"))
                .unwrap()
                .version,
            current
        );
        assert_eq!(
            load_local("[defaults]\nbackup = false\n").unwrap().version,
            0
        );
    }

    #[test]
    fn log_sink_is_off_by_default_and_rotation_is_validated() {
        let cfg = AtomwriteConfig::default();
        assert!(!cfg.log.file);
        assert!(validate_log(&cfg.log).is_ok());
        let on = load_local("[log]\nfile = true\nrotation = \"hourly\"\n").unwrap();
        assert!(on.log.file && validate_log(&on.log).is_ok());
        let bad = load_local("[log]\nrotation = \"weekly\"\n").unwrap();
        assert!(matches!(
            validate_log(&bad.log),
            Err(AtomwriteError::ConfigInvalid { .. })
        ));
    }

    #[test]
    fn fuzzy_max_content_bytes_defaults_to_the_named_constant() {
        let cfg = AtomwriteConfig::default();
        assert_eq!(
            cfg.fuzzy.max_content_bytes,
            crate::constants::FUZZY_MAX_CONTENT_BYTES
        );
        let partial = load_local("[fuzzy]\nthreshold = 0.9\n").unwrap();
        assert_eq!(
            partial.fuzzy.max_content_bytes,
            crate::constants::FUZZY_MAX_CONTENT_BYTES
        );
        assert!(validate_fuzzy(&cfg.fuzzy).is_ok());
    }

    #[test]
    fn fuzzy_max_content_bytes_is_read_from_toml() {
        let cfg = load_local("[fuzzy]\nmax_content_bytes = 1000000\n").unwrap();
        assert_eq!(cfg.fuzzy.max_content_bytes, 1_000_000);
        assert!(validate_fuzzy(&cfg.fuzzy).is_ok());
        let opts = crate::fuzzy::match_opts_from_section(FuzzyMode::Auto, None, &cfg.fuzzy, false);
        assert_eq!(opts.max_content_bytes, 1_000_000);
    }

    #[test]
    fn fuzzy_max_content_bytes_zero_is_rejected() {
        let cfg = load_local("[fuzzy]\nmax_content_bytes = 0\n").unwrap();
        let err = validate_fuzzy(&cfg.fuzzy).unwrap_err();
        match err {
            AtomwriteError::InvalidInput { reason } => {
                assert!(reason.contains("max_content_bytes"), "{reason}");
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn fuzzy_unknown_key_is_still_rejected() {
        assert!(load_local("[fuzzy]\nmax_content_byte = 1\n").is_err());
    }

    #[test]
    fn explicit_path_takes_precedence() {
        let dir = tempdir().unwrap();
        let custom = dir.path().join("custom.toml");
        std::fs::write(&custom, "[fuzzy]\nthreshold = 0.95\n").unwrap();
        let config = load_config(dir.path(), Some(&custom)).unwrap();
        assert!((config.fuzzy.threshold - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn explicit_path_missing_is_hard_error() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope.toml");
        let err = load_config(dir.path(), Some(&missing)).unwrap_err();
        assert!(
            matches!(err, AtomwriteError::ConfigInvalid { .. }),
            "expected ConfigInvalid, got {err:?}"
        );
    }

    #[test]
    fn merge_overlays_tables_key_by_key() {
        let base: toml::Table =
            toml::from_str("[fuzzy]\nthreshold = 0.8\nmax_content_bytes = 5\n").unwrap();
        let over: toml::Table =
            toml::from_str("[fuzzy]\nmax_content_bytes = 9\n[lock]\nacquire_timeout_ms = 700\n")
                .unwrap();
        let merged: AtomwriteConfig = toml::Value::Table(merge_tables(base, over))
            .try_into()
            .unwrap();
        assert!(
            (merged.fuzzy.threshold - 0.8).abs() < f64::EPSILON,
            "kept from base"
        );
        assert_eq!(merged.fuzzy.max_content_bytes, 9, "the overlay wins");
        assert_eq!(
            merged.lock.acquire_timeout_ms, 700,
            "a new section is added"
        );
    }

    #[test]
    fn tuning_defaults_are_the_named_constants() {
        let t = Tuning::default();
        assert_eq!(t.max_threads, crate::constants::MAX_AUTO_THREADS);
        assert_eq!(
            t.ram_fraction_percent,
            crate::constants::RAM_FRACTION_PERCENT
        );
        assert_eq!(
            t.lock_acquire_timeout_ms,
            crate::constants::LOCK_ACQUIRE_TIMEOUT_MS
        );
        assert_eq!(
            t.search_max_inflight_bytes,
            crate::constants::SEARCH_MAX_INFLIGHT_BYTES
        );
        assert_eq!(t.retry_base_ms, None);
        assert_eq!(t.retry_max_ms, None);
        assert_eq!(t.retry_max_elapsed_ms, None);
        assert!(validate_all(&AtomwriteConfig::default()).is_ok());
    }

    #[test]
    fn tuning_reads_every_new_key_from_toml() {
        let cfg = load_local(
            "[concurrency]\nmax_threads = 3\nram_fraction_percent = 20\n\
             [lock]\nacquire_timeout_ms = 900\n\
             [retry]\nbase_ms = 5\nmax_ms = 50\nmax_elapsed_ms = 700\n\
             [search]\nmax_inflight_bytes = 8388608\n\
             [extract]\nmax_sample_rows = 0\nmax_text_chars = 12\nmax_sheets = 2\nmax_slides = 4\n",
        )
        .unwrap();
        assert!(validate_all(&cfg).is_ok());
        let t = Tuning::from_config(&cfg);
        assert_eq!((t.max_threads, t.ram_fraction_percent), (3, 20));
        assert_eq!(t.lock_acquire_timeout_ms, 900);
        assert_eq!(
            (t.retry_base_ms, t.retry_max_ms, t.retry_max_elapsed_ms),
            (Some(5), Some(50), Some(700))
        );
        assert_eq!(t.search_max_inflight_bytes, 8_388_608);
        assert_eq!(
            (t.extract_max_sample_rows, t.extract_max_text_chars),
            (0, 12)
        );
        assert_eq!((t.extract_max_sheets, t.extract_max_slides), (2, 4));
    }

    #[test]
    fn tuning_values_out_of_range_are_rejected() {
        for body in [
            "[concurrency]\nmax_threads = 0\n",
            "[concurrency]\nram_fraction_percent = 0\n",
            "[concurrency]\nram_fraction_percent = 101\n",
            "[lock]\nacquire_timeout_ms = 1\n",
            "[retry]\nbase_ms = 0\n",
            "[retry]\nmax_ms = 0\n",
            "[retry]\nmax_elapsed_ms = 0\n",
            "[search]\nmax_inflight_bytes = 10\n",
            // Below the 4 MiB minimum (1 MiB starved the search pipeline).
            "[search]\nmax_inflight_bytes = 1048576\n",
            "[extract]\nmax_text_chars = 0\n",
            "[extract]\nmax_sheets = 0\n",
            "[extract]\nmax_slides = 0\n",
            "[concurrency]\nmax_threads = 1000\n",
            "[lock]\nacquire_timeout_ms = 99999999999\n",
            "[retry]\nbase_ms = 100000\n",
            "[retry]\nmax_elapsed_ms = 99999999999\n",
            "[search]\nmax_inflight_bytes = 99999999999\n",
            "[extract]\nmax_sample_rows = 99999999\n",
            "[extract]\nmax_text_chars = 99999999999\n",
            "[extract]\nmax_sheets = 99999999\n",
            "[extract]\nmax_slides = 99999999\n",
        ] {
            let cfg = load_local(body).unwrap();
            let err = validate_all(&cfg).expect_err(body);
            assert!(
                matches!(err, AtomwriteError::InvalidInput { .. }),
                "{body}: {err:?}"
            );
        }
    }

    #[test]
    fn retry_keys_must_be_coherent_with_each_other() {
        // A key left unset counts with the value it has at run time.
        for body in [
            "[retry]\nbase_ms = 10\nmax_ms = 5\n",
            "[retry]\nbase_ms = 1000\n",
            "[retry]\nbase_ms = 300\nmax_elapsed_ms = 100\n",
            "[retry]\nmax_ms = 10\n",
        ] {
            let cfg = load_local(body).unwrap();
            let err = validate_all(&cfg).expect_err(body);
            assert!(
                matches!(err, AtomwriteError::InvalidInput { .. }),
                "{body}: {err:?}"
            );
        }
        for body in [
            "[retry]\nbase_ms = 50\nmax_ms = 50\n",
            "[retry]\nbase_ms = 5\nmax_ms = 5\nmax_elapsed_ms = 5\n",
            "[retry]\nmax_ms = 400\n",
        ] {
            let cfg = load_local(body).unwrap();
            assert!(validate_all(&cfg).is_ok(), "{body}");
        }
    }

    #[test]
    fn the_largest_accepted_values_are_the_named_ceilings() {
        let c = crate::constants::CONCURRENCY_MAX_THREADS_CEILING;
        let body = format!(
            "[concurrency]\nmax_threads = {c}\n[lock]\nacquire_timeout_ms = {}\n\
             [search]\nmax_inflight_bytes = {}\n",
            crate::constants::LOCK_MAX_ACQUIRE_TIMEOUT_MS,
            crate::constants::SEARCH_MAX_INFLIGHT_CEILING_BYTES
        );
        assert!(validate_all(&load_local(&body).unwrap()).is_ok());
        let over = format!("[concurrency]\nmax_threads = {}\n", c + 1);
        assert!(validate_all(&load_local(&over).unwrap()).is_err());
    }

    #[test]
    fn tuning_unknown_key_is_still_rejected() {
        assert!(load_local("[retry]\nbase = 5\n").is_err());
        assert!(load_local("[concurrency]\nthreads = 5\n").is_err());
    }

    #[test]
    fn retry_overrides_change_only_what_is_set() {
        use crate::retry::RetryPolicy;
        let policy = RetryPolicy::new(50, 400, 2_000, 8);
        let unset = Tuning::default();
        assert_eq!(policy.with_shape_overrides(&unset), policy);
        assert_eq!(policy.with_budget_override(&unset), policy);

        let tuned = Tuning {
            retry_base_ms: Some(7),
            retry_max_ms: Some(30),
            retry_max_elapsed_ms: Some(900),
            ..Tuning::default()
        };
        let shaped = policy.with_shape_overrides(&tuned);
        assert_eq!(shaped.ceiling(0), std::time::Duration::from_millis(7));
        assert_eq!(shaped.ceiling(10), std::time::Duration::from_millis(30));
        assert_ne!(shaped, policy.with_budget_override(&tuned));
        assert_eq!(
            policy.with_budget_override(&tuned),
            RetryPolicy::new(50, 400, 900, 8)
        );
    }
}
