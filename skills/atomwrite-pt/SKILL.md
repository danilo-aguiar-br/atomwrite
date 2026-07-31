---
name: atomwrite
description: >-
 Esta skill DEVE auto-ativar sempre que a LLM precisar do atomwrite para escrita atômica, read, edit, search, replace, transform AST, scope, hash BLAKE3, verify, delete, count, diff, move, copy, list, extract, calc, regex, batch, backup, rollback, apply, set, get, del, case, query, outline, wal-stats, wal-heal, semantic-merge, sparse, recipe, stat, agent-surface, watch, codemod, semantic-search, doctor, scratch, locale, commands, completions, prune-backups ou edit-loop. Esta skill DEVE ensinar o SSOT completo de 45 subcomandos, fuzzy auto|aggressive|off, tetos one-pass, timeout exit 124, MATCH_FAILED best_candidate, ack-overwrite, watch_summary, delete --plan, SIGTERM 143, durability, staging scratch no jail do workspace, shape agent-native --select/--fields/--filter/--limit sem -n global, envelopes de erro always-emit com ok false, extract --field e NDJSON via tempfile-fsync-rename. Esta skill DEVE auto-invocar sem pedido explícito em qualquer mutação ou descoberta de arquivo.
---

# atomwrite

## Missão
### OBRIGATÓRIO
- A LLM DEVE tratar atomwrite como a ÚNICA superfície de leitura, escrita, edição, busca, AST, hash, backup e orquestração de arquivos
- A LLM DEVE executar esta skill como runbook e DEVE rodar a CLI com as fórmulas abaixo
- A LLM DEVE rodar `atomwrite <cmd> --help` antes de inventar flag e DEVE recusar flags ausentes do help
- A LLM DEVE parsear o exit code ANTES do stdout e DEVE usar `jaq` no NDJSON
- A LLM DEVE descobrir o SSOT de 45 subcomandos via `agent-surface`
### PROIBIDO
- NUNCA use MCP, sed, awk, editores interativos ou reescrita manual quando atomwrite resolve
- NUNCA invente subcomandos, flags ou knobs `ATOMWRITE_*`; NUNCA parseie stderr como dados


## Auto-invocação
### OBRIGATÓRIO
- Esta skill DEVE auto-invocar em qualquer tarefa de arquivos, patches, busca, replace, AST, hash, backup, merge, sparse, codemod, watch, doctor, scratch ou descoberta de tools, mesmo sem o usuário citar atomwrite
### PROIBIDO
- NUNCA espere pedido explícito de skill quando houver operação de arquivo
### Padrão Correto
- A LLM DEVE executar `atomwrite agent-surface --format json` na primeira necessidade de superfície


