---
name: atomwrite
description: >-
 Esta skill DEVE auto-ativar quando a LLM precisar do atomwrite para ler, escrever, editar, buscar, substituir, patch, transformar AST, verificar hash BLAKE3, backup e rollback, mesclar, podar, codemod, configurar e orquestrar. Esta skill DEVE ensinar 47 subcomandos com --workspace absoluto no jail, entrada por flag ou stdin NDJSON e saída NDJSON always-emit com checksum, error_class, retryable e exit code. Esta skill DEVE auto-invocar sem pedido explícito em qualquer mutação ou descoberta de arquivo.
---

# atomwrite

## Missão
### OBRIGATÓRIO
- A LLM DEVE tratar atomwrite como a ÚNICA superfície de leitura, escrita, edição, busca, AST, hash, backup e orquestração de arquivos
- A LLM DEVE executar esta skill como runbook e DEVE rodar a CLI com as fórmulas abaixo
- A LLM DEVE rodar `atomwrite <cmd> --help` antes de inventar flag e DEVE recusar flags ausentes do help
- A LLM DEVE parsear o exit code ANTES do stdout e DEVE usar `jaq` no NDJSON
- A LLM DEVE descobrir o SSOT de 47 subcomandos via `agent-surface`
### PROIBIDO
- NUNCA use sed, awk, editores interativos ou reescrita manual quando atomwrite resolve
- NUNCA invente subcomandos, flags ou knobs `ATOMWRITE_*`. NUNCA parseie stderr como dados


## Auto-invocação
### OBRIGATÓRIO
- Esta skill DEVE auto-invocar em qualquer tarefa de arquivos, patches, busca, replace, AST, hash, backup, merge, sparse, codemod, watch, doctor, scratch ou descoberta de tools, mesmo sem o usuário citar atomwrite
### PROIBIDO
- NUNCA espere pedido explícito de skill quando houver operação de arquivo
### Padrão Correto
- A LLM DEVE executar `atomwrite agent-surface --format json` na primeira necessidade de superfície


