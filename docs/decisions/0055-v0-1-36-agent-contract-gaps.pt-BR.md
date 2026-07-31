# ADR-0055 — Contrato agent-first v0.1.36 (GAP-2026-019…033)

## Status
Aceito (2026-07-31)

## Contexto
Sessões de agente falhavam com `--old-file` em `/tmp` (WORKSPACE_JAIL 126), liam envelopes de erro como “tudo nulo”, confiavam em `similarity: 1.0` falso em estratégias normalizadas e reincidiam em shell/`--strategy`/shrink.

## Decisão
1. Manter jail em paths de content-source; staging canônico `$workspace/.atomwrite/scratch/` (`constants::SCRATCH_DIR_REL`); subcomando `scratch init`.
2. NDJSON de erro sempre com `ok: false` e `match_count` estável.
3. Estratégias fuzzy normalizadas emitem `similarity: null` (nunca 1.0 falso).
4. Escape-normalized mapeia offsets Normalized→Original.
5. `MatchAmbiguous` inclui `best_candidate`.
6. Soft-floor via XDG `[fuzzy].soft_floor_delta`.
7. `block_anchor` dual-gate no miolo quando o padrão tem miolo.
8. Skills EN/PT: staging, exit-first, denylist `--strategy`, `$` literal, `--allow-shrink`, multi-root.
9. **Não** remover jail/shrink; **não** inventar AST no fuzzy.
10. Shaping agent-native global: `--select`/`--fields`, `--filter`, `--limit` no writer (e no path de erro do `main`).
11. Suggestion distinta para content-source vs mutation-target em WORKSPACE_JAIL.

## Consequências
- Agentes estagiam patches no jail na primeira tentativa (`scratch init` + `.atomwrite/scratch/`).
- Parsers distinguem erro vs sucesso sem inventar campos nulos de sucesso.
- Telemetria fuzzy honesta; menos patches silenciosos errados.
- Agentes projetam/filtram NDJSON **sem** jq/sed no prompt.
- Threshold aggressive 0.50 mantido após reavaliação FP (override XDG).

## Referências
- [gaps.md](../../gaps.md) (GAP-2026-019…033)
- [0055 EN](0055-v0-1-36-agent-contract-gaps.md)
- [CHANGELOG.pt-BR](../../CHANGELOG.pt-BR.md)
