// SPDX-License-Identifier: MIT OR Apache-2.0

//! Watch paths and emit NDJSON change events (v0.1.29 P3-1, feature `watch`).
//!
//! Workload: I/O-bound / event-driven (notify FS events + debounce).
//! Parallelism: single consumer loop for event coalescing (order/debounce
//! contract). When `--checksum` is set and multiple paths flush together,
//! BLAKE3 digests fan out via `rayon::par_iter` (CPU/I/O multi-item) then
//! emit in path order. Bound: global rayon pool (`--threads` /
//! `--max-concurrency`). Heavy-memory: pending map is per-run, not a
//! process-wide singleton.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, ValueHint};

use crate::cli::GlobalArgs;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `watch`.
#[derive(Args, Debug)]
pub struct WatchArgs {
    /// Path to watch.
    #[arg(default_value = ".", value_hint = ValueHint::AnyPath)]
    pub path: PathBuf,
    /// Debounce milliseconds (coalesce per-path quiet period).
    ///
    /// Default: CLI if set, else XDG `[watch].debounce_ms`, else
    /// `constants::DEFAULT_WATCH_DEBOUNCE_MS` (A-XDG-002).
    #[arg(long, value_name = "MS")]
    pub debounce_ms: Option<u64>,
    /// Maximum events before exit (0 = unlimited until signal).
    #[arg(long, default_value_t = 0)]
    pub max_events: u64,
    /// Idle exit after N milliseconds with zero events (R-XDG-007).
    ///
    /// Default: XDG / `.atomwrite.toml` `[watch].idle_exit_ms`, else
    /// `constants::DEFAULT_WATCH_IDLE_EXIT_MS`. `0` disables idle-exit
    /// (still requires `--max-events` and/or global `--timeout-secs`).
    #[arg(long, value_name = "MS")]
    pub idle_exit_ms: Option<u64>,
    /// Include BLAKE3 checksum of the file when the path is a regular file.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub checksum: bool,
    /// Respect `.gitignore` (default true).
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub gitignore: bool,
}

/// Watch filesystem events (requires `--features watch`).
#[cfg(feature = "watch")]
#[cfg_attr(docsrs, doc(cfg(feature = "watch")))]
#[tracing::instrument(skip_all, fields(command = "watch"))]
pub fn cmd_watch(
    args: &WatchArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
    _defaults: &crate::config::DefaultsSection,
    watch_cfg: &crate::config::WatchSection,
) -> Result<()> {
    use std::collections::HashMap;
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};

    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    use crate::error::AtomwriteError;
    use crate::path_safety::validate_path;

    // One-shot rule: unbounded watch is daemon-shaped. Require a bound.
    if args.max_events == 0 && global.timeout_secs == 0 {
        return Err(AtomwriteError::InvalidInput {
            reason: "watch requires --max-events N and/or global --timeout-secs N \
                     (unbounded watch is forbidden for one-shot CLI)"
                .into(),
        }
        .into());
    }

    let workspace = global.resolve_workspace()?;
    let root = validate_path(&args.path, &workspace)?;
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    // CLI > XDG `[watch].debounce_ms` > constants (A-XDG-002).
    let debounce_ms = args
        .debounce_ms
        .unwrap_or(watch_cfg.debounce_ms)
        .max(1);
    let debounce = Duration::from_millis(debounce_ms);
    // B-007 / R-XDG-007: CLI flag > XDG `[watch].idle_exit_ms` > constants default.
    let idle_ms = args.idle_exit_ms.unwrap_or(watch_cfg.idle_exit_ms);
    let idle_exit = Duration::from_millis(idle_ms);
    let idle_enabled = idle_ms > 0;
    let watch_started = Instant::now();
    let mut saw_any_event = false;
    // path -> (last_seen, last_kind)
    let mut pending: HashMap<PathBuf, (Instant, String)> = HashMap::new();
    let mut count = 0u64;
    let git = ignore::gitignore::GitignoreBuilder::new(&root);
    let matcher = if args.gitignore {
        let mut b = ignore::gitignore::GitignoreBuilder::new(&root);
        let _ = b.add(root.join(".gitignore"));
        b.build().ok()
    } else {
        None
    };
    let _ = git;

    let exit_reason;
    loop {
        if shutdown.is_shutdown() {
            flush_pending(
                &mut pending,
                Instant::now(),
                Duration::ZERO,
                args,
                global,
                writer,
                &mut count,
                &matcher,
            )?;
            exit_reason = "signal";
            break;
        }

        // A-020: named floor so poll sleep is not a magic literal.
        let wait = debounce.min(Duration::from_millis(crate::constants::WATCH_DEBOUNCE_FLOOR_MS));
        match rx.recv_timeout(wait) {
            Ok(Ok(event)) => {
                if !is_core_kind(&event.kind) {
                    continue;
                }
                saw_any_event = true;
                let kind = format!("{:?}", event.kind);
                let now = Instant::now();
                for path in event.paths {
                    if let Some(ref m) = matcher {
                        let is_dir = path.is_dir();
                        if m.matched(&path, is_dir).is_ignore() {
                            continue;
                        }
                    }
                    pending.insert(path, (now, kind.clone()));
                }
            }
            Ok(Err(e)) => return Err(anyhow::Error::from(e)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                exit_reason = "complete";
                break;
            }
        }

        flush_pending(
            &mut pending,
            Instant::now(),
            debounce,
            args,
            global,
            writer,
            &mut count,
            &matcher,
        )?;
        if args.max_events > 0 && count >= args.max_events {
            exit_reason = "max_events";
            break;
        }
        // Idle exit: no events at all within idle window (CLI/XDG/constants).
        if idle_enabled
            && !saw_any_event
            && pending.is_empty()
            && watch_started.elapsed() >= idle_exit
        {
            tracing::debug!(idle_ms, "watch idle-exit (no filesystem events)");
            exit_reason = "idle";
            break;
        }
    }

    // A-WATCH-001: always emit a terminal summary so agents never see silent idle.
    writer.write_event(&crate::ndjson_types::WatchSummary {
        r#type: "watch_summary",
        events: count,
        reason: exit_reason.into(),
        idle_exit_ms: idle_ms,
        debounce_ms,
        max_events: args.max_events,
        elapsed_ms: watch_started.elapsed().as_millis() as u64,
    })?;
    Ok(())
}

