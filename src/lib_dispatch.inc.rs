// SPDX-License-Identifier: MIT OR Apache-2.0
// Command dispatch and startup maintenance for `run`.
// Included from lib.rs.

include!("dispatch_heal.inc.rs");

/// Close the run: emit the terminal `cancelled` record when cancelled, flush,
/// and fold a data-channel write failure into the result.
///
/// On cancel the writer drops whatever it still holds for the tail pass
/// (`--sort-by` / `--dedupe` / `--aggregate`), so `cancelled` stays the last
/// line of the stream. A failed write to stdout (broken pipe included) is never
/// swallowed: it replaces the result so the exit code reflects it (141 for a
/// broken pipe). Only a failure that the broken-pipe path does not explain is
/// logged, so a caller that closed the reader gets silence on stderr too.
fn finish_run<W: Write>(
    result: Result<()>,
    writer: &mut NdjsonWriter<W>,
    shutdown: &signal::ShutdownSignal,
) -> Result<()> {
    let mut result = result.map_err(stdin_cancel::surface_cancel);
    if shutdown.is_shutdown() {
        writer.discard_pending();
        let event = crate::ndjson_types::CancelledEvent::from_shutdown(shutdown, None);
        let written = writer.write_event_unshaped(&event);
        result = merge_channel_failure(result, written, "cancelled event", shutdown);
    }
    let flushed = writer.flush();
    merge_channel_failure(result, flushed, "flush", shutdown)
}

/// Let a failed data-channel write win over the command result.
///
/// A closed reader is not a defect: it is the caller saying it stopped reading,
/// and the exit code already says 141. Logging it would put a line on stderr
/// for a run that behaved exactly as asked, so the warning is reserved for
/// failures that are not the broken pipe.
fn merge_channel_failure(
    result: Result<()>,
    write: Result<()>,
    what: &str,
    shutdown: &signal::ShutdownSignal,
) -> Result<()> {
    match write {
        Ok(()) => result,
        Err(e) => {
            if !shutdown.is_broken_pipe() {
                tracing::warn!(error = %e, what, "data channel write failed");
            }
            Err(e)
        }
    }
}

/// Roots a verb with side effects received in its invocation line (ETD).
///
/// Returns `None` for read-only verbs. For `copy` and `move` the root is the
/// destination, the path that is created or overwritten.
fn explicit_target_roots(command: &Commands) -> Option<Vec<std::path::PathBuf>> {
    use commands::recipe::RecipeAction;
    use commands::scratch::ScratchAction;
    let roots = match command {
        Commands::Edit(a) => vec![a.path.clone()],
        Commands::Write(a) => vec![a.target.clone()],
        Commands::EditLoop(a) => vec![a.path.clone()],
        Commands::Set(a) => vec![a.path.clone()],
        Commands::Del(a) => vec![a.path.clone()],
        Commands::Apply(a) => vec![a.file.clone()],
        Commands::Rollback(a) => vec![a.path.clone()],
        Commands::Copy(a) => vec![a.target.clone()],
        Commands::Move(a) => vec![a.target.clone()],
        Commands::Delete(a) => a.paths.clone(),
        Commands::Backup(a) => a.paths.clone(),
        Commands::PruneBackups(a) => a.paths.clone(),
        Commands::Case(a) => a.paths.clone(),
        Commands::Replace(a) => a.paths.clone(),
        Commands::Transform(a) => a.paths.clone(),
        Commands::Scope(a) => a.paths.clone(),
        Commands::Codemod(a) => a.paths.clone(),
        Commands::SemanticMerge(a) => vec![a.output.clone()],
        Commands::WalHeal(a) => vec![a.path.clone()],
        Commands::Scratch(a) => match &a.action {
            ScratchAction::Init(init) => vec![init.path.clone()],
        },
        Commands::Recipe(a) => match &a.action {
            RecipeAction::Run(run) => run.path.iter().chain(run.target.iter()).cloned().collect(),
            RecipeAction::List => return None,
        },
        _ => return None,
    };
    Some(roots)
}

