# ADR-0028: G119 — Intelligent WAL sidecar cleanup in 5 autonomous layers

> Status: Superseded in v0.1.37 for the sidecar lifetime. A journal that was written is removed when the write completes, so Committed sidecars do not remain on disk (measured: `write --wal-policy always` leaves no journal file).

> Historical (pre-0.1.35): product `ATOMWRITE_*` / env knobs described below are superseded. Runtime config is CLI flags + XDG `config.toml` / `atomwrite set|get` only.


- Status: Accepted
- Date: 2026-06-13
- Context: G119 started partially in v0.1.15, with L2 (Drop guard), L3 (wal-heal) and L5 (wal-stats) delivered. L1 (prevention via `--wal-policy`) and L4 (advanced heuristics) were missing. Without L1, 100% of the mutations with `--strict-atomic` or `ATOMWRITE_WAL=1` keep leaving a sidecar, and without L4 the decision engine is binary (drop or keep), with no nuance for TTL/LRU/rate-limit/sentinel/archiving. The empirical audit of 2026-06-13 counted 60 `Committed` sidecars in 10 directories (18 root, 18 docs, 14 schemas, 2 workflows, 2 skill-en, 2 skill-pt, 2 src, 1 commands, 1 tests, 1 snapshots), totalling 47,624 bytes. Each of these sidecars is wasted I/O on a trivial write (set/del, a TOML key set, a one-line edit) that L1 would have prevented at the source.

- Decision: Close G119 in v0.1.16 with the two layers that were missing:
  1. L1 (Prevention): new `enum WalPolicy { Auto, Always, Never }` in `src/wal.rs`, exposed through the CLI flag `--wal-policy` on `write` and `edit`. The `Auto` policy (default) skips the sidecar when the operation is trivial — file ≤ 1 MiB AND not Edit/Replace AND directory under Git AND size ≤ 4 KiB. `Always` forces the sidecar (legacy). `Never` suppresses it even with `--strict-atomic`. Git detection heuristic: walk up to 16 ancestor levels looking for `.git`.
  2. L4 (Heuristics): new submodule `crate::wal::heuristics` with 5 composable functions (`h1_ttl`, `h2_lru_within_cap`, `h3_rate_limit`, `h4_sentinel`, `h5_archive`). Each returns `true` to PRESERVE, `false` to clean. `heuristics_should_preserve(target, committed_at_unix, count, rank)` aggregates through OR. H1/H2/H3/H5 read env vars (`ATOMWRITE_WAL_KEEP_SECS`, `ATOMWRITE_WAL_MAX_COUNT`, `ATOMWRITE_WAL_RATE_LIMIT`, `ATOMWRITE_WAL_ARCHIVE_DAYS`) with safe defaults (0, 100, 10, 7). H4 detects the `.atomwrite_keep_wal` sentinel in the directory. H3 uses an `AtomicU64` with a 60s window (no `unsafe`, no env-var mutation in tests). New field `wal_policy: &'static str` in the `WriteOutput` envelope for telemetry.
  3. **`AtomicWriteOptions.wal_policy: WalPolicy`**: new field, default `WalPolicy::Auto`. `atomic_write` calls `should_create_sidecar` (L1) before `journal_started_with_guard`. When L1 votes `false`, an inert guard replaces the sidecar (O(0) cost, zero extra I/O).
  4. **`wal_policy` telemetry in the envelope**: field `wal_policy: "auto" | "always" | "never"` in `WriteOutput`, so the agent can audit which policy was applied.

- Consequences:
  - + L1 prevents 60-80% of the sidecars for typical LLM agent workloads (small writes in versioned directories): the root of the pollution.
  - + L4 gives operational flexibility: `ATOMWRITE_WAL_KEEP_SECS=60` in local scripts, `ATOMWRITE_WAL_MAX_COUNT=50` in dense workspaces, a local sentinel at `target/.atomwrite_keep_wal` to disable it.
  - + H3 (rate limit) protects against looping agents that generate 1000 sidecars/min.
  - + H5 (archive) preserves the audit history without polluting the active tree.
  - + The `Auto` default is safe: sidecars still exist for non-trivial operations (file > 1 MiB, edit, replace, unversioned directory).
  - + Telemetry makes misconfiguration detectable (`auto` producing many sidecars = wrong heuristic).
  - - `--wal-policy never`, misused, eliminates the crash audit trail — the operator must document the decision.
  - - L4 has 5 env vars: cognitive complexity. Mitigated by sensible defaults and by the "OR preserves" rule (any heuristic voting true is enough).
  - - The Git detector heuristic does not tell a `git init` in a subdirectory from a checkout of a Git parent, so it may underestimate (false positive → sidecar skipped unnecessarily). Acceptable: a skipped sidecar is cheaper than a sidecar created without need.

