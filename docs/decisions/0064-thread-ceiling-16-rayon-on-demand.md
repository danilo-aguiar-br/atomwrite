# ADR-0064 — Automatic thread ceiling of 16 and a rayon pool built on demand

## Status
Accepted (2026-10-15)

## Context
- The rayon pool was built with one thread per CPU for every verb. On a host with 72 CPUs a `read` at the default `-j` used 44.3 MB and created 72 threads (gaps.md W10).
- The walker cost about 1.5 MB per thread (31 MB at 1 thread, 38 MB at 8, 142 MB at 72).
- A large `read` or `verify` built one pool thread per CPU through the hash path.

## Decision
- The automatic thread count is `min(CPUs, MAX_AUTO_THREADS = 16, RAM cap)`. The automatic directory walker is further capped at `WALK_AUTO_MAX_THREADS = 12`, the same guard the `ignore` crate applies.
- An explicit `-j N` is honoured as requested, even above 16.
- The rayon pool is built on demand, only for verbs that use `par_iter`: `ensure_global_pool()` runs before the first parallel iterator. `read`, `stat`, `write`, `move`, `get`, `set`, `del`, `rollback`, `calc`, `regex`, `extract`, `extract-artifact`, `verify`, `query`, `outline`, `scratch`, `locale`, `commands`, `agent-surface` and `completions` build none.
- `hash_bytes` and the mmap hash take the bounded pool before the first rayon hash (files from 1 MiB).
- `concurrency.max_threads` (1 to 256, default 16) and `concurrency.ram_fraction_percent` (1 to 100) are config keys (ADR-0058). A compile-time assert keeps `MAX_AUTO_THREADS` under the ceiling of the key.

## Consequences
- `read` at the default `-j` went from 44.3 MB to 17.9 MB. `search` at the default `-j` went from 142.6 MB to 45.3 MB (ADR-0060).
- `list` and `count` still build the pool, the documented safe pattern in `src/concurrency_walk.inc.rs` (16 threads in `-vvv list .`, gaps.md W42, P3).
- An explicit `-j 72` is not limited (gaps.md W44 B7, P3).
- The RAM cap exists only on Linux and does not read cgroups (gaps.md P3 list).
- Docs that say the default uses all cores must say "all cores up to a built-in ceiling, limited by RAM". The `-j` help already does.

## Alternatives
- One thread per CPU for every verb: rejected, the measured memory and thread cost for verbs that never parallelize.
- Clamp an explicit `-j` to the ceiling too: not done, an explicit request is honoured (gaps.md W10 and W44 B7, P3).

## Evidence
- `CHANGELOG.md:69,137-145,193,276` and `gaps.md:177-181,288` (W10, W42).
- `src/constants_concurrency.inc.rs:42-47` (16 and 12), `src/concurrency.rs:40-47,146,179-190`, `src/concurrency_walk.inc.rs:242-281` (`ensure_global_pool`), `src/checksum.rs:49,107`, `src/config_tuning.inc.rs:18`, `src/constants_config.inc.rs:34,66`.
- `tests/cli_concurrency.rs` and the unit tests `src/concurrency.rs:380-386`.
- Binary 0.1.37: `config get concurrency.max_threads` returned 16 (`origin` `default`).

## Trigger to revisit
- A default-`-j` measurement leaves the 64 MB target.
- Cgroup limits are read for the RAM cap.
