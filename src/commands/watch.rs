// SPDX-License-Identifier: MIT OR Apache-2.0

//! Watch paths and emit NDJSON change events (v0.1.29 P3-1, feature `watch`).

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalArgs;
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for `watch`.
#[derive(Args, Debug)]
pub struct WatchArgs {
    /// Path to watch.
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Debounce milliseconds (coalesce per-path quiet period).
    #[arg(long, default_value_t = 200)]
    pub debounce_ms: u64,
    /// Maximum events before exit (0 = unlimited until signal).
    #[arg(long, default_value_t = 0)]
    pub max_events: u64,
    /// Include BLAKE3 checksum of the file when the path is a regular file.
    #[arg(long)]
    pub checksum: bool,
    /// Respect `.gitignore` (default true).
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub gitignore: bool,
}

/// Watch filesystem events (requires `--features watch`).
#[cfg(feature = "watch")]
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

    use crate::path_safety::validate_path;

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
    let ready: Vec<PathBuf> = pending
        .iter()
        .filter(|(_, (seen, _))| now.duration_since(*seen) >= debounce)
        .map(|(p, _)| p.clone())
        .collect();
    for path in ready {
        if let Some((_, kind)) = pending.remove(&path) {
            let checksum = if args.checksum && path.is_file() {
                crate::checksum::hash_file(&path, global.effective_max_filesize()).ok()
            } else {
                None
            };
            writer.write_event(&serde_json::json!({
                "type": "watch",
                "path": path.display().to_string(),
                "kind": kind,
                "checksum": checksum,
            }))?;
            *count += 1;
            if args.max_events > 0 && *count >= args.max_events {
                break;
            }
        }
    }
    Ok(())
}

/// Stub when `watch` feature is disabled.
#[cfg(not(feature = "watch"))]
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
