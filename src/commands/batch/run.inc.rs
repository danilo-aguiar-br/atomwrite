/// Execute multiple operations from an NDJSON manifest in batch mode.
///
/// When `manifest_path` is `Some`, the NDJSON manifest is read from that file
/// instead of from stdin. The path is validated against the workspace jail.
///
/// # Errors
///
/// Returns `AtomwriteError::Io` if reading stdin or writing results fails.
/// Returns `AtomwriteError::InvalidInput` if the manifest contains invalid operations.
#[tracing::instrument(skip_all, fields(command = "batch"))]
#[allow(clippy::too_many_arguments)]
pub fn cmd_batch(
    global: &GlobalArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
    dry_run: bool,
    transaction: bool,
    manifest_path: Option<&std::path::Path>,
    shutdown: &ShutdownSignal,
    backup_opts: &crate::cli_args::BackupOpts,
    defaults: &crate::config::DefaultsSection,
    fuzzy_cfg: &crate::config::FuzzySection,
    strict_atomic: bool,
) -> Result<()> {
    let resolved = crate::commands::resolve_backup(backup_opts, defaults);
    let (batch_fuzzy_mode, batch_fuzzy_threshold) =
        crate::config::resolve_fuzzy(crate::cli::FuzzyMode::Auto, None, fuzzy_cfg)?;
    let start = Instant::now();
    let workspace = global.resolve_workspace()?;

    // Stage 1 — manifest: one reader, one shape, refused before any side effect.
    let ops = read_manifest(stdin, manifest_path, &workspace)?;

    let mut run = BatchRun {
        ops: &ops,
        workspace: &workspace,
        global,
        dry_run,
        transaction,
        keep_backup: resolved.keep,
        no_backup: backup_opts.no_backup,
        backup_explicit: backup_opts.backup == Some(true),
        retention: resolved.retention,
        strict_atomic,
        fuzzy_mode: batch_fuzzy_mode,
        fuzzy_threshold: batch_fuzzy_threshold,
        fuzzy_cfg,
        shutdown,
        writer,
        start,
        succeeded: 0,
        failed: 0,
    };

    // Stage 2 — execution: pre-backup snapshot, then the fan-out or the
    // ordered pass; both report the same counters.
    run.execute()?;

    // Stage 3 — verdict: the summary is the single terminal event.
    run.verdict(dry_run, transaction)
}

include!("manifest.inc.rs");
include!("execute.inc.rs");
