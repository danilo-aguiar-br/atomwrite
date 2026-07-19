# atomwrite JSON Schemas

_Last updated: 2026-07-19 (v0.1.34) — 38 schemas in index; no envelope change since v0.1.30; one-shot runtime is algorithmic only_

## English
### Purpose
- Each schema describes the NDJSON output of one atomwrite subcommand
- All schemas follow JSON Schema draft/2020-12 (including `error-output.schema.json`)
- Use these schemas to validate agent-consumed output programmatically

### Schema Index
- `write-output.schema.json` -- output of `atomwrite write` (v0.1.29: `platform.durability`, `platform.rename_method`, `platform.backup_method`; v0.1.30: `backup_method` is `reflink_or_copy` or `copy`, never hardlink of live file)
- `read-output.schema.json` -- output of `atomwrite read`
- `edit-output.schema.json` -- output of `atomwrite edit` (v0.1.15: `pairs_total`/`pair_results` -- G117) (v0.1.23: `source` in `pair_results`) (v0.1.30 residual: `match_count`, `indent_adjusted`)
- `search-match.schema.json` -- output of `atomwrite search` (per-match event)
- `replace-result.schema.json` -- output of `atomwrite replace` (per-file event) (v0.1.29: `fuzzy`, `strategy`, `similarity`, `strategies_tried`)
- `delete-output.schema.json` -- output of `atomwrite delete` (v0.1.28: adds `warnings`)
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
- `error-output.schema.json` -- error envelope emitted by all subcommands (v0.1.15: adds `failed_pair_index`, `pairs_total`, `pair_results` -- G117; v0.1.4: adds `suggestion`; v0.1.29: adds `best_candidate`)
- `wal-stats-output.schema.json` -- output of `atomwrite wal-stats` (v0.1.16: G119 L5 local diagnostics; `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `by_directory`, `auto_heal_recommended`, `estimated_reclaim_bytes`)
- `count-by-size-output.schema.json` -- output of `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N files by descending size; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- nested risk assessment fields in `atomwrite write` output (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)
- `edit-loop-output.schema.json` -- output of `atomwrite edit-loop` (v0.1.22: N `{old, new}` pairs in 1 invocation via NDJSON; `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`, `pair_results[].old`, `pair_results[].new`)
- `prune-backups-output.schema.json` -- output of `atomwrite prune-backups` (v0.1.22: per-backup line + summary; `path`, `reason`, `action`, `total`, `elapsed_ms`)
- `best-candidate.schema.json` -- nested near-miss diagnostic on match failure (v0.1.29 P0-2; also referenced by `error-output`)
- `progress-event.schema.json` -- progress heartbeats for `replace`/`batch` (v0.1.29 P1-3)
- `cancelled-event.schema.json` -- cooperative cancel event (v0.1.29 P0-4, process exit **143**; distinct from timeout deadline exit **124** via **error-output**)
- `recipe-result.schema.json` -- output of `atomwrite recipe list|run` (v0.1.29 P1-4)

### v0.1.29 subcommands without dedicated schemas
- Subcommands added in 0.1.29 (`sparse`, `semantic-merge`, `agent-surface`, `watch`, `codemod`, `semantic-search`, `stat`) reuse existing envelopes or emit types documented in the binary `help` / `--json-schema`
- Dedicated schemas listed for 0.1.29: `best-candidate`, `progress-event`, `cancelled-event`, `recipe-result` (plus `write` / `replace` / `error` field extensions)

### v0.1.30 residual schema updates
- `edit-output.schema.json` regenerated with `match_count` (uint64|null) and `indent_adjusted` (bool|null)
- `write-output.schema.json` documents honest `platform.backup_method` values (`reflink_or_copy` | `copy`)
- Regenerate with `scripts/regen-schemas.sh` or `atomwrite --json-schema edit` / write as needed
- Contract suite: `cargo test --test cli_v0130_agent_contract`

### v0.1.34 behavioral contract (no envelope change)
- JSON schemas intentionally **unchanged** in v0.1.33/v0.1.34 — one-shot fuzzy + timeout are algorithmic, not new NDJSON fields
- Fuzzy multi-apply is **one-pass** L→R on original content (`apply_fuzzy_one_pass`); never re-scans inserted text
- Default max fuzzy applies = **1** when `--max-replacements` omitted; hard ceiling 10_000; embeds force single apply
- Global `--timeout-secs` / `--timeout` default **120**; `0` disables; deadline → process exit **124** via **error-output** envelope (generic `exit` field)
- Cooperative cancel (SIGINT/SIGTERM) → **cancelled-event** schema, process exit **143** — distinct from timeout **124**
- See ADR-0054 (`docs/decisions/0054-v0-1-34-oneshot-fuzzy-timeout.md`) and `cargo test --test cli_v0133_oneshot_fuzzy`

## Português
### Última atualização: 2026-07-19 (v0.1.34) — 38 schemas no índice; sem mudança de envelope desde v0.1.30; runtime one-shot é apenas algorítmico

### Objetivo
- Cada schema descreve a saída NDJSON de um subcomando do atomwrite
- Todos os schemas seguem JSON Schema draft/2020-12 (incluindo `error-output.schema.json`)
- Use estes schemas para validar saída consumida por agentes de forma programática

### Índice de Schemas
- `write-output.schema.json` -- saída do `atomwrite write` (v0.1.29: `platform.durability`, `platform.rename_method`, `platform.backup_method`; v0.1.30: `backup_method` é `reflink_or_copy` ou `copy`, nunca hardlink do arquivo vivo)
- `read-output.schema.json` -- saída do `atomwrite read`
- `edit-output.schema.json` -- saída do `atomwrite edit` (v0.1.15: `pairs_total`/`pair_results` -- G117) (v0.1.23: `source` em `pair_results`) (residual v0.1.30: `match_count`, `indent_adjusted`)
- `search-match.schema.json` -- saída do `atomwrite search` (evento por match)
- `replace-result.schema.json` -- saída do `atomwrite replace` (evento por arquivo) (v0.1.29: `fuzzy`, `strategy`, `similarity`, `strategies_tried`)
- `delete-output.schema.json` -- saída do `atomwrite delete` (v0.1.28: adiciona `warnings`)
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
- `error-output.schema.json` -- envelope de erro emitido por todos os subcomandos (v0.1.15: adiciona `failed_pair_index`, `pairs_total`, `pair_results` -- G117; v0.1.4: adiciona `suggestion`; v0.1.29: adiciona `best_candidate`)
- `wal-stats-output.schema.json` -- saída do `atomwrite wal-stats` (v0.1.16: diagnósticos locais G119 L5; `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `by_directory`, `auto_heal_recommended`, `estimated_reclaim_bytes`)
- `count-by-size-output.schema.json` -- saída do `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N arquivos por tamanho decrescente; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- campos aninhados de avaliação de risco na saída do `atomwrite write` (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)
- `edit-loop-output.schema.json` -- saída do `atomwrite edit-loop` (v0.1.22: N pares `{old, new}` em 1 invocação via NDJSON; `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`, `pair_results[].old`, `pair_results[].new`)
- `prune-backups-output.schema.json` -- saída do `atomwrite prune-backups` (v0.1.22: linha por backup + summary; `path`, `reason`, `action`, `total`, `elapsed_ms`)
- `best-candidate.schema.json` -- diagnóstico de near-miss em falha de match (v0.1.29 P0-2; também referenciado por `error-output`)
- `progress-event.schema.json` -- heartbeats de progresso de `replace`/`batch` (v0.1.29 P1-3)
- `cancelled-event.schema.json` -- evento de cancel cooperativo (v0.1.29 P0-4, process exit **143**; distinto do prazo de timeout exit **124** via **error-output**)
- `recipe-result.schema.json` -- saída do `atomwrite recipe list|run` (v0.1.29 P1-4)

### Subcomandos 0.1.29 sem schema dedicado
- Subcomandos adicionados na 0.1.29 (`sparse`, `semantic-merge`, `agent-surface`, `watch`, `codemod`, `semantic-search`, `stat`) reutilizam envelopes existentes ou emitem tipos documentados no `help` / `--json-schema` do binário
- Schemas dedicados listados para 0.1.29: `best-candidate`, `progress-event`, `cancelled-event`, `recipe-result` (mais extensões de campos em `write` / `replace` / `error`)

### Atualizações de schema residual v0.1.30
- `edit-output.schema.json` regenerado com `match_count` (uint64|null) e `indent_adjusted` (bool|null)
- `write-output.schema.json` documenta valores honestos de `platform.backup_method` (`reflink_or_copy` | `copy`)
- Regenere com `scripts/regen-schemas.sh` ou `atomwrite --json-schema edit` / write quando preciso
- Suíte de contrato: `cargo test --test cli_v0130_agent_contract`

### Contrato comportamental v0.1.34 (sem mudança de envelope)
- Schemas JSON intencionalmente **inalterados** em v0.1.33/v0.1.34 — fuzzy one-shot + timeout são algorítmicos, não novos campos NDJSON
- Multi-apply fuzzy é **one-pass** E→D no conteúdo original (`apply_fuzzy_one_pass`); nunca reescaneia texto inserido
- Máximo de applies fuzzy padrão = **1** quando `--max-replacements` omitido; teto rígido 10_000; embeds forçam apply único
- Global `--timeout-secs` / `--timeout` padrão **120**; `0` desabilita; prazo → process exit **124** via envelope **error-output** (campo genérico `exit`)
- Cancel cooperativo (SIGINT/SIGTERM) → schema **cancelled-event**, process exit **143** — distinto do timeout **124**
- Ver ADR-0054 (`docs/decisions/0054-v0-1-34-oneshot-fuzzy-timeout.md`) e `cargo test --test cli_v0133_oneshot_fuzzy`
