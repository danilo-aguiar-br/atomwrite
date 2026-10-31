// SPDX-License-Identifier: MIT OR Apache-2.0
// Included from search_stream.inc.rs (v0.1.37, T3b).
//
// The global byte budget for search events in flight. Before it, memory was
// bounded by counting events per worker, which says nothing about their size:
// with a large `--max-columns` the same counts hold arbitrarily many bytes.

mod inflight {
    //! One budget, shared by every walker worker, measured in bytes.
    //!
    //! # Why it cannot deadlock
    //!
    //! The consumer drains one file at a time, in the order the files were
    //! announced. A worker that waits for room while the consumer waits for that
    //! worker would stall the run. Two rules prevent it:
    //!
    //! - the stream the consumer is draining right now (the *head*) passes the
    //!   limit while it has no event of its own in flight, so the consumer always
    //!   has something to drain; once it has one, it waits like everyone else;
    //! - a worker holding no event never waits on anything but the budget, and a
    //!   worker buffering events inline never waits at all.
    //!
    //! Room is released when an event leaves the pipeline, whether the consumer
    //! wrote it or a cancelled run dropped it, because the release lives in
    //! `Drop` of [`Charge`].

    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
    use std::sync::{Condvar, Mutex, OnceLock, PoisonError};
    use std::time::Duration;

    /// Bytes of events in flight, with the limit that gates new streams.
    ///
    /// The hot path (charge on send, release on drop) is lock free: one atomic
    /// add or subtract per event. The mutex and the condvar exist only to put a
    /// blocked worker to sleep, and a release touches them only when a worker is
    /// actually asleep.
    pub(super) struct InflightBudget {
        limit: u64,
        used: AtomicU64,
        /// Workers parked on the condvar.
        waiters: AtomicU32,
        gate: Mutex<()>,
        room: Condvar,
        /// Id of the stream the consumer is draining; `0` when none.
        head: AtomicU64,
        /// Last stream id handed out; ids start at 1.
        last_stream: AtomicU64,
    }

    /// Bytes held in the budget by one event; given back when it is dropped.
    pub(super) struct Charge {
        bytes: u64,
        budget: &'static InflightBudget,
    }

    impl Charge {
        /// Bytes this charge holds.
        pub(super) fn bytes(&self) -> u64 {
            self.bytes
        }
    }

    impl Drop for Charge {
        fn drop(&mut self) {
            self.budget.release(self.bytes);
        }
    }

    impl InflightBudget {
        pub(super) const fn new(limit: u64) -> Self {
            Self {
                limit,
                used: AtomicU64::new(0),
                waiters: AtomicU32::new(0),
                gate: Mutex::new(()),
                room: Condvar::new(),
                head: AtomicU64::new(0),
                last_stream: AtomicU64::new(0),
            }
        }

        /// Bytes currently held.
        #[cfg(test)]
        pub(super) fn used(&self) -> u64 {
            self.used.load(Ordering::SeqCst)
        }

        fn has_room(&self) -> bool {
            self.used.load(Ordering::SeqCst) < self.limit
        }

        /// Wake every parked worker, if there are any.
        ///
        /// For the rare events every sleeper must re-check (the head changed).
        /// Taking the gate before the notify closes the window in which a worker
        /// has checked its condition but not yet started to wait.
        fn wake(&self) {
            if self.waiters.load(Ordering::SeqCst) > 0 {
                drop(self.gate.lock().unwrap_or_else(PoisonError::into_inner));
                self.room.notify_all();
            }
        }

        /// Wake one parked worker, if there are any.
        ///
        /// Room is freed a little at a time, so waking all sleepers on every
        /// release makes dozens of workers fight for a few bytes (a futex storm
        /// that took `search` from 3 s to 50 s at 72 workers). One is woken, and
        /// it passes the wake on while room remains (see [`Self::park`]).
        fn wake_one(&self) {
            if self.waiters.load(Ordering::SeqCst) > 0 {
                drop(self.gate.lock().unwrap_or_else(PoisonError::into_inner));
                self.room.notify_one();
            }
        }

        /// The head stream has no event in flight any more: its producer may pass.
        pub(super) fn head_drained(&self) {
            self.wake();
        }