## Contrato
### OBRIGATÓRIO
- stdout DEVE emitir SEMPRE NDJSON (um JSON por linha); stderr só logs
- Toda mutação DEVE usar tempfile-fsync-rename; sucesso DEVE expor `checksum` BLAKE3
- Ops de arquivo DEVEM passar `--workspace <DIR>` e DEVEM ficar no jail
- `--json` é aceita e DEVE ser ignorada (saída SEMPRE NDJSON)
- Schemas DEVEM seguir `docs/schemas/` quando validação formal for necessária
- Precedência DEVE ser flags CLI depois `.atomwrite.toml` ou XDG depois defaults embutidos
### PROIBIDO
- NUNCA escreva fora do jail; NUNCA dependa de knobs de env de produto em runtime
- NUNCA trate exit 1 em `search`/`replace`/`transform`/`scope`/`semantic-search` como crash quando for `NO_MATCHES`
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace . --timeout-secs 120 --locale pt-BR --no-progress <cmd>`


## Contratos de segurança do agente
### OBRIGATÓRIO
- Sobrescrita grande DEVE passar `--ack-overwrite` (XDG `[write].confirm_large_bytes`, padrão 100 KiB); falta = exit 65
- Delete plan-only DEVE usar `delete --plan`; execução DEVE omitir `--plan` e flags de confirm
- `watch` DEVE terminar com NDJSON `watch_summary` e DEVE ter teto `--max-events` ou timeout (idle ~500 ms)
- `semantic-merge` DEVE gravar markers line-based por padrão; prefer-ours sem markers DEVE usar `--no-conflict-markers`
- Exit 65 DEVE ler `best_candidate` e `match_count`; 82 DEVE recarregar checksum; 124 DEVE aumentar timeout ou estreitar path; 143 = SIGTERM limpo; 126 = jail; 81 = checksum; 88 = sintaxe
### PROIBIDO
- NUNCA passe `delete --confirm`/`--yes`/`-y`; NUNCA sobrescreva grande sem `--ack-overwrite`
- NUNCA use confirm interativo; NUNCA assuma stdout vazio de `watch`; NUNCA desligue markers por acidente; NUNCA embuta MCP
### Padrão Correto
- A LLM DEVE executar `printf 'x' | atomwrite --workspace . write --ack-overwrite big.txt` e `atomwrite --workspace . delete --plan tmp/`


## Flags globais
### OBRIGATÓRIO
- DEVE usar `--workspace <DIR>`, `--config <PATH>`, `--timeout-secs <N>` ou `--timeout <N>` (padrão 120 → exit 124; `0` só se intencional)
- DEVE usar `-j N` / `--threads N` / `--max-concurrency N` (omitido ou zero = todos cores com teto de RAM)
- DEVE usar `--max-filesize <BYTES>` (padrão global 1 GiB), `--locale <TAG>` (NUNCA global `--lang`)
- DEVE usar `--json-schema`, `--no-auto-heal`, `--no-progress`, `--no-gitignore`, `--hidden`, `--follow-symlinks`
- DEVE usar `--color auto|always|never` ou `--no-color`; `-v`/`-vv`/`-vvv` ou `-q`/`-qq`
- Shape agent-native OBRIGATÓRIO — `--select`/`--fields KEYS`, `--filter key=value` (AND; `!=`/`==`), `--limit N`
- Envelopes always-emit — `--filter` NÃO silencia `ok` false; DEVE preferir shape a pipes jq/sed
- DEVE descobrir superfície via `atomwrite agent-surface` (SSOT 45)
### PROIBIDO
- NUNCA invente flags ausentes de `atomwrite --help`
- NUNCA use `-n` global para limit (`-n` é do `replace`; use `--limit`)
- NUNCA despeje NDJSON completo quando `--select` basta
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace . --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 <cmd>`


## Superfície completa de comandos
### OBRIGATÓRIO
A LLM DEVE invocar cada um dos 45 subcomandos. Prefixo típico `atomwrite --workspace .` omitido quando óbvio; a LLM DEVE incluir `--workspace` em ops de arquivo.

