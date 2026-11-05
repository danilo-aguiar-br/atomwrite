// SPDX-License-Identifier: MIT OR Apache-2.0

//! Environment diagnostic for agent hosts (`atomwrite doctor`).
//!
//! Emits a single NDJSON report so autonomous callers can verify workspace,
//! config, compiled features, and stream contract readiness without human UI.
//!
//! Workload: I/O-bound (stat workspace + config + feature flags + env probes).
//! Parallelism: independent checks fan out via `rayon::par_iter` after the
//! workspace-dependent block; results sorted by name for stable NDJSON.
//! Bound: process-wide rayon pool. Reports concurrency bound
//! (`effective_threads`, CPUs, RAM cap, configured pool) for operators.

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use clap::Args;
use rayon::prelude::*;
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::GlobalArgs;
use crate::concurrency::should_parallelize;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `doctor`.
#[derive(Args, Debug, Default)]
pub struct DoctorArgs {
    /// Fail (exit 1) if any check has status `fail`.
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Exit non-zero when any check fails")]
    pub strict: bool,
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct DoctorReport {
    r#type: &'static str,
    version: String,
    target: String,
    ok: bool,
    checks: Vec<DoctorCheck>,
}

#[derive(Serialize, JsonSchema, Clone)]
struct DoctorCheck {
    name: String,
    status: String,
    detail: String,
}

/// Diagnose environment and dependencies; write one NDJSON report to stdout.
///
/// # Errors
///
/// Returns `AtomwriteError::DoctorChecksFailed` under `--strict` when at least one check
/// reports `fail` or `warn`; the report itself is published as the one terminal event.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
/// Returns a serialization error when the report cannot be encoded as JSON.
#[tracing::instrument(skip_all, fields(command = "doctor"))]
pub fn cmd_doctor(
    args: &DoctorArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
    _defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    // Phase 1 — workspace-dependent checks (shared resolve; sequential unit).
    let mut checks = workspace_checks(global);

    // Phase 2 — independent checks (no shared mutability) → parallel fan-out.
    let lang_override = global.lang.clone();
    let threads = global.threads;
    let max_filesize = global.effective_max_filesize();
    let no_auto_heal = global.no_auto_heal;

    let independent: Vec<Box<dyn Fn() -> DoctorCheck + Send + Sync>> = vec![
        Box::new(move || {
            let cpus = crate::concurrency::available_cpus();
            let effective = crate::concurrency::effective_threads(threads);
            let ram = crate::concurrency::ram_concurrency_cap();
            let pool = crate::concurrency::configured_pool_size();
            let detail = format!(
                "cpus={cpus}, effective_threads={effective}, ram_cap={}, configured_pool={}, cli_threads={:?}",
                ram.map(|n| n.to_string()).unwrap_or_else(|| "n/a".into()),
                pool.map(|n| n.to_string())
                    .unwrap_or_else(|| "unset".into()),
                threads
            );
            check("concurrency_bound", "pass", detail)
        }),
        Box::new(|| {
            check(
                "feature_ast",
                if cfg!(feature = "ast") {
                    "pass"
                } else {
                    "warn"
                },
                if cfg!(feature = "ast") {
                    "ast feature enabled (scope/query/outline/transform)".to_string()
                } else {
                    "ast feature disabled — install with --features ast for AST commands"
                        .to_string()
                },
            )
        }),
        Box::new(|| {
            // A-006: opt-in long-running feature uses status `info` when off so
            // agents do not interpret `pass` as "watch is usable".
            // `info` never fails `--strict` (only fail/warn do).
            if cfg!(feature = "watch") {
                check(
                    "feature_watch",
                    "pass",
                    "watch feature enabled (long-running; not one-shot default)",
                )
            } else {
                check(
                    "feature_watch",
                    "info",
                    "watch disabled in this build (one-shot default; use --features watch)",
                )
            }
        }),
        Box::new(|| {
            // G-012 / A-017: offline Jaccard semantic-search is always in the binary
            // (empty Cargo feature removed; not a gate).
            check(
                "feature_semantic",
                "pass",
                "semantic-search offline Jaccard always available (no feature gate)",
            )
        }),
        Box::new(|| {
            let runtime = crate::env_detect::detect();
            check(
                "runtime_environment",
                "pass",
                crate::env_detect::summary(&runtime),
            )
        }),
        Box::new(|| {
            let runtime = crate::env_detect::detect();
            check(
                "platform_os",
                "pass",
                format!(
                    "os={} arch={} family={} target={}",
                    runtime.os, runtime.arch, runtime.family, runtime.target
                ),
            )
        }),
        Box::new(|| {
            if let Some(cfg_dir) = crate::storage::config_dir() {
                let via = if crate::storage::home_override().is_some() {
                    "XDG/ProjectDirs config"
                } else {
                    "ProjectDirs/XDG"
                };
                check(
                    "storage_config_dir",
                    "pass",
                    format!("config_dir={} via={via}", cfg_dir.display()),
                )
            } else {
                check(
                    "storage_config_dir",
                    "warn",
                    "cannot resolve config dir (no ProjectDirs / XDG home)",
                )
            }
        }),
        Box::new(|| {
            check(
                "platform_fsync",
                "pass",
                format!(
                    "file={} dir={} rename={}",
                    crate::platform::platform_fsync_name(),
                    crate::platform::platform_dir_fsync_name(),
                    crate::platform::platform_rename_method()
                ),
            )
        }),
        Box::new(|| {
            check(
                "stdout_contract",
                "pass",
                "structured NDJSON exclusively on stdout; logs on stderr",
            )
        }),
        Box::new(move || {
            let resolved = crate::locale::resolved_state();
            check(
                "locale",
                "pass",
                format!(
                    "resolved={} source={} override={:?} (NDJSON codes/Display English; suggestions locale-aware)",
                    resolved.map(|s| s.language.as_str()).unwrap_or("en"),
                    resolved.map(|s| s.source.as_str()).unwrap_or("default"),
                    lang_override.as_deref().unwrap_or("system/default")
                ),
            )
        }),
        Box::new(move || {
            check(
                "max_filesize",
                "pass",
                format!("effective max_filesize={max_filesize} bytes"),
            )
        }),
        Box::new(move || {
            check(
                "auto_heal",
                "pass",
                if no_auto_heal {
                    "startup wal-heal disabled (--no-auto-heal)".to_string()
                } else {
                    "startup wal-heal enabled (orphan cleanup on startup)".to_string()
                },
            )
        }),
    ];

    let mut independent_results: Vec<DoctorCheck> = if should_parallelize(independent.len()) {
        independent.par_iter().map(|job| job()).collect()
    } else {
        independent.iter().map(|job| job()).collect()
    };
    checks.append(&mut independent_results);

    // Stable NDJSON for agents (par_iter order is not sorted).
    checks.sort_by(|a, b| a.name.cmp(&b.name));

    // G-009 / A-006: non-strict `ok` is fail-only; strict also rejects `warn`.
    // Status `info` is informational only (never fails ok/strict).
    let n_fail = checks.iter().filter(|c| c.status == "fail").count() as u64;
    let n_warn = checks.iter().filter(|c| c.status == "warn").count() as u64;
    let has_fail = n_fail > 0;
    let has_warn = n_warn > 0;
    let ok = if args.strict {
        !has_fail && !has_warn
    } else {
        !has_fail
    };
    let report = DoctorReport {
        r#type: "doctor_report",
        version: env!("CARGO_PKG_VERSION").to_string(),
        target: env!("TARGET").to_string(),
        ok,
        checks,
    };

    if args.strict && !ok {
        // The report is the run's single terminal event: it carries the
        // failure detail itself, so no second error line follows.
        let failure = AtomwriteError::DoctorChecksFailed {
            failed: n_fail,
            warned: n_warn,
        };
        return Err(writer.publish_failure(&report, failure.into()));
    }
    writer.write_event(&report)?;
    Ok(())
}

fn workspace_checks(global: &GlobalArgs) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    match global.resolve_workspace() {
        Ok(ws) => {
            // Independent I/O against the same root → nested rayon::join.
            let ws_a = ws.clone();
            let ws_b = ws.clone();
            let root_for_docs = ws.clone();
            let ws_c = ws;
            let config_path = global.config.clone();
            let (exists, (writable, config)) = rayon::join(
                move || {
                    if ws_a.is_dir() {
                        check(
                            "workspace_exists",
                            "pass",
                            format!("workspace directory: {}", ws_a.display()),
                        )
                    } else {
                        check(
                            "workspace_exists",
                            "fail",
                            format!("workspace path is not a directory: {}", ws_a.display()),
                        )
                    }
                },
                move || {
                    rayon::join(
                        || match is_writable_dir(&ws_b) {
                            Ok(true) => check(
                                "workspace_writable",
                                "pass",
                                "can create temporary files in workspace",
                            ),
                            Ok(false) => check(
                                "workspace_writable",
                                "fail",
                                "cannot write temporary files in workspace",
                            ),
                            Err(e) => check(
                                "workspace_writable",
                                "warn",
                                format!("write probe error: {e}"),
                            ),
                        },
                        || match crate::config::load_config(&ws_c, config_path.as_deref()) {
                            Ok(cfg) => check(
                                "config_load",
                                "pass",
                                format!(
                                    "defaults.backup={}, retention={}",
                                    cfg.defaults.backup.map_or("unset", |on| if on {
                                        "true"
                                    } else {
                                        "false"
                                    }),
                                    cfg.defaults.retention
                                ),
                            ),
                            Err(e) => check("config_load", "fail", format!("config error: {e}")),
                        },
                    )
                },
            );
            checks.push(exists);
            checks.push(writable);
            checks.push(config);
            checks.push(docs_test_targets_check(&root_for_docs));
            checks.push(docs_config_keys_check(&root_for_docs));
            checks.push(grammar_cache_check());
        }
        Err(e) => {
            checks.push(check(
                "workspace_exists",
                "fail",
                format!("cannot resolve workspace: {e:#}"),
            ));
            checks.push(check(
                "workspace_writable",
                "fail",
                "skipped — workspace unresolved",
            ));
            checks.push(check(
                "config_load",
                "fail",
                "skipped — workspace unresolved",
            ));
        }
    }
    checks
}

