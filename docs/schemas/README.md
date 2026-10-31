# atomwrite JSON Schemas


## English
### Update
- Last updated: 2026-10-31 (v0.1.37)
- 55 schema names served by `atomwrite --print-schema`, backed by 54 files in this directory: 54 files are the schema of one served name (two names, `read` and `stat`, share `read-output.schema.json`), and `write-risk` serves `write-risk-assessment.schema.json`, so the nested risk object has a door of its own and no file is an orphan
- Each file is meant to be identical to what the binary serves; the refresh procedure and the name-to-file map are below
- New in v0.1.37: `config.schema.json`; `extract-artifact.schema.json`; `count-by-ext-output` and `count-by-size-output`; one union per verb, where `recipe`, `codemod`, `batch`, `search`, `replace`, `list`, `backup`, `semantic-search`, `sparse`, `semantic-merge`, `case` and `watch` serve every record they write plus the error envelope in one document; success events list the optional `ok`, `code`, `exit`, `target_resolved` and `target_source`, plus the cut counters `records_suppressed`, `files_budget_dropped`, `files_planned`, `truncated` and `truncated_records`, where `records_suppressed` counts records and `files_budget_dropped` counts entries in the unit the verb produces
- Surface SSOT: 47 tools via `agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`

### Purpose
- Each schema describes the NDJSON output of one atomwrite subcommand
- All schemas follow JSON Schema draft/2020-12 (including `error-output.schema.json`)
- Use these schemas to validate agent-consumed output programmatically

