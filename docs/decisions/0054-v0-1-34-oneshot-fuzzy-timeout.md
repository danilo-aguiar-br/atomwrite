# ADR-0054 — v0.1.34 one-shot fuzzy + timeout default 120

## Status
- Accepted 2026-07-19
- Docs-complete publish of the v0.1.33 one-shot runtime (same binary behavior)

## Context
- Agent expand-section edits often produce a replacement that **contains** the pattern (NEW embeds OLD)
- Pre-0.1.33 fuzzy multi-apply re-scanned inserted text and could hang for days (CPU-bound infinite growth)
- Global `--timeout-secs` defaulted to `0` (disabled), so unbounded monorepo jobs and hangs had no wall-clock backstop
- Default multi-apply was effectively unbounded when `--max-replacements` was omitted
- gaps.md tracks ONESHOT-* residual; code fix shipped as 0.1.33; 0.1.34 aligns agent-facing docs

## Decision
- Fuzzy multi-apply is **one-pass** left-to-right on the **original** content (`apply_fuzzy_one_pass`); it **never** re-scans inserted text
- Default max fuzzy applies = **1** when `--max-replacements` is omitted; hard ceiling **10_000**
- If replacement **contains** pattern (fixed-string), force a **single** apply even with a large `--max-replacements` (prevents infinite growth on section expansion)
- Global `--timeout-secs` (alias `--timeout`) **default 120**; `0` disables; deadline → exit **124**
- Cooperative cancel is polled mid-fuzzy cascade
- **Cancel vs timeout exits (distinct):**
  - Cooperative cancel mid-cascade (SIGINT/SIGTERM or cancel flag) → `cancelled` NDJSON event / exit **143**
  - Wall-clock `--timeout-secs` deadline → error envelope / exit **124**
- Dense CPU slices may slightly overshoot the deadline before the next poll (honest ONESHOT-017); still cooperative, not a hard preemption
- Caps: pattern 64 KiB; lev 8192 chars; max windows 4096; growth max(4×, +16 MiB)
- No telemetry of file contents or patterns
- Fuzzy modes remain only `auto|aggressive` (`off` rejected exit 65 since v0.1.30)
- Regression suite: `tests/cli_v0133_oneshot_fuzzy.rs` (hang < 2s, embeds, timeout)
- 41 subcommands unchanged

## Consequences
- Agents may safely expand sections where NEW embeds OLD without hang risk
- Scripts that relied on unlimited runtime must pass `--timeout-secs 0` (or a higher value) intentionally
- Bulk multi-hit fuzzy requires explicit `--max-replacements N` (and still one-pass)
- Documentation agent-facing (docs/, skill, llms, README) must reflect 0.1.34
- Pin `^0.1.34` for docs-complete; runtime identical to 0.1.33 oneshot

## References
- [gaps.md](../../gaps.md) (ONESHOT-*)
- [CHANGELOG](../../CHANGELOG.md)
- [MIGRATION.md](../MIGRATION.md#v0133-to-v0134-current)
- [MIGRATION.md](../MIGRATION.md#v0132-to-v0133) (runtime BREAKING)
- ADR-0052 (v0.1.30 residual agent contract)
- `tests/cli_v0133_oneshot_fuzzy.rs`
