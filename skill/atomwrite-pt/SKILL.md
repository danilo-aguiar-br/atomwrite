---
name: atomwrite
description: >-
  Esta skill DEVE ativar quando a LLM precisar de escrita atômica, leitura, edição, busca, substituição, refatoração AST, scoping gramatical, hash BLAKE3, lotes transacionais, fuzzy Jaro-Winkler, cálculo, regex, backup, rollback, sparse, recipe, semantic-merge, agent-surface anti-MCP, watch, codemod, semantic-search Jaccard offline, durability, hardlink backup, rename atômico, progress, best_candidate, cancel SIGTERM ou stat. Cobre os 41 subcomandos (read write edit search replace hash verify delete count diff move copy list extract calc regex transform scope batch backup rollback apply set get del case query outline wal-stats wal-heal edit-loop prune-backups completions recipe sparse semantic-merge agent-surface watch codemod semantic-search stat). Saída SEMPRE NDJSON. Pipeline atômico tempfile-fsync-rename. Triggers — atomwrite, escrita atômica, BLAKE3, locking otimista, fuzzy replace, sparse, codemod
---


# atomwrite


## Identidade
- stdout SEMPRE emite NDJSON (um objeto JSON por linha)
- stderr serve APENAS para logs e tracing
- Toda escrita passa pelo pipeline atômico tempfile, fsync, rename
- Checksum BLAKE3 está presente em TODA resposta de `write` e `read`
- SEMPRE passar `--workspace <DIR>` em operações de arquivo
- Todos os caminhos resolvem relativos ao workspace
- Flag `--json` é aceita mas ignorada (saída SEMPRE NDJSON)
- NDJSON DEVE ser parseado de stdout com `jaq` (NUNCA `jq`)
- NUNCA parsear stderr como dados estruturados
- NUNCA escrever arquivos fora do jail do workspace
- NUNCA assumir que exit 1 é erro de sistema em `search`, `replace`, `transform`, `scope` ou `semantic-search` com zero matches
- Backup default é `true` nos subcomandos mutantes
- Flags de backup são `--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`
- `--backup` e `--no-backup` são mutuamente exclusivos (exit 2 se ambos)
- O `--backup` padrão é transacional — cria `.bak.<timestamp>` adjacente e AUTO-REMOVE no sucesso
- Em falha o `.bak` permanece para rollback
- EXCEÇÃO — `delete` e `replace` PRESERVAM o `.bak` no sucesso
- EXCEÇÃO — `rollback` mantém snapshot pré-rollback OPT-IN via `--backup` explícito
- EXCEÇÃO — `move` e `copy` exigem `--force` OU `--backup` explícito OU env `ATOMWRITE_BACKUP` para sobrescrever destino
- Backup prefere hardlink no mesmo filesystem e cai para cópia quando necessário
- Cancelamento com SIGTERM limpa tempfile e retorna exit 143
- Quando o envelope NDJSON incluir `platform.durability` ou `platform.rename_method`, DEVE ler esses campos
- Precedência de backup resolvido — env `ATOMWRITE_BACKUP` (valor exato `0` desliga) > flags CLI > `.atomwrite.toml` `[defaults]` > default embutido
- Arquivo `.atomwrite.toml` define defaults por projeto


## Hierarquia de Edição
- OBRIGATÓRIO preferir o nível mais preciso que resolve a mudança
- Ordem OBRIGATÓRIA do mais preciso ao mais bruto — search/ssr → transform → scope → replace → edit → write
- DEVE usar `search` para localizar e inspecionar antes de mutar
- DEVE usar `transform` para reescrita estrutural por AST quando o padrão é sintático
- DEVE usar `scope` para ações gramaticais (comments, strings, fn, imports)
- DEVE usar `replace` para substituição textual multiarquivo
- DEVE usar `edit` para edição cirúrgica em um arquivo (pares, linhas, marcadores)
- DEVE usar `write` SOMENTE quando o conteúdo inteiro precisa ser recriado
- DEVE usar `--dry-run` antes de mutação destrutiva em massa
- DEVE capturar checksum com `read` e aplicar `--expect-checksum` em sobrescrita concorrente
- NUNCA pular a hierarquia para reescrever arquivo inteiro sem necessidade


