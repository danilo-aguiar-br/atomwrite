# ADR-0051 — v0.1.29 gap close, superfície de agente e honestidade de binário

> **Status: Superado na v0.1.37 quanto ao `--fuzzy off`.** `--fuzzy off` é aceito e significa só casamento exato, e uma falha de casamento sai com 65 `MATCH_FAILED` (medido com `edit a.txt --old zzz --new bar --fuzzy off`).

- Status: Aceito
- Data: 2026-07-13
- Decisores: mantenedor atomwrite


## Contexto
- O gaps.md da 0.1.28 listava P0–P3 e residuais de auditoria
- O monólito AST gerava cerca de 52 MB no release default
- O PRD prometia 5–8 MB autocontido
- Agentes pediam fuzzy no replace, diagnóstico de match e pipelines nomeados
- A superfície é integração por subprocesso, CLI com saída NDJSON via agent-surface


## Decisão
- Fechar P0–P3 e residuais na 0.1.29 no working tree
- Extrair a cascata fuzzy compartilhada em src/fuzzy.rs para edit e replace
- Emitir best_candidate em falhas de match (exit 65)
- Fatiar as Cargo features: core, ast, lang-*, watch, semantic, full
- Declarar a meta do PRD de 5–8 MB como alvo do build core, não do default com AST
- Local size-gate: slim core no máximo 15 MiB
- Implementar recipe com dispatch real search-replace-hash
- Implementar write --durability, Linux renameat2, stat, sparse, semantic-merge
- Foundation P3: watch com debounce/checksum/gitignore, semantic-search Jaccard offline, codemod com by_rule_id
- Regenerar os schemas NDJSON e atualizar a documentação EN/PT da raiz e de docs/


## Consequências
- Agentes devem fixar `cargo install --path . --force` até publicar a 0.1.29 no crates.io
- Pipelines exact-only no replace precisavam de --fuzzy off na 0.1.29, e na 0.1.30 o off é rejeitado (ver ADR-0052)
- O build slim omite AST e retorna exit 78 nos stubs
- A documentação histórica que cita 33 subcomandos permanece em seções de versões antigas
- As seções Current na época da 0.1.29 apontavam para a 0.1.29, e o residual da 0.1.30 superou Current via ADR-0052


## Alternativas rejeitadas
- Embedding OpenRouter no semantic-search — o P3 permanece offline e local
- Mentir que o release default tem 5–8 MB — honestidade no CHANGELOG e no README


## Ver também
- [gaps.md](../../gaps.md)
- [CHANGELOG.md](../../CHANGELOG.md)
- [MIGRATION.md](../MIGRATION.md)
- [INSTALL.md](../INSTALL.md)
- [HOW_TO_USE.md](../HOW_TO_USE.md)
- [schemas/README.md](../schemas/README.md)
