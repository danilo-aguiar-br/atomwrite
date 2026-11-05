/// Write each worker event to stdout in arrival order.
///
/// Returns early, without draining, once the shutdown flag is set. Files that
/// failed are not events of their own: they are tallied and reported by the
/// terminal summary.
fn drain_transform_events(
    rx: &crossbeam_channel::Receiver<TransformEvent>,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<FailureTally> {
    let mut failures = FailureTally::default();
    for event in rx {
        if shutdown.is_shutdown() {
            break;
        }

        match event {
            TransformEvent::Transformed {
                path,
                language,
                matches,
                bytes_before,
                bytes_after,
                checksum_before,
                checksum_after,
                elapsed_ms,
                risk_assessment,
                write_strategy,
                backup_skipped,
            } => {
                writer.write_event(&TransformResult {
                    r#type: "transformed",
                    path: path.display().to_string(),
                    language,
                    matches,
                    replacements: matches,
                    bytes_before,
                    bytes_after,
                    checksum_before,
                    checksum_after,
                    elapsed_ms,
                    risk_assessment,
                    write_strategy,
                    backup_skipped,
                })?;
            }
            TransformEvent::Error { path, error } => failures.record(&path, error),
        }
    }

    Ok(failures)
}

enum TransformEvent {
    Transformed {
        path: PathBuf,
        language: String,
        matches: u64,
        bytes_before: u64,
        bytes_after: u64,
        checksum_before: String,
        checksum_after: String,
        elapsed_ms: u64,
        risk_assessment: Option<crate::ndjson_types::WriteRiskAssessment>,
        write_strategy: &'static str,
        backup_skipped: Option<String>,
    },
    /// A file that could not be read, checked or written; the error travels whole.
    Error { path: PathBuf, error: anyhow::Error },
}

/// Multi-rule transform dispatcher (G44).
///
/// Reads rules from `--rules PATH` (YAML file) or `--inline-rules "YAML"`
/// and applies each rule in order. For each rule, the walker is
/// re-constructed with the rule's specific language filter. The summary
/// at the end aggregates counts across all rules.
fn cmd_transform_multi(
    args: &TransformArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let effective_backup = resolve_backup(&args.backup_opts, defaults);

    // Load the YAML rules.
    let rules: Vec<YamlRule> = if let Some(path) = &args.rules {
        load_rules_file(path, global.effective_max_filesize())?
    } else if let Some(inline) = &args.inline_rules {
        parse_rules(inline, "inline-rules")?
    } else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "no --rules or --inline-rules provided".into(),
        }
        .into());
    };

    if rules.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "rules manifest is empty (must contain at least one rule)".into(),
        }
        .into());
    }

    // For each rule, build a synthetic single-rule TransformArgs and run the
    // single-rule pipeline (walker, pattern match, atomic write) through
    // `transform_one_rule`, which does NOT write a terminal record: five rules
    // must end with one summary of the whole invocation, not five summaries
    // that each describe a part and claim to describe everything.
    let mut total_visited: u64 = 0;
    let mut total_matched: u64 = 0;
    let mut total_replacements: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_skipped: u64 = 0;
    let mut total_modified: u64 = 0;
    let mut first_failure: Option<anyhow::Error> = None;
    // Per-file failures of every rule, counted once for the whole invocation
    // so the terminal summary says the run was partial instead of dropping
    // what each rule already knew. Each rule walks the tree on its own, so the
    // same unreadable file fails once per rule; `failed_paths` keeps
    // `files_failed` counting distinct files and `errors[]` listing each once.
    let mut failed_paths: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // Failures a rule counted without publishing a path, which happens past the
    // `summary.errors_max_entries` cap. They are added to the distinct count.
    let mut unlisted_failed: u64 = 0;
    let mut failed_items: Vec<crate::ndjson_types::FileFailure> = Vec::new();

    // A root named in the invocation that does not exist is counted once, and
    // the rules walk only the roots that exist. The roots are resolved first so
    // the error names the same absolute path single-rule mode names.
    let resolved_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;
    let (present_paths, missing_roots) =
        crate::commands::path_resolution::split_missing_roots(resolved_paths);

    for rule in &rules {
        let synthetic_args = TransformArgs {
            strict: args.strict,
            pattern: Some(rule.pattern.clone()),
            rewrite: Some(rule.rewrite.clone()),
            language: Some(rule.language.clone()),
            paths: present_paths.clone(),
            include: args.include.clone(),
            exclude: args.exclude.clone(),
            dry_run: args.dry_run,
            rules: None,
            inline_rules: None,
            backup_opts: crate::cli_args::BackupOpts {
                backup: effective_backup.backup.then_some(true),
                no_backup: !effective_backup.backup,
                keep_backup: effective_backup.keep,
                retention: Some(effective_backup.retention),
            },
            verify_parse: args.verify_parse,
        };

        // Emit a "rule_begin" event so consumers can correlate subsequent
        // transform events with the rule that produced them.
        writer.write_event(&crate::ndjson_types::TransformRuleBegin {
            r#type: "rule_begin",
            id: rule.id.clone(),
            language: rule.language.clone(),
        })?;

        match transform_one_rule(&synthetic_args, global, writer, _shutdown, defaults, write_cfg) {
            Ok(run) => {
                total_visited = total_visited.saturating_add(run.summary.base.files_visited);
                total_matched = total_matched.saturating_add(run.summary.base.files_matched);
                total_skipped = total_skipped.saturating_add(run.summary.base.files_skipped.unwrap_or(0));
                total_modified = total_modified.saturating_add(run.summary.base.files_modified.unwrap_or(0));
                total_replacements =
                    total_replacements.saturating_add(run.summary.base.total_replacements.unwrap_or(0));
                total_bytes =
                    total_bytes.saturating_add(run.summary.base.total_bytes.unwrap_or(0));
                let rule_failed = run.summary.files_failed.unwrap_or(0);
                let rule_items = run.summary.errors.unwrap_or_default();
                let rule_listed = rule_items.len() as u64;
                for item in rule_items {
                    if failed_paths.insert(item.path.clone()) {
                        let room = crate::config::summary_errors_max_entries()
                            .saturating_sub(failed_items.len());
                        if room > 0 {
                            failed_items.push(item);
                        }
                    }
                }
                unlisted_failed =
                    unlisted_failed.saturating_add(rule_failed.saturating_sub(rule_listed));
                if let Some(err) = run.first_failure {
                    writer.write_event(&crate::ndjson_types::TransformRuleError {
                        r#type: "rule_error",
                        id: rule.id.clone(),
                        error: crate::output::error_message(&err),
                    })?;
                    if first_failure.is_none() {
                        first_failure = Some(err);
                    }
                }
            }
            Err(e) => {
                writer.write_event(&crate::ndjson_types::TransformRuleError {
                    r#type: "rule_error",
                    id: rule.id.clone(),
                    error: crate::output::error_message(&e),
                })?;
                // Continue with the next rule — partial success is OK in
                // multi-rule mode; the user can re-run individual rules.
                if first_failure.is_none() {
                    first_failure = Some(e);
                }
            }
        }
    }

    // A root the invocation named and never reached is counted once, exactly
    // as single-rule mode counts it, so both modes report the same field.
    let mut root_tally = FailureTally::default();
    root_tally.record_missing_roots(&missing_roots);
    let (roots_failed, roots_items, _) = root_tally.into_parts();
    let total_failed = (failed_paths.len() as u64)
        .saturating_add(unlisted_failed)
        .saturating_add(roots_failed);
    let room = crate::config::summary_errors_max_entries().saturating_sub(failed_items.len());
    failed_items.extend(roots_items.into_iter().take(room));

    let files_failed = (total_failed > 0).then_some(total_failed);
    let summary = crate::ndjson_types::FailureSummary {
        base: Summary {
            r#type: "summary",
            files_visited: total_visited,
            files_matched: total_matched,
            files_modified: Some(total_modified),
            files_skipped: Some(total_skipped),
            total_matches: Some(total_replacements),
            total_replacements: Some(total_replacements),
            total_bytes: Some(total_bytes),
            elapsed_ms: start.elapsed().as_millis() as u64,
            walker_stopped_early: crate::output::limit_reached().then_some(true),
            ..Summary::default()
        },
        files_failed,
        partial: crate::ndjson_types::partial_of(files_failed),
        errors: (!failed_items.is_empty()).then_some(failed_items),
    };

    // A root named in the invocation that does not exist outranks a rewrite
    // that landed, so the run fails even when it matched.
    if let Some(first_missing) = missing_roots.first() {
        let err = crate::error::AtomwriteError::NotFound {
            path: first_missing.clone(),
        };
        return Err(writer.publish_failure(&summary, err.into()));
    }

    // A rewrite that landed is an answer the run did produce, and the summary
    // names every file it could not read, check or write in `files_failed` and
    // `errors[]`. Failing the run would throw that answer away, which is the
    // rule single-rule mode already follows.
    if total_replacements > 0 {
        writer.write_terminal(&summary)?;
        return Ok(());
    }
    // With nothing rewritten the run has no answer: the first failure explains
    // why, and only a run that reached every file falls through to the summary.
    if let Some(err) = first_failure {
        return Err(writer.publish_failure(&summary, err));
    }
    writer.write_terminal(&summary)?;
    Ok(())
}