## Contrato
### OBRIGATÓRIO
- stdout DEVE emitir SEMPRE NDJSON (um JSON por linha), stderr só logs
- Toda mutação DEVE usar tempfile-fsync-rename, sucesso DEVE expor o BLAKE3: `write`, `hash` e `read` trazem `checksum`, e `edit` traz `checksum_before` e `checksum_after` (nunca `checksum`)
- Todo envelope terminal DEVE trazer `workspace_source` valendo `argv` quando `--workspace` foi passado e `cwd` quando não foi, mais `target_resolved` e `target_source`. DEVE conferir os três antes de afirmar onde a CLI tocou
- Cada schema é UNIÃO de todos os eventos do verbo mais o envelope de erro, então DEVE validar contra a união inteira em `docs/schemas/` e NUNCA contra um formato fixo imaginado
- Ops de arquivo DEVEM passar `--workspace <DIR>` e DEVEM ficar no jail
- `--json` é aceita e DEVE ser ignorada (saída SEMPRE NDJSON)
- Schemas DEVEM seguir `docs/schemas/` quando validação formal for necessária
- Precedência DEVE ser flags CLI, depois o `.atomwrite.toml` do workspace, depois o `config.toml` XDG global (os dois arquivos são mesclados chave a chave), depois defaults embutidos
### PROIBIDO
- NUNCA escreva fora do jail. NUNCA dependa de knobs de env de produto em runtime
- NUNCA trate exit 1 em `search`/`replace`/`transform`/`scope`/`semantic-search` como crash quando for `NO_MATCHES`
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace "$WS" --timeout-secs 120 --locale pt-BR --no-progress <cmd>`


## Workspace absoluto
### OBRIGATÓRIO
- A LLM DEVE definir `WS` UMA vez como o caminho ABSOLUTO da raiz do workspace, antes de qualquer subcomando, e escrever `--workspace "$WS"`
- A LLM DEVE passar `--workspace` ABSOLUTO a processo destacado e a subagente, que não herdam CWD com significado
- Em PowerShell a mesma regra vale com `$WS = 'C:\caminho\da\raiz'` e `--workspace $WS`
### PROIBIDO
- NUNCA escreva `--workspace` com ponto como valor, porque `.` é o CWD do PROCESSO e nenhum contrato deste documento acorda qual é o CWD
- NUNCA esqueça que o jail é ANCORADO no valor de `--workspace`, então um `.` errado move o jail inteiro para a pasta errada em SILÊNCIO
- NUNCA esqueça que projetos ANINHADOS fazem `.` resolver para o filho quando a intenção era o pai
### Padrão Correto
- A LLM DEVE executar `WS=/caminho/absoluto/da/raiz; atomwrite --workspace "$WS" read --head 20 path`


## Contratos de segurança do agente
### OBRIGATÓRIO
- Sobrescrita grande DEVE passar `--ack-overwrite` (XDG `[write].confirm_large_bytes`, padrão 100 KiB). Falta = exit 65, `--ack-overwrite` cobre só o arquivo grande, e encolher mais de 50% exige `--allow-shrink` à parte (sem ele sai 65)
- Delete plan-only DEVE usar `delete --plan`. Execução DEVE omitir `--plan` e flags de confirm. Diretório exige `-r` até no plano (sem ele sai 65 com `missing_flag`)
- `watch` DEVE terminar com NDJSON `watch_summary` e DEVE ter teto `--max-events` ou timeout (idle ~500 ms)
- `watch` só existe em build com `--features watch` (`cargo install atomwrite --features watch`), e sem ela sai 78 `CONFIG_INVALID`, então DEVE checar `atomwrite agent-surface | jaq '.tools[]|select(.name=="watch").available'` antes
- `semantic-merge` DEVE gravar markers line-based por padrão. Prefer-ours sem markers DEVE usar `--no-conflict-markers`
- Exit 65 DEVE ler `best_candidate` e `match_count`, 82 DEVE recarregar checksum, 124 DEVE aumentar timeout ou estreitar path, 143 = SIGTERM limpo, 126 = jail, 81 = checksum, 88 = sintaxe
### PROIBIDO
- NUNCA passe `delete --confirm`/`--yes`/`-y`. NUNCA sobrescreva grande sem `--ack-overwrite`
- NUNCA use confirm interativo. NUNCA assuma stdout vazio de `watch`. NUNCA desligue markers por acidente
### Padrão Correto
- A LLM DEVE executar `printf 'x' | atomwrite --workspace "$WS" write --ack-overwrite --allow-shrink big.txt` e `atomwrite --workspace "$WS" delete --plan -r tmp/`


## Flags globais
### OBRIGATÓRIO
- DEVE usar `--workspace <DIR>`, `--config <PATH>`, `--timeout-secs <N>` ou `--timeout <N>` (padrão 120 → exit 124, `0` só se intencional)
- DEVE usar `-j N` / `--threads N` / `--max-concurrency N` (omitido ou zero = todos os cores até um teto embutido de 16, limitado pela RAM, um N explícito vale como pedido, até acima de 16)
- DEVE usar `--max-filesize <BYTES>` (padrão global 1 GiB), `--locale <TAG>` (NUNCA global `--lang`)
- DEVE usar `--json-schema`, `--no-auto-heal`, `--no-progress`, `--no-gitignore`, `--hidden`, `--follow-symlinks`
- `--strict-atomic` DEVE falhar com exit 91 quando o rename atômico for impossível, em vez de cair na cópia entre dispositivos
- DEVE usar `--color auto|always|never` ou `--no-color`, `-v`/`-vv`/`-vvv` ou `-q`/`-qq`
- Shape agent-native OBRIGATÓRIO: `--select`/`--fields KEYS`, `--filter key=value` (AND, `!=`/`==`), `--limit N`
- Envelopes always-emit: `--filter` NÃO silencia `ok` false. DEVE preferir shape a pipes jq/sed
- DEVE descobrir superfície via `atomwrite agent-surface` (SSOT 47)
- Shape de stream, tudo opcional e global: `--sort-by path-line|line-path|CHAVE`, `--dedupe CHAVES` (CSV), `--aggregate CHAVE[:OP[:CAMPO]]` (OP `count|sum|min|max`), `--join CHAVE:SUFIXO` (a tabela vem do stdin), `--diff-against CAMINHO` (NDJSON base dentro do jail, emite uma linha `stream_diff`), `--redact`, `--normalize-encoding` (NFC), `--truncate-content CHAVE:TAM`
- `--max-output-bytes N` corta todo valor string, `checksum` incluído, e DEVE vir junto com `--allow-truncate` (cada um é rejeitado sem o outro, exit 2). NUNCA use os dois quando o `checksum` ou o `content` inteiro for necessário
- `--non-interactive` declara execução hospedada por agente (nunca pergunta, nunca lê confirmação do stdin), `--confirm-side-effect` reconhece um efeito colateral só onde o contrato do comando pede (veja `commands --include-globals`)
- `--print-schema [NOME]` lista os 55 nomes de schema, ou emite um schema. Nome desconhecido sai 65. Todo verbo que escreve mais de um registro serve a união de todo registro mais o envelope de erro, e os nomes de sinal são `best-candidate`, `cancelled`, `error`, `progress`, `stream_diff`, `write-risk`, `count-by-ext` e `count-by-size`
### PROIBIDO
- NUNCA invente flags ausentes de `atomwrite --help`
- NUNCA use `-n` global para limit (`-n` é do `replace`, use `--limit`)
- NUNCA despeje NDJSON completo quando `--select` basta
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace "$WS" --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 <cmd>`


## Superfície completa de comandos
### OBRIGATÓRIO
A LLM DEVE invocar cada um dos 47 subcomandos. Prefixo típico `atomwrite --workspace "$WS"` omitido quando óbvio. A LLM DEVE incluir `--workspace` em ops de arquivo.

