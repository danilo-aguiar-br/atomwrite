# ADR: itens AVALIAR P3 do fuzzy para a v0.1.35

## Status
REJEITADO para a v0.1.35 (restrições de CLI one-shot e offline).

## Contexto
O G-FZZ P3 AVALIAR do gaps.md listava Smith-Waterman do nucleo, fonético (rphonetic), BM25/IR semântico, embeddings remotos e fan-out multiprocesso do SO.

## Decisão
- Rejeitar SW do nucleo/fzf, fonético, BM25, embeddings remotos e multiprocesso do SO.
- Manter a cascata local: exact → normalize → Damerau → JW dual-gate + gestalt/line-vote.
- Manter o rayon apenas intraprocesso (pool limitado), sem fan-out por fork/exec.

## Consequências
- Binário menor, latência menor, sem superfície de rede/telemetria.
- Os agentes obtêm qualidade de match determinística e offline, sem manter índice de IR.
- Revisitar só se o golden e2e mostrar uma classe sistemática de falhas que esses itens corrigiriam com <10% de orçamento de latência.

## Data
2026-07-19
