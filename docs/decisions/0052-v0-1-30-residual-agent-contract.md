# ADR-0052 — v0.1.30 residual agent contract

> **Status: Superseded in v0.1.37 for `--fuzzy off`.** `--fuzzy off` is accepted and means exact-only; a miss exits 65 `MATCH_FAILED` (measured with `edit a.txt --old zzz --new bar --fuzzy off`).

## Status
- Accepted on 2026-07-13
- Residual CLOSED on top of the v0.1.29 surface

## Context
- v0.1.29 closed P0–P3 and the agent surface
- The post-0.1.29 audit found the residual P17–P24 in the NDJSON contract and in the config
- gaps.md is the source of truth for the residual status

## Decision
- Emit `match_count` and `indent_adjusted` in the NDJSON EditOutput
- Reject `--fuzzy off` and `[fuzzy] mode = "off"` with exit 65 INVALID_INPUT
- Report `platform.backup_method` as `reflink_or_copy` or `copy` (never a hardlink of the live file)
- Exclude `*.bak.*` from recursive recipe/hash
- Sparse outline emits real AST `outline_item` under budget
- The `semantic-merge` help states that the merge is line-based (not AST)
- The `tests/cli_v0130_agent_contract.rs` suite locks the contract
- Regenerate `docs/schemas/edit-output.schema.json` with the residual fields

## Consequences
- Agents can NOT use exact-only through `--fuzzy off`
- Pipelines must parse `match_count` in multi-occurrence edits with `--replace-all`
- Agent-facing documentation (docs/, skill, llms, README) must reflect 0.1.30
- Real AST merge and removal of the `FuzzyMode::Off` enum remain as opportunities (they do not block)

## References
- [gaps.md](../../gaps.md)
- [MIGRATION.md](../MIGRATION.md#v0129-to-v0130-current)
- ADR-0051 (v0.1.29 gap close)
