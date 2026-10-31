// SPDX-License-Identifier: MIT OR Apache-2.0
// Included by runtime.rs (A-MONO-001).

/// Whether stderr human messages may use ANSI colors.
///
/// Agent-first: never color on non-TTY unless `ColorMode::Always`.
pub fn stderr_color_enabled(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Never => false,
        ColorMode::Always => true,
        ColorMode::Auto => std::io::IsTerminal::is_terminal(&std::io::stderr()),
    }
}

/// Emit a human warning on the diagnostic channel (never on the data channel).
///
/// The warning travels through the log subsystem, so it inherits two things it
/// used to ignore: the `-q` / `-qq` silence levels (an agent that asked for
/// quiet gets quiet) and the single stderr sink. ANSI is owned by the tracing
/// fmt layer, which is initialized with the same [`ColorMode`]; painting escape
/// codes here would double-color and would bypass the JSON file sink.
///
/// # Telling the two channels apart
///
/// A merged stream (`2>&1`) carries diagnostics and payload in one place, so the
/// split has to be decidable from the record itself. It is: every diagnostic
/// carries `level` and `timestamp`, and no payload has ever carried either.
/// A constant `channel` marker would be a stronger criterion, but
/// `tracing-subscriber` exposes no way to inject a constant field into its JSON
/// formatter, so declaring one here would publish a key nothing stamps — a
/// promise the reader cannot rely on, which is worse than no key at all.
/// The discriminator that really holds is guarded by
/// `diagnostic_records_are_distinguishable_from_payload` in the CLI tests.
///
/// `_mode` stays in the signature because callers already hold the resolved
/// color policy and one source of truth beats two.
pub fn warn_stderr(_mode: ColorMode, message: impl AsRef<str>) {
    tracing::warn!(
        target: "atomwrite::warning",
        warning = %message.as_ref(),
        "command completed with a warning"
    );
}

