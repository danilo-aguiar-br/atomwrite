// SPDX-License-Identifier: MIT OR Apache-2.0

//! Three-way **line-based** merge (v0.1.29 P1-1, honesty note v0.1.30).
//!
//! Despite the name, this is NOT AST or embedding merge — it aligns by line
//! index with optional first/last-line anchoring for insert shifts.
//!
//! Workload: I/O-bound (three file reads + line align + optional atomic write).
//! Parallelism: three independent file reads via nested `rayon::join`
//! (base ∥ (ours ∥ theirs)); line merge stays sequential (shared buffers).
//! Multi-file campaigns use `batch` / scripts, not multi-root merge.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, ValueHint};
use schemars::JsonSchema;
use serde::Serialize;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::GlobalArgs;
use crate::commands::resolve_backup;
use crate::error::AtomwriteError;
use crate::output::NdjsonWriter;
use crate::path_safety::validate_path;
use crate::signal::ShutdownSignal;

/// Arguments for `semantic-merge` (line-based three-way merge, not AST/embedding).
#[derive(Args, Debug)]
#[command(
    about = "Three-way line-based merge by line index (not AST or embedding)",
    long_about = "Three-way LINE-BASED merge: aligns base/ours/theirs by line index \
with optional first/last-line anchoring. This is NOT AST merge and NOT embedding merge."
)]
pub struct SemanticMergeArgs {
    /// Refuse the cross-device copy-then-rename fallback (exit 91).
    #[command(flatten)]
    pub strict: crate::cli_args::StrictAtomicArgs,

