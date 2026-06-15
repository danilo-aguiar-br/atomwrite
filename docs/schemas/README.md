# atomwrite JSON Schemas


## English
### Purpose
- Each schema describes the NDJSON output of one atomwrite subcommand
- All schemas follow JSON Schema draft-07
- Use these schemas to validate agent-consumed output programmatically

### Schema Index
- `write-output.schema.json` -- output of `atomwrite write`
- `read-output.schema.json` -- output of `atomwrite read`
- `edit-output.schema.json` -- output of `atomwrite edit` (v0.1.15: adds `pairs_total` and `pair_results` -- G117)
- `search-match.schema.json` -- output of `atomwrite search` (per-match event)
- `replace-result.schema.json` -- output of `atomwrite replace` (per-file event)
- `delete-output.schema.json` -- output of `atomwrite delete`
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
- `batch-summary.schema.json` -- output of `atomwrite batch` (summary event)
- `scope-result.schema.json` -- output of `atomwrite scope` (per-file event)
- `backup-result.schema.json` -- output of `atomwrite backup` (per-file event)
- `rollback-result.schema.json` -- output of `atomwrite rollback`
- `apply-result.schema.json` -- output of `atomwrite apply`
- `error-output.schema.json` -- error envelope emitted by all subcommands (v0.1.15: adds `failed_pair_index`, `pairs_total`, `pair_results` -- G117)
- `wal-stats-output.schema.json` -- output of `atomwrite wal-stats` (v0.1.16: G119 L5 telemetry; `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `by_directory`, `auto_heal_recommended`, `estimated_reclaim_bytes`)
- `count-by-size-output.schema.json` -- output of `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N files by descending size; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- nested risk telemetry in `atomwrite write` output (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)


## Portugues
### Objetivo
- Cada schema descreve a saída NDJSON de um subcomando do atomwrite
- Todos os schemas seguem JSON Schema draft-07
- Use estes schemas para validar saída consumida por agentes de forma programática

### Índice de Schemas
- `write-output.schema.json` -- saída do `atomwrite write`
- `read-output.schema.json` -- saída do `atomwrite read`
- `edit-output.schema.json` -- saída do `atomwrite edit` (v0.1.15: adiciona `pairs_total` e `pair_results` -- G117)
- `search-match.schema.json` -- saída do `atomwrite search` (evento por match)
- `replace-result.schema.json` -- saída do `atomwrite replace` (evento por arquivo)
- `delete-output.schema.json` -- saída do `atomwrite delete`
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
- `batch-summary.schema.json` -- saída do `atomwrite batch` (evento de resumo)
- `scope-result.schema.json` -- saída do `atomwrite scope` (evento por arquivo)
- `backup-result.schema.json` -- saída do `atomwrite backup` (evento por arquivo)
- `rollback-result.schema.json` -- saída do `atomwrite rollback`
- `apply-result.schema.json` -- saída do `atomwrite apply`
- `error-output.schema.json` -- envelope de erro emitido por todos os subcomandos (v0.1.15: adiciona `failed_pair_index`, `pairs_total`, `pair_results` -- G117)
- `count-by-size-output.schema.json` -- saída do `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N arquivos por tamanho decrescente; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- telemetria de risco aninhada na saída do `atomwrite write` (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)