1. `read` — ler com checksum; `read --head 40 path`
2. `write` — criar/sobrescrever via stdin; `echo body | write --durability auto path`
3. `edit` — edição cirúrgica; `edit path --old a --new b --fuzzy auto`
4. `search` — localizar padrões; `search TODO src/ --include '*.rs'`
5. `replace` — replace multiarquivo; `replace --dry-run --fuzzy auto OLD NEW src/`
6. `hash` — BLAKE3; `hash path | jaq -r .checksum`
7. `delete` — remover com plan; `delete --plan path`
8. `count` — contagens; `count --by-size --top 10 src/`
9. `diff` — diff unificado; `diff a.rs b.rs --unified`
10. `move` — mover/renomear; `move src/old.rs src/new.rs`
11. `copy` — copiar; `copy --recursive --preserve src/ dest/`
12. `list` — listar árvore; `list --long --depth 2 src/`
13. `extract` — projetar campos NDJSON; `extract --field path` ou `extract path`
14. `calc` — expressões/unidades; `atomwrite calc "2 hours to seconds"`
15. `regex` — gerar regex; `atomwrite regex abc-12 xyz-99 id-7 --digits`
16. `transform` — rewrite AST; `transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
17. `scope` — ações gramaticais; `scope src/ --lang rust --query comments --delete --dry-run`
18. `batch` — manifesto NDJSON; `echo '{"op":"write","target":"a.txt","content":"x"}' | batch --transaction`
19. `backup` — backup com retention; `backup path --retention 3`
20. `rollback` — restaurar; `rollback path --latest --verify`
21. `apply` — patch unified/SR; `echo body | apply path --format full`
22. `set` — gravar dotted TOML/JSON; `set Cargo.toml package.name x`
23. `get` — ler dotted; `get config.toml database.pool.max`
24. `del` — remover chave; `del --force-missing config.toml features.x`
25. `case` — case/subvert; `case --to kebab --subvert API API --dry-run src/`
26. `query` — nós AST; `query --kinds src/main.rs`
27. `outline` — outline; `outline --kind function_item --positions src/main.rs`
28. `wal-stats` — journals WAL; `wal-stats`
29. `wal-heal` — limpar órfãos; `wal-heal --threshold-secs 3600`
30. `semantic-merge` — merge line-based; `semantic-merge --base b --ours o --theirs t --output out --fail-on-conflict`
31. `sparse` — monorepo sob orçamento; `sparse list src/ --max-files 50 --max-bytes 524288`
32. `recipe` — receitas; `recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run`
33. `stat` — alias `read --stat`; `stat path`
34. `agent-surface` — SSOT 45 tools; `atomwrite agent-surface --format json`
35. `watch` — FS com teto; `watch src/ --max-events 100 --idle-exit-ms 500`
36. `codemod` — rewrites em massa; `codemod --rules rules.yaml --dry-run src/`
37. `semantic-search` — Jaccard offline; `semantic-search query src/ --k 20 --min-score 0.05`
38. `doctor` — host; `atomwrite doctor --strict`
39. `scratch` — staging no jail; `scratch init`
40. `locale` — locale; `atomwrite locale --set pt-BR`
41. `commands` — árvore JSON; `atomwrite commands --include-globals`
42. `completions` — shell; `atomwrite completions bash --install`
43. `prune-backups` — podar; `prune-backups --max-age-secs 86400 --dry-run .`
44. `edit-loop` — N pares numa escrita; `echo '[{"old":"a","new":"b"}]' | edit-loop path`
45. `verify` — hash esperado; `verify path EXPECTED_HASH`
### PROIBIDO
- NUNCA invente um 46º subcomando; NUNCA omita `agent-surface` quando a superfície for desconhecida
- NUNCA omita `--workspace` em ops de arquivo dentro do jail


## Fuzzy
### OBRIGATÓRIO
- Fuzzy em `edit`/`replace` padrão `auto`; valores SOMENTE `--fuzzy auto|aggressive|off`
- `--fuzzy off` = exact-only; miss edit = exit 65 `MATCH_FAILED`; miss replace árvore = exit 1 `NO_MATCHES` frequente
- DEVE usar `--fuzzy-threshold` 0.0–1.0; com `--regex` no replace o fuzzy DEVE ser ignorado
- Multi-apply DEVE ser one-pass L→R no original; NEW embute OLD → força 1 ocorrência; sem `--max-replacements` padrão multi = 1; teto duro 10000
- Caps — pattern 64 KiB, Levenshtein 8192, janelas 4096, crescimento max 4x ou +16 MiB
- Estratégias normalizadas (`line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `trimmed_boundary`, `escape_normalized`) emitem `similarity` null — NUNCA trate 1.0 falso como igualdade byte a byte se não for `exact`
- Soft-floor `context_aware` — XDG `[fuzzy].soft_floor_delta` padrão 0.05; aceite efetivo = threshold OU threshold menos soft_floor_delta
- `block_anchor` dual-gate (bordas + ratio do miolo)
- `MATCH_AMBIGUOUS` DEVE incluir `best_candidate`; DEVE expandir `--old` ou `--replace-all` (edit) ou estreitar escopo — NUNCA chute
- Fuzzy NÃO é AST — DEVE usar `transform`/`scope`/`codemod` para semântica
### PROIBIDO
- NUNCA espere rebusca do texto inserido no mesmo replace fuzzy; NUNCA use `--replace-all` em `replace`
- NUNCA defina `--timeout-secs 0` em monorepo fuzzy salvo job sem prazo intencional
- NUNCA rode `--fuzzy aggressive` em árvore sem `--dry-run`; NUNCA passe flag inventada `--strategy` no edit
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace . replace --dry-run --fuzzy auto --max-replacements 1 OLD NEW src/`


## Hierarquia de edição + fluxo de execução
### OBRIGATÓRIO
- Ordem do mais preciso ao mais bruto — `search` → `transform` → `scope` → `codemod` → `replace` → `edit` → `write`
- `write` SOMENTE para criar ou sobrescrever arquivo inteiro de forma intencional
- DEVE localizar com `search` ou `sparse` antes de mutar; DEVE capturar checksum com `read` sob concorrência
- DEVE rodar `--dry-run` antes de mutação destrutiva em massa; DEVE aplicar sobrescrita concorrente com `--expect-checksum`
- DEVE validar `checksum`, `match_count`, `indent_adjusted`, `platform` quando presentes
- DEVE tratar falhas por `error_class` e `suggestion`; retentar SOMENTE se `retryable` true
### PROIBIDO
- NUNCA reescreva inteiro com `write` quando `edit`/`replace`/`transform` resolve
- NUNCA ignore exit não-zero exceto zero-match documentado
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace . search alvo src/` e só então mutar