        /// Sleep until `ready` holds, in slices so a cancel is never missed.
        fn park(&self, ready: impl Fn() -> bool) {
            let slice = Duration::from_millis(crate::constants::SEARCH_BUDGET_WAIT_SLICE_MS);
            self.waiters.fetch_add(1, Ordering::SeqCst);
            let mut gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
            while !ready() {
                gate = self
                    .room
                    .wait_timeout(gate, slice)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
            drop(gate);
            self.waiters.fetch_sub(1, Ordering::SeqCst);
            // Pass the wake on: if this worker took only part of the room, the
            // next sleeper can use the rest.
            if self.has_room() {
                self.wake_one();
            }
        }

        /// Hand out the id of a new stream.
        pub(super) fn next_stream_id(&self) -> u64 {
            self.last_stream.fetch_add(1, Ordering::Relaxed) + 1
        }

        /// Mark `id` as the stream the consumer is draining.
        pub(super) fn set_head(&self, id: u64) {
            self.head.store(id, Ordering::SeqCst);
            self.wake();
        }

        /// The consumer is done with stream `id`.
        pub(super) fn clear_head(&self, id: u64) {
            self.head
                .compare_exchange(id, 0, Ordering::SeqCst, Ordering::SeqCst)
                .ok();
            self.wake();
        }

        /// Count `bytes` without waiting.
        ///
        /// For events buffered inline, which are few and small by construction.
        pub(super) fn charge(&'static self, bytes: u64) -> Charge {
            self.used.fetch_add(bytes, Ordering::SeqCst);
            Charge {
                bytes,
                budget: self,
            }
        }

        fn release(&self, bytes: u64) {
            let before = self.used.fetch_sub(bytes, Ordering::SeqCst);
            // Only the release that brings the pipeline back under the limit
            // can let a sleeper in.
            if before >= self.limit && before.saturating_sub(bytes) < self.limit {
                self.wake_one();
            }
        }

        /// Wait until the budget has room, without counting anything.
        ///
        /// A worker calls this before it starts a file, while it holds no event,
        /// so it can never be the one a full pipeline is waiting for.
        pub(super) fn wait_until_room(&self, cancelled: impl Fn() -> bool) {
            if !self.has_room() {
                self.park(|| self.has_room() || cancelled());
            }
        }

        /// Wait until the budget has room, then count `bytes`.
        ///
        /// Room means fewer bytes in flight than the limit, so one event larger
        /// than the limit still passes when the pipeline is empty instead of
        /// waiting forever. Several workers may pass the check together, so the
        /// total can overshoot the limit by one event per worker. `stream` is the
        /// caller's stream id with the bytes it has in flight; the head stream
        /// passes the limit only while it has nothing in flight, which is enough
        /// for the consumer to always have something to drain. Returns `None`
        /// once `cancelled` reports a cancel.
        pub(super) fn acquire(
            &'static self,
            bytes: u64,
            stream: Option<(u64, &AtomicU64)>,
            cancelled: impl Fn() -> bool,
        ) -> Option<Charge> {
            let is_head = || {
                stream.is_some_and(|(id, own)| {
                    self.head.load(Ordering::SeqCst) == id && own.load(Ordering::SeqCst) == 0
                })
            };
            loop {
                if self.has_room() || is_head() {
                    return Some(self.charge(bytes));
                }
                if cancelled() {
                    return None;
                }
                self.park(|| self.has_room() || is_head() || cancelled());
            }
        }
    }

    /// The budget of this process: the configured `search.max_inflight_bytes`,
    /// raised to what the walker workers need to keep working.
    ///
    /// Every worker buffers up to `FILE_INLINE_MAX_BYTES` of events inline
    /// before it hands them over, and holds one more event while it waits. A
    /// limit below that total is full before the first consumer read, so the
    /// workers sleep behind a budget they themselves fill (a 1 MiB limit made
    /// `search` 10 to 15 times slower at 16 workers or more). The floor is that
    /// total, so no configured value can starve the pipeline.
    pub(super) fn budget() -> &'static InflightBudget {
        static BUDGET: OnceLock<InflightBudget> = OnceLock::new();
        BUDGET.get_or_init(|| {
            let configured = crate::config::tuning().search_max_inflight_bytes;
            let workers = crate::concurrency::requested_walk_threads() as u64;
            let floor = workers.saturating_mul(crate::constants::SEARCH_BUDGET_BYTES_PER_WORKER);
            InflightBudget::new(configured.max(floor))
        })
    }

    #[cfg(test)]
    mod tests {
        use std::time::Instant;

        use super::super::{
            FILE_FEED_CAP, FILE_INLINE_MAX_BYTES, FileFeed, FileSender, SearchEvent,
        };
        use super::*;

        fn leaked(limit: u64) -> &'static InflightBudget {
            Box::leak(Box::new(InflightBudget::new(limit)))
        }

        #[test]
        fn charge_is_returned_on_drop() {
            let b = leaked(1_000);
            let c = b.charge(300);
            assert_eq!(b.used(), 300);
            drop(c);
            assert_eq!(b.used(), 0);
        }

        #[test]
        fn acquire_passes_under_the_limit_and_for_one_oversize_event() {
            let b = leaked(1_000);
            let big = b
                .acquire(5_000, None, || false)
                .expect("empty pipeline admits");
            assert_eq!(b.used(), 5_000);
            drop(big);
        }