    /// Common ancestor file.
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub base: PathBuf,
    /// Our version.
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub ours: PathBuf,
    /// Their version.
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub theirs: PathBuf,
    /// Output path.
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub output: PathBuf,
    /// Fail with exit 65 on conflicts.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub fail_on_conflict: bool,
    /// Write conflict markers into the output file (default: true).
    ///
    /// Pass `--write-conflict-markers false` or `--no-conflict-markers` for
    /// prefer-ours without markers (NDJSON still lists `conflicts[]`).
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub write_conflict_markers: bool,
    /// Prefer-ours without markers (overrides `--write-conflict-markers`).
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub no_conflict_markers: bool,
    /// Optimistic lock on output.
    #[arg(long)]
    pub expect_checksum: Option<String>,
    /// Report the planned merge and write nothing.
    #[arg(long, help = "Report the planned merge and write nothing", action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,
    /// Shared backup flags for the output write.
    #[command(flatten)]
    pub backup_opts: crate::cli_args::BackupOpts,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct SemanticMergeResult {
    r#type: &'static str,
    status: String,
    output: String,
    checksum_base: String,
    checksum_ours: String,
    checksum_theirs: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checksum_output: Option<String>,
    /// Write strategy the atomic engine used for the output file.
    write_strategy: &'static str,
    /// Why a requested backup was skipped, when the directory refused it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("type" = "string"))]
    backup_skipped: Option<String>,
    conflicts: Vec<MergeConflict>,
    elapsed_ms: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
struct MergeConflict {
    start_line: u64,
    end_line: u64,
    ours: String,
    theirs: String,
}

/// Run line-based three-way merge.
///
/// # Errors
///
/// Returns `AtomwriteError::Cancelled` when shutdown is requested before the reads or right after
/// them.
/// Returns `AtomwriteError::WorkspaceJail` when `--base`, `--ours`, `--theirs` or `--output`
/// escapes the workspace.
/// Returns `AtomwriteError::NotFound` when an input file does not exist, and
/// `AtomwriteError::FileTooLarge` when one exceeds the effective max filesize.
/// Returns `AtomwriteError::MergeConflict` with `--fail-on-conflict` when the line aligner leaves
/// conflicting regions; the merged record is published as the one terminal event.
/// Returns `AtomwriteError::StateDrift` when `--expect-checksum` does not match the output on disk.
/// Returns `AtomwriteError::Io` when an input cannot be read or the output cannot be written.
/// Returns `AtomwriteError::BrokenPipe` when the consumer closes the NDJSON stream.
#[tracing::instrument(skip_all, fields(command = "semantic-merge"))]
pub fn cmd_semantic_merge(
    args: &SemanticMergeArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    defaults: &crate::config::DefaultsSection,
) -> Result<()> {
    let start = std::time::Instant::now();
    if shutdown.is_shutdown() {
        return Err(crate::signal::cancelled_error("semantic-merge cancelled before read").into());
    }
    let workspace = global.resolve_workspace()?;
    let max = global.effective_max_filesize();
    let base_p = validate_path(&args.base, &workspace)?;
    let ours_p = validate_path(&args.ours, &workspace)?;
    let theirs_p = validate_path(&args.theirs, &workspace)?;
    let out_p = validate_path(&args.output, &workspace)?;

    // Independent I/O: nested join saturates up to 3 cores on large files.
    let (base, (ours, theirs)) = rayon::join(
        || crate::file_io::read_file_string(&base_p, max),
        || {
            rayon::join(
                || crate::file_io::read_file_string(&ours_p, max),
                || crate::file_io::read_file_string(&theirs_p, max),
            )
        },
    );
    let base = base?;
    let ours = ours?;
    let theirs = theirs?;
    if shutdown.is_shutdown() {
        return Err(crate::signal::cancelled_error(
            "semantic-merge cancelled after parallel reads",
        )
        .into());
    }

    let cb = checksum::hash_bytes(base.as_bytes());
    let co = checksum::hash_bytes(ours.as_bytes());
    let ct = checksum::hash_bytes(theirs.as_bytes());

    // A-MERGE-001: markers default on unless --no-conflict-markers.
    let markers = args.write_conflict_markers && !args.no_conflict_markers;
    // Move owned file contents into the merge result (no clone of large strings).
    let (merged, conflicts, status) = if co == ct {
        (ours, vec![], "already_equal".to_string())
    } else if cb == co {
        (theirs, vec![], "took_theirs".to_string())
    } else if cb == ct {
        (ours, vec![], "took_ours".to_string())
    } else {
        line_merge(&base, &ours, &theirs, markers)
    };

    if args.fail_on_conflict && !conflicts.is_empty() {
        // v0.1.36: a conflicted merge is a domain result, not malformed input.
        // Emit the event with the regions, then fail with MERGE_CONFLICT (84).
        let conflict_count = conflicts.len() as u64;
        let failure = AtomwriteError::MergeConflict {
            count: conflict_count,
            path: out_p.clone(),
        };
        // The event states the failure itself (`ok: false`, `code`,
        // `conflict_count`) and is the one terminal event of the run, so the
        // stream never shows `ok: true` followed by the error that contradicts
        // it, nor a second envelope for the same conflict.
        let result = SemanticMergeResult {
            r#type: "semantic_merge",
            status: "conflict".into(),
            output: out_p.display().to_string(),
            checksum_base: cb,
            checksum_ours: co,
            checksum_theirs: ct,
            checksum_output: None,
            // The conflict returns before the write, so no mechanism ran.
            write_strategy: "none",
            backup_skipped: None,
            conflicts,
            elapsed_ms: start.elapsed().as_millis() as u64,
        };
        return Err(writer.publish_failure(&result, failure.into()));
    }

    if let Some(ref exp) = args.expect_checksum {
        if out_p.exists() {
            let cur = checksum::hash_file(&out_p, max)?;
            if &cur != exp {
                return Err(AtomwriteError::StateDrift {
                    path: out_p,
                    expected: exp.clone(),
                    actual: cur,
                }
                .into());
            }
        }
    }

    if args.dry_run {
        let plan = crate::ndjson_types::DryRunPlan {
            r#type: "plan",
            operation: "semantic-merge".into(),
            path: out_p.display().to_string(),
            would_modify: !merged.is_empty(),
            details: Some(format!("status {status}, {} conflict(s)", conflicts.len())),
            // A merge of two memory documents is structured data, not a shell
            // payload, so the content patterns do not describe it.
            risk_assessment: None,
        };
        writer.write_event(&plan)?;
        return Ok(());
    }

    let resolved = resolve_backup(&args.backup_opts, defaults);
    let opts = AtomicWriteOptions {
        backup: resolved.backup,
        retention: resolved.retention,
        preserve_timestamps: false,
        backup_output_dir: None,
        strategy: None,
        strict_atomic: args.strict.strict_atomic,
        syntax_check: false,
        wal_policy: crate::wal::WalPolicy::Auto,
        keep_backup: resolved.keep,
        durability: crate::platform::Durability::Auto,
    };
    let wr = atomic_write(&out_p, merged.as_bytes(), &opts, &workspace)?;
    writer.write_event(&SemanticMergeResult {
        r#type: "semantic_merge",
        status,
        output: out_p.display().to_string(),
        checksum_base: cb,
        checksum_ours: co,
        checksum_theirs: ct,
        checksum_output: Some(wr.checksum),
        write_strategy: wr.write_strategy,
        backup_skipped: wr.backup_skipped,
        conflicts,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })?;
    Ok(())
}

fn line_merge(
    base: &str,
    ours: &str,
    theirs: &str,
    markers: bool,
) -> (String, Vec<MergeConflict>, String) {
    let b: Vec<&str> = base.lines().collect();
    let o: Vec<&str> = ours.lines().collect();
    let t: Vec<&str> = theirs.lines().collect();
    let mut out = Vec::new();
    let mut conflicts = Vec::new();
    let max = b.len().max(o.len()).max(t.len());
    for i in 0..max {
        let bl = b.get(i).copied().unwrap_or("");
        let ol = o.get(i).copied().unwrap_or("");
        let tl = t.get(i).copied().unwrap_or("");
        if ol == tl {
            out.push(ol.to_string());
        } else if ol == bl {
            out.push(tl.to_string());
        } else if tl == bl {
            out.push(ol.to_string());
        } else {
            conflicts.push(MergeConflict {
                start_line: (i as u64) + 1,
                end_line: (i as u64) + 1,
                ours: ol.to_string(),
                theirs: tl.to_string(),
            });
            if markers {
                out.push("<<<<<<< ours".into());
                out.push(ol.to_string());
                out.push("=======".into());
                out.push(tl.to_string());
                out.push(">>>>>>> theirs".into());
            } else {
                out.push(ol.to_string());
            }
        }
    }
    let status = if conflicts.is_empty() {
        "merged".into()
    } else {
        "conflict".into()
    };
    let mut s = out.join("\n");
    if base.ends_with('\n') && !s.ends_with('\n') {
        s.push('\n');
    }
    (s, conflicts, status)
}