/// Read a rules manifest from disk and parse it.
///
/// # Errors
///
/// Returns [`crate::error::AtomwriteError::Io`] when the file cannot be read, and
/// [`crate::error::AtomwriteError::InvalidInput`] when its content is not a
/// valid manifest.
pub fn load_rules_file(
    path: &std::path::Path,
    max_size: u64,
) -> Result<Vec<YamlRule>> {
    let content = crate::file_io::read_file_string(path, max_size)
        .with_context(|| format!("cannot read rules file {}", path.display()))?;
    parse_rules(&content, &path.display().to_string())
}

/// Parse a rules manifest and report the field that is wrong, not just "invalid YAML".
///
/// A manifest whose YAML is well formed but whose schema is not is a different
/// mistake from a file with a broken quote, and the two need different answers:
/// `SerdeMissingField` and `SerdeUnknownField` are schema errors, so the message
/// names the schema and the field; anything else is a syntax error and the
/// message says so. Both keep the `INVALID_INPUT` exit code.
///
/// # Errors
///
/// Returns [`crate::error::AtomwriteError::InvalidInput`] naming either the
/// schema or the syntax as the failing layer.
pub fn parse_rules(text: &str, label: &str) -> Result<Vec<YamlRule>> {
    serde_saphyr::from_str(text).map_err(|error| {
        // A rules manifest can fail in two layers, and the agent needs to know
        // which one. `serde_saphyr` reports a schema violation either as its own
        // typed variant or, when the error crosses the parser boundary, as an
        // `ExternalMessage` whose text is the serde diagnostic verbatim — so the
        // message prefix is the only signal left on that path. Both are measured.
        let message = error.to_string();
        let schema = matches!(
            error,
            serde_saphyr::Error::SerdeMissingField { .. }
                | serde_saphyr::Error::SerdeUnknownField { .. }
                | serde_saphyr::Error::SerdeInvalidType { .. }
                | serde_saphyr::Error::SerdeInvalidValue { .. }
        ) || message.contains("missing field `")
            || message.contains("unknown field `")
            || message.contains("invalid type:")
            || message.contains("invalid value:");
        let layer = if schema {
            "rules manifest schema"
        } else {
            "rules manifest YAML syntax"
        };
        // Same short-line extraction `AtomwriteError::invalid` uses, without
        // the `what:` prefix it would leave empty here.
        let detail = message
            .lines()
            .rev()
            .map(str::trim)
            .find(|line| line.chars().any(char::is_alphabetic))
            .map_or(message.trim(), |line| line.trim_start_matches("error: "));
        crate::error::AtomwriteError::InvalidInput {
            reason: format!("{label}: invalid {layer}: {detail}"),
        }
        .into()
    })
}

/// The `id` of every rule in a manifest, in file order.
///
/// The ids come from the parsed manifest, so a flow-style sequence, a quoted
/// value or `id :` with a space before the colon all read the same. A rule
/// without `id` is named after its 1-based position, so the label is never
/// empty and two different rules never share one.
///
/// # Errors
///
/// Returns whatever [`load_rules_file`] returns.
pub fn rule_ids_from_file(path: &std::path::Path, max_size: u64) -> Result<Vec<String>> {
    let rules = load_rules_file(path, max_size)?;
    Ok(rules
        .iter()
        .enumerate()
        .map(|(index, rule)| {
            rule.id
                .clone()
                .unwrap_or_else(|| format!("rule-{}", index + 1))
        })
        .collect())
}