        #[test]
        fn acquire_waits_for_room_and_resumes_when_it_appears() {
            let b = leaked(1_000);
            let full = b.charge(1_000);
            let freer = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(60));
                drop(full);
            });
            let begun = Instant::now();
            let got = b.acquire(10, None, || false).expect("room appears");
            freer.join().expect("freer thread");
            assert!(begun.elapsed() >= Duration::from_millis(50), "did not wait");
            assert!(begun.elapsed() < Duration::from_secs(2), "did not resume");
            drop(got);
        }

        #[test]
        fn the_head_stream_with_nothing_in_flight_does_not_wait() {
            let b = leaked(1_000);
            let _full = b.charge(1_000);
            let id = b.next_stream_id();
            b.set_head(id);
            let own = AtomicU64::new(0);
            let begun = Instant::now();
            let got = b.acquire(10, Some((id, &own)), || false);
            assert!(got.is_some());
            assert!(begun.elapsed() < Duration::from_millis(50));
            b.clear_head(id);
        }

        #[test]
        fn the_head_stream_with_bytes_in_flight_waits_like_the_others() {
            let b = leaked(1_000);
            let _full = b.charge(1_000);
            let id = b.next_stream_id();
            b.set_head(id);
            let own = AtomicU64::new(10);
            let begun = Instant::now();
            let got = b.acquire(10, Some((id, &own)), || {
                begun.elapsed() > Duration::from_millis(40)
            });
            assert!(got.is_none(), "the head may not pile bytes past the limit");
            b.clear_head(id);
        }

        #[test]
        fn a_non_head_stream_waits_and_cancel_releases_it() {
            let b = leaked(1_000);
            let _full = b.charge(1_000);
            let other = b.next_stream_id();
            b.set_head(other);
            let mine = b.next_stream_id();
            let begun = Instant::now();
            let own = AtomicU64::new(0);
            let got = b.acquire(10, Some((mine, &own)), || {
                begun.elapsed() > Duration::from_millis(40)
            });
            assert!(got.is_none(), "a cancelled wait must not take room");
            assert_eq!(b.used(), 1_000);
        }

        fn match_event(
            path: &std::sync::Arc<std::path::Path>,
            n: u64,
            bytes: usize,
        ) -> SearchEvent {
            SearchEvent::Match {
                path: std::sync::Arc::clone(path),
                line_number: n,
                lines: "x".repeat(bytes),
                byte_offset: n,
                submatches: Vec::new(),
                raw: Vec::new(),
            }
        }

        /// Several workers stream events far larger than the budget to one slow
        /// consumer: every event arrives, in order per file, and the bytes in
        /// flight never run away.
        #[test]
        fn large_events_flow_through_the_budget_without_stalling() {
            const WORKERS: usize = 3;
            const EVENTS: u64 = 12;
            const EVENT_BYTES: usize = 2 * 1024 * 1024;

            let (feeds_tx, feeds_rx) = crossbeam_channel::bounded::<FileFeed>(FILE_FEED_CAP);
            let producers: Vec<_> = (0..WORKERS)
                .map(|w| {
                    let feeds = feeds_tx.clone();
                    std::thread::spawn(move || {
                        let path: std::sync::Arc<std::path::Path> =
                            std::sync::Arc::from(std::path::Path::new(&format!("f{w}")));
                        let mut out = FileSender::new(&feeds);
                        for n in 0..EVENTS {
                            assert!(out.send(match_event(&path, n, EVENT_BYTES)));
                        }
                        assert!(out.finish());
                    })
                })
                .collect();
            drop(feeds_tx);

            let limit = crate::config::tuning().search_max_inflight_bytes;
            let slack = (WORKERS as u64 + 1) * (EVENT_BYTES as u64 + FILE_INLINE_MAX_BYTES);
            let mut seen = 0u64;
            let mut peak = 0u64;
            while let Ok(feed) = feeds_rx.recv_timeout(Duration::from_secs(20)) {
                let mut next = 0u64;
                for event in feed {
                    let SearchEvent::Match { line_number, .. } = event else {
                        continue;
                    };
                    assert_eq!(line_number, next, "events of one file must stay in order");
                    next += 1;
                    seen += 1;
                    peak = peak.max(budget().used());
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            for p in producers {
                p.join().expect("producer thread");
            }
            assert_eq!(
                seen,
                WORKERS as u64 * EVENTS,
                "an event was lost or the run stalled"
            );
            assert!(
                peak <= limit + slack,
                "bytes in flight ran past the budget: peak {peak}, limit {limit}, slack {slack}"
            );
            assert_eq!(budget().used(), 0, "every charge must be returned");
        }
    }
}
