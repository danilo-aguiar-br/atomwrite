# ADR-0060 — A byte budget for `search` events in flight

## Status
Accepted (2026-10-15)

## Context
- `search` resident memory grew with the thread count. Measured on a machine with 72 CPUs and 160 GB of RAM, a tree of 1620 files and 380.9 MiB, warm cache, output to `/dev/null`: 142.6 MB at the default `-j` (gaps.md W10).
- The cause was a cost of about 1.5 MB per walker thread (31 MB at 1 thread, 38 MB at 8, 142 MB at 72), not the event channel.
- Memory had been bounded by counting events per worker, which says nothing about their size: with a large `--max-columns` the same counts hold arbitrarily many bytes.
- A first version of the budget, with a 1 MiB minimum, made `search` 10 to 15 times slower with 16 or more workers (`-j 72`: 39.99 to 59.18 s), because every release woke every sleeping worker (gaps.md W33).

## Decision
- One byte budget, `SEARCH_MAX_INFLIGHT_BYTES` (8 MiB), is shared by every `search` worker. It is the key `search.max_inflight_bytes`, accepted from 4 MiB to 1 GiB.
- The effective budget is `max(configured, workers x SEARCH_BUDGET_BYTES_PER_WORKER)`, where the per-worker floor is 128 KiB (twice the 64 KiB inline buffer).
- The stream the consumer is draining (the head) passes the limit only while it has no event in flight, so a slow consumer cannot deadlock the pipeline. Once the head has an event in flight it waits like every other stream.
- The charge lives in a `Charge` whose `Drop` releases the bytes, so a written event and an event dropped by a cancelled run both give the room back.
- The hot path is lock free (one atomic add or subtract per event). The mutex and condvar only put a blocked worker to sleep, and a release wakes one worker, only when the total falls under the limit.
- `search` keeps a bounded channel per file instead of one global map.

## Consequences
- Memory is bounded whatever the thread count and the match count.
- `search` at the default `-j` went from 142.6 MB to 45.3 MB on the 1620-file tree. `read` at the default `-j` went from 44.3 MB to 17.9 MB because the rayon pool is no longer built for it (ADR-0064).
- An explicit `-j 72` is honoured above the ceiling and measured 143.6 MB in one measurement and 71 to 80 MB in another. The 64 MB target applies to the default `-j`.
- A `.atomwrite.toml` that still says 1 MiB for the key is refused when it loads.
- The measurement is manual. The test that runs is `cli_v0137_search_rss`, with a 120 MiB ceiling.
- Dense `search` (3.24 million matching lines) went from 10.4 s to 12.9 s on a single sample under load 50, and the difference was not investigated (gaps.md W27, P3).

## Alternatives
- Count events per worker: rejected, event size is unbounded.
- One global `notify_all` on every release: rejected, it caused a futex storm (423 s of system time against 83 s of user time).
- Keep the 1 MiB minimum: rejected, it made `search` up to 15 times slower with many workers.

## Evidence
- `CHANGELOG.md:68,137-141,146,215-216,218,267,280` and `gaps.md:177-181` (W10), `gaps.md:270-273` (W33).
- `src/commands/search_budget.inc.rs:1-60` (design notes) and `:239-240` (effective budget); `src/constants_concurrency.inc.rs:58-72` (8 MiB, 64 KiB inline, 128 KiB per worker); `src/constants_config.inc.rs:50-65` (4 MiB to 1 GiB).
- `tests/cli_v0137_search_rss.rs`, `tests/cli_v0137_search_stream.rs`.
- Binary 0.1.37: `config get search.max_inflight_bytes` returned 8388608, and `config set search.max_inflight_bytes 1048576 --scope workspace` exited 65 with "must be 4194304..=1073741824".

## Trigger to revisit
- A measurement on the default `-j` goes above the 64 MB target.
- Another verb streams unbounded events and needs the same budget.
