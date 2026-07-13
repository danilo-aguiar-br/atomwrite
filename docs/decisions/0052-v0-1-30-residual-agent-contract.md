# ADR-0052 — v0.1.30 residual de contrato de agente

## Status
- Aceito em 2026-07-13
- Residual FECHADO sobre a superfície v0.1.29

## Contexto
- A v0.1.29 fechou P0–P3 e superfície de agente
- Auditoria pós-0.1.29 encontrou residual P17–P24 no contrato NDJSON e config
- gaps.md é a fonte de verdade do status residual

## Decisão
- Emitir `match_count` e `indent_adjusted` no EditOutput NDJSON
- Rejeitar `--fuzzy off` e `[fuzzy] mode = "off"` com exit 65 INVALID_INPUT
- Reportar `platform.backup_method` como `reflink_or_copy` ou `copy` (nunca hardlink do vivo)
- Excluir `*.bak.*` de recipe/hash recursivo
- Sparse outline emite `outline_item` AST real sob orçamento
- Help de `semantic-merge` admite merge line-based (não AST)
- Suíte `tests/cli_v0130_agent_contract.rs` trava o contrato
- Regenerar `docs/schemas/edit-output.schema.json` com os campos residuais

## Consequências
- Agentes NÃO podem usar exact-only via `--fuzzy off`
- Pipelines devem parsear `match_count` em edições multi-ocorrência com `--replace-all`
- Documentação agent-facing (docs/, skill, llms, README) deve refletir 0.1.30
- Merge AST real e remoção do enum `FuzzyMode::Off` ficam como oportunidades (não bloqueiam)

## Referências
- [gaps.md](../../gaps.md)
- [MIGRATION.md](../MIGRATION.md#v0129-to-v0130-current)
- ADR-0051 (v0.1.29 gap close)