## Flags Globais
- `--workspace <DIR>` — raiz do jail (OBRIGATÓRIO em ops de arquivo)
- `--max-filesize <BYTES>` — tamanho máximo aceito (padrão 1 GiB)
- `-j` / `--threads <N>` — threads paralelos (0 = todos os cores)
- `--timeout-secs <SECONDS>` — timeout global (0 = sem timeout)
- `--color auto|always|never`
- `--no-color` — desabilitar cores
- `--no-gitignore` — ignorar `.gitignore`
- `--hidden` — incluir arquivos ocultos
- `--follow-symlinks` — seguir links simbólicos
- `--locale <LANG>` — idioma de mensagens (`pt-BR`, `en`)
- `--json-schema` — emitir JSON Schema do subcomando e sair
- `--no-auto-heal` — pular wal-heal automático na inicialização
- `-v` info, `-vv` debug, `-vvv` trace
- `-q` error, `-qq` off
- NUNCA omitir `--workspace` em caminhos de arquivo


## write
- DEVE enviar conteúdo via stdin
- DEVE usar `--workspace` em toda escrita
- DEVE usar `--expect-checksum <BLAKE3>` para locking otimista em arquivo existente
- DEVE usar `--durability full|fast|auto` conforme o risco (padrão `auto`)
- `full` aplica fsync mais forte; `fast` usa `sync_data`; `auto` escolhe full em configs e fast em código
- DEVE usar `--allow-shrink` se a escrita reduzir o arquivo em mais de 50% com `--expect-checksum`
- DEVE usar `--allow-empty-stdin` para stdin vazio intencional
- DEVE usar `--no-checksum-when-empty` para pular checksum quando stdin vazio
- DEVE usar `--dry-run` antes de sobrescrita destrutiva
- DEVE usar `--syntax-check` para validar tree-sitter antes de gravar (exit 88 em falha)
- Flags — `--append`, `--prepend`, `--max-size`, `--line-ending lf|crlf|cr|auto`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--confirm`, `--risk-threshold`, `--wal-policy auto|always|never`, opts de backup
- SIGTERM durante write limpa tempfile e retorna exit 143
- NUNCA passar conteúdo como argumento CLI
- NUNCA escrever sem `--workspace`
- FÓRMULA `echo "conteúdo" | atomwrite --workspace . write --durability full alvo.rs`
- FÓRMULA locking `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto arq`


## read e stat
- DEVE usar `read` para conteúdo com metadados (checksum, size, lines, mode)
- DEVE usar `stat PATH` como alias de `read --stat` para metadados sem corpo
- Flags de leitura — `--lines 1:50`, `--line N`, `-C/--context N`, `--head N`, `--tail N`, `--format raw|ndjson`, `--grep <REGEX>`, `--verify-checksum <BLAKE3>`
- Campo `mode` indica variante — full, head, tail, line, lines, grep, stat
- Exit 81 em mismatch de `--verify-checksum`
- NUNCA confiar em conteúdo sem checar envelope e exit code
- FÓRMULA `atomwrite --workspace . read --head 20 src/main.rs`
- FÓRMULA `atomwrite --workspace . stat src/main.rs`


## edit
- DEVE usar `--old "texto" --new "texto"` para pares exatos (repetível)
- Multi-par roda cascata fuzzy de 9 estratégias incluindo Jaro-Winkler
- Resposta inclui `pairs_total` e `pair_results` (index, matched, strategy, similarity, diff_preview)
- Par falho aborta o lote inteiro por padrão (all-or-nothing)
- DEVE usar `--partial` para aplicar pares que casam e relatar os demais
- DEVE usar `--fuzzy auto|off|aggressive` e `--fuzzy-threshold <FLOAT>`
- Flags de posição — `--after-line N`, `--before-line N`, `--range N:M`, `--delete-range N:M`, `--after-match`, `--before-match`, `--between`, `--multi`
- Modos de stdin rejeitam `--old`/`--new`/`--old-file`/`--new-file` no parse (exit 2)
- Stdin de terminal nesses modos falha com exit 65 (NÃO trava)
- DEVE usar `--old-file` e `--new-file` para payloads grandes
- DEVE usar `--expect-checksum` em edição concorrente
- DEVE usar `--allow-sequential-drift` só em pipeline sequencial controlado
- NUNCA pipar edit para `jaq` sem checar `${PIPESTATUS[0]}`
- FÓRMULA `atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo"`
- FÓRMULA `echo "linha" | atomwrite --workspace . edit src/main.rs --after-line 10`