fn extract_clap_tip(msg: &str) -> Option<String> {
    for line in msg.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("tip:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Enrich clap parse errors with agent-oriented suggestions (edit hyphen values).
pub fn enrich_clap_suggestion(msg: &str) -> Option<String> {
    let clap_tip = extract_clap_tip(msg);
    let msg_lower = msg.to_ascii_lowercase();

    let mentions_edit_args = msg_lower.contains("--old")
        || msg_lower.contains("--new")
        || msg_lower.contains("--after-match")
        || msg_lower.contains("--before-match")
        || msg_lower.contains("--between");

    let is_edit_subcommand =
        msg.contains("Usage: atomwrite edit") || msg.contains("atomwrite edit ");

    let hyphen_value_error = msg.contains("wasn't expected")
        || msg.contains("unexpected argument")
        || (msg.contains("tip:") && msg.contains("'--'"));

    if mentions_edit_args || (hyphen_value_error && is_edit_subcommand) {
        let base = "For content with special characters (hyphens, quotes, shell metacharacters), \
                    use --old-file <PATH> and --new-file <PATH> to read content from files \
                    instead of CLI arguments. This bypasses shell expansion and argument \
                    parsing entirely.";
        return Some(match clap_tip {
            Some(tip) => format!("{base} (original clap tip: {tip})"),
            None => base.to_string(),
        });
    }

    clap_tip
}

/// Early `--json-schema` path: emit schema without full clap parse of subcommand args.
///
/// Justification vs rules "never parse `env::args` before Clap": agents need a
/// schema dump even when remaining argv would fail validation (e.g. missing
/// required path). Full parse still goes through Clap for all normal paths.
pub fn prescan_json_schema() -> Option<String> {
    // `args_os` + lossy: `env::args()` panics on a non-UTF-8 argument, and the
    // comparisons below only look for ASCII names.
    let args: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    if !args.iter().any(|a| a == "--json-schema") {
        return None;
    }
    for arg in &args[1..] {
        if crate::SUBCOMMANDS.contains(&arg.as_str()) {
            return Some(arg.clone());
        }
    }
    None
}

/// Early `--print-schema` path: emit a schema, or list the names, before clap.
///
/// Two forms, both answered without a subcommand, so discovery never depends on
/// a subcommand happening to parse:
/// - `--print-schema` alone lists every available schema name as one NDJSON
///   line per name, alphabetically ordered.
/// - `--print-schema NAME` emits that schema on the data channel.
///
/// The decision of WHETHER to answer belongs to
/// [`crate::plan_print_schema`], which sees the whole argv and therefore knows
/// whether the flag sat before or after the command token. This function only
/// writes.
///
/// An unknown name is reported on the data channel as an error envelope with a
/// machine code, exactly like every other usage error. It used to exit 1 with
/// an empty stdout, which a consumer cannot tell apart from a schema that
/// happened to be empty, and which contradicts this project's rule that the
/// data channel carries the failure and not silence.
///
/// # Errors
///
/// Returns an error only when writing to stdout fails; an unknown name is an
/// ordinary error envelope, not an I/O failure.
pub fn prescan_print_schema(argv: &[String]) -> anyhow::Result<Option<ExitCode>> {
    let name = match crate::plan_print_schema(argv) {
        crate::PrintSchemaPlan::Defer => return Ok(None),
        crate::PrintSchemaPlan::List => crate::SCHEMA_LIST_REQUEST.to_string(),
        crate::PrintSchemaPlan::Print(name) => name,
    };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if crate::emit_schema_by_name(&name, &mut out)? {
        return Ok(Some(ExitCode::from(0)));
    }
    let err = crate::error::AtomwriteError::InvalidInput {
        reason: format!(
            "unknown schema name {name:?}; run `--print-schema` with no name to list every \
             schema this build publishes"
        ),
    };
    let ctx = crate::error::ErrorContext::default();
    let shape = crate::output::OutputShape::default();
    let written = crate::output::write_error_json_shaped(&mut out, &err, None, &ctx, &shape)
        .and_then(|()| out.flush().map_err(anyhow::Error::from));
    Ok(Some(ExitCode::from(
        crate::best_effort::settle_error_report(written, err.exit_code()),
    )))
}

/// Map an `anyhow::Error` chain to an I/O error kind when present.
pub fn find_io_error(err: &anyhow::Error) -> Option<std::io::ErrorKind> {
    for cause in err.chain() {
        if let Some(io_err) = cause.downcast_ref::<std::io::Error>() {
            return Some(io_err.kind());
        }
    }
    None
}

/// Convert a detected I/O kind into a typed [`crate::error::AtomwriteError`].
pub fn io_to_atomwrite_error(
    kind: std::io::ErrorKind,
    err: &anyhow::Error,
) -> crate::error::AtomwriteError {
    let msg = format!("{err:#}");
    match kind {
        std::io::ErrorKind::PermissionDenied => crate::error::AtomwriteError::PermissionDenied {
            path: extract_path_from_message(&msg),
        },
        std::io::ErrorKind::NotFound => crate::error::AtomwriteError::NotFound {
            path: extract_path_from_message(&msg),
        },
        _ => crate::error::AtomwriteError::Io {
            source: std::io::Error::new(kind, msg),
        },
    }
}

fn extract_path_from_message(msg: &str) -> PathBuf {
    if let Some(start) = msg.find('/') {
        let rest = &msg[start..];
        let end = rest.find(':').unwrap_or(rest.len());
        return PathBuf::from(&rest[..end]);
    }
    PathBuf::from("unknown")
}

/// Handle clap parse failure: help/version exit via clap; other errors as NDJSON exit 2.
pub fn handle_clap_parse_error(clap_err: clap::Error) -> ExitCode {
    use clap::error::ErrorKind;
    match clap_err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            // `Error::exit` ignores a failed write, so a closed reader looked
            // like success; print here and settle the status instead.
            let written = clap_err.print().map_err(anyhow::Error::from);
            ExitCode::from(crate::best_effort::settle_error_report(
                written,
                clap_err.exit_code() as u8,
            ))
        }
        _ => {
            let msg = clap_err.to_string();
            let suggestion = enrich_clap_suggestion(&msg);
            let ej = crate::error::ErrorJson {
                error: true,
                ok: false,
                code: "ARGUMENT_PARSE_ERROR",
                exit: 2,
                message: msg,
                path: None,
                error_class: crate::error::ErrorClass::Permanent.as_str(),
                retryable: false,
                suggestion,
                workspace: None,
                failed_pair_index: None,
                pairs_total: None,
                pair_results: None,
                best_candidate: None,
                candidates: None,
                match_count: None,
                similar_paths: None,
                // v0.1.37 envelope fields — a clap parse error carries none of them.
                conflict_count: None,
                missing_flag: None,
                pairs_unmatched: None,
                pairs_applied: None,
                suggestion_lang: crate::locale::resolved_language().as_str(),
            };
            let mut out = io::stdout().lock();
            // `io::Error::from` keeps the original kind of a failed write, so a
            // closed reader is still recognised as a broken pipe.
            let written = serde_json::to_writer(&mut out, &ej)
                .map_err(io::Error::from)
                .and_then(|()| out.write_all(b"\n"))
                .and_then(|()| out.flush())
                .map_err(anyhow::Error::from);
            ExitCode::from(crate::best_effort::settle_error_report(written, 2))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_log_level_matrix() {
        assert_eq!(cli_log_level(0, 0), "warn");
        assert_eq!(cli_log_level(1, 0), "info");
        assert_eq!(cli_log_level(2, 0), "debug");
        assert_eq!(cli_log_level(3, 0), "trace");
        assert_eq!(cli_log_level(0, 1), "error");
        assert_eq!(cli_log_level(0, 2), "off");
        // verbose wins over quiet when both set
        assert_eq!(cli_log_level(1, 1), "info");
    }

    #[test]
    fn resolve_log_format_matrix() {
        // G-007: format is file_sink-driven only (no env override).
        assert_eq!(resolve_log_format(false), LogFormat::Compact);
        assert_eq!(resolve_log_format(true), LogFormat::Json);
    }

    #[test]
    fn env_filter_parses_target_directive() {
        let f = EnvFilter::builder()
            .parse("warn,atomwrite=info")
            .expect("parse directive");
        let s = f.to_string();
        assert!(
            s.contains("warn") || s.contains("info") || !s.is_empty(),
            "unexpected filter display: {s}"
        );
    }
}
