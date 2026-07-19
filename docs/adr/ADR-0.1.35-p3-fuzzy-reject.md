# ADR: P3 fuzzy AVALIAR items for v0.1.35

## Status
**REJECTED** for v0.1.35 (one-shot offline CLI constraints).

## Context
gaps.md G-FZZ P3 AVALIAR listed nucleo Smith-Waterman, phonetic (rphonetic), BM25/IR semantic, remote embeddings, multi-process OS fan-out.

## Decision
- **Reject** nucleo/fzf SW, phonetic, BM25, remote embeddings, OS multi-proc.
- **Keep** local cascade: exact → normalize → Damerau → JW dual-gate + gestalt/line-vote.
- **Keep** rayon intra-process only (bounded pool); no fork/exec fan-out.

## Consequences
- Smaller binary, lower latency, no network/telemetry surface.
- Agents get deterministic offline match quality without IR index maintenance.
- Revisit only if e2e golden shows systematic miss class these would fix with <10% latency budget.

## Date
2026-07-19
