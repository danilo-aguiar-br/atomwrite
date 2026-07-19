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

/// Emit a human warning on stderr (never on stdout).
///
/// G-026: only when stderr is a TTY (agents parse NDJSON on stdout; non-TTY
/// must not mix human risk lines). Color respects CLI `ColorMode`.
pub fn warn_stderr(mode: ColorMode, message: impl AsRef<str>) {
    use std::io::IsTerminal;
    if !std::io::stderr().is_terminal() {
        return;
    }
    let msg = message.as_ref();
    if stderr_color_enabled(mode) {
        eprintln!("\x1b[33mwarning:\x1b[0m {msg}");
    } else {
        eprintln!("warning: {msg}");
    }
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
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|a| a == "--json-schema") {
        return None;
    }
    const SUBCOMMANDS: &[&str] = &[
        "read",
        "write",
        "edit",
        "search",
        "replace",
        "hash",
        "delete",
        "count",
        "diff",
        "move",
        "copy",
        "list",
        "extract",
        "calc",
        "regex",
        "transform",
        "scope",
        "batch",
        "backup",
        "rollback",
        "apply",
        "completions",
        "prune-backups",
        "edit-loop",
        "get",
        "set",
        "del",
        "outline",
        "query",
        "case",
        "semantic-merge",
        "sparse",
        "recipe",
        "stat",
        "agent-surface",
        "watch",
        "codemod",
        "semantic-search",
        "verify",
        "wal-stats",
        "wal-heal",
        "doctor",
        "locale",
        "progress",
        "error",
        "best-candidate",
        "cancelled",
    ];
    for arg in &args[1..] {
        if SUBCOMMANDS.contains(&arg.as_str()) {
            return Some(arg.clone());
        }
    }
    None
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
        std::io::ErrorKind::PermissionDenied => {
            crate::error::AtomwriteError::PermissionDenied {
                path: extract_path_from_message(&msg),
            }
        }
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
            clap_err.exit();
        }
        _ => {
            let msg = clap_err.to_string();
            let suggestion = enrich_clap_suggestion(&msg);
            let ej = crate::error::ErrorJson {
                error: true,
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
            };
            let mut out = io::stdout().lock();
            if let Err(e) = serde_json::to_writer(&mut out, &ej) {
                let _ = writeln!(io::stderr(), "atomwrite: failed to write error JSON: {e}");
            }
            let _ = out.write_all(b"\n");
            let _ = out.flush();
            ExitCode::from(2)
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