## Backup e configuração
### OBRIGATÓRIO
- Backup default true nos mutadores via `--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`
- Backup padrão DEVE criar `.bak.<timestamp>` adjacente e auto-remover no sucesso; em falha DEVE permanecer
- `delete` e `replace` DEVEM preservar `.bak` no sucesso
- `rollback` DEVE usar `--backup` explícito para backup opt-in do alvo restaurado
- `move` e `copy` DEVEM passar `--force` ou `--backup` para sobrescrever destino
- Backup DEVE usar `reflink_or_copy` e NUNCA hardlink do arquivo vivo
- `.atomwrite.toml` governa `[defaults]` e `[fuzzy]` (`auto|aggressive|off`); flags CLI vencem TOML
### PROIBIDO
- NUNCA passe `--backup` e `--no-backup` juntos (exit 2)
- NUNCA assuma que TOML `backup = true` sozinho autoriza sobrescrita em move/copy
### Padrão Correto
- A LLM DEVE definir `[fuzzy] mode = "auto"` e `threshold = 0.85` em `.atomwrite.toml`


## Mutação write/edit/replace
### OBRIGATÓRIO
- `write` DEVE enviar conteúdo via stdin; DEVE usar `--expect-checksum`, `--durability full|fast|auto` (padrão `auto`)
- Rewrite que encolhe >50% DEVE usar `--allow-shrink`
- DEVE usar `--allow-empty-stdin`, `--no-checksum-when-empty`, `--syntax-check` (exit 88), `--ack-overwrite`
- Flags write — `--append`, `--prepend`, `--max-size`, `--line-ending`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--require-large-ack`, `--risk-threshold`, `--wal-policy`, `--dry-run`, opts de backup
- SIGTERM no write DEVE limpar tempfile e retornar exit 143
- `edit` DEVE usar `--old`/`--new` (pares repetíveis); multi-par emite `pairs_total`/`pair_results`; falha aborta lote salvo `--partial`
- Multi-ocorrência em edit DEVE usar `--replace-all` e DEVE emitir `match_count`; indent realinhado emite `indent_adjusted` true
- Flags posição — `--after-line`, `--before-line`, `--range`, `--delete-range`, `--after-match`, `--before-match`, `--between`, `--multi`
- Patches com `//`, jq, aspas, multilinha DEVE usar `--old-file`/`--new-file` estagiados em `$WS/.atomwrite/scratch/` após `scratch init`
- DEVE usar `--expect-checksum` e `--allow-sequential-drift` só em pipeline controlado
- Stdin de terminal em inserção DEVE falhar exit 65 sem travar
- Multi-root — se alvo está em skills no home, DEVE passar `--workspace` na raiz permitida
- `replace` exit 1 = zero matches (NÃO crash); DEVE SEMPRE `--dry-run`/`--preview` antes de apply em massa
- Multi fuzzy em replace DEVE usar só `--max-replacements` (`-n`)
- Flags replace — `--regex`, `-w`, `-F`, `-g/--include`, `--exclude`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case`, opts de backup
### PROIBIDO
- NUNCA pipe edit para `jaq` sem `${PIPESTATUS[0]}`; NUNCA passe `--strategy` no edit
- NUNCA coloque old/new-file sob `/tmp` do SO fora do jail
- NUNCA passe `$VAR` sem aspas quando o dólar literal deve sobreviver — DEVE quote `"$VAR"`
- NUNCA trate fuzzy como AST; NUNCA quote no shell patches frágeis como `--old`/`--new` quando scratch existe
- NUNCA use `--replace-all` em replace; NUNCA pule dry-run em árvore grande; NUNCA encolha >50% sem `--allow-shrink`
### Padrão Correto
- A LLM DEVE executar staging — `atomwrite --workspace "$WS" scratch init; SCR="$WS/.atomwrite/scratch"; printf %s "$OLD" >"$SCR/old.txt"; printf %s "$NEW" >"$SCR/new.txt"; atomwrite --workspace "$WS" edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" path`
- A LLM DEVE executar `atomwrite --workspace "$WS" write --durability auto --no-backup --allow-shrink "$path" < "$novo"`


