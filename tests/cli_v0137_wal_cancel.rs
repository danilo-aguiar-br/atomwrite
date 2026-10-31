// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 — a cooperative cancel leaves no WAL journal behind; a failure does.
//!
//! `SIGTERM` in the middle of `replace` left a `started` journal in about one
//! run in twenty: the cancel was observed after the journal was opened and
//! before the rename, and the guard kept the journal as it does for a failure.
//! A cooperative cancel that unwinds normally has an intact target and nothing
//! to recover, so its journal is discarded. A failed write (and a crash, which
//! never runs `Drop`) still keeps it for `wal-heal`.
//!
//! The shutdown flag is process-wide and one-way, so this binary holds a single
//! test: the failure case runs first, then the cancel is raised and never lowered.

use std::path::Path;
use std::sync::atomic::Ordering;

use atomwrite::atomic::{AtomicWriteOptions, atomic_write};
use atomwrite::wal::{WalPolicy, journal_path};

/// Names of the tempfiles atomwrite leaves behind when it dies mid-write.
fn tempfiles_under(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .expect("read_dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(atomwrite::constants::TEMPFILE_PREFIX))
        .collect()
}

#[test]
fn cancel_discards_the_journal_and_failure_keeps_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ws = dir.path();
    let options = |syntax_check: bool| AtomicWriteOptions {
        wal_policy: WalPolicy::Always,
        syntax_check,
        ..AtomicWriteOptions::default()
    };
    let opts = options(false);

    // 1. A failed write (syntax check rejects the content after the journal
    //    is opened) keeps its journal.
    let broken = ws.join("broken.json");
    let failed = atomic_write(&broken, b"{\"a\": [1, 2", &options(true), ws);
    assert!(failed.is_err(), "the syntax check must reject the content");
    assert!(
        journal_path(&broken).exists(),
        "a failed write must keep its journal for recovery"
    );

    // 2. Raise the cooperative cancel: the first check on the rename path sees
    //    it, between journal creation and commit.
    let signal = atomwrite::signal::install_handlers().expect("handlers");
    signal.flag().store(true, Ordering::Release);

    let existing = ws.join("existing.txt");
    std::fs::write(&existing, "original\n").expect("seed");
    let err = atomic_write(&existing, b"replacement\n", &opts, ws)
        .err()
        .expect("a cancelled write is an error");
    assert!(
        matches!(
            err.downcast_ref::<atomwrite::error::AtomwriteError>(),
            Some(atomwrite::error::AtomwriteError::Cancelled { .. })
        ),
        "expected Cancelled, got {err:#}"
    );
    assert_eq!(std::fs::read_to_string(&existing).unwrap(), "original\n");
    assert!(
        !journal_path(&existing).exists(),
        "a cancelled write must not leave a journal"
    );

    let fresh = ws.join("fresh.txt");
    assert!(atomic_write(&fresh, b"new\n", &opts, ws).is_err());
    assert!(
        !fresh.exists(),
        "a cancelled write must not create the target"
    );
    assert!(!journal_path(&fresh).exists());
    assert!(tempfiles_under(ws).is_empty(), "{:?}", tempfiles_under(ws));
}