## search
- Exit 1 significa zero resultados (NÃO é erro de sistema)
- DEVE usar search para localizar alvos antes de mutar
- Flags — `-g/--include`, `--exclude`, `-C/--context`, `-F/--fixed`, `-e/--regex`, `-w/--word`, `-i`, `-S/--smart-case`, `-c/--count`, `-l/--files`, `-m/--max-count`, `-U/--multiline`, `-P/--pcre2`, `--invert`, `--sort path|modified|created|none`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Default de `--max-filesize` no search é 10 MiB (diferente do global 1 GiB)
- NUNCA tratar exit 1 de zero matches como falha fatal
- FÓRMULA `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs'`


## replace
- Exit 1 significa zero matches (NÃO é erro de sistema)
- DEVE SEMPRE rodar `--dry-run` ou `--preview` antes de aplicar
- DEVE usar `--fuzzy auto|off|aggressive` (padrão `auto`; ignorado com `--regex`)
- DEVE usar `--fuzzy-threshold <FLOAT>` para similaridade (0.0 a 1.0)
- DEVE usar `--progress-every <N>` para progresso NDJSON a cada N arquivos (padrão 50; 0 desliga)
- Em falha de match (exit 65) DEVE ler `best_candidate` do envelope em vez de reler arquivos
- Flags — `--regex`, `-w`, `-F`, `-g/--include`, `--exclude`, `-n/--max-replacements`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case`, opts de backup
- `replace` PRESERVA `.bak` no sucesso
- NUNCA aplicar replace em massa sem dry-run
- FÓRMULA `atomwrite --workspace . replace --dry-run --fuzzy auto --progress-every 20 'antigo' 'novo' src/`
- FÓRMULA `atomwrite --workspace . replace --fuzzy aggressive --fuzzy-threshold 0.8 'API' 'Api' src/ --include '*.rs'`


## transform
- Exit 1 significa zero matches
- DEVE especificar `-l/--language` (ou alias de linguagem do subcomando)
- DEVE usar `$NAME` para um nó e `$$$ARGS` para múltiplos
- DEVE usar `-p/--pattern` e `-r/--rewrite` juntos no modo single-rule
- Flags — `--rules <PATH>`, `--inline-rules <JSON>`, `--verify-parse`, `--include`, `--exclude`, `--dry-run`, opts de backup
- DEVE preferir transform sobre replace quando o padrão é sintático
- NUNCA reescrever sem dry-run em codebase grande
- FÓRMULA `atomwrite --workspace . transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## scope
- Exit 1 significa zero matches
- DEVE especificar linguagem com `--language` (alias `--lang`)
- DEVE usar `--query` para categorias preparadas ou `--pattern` para AST customizado
- DEVE usar `--delete` para remover conteúdo casado
- DEVE usar `--action upper|lower|titlecase|squeeze|symbols|normalize`
- DEVE usar `--replace-with "texto"` para substituição customizada
- Queries Rust úteis — comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, attribute, derive, return, match
- Queries Python — comments, strings, class, def, async-def, import, from-import, decorator
- Queries JS/TS — comments, strings, fn, arrow-fn, class, import, export
- NUNCA aplicar `--delete` sem dry-run
- FÓRMULA `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`