## Path e estruturado
### OBRIGATÓRIO
- `delete` DEVE manter backup e preservar `.bak` no sucesso; idade via `--older-than`; recursivo `-r`; globs `-g/--include`; plan via `--plan`
- `move`/`copy` DEVEM `--force` ou `--backup` para sobrescrever destino
- `copy` DEVE usar `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr` quando exigido
- `apply` detecta unified, SEARCH/REPLACE, markdown, full; DEVE usar `--format auto|unified|search-replace|full|markdown` se ambíguo
- `set`/`get`/`del` SOMENTE TOML ou JSON via dotted path; get ausente = exit 65; `del` aceita `--force-missing`
- `case` DEVE `--subvert OLD NEW` e DEVE dry-run em codebase grande
### PROIBIDO
- NUNCA passe `--confirm`/`--yes`/`-y` em delete; NUNCA use set/get/del em texto puro
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace . move src/old.rs src/new.rs` e `atomwrite --workspace . set Cargo.toml package.name x`


## Leitura e busca
### OBRIGATÓRIO
- `read` — conteúdo + checksum/size/lines/mode; `stat PATH` = alias de `read --stat`
- Flags read — `--lines`, `--line`, `-C/--context`, `--head`, `--tail`, `--format raw|ndjson`, `--grep`, `--verify-checksum` (mismatch exit 81)
- `search` ANTES de mutar se alvo desconhecido; flags `-g/--include`, `--exclude`, `-C`, `-F`, `-e/--regex`, `-w`, `-i`, `-S`, `-c`, `-l`, `-m`, `-U`, `-P`, `--invert`, `--sort`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Default local `--max-filesize` no search = 10 MiB
- `list` — `--include`, `--exclude`, `--long`, `--depth`, `--count-by-ext`, `--all`
- `count` — `--by-extension`, `--by-size`, `--top`
- `extract` — posicionais OU `--field` local (repetível; NÃO combinar ambos) + `--delimiter` + `--stdin`
- `diff` — `--unified`, `--stat`, `-C`, `--algorithm myers|patience|lcs`; path `-` = stdin
### PROIBIDO
- NUNCA passe `extract --select` — `--select` é shape global; flag local é `--field`
- NUNCA trate zero matches de search como crash
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace . read --head 20 path` e `printf '{"a":1}\n' | atomwrite extract --field a`