fn check(name: &str, status: &str, detail: impl AsRef<str>) -> DoctorCheck {
    DoctorCheck {
        name: name.to_string(),
        status: status.to_string(),
        detail: detail.as_ref().to_string(),
    }
}

/// Languages the AST verbs need, deduplicated and sorted.
///
/// `outline`, `query` and `syntax_check` all resolve a language from the file
/// extension through [`crate::constants::EXTENSION_LANGUAGES`], so the cache
/// check asks about exactly that set and no other.
#[cfg(feature = "ast")]
fn needed_grammars() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = crate::constants::EXTENSION_LANGUAGES
        .iter()
        .map(|(_, language)| *language)
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// Report whether the grammars the AST verbs need are already on disk.
///
/// `has_language` answers "can this load without the network?" — it looks at
/// the cache directory and never downloads — so this check measures the exact
/// condition the operator needs to know BEFORE running `outline`, `query` or
/// `syntax_check` on a language for the first time.
///
/// A missing grammar is `info`, never `fail`: the pack downloads it on first
/// use and nothing on the host is broken. `info` also keeps `--strict` usable,
/// which a `warn` would not (strict rejects `warn`).
#[cfg(feature = "ast")]
fn grammar_cache_check() -> DoctorCheck {
    let cache_dir = match tree_sitter_language_pack::cache_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return check(
                "grammar_cache",
                "fail",
                format!("cannot resolve the grammar cache directory: {e}"),
            );
        }
    };
    let needed = needed_grammars();
    let missing: Vec<&str> = needed
        .iter()
        .copied()
        .filter(|name| !tree_sitter_language_pack::has_language(name))
        .collect();
    let present = needed.len() - missing.len();
    if missing.is_empty() {
        return check(
            "grammar_cache",
            "pass",
            format!(
                "{present}/{} grammars ready in {cache_dir}; outline, query and syntax_check need no network",
                needed.len()
            ),
        );
    }
    check(
        "grammar_cache",
        "info",
        format!(
            "{present}/{} grammars ready in {cache_dir}; missing: {} — the first use of each one downloads \
             from the network, so run the verb once per language while online",
            needed.len(),
            missing.join(", ")
        ),
    )
}

