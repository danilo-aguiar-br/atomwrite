# ADR-0055 — v0.1.36 agent-first contract (GAP-2026-019…033)

## Status
Accepted (2026-07-31)

## Context
Agent sessions failed on `--old-file` under `/tmp` (WORKSPACE_JAIL 126), misread error envelopes as “all null”, trusted fake `similarity: 1.0` on normalized fuzzy strategies, and re-hit shell/`--strategy`/shrink misuse.

## Decision
1. Keep workspace jail on content-source paths; document canonical staging `$workspace/.atomwrite/scratch/` (`constants::SCRATCH_DIR_REL`).
2. Error NDJSON always includes `ok: false` and stable `match_count` for match failures (0) / ambiguity (count).
3. Normalized fuzzy strategies emit `similarity: null` (not fake 1.0).
4. Escape-normalized match maps offsets via Normalized→Original map.
5. `MatchAmbiguous` includes `best_candidate`.
6. Soft-floor delta is XDG `[fuzzy].soft_floor_delta` (default constant).
7. `block_anchor` dual-gates middle ratio when pattern has a middle.
8. Skills EN/PT teach staging, exit-first parse, denylist `--strategy`, `$` literal, `--allow-shrink`, multi-root workspace.
9. Do **not** remove jail or shrink guard; do **not** invent AST inside fuzzy.

## Consequences
- Agents stage patches inside the jail on first try (`scratch init` + `.atomwrite/scratch/`).
- Parsers can distinguish error vs success without inventing null success fields.
- Fuzzy telemetry is honest; fewer silent wrong patches.
- Global `--select` / `--filter` / `--limit` shrink NDJSON without jq (agent-native).
- Content-source WORKSPACE_JAIL suggestions point at staging; mutation-target keeps inside-workspace tip.
- Aggressive threshold 0.50 re-evaluated with FP corpus (dual-gate); XDG override remains.

## Residual CAPA (same release)
- Docs SSOT (`gaps.md` Status sync + `scripts/check-gaps-ssot.sh`)
- ADR pt-BR parity
- `collect.inc.rs` SRP extract
- proptest FP oracle