1. `read`: ler com checksum, `read --head 40 path`
2. `write`: criar/sobrescrever via stdin, `echo body | write --durability auto path`
3. `edit`: edição cirúrgica, `edit path --old a --new b --fuzzy auto`
4. `search`: localizar padrões, `search TODO src/ --include '*.rs'`
5. `replace`: replace multiarquivo, `replace --dry-run --fuzzy auto OLD NEW src/`
6. `hash`: BLAKE3, `hash path | jaq -r .checksum`
7. `delete`: remover com plan, `delete --plan path`
8. `count`: contagens, `count --by-size --top 10 src/`
9. `diff`: diff unificado, `diff a.rs b.rs --unified`
10. `move`: mover/renomear, `move src/old.rs src/new.rs`
11. `copy`: copiar, `copy --recursive --preserve src/ dest/`
12. `list`: listar árvore, `list --long --depth 2 src/`
13. `extract`: projetar campos NDJSON, `extract --field path` ou `extract path`
14. `calc`: expressões/unidades, `atomwrite calc "2 hours to seconds"`
15. `regex`: gerar regex, `atomwrite regex abc-12 xyz-99 id-7 --digits`
16. `transform`: rewrite AST, `transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
17. `scope`: ações gramaticais, `scope src/ --lang rust --query comments --delete --dry-run`
18. `batch`: manifesto NDJSON, `echo '{"op":"write","target":"a.txt","content":"x"}' | batch --transaction`
19. `backup`: backup com retention, `backup path --retention 3`
20. `rollback`: restaurar, `rollback path --latest --verify`
21. `apply`: patch unified/SR, `echo body | apply path --format full`
22. `set`: gravar dotted TOML/JSON, `set Cargo.toml package.name x`
23. `get`: ler dotted, `get config.toml database.pool.max`
24. `del`: remover chave, `del --force-missing config.toml features.x`
25. `case`: case/subvert em ARQUIVO (diretório não faz nada), `case --to pascal --subvert user_id UserId --dry-run src/a.rs`
26. `query`: nós AST, `query --kinds src/main.rs`
27. `outline`: outline, `outline --kind function_item --positions src/main.rs`
28. `wal-stats`: journals WAL, `wal-stats`
29. `wal-heal`: remover journals terminais antigos, `wal-heal . --threshold-secs 3600`
30. `semantic-merge`: merge line-based, `semantic-merge --base b --ours o --theirs t --output out --fail-on-conflict`
31. `sparse`: monorepo sob orçamento, `sparse list src/ --max-files 50 --max-bytes 524288`
32. `recipe`: receitas, `recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run`
33. `stat`: alias `read --stat`, `stat path`
34. `agent-surface`: SSOT 47 tools, `atomwrite agent-surface --format json`
35. `watch`: FS com teto, só em build `--features watch` (sem ela sai 78), `watch src/ --max-events 100 --idle-exit-ms 500`
36. `codemod`: rewrites em massa, `codemod --rules rules.yaml --dry-run src/`
37. `semantic-search`: Jaccard offline, `semantic-search query src/ --k 20 --min-score 0.05`
38. `doctor`: host, `atomwrite doctor --strict`
39. `scratch`: staging no jail, `scratch init .atomwrite/scratch`
40. `locale`: locale, `atomwrite locale --set pt-BR`
41. `commands`: árvore JSON, `atomwrite commands --include-globals`
42. `completions`: shell, `atomwrite completions bash --install`
43. `prune-backups`: podar, `prune-backups --max-age-secs 86400 --dry-run src/lib.rs`, só nomes `.bak.<carimbo>` são candidatos
44. `edit-loop`: N pares numa escrita, tudo-ou-nada (par sem match sai com 65), `--partial` grava os que casam e sai com 87, `echo '[{"old":"a","new":"b"}]' | edit-loop path`
45. `verify`: hash esperado, `verify path EXPECTED_HASH`
46. `extract-artifact`: sinal delimitado de cabeçalhos de PDF, OOXML e imagem, `extract-artifact path`
47. `config`: chaves de ajuste persistentes com `--scope global|workspace`, `config set fuzzy.max_content_bytes 1000000 --scope global`
### PROIBIDO
- NUNCA invente um 48º subcomando. NUNCA omita `agent-surface` quando a superfície for desconhecida
- NUNCA omita `--workspace` em ops de arquivo dentro do jail


## Fuzzy
### OBRIGATÓRIO
- Fuzzy em `edit`/`replace` padrão `auto`, valores SOMENTE `--fuzzy auto|aggressive|off`
- `--fuzzy off` = exact-only. Miss edit = exit 65 `MATCH_FAILED`, miss replace árvore = exit 1 `NO_MATCHES` frequente
- DEVE usar `--fuzzy-threshold` 0.0–1.0. Com `--regex` no replace o fuzzy DEVE ser ignorado
- Multi-apply DEVE ser one-pass L→R no original. NEW embute OLD → força 1 ocorrência. Sem `--max-replacements` padrão multi = 1, teto duro 10000
- Caps: pattern 64 KiB, Levenshtein 8192, janelas 4096, crescimento max 4x ou +16 MiB
- Estratégias normalizadas (`line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `trimmed_boundary`, `escape_normalized`) emitem `similarity` null: NUNCA trate 1.0 falso como igualdade byte a byte se não for `exact`
- Soft-floor `context_aware`: XDG `[fuzzy].soft_floor_delta` padrão 0.05. Aceite efetivo = threshold OU threshold menos soft_floor_delta
- `block_anchor` dual-gate (bordas + ratio do miolo)
- Conteúdo acima de `[fuzzy].max_content_bytes` (padrão 256 MiB) nunca roda a cascata: `edit` sai 65 `INVALID_INPUT` citando a chave e `--fuzzy off`, e `replace` registra o arquivo em `files_failed`. Match exato e `--fuzzy off` ainda funcionam acima dele
- `escape_normalized` é pulada acima de 32 MiB de conteúdo, e a cascata segue com as outras estratégias
- Um miss em arquivo enorme ainda respeita `--timeout-secs`: exit 124 com envelope `cancelled`, nunca um `MATCH_FAILED` tardio
- `MATCH_AMBIGUOUS` DEVE incluir `best_candidate`. DEVE expandir `--old` ou `--replace-all` (edit) ou estreitar escopo: NUNCA chute. Só ocorre quando dois ou mais candidatos passam o dual-gate, e um bloco ausente que apenas lembra várias linhas é `MATCH_FAILED` com `best_candidate`
- Fuzzy NÃO é AST: DEVE usar `transform`/`scope`/`codemod` para semântica
### PROIBIDO
- NUNCA espere rebusca do texto inserido no mesmo replace fuzzy. NUNCA use `--replace-all` em `replace`
- NUNCA defina `--timeout-secs 0` em monorepo fuzzy salvo job sem prazo intencional
- NUNCA rode `--fuzzy aggressive` em árvore sem `--dry-run`. NUNCA passe flag inventada `--strategy` no edit
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace "$WS" replace --dry-run --fuzzy auto --max-replacements 1 OLD NEW src/`


## Hierarquia de edição + fluxo de execução
### OBRIGATÓRIO
- Ordem do mais preciso ao mais bruto: `search` → `transform` → `scope` → `codemod` → `replace` → `edit` → `write`
- `write` SOMENTE para criar ou sobrescrever arquivo inteiro de forma intencional
- DEVE localizar com `search` ou `sparse` antes de mutar. DEVE capturar checksum com `read` sob concorrência
- DEVE rodar `--dry-run` antes de mutação destrutiva em massa. DEVE aplicar sobrescrita concorrente com `--expect-checksum`
- DEVE validar `checksum`, `match_count`, `indent_adjusted`, `platform` quando presentes
- DEVE tratar falhas por `error_class` e `suggestion`. Retentar SOMENTE se `retryable` true
### PROIBIDO
- NUNCA reescreva inteiro com `write` quando `edit`/`replace`/`transform` resolve
- NUNCA ignore exit não-zero exceto zero-match documentado
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace "$WS" search alvo src/` e só então mutar