/// Same check when the crate was built without the `ast` feature.
#[cfg(not(feature = "ast"))]
fn grammar_cache_check() -> DoctorCheck {
    check(
        "grammar_cache",
        "skipped",
        "ast feature disabled; outline, query and syntax_check are unavailable",
    )
}

/// Count the integration test targets the build actually has.
///
/// One `tests/*.rs` file is one target, so `read_dir` on `tests/` is the same
/// count cargo reports, without a second tool.
fn count_test_targets(root: &Path) -> Option<usize> {
    let dir = root.join(crate::constants::TESTS_DIR);
    let entries = std::fs::read_dir(dir).ok()?;
    Some(
        entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
            .count(),
    )
}

/// Every live document that states the target count, with the number it states.
fn stated_target_counts(root: &Path) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    for name in crate::constants::DOCS_TARGET_COUNT_FILES {
        let path = root.join(name);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if let Some(n) = crate::commands::doctor_docs::target_count_in(line) {
                found.push(((*name).to_string(), n));
            }
        }
    }
    found
}

/// Report whether every stated test-target count matches the tree.
///
/// `skipped`, never `pass`, when the tree is absent: a directory that is not
/// the source tree proves nothing about the documents.
fn docs_test_targets_check(root: &Path) -> DoctorCheck {
    let Some(actual) = count_test_targets(root) else {
        return check(
            "docs_test_targets",
            "skipped",
            "no tests/ directory in the workspace; nothing to compare",
        );
    };
    let stated = stated_target_counts(root);
    if stated.is_empty() {
        return check(
            "docs_test_targets",
            "skipped",
            format!("no stated target count found; measured {actual}"),
        );
    }
    let stale: Vec<String> = stated
        .iter()
        .filter(|(_, n)| *n != actual)
        .map(|(f, n)| format!("{f} says {n}"))
        .collect();
    if stale.is_empty() {
        check(
            "docs_test_targets",
            "pass",
            format!(
                "{} statement(s) agree with the measured {actual} targets",
                stated.len()
            ),
        )
    } else {
        check(
            "docs_test_targets",
            "fail",
            format!(
                "measured {actual} targets; {} disagree: {}",
                stale.len(),
                stale.join(", ")
            ),
        )
    }
}