- Alternatives considered:
  1. Implement L1 and L4 in separate releases (v0.1.17 and v0.1.19), as the original plan foresaw. Rejected: the user required closure in a single release, and splitting delays the benefit.
  2. Use the `ignore` crate for Git detection instead of a manual walk. Rejected: an extra dependency, while the heuristic only needs a boolean and the search for `.git` is trivial.
  3. Make L1+L4 opt-in through a global flag. Rejected: the current default is already too permissive (always creates), and the safe default is `Auto`, which SKIPS the sidecar for trivial operations.
  4. H3 with a Mutex instead of an AtomicU64. Rejected: an AtomicU64 is lock-free, enough for the coarse counter of a single window.

- Trigger to revisit: If L1+L4 introduce a new gap (for example an agent complains that `Auto` skipped a needed sidecar), add a `--wal-policy-strict` flag that always preserves. If the cost of reading env vars per call proves measurable (>5% on a small write), cache it through a global `OnceCell`.

## Update v0.1.17 — Wiring of L3 startup + L4 in the Drop guard

v0.1.16 delivered L1 + L4 as library code, but both were disconnected from the points where the sidecar is created and removed. v0.1.17 closes the wiring:

- **L3 wired in `lib.rs::run`**: after `resolve_workspace()` and BEFORE dispatching the subcommand, call `auto_heal_on_startup(&workspace, threshold_secs=3600, max_duration_ms=100)`. A 1h threshold is the operational default, and a 100ms budget keeps the startup cost deterministic. The `Started` sidecar (a potential orphan) is NEVER reaped automatically — it is the signal that deserves the operator's attention. The global flag `--no-auto-heal` (or env `ATOMWRITE_WAL_NO_AUTO_HEAL=1`) disables it for high-cadence local loops and benchmarks.
- **L4 wired in `JournalGuard::drop`**: `Drop` now consults `heuristics_should_preserve` before removing. Since the Drop context does not know `workspace_committed_count` (an expensive scan), we pass `u64::MAX` to force `h2_lru_within_cap` to return `false`. The OR composition guarantees that `h1_ttl`, `h3_rate_limit`, `h4_sentinel` and `h5_archive` keep voting freely. Cost: O(1) per sidecar (only the env `metadata` + the sentinel check).
- **New field `committed_at_unix: Option<u64>` in `JournalGuard`**: `release()` stamps the current Unix timestamp, and `drop()` reads the value to feed H1 (TTL) and H5 (archive). Without a timestamp, those two heuristics stayed permanently disabled on the Drop path.
- Compatibility with existing tests: 2 tests in `tests/cli_v012_wal.rs` and 1 in `tests/cli_wal.rs` were adjusted to pass `--no-auto-heal` when the setup pre-conditions stale sidecars. Without the flag, L3 reaps the seeds before the subcommand runs.

### Consequences of the wiring
- + Every invocation now starts with a clean working tree: Committed/Aborted sidecars older than 1h die at startup. In a workspace with 60 stale sidecars, the first command deletes 60 at no perceptible cost (~5ms in `walk_journal_paths`).
- + The `--no-auto-heal` flag gives an escape for sensitive environments (parallel local automation, benchmarks, dry-run chains) without custom code.
- + L4 in the Drop is the first layer that consults heuristics PER EVENT (h4_sentinel, h3_rate_limit) instead of per batch. The operator can disable the sidecar in sensitive directories with `.atomwrite_keep_wal`.
- - Adding `--no-auto-heal` to manual tests is necessary for forensic inspection of stale sidecars (see 0027/0030).
- - Startup cost rises ~5-10ms in workspaces with >100 sidecars (the 100ms budget protects against regression).
- - The OR composition of L4 + the `u64::MAX` trick is subtle: the reviewer must understand that h2 is effectively off in the Drop. Documented inline at `wal.rs:540`.
