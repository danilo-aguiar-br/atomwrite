# atomwrite JSON Schemas

_Last updated: 2026-06-23 (v0.1.26) — 29 schemas in index_

## English
### Purpose
- Each schema describes the NDJSON output of one atomwrite subcommand
- All schemas follow JSON Schema draft/2020-12 (with the historical exception of `error-output.schema.json` which uses draft-07 for backward compatibility with pre-v0.1.12 agents)
- Use these schemas to validate agent-consumed output programmatically

### Schema Index
- `write-output.schema.json` -- output of `atomwrite write`
- `read-output.schema.json` -- output of `atomwrite read`
- `edit-output.schema.json` -- output of `atomwrite edit` (v0.1.15: adds `pairs_total` and `pair_results` -- G117) (v0.1.23: adds `source` field in `pair_results` -- GAP-2026-018)
- `search-match.schema.json` -- output of `atomwrite search` (per-match event)
- `replace-result.schema.json` -- output of `atomwrite replace` (per-file event)
- `delete-output.schema.json` -- output of `atomwrite delete`
- `get-result.schema.json` -- output of `atomwrite get` (v0.1.12, v14 Tier 3: single key read; `value` auto-parsed)
- `hash-output.schema.json` -- output of `atomwrite hash`
- `count-summary.schema.json` -- output of `atomwrite count`
- `diff-output.schema.json` -- output of `atomwrite diff`
- `move-output.schema.json` -- output of `atomwrite move`
- `copy-output.schema.json` -- output of `atomwrite copy`
- `list-entry.schema.json` -- output of `atomwrite list` (per-entry event)
- `extract-output.schema.json` -- output of `atomwrite extract`
- `calc-output.schema.json` -- output of `atomwrite calc`
- `regex-output.schema.json` -- output of `atomwrite regex`
- `transform-result.schema.json` -- output of `atomwrite transform`
- `outline-output.schema.json` -- output of `atomwrite outline` (v0.1.12, v14 Tier 3: high-level structure extraction; `items[].kind`, `items[].name`)
- `query-output.schema.json` -- output of `atomwrite query` (v0.1.12, v14 Tier 3: tree-sitter AST query; oneOf `kinds`|`tree`|`matches`)
- `batch-summary.schema.json` -- output of `atomwrite batch` (summary event)
- `scope-result.schema.json` -- output of `atomwrite scope` (per-file event)
- `backup-result.schema.json` -- output of `atomwrite backup` (per-file event)
- `rollback-result.schema.json` -- output of `atomwrite rollback`
- `set-result.schema.json` -- output of `atomwrite set` (v0.1.12, v14 Tier 3: dotted-path key write; `action: set`)
- `wal-recovery.schema.json` -- output of `recover_orphan_journals` consultive API (v0.1.12, G114: `JournalEntry::{Started, Committed, Aborted}` reports)
- `apply-result.schema.json` -- output of `atomwrite apply`
- `case-result.schema.json` -- output of `atomwrite case` (v0.1.12, v14 Tier 3: identifier case conversion; `files_modified`, `identifiers_renamed`)
- `del-result.schema.json` -- output of `atomwrite del` (v0.1.12, v14 Tier 3: key deletion; `action: deleted|already_missing`)
- `get-result.schema.json` -- output of `atomwrite get` (v0.1.12, v14 Tier 3: single key read; `value` auto-parsed)
- `error-output.schema.json` -- error envelope emitted by all subcommands (v0.1.15: adds `failed_pair_index`, `pairs_total`, `pair_results` -- G117)
- `wal-stats-output.schema.json` -- output of `atomwrite wal-stats` (v0.1.16: G119 L5 telemetry; `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `by_directory`, `auto_heal_recommended`, `estimated_reclaim_bytes`)
- `count-by-size-output.schema.json` -- output of `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N files by descending size; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- nested risk telemetry in `atomwrite write` output (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)
- `edit-loop-output.schema.json` -- output of `atomwrite edit-loop` (v0.1.22: N `{old, new}` pairs in 1 invocation via NDJSON; `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`)
- `prune-backups-output.schema.json` -- output of `atomwrite prune-backups` (v0.1.22: per-backup line + summary; `path`, `reason`, `action`, `total`, `elapsed_ms`)


## Português
### Última atualização: 2026-06-23 (v0.1.26) — 29 schemas no índice

### Objetivo
- Cada schema descreve a saída NDJSON de um subcomando do atomwrite
- Todos os schemas seguem JSON Schema draft/2020-12 (com a exceção histórica de `error-output.schema.json` que usa draft-07 para compatibilidade retroativa com agentes pré-v0.1.12)
- Use estes schemas para validar saída consumida por agentes de forma programática

### Índice de Schemas
- `write-output.schema.json` -- saída do `atomwrite write`
- `read-output.schema.json` -- saída do `atomwrite read`
- `edit-output.schema.json` -- saída do `atomwrite edit` (v0.1.15: adiciona `pairs_total` e `pair_results` -- G117) (v0.1.23: adiciona campo `source` em `pair_results` -- GAP-2026-018)
- `search-match.schema.json` -- saída do `atomwrite search` (evento por match)
- `replace-result.schema.json` -- saída do `atomwrite replace` (evento por arquivo)
- `delete-output.schema.json` -- saída do `atomwrite delete`
- `get-result.schema.json` -- saída do `atomwrite get` (v0.1.12, v14 Tier 3: leitura de chave única; `value` auto-parseado)
- `hash-output.schema.json` -- saída do `atomwrite hash`
- `count-summary.schema.json` -- saída do `atomwrite count`
- `diff-output.schema.json` -- saída do `atomwrite diff`
- `move-output.schema.json` -- saída do `atomwrite move`
- `copy-output.schema.json` -- saída do `atomwrite copy`
- `list-entry.schema.json` -- saída do `atomwrite list` (evento por entrada)
- `extract-output.schema.json` -- saída do `atomwrite extract`
- `calc-output.schema.json` -- saída do `atomwrite calc`
- `regex-output.schema.json` -- saída do `atomwrite regex`
- `transform-result.schema.json` -- saída do `atomwrite transform`
- `outline-output.schema.json` -- saída do `atomwrite outline` (v0.1.12, v14 Tier 3: extração de estrutura de alto nível; `items[].kind`, `items[].name`)
- `query-output.schema.json` -- saída do `atomwrite query` (v0.1.12, v14 Tier 3: query tree-sitter AST; oneOf `kinds`|`tree`|`matches`)
- `batch-summary.schema.json` -- saída do `atomwrite batch` (evento de resumo)
- `scope-result.schema.json` -- saída do `atomwrite scope` (evento por arquivo)
- `backup-result.schema.json` -- saída do `atomwrite backup` (evento por arquivo)
- `rollback-result.schema.json` -- saída do `atomwrite rollback`
- `set-result.schema.json` -- saída do `atomwrite set` (v0.1.12, v14 Tier 3: escrita de chave via dotted-path; `action: set`)
- `wal-recovery.schema.json` -- saída da API consultiva `recover_orphan_journals` (v0.1.12, G114: `JournalEntry::{Started, Committed, Aborted}` reports)
- `apply-result.schema.json` -- saída do `atomwrite apply`
- `case-result.schema.json` -- saída do `atomwrite case` (v0.1.12, v14 Tier 3: conversão de case de identificadores; `files_modified`, `identifiers_renamed`)
- `del-result.schema.json` -- saída do `atomwrite del` (v0.1.12, v14 Tier 3: deleção de chave; `action: deleted|already_missing`)
- `get-result.schema.json` -- saída do `atomwrite get` (v0.1.12, v14 Tier 3: leitura de chave única; `value` auto-parseado)
- `error-output.schema.json` -- envelope de erro emitido por todos os subcomandos (v0.1.15: adiciona `failed_pair_index`, `pairs_total`, `pair_results` -- G117)
- `count-by-size-output.schema.json` -- saída do `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N arquivos por tamanho decrescente; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- telemetria de risco aninhada na saída do `atomwrite write` (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)
- `edit-loop-output.schema.json` -- saída do `atomwrite edit-loop` (v0.1.22: N pares `{old, new}` em 1 invocação via NDJSON; `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`)
- `prune-backups-output.schema.json` -- saída do `atomwrite prune-backups` (v0.1.22: linha por backup + summary; `path`, `reason`, `action`, `total`, `elapsed_ms`)
