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
struct DoctorReport {
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
                pool.map(|n| n.to_string()).unwrap_or_else(|| "unset".into()),
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
                    "ast feature disabled — install with --features ast for AST commands".to_string()
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
        Box::new(|| {
            check(
                "mcp_policy",
                "pass",
                "MCP forbidden; use CLI + NDJSON (agent-surface / commands)",
            )
        }),
        Box::new(move || {
            let resolved = crate::locale::resolved_state();
            check(
                "locale",
                "pass",
                format!(
                    "resolved={} source={} override={:?} (NDJSON codes/Display English; suggestions locale-aware)",
                    resolved.map(|s| s.idioma.as_str()).unwrap_or("en"),
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
    let has_fail = checks.iter().any(|c| c.status == "fail");
    let has_warn = checks.iter().any(|c| c.status == "warn");
    let ok = if args.strict {
        !has_fail && !has_warn
    } else {
        !has_fail
    };
    writer.write_event(&DoctorReport {
        r#type: "doctor_report",
        version: env!("CARGO_PKG_VERSION").to_string(),
        target: env!("TARGET").to_string(),
        ok,
        checks,
    })?;

    if args.strict && !ok {
        anyhow::bail!("doctor: one or more checks failed or warned (strict mode)");
    }
    Ok(())
}

fn workspace_checks(global: &GlobalArgs) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    match global.resolve_workspace() {
        Ok(ws) => {
            // Independent I/O against the same root → nested rayon::join.
            let ws_a = ws.clone();
            let ws_b = ws.clone();
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
                                    cfg.defaults.backup, cfg.defaults.retention
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

fn is_writable_dir(dir: &Path) -> std::io::Result<bool> {
    let probe = dir.join(".atomwrite-doctor-probe");
    match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Ok(false),
        Err(e) => Err(e),
    }
}