## Backup e configuração
### OBRIGATÓRIO
- Backup default true nos mutadores via `--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`
- Backup padrão DEVE criar `.bak.<timestamp>` adjacente e auto-remover no sucesso, em falha DEVE permanecer
- A LLM DEVE passar `--no-backup` em todo `delete` e `replace`, para não deixar `.bak`
- Nenhum hook apaga `.bak`, então `--no-backup` é a ÚNICA garantia de não deixar resíduo
- `rollback` DEVE usar `--backup` explícito para backup opt-in do alvo restaurado
- `move` e `copy` DEVEM passar `--force` ou `--backup` para sobrescrever destino
- Backup DEVE usar `reflink_or_copy` e NUNCA hardlink do arquivo vivo
- `.atomwrite.toml` governa `[defaults]` e `[fuzzy]` (`auto|aggressive|off`), flags CLI vencem TOML
### PROIBIDO
- NUNCA passe `--backup` e `--no-backup` juntos (exit 2)
- NUNCA assuma que TOML `backup = true` sozinho autoriza sobrescrita em move/copy
### Padrão Correto
- A LLM DEVE definir `[fuzzy] mode = "auto"` e `threshold = 0.85` em `.atomwrite.toml`


## Verbo config
### OBRIGATÓRIO
- `config path|list|get|set|unset` é a ÚNICA forma de mudar uma chave de ajuste persistente. NUNCA edite `config.toml` nem `.atomwrite.toml` à mão
- `config set CHAVE VALOR --scope global|workspace`, `config unset CHAVE --scope ...` e `config path --scope ...` DEVEM passar `--scope` (sem padrão, sem ele sai 2 e nada é tocado)
- `config list` e `config get CHAVE` DEVEM ser lidos antes de um `set`. Eles mostram `value`, `origin` (`default`, `global`, `workspace` ou `explicit` com `--config`) e `default`, sem `--scope`
- O escopo `global` é o `config.toml` XDG (modo 0600), o escopo `workspace` é o `.atomwrite.toml` na raiz do workspace. O `set` preserva comentários, valida o arquivo inteiro e grava por tempfile-fsync-rename sem backup
- O evento terminal carrega `target_resolved` e `target_source: "argv"`. DEVE conferir que o caminho é o arquivo do escopo pretendido
- As 37 chaves são `defaults.line_ending`, `defaults.backup`, `defaults.retention`, `defaults.max_filesize`, `fuzzy.mode`, `fuzzy.threshold`, `fuzzy.threshold_aggressive`, `fuzzy.threshold_context`, `fuzzy.threshold_jw`, `fuzzy.soft_floor_delta`, `fuzzy.max_content_bytes`, `write.confirm_large_bytes`, `write.shrink_block_percent`, `write.auto_rotate_max_age_secs`, `write.content_risk_patterns`, `watch.idle_exit_ms`, `watch.debounce_ms`, `log.file`, `log.rotation`, `storage.home`, `locale`, `concurrency.max_threads`, `concurrency.ram_fraction_percent`, `search.context`, `search.smart_case`, `search.max_filesize`, `search.max_columns`, `search.max_inflight_bytes`, `summary.errors_max_entries`, `lock.acquire_timeout_ms`, `retry.base_ms`, `retry.max_ms`, `retry.max_elapsed_ms`, `extract.max_sample_rows`, `extract.max_text_chars`, `extract.max_sheets` e `extract.max_slides`
- `log.file`, `log.rotation`, `storage.home` e `locale` são só globais (escopo workspace = exit 65). Chave desconhecida ou valor fora da faixa sai 65 e não grava nada
- Faixas aceitas: `concurrency.max_threads` de 1 a 256, `concurrency.ram_fraction_percent` de 1 a 100, `lock.acquire_timeout_ms` de 100 a 3600000, `retry.base_ms` e `retry.max_ms` de 1 a 60000 (a base não pode passar do máximo), `retry.max_elapsed_ms` de 1 a 3600000, `search.max_inflight_bytes` de 4 MiB a 1 GiB, `extract.max_sample_rows` até 100000, `extract.max_text_chars` até 10000000, `extract.max_sheets` e `extract.max_slides` até 10000
- Uma flag ainda vence a configuração (`extract-artifact --max-text-chars` e `--max-sample-rows`), e `--config CAMINHO` substitui os dois arquivos em `get` e `list`
- `config` funciona com um `config.toml` quebrado. Um arquivo que não é TOML válido nunca é reescrito (`set` e `unset` saem 78)
### PROIBIDO
- NUNCA invente chave, `defaults.line_ending`, `defaults.backup`, `defaults.retention` e `defaults.max_filesize` SÃO chaves do `config` e mudam o padrão de todo verbo mutador
- NUNCA rode `config set` nem `config unset` sem `--scope`
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace "$WS" config get fuzzy.max_content_bytes` e depois `atomwrite --workspace "$WS" config set fuzzy.max_content_bytes 1000000 --scope global`


## Mutação write/edit/replace
### OBRIGATÓRIO
- `write` DEVE enviar conteúdo via stdin. DEVE usar `--expect-checksum`, `--durability full|fast|auto` (padrão `auto`)
- Rewrite que encolhe >50% DEVE usar `--allow-shrink`
- DEVE usar `--allow-empty-stdin`, `--no-checksum-when-empty`, `--syntax-check` (exit 88), `--ack-overwrite`
- Flags write: `--append`, `--prepend`, `--max-size`, `--line-ending`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--require-large-ack`, `--risk-threshold`, `--wal-policy`, `--dry-run`, opts de backup
- SIGTERM no write DEVE limpar tempfile e retornar exit 143
- `edit` DEVE usar `--old`/`--new` (pares repetíveis). Multi-par emite `pairs_total`/`pair_results`. Falha aborta lote salvo `--partial`
- Multi-ocorrência em edit DEVE usar `--replace-all` e DEVE emitir `match_count`. Indent realinhado emite `indent_adjusted` true
- Flags posição: `--after-line`, `--before-line`, `--range`, `--delete-range`, `--after-match`, `--before-match`, `--between`, `--multi`
- Patch com `//`, jq, aspas ou multilinha DEVE usar `--old-file`/`--new-file` estagiados em `$WS/.atomwrite/scratch/` após `scratch init .atomwrite/scratch`
- Patches com `//`, jq, aspas, `$`, crases ou multilinha DEVEM usar `apply --no-backup` com heredoc `<<'EOF'` e blocos SEARCH/REPLACE, porque o texto vai cru, sem JSON e sem escape
- A LLM DEVE preferir `apply` com heredoc ao `edit-loop`, que exige JSON no stdin e leva o modelo a usar `python` só para gerar esse JSON
- DEVE usar `--expect-checksum` e `--allow-sequential-drift` só em pipeline controlado
- Stdin de terminal em inserção DEVE falhar exit 65 sem travar
- Multi-root: se alvo está em skills no home, DEVE passar `--workspace` na raiz permitida
- `replace` exit 1 = zero matches (NÃO crash). DEVE SEMPRE `--dry-run`/`--preview` antes de apply em massa
- Multi fuzzy em replace DEVE usar só `--max-replacements` (`-n`)
- Flags replace: `--regex`, `-w/--word`, `-F/--literal`, `-g/--include`, `--exclude`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case`, opts de backup
### PROIBIDO
- NUNCA pipe edit para `jaq` sem `${PIPESTATUS[0]}`. NUNCA passe `--strategy` no edit
- NUNCA coloque old/new-file sob `/tmp` do SO fora do jail
- NUNCA passe `$VAR` sem aspas quando o dólar literal deve sobreviver: DEVE quote `"$VAR"`
- NUNCA trate fuzzy como AST. NUNCA quote no shell patches frágeis como `--old`/`--new` quando scratch existe
- NUNCA use `--replace-all` em replace. NUNCA pule dry-run em árvore grande. NUNCA encolha >50% sem `--allow-shrink`
### Padrão Correto
- A LLM DEVE executar edição multilinha: `atomwrite --workspace "$WS" apply --no-backup path <<'EOF'`, depois uma linha com `<<<<<<< SEARCH`, o texto antigo, uma linha com `=======`, o texto novo, uma linha com `>>>>>>> REPLACE`, e por fim `EOF`. Vários blocos cabem no mesmo heredoc
- A LLM DEVE executar staging: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch; SCR="$WS/.atomwrite/scratch"; printf %s "$OLD" >"$SCR/old.txt"; printf %s "$NEW" >"$SCR/new.txt"; atomwrite --workspace "$WS" edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" path`
- A LLM DEVE executar `atomwrite --workspace "$WS" write --durability auto --no-backup --allow-shrink "$path" < "$novo"`


