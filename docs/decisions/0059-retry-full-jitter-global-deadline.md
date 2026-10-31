# ADR-0059 — One retry module with full jitter and a global deadline

## Status
Accepted (2026-10-15)

## Context
- The advisory lock, the Windows tempfile persist and the `EAGAIN` loop of the shutdown banner each had their own wait loop, with a fixed interval and no jitter (gaps.md W8).
- None of them took a ceiling from `--timeout-secs`, so a stuck lock could keep a run alive past its deadline.
- The `EAGAIN` loop spun on `yield_now` at 100% CPU when stderr was a full pipe.

## Decision
- All waits on a contended resource go through `src/retry.rs`. A `RetryPolicy` holds a base, a ceiling, `max_elapsed` and `max_attempts`.
- Wait number `n` is uniform in `[0, min(cap, base * 2^n)]` (full jitter, `fastrand`, a direct dependency since this release). The first wait comes before the second attempt, never an immediate retry.
- There are two stop conditions: `max_attempts` waits spent, or the time budget spent. The budget is the smaller of the policy `max_elapsed` and the time left of `--timeout-secs`.
- A sleep is cut into `RETRY_SLICE_MS` (15 ms) slices, and the shutdown flag is polled between slices, so SIGINT, SIGTERM, SIGHUP and the deadline are seen within one slice.
- The lock policy is base 50 ms, ceiling 400 ms, 512 attempts, and a budget equal to `lock.acquire_timeout_ms` (default 5000 ms). A cancel ends it as `Cancelled`, a spent budget ends it as `LOCK_TIMEOUT` (exit 83). When the budget is the global deadline, the wait gives the watchdog up to `RETRY_GLOBAL_DEADLINE_GRACE_MS` (100 ms) to raise the flag, so the run ends in exit 124 and not 83.
- The persist retry (`src/atomic/persist_retry.rs`, Windows) uses the same module with 8 attempts and a 2000 ms budget, and now also treats `ERROR_SHARING_VIOLATION` as transient.
- The `EAGAIN` loop waits with `poll(2)` and `POLLOUT` in 20 ms slices up to 200 ms.
- The shape is tunable with `retry.base_ms`, `retry.max_ms` and `retry.max_elapsed_ms` (ADR-0058), and `retry.base_ms` cannot exceed `retry.max_ms` nor `retry.max_elapsed_ms`.

## Consequences
- Retry is never the reason a run overstays its deadline.
- Writers that collided once do not collide again in lockstep.
- The Windows branch (`persist_retry.rs`) was checked only by compiling a copy of its body on Linux, because the Windows target could not be built here (gaps.md W16, P3).
- The lock has a cost: a contended lock now waits with randomized sleeps, so the exact wake-up time varies from run to run.

## Alternatives
- Keep three separate loops: rejected, they had no jitter, no deadline and one of them burned a core.
- Retry immediately on the first failure: rejected, the module enforces a first wait.

## Evidence
- `CHANGELOG.md:65-66,168-172` and `gaps.md:174-176` (W8), `gaps.md:361` (C12).
- `src/retry.rs:3-24` (rules), `src/retry.rs:36-111` (`RetryPolicy`, `with_shape_overrides`, `with_budget_override`, `jittered`), `src/retry.rs:182-239` (`Backoff::wait_with`, `sleep_sliced`); `src/constants_concurrency.inc.rs:10-27` (slice 15 ms, grace 100 ms, lock and persist shapes).
- `src/lock.rs`, `src/atomic/persist_retry.rs` (81 lines).
- `tests/cli_v0137_retry_deadline.rs` (4 tests), `tests/cli_v0137_retry_guards.rs` (4 tests), plus the unit tests of `src/retry.rs`.
- gaps.md C12 (V1 and V2): a stuck lock with `--timeout-secs 2` exits 124 in about 2031 ms, `lock.acquire_timeout_ms=500` exits 83 in 519 ms, SIGTERM exits 143 in 14 ms.
- Binary 0.1.37: `config get retry.base_ms`, `retry.max_ms`, `retry.max_elapsed_ms` and `lock.acquire_timeout_ms` returned 50, 400, 2000 and 5000 (all `origin` `default`, exit 0).

## Trigger to revisit
- A new resource needs a wait that does not fit "uniform in `[0, ceiling]`".
- The Windows persist path is validated on a real Windows host and behaves differently.