/// Roots a read-only verb named, plus whether its only root is the ambient `.`.
///
/// Every verb that answers over a path the caller named, so an agent that
/// checks `target_resolved` gets the same stamp the writing verbs carry.
///
/// `count`, `list`, `sparse list`, `sparse outline`, `semantic-search` and
/// `watch` default to `.`, so that single root is `workspace` and every root
/// the caller typed is `argv`. `sparse read` answers over the paths file it
/// reads, and `hash --stdin` answers over the stream, which carries no path.
fn read_target_roots(
    command: &Commands,
) -> Option<(Vec<std::path::PathBuf>, bool)> {
    use commands::sparse::SparseAction;
    let dot = std::path::PathBuf::from(".");
    let is_dot = |paths: &[std::path::PathBuf]| paths.len() == 1 && paths[0] == dot;
    let (roots, ambient) = match command {
        Commands::Read(a) | Commands::Stat(a) => (a.paths.clone(), false),
        Commands::Count(a) => (a.paths.clone(), is_dot(&a.paths)),
        Commands::List(a) => (a.paths.clone(), is_dot(&a.paths)),
        Commands::Verify(a) => (vec![a.path.clone()], false),
        Commands::Get(a) => (vec![a.path.clone()], false),
        Commands::Query(a) => (vec![a.path.clone()], false),
        Commands::Outline(a) => (vec![a.path.clone()], false),
        Commands::ExtractArtifact(a) => (vec![a.path.clone()], false),
        Commands::Sparse(a) => match &a.action {
            SparseAction::List(list) => (vec![list.path.clone()], is_dot(std::slice::from_ref(&list.path))),
            SparseAction::Outline(outline) => {
                (vec![outline.path.clone()], is_dot(std::slice::from_ref(&outline.path)))
            }
            SparseAction::Read(read) => (vec![read.paths_file.clone()], false),
        },
        Commands::SemanticSearch(a) => (a.paths.clone(), is_dot(&a.paths)),
        Commands::Watch(a) => (vec![a.path.clone()], is_dot(std::slice::from_ref(&a.path))),
        Commands::Hash(a) => {
            if a.stdin {
                return None;
            }
            (a.paths.clone(), false)
        }
        Commands::Diff(a) => (vec![a.file_a.clone(), a.file_b.clone()], false),
        _ => return None,
    };
    Some((roots, ambient))
}

/// Resolve the subject of a read verb into the envelope pair.
///
/// The `-` operand names standard input, so it is kept as the token instead of
/// being resolved against the workspace: `diff a.txt -` reads one file and one
/// stream, and stamping `/workspace/-` named a path the run never touched.
fn resolve_read_targets(
    command: &Commands,
    workspace: &std::path::Path,
) -> Option<(String, commands::path_resolution::TargetSource)> {
    use commands::path_resolution::{
        STDIN_TARGET, TargetMode, TargetSource, resolve_mutation_targets,
    };
    if let Commands::Hash(a) = command
        && a.stdin
    {
        return Some((STDIN_TARGET.to_string(), TargetSource::Stdin));
    }
    let (roots, ambient) = read_target_roots(command)?;
    let source = if ambient {
        TargetSource::Workspace
    } else {
        TargetSource::Argv
    };
    let mode = if ambient {
        TargetMode::AmbientAllowed
    } else {
        TargetMode::Explicit
    };
    let files: Vec<std::path::PathBuf> = roots
        .iter()
        .filter(|root| root.as_os_str() != STDIN_TARGET)
        .cloned()
        .collect();
    let resolved = if files.is_empty() {
        Vec::new()
    } else {
        resolve_mutation_targets(&files, workspace, mode).ok()?
    };
    let mut resolved = resolved
        .iter()
        .map(|t| t.target_resolved.display().to_string());
    let mut parts: Vec<String> = Vec::with_capacity(roots.len());
    for root in &roots {
        if root.as_os_str() == STDIN_TARGET {
            parts.push(STDIN_TARGET.to_string());
        } else {
            parts.push(resolved.next()?);
        }
    }
    Some((parts.join(","), source))
}

/// Stamp `target_resolved` / `target_source` for the read-only answer verbs.
fn stamp_read_target<W: Write>(
    command: &Commands,
    writer: &mut NdjsonWriter<W>,
    workspace: &std::path::Path,
) {
    let Some((joined, source)) = resolve_read_targets(command, workspace) else {
        return;
    };
    writer.set_target(joined, source.as_str());
}

/// Refuse `--confirm-side-effect` on a verb that writes nothing.
///
/// The flag's own help promises it is accepted only where the command contract
/// asks for it, and a deletion or a rewrite confirmed by mistake is the exact
/// case the acknowledgement exists for. Accepting it on `read` and exiting 0
/// makes the acknowledgement meaningless: the caller learns nothing about
/// where the flag is honoured. The verb list is the ETD list itself, so the two
/// contracts cannot drift apart.
///
/// # Errors
///
/// Returns [`crate::error::AtomwriteError::InvalidInput`] when the flag is set
/// and the verb has no side effect.
fn reject_side_effect_ack_without_side_effect(
    command: &Commands,
    global: &cli::GlobalArgs,
) -> Result<()> {
    if global.side_effect_mode() != cli::SideEffectMode::Acknowledged {
        return Ok(());
    }
    if explicit_target_roots(command).is_some() {
        return Ok(());
    }
    Err(crate::error::AtomwriteError::InvalidInput {
        reason: format!(
            "{} writes nothing, so --confirm-side-effect has nothing to acknowledge [{}]",
            command_schema_name(command),
            crate::constants::SIDE_EFFECT_NO_SIDE_EFFECT_REASON_MARKER
        ),
    }
    .into())
}

