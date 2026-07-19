# ADR-0051 — v0.1.29 gap close, surface de agente e honestidade de binário

- Status: Aceito
- Data: 2026-07-13
- Decisores: mantenedor atomwrite


## Contexto
- O gaps.md da 0.1.28 listava P0–P3 e residuais de auditoria
- O monólito AST gerava cerca de 52 MB no release default
- O PRD prometia 5–8 MB auto-contido
- Agentes pediam fuzzy no replace, diagnóstico de match e pipelines nomeados
- MCP foi re-escopado como proibido; a surface é CLI via agent-surface


## Decisão
- Fechar P0–P3 e residuais na 0.1.29 no working tree
- Extrair cascata fuzzy compartilhada em src/fuzzy.rs para edit e replace
- Emitir best_candidate em falhas de match (exit 65)
- Fatiar Cargo features: core, ast, lang-*, watch, semantic, full
- Declarar meta PRD 5–8 MB como alvo do build core, não do default com AST
- Local size-gate: slim core no máximo 15 MiB
- Implementar recipe com dispatch real search-replace-hash
- Implementar write --durability, Linux renameat2, stat, sparse, semantic-merge
- Foundation P3: watch com debounce/checksum/gitignore, semantic-search Jaccard offline, codemod com by_rule_id
- Regenerar schemas NDJSON e atualizar documentação EN/PT da raiz e de docs/


## Consequências
- Agentes devem pin cargo install --path . --force até publicar crates.io 0.1.29
- Pipelines exact-only no replace precisavam de --fuzzy off na 0.1.29; na 0.1.30 off é rejeitado (ver ADR-0052)
- Build slim omite AST e retorna exit 78 nos stubs
- Documentação histórica que cita 33 subcomandos permanece em seções de versões antigas
- Seções Current na época da 0.1.29 apontavam para 0.1.29; a 0.1.30 residual superou Current via ADR-0052


## Alternativas rejeitadas
- Publicar MCP server — proibido pelas rules do projeto
- Embedding OpenRouter no semantic-search — P3 permanece offline e local
- Mentir que o default release tem 5–8 MB — honestidade no CHANGELOG e README


## Ver também / See also
- [gaps.md](../../gaps.md)
- [CHANGELOG.md](../../CHANGELOG.md)
- [MIGRATION.md](../MIGRATION.md)
- [INSTALL.md](../INSTALL.md)
- [HOW_TO_USE.md](../HOW_TO_USE.md)
- [schemas/README.md](../schemas/README.md)
