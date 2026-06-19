# ADR-0043: shrink guard with --expect-checksum

- **Status**: Accepted
- **Date**: 2026-06-19
- **Context**: `--expect-checksum` only validates concurrency (hash match), NOT content correctness. The `verify_checksum()` function returns `Ok(())` when hashes match without inspecting stdin size. The `risk_assessment` (L1, from ADR-0035 v0.1.20) calculates size delta but only emits an `eprintln!` warning — it never blocks the operation. In the 2026-06-15 incident, `--expect-checksum` passed (the file had not been modified by a third party) but the write shrunk the file from 122,994 to 16,780 bytes (86% reduction). The agent believed it was protected by `--expect-checksum` because the documentation says "USE `--expect-checksum` for optimistic locking (state drift detection)" — language that suggests safety when it only provides concurrency control. LLM agents consume stdout (NDJSON), NOT stderr — the L1 warning was invisible to the agent. The exit code was 0 ("success") even after destroying 86% of the content. The combination `--expect-checksum` + exit 0 + `status: "success"` created false confidence that the operation was safe.

- **Decision**: Add a shrink guard that blocks writes reducing the file by more than 50% when `--expect-checksum` is active. The guard runs AFTER `verify_checksum()` and BEFORE `atomic_write()`. When triggered: return exit 65 (`INVALID_INPUT`) with message `"stdin is {pct}% smaller than target ({original} → {new} bytes); pass --allow-shrink to confirm"`. Add `--allow-shrink` flag to `WriteArgs` for explicit override when intentional truncation is desired. Make `risk_assessment` (L1) blocking when `--expect-checksum` is active AND shrink is detected: without `--expect-checksum`, L1 remains informative (warning on stderr) — current behavior preserved; with `--expect-checksum`, L1 becomes blocking — exit 65 when delta exceeds `--risk-threshold`. Rationale: if the caller cared enough to pass `--expect-checksum`, they want real protection. Without `--expect-checksum`, behavior is completely unchanged (backward compatible).

- **Consequences**:
  - **+** Agents using `--expect-checksum` get real protection against accidental truncation — the 2026-06-15 incident would have been blocked.
  - **+** Zero false positives for growth (the guard only blocks shrink, not growth). Writing a larger file than the original always succeeds.
  - **+** `--allow-shrink` provides explicit override for intentional truncation — zero friction for legitimate use cases.
  - **+** The guard is zero-cost when stdin is larger than the original (one integer comparison).
  - **+** Error response includes `shrink_blocked: true` and `shrink_pct: N` so agents understand WHY the operation was blocked and can take corrective action.
  - **-** (acceptable) Intentional truncation with `--expect-checksum` now requires `--allow-shrink`. This is a deliberate tradeoff: the guard protects the common case (accidental truncation) at the cost of one extra flag for the rare case (intentional truncation with concurrency control).
  - **-** (acceptable) The guard only activates with `--expect-checksum`. Bare writes without `--expect-checksum` are NOT protected by this guard — that case is covered by GAP-016 (backup-by-default) which provides a recovery mechanism instead of a blocking guard.

- **Alternatives considered**:
  1. **Always block shrink regardless of `--expect-checksum`.** Rejected: breaks backward compatibility for scripts that intentionally truncate files via `write`. The `--expect-checksum` gate ensures the guard only activates when the caller has expressed intent for safe operations.
  2. **Only emit a warning (keep L1 informative).** Rejected: agents don't read stderr. The 2026-06-15 incident proves that informative-only guards are invisible to LLM agents. The guard must be blocking (exit non-zero) to be effective.
  3. **Use a configurable threshold instead of fixed 50%.** Accepted as partial: the `--risk-threshold` flag (from ADR-0035 L1) already provides configurability. The 50% default is the shrink guard's hard floor; `--risk-threshold` can lower it further. A file that shrinks by 51% is almost certainly an error; a file that shrinks by 10% might be legitimate cleanup.
  4. **Add `--expect-checksum-strict` as a separate flag.** Rejected: adding another flag increases cognitive load. The shrink guard activates automatically when `--expect-checksum` is present — no new flag to remember. `--allow-shrink` is the escape hatch, not a new mode.

- **Trigger to revisit**: If legitimate use cases for >50% shrink with `--expect-checksum` are common (e.g., config file rotation, log truncation), consider lowering the threshold to 75% or making it configurable via `ATOMWRITE_SHRINK_THRESHOLD`.