/// Stamp `target_resolved` / `target_source` for a verb with side effects.
///
/// The target is always the one named in argv (`target_source = "argv"`): clap
/// already rejects an empty root list with exit 2, so no verb reaches here with
/// an ambient subject. Resolution is best-effort: a root the jail rejects is
/// reported by the verb itself, in its own error envelope, so a failure here
/// only leaves the keys unstamped. Several roots are joined with `,` in argv
/// order; one root is published as its canonical absolute path.
fn stamp_explicit_target<W: Write>(
    command: &Commands,
    writer: &mut NdjsonWriter<W>,
    workspace: &std::path::Path,
) {
    use commands::path_resolution::{TargetMode, TargetSource, resolve_mutation_targets};
    let Some(roots) = explicit_target_roots(command) else {
        return;
    };
    let Ok(resolved) = resolve_mutation_targets(&roots, workspace, TargetMode::Explicit) else {
        return;
    };
    let joined = resolved
        .iter()
        .map(|t| t.target_resolved.display().to_string())
        .collect::<Vec<_>>()
        .join(",");
    writer.set_target(joined, TargetSource::Argv.as_str());
}

/// Resolved subject of a failing run, for the error envelope `main` writes.
///
/// The terminal record normally carries `target_resolved` and `target_source`.
/// A run that fails before writing one still has to name the subject it did not
/// touch, so `main` asks here instead of leaving the envelope unstamped. The
/// answer is the same one the success path uses: the roots in argv for a verb
/// with side effects, and the config file of `--scope` for `config`.
#[must_use]
pub fn error_envelope_target(cli: &cli::Cli) -> Option<(String, String)> {
    use commands::path_resolution::{TargetMode, TargetSource, resolve_mutation_targets};
    let workspace = cli.global.resolve_workspace().ok()?;
    if let Commands::Config(args) = &cli.command {
        let file = commands::config_cmd::target_file(args, &workspace)?;
        return Some((
            file.display().to_string(),
            TargetSource::Argv.as_str().to_string(),
        ));
    }
    if let Commands::Locale(args) = &cli.command
        && (args.set.is_some() || args.clear)
        && let Some(path) = locale::preference_path()
    {
        return Some((
            path.display().to_string(),
            TargetSource::Argv.as_str().to_string(),
        ));
    }
    // The failing run names the subject the same way the succeeding one does:
    // `count`/`list` over the default `.` is `workspace` on both paths, and a
    // `hash --stdin` failure names the stream instead of staying unstamped.
    if let Some((joined, source)) = resolve_read_targets(&cli.command, &workspace) {
        return Some((joined, source.as_str().to_string()));
    }
    let roots = explicit_target_roots(&cli.command)?;
    let resolved = resolve_mutation_targets(&roots, &workspace, TargetMode::Explicit).ok()?;
    let joined = resolved
        .iter()
        .map(|t| t.target_resolved.display().to_string())
        .collect::<Vec<_>>()
        .join(",");
    Some((joined, TargetSource::Argv.as_str().to_string()))
}

/// Stamp the preference file that `locale --set` / `--clear` writes.
///
/// The file lives at a fixed path in the XDG config directory, outside the
/// workspace jail, so [`stamp_explicit_target`] cannot resolve it; the verb is
/// the caller's explicit request (`--set` / `--clear` in argv) to change that
/// one file, hence `argv`, like the `config` verb. Plain `locale` only reads.
fn stamp_locale_target<W: Write>(command: &Commands, writer: &mut NdjsonWriter<W>) {
    use commands::path_resolution::TargetSource;
    let Commands::Locale(args) = command else {
        return;
    };
    if args.set.is_none() && !args.clear {
        return;
    }
    if let Some(path) = locale::preference_path() {
        writer.set_target(path.display().to_string(), TargetSource::Argv.as_str());
    }
}

