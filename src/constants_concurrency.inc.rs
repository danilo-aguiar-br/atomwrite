// SPDX-License-Identifier: MIT OR Apache-2.0
// Retry, thread-cap and in-flight memory constants (v0.1.37, T3b).
// Included from constants.rs.

// --- Retry with backoff and full jitter (src/retry.rs) ---
/// Longest single sleep inside a retry wait, in milliseconds.
///
/// A wait is cut into slices of this length so cancellation is observed within
/// one slice, whatever the backoff delay is.
pub const RETRY_SLICE_MS: u64 = 15;
/// Extra time a wait gives the global deadline watchdog to publish the cancel
/// flag once the global deadline is reached, in milliseconds.
pub const RETRY_GLOBAL_DEADLINE_GRACE_MS: u64 = 100;
/// Advisory lock retry: base of the exponential ceiling, in milliseconds.
pub const LOCK_RETRY_BASE_MS: u64 = 50;
/// Advisory lock retry: ceiling of one wait, in milliseconds.
pub const LOCK_RETRY_CAP_MS: u64 = 400;
/// Advisory lock retry: attempts before the lock is reported as timed out.
pub const LOCK_RETRY_MAX_ATTEMPTS: u32 = 512;
/// Tempfile persist retry: base of the exponential ceiling, in milliseconds.
pub const PERSIST_RETRY_BASE_MS: u64 = 50;
/// Tempfile persist retry: ceiling of one wait, in milliseconds.
pub const PERSIST_RETRY_CAP_MS: u64 = 400;
/// Tempfile persist retry: total time budget, in milliseconds.
pub const PERSIST_RETRY_MAX_ELAPSED_MS: u64 = 2_000;
/// Tempfile persist retry: attempts before the failure is reported.
pub const PERSIST_RETRY_MAX_ATTEMPTS: u32 = 8;
/// Windows `ERROR_SHARING_VIOLATION`: another process holds the file open.
pub const WINDOWS_ERROR_SHARING_VIOLATION: i32 = 32;
/// Longest wait for a full stderr pipe while the cancel banner is written, in
/// milliseconds.
pub const SHUTDOWN_BANNER_MAX_WAIT_MS: u64 = 200;
/// One `poll(2)` slice while waiting for stderr to become writable, in
/// milliseconds.
pub const SHUTDOWN_BANNER_POLL_SLICE_MS: u64 = 20;

// --- Thread caps (src/concurrency.rs) ---
/// Hard ceiling of the automatic worker bound (`--threads` omitted or `0`).
///
/// An explicit `--threads N` above this value is honoured: the operator asked
/// for it by name.
pub const MAX_AUTO_THREADS: usize = 16;
/// Ceiling of the automatic directory-walk thread count.
///
/// Mirrors the internal default of `ignore::WalkParallel` (`min(cpus, 12)`) so
/// the walker is never wider than the library protects it to be.
pub const WALK_AUTO_MAX_THREADS: usize = 12;
/// Percentage of free RAM eligible for concurrent file tasks.
pub const RAM_FRACTION_PERCENT: u64 = 50;
/// Logical CPU count assumed when the platform cannot report it.
pub const CPU_FALLBACK_COUNT: usize = 4;
/// Poll interval while a scan's walker thread is being stopped, in milliseconds.
pub const JOIN_WALKER_POLL_MS: u64 = 10;