/// Report whether every stated config-key count matches the live table.
fn docs_config_keys_check(root: &Path) -> DoctorCheck {
    let actual = crate::commands::config_cmd::config_key_count();
    let mut stale = Vec::new();
    let mut stated = 0usize;
    for name in crate::constants::DOCS_KEY_COUNT_FILES {
        let path = root.join(name);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if let Some(n) = crate::commands::doctor_docs::key_count_in(line) {
                stated += 1;
                if n != actual {
                    stale.push(format!("{name} says {n}"));
                }
            }
        }
    }
    if stated == 0 {
        return check(
            "docs_config_keys",
            "skipped",
            format!("no stated key count found; the table has {actual}"),
        );
    }
    if stale.is_empty() {
        check(
            "docs_config_keys",
            "pass",
            format!("{stated} statement(s) agree with the {actual} live keys"),
        )
    } else {
        check(
            "docs_config_keys",
            "fail",
            format!(
                "the table has {actual} keys; {} disagree: {}",
                stale.len(),
                stale.join(", ")
            ),
        )
    }
}

fn is_writable_dir(dir: &Path) -> std::io::Result<bool> {
    let probe = dir.join(crate::constants::DOCTOR_PROBE_FILE);
    match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(_) => {
            crate::best_effort::best_effort(std::fs::remove_file(&probe), "remove doctor probe");
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Ok(false),
        Err(e) => Err(e),
    }
}
