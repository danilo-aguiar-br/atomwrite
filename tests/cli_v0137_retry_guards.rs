// SPDX-License-Identifier: MIT OR Apache-2.0

//! v0.1.37 source guards for the retry and memory work (T3b).
//!
//! These pin design decisions that a behavioural test cannot see: which
//! primitive a module uses. Each assertion names the rule it protects.

const WATCH: &str = include_str!("../src/commands/watch.rs");
const LOCK: &str = include_str!("../src/lock.rs");
const PERSIST: &str = include_str!("../src/atomic/persist_retry.rs");
const SEARCH_STREAM: &str = include_str!("../src/commands/search_stream.inc.rs");
const CONCURRENCY: &str = include_str!("../src/concurrency.rs");

#[test]
fn watch_uses_a_bounded_channel() {
    assert!(
        WATCH.contains("sync_channel("),
        "watch must feed its event loop through a bounded channel"
    );
    assert!(
        !WATCH.contains("mpsc::channel") && !WATCH.contains(" channel()"),
        "an unbounded std channel lets a burst of events grow the queue without limit"
    );
}

#[test]
fn lock_and_persist_wait_through_the_retry_module() {
    assert!(
        LOCK.contains("crate::retry::"),
        "lock.rs must use src/retry.rs"
    );
    assert!(
        PERSIST.contains("crate::retry::"),
        "persist_retry.rs must use src/retry.rs"
    );
    for (name, src) in [("lock.rs", LOCK), ("persist_retry.rs", PERSIST)] {
        assert!(
            !src.contains("thread::sleep"),
            "{name} must not sleep on its own: a fixed sleep has no jitter, no deadline and no cancel"
        );
    }
}

#[test]
fn search_pipeline_is_bounded_in_bytes() {
    assert!(
        SEARCH_STREAM.contains("budget()"),
        "search events must go through the global byte budget"
    );
    assert!(
        !SEARCH_STREAM.contains("unbounded("),
        "the search pipeline must not use an unbounded channel"
    );
}

#[test]
fn rayon_pool_is_built_on_demand() {
    let lib = include_str!("../src/lib.rs");
    assert!(
        lib.contains("plan_global_pool"),
        "run() must plan the pool, not build it for every command"
    );
    assert!(
        !lib.contains("configure_global_pool("),
        "run() must not build the rayon pool unconditionally"
    );
    assert!(
        CONCURRENCY.contains("MAX_AUTO_THREADS"),
        "the automatic worker bound must keep its hard ceiling"
    );
}
