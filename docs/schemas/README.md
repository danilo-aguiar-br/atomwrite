# atomwrite JSON Schemas

_Last updated: 2026-07-31 (v0.1.36 residual CAPA) — error-output `ok:false` always-emit; `scratch.schema.json`; agent-native `--select`/`--filter`/`--limit` (runtime shape; errors never filter-dropped; no global `-n`); surface SSOT **45** tools via `agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`; ADR-0055_

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
- `hash-output.schema.json` -- output of `atomwrite hash` (also reused by `atomwrite verify`, which delegates to `hash --verify`)
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
- `wal-recovery.schema.json` -- output of `recover_orphan_journals` consultive API and `atomwrite wal-heal` recovery reports (v0.1.12, G114: `JournalEntry::{Started, Committed, Aborted}` reports)
- `apply-result.schema.json` -- output of `atomwrite apply`
- `case-result.schema.json` -- output of `atomwrite case` (v0.1.12, v14 Tier 3: identifier case conversion; `files_modified`, `identifiers_renamed`)
- `del-result.schema.json` -- output of `atomwrite del` (v0.1.12, v14 Tier 3: key deletion; `action: deleted|already_missing`)
- `error-output.schema.json` -- error envelope emitted by all subcommands (v0.1.15: adds `failed_pair_index`, `pairs_total`, `pair_results` -- G117; v0.1.4: adds `suggestion`; v0.1.29: adds `best_candidate`; v0.1.36: required `ok: false` + present `match_count`)
- `wal-stats-output.schema.json` -- output of `atomwrite wal-stats` (v0.1.16: G119 L5 local diagnostics; `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `by_directory`, `auto_heal_recommended`, `estimated_reclaim_bytes`)
- `count-by-size-output.schema.json` -- output of `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N files by descending size; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- nested risk assessment fields in `atomwrite write` output (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)
- `edit-loop-output.schema.json` -- output of `atomwrite edit-loop` (v0.1.22: N `{old, new}` pairs in 1 invocation via NDJSON; `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`, `pair_results[].old`, `pair_results[].new`)
- `prune-backups-output.schema.json` -- output of `atomwrite prune-backups` (v0.1.22: per-backup line + summary; `path`, `reason`, `action`, `total`, `elapsed_ms`)
- `best-candidate.schema.json` -- nested near-miss diagnostic on match failure (v0.1.29 P0-2; also referenced by `error-output`)
- `progress-event.schema.json` -- progress heartbeats for `replace`/`batch` (v0.1.29 P1-3)
- `cancelled-event.schema.json` -- cooperative cancel event (v0.1.29 P0-4, process exit **143**; distinct from timeout deadline exit **124** via **error-output**)
- `recipe-result.schema.json` -- output of `atomwrite recipe list|run` (v0.1.29 P1-4)
- `scratch.schema.json` -- NDJSON success envelope for `atomwrite scratch init` (agent staging dir under workspace jail; fields `ok`, `status`, `command`, `action`, `path`, `relative`, `already_existed`)
- `doctor.schema.json` -- output of `atomwrite doctor` host diagnostics
- `locale.schema.json` -- output of `atomwrite locale` (resolved locale / preference report)
- `commands.schema.json` -- output of `atomwrite commands` (full command-tree manifesto)
- `completions.schema.json` -- output of `atomwrite completions` (shell completion generation; optional `--install`)
- `agent-surface.schema.json` -- output of `atomwrite agent-surface` (G-042 minimal published schema; additionalProperties allowed for evolution)
- `sparse.schema.json` -- output of `atomwrite sparse` (budgeted monorepo list/read/outline; G-042 minimal envelope)
- `semantic-merge.schema.json` -- output of `atomwrite semantic-merge` (three-way line-based merge; G-042 minimal envelope)
- `semantic-search.schema.json` -- output of `atomwrite semantic-search` (offline token ranking; G-042 minimal envelope; feature `semantic`)
- `codemod.schema.json` -- output of `atomwrite codemod` (multi-rule AST campaign; G-042 minimal envelope)
- `watch.schema.json` -- output of `atomwrite watch` (filesystem events + final `type:watch_summary`; G-042 minimal envelope; feature `watch`)

### Error envelope contract (`error-output.schema.json`)
- **Required** on every error line: `error`, **`ok`**, `code`, `exit`, `message`, `error_class`, `retryable`
- **`ok` is always `false`** on error envelopes (GAP-2026-020 agent sentinel) — do not treat a missing success field as “all null success”
- **`match_count` is present** on match-related failures: `0` on most failures (`MATCH_FAILED` / `NO_MATCHES`); non-zero when ambiguity reports a count
- Optional near-miss: `best_candidate` (v0.1.29+; also on `MATCH_AMBIGUOUS`)
- Multi-pair edit (G117): `failed_pair_index`, `pairs_total`, `pair_results`

### Runtime shape flags (NOT schema fields)
- Global `--select` / `--fields`, `--filter`, and `--limit` reshape **stdout emission at runtime** (agent-native projection / filter / bound)
- They are **not** properties of any NDJSON schema; schemas describe the full object before projection
- Errors are **always-emit**: `--filter` must not drop `ok: false` envelopes; `--limit` counts success events only
- No global `-n` for limit (`-n` belongs to `replace`)

### v0.1.29 subcommand schemas (updated)
- Dedicated G-042 minimal envelopes (now indexed): `agent-surface`, `sparse`, `semantic-merge`, `semantic-search`, `codemod`, `watch`
- Nested / event schemas from 0.1.29: `best-candidate`, `progress-event`, `cancelled-event`, `recipe-result` (plus `write` / `replace` / `error` field extensions)
- `stat` reuses `read-output.schema.json` (alias of `read --stat`)

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

### v0.1.35 residual contract (no envelope schema change for A-*)
- A-* residuals closed without new NDJSON schema fields: large write `--ack-overwrite`, `delete --plan` (not `--confirm`/`--yes`), semantic-merge markers default ON
- `watch` always ends with a final NDJSON event `type:watch_summary` (even idle / zero events)
- `MATCH_FAILED` remains the **error-output** code for edit/fuzzy miss (exit 65) with optional `best_candidate`
- Contract suite: `cargo test --test cli_e2e_v0135`

### v0.1.36 residual CAPA (ADR-0055)
- `error-output.schema.json` requires `ok` (const false on errors) and documents `match_count`
- `scratch.schema.json` indexes `scratch init` staging under the workspace jail
- Agent surface SSOT **45** tools (`agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`) — includes `scratch`, `doctor`, `locale`, `commands`
- Schema coverage note: `verify` reuses `hash-output.schema.json`; `wal-heal` reports align with `wal-recovery.schema.json`; full tool list is 45 (`agent-surface`); `completions` has dedicated `completions.schema.json`
- Fuzzy honesty is **behavioral** (normalized strategies emit `similarity: null`; dual-gate `block_anchor`; XDG `[fuzzy].soft_floor_delta`) — no invented NDJSON fields beyond shipped schemas
- Global shape flags remain runtime-only (see above)
- Contract suite: `cargo test --test cli_v0136_gaps`; SSOT: `scripts/check-gaps-ssot.sh`

## Português
### Última atualização: 2026-07-31 (v0.1.36 residual CAPA) — error-output `ok:false` always-emit; `scratch.schema.json`; shape agent-native `--select`/`--filter`/`--limit` (runtime; erros nunca dropados por filter; sem `-n` global); superfície SSOT **45** tools via `agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`; ADR-0055

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
- `hash-output.schema.json` -- saída do `atomwrite hash` (também reutilizado por `atomwrite verify`, que delega a `hash --verify`)
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
- `wal-recovery.schema.json` -- saída da API consultiva `recover_orphan_journals` e relatórios de recuperação do `atomwrite wal-heal` (v0.1.12, G114: `JournalEntry::{Started, Committed, Aborted}` reports)
- `apply-result.schema.json` -- saída do `atomwrite apply`
- `case-result.schema.json` -- saída do `atomwrite case` (v0.1.12, v14 Tier 3: conversão de case de identificadores; `files_modified`, `identifiers_renamed`)
- `del-result.schema.json` -- saída do `atomwrite del` (v0.1.12, v14 Tier 3: deleção de chave; `action: deleted|already_missing`)
- `error-output.schema.json` -- envelope de erro emitido por todos os subcomandos (v0.1.15: adiciona `failed_pair_index`, `pairs_total`, `pair_results` -- G117; v0.1.4: adiciona `suggestion`; v0.1.29: adiciona `best_candidate`; v0.1.36: `ok` obrigatório `false` + `match_count`)
- `wal-stats-output.schema.json` -- saída do `atomwrite wal-stats` (v0.1.16: diagnósticos locais G119 L5; `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `by_directory`, `auto_heal_recommended`, `estimated_reclaim_bytes`)
- `count-by-size-output.schema.json` -- saída do `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N arquivos por tamanho decrescente; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- campos aninhados de avaliação de risco na saída do `atomwrite write` (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)
- `edit-loop-output.schema.json` -- saída do `atomwrite edit-loop` (v0.1.22: N pares `{old, new}` em 1 invocação via NDJSON; `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`, `pair_results[].old`, `pair_results[].new`)
- `prune-backups-output.schema.json` -- saída do `atomwrite prune-backups` (v0.1.22: linha por backup + summary; `path`, `reason`, `action`, `total`, `elapsed_ms`)
- `best-candidate.schema.json` -- diagnóstico de near-miss em falha de match (v0.1.29 P0-2; também referenciado por `error-output`)
- `progress-event.schema.json` -- heartbeats de progresso de `replace`/`batch` (v0.1.29 P1-3)
- `cancelled-event.schema.json` -- evento de cancel cooperativo (v0.1.29 P0-4, process exit **143**; distinto do prazo de timeout exit **124** via **error-output**)
- `recipe-result.schema.json` -- saída do `atomwrite recipe list|run` (v0.1.29 P1-4)
- `scratch.schema.json` -- envelope de sucesso NDJSON de `atomwrite scratch init` (dir de staging do agente sob o jail do workspace; campos `ok`, `status`, `command`, `action`, `path`, `relative`, `already_existed`)
- `doctor.schema.json` -- saída do `atomwrite doctor` (diagnóstico do host)
- `locale.schema.json` -- saída do `atomwrite locale` (relatório de locale resolvido / preferência)
- `commands.schema.json` -- saída do `atomwrite commands` (manifesto completo da árvore de comandos)
- `completions.schema.json` -- saída do `atomwrite completions` (geração de completions de shell; `--install` opcional)
- `agent-surface.schema.json` -- saída do `atomwrite agent-surface` (schema mínimo publicado G-042; additionalProperties permitido para evolução)
- `sparse.schema.json` -- saída do `atomwrite sparse` (list/read/outline monorepo orçado; envelope mínimo G-042)
- `semantic-merge.schema.json` -- saída do `atomwrite semantic-merge` (merge three-way baseado em linhas; envelope mínimo G-042)
- `semantic-search.schema.json` -- saída do `atomwrite semantic-search` (ranking de tokens offline; envelope mínimo G-042; feature `semantic`)
- `codemod.schema.json` -- saída do `atomwrite codemod` (campanha AST multi-regra; envelope mínimo G-042)
- `watch.schema.json` -- saída do `atomwrite watch` (eventos de filesystem + final `type:watch_summary`; envelope mínimo G-042; feature `watch`)

### Contrato do envelope de erro (`error-output.schema.json`)
- **Obrigatórios** em toda linha de erro: `error`, **`ok`**, `code`, `exit`, `message`, `error_class`, `retryable`
- **`ok` é sempre `false`** em envelopes de erro (sentinela de agente GAP-2026-020) — não trate campo de sucesso ausente como “sucesso todo null”
- **`match_count` está presente** em falhas de match: `0` na maioria das falhas (`MATCH_FAILED` / `NO_MATCHES`); não-zero quando ambiguidade reporta contagem
- Near-miss opcional: `best_candidate` (v0.1.29+; também em `MATCH_AMBIGUOUS`)
- Edit multi-par (G117): `failed_pair_index`, `pairs_total`, `pair_results`

### Flags de shape em runtime (NÃO são campos de schema)
- Globais `--select` / `--fields`, `--filter` e `--limit` remodelam a **emissão de stdout em runtime** (projeção / filtro / teto agent-native)
- **Não** são propriedades de nenhum schema NDJSON; schemas descrevem o objeto completo antes da projeção
- Erros são **always-emit**: `--filter` não deve silenciar envelopes `ok: false`; `--limit` conta só eventos de sucesso
- Sem `-n` global para limit (`-n` pertence ao `replace`)

### Schemas de subcomandos 0.1.29 (atualizado)
- Envelopes mínimos G-042 dedicados (agora indexados): `agent-surface`, `sparse`, `semantic-merge`, `semantic-search`, `codemod`, `watch`
- Schemas aninhados / de evento da 0.1.29: `best-candidate`, `progress-event`, `cancelled-event`, `recipe-result` (mais extensões de campos em `write` / `replace` / `error`)
- `stat` reutiliza `read-output.schema.json` (alias de `read --stat`)

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

### Contrato residual v0.1.35 (sem mudança de schema de envelope para A-*)
- Residuais A-* fechados sem novos campos de schema NDJSON: write grande `--ack-overwrite`, `delete --plan` (não `--confirm`/`--yes`), markers de semantic-merge default ON
- `watch` sempre termina com evento NDJSON final `type:watch_summary` (mesmo idle / zero eventos)
- `MATCH_FAILED` permanece o código de **error-output** para miss de edit/fuzzy (exit 65) com `best_candidate` opcional
- Suíte de contrato: `cargo test --test cli_e2e_v0135`

### CAPA residual v0.1.36 (ADR-0055)
- `error-output.schema.json` exige `ok` (sempre `false` em erros) e documenta `match_count`
- `scratch.schema.json` indexa `scratch init` de staging sob o jail do workspace
- Superfície de agente SSOT **45** tools (`agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`) — inclui `scratch`, `doctor`, `locale`, `commands`
- Nota de cobertura de schemas: `verify` reutiliza `hash-output.schema.json`; relatórios de `wal-heal` alinham com `wal-recovery.schema.json`; lista completa de tools é 45 (`agent-surface`); `completions` tem `completions.schema.json` dedicado
- Honestidade fuzzy é **comportamental** (estratégias normalizadas emitem `similarity: null`; dual-gate `block_anchor`; XDG `[fuzzy].soft_floor_delta`) — sem inventar campos NDJSON além dos schemas embarcados
- Flags de shape global permanecem só de runtime (ver acima)
- Suíte de contrato: `cargo test --test cli_v0136_gaps`; SSOT: `scripts/check-gaps-ssot.sh`