## batch
- Entrada NDJSON via stdin com campo `op` obrigatório — write, replace, delete, edit, move, copy, hash
- move/copy no manifesto exigem `"force":true` para sobrescrever
- DEVE usar `--transaction` para atomicidade total com rollback automático
- DEVE usar `--file <PATH>` para manifesto em disco
- Flags — `--dry-run`, `--batch-size <N>`, `--input-schema`, opts de backup e `--retention`
- NUNCA rodar batch mutante sem dry-run em produção
- FÓRMULA `echo '{"op":"write","target":"a.txt","content":"olá"}' | atomwrite --workspace . batch --transaction`


## sparse recipe semantic-merge agent-surface watch codemod semantic-search
- DEVE usar `sparse list` com `--max-files` e `--max-bytes` para orçamento em monorepo
- DEVE usar `sparse list --include` e `--exclude` para filtrar
- DEVE usar `sparse read --paths-file <FILE> --head N --max-files N` para leitura orçada
- DEVE usar `recipe list` para listar receitas embutidas
- DEVE usar `recipe run --name search-replace-verify --pattern OLD --replacement NEW --path . --dry-run --fuzzy auto`
- Receitas embutidas — `search-replace-verify` e `edit-loop-syntax-check`
- `recipe run` aceita `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- DEVE usar `semantic-merge --base A --ours B --theirs C --output OUT` para conflito multi-agente
- Flags de merge — `--fail-on-conflict`, `--write-conflict-markers`, `--expect-checksum`, opts de backup
- DEVE usar `agent-surface --format json` para manifesto de tools da CLI
- MCP é PROIBIDO como superfície de escrita deste projeto — DEVE usar a CLI atomwrite
- DEVE usar `watch [PATH] --debounce-ms N --max-events N --checksum --gitignore true|false`
- DEVE encapsular `watch` com limite de eventos ou sinal externo (evita hang infinito)
- DEVE usar `codemod --rules rules.yaml --dry-run` antes de aplicar campanha multi-regra
- DEVE usar `semantic-search "query" PATH --k 20 --min-score 0.05`
- `semantic-search` é Jaccard de tokens offline — NUNCA depende de embedding de rede
- DEVE usar `--index-dir .atomwrite/semantic-index` para índice invertido local opcional
- Exit 1 em `semantic-search` com zero resultados NÃO é erro de sistema
- FÓRMULA sparse `atomwrite --workspace . sparse list src/ --max-files 50 --max-bytes 524288`
- FÓRMULA sparse read `atomwrite --workspace . sparse read --paths-file paths.txt --head 30 --max-files 10`
- FÓRMULA recipe `atomwrite --workspace . recipe run --name search-replace-verify --pattern old_api --replacement new_api --path src --dry-run --fuzzy auto`
- FÓRMULA merge `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output out.rs --fail-on-conflict`
- FÓRMULA agent `atomwrite --workspace . agent-surface --format json`
- FÓRMULA watch `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 100 --checksum`
- FÓRMULA codemod `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- FÓRMULA semantic-search `atomwrite --workspace . semantic-search "optimistic lock checksum" src/ --k 15 --min-score 0.1`