/// Run the `config` verb, which is answered before the config load.
///
/// The target it acts on is the config file of the `--scope` named in argv. The
/// global scope lives in the XDG config directory, outside the workspace jail,
/// so [`stamp_explicit_target`] (which resolves roots through the jail) would
/// drop the stamp without a word; this branch stamps the exact file instead.
fn run_config_verb<W: Write>(
    args: &commands::config_cmd::ConfigArgs,
    global: &cli::GlobalArgs,
    writer: &mut NdjsonWriter<W>,
    workspace: &std::path::Path,
) -> Result<()> {
    use commands::path_resolution::TargetSource;
    if let Some(target) = commands::config_cmd::target_file(args, workspace) {
        writer.set_target(target.display().to_string(), TargetSource::Argv.as_str());
    }
    commands::config_cmd::cmd_config(args, global, writer, workspace)
}

/// Route the parsed subcommand to its handler and return the handler result.
fn dispatch_command<R: Read, W: Write>(
    cli: &Cli,
    stdin: R,
    stdin_is_tty: bool,
    writer: &mut NdjsonWriter<W>,
    shutdown: &signal::ShutdownSignal,
    workspace: &std::path::Path,
    config: &crate::config::AtomwriteConfig,
) -> Result<()> {
    let defaults = &config.defaults;
    let write_cfg = &config.write;
    let watch_cfg = &config.watch;
    let fuzzy_cfg = &config.fuzzy;
    let search_cfg = &config.search;

    reject_side_effect_ack_without_side_effect(&cli.command, &cli.global)?;

    stamp_explicit_target(&cli.command, writer, workspace);
    stamp_read_target(&cli.command, writer, workspace);
    stamp_locale_target(&cli.command, writer);

    match &cli.command {
        Commands::Read(args) => commands::read::cmd_read(args, &cli.global, writer, "read"),
        Commands::Write(args) => commands::write::cmd_write(
            args,
            &cli.global,
            stdin,
            writer,
            shutdown,
            defaults,
            write_cfg,
        ),
        Commands::Edit(args) => commands::edit::cmd_edit(
            args,
            &cli.global,
            stdin,
            writer,
            workspace,
            defaults,
            fuzzy_cfg,
            write_cfg,
            stdin_is_tty,
        ),
        Commands::Search(args) => {
            // A-019: apply XDG [search] defaults when CLI still has constant defaults.
            let mut args = args.clone();
            if args.max_filesize == crate::constants::DEFAULT_SEARCH_MAX_FILESIZE_BYTES {
                args.max_filesize = search_cfg.max_filesize;
            }
            if args.max_columns == crate::constants::DEFAULT_SEARCH_MAX_COLUMNS {
                args.max_columns = search_cfg.max_columns;
            }
            if args.context == 0 && search_cfg.context > 0 {
                args.context = search_cfg.context as usize;
            }
            if search_cfg.smart_case && !args.case_insensitive {
                args.smart_case = true;
            }
            commands::search::cmd_search(&args, &cli.global, writer, shutdown)
        }
        Commands::Replace(args) => {
            commands::replace::cmd_replace(
                args,
                &cli.global,
                writer,
                shutdown,
                defaults,
                fuzzy_cfg,
                write_cfg,
            )
        }
        Commands::Hash(args) => commands::hash::cmd_hash(args, &cli.global, stdin, writer),
        Commands::Delete(args) => commands::delete::cmd_delete(args, &cli.global, writer, defaults),
        Commands::Count(args) => commands::count::cmd_count(args, &cli.global, writer),
        Commands::Diff(args) => commands::diff::cmd_diff(args, &cli.global, stdin, writer),
        Commands::Move(args) => commands::r#move::cmd_move(args, &cli.global, writer, defaults),
        Commands::Copy(args) => commands::copy::cmd_copy(args, &cli.global, writer, defaults),
        Commands::List(args) => commands::list::cmd_list(args, &cli.global, writer),
        Commands::Extract(args) => commands::extract::cmd_extract(args, stdin, writer),
        Commands::Calc(args) => commands::calc::cmd_calc(args, stdin, writer),
        Commands::Regex(args) => commands::regex_gen::cmd_regex(args, stdin, writer),
        Commands::Transform(args) => {
            commands::transform::cmd_transform(
                args,
                &cli.global,
                writer,
                shutdown,
                defaults,
                write_cfg,
            )
        }
        Commands::Batch(args) => commands::batch::cmd_batch(
            &cli.global,
            stdin,
            writer,
            args.dry_run,
            args.transaction,
            args.file.as_deref(),
            shutdown,
            &args.backup_opts,
            defaults,
            fuzzy_cfg,
            args.strict.strict_atomic,
        ),
        Commands::Scope(args) => {
            commands::scope::cmd_scope(args, &cli.global, writer, shutdown, defaults, write_cfg)
        }
        Commands::Backup(args) => commands::backup::cmd_backup(args, &cli.global, writer),
        Commands::Rollback(args) => {
            commands::rollback::cmd_rollback(args, &cli.global, writer, defaults)
        }
        Commands::Apply(args) => {
            commands::apply::cmd_apply(
                args, &cli.global, stdin, writer, defaults, fuzzy_cfg, write_cfg,
            )
        }
        Commands::Set(args) => commands::set::cmd_set(args, &cli.global, writer, defaults),
        Commands::Get(args) => commands::get::cmd_get(args, &cli.global, writer),
        Commands::Del(args) => commands::del::cmd_del(args, &cli.global, writer, defaults),
        Commands::Case(args) => {
            commands::case::cmd_case(args, &cli.global, writer, defaults, write_cfg)
        }
        Commands::Query(args) => commands::query::cmd_query(args, &cli.global, writer),
        Commands::Outline(args) => commands::outline::cmd_outline(args, &cli.global, writer),
        Commands::WalStats(args) => commands::wal_stats::cmd_wal_stats(args, &cli.global, writer),
        Commands::WalHeal(args) => commands::wal_stats::cmd_wal_heal(args, &cli.global, writer),
        Commands::PruneBackups(args) => {
            commands::prune_backups::cmd_prune_backups(args, &cli.global, writer)
        }
        Commands::EditLoop(args) => {
            // Pass the already-locked `stdin` from `run()`'s argument
            // instead of re-locking with `std::io::stdin().lock()`. The
            // latter is a deadlock: `Stdin::lock` is non-recursive, so
            // attempting to re-acquire it from the same thread that
            // holds it (via `main.rs:106 stdin.lock()`) blocks forever.
            // See audit 2026-06-17.
            commands::edit_loop::cmd_edit_loop(
                args,
                &cli.global,
                stdin,
                writer,
                defaults,
                fuzzy_cfg,
                write_cfg,
            )
        }
        Commands::Verify(args) => {
            let hash_args = cli_args::HashArgs {
                paths: vec![args.path.clone()],
                verify: Some(args.checksum.clone()),
                stdin: false,
                recursive: false,
                exclude: Vec::new(),
            };
            commands::hash::cmd_hash(&hash_args, &cli.global, stdin, writer)
        }
        Commands::SemanticMerge(args) => commands::semantic_merge::cmd_semantic_merge(
            args,
            &cli.global,
            writer,
            shutdown,
            defaults,
        ),
        Commands::Sparse(args) => {
            commands::sparse::cmd_sparse(args, &cli.global, writer, shutdown, defaults)
        }
        Commands::Recipe(args) => {
            commands::recipe::cmd_recipe(
                args,
                &cli.global,
                writer,
                shutdown,
                defaults,
                fuzzy_cfg,
                write_cfg,
            )
        }
        Commands::Stat(args) => {
            let mut a = args.clone();
            a.stat = true;
            commands::read::cmd_read(&a, &cli.global, writer, "stat")
        }
        Commands::AgentSurface(args) => commands::agent_surface::cmd_agent_surface(
            args,
            &cli.global,
            writer,
            shutdown,
            defaults,
        ),
        Commands::Watch(args) => {
            commands::watch::cmd_watch(args, &cli.global, writer, shutdown, defaults, watch_cfg)
        }
        Commands::ExtractArtifact(args) => {
            commands::extract_artifact::cmd_extract_artifact(args, &cli.global, writer)
        }
        Commands::Codemod(args) => {
            commands::codemod::cmd_codemod(args, &cli.global, writer, shutdown, defaults, write_cfg)
        }
        Commands::SemanticSearch(args) => commands::semantic_search::cmd_semantic_search(
            args,
            &cli.global,
            writer,
            shutdown,
            defaults,
        ),
        Commands::Scratch(args) => commands::scratch::cmd_scratch(args, &cli.global, writer),
        Commands::Doctor(args) => {
            commands::doctor::cmd_doctor(args, &cli.global, writer, shutdown, defaults)
        }
        Commands::Locale(args) => {
            commands::locale_cmd::cmd_locale(args, &cli.global, writer, shutdown, defaults)
        }
        Commands::CommandsTree(args) => {
            commands::command_tree::cmd_commands_tree(args, &cli.global, writer, shutdown, defaults)
        }
        Commands::Completions(_) => Err(crate::error::AtomwriteError::InternalError {
            reason: "completions must be answered before dispatch".to_string(),
        }
        .into()),
        Commands::Config(_) => Err(crate::error::AtomwriteError::InternalError {
            reason: "config must be answered before the config load".to_string(),
        }
        .into()),
    }
}