## AST
### OBRIGATÓRIO
- Exit 1 em `transform` = zero matches; DEVE definir `-l/--language`; `$NAME` um nó, `$$$ARGS` múltiplos
- Single-rule DEVE passar `-p/--pattern` e `-r/--rewrite` juntos
- Flags transform — `--rules`, `--inline-rules`, `--verify-parse`, `--include`, `--exclude`, `--dry-run`, opts de backup
- DEVE preferir transform sobre replace quando o padrão for sintático
- `scope` DEVE `--language` (alias `--lang`); `--query` ou `--pattern`; `--delete`/`--action`/`--replace-with`
- Queries scope — comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- `query` — `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- `outline` — `--kind` repetível, `--positions`, `--language`
### PROIBIDO
- NUNCA aplique `scope --delete` sem dry-run; NUNCA use fuzzy textual quando transform/scope resolve
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace . transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## Orquestração
### OBRIGATÓRIO
- `batch` — NDJSON stdin com `op` em write/replace/delete/edit/move/copy/hash; move/copy com `"force":true` para sobrescrever
- DEVE usar `--transaction`, `--file <PATH>`, `--dry-run`, `--batch-size`, `--input-schema`, opts de backup
- `sparse list` — `--max-files`, `--max-bytes`; `sparse read --paths-file` — `--head`, `--max-files`; `sparse outline` sob orçamento
- `recipe list` e `recipe run --name search-replace-verify`; receitas embutidas `search-replace-verify` e `edit-loop-syntax-check`
- Recipe search/replace/hash DEVEM excluir `*.bak.*` por padrão
- `recipe run` aceita `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- `semantic-merge --base A --ours B --theirs C --output OUT` é line-based com markers ON (NUNCA AST/embedding)
- Flags merge — `--fail-on-conflict`, `--write-conflict-markers`, `--no-conflict-markers`, `--expect-checksum`, opts de backup
- `agent-surface --format json` = manifesto SSOT
- `watch` — `--debounce-ms`, `--max-events`, `--idle-exit-ms`, `--checksum`, `--gitignore`; feature binária `watch`; termina com `watch_summary`
- `codemod --rules rules.yaml --dry-run` ANTES de apply
- `semantic-search "query" PATH --k 20 --min-score 0.05` é Jaccard offline; `--index-dir` só se índice local for necessário
### PROIBIDO
- NUNCA rode `watch` sem teto max-events ou timeout
- NUNCA trate zero hits de semantic-search como crash; NUNCA trate semantic-merge como AST
### Padrão Correto
- A LLM DEVE executar `echo '{"op":"write","target":"a.txt","content":"x"}' | atomwrite --workspace . batch --transaction`


## Host
### OBRIGATÓRIO
- DEVE usar `doctor` e `doctor --strict` quando falha DEVE abortar pipeline
- DEVE usar `locale`, `locale --set pt-BR|en`, `locale --clear`
- DEVE usar `commands` e `commands --include-globals`
### PROIBIDO
- NUNCA ignore falhas de `doctor --strict` em CI ou agente
### Padrão Correto
- A LLM DEVE executar `atomwrite doctor --strict && atomwrite commands --include-globals`