## hash e verify
- DEVE usar `hash` para checksums BLAKE3 de um ou mais arquivos
- Campo de saída é `checksum` (NÃO `value`)
- Flags de hash — `--verify <BLAKE3>`, `--stdin`, `-r/--recursive`
- DEVE usar `verify <PATH> <EXPECTED_HASH>` para checagem posicional
- Exit 0 quando casa; exit 81 quando NÃO casa
- FÓRMULA `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FÓRMULA `atomwrite --workspace . verify src/main.rs abc123def456`


## delete
- Backup fica LIGADO por padrão e o `.bak` de deleção é PRESERVADO no sucesso
- DEVE usar `--no-backup` apenas quando a remoção for descartável
- DEVE usar `-y/--yes` para pular confirmação em automação
- Flags — `-r/--recursive`, `--include`, `--exclude`, `--dry-run`, `--confirm`, `--older-than <DURATION>`, `--retention`
- `--keep-backup` em delete é redundante e emite `warnings`
- NUNCA deletar sem dry-run em caminhos amplos
- FÓRMULA `atomwrite --workspace . delete --older-than 7d --yes tmp/`


## diff
- DEVE usar `diff` para comparar dois arquivos
- Flags — `--unified`, `--stat`, `-C/--context N`, `--algorithm myers|patience|lcs`
- FÓRMULA `atomwrite --workspace . diff src/old.rs src/new.rs --unified`


## move e copy
- DEVE usar `--force` OU `--backup` explícito OU env `ATOMWRITE_BACKUP` para sobrescrever destino
- `[defaults] backup = true` do TOML sozinho NÃO autoriza sobrescrita
- copy aceita `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`
- move aceita `--preserve-hardlinks` e `--retention`
- NUNCA sobrescrever destino sem flag explícita de autorização
- FÓRMULA `atomwrite --workspace . move src/old.rs src/new.rs`
- FÓRMULA `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`


## list count extract
- list — `-g/--include`, `--exclude`, `--long`, `--depth N`, `--count-by-ext`, `--all`
- count — `--by-extension`, `--by-size` com `--top N`, `--include`, `--exclude`
- extract — campos posicionais (`path`, `line_number`), `--delimiter`, `--stdin`
- FÓRMULA `atomwrite --workspace . list --long --depth 2 src/`
- FÓRMULA `atomwrite --workspace . count --by-size --top 10 src/`
- FÓRMULA `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`


## calc e regex
- calc avalia expressão e conversão de unidades (stateless; `--workspace` desnecessário)
- DEVE usar `calc` com expressão entre aspas ou `--stdin`
- regex gera padrão a partir de exemplos (forneça 3+ exemplos)
- Flags regex — `-d/--digits`, `-w/--words`, `-s/--spaces`, `-r/--repetitions`, `-i`, `--no-anchors`, `--stdin`
- FÓRMULA `atomwrite calc "2 hours + 30 minutes to seconds"`
- FÓRMULA `atomwrite regex "abc-12" "xyz-99" "id-7" --digits`


## backup e rollback
- DEVE usar `backup` para snapshots timestampados com BLAKE3
- Flags de backup — `--retention N`, `--output-dir`, `--dry-run`
- Backup prefere hardlink no mesmo FS
- DEVE usar `rollback --latest --verify` para restaurar e validar
- Flags de rollback — `--timestamp YYYYMMDD_HHMMSS`, `--latest`, `--verify`, `--dry-run`, `--backup` OPT-IN
- NUNCA rollback sem `--verify` em arquivo crítico
- FÓRMULA `atomwrite --workspace . backup src/main.rs --retention 3`
- FÓRMULA `atomwrite --workspace . rollback src/config.toml --latest --verify`


## apply
- Detecta formato — unified, SEARCH/REPLACE, markdown-fenced, full file
- DEVE usar `--format auto|unified|search-replace|full|markdown` quando o auto-detect for ambíguo
- Conteúdo do patch entra via stdin
- FÓRMULA `echo "conteúdo" | atomwrite --workspace . apply src/file.txt --format full`


## set get del
- DEVE usar apenas em TOML ou JSON (NUNCA em texto puro)
- set escreve valor via dotted path com auto-coerce
- get lê valor via dotted path (exit 65 se chave ausente)
- del remove chave; `--force-missing` sucede se a chave não existir
- Flags — opts de backup e `--preserve-timestamps` em set/del
- FÓRMULA `atomwrite --workspace . set Cargo.toml package.name meu-crate`
- FÓRMULA `atomwrite --workspace . get config.toml database.pool.max`
- FÓRMULA `atomwrite --workspace . del --force-missing config.toml features.experimental`


## case query outline
- case converte identificadores — snake, camel, pascal, kebab, screaming-snake
- DEVE passar `--subvert OLD NEW` (sem isso exit 65)
- DEVE rodar case com `--dry-run` em codebase grande
- query inspeciona AST tree-sitter — `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- outline extrai estrutura — `--kind` (repetível), `--positions`, `--language`
- FÓRMULA `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`
- FÓRMULA `atomwrite --workspace . query --kinds src/main.rs`
- FÓRMULA `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## wal edit-loop prune completions
- `wal-stats` inspeciona journals (consultivo; NÃO modifica)
- `wal-heal` remove journals terminais órfãos — `--threshold-secs` (padrão 3600), `--max-duration-ms` (padrão 100)
- `edit-loop` aplica N pares `{old,new}` em uma escrita (JSON array ou NDJSON)
- edit-loop aceita `--line-ending`, `--syntax-check`, `--allow-sequential-drift`, opts de backup
- `prune-backups` limpa legados — `--max-age-secs` (NÃO `--max-age`), `--max-count`, `--dry-run`
- `completions` gera shell completions — `bash|zsh|fish|elvish|powershell` com `--install`
- FÓRMULA `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FÓRMULA `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`
- FÓRMULA `atomwrite completions bash --install`


