# ADR-0026: Multi-pair edit gets fuzzy parity, per-pair reporting, and opt-in --partial (G117)

- **Status**: Accepted
- **Date**: 2026-06-11
- **Context**: `edit` accepts repeated `--old`/`--new` pairs, but the multi-pair path (`edit_old_new_multi`) used exact `find_str` matching only and returned `InvalidInput` on the first miss — losing the whole batch, reporting neither the failing pair index nor which pairs would have applied. The single-pair path has a 9-strategy fuzzy cascade, so the same whitespace-divergent pair that the single path rescues killed the multi batch. On top of that, the NDJSON error envelope goes to stdout, so `edit ... | jaq '.edits'` masked exit 65 as `{"edits": null}` with pipeline exit 0 (G117, documented in gaps.md on 2026-06-11).
- **Decision**: Extract the cascade into `match_pair(content, old, new, fuzzy_mode)` shared by both paths. The multi path now runs the full cascade per pair, accumulates `PairResult {index (1-based), matched, strategy, similarity}`, and on failure raises the new `AtomwriteError::EditPairFailed {index, total, reason, pair_results}` variant — which reuses the `INVALID_INPUT` code and exit 65 (no 26th error code) and enriches the error envelope with `failed_pair_index`, `pairs_total`, and `pair_results`. Success envelopes gain `pairs_total` and `pair_results`; `mode` becomes `fuzzy-multi(N)` when any pair matched fuzzily and stays `exact-multi(N)` otherwise. A new opt-in `--partial` flag applies the matching pairs (exit 0, `edits < pairs_total`) and reports the rest with `matched: false`; zero applied pairs maps to `NO_MATCHES` (exit 1, same as `replace`) with no write.
- **Consequences**:
  - **+** Fuzzy parity removes the undocumented single × multi asymmetry.
  - **+** `failed_pair_index` makes the error actionable without manual bisection; `pair_results` is the per-item ground truth (Agent-First CLI partial-failure principle).
  - **+** One process, one tempfile, one fsync, one rename for N pairs again — the "1 pair per invocation" workaround dies.
  - **+** All-or-nothing stays the default: when atomicity is possible it is strictly better than partial-failure reporting; `--partial` is explicit opt-in.
  - **-** In the all-or-nothing error, pairs after the failed index are never attempted and are absent from `pair_results` (absence + `failed_pair_index` distinguishes "failed" from "not attempted").
  - **-** Pair order is defined behavior: content evolves between pairs, so a later pair can match text introduced by an earlier one (same as pre-G117).
- **Alternatives considered**:
  1. New error code `EDIT_PAIR_FAILED` with its own exit. Rejected: 26th code churns every exit-code table in docs/skill for zero agent benefit; the structured fields carry the diagnosis.
  2. Stuff pair diagnostics into the `InvalidInput` message string. Rejected: agents would parse prose; structured fields are the contract.
  3. Make `--partial` the default. Rejected: silent partial application breaks the atomicity expectation of every existing caller.
- **Trigger to revisit**: If agents need `--partial` to also continue past *parse* errors (unequal `--old`/`--new` counts), or need a distinct exit code for partial application (exit 2 per the Agent-First principle), revisit the exit-code mapping.