## Path e estruturado
### OBRIGATÓRIO
- `delete` DEVE usar `--no-backup`, idade via `--older-than`, recursivo `-r`, globs `-g/--include`, plan via `--plan`
- `delete -r` DEVE respeitar o filtro do walker, então arquivo ignorado como `*.bak.*` com gitignore ligado é preservado e NUNCA entra na remoção
- `move` DEVE usar `--preserve-hardlinks` quando o alvo tiver hardlink
- `move`/`copy` DEVEM `--force` ou `--backup` para sobrescrever destino
- `copy` DEVE usar `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr` quando exigido
- `apply` detecta unified, SEARCH/REPLACE, markdown, full. DEVE usar `--format auto|unified|search-replace|full|markdown` se ambíguo
- `set`/`get`/`del` SOMENTE TOML ou JSON via dotted path. Get ausente = exit 65, `del` aceita `--force-missing`
- `case` DEVE `--subvert OLD NEW` com OLD diferente de NEW (iguais saem 65) e DEVE dry-run em codebase grande
- `case` só processa ARQUIVO: diretório sai 0 com `identifiers_total` 0 sem tocar em nada, então DEVE passar o arquivo
### PROIBIDO
- NUNCA passe `--confirm`/`--yes`/`-y` em delete. NUNCA use set/get/del em texto puro
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace "$WS" move src/old.rs src/new.rs` e `atomwrite --workspace "$WS" set Cargo.toml package.name x`


## Leitura e busca
### OBRIGATÓRIO
- `read`: conteúdo + checksum/size/lines/mode, `stat PATH` é alias de `read --stat`
- `read --format raw` DEVE aceitar vários caminhos e glob numa chamada: relativo resolve contra `--workspace`, glob sem match sai 4 `GLOB_NO_MATCHES`, alvo fora do jail sai 126 sem imprimir byte, e metacaractere literal que existe no disco é lido literal
- `read --format raw` DEVE gravar os bytes no stdout e mover o envelope JSON para o stderr. DEVE separar os dois canais antes de parsear qualquer coisa
- `case --limit` DEVE publicar `walker_stopped_early`, e `list --limit` ou `hash --limit` DEVEM publicar `files_budget_dropped`. Campo ausente significa que o walker terminou sem corte
- Flags read: `--lines`, `--line`, `-C/--context`, `--head`, `--tail`, `--format raw|ndjson`, `--bytes` (redundante com `--format raw`, aceito também no `stat`), `--grep`, `--verify-checksum` (mismatch exit 81)
- `search` ANTES de mutar se alvo desconhecido, flags `-g/--include`, `--exclude`, `-C`, `-F/--fixed`, `-e/--regex`, `-w/--word`, `-i/--case-insensitive`, `-S/--smart-case`, `-c/--count`, `-l/--files`, `-m/--max-count`, `-U/--multiline`, `-P/--pcre2`, `--offset`, `--invert`, `--binary`, `--sort`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Default local `--max-filesize` no search = 10 MiB
- `list`: `--include`, `--exclude`, `--long`, `--depth`, `--count-by-ext`, `--all`
- `count`: `--by-extension`, `--by-size`, `--top`
- `extract`: posicionais OU `--field` local (repetível, NÃO combinar ambos) + `--delimiter` + `--stdin`
- `diff`: `--unified`, `--stat`, `-C`, `--algorithm myers|patience|lcs`, path `-` = stdin
### PROIBIDO
- NUNCA passe `extract --select`: `--select` é shape global. Flag local é `--field`
- NUNCA trate zero matches de search como crash
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace "$WS" read --head 20 path` e `printf '{"a":1}\n' | atomwrite extract --field a`