## Erros
- DEVE verificar exit code ANTES de parsear stdout
- DEVE parsear envelope JSON quando `error` for true
- Campos — error, code, exit, message, path, error_class, retryable, suggestion, workspace
- Em exit 65 de replace/edit DEVE ler `best_candidate` quando presente
- Estratégia por `error_class` — permanent (NUNCA retentar), transient (backoff), conflict (relê estado), precondition_failed (corrige pré-condição)
- DEVE retentar SOMENTE quando `retryable` for true
- DEVE usar `suggestion` para remediação
- DEVE tratar exit 82 como state drift — recarregar checksum e reaplicar
- DEVE tratar exit 143 como cancel limpo (tempfile removido)
- NUNCA ignorar exit não-zero exceto zero matches documentado
- NUNCA parsear stderr para dados de erro


## Exit codes
- 0 sucesso
- 1 zero matches em search/replace/transform/scope/semantic-search (NÃO é erro de sistema)
- 2 argumento inválido / conflito de flags
- 4 não encontrado
- 13 permissão negada
- 28 disco cheio
- 30 cota excedida
- 65 entrada inválida (pattern vazio, chave ausente, best_candidate em match falho)
- 73 cross-device
- 74 erro de I/O
- 78 configuração inválida
- 81 checksum mismatch (verify / read --verify-checksum)
- 82 state drift (locking otimista)
- 83 timeout de lock
- 85 FIFO detectado
- 86 arquivo de dispositivo
- 88 erro de sintaxe tree-sitter
- 91 fallback EXDEV desabilitado
- 92 verificação BLAKE3 de copy-back falhou
- 93 journal órfão (consultivo)
- 126 violação do jail do workspace
- 127 symlink bloqueado
- 128 arquivo imutável
- 130 SIGINT
- 141 SIGPIPE
- 143 SIGTERM (cancel limpo)
- 255 erro interno


## Fórmulas prontas
- Locking otimista — `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto arq`
- Durability forte — `echo "cfg" | atomwrite --workspace . write --durability full config.toml`
- Fuzzy replace com progresso — `atomwrite --workspace . replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run 'old_api' 'new_api' src/`
- Best candidate — leia envelope exit 65 com `jaq '.best_candidate'` após replace/edit falho
- Recipe search-replace-verify — `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- Sparse monorepo — `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- Sparse read — `atomwrite --workspace . sparse read --paths-file paths.txt --head 40 --max-files 15`
- Semantic merge — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict --expect-checksum "$CS"`
- Semantic search offline — `atomwrite --workspace . semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- Codemod dry-run — `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- Agent surface — `atomwrite --workspace . agent-surface --format json`
- Batch transaction — `atomwrite --workspace . batch --file ops.ndjson --transaction --dry-run`
- Transform dry-run — `atomwrite --workspace . transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- Rollback verify — `atomwrite --workspace . rollback src/config.toml --latest --verify`
- Cancel awareness — em exit 143 após SIGTERM trate como cancel limpo e NÃO reutilize tempfile
- Hash e verify — `H=$(atomwrite --workspace . hash src/main.rs | jaq -r '.checksum') && atomwrite --workspace . verify src/main.rs "$H"`
- Watch limitado — `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 50 --checksum --gitignore true`
- Stat rápido — `atomwrite --workspace . stat src/main.rs`
- Edit multi-par — `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --old "c" --new "d"`
- Scope comments — `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`
