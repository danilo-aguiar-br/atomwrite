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
    #[arg(long, default_value_t = 200)]
    pub debounce_ms: u64,
    /// Maximum events before exit (0 = unlimited until signal).
    #[arg(long, default_value_t = 0)]
    pub max_events: u64,
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

    let debounce = Duration::from_millis(args.debounce_ms.max(1));
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
            break;
        }

        let wait = debounce.min(Duration::from_millis(50));
        match rx.recv_timeout(wait) {
            Ok(Ok(event)) => {
                if !is_core_kind(&event.kind) {
                    continue;
                }
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
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
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
            return Ok(());
        }
    }
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
) -> Result<()> {
    Err(crate::error::AtomwriteError::ConfigInvalid {
        reason: "watch requires rebuild: cargo install atomwrite --features watch (or cargo build --features watch)".into(),
    }
    .into())
}
