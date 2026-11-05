// SPDX-License-Identifier: MIT OR Apache-2.0

//! Atomic file creation and overwrite from stdin content.
//! Workload: I/O-bound (stdin read + atomic write).
//! Parallelism: none — single target file.

use std::io::{Read, Write};
use std::time::Instant;

use anyhow::{Context, Result};

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::{GlobalArgs, WriteArgs};
use crate::commands::{StdinCapOrigin, read_stdin_content_cancellable, resolve_backup};
use crate::constants::{PERCENT_SCALE, WRITE_RISK_HIGH_PERCENT, WRITE_RISK_MEDIUM_PERCENT};
use crate::error::AtomwriteError;
use crate::ndjson_types::WriteOutput;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Line-ending mode in effect: the flag when it names one, else the config.
///
/// `--line-ending` defaults to `auto`, so a value of `auto` cannot tell "the
/// caller asked for auto" from "the caller said nothing". The config key
/// `defaults.line_ending` is only consulted in the second case, and `auto`
/// there means the same auto-detection the flag default asks for.
fn effective_line_ending(
    args: &WriteArgs,
    defaults: &crate::config::DefaultsSection,
) -> crate::line_endings::LineEnding {
    use clap::ValueEnum;
    if args.line_ending != crate::line_endings::LineEnding::Auto {
        return args.line_ending;
    }
    crate::line_endings::LineEnding::from_str(&defaults.line_ending, true)
        .unwrap_or(crate::line_endings::LineEnding::Auto)
}

