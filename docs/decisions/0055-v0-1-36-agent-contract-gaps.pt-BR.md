# ADR-0055 — Contrato agent-first v0.1.36 (GAP-2026-019…033)

Emendado por: 0061, 0065, 0072

## Status
Aceito (2026-07-31)

## Contexto
Sessões de agente falhavam com `--old-file` em `/tmp` (WORKSPACE_JAIL 126), liam envelopes de erro como “tudo nulo”, confiavam em `similarity: 1.0` falso em estratégias fuzzy normalizadas e reincidiam em mau uso de shell, de `--strategy` e do shrink.

## Decisão
1. Manter o jail nos caminhos de content-source e documentar o staging canônico `$workspace/.atomwrite/scratch/` (`constants::SCRATCH_DIR_REL`).
2. O NDJSON de erro sempre inclui `ok: false` e um `match_count` estável para falhas de casamento (0) e ambiguidade (contagem).
3. Estratégias fuzzy normalizadas emitem `similarity: null` (nunca um 1.0 falso).
4. O casamento escape-normalized mapeia os offsets pelo mapa Normalized→Original.
5. `MatchAmbiguous` inclui `best_candidate`.
6. O delta do soft-floor é a chave XDG `[fuzzy].soft_floor_delta` (padrão em constante).
7. `block_anchor` aplica dual-gate à razão do miolo quando o padrão tem miolo.
8. As skills EN/PT ensinam staging, parse com exit primeiro, denylist de `--strategy`, `$` literal, `--allow-shrink` e workspace multi-root.
9. Não remover o jail nem a guarda de shrink, e não inventar AST dentro do fuzzy.
10. Moldagem agent-native global: `--select`/`--fields`, `--filter`, `--limit` no writer (e no caminho de erro do `main`).
11. Sugestão distinta para content-source vs mutation-target em WORKSPACE_JAIL.

## Consequências
- Agentes estagiam patches dentro do jail já na primeira tentativa (`scratch init` + `.atomwrite/scratch/`).
- Parsers distinguem erro de sucesso sem inventar campos nulos de sucesso.
- A telemetria do fuzzy é honesta, com menos patches errados em silêncio.
- `--select` / `--filter` / `--limit` globais reduzem o NDJSON sem jq (agent-native).
- As sugestões de WORKSPACE_JAIL para content-source apontam para o staging, e as de mutation-target mantêm a dica de dentro do workspace.
- O threshold aggressive 0.50 foi reavaliado com o corpus de FP (dual-gate), e a sobrescrita via XDG continua disponível.

## CAPA residual (mesmo release)
- SSOT de documentação: o status corrente de cada gap vive na série de ADRs, nunca em nota não versionada
- Paridade dos ADRs pt-BR
- Extração SRP do `collect.inc.rs`
- Oráculo de FP com proptest

## Referências
- [Índice de decisões](README.md) (status de GAP-2026-019…033)
- [CHANGELOG](../../CHANGELOG.md)