### Schema Index
- `write-output.schema.json` -- output of `atomwrite write` (v0.1.29: `platform.durability`, `platform.rename_method`, `platform.backup_method`; v0.1.30: `backup_method` is `reflink_or_copy` or `copy`, never hardlink of live file)
- `read-output.schema.json` -- output of `atomwrite read`
- `edit-output.schema.json` -- output of `atomwrite edit` (v0.1.15: `pairs_total`/`pair_results` -- G117) (v0.1.23: `source` in `pair_results`) (v0.1.30 residual: `match_count`, `indent_adjusted`)
- `search-output.schema.json` -- output of `atomwrite search` (`--print-schema search`; union of `begin`, `context`, `file`, `match`, `end`, the terminal summary and the error envelope)
- `replace-output.schema.json` -- output of `atomwrite replace` (`--print-schema replace`; union of `plan`, `replace`, the terminal summary and the error envelope) (v0.1.29: `fuzzy`, `strategy`, `similarity`, `strategies_tried`)
- `delete-output.schema.json` -- output of `atomwrite delete` (v0.1.28: adds `warnings`)
- `get-result.schema.json` -- output of `atomwrite get` (v0.1.12, v14 Tier 3: single key read; `value` auto-parsed)
- `hash-output.schema.json` -- output of `atomwrite hash` (`verify` has its own schema, `hash-verify.schema.json`)
- `count-summary.schema.json` -- output of `atomwrite count` (total mode; `--print-schema count`, struct `CountTotalOutput`)
- `count-by-ext-output.schema.json` -- output of `atomwrite count --by-extension` (v0.1.37; `by_extension`; `--print-schema count-by-ext`)
- `diff-output.schema.json` -- output of `atomwrite diff`
- `move-output.schema.json` -- output of `atomwrite move`
- `copy-output.schema.json` -- output of `atomwrite copy`
- `list-output.schema.json` -- output of `atomwrite list` (`--print-schema list`; union of `entry`, the terminal summary and the error envelope)
- `extract-output.schema.json` -- output of `atomwrite extract` (`--print-schema extract` serves a union of four record shapes: an NDJSON object narrowed to the selected fields, an NDJSON object passed through whole, and the two text shapes `{type: "text", fields: [...]}` and `{type: "text", values: [...]}`; the two object branches stay open because the input line decides the keys)
- `calc-output.schema.json` -- output of `atomwrite calc`
- `regex-output.schema.json` -- output of `atomwrite regex`
- `transform-result.schema.json` -- output of `atomwrite transform`
- `outline-output.schema.json` -- output of `atomwrite outline` (v0.1.12, v14 Tier 3: high-level structure extraction; `items[].kind`, `items[].name`)
- `query-output.schema.json` -- output of `atomwrite query` (v0.1.12, v14 Tier 3: tree-sitter AST query; oneOf `kinds`|`tree`|`matches`)
- `batch-output.schema.json` -- output of `atomwrite batch` (`--print-schema batch`; union of `batch_op`, `progress`, `rollback`, the terminal summary, the manifest schema of `--input-schema` and the error envelope)
- `scope-result.schema.json` -- output of `atomwrite scope` (per-file event)
- `backup-output.schema.json` -- output of `atomwrite backup` (`--print-schema backup`; union of `backup` or `plan`, the terminal summary and the error envelope)
- `rollback-result.schema.json` -- output of `atomwrite rollback`
- `set-result.schema.json` -- output of `atomwrite set` (v0.1.12, v14 Tier 3: dotted-path key write; `action: set`)
- `wal-recovery.schema.json` -- output of `recover_orphan_journals` consultive API and `atomwrite wal-heal` recovery reports (v0.1.12, G114: `JournalEntry::{Started, Committed, Aborted}` reports)
- `apply-result.schema.json` -- output of `atomwrite apply`
- `case-output.schema.json` -- output of `atomwrite case` (`--print-schema case`; union of `case` or `case_preview`, the terminal summary and the error envelope) (v0.1.12, v14 Tier 3: identifier case conversion; `files_modified`, `identifiers_renamed`)
- `del-result.schema.json` -- output of `atomwrite del` (v0.1.12, v14 Tier 3: key deletion; `action: deleted|already_missing`)
- `error-output.schema.json` -- error envelope emitted by all subcommands (v0.1.37: declares `target_resolved` and `target_source`, because the writer stamps the resolved subject on the failure path too; v0.1.15: adds `failed_pair_index`, `pairs_total`, `pair_results` -- G117; v0.1.4: adds `suggestion`; v0.1.29: adds `best_candidate`; v0.1.36: required `ok: false` + present `match_count`)
- `wal-stats-output.schema.json` -- output of `atomwrite wal-stats` (v0.1.16: G119 L5 local diagnostics; `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `by_directory`, `auto_heal_recommended`, `estimated_reclaim_bytes`)
- `count-by-size-output.schema.json` -- output of `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N files by descending size; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- risk assessment object of the `atomwrite write` output, served under its own name `write-risk` (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)
- `edit-loop-output.schema.json` -- output of `atomwrite edit-loop` (v0.1.22: N `{old, new}` pairs in 1 invocation via NDJSON; `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`, `pair_results[].old`, `pair_results[].new`)
- `prune-backups-output.schema.json` -- output of `atomwrite prune-backups` (v0.1.37: per-backup line, then the terminal summary; the line carries `path`, `reason` and `action`, and the summary carries `total`, `elapsed_ms`, `files_failed`, `partial` and `errors[]`)
- `best-candidate.schema.json` -- nested near-miss diagnostic on match failure (v0.1.29 P0-2; also referenced by `error-output`)
- `progress-event.schema.json` -- progress heartbeats for `replace`/`batch` (v0.1.29 P1-3)
- `cancelled-event.schema.json` -- terminal record of an interrupted scan, emitted by every scanning verb (v0.1.29 P0-4 added the event; v0.1.37 added `deadline_exceeded`, which now makes the deadline and signal cases distinct inside this schema instead of only through error-output)
- `recipe-output.schema.json` -- output of `atomwrite recipe list|run` (`--print-schema recipe`; union of the listing, every child record tagged with `step`, the step records, the terminal result and the error envelope) (v0.1.29 P1-4)
- `config.schema.json` -- NDJSON success envelope for `atomwrite config path|list|get|set|unset` (persistent tuning keys; fields `type`, `command`, `action`, `scope`, `config_path`, `exists`, `key`, `value`, `origin`, `default`, `old_value`, `changed`, `keys`, `warnings`; `ok`, `code`, `exit`, `target_resolved` and `target_source` are the optional keys the writer stamps, the last two on `path`, `set` and `unset`)

- `scratch.schema.json` -- NDJSON success envelope for `atomwrite scratch init` (agent staging dir under workspace jail; fields `ok`, `status`, `command`, `action`, `path`, `relative`, `already_existed`)
- `doctor.schema.json` -- output of `atomwrite doctor` host diagnostics
- `locale.schema.json` -- output of `atomwrite locale` (resolved locale / preference report)
- `commands.schema.json` -- output of `atomwrite commands` (full command-tree manifesto)
- `completions.schema.json` -- the `installed` event of `atomwrite completions SHELL --install` (`type`, `path`; `--print-schema completions`). Without `--install` the command prints the shell script as plain text and emits no NDJSON, so there is no other record to describe
- `agent-surface.schema.json` -- output of `atomwrite agent-surface` (the manifest: `tools`, `exit_codes`, `discovery`; `--print-schema agent-surface`)
- `sparse.schema.json` -- output of `atomwrite sparse` (budgeted monorepo list/read/outline; `--print-schema sparse`)
- `semantic-merge.schema.json` -- output of `atomwrite semantic-merge` (three-way line-based merge; `--print-schema semantic-merge`)
- `semantic-search.schema.json` -- output of `atomwrite semantic-search` (offline token ranking; `--print-schema semantic-search`)
- `codemod.schema.json` -- output of `atomwrite codemod` (multi-rule AST campaign; `--print-schema codemod`)
- `extract-artifact.schema.json` -- output of `atomwrite extract-artifact` (v0.1.37; bounded signal from PDF, OOXML and image headers; `--print-schema extract-artifact`)
- `watch.schema.json` -- output of `atomwrite watch` (filesystem events + final `type:watch_summary`; feature `watch`)
- `hash-verify.schema.json` -- output of `atomwrite verify` and `hash --verify` (one `hash_verify` event carrying the whole verdict; `--print-schema verify`)
- `stream-diff.schema.json` -- the single `stream_diff` record emitted by `--diff-against`
- v0.1.37: the schemas served by `--print-schema NAME` describe the struct each verb really serializes; the keys the writer stamps on terminal events (`ok`, `code`, `exit`, `target_resolved`, `target_source`) are listed as optional properties

### Name to File Map
- `atomwrite --print-schema` prints the 55 names it serves, one per line; `atomwrite --print-schema NAME` prints one schema; an unknown name exits 65 `INVALID_INPUT`
- The flag must come before the command name. Both `--print-schema NAME` and `--print-schema=NAME` work
- 41 of the 55 names differ from the file name, so use this map: `NAME` is what the binary accepts and `FILE` is what this directory holds
- `read` and `stat` serve the same schema, because `stat` is an alias of `read --stat`
- Short names such as `read` and `edit` are the served names; the files keep descriptive names such as `read-output` and `edit-output`

| Name for `--print-schema` | File in this directory |
|---|---|
| `agent-surface` | `agent-surface.schema.json` |
| `apply` | `apply-result.schema.json` |
| `backup` | `backup-output.schema.json` |
| `batch` | `batch-output.schema.json` |
| `best-candidate` | `best-candidate.schema.json` |
| `calc` | `calc-output.schema.json` |
| `cancelled` | `cancelled-event.schema.json` |
| `case` | `case-output.schema.json` |
| `codemod` | `codemod.schema.json` |
| `commands` | `commands.schema.json` |
| `completions` | `completions.schema.json` |
| `config` | `config.schema.json` |
| `copy` | `copy-output.schema.json` |
| `count` | `count-summary.schema.json` |
| `count-by-ext` | `count-by-ext-output.schema.json` |
| `count-by-size` | `count-by-size-output.schema.json` |
| `del` | `del-result.schema.json` |
| `delete` | `delete-output.schema.json` |
| `diff` | `diff-output.schema.json` |
| `doctor` | `doctor.schema.json` |
| `edit` | `edit-output.schema.json` |
| `edit-loop` | `edit-loop-output.schema.json` |
| `error` | `error-output.schema.json` |
| `extract` | `extract-output.schema.json` |
| `extract-artifact` | `extract-artifact.schema.json` |
| `get` | `get-result.schema.json` |
| `hash` | `hash-output.schema.json` |
| `list` | `list-output.schema.json` |
| `locale` | `locale.schema.json` |
| `move` | `move-output.schema.json` |
| `outline` | `outline-output.schema.json` |
| `progress` | `progress-event.schema.json` |
| `prune-backups` | `prune-backups-output.schema.json` |
| `query` | `query-output.schema.json` |
| `read` | `read-output.schema.json` |
| `recipe` | `recipe-output.schema.json` |
| `regex` | `regex-output.schema.json` |
| `replace` | `replace-output.schema.json` |
| `rollback` | `rollback-result.schema.json` |
| `scope` | `scope-result.schema.json` |
| `scratch` | `scratch.schema.json` |
| `search` | `search-output.schema.json` |
| `semantic-merge` | `semantic-merge.schema.json` |
| `semantic-search` | `semantic-search.schema.json` |
| `set` | `set-result.schema.json` |
| `sparse` | `sparse.schema.json` |
| `stat` | `read-output.schema.json` |
| `stream_diff` | `stream-diff.schema.json` |
| `transform` | `transform-result.schema.json` |
| `verify` | `hash-verify.schema.json` |
| `wal-heal` | `wal-recovery.schema.json` |
| `wal-stats` | `wal-stats-output.schema.json` |
| `watch` | `watch.schema.json` |
| `write` | `write-output.schema.json` |
| `write-risk` | `write-risk-assessment.schema.json` |

### Refresh Procedure
- Print one schema: `atomwrite --print-schema write`
- Refresh one file from the binary: `atomwrite --print-schema NAME > docs/schemas/FILE.schema.json`, with `NAME` and `FILE` taken from a row of the map. Example: `atomwrite --print-schema write > docs/schemas/write-output.schema.json`
- Check one file without rewriting it: compare the checksum of the served schema with the checksum of the file, both normalised by `jaq -S -c .` so that key order does not matter
- Served schema: `atomwrite --print-schema write | jaq -S -c . | atomwrite hash --stdin`
- File: `jaq -S -c . docs/schemas/write-output.schema.json | atomwrite hash --stdin`
- The two `checksum` values are equal when the file is in sync. Measured for `write`: both give `c0c33dd9f0c110e31d55f1932c76d69423915ee1b107a93211c11aa848c62c94`
- Do this after any change to a struct that a verb serializes, for every row of the map whose struct changed
- `tests/docs_schemas_sync.rs` keeps the directory in sync: `every_schema_name_has_a_file_and_every_file_a_name` checks the map and the orphans (its `NAME_TO_FILE` is the map above), and `docs_schemas_equal_the_printed_ones` fails listing the files that differ from `--print-schema`. All 54 files are identical to the printed schema (the alias `stat` shares `read-output`), and no file is an orphan
- `write-risk-assessment.schema.json` is served by `--print-schema write-risk` and stays in sync with the nested object inside `write-output.schema.json`, because both come from the same `WriteRiskAssessment` struct

### Known Limits
- `--print-schema extract` serves the `ExtractOutput` union: an NDJSON object narrowed to the selected fields, an NDJSON object passed through whole, and the two text shapes. The object branches stay open because the input line decides the keys
- `completions` without `--install` prints a shell script and no NDJSON, so the `completions` schema describes only the `installed` event
- The Rust names of the served structs (`WriteOutput`, `CountTotalOutput`, `SearchSummary`) are the `title` of each schema

### Error envelope contract (`error-output.schema.json`)
- Required on every error line: `error`, `ok`, `code`, `exit`, `message`, `error_class`, `retryable`
- `ok` is always `false` on error envelopes (GAP-2026-020 agent sentinel) — do not treat a missing success field as “all null success”
- `match_count` is present on match-related failures: `0` on most failures (`MATCH_FAILED` / `NO_MATCHES`); non-zero when ambiguity reports a count
- Optional near-miss: `best_candidate` (v0.1.29+; also on `MATCH_AMBIGUOUS`)
- Multi-pair edit (G117): `failed_pair_index`, `pairs_total`, `pair_results`
- Target stamp (v0.1.37): `target_resolved` and `target_source` are present whenever the failing verb resolves a subject, and they name the same target the success path names

### Runtime shape flags (NOT schema fields)
- Global `--select` / `--fields`, `--filter`, and `--limit` reshape stdout emission at runtime (agent-native projection / filter / bound)
- They are not properties of any NDJSON schema; schemas describe the full object before projection
- Errors are always-emit: `--filter` must not drop `ok: false` envelopes; `--limit` counts success events only
- No global `-n` for limit (`-n` belongs to `replace`)

### v0.1.29 subcommand schemas (updated)
- Dedicated schemas added in 0.1.29 (all indexed): `agent-surface`, `sparse`, `semantic-merge`, `semantic-search`, `codemod`, `watch`. They were minimal envelopes then; today each file is what the binary serves for that name
- Nested / event schemas from 0.1.29: `best-candidate`, `progress-event`, `cancelled-event`, `recipe-output` (plus `write` / `replace` / `error` field extensions)
- `stat` reuses `read-output.schema.json` (alias of `read --stat`)

### v0.1.30 residual schema updates
- `edit-output.schema.json` regenerated with `match_count` (uint64|null) and `indent_adjusted` (bool|null)
- `write-output.schema.json` documents honest `platform.backup_method` values (`reflink_or_copy` | `copy`)
- Regenerate with `atomwrite --print-schema edit > docs/schemas/edit-output.schema.json` (see Refresh Procedure)
- Contract suite: `cargo test --test cli_v0130_agent_contract`

### v0.1.34 behavioral contract (no envelope change)
- JSON schemas intentionally unchanged in v0.1.33/v0.1.34 — one-shot fuzzy + timeout are algorithmic, not new NDJSON fields
- Fuzzy multi-apply is one-pass L→R on original content (`apply_fuzzy_one_pass`); never re-scans inserted text
- Default max fuzzy applies = 1 when `--max-replacements` omitted; hard ceiling 10_000; embeds force single apply
- Global `--timeout-secs` / `--timeout` default 120; `0` disables; deadline → process exit 124 via error-output envelope (generic `exit` field)
- Cooperative cancel (SIGINT/SIGTERM) → cancelled-event schema, process exit 143 — distinct from timeout 124
- Superseded in v0.1.37: the deadline also ends the data channel with the cancelled-event envelope, whose `deadline_exceeded: true` tells it apart from a signal (exit 124 against 130, 143 and 129)
- See ADR-0054 (`docs/decisions/0054-v0-1-34-oneshot-fuzzy-timeout.md`) and `cargo test --test cli_v0133_oneshot_fuzzy`

### v0.1.35 residual contract (no envelope schema change for A-*)
- A-* residuals closed without new NDJSON schema fields: large write `--ack-overwrite`, `delete --plan` (not `--confirm`/`--yes`), semantic-merge markers default ON
- `watch` always ends with a final NDJSON event `type:watch_summary` (even idle / zero events)
- `MATCH_FAILED` remains the error-output code for edit/fuzzy miss (exit 65) with optional `best_candidate`
- Contract suite: `cargo test --test cli_e2e_v0135`

### v0.1.36 residual CAPA (ADR-0055)
- `error-output.schema.json` requires `ok` (const false on errors) and documents `match_count`
- `scratch.schema.json` indexes `scratch init` staging under the workspace jail
- Historical (v0.1.36): agent surface SSOT was 46 tools (`agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`) — includes `scratch`, `doctor`, `locale`, `commands`; the v0.1.37 SSOT is 47 with `config`
- Schema coverage note: `verify` serves `hash-verify.schema.json`; `wal-heal` reports align with `wal-recovery.schema.json`; full tool list is 47 (`agent-surface`); `completions` has its own `completions.schema.json`
- Fuzzy honesty is behavioral (normalized strategies emit `similarity: null`; dual-gate `block_anchor`; XDG `[fuzzy].soft_floor_delta`) — no invented NDJSON fields beyond shipped schemas
- Global shape flags remain runtime-only (see above)
- Contract suite: `cargo test --test cli_v0136_gaps`; gap status notes: `gaps.md`

### v0.1.37 terminal events and summaries
- A failing summary is the only error line of the run: it carries `ok: false`, `code`, `exit` and the error detail itself (`error_class`, `retryable`, `suggestion`, `message`); no second `error-output` envelope follows. Exceptions: a broken pipe and a `hash_verify` verdict already published
- `MATCH_FAILED` (exit 65, optional `best_candidate`) is also the code for a pair with no match in `edit-loop` and in a multi-pair `edit`
- Every summary that publishes `files_failed` publishes `partial: true` beside it, and that covers `search`, `transform`, `replace`, `scope`, `count`, `list`, `sparse`, `semantic-search`, `case`, `backup` and `prune-backups`. Each `errors[]` entry has `path`, `code` and `message`, `path` is the marker `<walk>` when the walk failed without naming a file of its own, and the list stops at `summary.errors_max_entries` entries while `files_failed` keeps the full count
- `count` serves `CountTotalOutput` (`count-summary`); `count --by-extension` serves `count-by-ext`; `count --by-size` serves `count-by-size`
- Success events list `ok`, `code` and `exit` (the last two only when `ok` is false) and `target_resolved` and `target_source` as optional properties
- Every file is produced with the Refresh Procedure below: a file is the output of `atomwrite --print-schema NAME`, so `apply-result`, `backup-output`, `prune-backups-output`, `rollback-result` and `scope-result` carry the fields of the structs the binary serializes. Only `write-risk-assessment.schema.json` has no served name
- Contract suites: `cargo test --test cli_v0137_terminal_event --test cli_v0137_terminal_schema`


## Português
### Atualização
- Última atualização: 2026-10-31 (v0.1.37)
- 54 arquivos de schema neste diretório e 55 nomes servidos por `atomwrite --print-schema`: 54 arquivos são o schema de um nome servido (dois nomes, `read` e `stat`, dividem o `read-output.schema.json`), e `write-risk` serve o `write-risk-assessment.schema.json`, então o objeto aninhado de risco tem porta própria e nenhum arquivo fica órfão
- Cada arquivo deve ser idêntico ao que o binário serve; o procedimento de atualização e o mapa nome-arquivo estão abaixo
- Novo na v0.1.37: `config.schema.json`; `extract-artifact.schema.json`; `count-by-ext-output` e `count-by-size-output`; uma união por verbo, em que `recipe`, `codemod`, `batch`, `search`, `replace`, `list`, `backup`, `semantic-search`, `sparse`, `semantic-merge`, `case` e `watch` servem num só documento todo registro que escrevem mais o envelope de erro; eventos de sucesso listam os opcionais `ok`, `code`, `exit`, `target_resolved` e `target_source`, mais os contadores de corte `records_suppressed`, `files_budget_dropped`, `files_planned`, `truncated` e `truncated_records`, onde `records_suppressed` conta registros e `files_budget_dropped` conta entradas na unidade que o verbo produz
- Superfície SSOT: 47 tools via `agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`

### Objetivo
- Cada schema descreve a saída NDJSON de um subcomando do atomwrite
- Todos os schemas seguem JSON Schema draft/2020-12 (incluindo `error-output.schema.json`)
- Use estes schemas para validar saída consumida por agentes de forma programática

### Índice de Schemas
- `write-output.schema.json` -- saída do `atomwrite write` (v0.1.29: `platform.durability`, `platform.rename_method`, `platform.backup_method`; v0.1.30: `backup_method` é `reflink_or_copy` ou `copy`, nunca hardlink do arquivo vivo)
- `read-output.schema.json` -- saída do `atomwrite read`
- `edit-output.schema.json` -- saída do `atomwrite edit` (v0.1.15: `pairs_total`/`pair_results` -- G117) (v0.1.23: `source` em `pair_results`) (residual v0.1.30: `match_count`, `indent_adjusted`)
- `search-output.schema.json` -- saída do `atomwrite search` (`--print-schema search`; união de `begin`, `context`, `file`, `match`, `end`, o resumo terminal e o envelope de erro)
- `replace-output.schema.json` -- saída do `atomwrite replace` (`--print-schema replace`; união de `plan`, `replace`, o resumo terminal e o envelope de erro) (v0.1.29: `fuzzy`, `strategy`, `similarity`, `strategies_tried`)
- `delete-output.schema.json` -- saída do `atomwrite delete` (v0.1.28: adiciona `warnings`)
- `get-result.schema.json` -- saída do `atomwrite get` (v0.1.12, v14 Tier 3: leitura de chave única; `value` auto-parseado)
- `hash-output.schema.json` -- saída do `atomwrite hash` (o `verify` tem schema próprio, `hash-verify.schema.json`)
- `count-summary.schema.json` -- saída do `atomwrite count` (modo total; `--print-schema count`, struct `CountTotalOutput`)
- `count-by-ext-output.schema.json` -- saída do `atomwrite count --by-extension` (v0.1.37; `by_extension`; `--print-schema count-by-ext`)
- `diff-output.schema.json` -- saída do `atomwrite diff`
- `move-output.schema.json` -- saída do `atomwrite move`
- `copy-output.schema.json` -- saída do `atomwrite copy`
- `list-output.schema.json` -- saída do `atomwrite list` (`--print-schema list`; união de `entry`, o resumo terminal e o envelope de erro)
- `extract-output.schema.json` -- saída do `atomwrite extract` (`--print-schema extract` serve a união de quatro formatos de registro: um objeto NDJSON estreitado para os campos pedidos, um objeto NDJSON repassado inteiro, e as duas formas de texto `{type: "text", fields: [...]}` e `{type: "text", values: [...]}`; os dois ramos de objeto ficam abertos porque a linha de entrada decide as chaves)
- `calc-output.schema.json` -- saída do `atomwrite calc`
- `regex-output.schema.json` -- saída do `atomwrite regex`
- `transform-result.schema.json` -- saída do `atomwrite transform`
- `outline-output.schema.json` -- saída do `atomwrite outline` (v0.1.12, v14 Tier 3: extração de estrutura de alto nível; `items[].kind`, `items[].name`)
- `query-output.schema.json` -- saída do `atomwrite query` (v0.1.12, v14 Tier 3: query tree-sitter AST; oneOf `kinds`|`tree`|`matches`)
- `batch-output.schema.json` -- saída do `atomwrite batch` (`--print-schema batch`; união de `batch_op`, `progress`, `rollback`, o resumo terminal, o schema de manifesto do `--input-schema` e o envelope de erro)
- `scope-result.schema.json` -- saída do `atomwrite scope` (evento por arquivo)
- `backup-output.schema.json` -- saída do `atomwrite backup` (`--print-schema backup`; união de `backup` ou `plan`, o resumo terminal e o envelope de erro)
- `rollback-result.schema.json` -- saída do `atomwrite rollback`
- `set-result.schema.json` -- saída do `atomwrite set` (v0.1.12, v14 Tier 3: escrita de chave via dotted-path; `action: set`)
- `wal-recovery.schema.json` -- saída da API consultiva `recover_orphan_journals` e relatórios de recuperação do `atomwrite wal-heal` (v0.1.12, G114: `JournalEntry::{Started, Committed, Aborted}` reports)
- `apply-result.schema.json` -- saída do `atomwrite apply`
- `case-output.schema.json` -- saída do `atomwrite case` (`--print-schema case`; união de `case` ou `case_preview`, o resumo terminal e o envelope de erro) (v0.1.12, v14 Tier 3: conversão de case de identificadores; `files_modified`, `identifiers_renamed`)
- `del-result.schema.json` -- saída do `atomwrite del` (v0.1.12, v14 Tier 3: deleção de chave; `action: deleted|already_missing`)
- `error-output.schema.json` -- envelope de erro emitido por todos os subcomandos (v0.1.37: declara `target_resolved` e `target_source`, porque o writer carimba o sujeito resolvido também no caminho de falha; v0.1.15: adiciona `failed_pair_index`, `pairs_total`, `pair_results` -- G117; v0.1.4: adiciona `suggestion`; v0.1.29: adiciona `best_candidate`; v0.1.36: `ok` obrigatório `false` + `match_count`)
- `wal-stats-output.schema.json` -- saída do `atomwrite wal-stats` (v0.1.16: diagnósticos locais G119 L5; `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `by_directory`, `auto_heal_recommended`, `estimated_reclaim_bytes`)
- `count-by-size-output.schema.json` -- saída do `atomwrite count --by-size --top N` (v0.1.20: GAP-2026-001 top-N arquivos por tamanho decrescente; `items[].path`, `items[].bytes`)
- `write-risk-assessment.schema.json` -- objeto de avaliação de risco da saída do `atomwrite write`, servido com nome próprio `write-risk` (v0.1.20: GAP-2026-011 L1/L6; `original_bytes`, `new_bytes`, `size_delta_pct`, `risk_level`, `guard_triggered`)
- `edit-loop-output.schema.json` -- saída do `atomwrite edit-loop` (v0.1.22: N pares `{old, new}` em 1 invocação via NDJSON; `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`, `pair_results[].old`, `pair_results[].new`)
- `prune-backups-output.schema.json` -- saída do `atomwrite prune-backups` (v0.1.37: linha por backup, depois o resumo terminal; a linha carrega `path`, `reason` e `action`, e o resumo carrega `total`, `elapsed_ms`, `files_failed`, `partial` e `errors[]`)
- `best-candidate.schema.json` -- diagnóstico de near-miss em falha de match (v0.1.29 P0-2; também referenciado por `error-output`)
- `progress-event.schema.json` -- heartbeats de progresso de `replace`/`batch` (v0.1.29 P1-3)
- `cancelled-event.schema.json` -- registro terminal de varredura interrompida, emitido por todo verbo que varre (v0.1.29 P0-4 criou o evento; v0.1.37 acrescentou `deadline_exceeded`, que passa a distinguir prazo de sinal dentro deste schema, e não só pelo error-output)
- `recipe-output.schema.json` -- saída do `atomwrite recipe list|run` (`--print-schema recipe`; união da listagem, de todo registro de filho marcado com `step`, dos registros de etapa, do resultado terminal e do envelope de erro) (v0.1.29 P1-4)
- `config.schema.json` -- envelope de sucesso NDJSON de `atomwrite config path|list|get|set|unset` (chaves de ajuste persistentes; campos `type`, `command`, `action`, `scope`, `config_path`, `exists`, `key`, `value`, `origin`, `default`, `old_value`, `changed`, `keys`, `warnings`; `ok`, `code`, `exit`, `target_resolved` e `target_source` são as chaves opcionais que o writer carimba, as duas últimas em `path`, `set` e `unset`)

- `scratch.schema.json` -- envelope de sucesso NDJSON de `atomwrite scratch init` (dir de staging do agente sob o jail do workspace; campos `ok`, `status`, `command`, `action`, `path`, `relative`, `already_existed`)
- `doctor.schema.json` -- saída do `atomwrite doctor` (diagnóstico do host)
- `locale.schema.json` -- saída do `atomwrite locale` (relatório de locale resolvido / preferência)
- `commands.schema.json` -- saída do `atomwrite commands` (manifesto completo da árvore de comandos)
- `completions.schema.json` -- o evento `installed` do `atomwrite completions SHELL --install` (`type`, `path`; `--print-schema completions`). Sem `--install` o comando imprime o script de shell como texto puro e não emite NDJSON, então não há outro registro para descrever
- `agent-surface.schema.json` -- saída do `atomwrite agent-surface` (o manifesto: `tools`, `exit_codes`, `discovery`; `--print-schema agent-surface`)
- `sparse.schema.json` -- saída do `atomwrite sparse` (list/read/outline de monorepo com orçamento; `--print-schema sparse`)
- `semantic-merge.schema.json` -- saída do `atomwrite semantic-merge` (merge de três vias por linha; `--print-schema semantic-merge`)
- `semantic-search.schema.json` -- saída do `atomwrite semantic-search` (ranking offline por tokens; `--print-schema semantic-search`)
- `codemod.schema.json` -- saída do `atomwrite codemod` (campanha AST multi-regra; `--print-schema codemod`)
- `extract-artifact.schema.json` -- saída do `atomwrite extract-artifact` (v0.1.37; sinal delimitado de cabeçalhos de PDF, OOXML e imagem; `--print-schema extract-artifact`)
- `watch.schema.json` -- saída do `atomwrite watch` (eventos de filesystem + final `type:watch_summary`; feature `watch`)
- `hash-verify.schema.json` -- saída do `atomwrite verify` e de `hash --verify` (um evento `hash_verify` com o veredito inteiro; `--print-schema verify`)
- `stream-diff.schema.json` -- o registro único `stream_diff` emitido por `--diff-against`
- v0.1.37: os schemas servidos por `--print-schema NOME` descrevem o struct que cada verbo realmente serializa; as chaves que o writer carimba nos eventos terminais (`ok`, `code`, `exit`, `target_resolved`, `target_source`) aparecem como propriedades opcionais

### Mapa Nome-Arquivo
- `atomwrite --print-schema` imprime os 55 nomes que serve, um por linha; `atomwrite --print-schema NOME` imprime um schema; nome desconhecido sai 65 `INVALID_INPUT`
- A flag precisa vir antes do nome do subcomando. As duas escritas funcionam, `--print-schema NOME` e `--print-schema=NOME`
- A maioria dos nomes difere do nome do arquivo, então use este mapa: `NOME` é o que o binário aceita e `ARQUIVO` é o que este diretório guarda
- `read` e `stat` servem o mesmo schema, porque `stat` é alias de `read --stat`
- Nomes curtos como `read` e `edit` são os nomes servidos; os arquivos mantêm nomes descritivos como `read-output` e `edit-output`

| Nome para `--print-schema` | Arquivo neste diretório |
|---|---|
| `agent-surface` | `agent-surface.schema.json` |
| `apply` | `apply-result.schema.json` |
| `backup` | `backup-output.schema.json` |
| `batch` | `batch-output.schema.json` |
| `best-candidate` | `best-candidate.schema.json` |
| `calc` | `calc-output.schema.json` |
| `cancelled` | `cancelled-event.schema.json` |
| `case` | `case-output.schema.json` |
| `codemod` | `codemod.schema.json` |
| `commands` | `commands.schema.json` |
| `completions` | `completions.schema.json` |
| `config` | `config.schema.json` |
| `copy` | `copy-output.schema.json` |
| `count` | `count-summary.schema.json` |
| `count-by-ext` | `count-by-ext-output.schema.json` |
| `count-by-size` | `count-by-size-output.schema.json` |
| `del` | `del-result.schema.json` |
| `delete` | `delete-output.schema.json` |
| `diff` | `diff-output.schema.json` |
| `doctor` | `doctor.schema.json` |
| `edit` | `edit-output.schema.json` |
| `edit-loop` | `edit-loop-output.schema.json` |
| `error` | `error-output.schema.json` |
| `extract` | `extract-output.schema.json` |
| `extract-artifact` | `extract-artifact.schema.json` |
| `get` | `get-result.schema.json` |
| `hash` | `hash-output.schema.json` |
| `list` | `list-output.schema.json` |
| `locale` | `locale.schema.json` |
| `move` | `move-output.schema.json` |
| `outline` | `outline-output.schema.json` |
| `progress` | `progress-event.schema.json` |
| `prune-backups` | `prune-backups-output.schema.json` |
| `query` | `query-output.schema.json` |
| `read` | `read-output.schema.json` |
| `recipe` | `recipe-output.schema.json` |
| `regex` | `regex-output.schema.json` |
| `replace` | `replace-output.schema.json` |
| `rollback` | `rollback-result.schema.json` |
| `scope` | `scope-result.schema.json` |
| `scratch` | `scratch.schema.json` |
| `search` | `search-output.schema.json` |
| `semantic-merge` | `semantic-merge.schema.json` |
| `semantic-search` | `semantic-search.schema.json` |
| `set` | `set-result.schema.json` |
| `sparse` | `sparse.schema.json` |
| `stat` | `read-output.schema.json` |
| `stream_diff` | `stream-diff.schema.json` |
| `transform` | `transform-result.schema.json` |
| `verify` | `hash-verify.schema.json` |
| `wal-heal` | `wal-recovery.schema.json` |
| `wal-stats` | `wal-stats-output.schema.json` |
| `watch` | `watch.schema.json` |
| `write` | `write-output.schema.json` |
| `write-risk` | `write-risk-assessment.schema.json` |

### Procedimento de Atualização
- Imprimir um schema: `atomwrite --print-schema write`
- Atualizar um arquivo a partir do binário: `atomwrite --print-schema NOME > docs/schemas/ARQUIVO.schema.json`, com `NOME` e `ARQUIVO` tirados de uma linha do mapa. Exemplo: `atomwrite --print-schema write > docs/schemas/write-output.schema.json`
- Conferir um arquivo sem reescrevê-lo: compare o checksum do schema servido com o checksum do arquivo, os dois normalizados por `jaq -S -c .` para a ordem das chaves não importar
- Schema servido: `atomwrite --print-schema write | jaq -S -c . | atomwrite hash --stdin`
- Arquivo: `jaq -S -c . docs/schemas/write-output.schema.json | atomwrite hash --stdin`
- Os dois valores de `checksum` são iguais quando o arquivo está em sincronia. Medido para `write`: os dois dão `c0c33dd9f0c110e31d55f1932c76d69423915ee1b107a93211c11aa848c62c94`
- Faça isso depois de qualquer mudança num struct que um verbo serializa, para cada linha do mapa cujo struct mudou
- `tests/docs_schemas_sync.rs` mantém o diretório em sincronia: `every_schema_name_has_a_file_and_every_file_a_name` confere o mapa e os órfãos (o `NAME_TO_FILE` dele é o mapa acima), e `docs_schemas_equal_the_printed_ones` falha listando os arquivos que diferem do `--print-schema`. Os 54 arquivos são idênticos ao schema impresso (o alias `stat` divide o `read-output`), e nenhum arquivo é órfão
- O `write-risk-assessment.schema.json` é servido por `--print-schema write-risk` e fica em sincronia com o objeto aninhado dentro do `write-output.schema.json`, porque os dois saem da mesma estrutura `WriteRiskAssessment`

### Limites Conhecidos
- `--print-schema extract` serve a união `ExtractOutput`: um objeto NDJSON estreitado para os campos pedidos, um objeto NDJSON repassado inteiro, e as duas formas de texto. Os ramos de objeto ficam abertos porque a linha de entrada decide as chaves
- `completions` sem `--install` imprime um script de shell e nenhum NDJSON, então o schema `completions` descreve só o evento `installed`
- Os nomes Rust dos structs servidos (`WriteOutput`, `CountTotalOutput`, `SearchSummary`) são o `title` de cada schema

### Contrato do envelope de erro (`error-output.schema.json`)
- Obrigatórios em toda linha de erro: `error`, `ok`, `code`, `exit`, `message`, `error_class`, `retryable`
- `ok` é sempre `false` em envelopes de erro (sentinela de agente GAP-2026-020) — não trate campo de sucesso ausente como “sucesso todo null”
- `match_count` está presente em falhas de match: `0` na maioria das falhas (`MATCH_FAILED` / `NO_MATCHES`); não-zero quando ambiguidade reporta contagem
- Near-miss opcional: `best_candidate` (v0.1.29+; também em `MATCH_AMBIGUOUS`)
- Edit multi-par (G117): `failed_pair_index`, `pairs_total`, `pair_results`
- Carimbo de alvo (v0.1.37): `target_resolved` e `target_source` aparecem sempre que o verbo que falha resolve um sujeito, e nomeiam o mesmo alvo que o caminho de sucesso nomeia

### Flags de shape em runtime (NÃO são campos de schema)
- Globais `--select` / `--fields`, `--filter` e `--limit` remodelam a emissão de stdout em runtime (projeção / filtro / teto agent-native)
- Não são propriedades de nenhum schema NDJSON; schemas descrevem o objeto completo antes da projeção
- Erros são always-emit: `--filter` não deve silenciar envelopes `ok: false`; `--limit` conta só eventos de sucesso
- Sem `-n` global para limit (`-n` pertence ao `replace`)

### Schemas de subcomandos 0.1.29 (atualizado)
- Schemas dedicados adicionados na 0.1.29 (todos indexados): `agent-surface`, `sparse`, `semantic-merge`, `semantic-search`, `codemod`, `watch`. Eram envelopes mínimos naquela época; hoje cada arquivo é o que o binário serve para o nome
- Schemas aninhados / de evento da 0.1.29: `best-candidate`, `progress-event`, `cancelled-event`, `recipe-output` (mais extensões de campos em `write` / `replace` / `error`)
- `stat` reutiliza `read-output.schema.json` (alias de `read --stat`)

### Atualizações de schema residual v0.1.30
- `edit-output.schema.json` regenerado com `match_count` (uint64|null) e `indent_adjusted` (bool|null)
- `write-output.schema.json` documenta valores honestos de `platform.backup_method` (`reflink_or_copy` | `copy`)
- Regenere com `atomwrite --print-schema edit > docs/schemas/edit-output.schema.json` (veja Procedimento de Atualização)
- Suíte de contrato: `cargo test --test cli_v0130_agent_contract`

### Contrato comportamental v0.1.34 (sem mudança de envelope)
- Schemas JSON intencionalmente inalterados em v0.1.33/v0.1.34 — fuzzy one-shot + timeout são algorítmicos, não novos campos NDJSON
- Multi-apply fuzzy é one-pass E→D no conteúdo original (`apply_fuzzy_one_pass`); nunca reescaneia texto inserido
- Máximo de applies fuzzy padrão = 1 quando `--max-replacements` omitido; teto rígido 10_000; embeds forçam apply único
- Global `--timeout-secs` / `--timeout` padrão 120; `0` desabilita; prazo → process exit 124 via envelope error-output (campo genérico `exit`)
- Cancel cooperativo (SIGINT/SIGTERM) → schema cancelled-event, process exit 143 — distinto do timeout 124
- Substituído na v0.1.37: o prazo também encerra o canal de dado com o envelope cancelled-event, cujo `deadline_exceeded: true` o distingue de um sinal (exit 124 contra 130, 143 e 129)
- Ver ADR-0054 (`docs/decisions/0054-v0-1-34-oneshot-fuzzy-timeout.md`) e `cargo test --test cli_v0133_oneshot_fuzzy`

### Contrato residual v0.1.35 (sem mudança de schema de envelope para A-*)
- Residuais A-* fechados sem novos campos de schema NDJSON: write grande `--ack-overwrite`, `delete --plan` (não `--confirm`/`--yes`), markers de semantic-merge default ON
- `watch` sempre termina com evento NDJSON final `type:watch_summary` (mesmo idle / zero eventos)
- `MATCH_FAILED` permanece o código de error-output para miss de edit/fuzzy (exit 65) com `best_candidate` opcional
- Suíte de contrato: `cargo test --test cli_e2e_v0135`

### CAPA residual v0.1.36 (ADR-0055)
- `error-output.schema.json` exige `ok` (sempre `false` em erros) e documenta `match_count`
- `scratch.schema.json` indexa `scratch init` de staging sob o jail do workspace
- Histórico (v0.1.36): a superfície de agente SSOT era de 46 tools (`agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`) — inclui `scratch`, `doctor`, `locale`, `commands`; o SSOT da v0.1.37 é 47 com `config`
- Nota de cobertura de schemas: `verify` serve `hash-verify.schema.json`; relatórios de `wal-heal` alinham com `wal-recovery.schema.json`; lista completa de tools é 47 (`agent-surface`); `completions` tem `completions.schema.json` próprio
- Honestidade fuzzy é comportamental (estratégias normalizadas emitem `similarity: null`; dual-gate `block_anchor`; XDG `[fuzzy].soft_floor_delta`) — sem inventar campos NDJSON além dos schemas embarcados
- Flags de shape global permanecem só de runtime (ver acima)
- Suíte de contrato: `cargo test --test cli_v0136_gaps`; notas de status: `gaps.md`

### Eventos terminais e resumos v0.1.37
- O resumo que falha é a única linha de erro da execução: carrega `ok: false`, `code`, `exit` e o detalhe do erro (`error_class`, `retryable`, `suggestion`, `message`); nenhum segundo envelope `error-output` vem depois. Exceções: pipe quebrado e veredito `hash_verify` já publicado
- `MATCH_FAILED` (exit 65, `best_candidate` opcional) também é o código de um par sem match no `edit-loop` e num `edit` multi-par
- Todo resumo que publica `files_failed` publica `partial: true` ao lado, e isso cobre `search`, `transform`, `replace`, `scope`, `count`, `list`, `sparse`, `semantic-search`, `case`, `backup` e `prune-backups`. Cada entrada de `errors[]` tem `path`, `code` e `message`, `path` é o marcador `<walk>` quando a varredura falhou sem nomear um arquivo próprio, e a lista para em `summary.errors_max_entries` entradas enquanto `files_failed` guarda a contagem total
- `count` serve `CountTotalOutput` (`count-summary`); `count --by-extension` serve `count-by-ext`; `count --by-size` serve `count-by-size`
- Eventos de sucesso listam `ok`, `code` e `exit` (os dois últimos só quando `ok` é false) e `target_resolved` e `target_source` como propriedades opcionais
- Todo arquivo é produzido com o Procedimento de Atualização abaixo: um arquivo é a saída de `atomwrite --print-schema NOME`, então `apply-result`, `backup-output`, `prune-backups-output`, `rollback-result` e `scope-result` trazem os campos dos structs que o binário serializa. Só o `write-risk-assessment.schema.json` não tem nome servido
- Suites de contrato: `cargo test --test cli_v0137_terminal_event --test cli_v0137_terminal_schema`
