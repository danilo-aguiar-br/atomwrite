# ADR-0.1.35 — SIMD fuzzy locate: memchr-first (reject triple_accel as default)

## Status
**REJECT** `triple_accel` as a required/default dependency for v0.1.35.

## Context
- GraphRAG fuzzy rules recommend `triple_accel` for high-volume batch edit distance.
- docsrs-cli: `triple_accel` is **u8-only** (no Unicode); AVX2→SSE→scalar fallback.
- atomwrite fuzzy operates on UTF-8 line windows with `strsim::normalized_damerau_levenshtein`.

## Decision
1. Keep **memchr** for exact locate and **strsim** for Unicode-safe similarity.
2. Intra-file window scoring uses **rayon** bound fan-out (`windows::match_context_aware`).
3. Do **not** add `triple_accel` unless a local microbench shows ≥15% on ASCII-only locate paths and a guarded ASCII fast-path is isolated.

## Consequences
- No Unicode semantic distortion from u8 SIMD distance.
- Optional future feature `simd-ascii` may re-evaluate with ADR amendment + benches.

## Alternatives considered
- Wire triple_accel on all windows — rejected (Unicode).
- nucleo-matcher — rejected for one-shot offline (P3 ADR already).