## Utilitários
### OBRIGATÓRIO
- `hash` — BLAKE3; ler campo `checksum` e NUNCA `value`; flags `--verify`, `--stdin`, `-r/--recursive`, `--exclude`
- `verify <PATH> <EXPECTED_HASH>` — exit 0 se casa, 81 se não
- `backup` — `--retention`, `--output-dir`; `rollback --latest --verify`; flags rollback `--timestamp`, `--latest`, `--verify`, `--dry-run`, `--backup` opt-in
- `prune-backups` — `--max-age-secs` e NUNCA `--max-age`
- `calc` sem workspace; `regex` a partir de ≥3 exemplos (`-d`, `-w`, `-s`, `-r`, `-i`, `--no-anchors`, `--stdin`)
- `wal-stats` só inspeciona; `wal-heal` remove órfãos (`--threshold-secs`, `--max-duration-ms`)
- `edit-loop` aplica N pares numa escrita; `completions` bash/zsh/fish/elvish/powershell com `--install`
- `scratch init` cria staging em `$WS/.atomwrite/scratch/`
### PROIBIDO
- NUNCA use `/tmp` do SO fora do jail para staging; NUNCA leia `value` no lugar de `checksum`
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace . hash path | jaq -r .checksum` e `atomwrite --workspace . scratch init`


## Parsing erros + exit codes
### OBRIGATÓRIO
- DEVE verificar exit code ANTES de parsear stdout
- Envelope de erro — `error` true, `ok` false, `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`, `best_candidate`, `match_count`
- Always-emit — `--filter` NÃO silencia falhas `ok` false
- `MATCH_FAILED` exit 65 — miss edit/fuzzy com `best_candidate`
- `MATCH_AMBIGUOUS` — inclui `best_candidate`; DEVE expandir contexto ou `--replace-all` (edit) — NUNCA chute
- `NO_MATCHES` exit 1 — search/replace zero matches
- permanent NUNCA retenta; transient DEVE backoff; conflict DEVE reler; precondition_failed DEVE corrigir pré-condição; DEVE usar `suggestion`
- Exit 0 sucesso; 1 `NO_MATCHES`; 2 arg inválido; 4 não encontrado; 65 entrada/`MATCH_FAILED`/ack; 78 config/feature; 81 checksum; 82 drift; 88 sintaxe; 124 timeout; 126 jail; 143 SIGTERM
- DEVE mapear demais códigos por `exit` e `error_class` sem inventar semântica
### PROIBIDO
- NUNCA parseie stderr para erro; NUNCA retente se `retryable` false
- NUNCA trate linha com `ok` false como sucesso; NUNCA chute entre candidatos ambíguos


## Fórmulas de execução obrigatórias
### OBRIGATÓRIO
- A LLM DEVE executar locking otimista — `CS=$(atomwrite --workspace . read arq | jaq -r .checksum) && echo novo | atomwrite --workspace . write --expect-checksum "$CS" --durability auto arq`
- A LLM DEVE executar edit com match_count — `atomwrite --workspace . edit src/f.rs --old x --new y --replace-all --fuzzy auto | jaq '{match_count,indent_adjusted,checksum}'`
- A LLM DEVE executar fuzzy replace — `atomwrite --workspace . replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run old_api new_api src/`
- A LLM DEVE recuperar exit 124 aumentando `--timeout-secs` ou estreitando o path
- A LLM DEVE ler `jaq .best_candidate` após exit 65
- A LLM DEVE executar recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- A LLM DEVE executar sparse — `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- A LLM DEVE executar merge line-based — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- A LLM DEVE executar semantic-search offline — `atomwrite --workspace . semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- A LLM DEVE executar host readiness — `atomwrite doctor --strict && atomwrite commands --include-globals`
- A LLM DEVE tratar exit 143 como cancel limpo e NUNCA reutilizar tempfile
- A LLM DEVE executar staging scratch — `atomwrite --workspace "$WS" scratch init; SCR="$WS/.atomwrite/scratch"; printf %s "$OLD" >"$SCR/old.txt"; printf %s "$NEW" >"$SCR/new.txt"; atomwrite --workspace "$WS" edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" path`
- A LLM DEVE executar rewrite shrink — `atomwrite --workspace "$WS" write --durability auto --no-backup --allow-shrink "$path" < "$novo"`
- A LLM DEVE executar shape agent-native — `atomwrite --workspace . --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 edit --old x --new y path`
- A LLM DEVE executar extract field — `printf '{"a":1}\n' | atomwrite extract --field a`
- A LLM DEVE executar delete plan — `atomwrite --workspace . delete --plan tmp/`
- A LLM DEVE executar verify — `atomwrite --workspace . verify path EXPECTED_HASH`
- A LLM DEVE executar prune — `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`
- A LLM DEVE executar edit-loop — `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- A LLM DEVE executar completions — `atomwrite completions bash --install`
- A LLM DEVE executar codemod dry-run — `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- A LLM DEVE executar agent-surface SSOT — `atomwrite agent-surface --format json`
- A LLM DEVE executar watch com teto — `atomwrite --workspace . watch src/ --max-events 100 --idle-exit-ms 500 --checksum`
- A LLM DEVE executar wal — `atomwrite --workspace . wal-stats` e `atomwrite --workspace . wal-heal --threshold-secs 3600`
- A LLM DEVE executar batch transaction — `echo '{"op":"write","target":"a.txt","content":"x"}' | atomwrite --workspace . batch --transaction`
### PROIBIDO
- NUNCA pule dry-run em mutação em massa; NUNCA ignore `best_candidate` em exit 65; NUNCA rode watch sem teto