/// Create or overwrite a file atomically from stdin content.
///
/// The target is resolved against the workspace once, up front, so
/// append/prepend, line-ending auto-detection, and `--expect-checksum`
/// operate on the same path identity as the final atomic write. Before
/// v0.1.15 these pre-steps used the raw CLI path relative to the CWD,
/// which truncated appends and skipped checksum verification whenever
/// the CWD differed from the workspace (G118, CWE-367).
///
/// # Errors
///
/// Returns `AtomwriteError::NotFound` if reading stdin fails.
/// Returns `AtomwriteError::WorkspaceJail` if the path escapes the workspace.
/// Returns `AtomwriteError::Io` if writing the file fails.
/// Returns `AtomwriteError::StateDrift` if `--checksum` is set and the expected hash does not match.
#[tracing::instrument(skip_all, fields(command = "write"))]
pub fn cmd_write(
    args: &WriteArgs,
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
    write_cfg: &crate::config::WriteSection,
) -> Result<()> {
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;
    let resolved = crate::path_safety::validate_path(&args.target, &workspace)?;
    let resolved_backup = resolve_backup(&args.backup_opts, defaults);
    let effective_backup = resolved_backup.backup;

    // The global `--max-filesize` bounds the payload too, exactly like it bounds
    // the target: a verb-local `--max-size` only narrows it further, and the two
    // never disagree. Checked before stdin is consumed so a rejected overwrite
    // does not drain the pipe.
    // The config key `defaults.max_filesize` fills the cap the flag did not
    // name. The flag wins because it is the explicit target designation; the
    // config key exists so an operator can lower the ceiling once, and a key
    // nothing reads is a promise the product does not keep.
    let global_cap = match global.max_filesize {
        Some(n) => n,
        None => defaults.max_filesize.max(1),
    };
    if let Ok(meta) = std::fs::metadata(&resolved) {
        if meta.is_file() && meta.len() > global_cap {
            return Err(AtomwriteError::FileTooLarge {
                path: resolved,
                size: meta.len(),
                max_size: global_cap,
            }
            .into());
        }
    }
    let (stdin_cap, cap_origin) = match args.max_size {
        Some(local) => (Some(local), StdinCapOrigin::LocalMaxSize),
        None => (Some(global_cap), StdinCapOrigin::GlobalMaxFilesize),
    };

    let stdin_bytes_read;
    let mut content = {
        let (buf, n) = read_stdin_content_cancellable(
            stdin,
            stdin_cap,
            cap_origin,
            args.allow_empty_stdin,
            Some(shutdown),
        )?;
        stdin_bytes_read = n;
        buf
    };

    if shutdown.is_shutdown() {
        return Err(crate::signal::cancelled_error("interrupted before write").into());
    }

    // A1: append/prepend compute the new content from the file on disk, so lock
    // BEFORE reading it (stdin was read above, never under the lock).
    let _update_lock = if (args.append || args.prepend)
        && resolved.parent().is_some_and(std::path::Path::is_dir)
    {
        Some(crate::lock::acquire_for_update(&resolved)?)
    } else {
        None
    };
    if args.append || args.prepend {
        content = handle_append_prepend(
            &resolved,
            &content,
            args.append,
            global.effective_max_filesize(),
            args.allow_empty_stdin,
        )?;
    }

    content = normalize_line_endings(&content, effective_line_ending(args, defaults), &resolved);

    // G120 L3: cross-validation. When the caller combines
    // `--append` (or `--prepend`) with `--expect-checksum` and the stdin
    // is empty, the situation is ambiguous: the checksum is for the
    // pre-mutation state but the empty append is effectively a no-op.
    // We emit a structured warning on stderr so the agent and operator
    // can audit the decision, but DO NOT abort — the user explicitly
    // opted into empty stdin with `--allow-empty-stdin`.
    if let Some(ref expected) = args.expect_checksum {
        if stdin_bytes_read == 0 && (args.append || args.prepend) {
            if args.no_checksum_when_empty {
                tracing::warn!(
                    path = %resolved.display(),
                    expected = %expected,
                    "G120 L3: --append/--prepend + --expect-checksum with empty stdin; \
                     skipping checksum verification per --no-checksum-when-empty"
                );
            } else {
                // Default: still verify against the pre-mutation state.
                // If the pre-mutation file exists and matches, this
                // passes and the empty append is a no-op. If it does
                // not exist, verify_checksum's `if !target.exists()`
                // short-circuit returns Ok — the same legacy behaviour
                // as pre-v0.1.15, but now explicitly logged.
                tracing::info!(
                    path = %resolved.display(),
                    expected = %expected,
                    "G120 L3: --append/--prepend + --expect-checksum with empty stdin; \
                     verifying pre-mutation state. Pass --no-checksum-when-empty to skip."
                );
                verify_checksum(&resolved, expected, global.effective_max_filesize())?;
            }
        } else {
            verify_checksum(&resolved, expected, global.effective_max_filesize())?;
        }
    }

    // A-WRITE-001: large-file overwrite is default-deny (not opt-in via --confirm).
    // One-shot: never read Y/N from stdin (collides with payload). Require --ack-overwrite
    // when the existing target exceeds XDG [write].confirm_large_bytes.
    if resolved.exists() {
        let original_size = std::fs::metadata(&resolved).map(|m| m.len()).unwrap_or(0);
        crate::commands::write_guards::check_replacement_size(
            &resolved,
            original_size,
            content.len() as u64,
            write_cfg,
            args.ack_overwrite,
            args.allow_shrink,
        )?;
    }

    let target_str = args.target.display().to_string();

    if args.dry_run {
        // The plan carries the risk the write would report: the dry-run is the
        // run an agent reads before deciding to write at all.
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "write".into(),
            path: target_str,
            would_modify: true,
            details: Some(format!("{} bytes from stdin", content.len())),
            risk_assessment: crate::commands::write_guards::assess_content_risk(
                &content,
                std::fs::metadata(&resolved).map(|m| m.len()).unwrap_or(0),
                content.len() as u64,
                write_cfg,
            ),
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    // GAP-2026-011 L2 — require-backup guard
    // GAP-2026-033: check no_backup explicitly since backup has default_value_t=true
    if args.require_backup && (args.backup_opts.no_backup || !effective_backup) && resolved.exists()
    {
        return Err(AtomwriteError::InvalidInput {
            reason: "--require-backup is set but --no-backup disables backup; remove --no-backup or remove --require-backup".into(),
        }
        .into());
    }

    // GAP-2026-011 L5 — auto-rotate guard (age from XDG `[write].auto_rotate_max_age_secs`)
    let auto_rotate_active = args.auto_rotate
        && effective_backup
        && resolved.exists()
        && std::fs::metadata(&resolved)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| {
                age < std::time::Duration::from_secs(write_cfg.auto_rotate_max_age_secs)
            });

    // GAP-2026-011 L1 + L6 — size guard and risk_assessment diagnostics
    // GAP-2026-024: skip size risk for append/prepend (never causes data loss)
    // B-013: content-pattern risk applies to create/overwrite payloads (not product telemetry).
    let new_bytes = content.len() as u64;
    let original_bytes = if resolved.exists() {
        std::fs::metadata(&resolved).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    let mut risk_assessment = None;

    if let Some(content_risk) = crate::commands::write_guards::assess_content_risk(
        &content,
        original_bytes,
        new_bytes,
        write_cfg,
    ) {
        crate::runtime::warn_stderr(
            global.color_mode(),
            format!(
                "write content risk {} (guard={})",
                content_risk.risk_level, content_risk.guard_triggered
            ),
        );
        risk_assessment = Some(content_risk);
    }

    if risk_assessment.is_none() && resolved.exists() && !args.append && !args.prepend {
        let original = original_bytes;
        if original > 0 {
            let delta_pct = ((new_bytes.abs_diff(original)) * PERCENT_SCALE / original) as u32;
            if delta_pct >= u32::from(args.risk_threshold) {
                let level = if delta_pct >= WRITE_RISK_HIGH_PERCENT {
                    "high"
                } else if delta_pct >= WRITE_RISK_MEDIUM_PERCENT {
                    "medium"
                } else {
                    "low"
                };
                crate::runtime::warn_stderr(
                    global.color_mode(),
                    &rust_i18n::t!(
                        "warn.write-risk",
                        level = level,
                        delta_pct = delta_pct,
                        original = original,
                        new_bytes = new_bytes
                    ),
                );
                // G-017: block large shrink always (CAS optional).
                if !args.allow_shrink && new_bytes < original {
                    return Err(AtomwriteError::InvalidInput {
                        reason: format!(
                            "write risk {} ({}% size delta, {} -> {} bytes) blocked; \
                             pass --allow-shrink to override",
                            level, delta_pct, original, new_bytes
                        ),
                    }
                    .into());
                }
                risk_assessment = Some(crate::ndjson_types::WriteRiskAssessment {
                    original_bytes: original,
                    new_bytes,
                    size_delta_pct: delta_pct,
                    risk_level: level,
                    guard_triggered: "size",
                });
            }
        }
    }

    let opts = AtomicWriteOptions {
        backup: effective_backup || auto_rotate_active,
        syntax_check: args.syntax_check,
        retention: resolved_backup.retention,
        preserve_timestamps: args.preserve_timestamps,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: args.strict.strict_atomic,
        wal_policy: args.wal_policy,
        // GAP-106: --require-backup implies --keep-backup so the backup
        // is retained on disk and the reported backup_path is valid.
        keep_backup: resolved_backup.keep || args.require_backup || auto_rotate_active,
        durability: args.durability.into(),
    };

    let result = atomic_write(&resolved, &content, &opts, &workspace)?;

    let output = WriteOutput {
        r#type: "write",
        status: "success",
        path: target_str,
        bytes_written: result.bytes_written,
        checksum: result.checksum,
        checksum_before: result.checksum_before,
        backup_path: result.backup_path,
        backup_skipped: result.backup_skipped,
        write_strategy: result.write_strategy,
        elapsed_ms: start.elapsed().as_millis() as u64,
        stdin_bytes_read,
        wal_policy: args.wal_policy.as_str(),
        platform: result.platform,
        mtime_preserved: if args.preserve_timestamps {
            Some(true)
        } else {
            None
        },
        risk_assessment,
    };

    writer.write_event(&output)?;
    Ok(())
}

include!("write_helpers.inc.rs");