#[cfg(feature = "watch")]
fn is_core_kind(kind: &notify::EventKind) -> bool {
    use notify::EventKind;
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

#[cfg(feature = "watch")]
#[allow(clippy::too_many_arguments)]
fn flush_pending(
    pending: &mut std::collections::HashMap<PathBuf, (std::time::Instant, String)>,
    now: std::time::Instant,
    debounce: std::time::Duration,
    args: &WatchArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    count: &mut u64,
    _matcher: &Option<ignore::gitignore::Gitignore>,
) -> Result<()> {
    use crate::concurrency::should_parallelize;
    use rayon::prelude::*;

    let mut ready: Vec<(PathBuf, String)> = pending
        .iter()
        .filter(|(_, (seen, _))| now.duration_since(*seen) >= debounce)
        .map(|(p, (_, kind))| (p.clone(), kind.clone()))
        .collect();
    if ready.is_empty() {
        return Ok(());
    }
    // Stable emit order after parallel checksums.
    ready.sort_by(|a, b| a.0.cmp(&b.0));

    let max_size = global.effective_max_filesize();
    let events: Vec<(PathBuf, String, Option<String>)> =
        if args.checksum && should_parallelize(ready.len()) {
            ready
                .par_iter()
                .map(|(path, kind)| {
                    let checksum = if path.is_file() {
                        crate::checksum::hash_file(path, max_size).ok()
                    } else {
                        None
                    };
                    (path.clone(), kind.clone(), checksum)
                })
                .collect()
        } else {
            ready
                .iter()
                .map(|(path, kind)| {
                    let checksum = if args.checksum && path.is_file() {
                        crate::checksum::hash_file(path, max_size).ok()
                    } else {
                        None
                    };
                    (path.clone(), kind.clone(), checksum)
                })
                .collect()
        };

    for (path, kind, checksum) in events {
        pending.remove(&path);
        writer.write_event(&crate::ndjson_types::WatchEvent {
            r#type: "watch",
            path: path.display().to_string(),
            kind,
            checksum,
        })?;
        *count += 1;
        if args.max_events > 0 && *count >= args.max_events {
            break;
        }
    }
    Ok(())
}

/// Stub when `watch` feature is disabled.
#[cfg(not(feature = "watch"))]
#[tracing::instrument(skip_all, fields(command = "watch"))]
pub fn cmd_watch(
    _args: &WatchArgs,
    _global: &GlobalArgs,
    _writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
    _defaults: &crate::config::DefaultsSection,
    _watch_cfg: &crate::config::WatchSection,
) -> Result<()> {
    Err(crate::error::AtomwriteError::ConfigInvalid {
        reason: "watch requires rebuild: cargo install atomwrite --features watch (or cargo build --features watch)".into(),
    }
    .into())
}