// --- `search` event pipeline (src/commands/search_stream.inc.rs) ---
/// Total bytes of search events in flight between the walker workers and the
/// one consumer that owns stdout.
pub const SEARCH_MAX_INFLIGHT_BYTES: u64 = 8 * 1024 * 1024;
/// Events a worker keeps in a local `Vec` before it opens a stream for the file.
pub const SEARCH_FILE_INLINE_EVENTS: usize = 32;
/// Bytes a worker may buffer inline before it opens a stream for the file.
pub const SEARCH_FILE_INLINE_MAX_BYTES: u64 = 64 * 1024;
/// Capacity of the stream opened for a file that outgrew its inline buffer.
pub const SEARCH_FILE_STREAM_CAP: usize = 256;
/// Files announced to the consumer and not yet drained.
pub const SEARCH_FILE_FEED_CAP: usize = 64;
/// Budget bytes each walker worker needs to keep working: its inline buffer plus
/// one event in flight. The effective budget is never below workers times this.
pub const SEARCH_BUDGET_BYTES_PER_WORKER: u64 = 2 * SEARCH_FILE_INLINE_MAX_BYTES;
/// One wait slice of a worker blocked on the in-flight byte budget, in
/// milliseconds.
pub const SEARCH_BUDGET_WAIT_SLICE_MS: u64 = 10;

// --- Read-modify-write lock and drift check (src/lock.rs, src/atomic/write.rs) ---
/// Checksum shown in `STATE_DRIFT` when the file the caller read no longer exists.
pub const MISSING_FILE_CHECKSUM: &str = "<missing>";

// --- `watch` (src/commands/watch.rs) ---
/// Capacity of the channel between the filesystem watcher and the event loop.
pub const WATCH_EVENT_CHANNEL_CAP: usize = 4096;

// --- Cooperative stdin (src/stdin_cancel.rs) and cancel blocks (src/signal.rs) ---
/// Wait on the stdin data channel before the shutdown flag is checked again,
/// in milliseconds.
pub const STDIN_POLL_INTERVAL_MS: u64 = 25;
/// Blocks the stdin reader thread may queue ahead of the consumer.
pub const STDIN_CHANNEL_BLOCKS: usize = 4;
/// Size in bytes of one block read from the real stdin.
pub const STDIN_BLOCK_BYTES: usize = 64 * 1024;
/// Bytes of work between two cooperative-cancellation checks.
pub const CANCEL_BLOCK_BYTES: u64 = 64 * 1024;
/// Matches between two cooperative-cancellation checks.
pub const CANCEL_BLOCK_MATCHES: u64 = 4096;
/// Gap between two `read` calls worth a debug line in the searcher, in ms.
///
/// The cancel flag is consulted per `read`, so a gap is time the deadline
/// cannot cut. Normal reads are microseconds apart; anything near a second is
/// the line buffer being rebuilt, which is what ran the deadline over.
pub const SEARCH_READ_GAP_LOG_MS: u64 = 20;

const _: () = assert!(RETRY_SLICE_MS >= 1);
const _: () = assert!(LOCK_RETRY_BASE_MS >= 1 && LOCK_RETRY_CAP_MS >= LOCK_RETRY_BASE_MS);
const _: () = assert!(PERSIST_RETRY_BASE_MS >= 1 && PERSIST_RETRY_CAP_MS >= PERSIST_RETRY_BASE_MS);
const _: () = assert!(PERSIST_RETRY_MAX_ELAPSED_MS >= PERSIST_RETRY_BASE_MS);
const _: () = assert!(LOCK_RETRY_MAX_ATTEMPTS >= 1 && PERSIST_RETRY_MAX_ATTEMPTS >= 1);
const _: () = assert!(MAX_AUTO_THREADS >= 1 && WALK_AUTO_MAX_THREADS >= 1);
const _: () = assert!(RAM_FRACTION_PERCENT >= 1 && RAM_FRACTION_PERCENT <= 100);
const _: () = assert!(SEARCH_MAX_INFLIGHT_BYTES >= 1024 * 1024);
const _: () = assert!(SEARCH_FILE_INLINE_EVENTS < SEARCH_FILE_STREAM_CAP);
const _: () = assert!(SEARCH_FILE_FEED_CAP >= 1 && WATCH_EVENT_CHANNEL_CAP >= 16);
const _: () = assert!(STDIN_CHANNEL_BLOCKS >= 1 && STDIN_BLOCK_BYTES >= 1024);