## AST
### OBRIGATÓRIO
- Exit 1 em `transform` = zero matches. DEVE definir `-l/--language`, `$NAME` um nó, `$$$ARGS` múltiplos
- Single-rule DEVE passar `-p/--pattern` e `-r/--rewrite` juntos
- Flags transform: `--rules` (mesmo YAML do `codemod`), `--inline-rules`, `--verify-parse`, `--include`, `--exclude`, `--dry-run`, opts de backup
- DEVE preferir transform sobre replace quando o padrão for sintático
- `scope` DEVE `--language` (alias `--lang`), `--query` ou `--pattern`, `--delete`/`--action`/`--replace-with`
- Queries scope: comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- `query`: `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- `outline`: `--kind` repetível, `--positions`, `--language`
### PROIBIDO
- NUNCA aplique `scope --delete` sem dry-run. NUNCA use fuzzy textual quando transform/scope resolve
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace "$WS" transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## Orquestração
### OBRIGATÓRIO
- `batch`: NDJSON stdin com `op` em write/replace/delete/edit/move/copy/hash, move/copy com `"force":true` para sobrescrever
- DEVE usar `--transaction`, `--file <PATH>`, `--dry-run`, `--batch-size`, `--input-schema`, opts de backup
- `batch --transaction` com falha DEVE publicar UM único evento `rollback` com `ok` false, `code` e `exit`, e o primeiro alvo já revertido. NUNCA espere um erro por operação
- `sparse list` aceita `--max-files`, `--max-bytes`, `--include` e `--exclude` (globs de verdade, os dois aplicados, um inválido sai 65), `sparse read --paths-file` aceita `--head` e `--max-files`, `sparse outline` roda sob orçamento
- `sparse list` DEVE publicar `files_scanned` e `path_bytes`. DEVE conferir os dois para saber quanto do orçamento o walker consumiu
- `recipe list` e `recipe run --name search-replace-verify`. Receitas embutidas `search-replace-verify` e `edit-loop-syntax-check`
- `recipe` sem limite DEVE emitir cada `match`, um `hash`, um `plan` e dois `recipe_step` com `child_summary`, `--limit N` DEVE acrescentar `steps_skipped` e NUNCA truncar em silêncio
- Recipe search/replace/hash DEVEM excluir `*.bak.*` por padrão
- `recipe run` aceita `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- `semantic-merge --base A --ours B --theirs C --output OUT` é line-based com markers ON (NUNCA AST/embedding)
- Flags merge: `--fail-on-conflict`, `--write-conflict-markers`, `--no-conflict-markers`, `--expect-checksum`, opts de backup
- `agent-surface --format json` = manifesto SSOT
- `watch`: `--debounce-ms`, `--max-events`, `--idle-exit-ms`, `--checksum`, `--gitignore`. Feature binária `watch` (build com `--features watch`, senão exit 78). Termina com `watch_summary`
- `codemod --rules rules.yaml --dry-run` ANTES de apply. O `rules.yaml` é uma lista YAML de itens com `id`, `language`, `pattern` e `rewrite`, e o formato com `rule:` e `fix:` do ast-grep sai 65
- `codemod` DEVE publicar `rule_id` por regra e `by_rule_id` somando os matches. DEVE ler `by_rule_id` para saber qual regra rendeu antes de culpar o alvo
- `semantic-search "query" PATH --k 20 --min-score 0.05` é Jaccard offline, `--index-dir` só se índice local for necessário, `--include-backups` inclui os `*.bak.*` que o padrão exclui
### PROIBIDO
- NUNCA rode `watch` sem teto max-events ou timeout
- NUNCA trate zero hits de semantic-search como crash. NUNCA trate semantic-merge como AST
### Padrão Correto
- A LLM DEVE executar `echo '{"op":"write","target":"a.txt","content":"x"}' | atomwrite --workspace "$WS" batch --transaction`


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
- `hash`: BLAKE3. Ler campo `checksum` e NUNCA `value`. Flags `--verify`, `--stdin`, `-r/--recursive`, `--exclude`
- `verify <PATH> <EXPECTED_HASH>`: exit 0 se casa, 81 se não
- `backup`: `--retention`, `--output-dir`, `rollback --latest --verify`. Flags rollback `--timestamp`, `--latest`, `--verify`, `--dry-run`, `--backup` opt-in
- `prune-backups`: `--max-age-secs` ou `--max-count <N>` e NUNCA `--max-age`. Diretório puro DEVE ser recusado com 65 `INVALID_INPUT` e sugestão, porque a varredura exige raiz de arquivos
- `calc` sem workspace, `regex` a partir de ≥3 exemplos (`-d`, `-w`/`--words`, `-s`/`--spaces`, `-r`/`--repetitions`, `-i`/`--case-insensitive`, `--no-anchors`, `--stdin`)
- `wal-stats` só inspeciona, `wal-heal PATH` remove só journals `committed` e `aborted` mais velhos que `--threshold-secs` (`--max-duration-ms`, `--dry-run`) e nunca remove `started`. O PATH é obrigatório
- Journals WAL: `--wal-policy always` registra todo write. Em `auto`, depois de 10 journals abertos em 60 s as próximas gravações não abrem nenhum (continuam atômicas). Journal que gravou é sempre removido, `startup_wal_heal` roda só em verbo que muta e só nos diretórios citados no argv. Uma execução cancelada por SIGTERM, SIGINT ou prazo não deixa journal, enquanto um crash (`kill -9`) deixa o journal `started`, o tempfile e o `.lock` com o alvo íntegro, e o `wal-heal` NÃO remove journal `started`, então DEVE conferir o alvo e tratar o resíduo à mão
- `edit-loop` aplica N pares numa escrita e é tudo-ou-nada, `--partial` sai com 87 (`EDIT_LOOP_PARTIAL`). Verbos que gravam exigem caminho explícito (exit 2 sem ele) e o evento terminal reporta `target_source: "argv"`, `completions` bash/zsh/fish/elvish/powershell com `--install`
- `scratch init .atomwrite/scratch` cria staging em `$WS/.atomwrite/scratch/`, e o PATH é obrigatório (sem ele sai 2)
### PROIBIDO
- NUNCA use `/tmp` do SO fora do jail para staging. NUNCA leia `value` no lugar de `checksum`
### Padrão Correto
- A LLM DEVE executar `atomwrite --workspace "$WS" hash path | jaq -r .checksum`


## Parsing erros + exit codes
### OBRIGATÓRIO
- DEVE verificar exit code ANTES de parsear stdout
- Envelope de erro: `error` true, `ok` false, `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `suggestion_lang`, `missing_flag` (a flag exigida que faltou), `workspace`, `best_candidate`, `match_count`
- Always-emit: `--filter` NÃO silencia falhas `ok` false
- `MATCH_FAILED` exit 65: miss edit/fuzzy com `best_candidate`
- `MATCH_AMBIGUOUS`: inclui `best_candidate`. DEVE expandir contexto ou `--replace-all` (edit): NUNCA chute
- `NO_MATCHES` exit 1: search/replace zero matches
- permanent NUNCA retenta. Transient DEVE backoff. Conflict DEVE reler. Precondition_failed DEVE corrigir pré-condição. DEVE usar `suggestion`
- Exit 0 sucesso, 1 `NO_MATCHES`, 2 arg inválido, 4 não encontrado, 13 `PERMISSION_DENIED`, 65 entrada/`MATCH_FAILED`/ack/conteúdo fuzzy acima do teto/chave ou valor inválido do `config`, 78 config/feature (também chave desconhecida dentro do `config.toml`), 81 checksum, 82 drift, 83 timeout de lock, 84 conflito de merge, 87 edit-loop parcial, 88 sintaxe, 124 timeout, 126 jail, 129 SIGHUP, 130 SIGINT ou cancelado, 141 pipe fechado, 143 SIGTERM, 255 erro interno
- DEVE mapear demais códigos por `exit` e `error_class` sem inventar semântica
- Uma execução com falha termina em EXATAMENTE UM evento terminal com `ok` false (`error_class`, `retryable` e `suggestion` vão dentro dele). NUNCA espere uma segunda linha de erro
- Uma execução cancelada termina em `{"type":"cancelled","reason",...,"deadline_exceeded"}` com exit 124 (prazo, `deadline_exceeded` true), 130 (SIGINT) ou 143 (SIGTERM), e NUNCA em `MATCH_FAILED` tardio
- `search`, `replace`, `transform` e `scope` relatam arquivo que não conseguiram ler ou gravar como `files_failed` e `errors[]` (primeiras 50 entradas) no resumo. Zero matches sai com o primeiro erro, matches com falha saem 0, então DEVE ler `files_failed` antes de confiar num resultado
- Exit 141 significa que o leitor fechou o pipe e a saída está incompleta. O processo faz a limpeza antes (nenhum lock, journal ou tempfile fica), e DEVE reconferir o estado em vez de assumir que o trabalho terminou
- Verbos de leitura-modificação-escrita (`edit`, `edit-loop`, `set`, `del`, `config set`, `apply`, `write --append`) serializam no lock advisory, `case`, `transform` e `scope` falham um arquivo que mudou desde a leitura com `STATE_DRIFT` (82, retryable), então DEVE refazer a execução e NUNCA assumir que a gravação aconteceu
- `apply` com diff unificado compara contexto e linhas removidas com o arquivo (deslocamento até 64 linhas): divergência é `MATCH_FAILED` (65) com `best_candidate` e nada gravado. SEARCH/REPLACE com qualquer bloco ausente falha o lote inteiro do mesmo jeito (`MATCH_FAILED`), e um bloco equivalente em dois lugares é `MATCH_AMBIGUOUS` (65) com `match_count`. NUNCA espere aplicação parcial
- `get`, `set` e `del` num arquivo TOML ou JSON inválido saem 65 `INVALID_INPUT` (só linha e coluna). Um erro sem tipo publica `INTERNAL_ERROR` no stdout
- Um erro de entrada do usuário (regex ou glob inválido em qualquer verbo, `sparse` e `hash -r` inclusive, padrão AST ou regras YAML, `query -Q`, `read --grep`, `locale --set`, ops de `edit --multi`) sai 65 `INVALID_INPUT`, 255 `INTERNAL_ERROR` é falha interna de verdade e NUNCA deve ser tratado como correção de entrada
### PROIBIDO
- NUNCA parseie stderr para erro. NUNCA retente se `retryable` false
- NUNCA trate linha com `ok` false como sucesso. NUNCA chute entre candidatos ambíguos


## Fórmulas de execução obrigatórias
### OBRIGATÓRIO
- A LLM DEVE executar locking otimista: `CS=$(atomwrite --workspace "$WS" read arq | jaq -r .checksum) && echo novo | atomwrite --workspace "$WS" write --expect-checksum "$CS" --durability auto arq`
- A LLM DEVE executar edit com match_count: `atomwrite --workspace "$WS" edit src/f.rs --old x --new y --replace-all --fuzzy auto | jaq '{match_count,indent_adjusted,checksum_after}'`
- A LLM DEVE executar fuzzy replace: `atomwrite --workspace "$WS" replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run old_api new_api src/`
- A LLM DEVE recuperar exit 124 aumentando `--timeout-secs` ou estreitando o path
- A LLM DEVE ler `jaq .best_candidate` após exit 65
- A LLM DEVE executar recipe: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- A LLM DEVE executar sparse: `atomwrite --workspace "$WS" sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- A LLM DEVE executar merge line-based: `atomwrite --workspace "$WS" semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- A LLM DEVE executar semantic-search offline: `atomwrite --workspace "$WS" semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- A LLM DEVE executar host readiness: `atomwrite doctor --strict && atomwrite commands --include-globals`
- A LLM DEVE tratar exit 143 como cancel limpo e NUNCA reutilizar tempfile
- A LLM DEVE executar edição com texto cru: `atomwrite --workspace "$WS" apply --no-backup path <<'EOF'` com blocos SEARCH/REPLACE, nunca JSON montado à mão nem gerado por `python`
- A LLM DEVE executar staging scratch: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch; SCR="$WS/.atomwrite/scratch"; printf %s "$OLD" >"$SCR/old.txt"; printf %s "$NEW" >"$SCR/new.txt"; atomwrite --workspace "$WS" edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" path`
- A LLM DEVE executar rewrite shrink: `atomwrite --workspace "$WS" write --durability auto --no-backup --allow-shrink "$path" < "$novo"`
- A LLM DEVE executar shape agent-native: `atomwrite --workspace "$WS" --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 edit --old x --new y path`
- A LLM DEVE executar extract field: `printf '{"a":1}\n' | atomwrite extract --field a`
- A LLM DEVE executar delete plan: `atomwrite --workspace "$WS" delete --plan -r tmp/`
- A LLM DEVE executar verify: `atomwrite --workspace "$WS" verify path EXPECTED_HASH`
- A LLM DEVE executar prune: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run src/lib.rs`, nomeando ARQUIVOS porque um diretório sai 65
- A LLM DEVE executar edit-loop: `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace "$WS" edit-loop src/foo.rs`
- A LLM DEVE executar completions: `atomwrite completions bash --install`
- A LLM DEVE executar codemod dry-run: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- A LLM DEVE executar agent-surface SSOT: `atomwrite agent-surface --format json`
- A LLM DEVE executar watch com teto: `atomwrite --workspace "$WS" watch src/ --max-events 100 --idle-exit-ms 500 --checksum`
- A LLM DEVE tratar exit 78 de `watch` como binário sem a feature e reinstalar com `cargo install atomwrite --features watch`
- A LLM DEVE executar wal: `atomwrite --workspace "$WS" wal-stats` e `atomwrite --workspace "$WS" wal-heal . --threshold-secs 3600`
- A LLM DEVE executar batch transaction: `echo '{"op":"write","target":"a.txt","content":"x"}' | atomwrite --workspace "$WS" batch --transaction`
- A LLM DEVE executar config: `atomwrite --workspace "$WS" config list` e `atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace`
- A LLM DEVE executar descoberta de schema: `atomwrite --print-schema` lista os 55 nomes e `atomwrite --print-schema read` emite um
- A LLM DEVE executar shape de stream: `atomwrite --workspace "$WS" --sort-by path-line --aggregate type:count list src/ --depth 1`
### PROIBIDO
- NUNCA pule dry-run em mutação em massa. NUNCA ignore `best_candidate` em exit 65. NUNCA rode watch sem teto
