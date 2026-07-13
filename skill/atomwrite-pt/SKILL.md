---
name: atomwrite
description: >-
  Esta skill DEVE ativar quando a LLM precisar de escrita atômica, leitura, edição, busca, substituição, transformação AST, scope gramatical, hash BLAKE3, verify, delete, count, diff, move, copy, list, extract, calc, regex, batch, backup, rollback, apply, set, get, del, case, query, outline, wal-stats, wal-heal, edit-loop, prune-backups, completions, recipe, sparse, semantic-merge, agent-surface, watch, codemod, semantic-search ou stat. Cobre os 41 subcomandos com fuzzy obrigatório auto|aggressive, match_count, indent_adjusted, replace-all, best_candidate, SIGTERM 143, durability, reflink_or_copy, recipe sem bak, sparse outline AST real, semantic-merge line-based, agent-surface anti-MCP e semantic-search Jaccard offline. Saída SEMPRE NDJSON via tempfile-fsync-rename. Triggers — atomwrite, escrita atômica, edit cirúrgico, checksum, locking otimista, tree-sitter, ast-grep, fuzzy replace, sparse, recipe, codemod, semantic-search
---


# atomwrite


## Contrato operacional
- atomwrite é a ÚNICA ferramenta de escrita e edição de arquivos deste projeto
- stdout SEMPRE emite NDJSON (um objeto JSON por linha)
- stderr serve APENAS para logs e tracing
- Toda escrita usa pipeline atômico tempfile → fsync → rename
- Checksum BLAKE3 está presente em TODA resposta bem-sucedida de `write` e `read`
- SEMPRE passe `--workspace <DIR>` em operações de arquivo
- Todos os caminhos resolvem relativos ao workspace (jail)
- Flag `--json` é aceita e IGNORADA (saída SEMPRE NDJSON)
- SEMPRE parseie NDJSON de stdout com `jaq` (NUNCA `jq`)
- NUNCA parseie stderr como dados estruturados
- NUNCA escreva fora do jail do workspace
- NUNCA invente flags ausentes de `atomwrite <cmd> --help`
- Exit 1 em `search`, `replace`, `transform`, `scope` e `semantic-search` significa ZERO matches e NÃO é erro de sistema
- SEMPRE verifique o exit code ANTES de parsear stdout
- Em exit 65 de match falho SEMPRE leia `best_candidate` no envelope
- Em exit 82 SEMPRE recarregue checksum e reaplique (state drift)
- Em exit 143 trate como cancel limpo por SIGTERM (tempfile removido)
- MCP é PROIBIDO como superfície de escrita — DEVE usar a CLI atomwrite


## Fluxo de execução OBRIGATÓRIO
- DEVE localizar o alvo com `search` ou `sparse` antes de mutar
- DEVE capturar checksum com `read` quando houver concorrência
- DEVE escolher o nível mais preciso da hierarquia de edição
- DEVE rodar `--dry-run` antes de mutação destrutiva em massa
- DEVE aplicar a mutação com `--expect-checksum` em sobrescrita concorrente
- DEVE validar o envelope NDJSON (checksum, match_count, indent_adjusted, platform)
- DEVE tratar falhas pelo `error_class` e pelo campo `suggestion`
- NUNCA pule dry-run em replace/transform/scope/codemod em codebase grande
- NUNCA reescreva arquivo inteiro com `write` se `edit`/`replace`/`transform` resolve


## Hierarquia de edição
- Ordem OBRIGATÓRIA do mais preciso ao mais bruto — search → transform → scope → replace → edit → write
- DEVE usar `search` para localizar e inspecionar
- DEVE usar `transform` para reescrita estrutural por AST quando o padrão é sintático
- DEVE usar `scope` para ações gramaticais (comments, strings, fn, imports)
- DEVE usar `replace` para substituição textual multiarquivo
- DEVE usar `edit` para edição cirúrgica em um arquivo
- DEVE usar `write` SOMENTE para criar arquivo ou sobrescrita total intencional


## Flags globais
- `--workspace <DIR>` — raiz do jail (OBRIGATÓRIO em ops de arquivo)
- `--max-filesize <BYTES>` — teto global (padrão 1 GiB)
- `-j` / `--threads <N>` — paralelismo (0 = todos os cores)
- `--timeout-secs <SECONDS>` — timeout global (0 = sem timeout)
- `--color auto|always|never` e `--no-color`
- `--no-gitignore`, `--hidden`, `--follow-symlinks`
- `--locale <LANG>` — idioma de mensagens (`pt-BR`, `en`)
- `--json-schema` — emite schema do subcomando e sai
- `--no-auto-heal` — pula wal-heal na inicialização
- `-v` info, `-vv` debug, `-vvv` trace
- `-q` error, `-qq` off
- NUNCA use flag global `--lang` para locale — use `--locale`


## Backup e plataforma
- Backup default é `true` nos mutadores via `--backup` / `--no-backup` / `--keep-backup` / `--retention <N>`
- `--backup` e `--no-backup` são mutuamente exclusivos (exit 2)
- Backup padrão é transacional — cria `.bak.<timestamp>` adjacente e AUTO-REMOVE no sucesso
- Em falha o `.bak` permanece para rollback
- EXCEÇÃO — `delete` e `replace` PRESERVAM o `.bak` no sucesso
- EXCEÇÃO — `rollback` usa backup OPT-IN com `--backup` explícito
- EXCEÇÃO — `move` e `copy` exigem `--force` OU `--backup` explícito OU env `ATOMWRITE_BACKUP` para sobrescrever destino
- `[defaults] backup = true` no TOML sozinho NÃO autoriza sobrescrita em move/copy
- Backup usa `reflink_or_copy` (NUNCA hardlink do arquivo vivo)
- SEMPRE leia `platform.backup_method`, `platform.durability` e `platform.rename_method` quando o envelope os emitir
- Precedência de backup — env `ATOMWRITE_BACKUP` (valor exato `0` desliga) > flags CLI > `.atomwrite.toml` `[defaults]` > default embutido


## Configuração do projeto
- Arquivo `.atomwrite.toml` define defaults por projeto
- Seção `[defaults]` governa `backup` e `retention`
- Seção `[fuzzy]` governa `mode` e `threshold` em edit, replace, edit-loop, batch e recipe
- `mode` VÁLIDO é SOMENTE `auto` ou `aggressive`
- NUNCA configure `mode = "off"` — a CLI rejeita com exit 65
- `threshold` DEVE estar entre 0.0 e 1.0
- Flags CLI vencem o TOML quando explicitadas
- FÓRMULA config fuzzy — em `.atomwrite.toml` defina `[fuzzy]` com `mode = "auto"` e `threshold = 0.85`


## Fuzzy obrigatório
- Fuzzy é OBRIGATÓRIO em edit e replace (modo padrão `auto`)
- Valores permitidos — `--fuzzy auto` e `--fuzzy aggressive`
- NUNCA passe `--fuzzy off` — rejeitado com exit 65
- DEVE usar `--fuzzy-threshold <FLOAT>` para ajustar similaridade (0.0 a 1.0)
- Com `--regex` no replace o fuzzy é IGNORADO
- Em falha de match SEMPRE leia `best_candidate` no envelope exit 65


## write
- DEVE enviar conteúdo via stdin (NUNCA como argumento CLI)
- DEVE usar `--workspace` em toda escrita
- DEVE usar `--expect-checksum <BLAKE3>` para locking otimista em arquivo existente
- DEVE usar `--durability full|fast|auto` (padrão `auto`)
- `full` aplica fsync mais forte; `fast` usa `sync_data`; `auto` escolhe full em configs e fast em código
- DEVE usar `--allow-shrink` se a escrita encolher mais de 50% com `--expect-checksum`
- DEVE usar `--allow-empty-stdin` para stdin vazio intencional
- DEVE usar `--no-checksum-when-empty` para pular checksum com stdin vazio
- DEVE usar `--dry-run` antes de sobrescrita destrutiva
- DEVE usar `--syntax-check` para validar tree-sitter (exit 88 em falha)
- Flags — `--append`, `--prepend`, `--max-size`, `--line-ending lf|crlf|cr|auto`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--confirm`, `--risk-threshold`, `--wal-policy auto|always|never`, opts de backup
- SIGTERM durante write limpa tempfile e retorna exit 143
- FÓRMULA `echo "conteúdo" | atomwrite --workspace . write --durability full alvo.rs`
- FÓRMULA locking `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto arq`


## read e stat
- DEVE usar `read` para conteúdo com metadados (checksum, size, lines, mode)
- DEVE usar `stat PATH` como alias de `read --stat` para metadados sem corpo
- Flags — `--lines 1:50`, `--line N`, `-C/--context N`, `--head N`, `--tail N`, `--format raw|ndjson`, `--grep <REGEX>`, `--verify-checksum <BLAKE3>`
- Campo `mode` indica variante — full, head, tail, line, lines, grep, stat
- Exit 81 em mismatch de `--verify-checksum`
- FÓRMULA `atomwrite --workspace . read --head 20 src/main.rs`
- FÓRMULA `atomwrite --workspace . stat src/main.rs`


## edit
- DEVE usar `--old "texto" --new "texto"` para pares exatos (repetível)
- Multi-par roda cascata fuzzy de 9 estratégias incluindo Jaro-Winkler
- Resposta inclui `pairs_total` e `pair_results` (index, matched, strategy, similarity, diff_preview)
- Par falho aborta o lote inteiro por padrão (all-or-nothing)
- DEVE usar `--partial` para aplicar pares que casam e relatar os demais
- DEVE usar `--fuzzy auto|aggressive` e `--fuzzy-threshold <FLOAT>`
- Multi-ocorrência EXIGE `--replace-all`; sem ele a unicidade é obrigatória
- Sucesso com `--replace-all` emite `match_count` no NDJSON
- Quando o indent realinha o replacement o NDJSON emite `indent_adjusted: true`
- SEMPRE leia `match_count` e `indent_adjusted` após edit bem-sucedido
- Flags de posição — `--after-line N`, `--before-line N`, `--range N:M`, `--delete-range N:M`, `--after-match`, `--before-match`, `--between`, `--multi`
- Modos de stdin rejeitam `--old`/`--new`/`--old-file`/`--new-file` no parse (exit 2)
- Stdin de terminal nesses modos falha com exit 65 (NÃO trava)
- DEVE usar `--old-file` e `--new-file` para payloads grandes
- DEVE usar `--expect-checksum` em edição concorrente
- DEVE usar `--allow-sequential-drift` só em pipeline sequencial controlado
- Flags extras — `--line-ending`, `--preserve-timestamps`, `--dry-run`, `--wal-policy`, opts de backup
- NUNCA pipe edit para `jaq` sem checar `${PIPESTATUS[0]}`
- FÓRMULA par `atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo" --fuzzy auto`
- FÓRMULA multi `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --replace-all | jaq '{match_count, indent_adjusted, checksum}'`
- FÓRMULA insert `echo "linha" | atomwrite --workspace . edit src/main.rs --after-line 10`


## search
- Exit 1 significa zero resultados (NÃO é erro de sistema)
- DEVE usar search para localizar alvos antes de mutar
- Flags — `-g/--include`, `--exclude`, `-C/--context`, `-F/--fixed`, `-e/--regex`, `-w/--word`, `-i`, `-S/--smart-case`, `-c/--count`, `-l/--files`, `-m/--max-count`, `-U/--multiline`, `-P/--pcre2`, `--invert`, `--sort path|modified|created|none`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Default de `--max-filesize` no search é 10 MiB (diferente do global 1 GiB)
- FÓRMULA `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs' --sort path`


## replace
- Exit 1 significa zero matches (NÃO é erro de sistema)
- DEVE SEMPRE rodar `--dry-run` ou `--preview` antes de aplicar
- DEVE usar `--fuzzy auto|aggressive` (padrão `auto`; ignorado com `--regex`)
- DEVE usar `--fuzzy-threshold <FLOAT>` para similaridade
- DEVE usar `--progress-every <N>` para progresso NDJSON a cada N arquivos (padrão 50; 0 desliga)
- Multi-ocorrência de string fixa EXIGE `--replace-all`
- Em falha de match (exit 65) DEVE ler `best_candidate`
- Flags — `--regex`, `-w`, `-F`, `-g/--include`, `--exclude`, `-n/--max-replacements`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case`, opts de backup
- `replace` PRESERVA `.bak` no sucesso
- FÓRMULA dry-run `atomwrite --workspace . replace --dry-run --fuzzy auto --progress-every 20 'antigo' 'novo' src/`
- FÓRMULA agressivo `atomwrite --workspace . replace --fuzzy aggressive --fuzzy-threshold 0.8 --replace-all 'API' 'Api' src/ --include '*.rs'`


## transform
- Exit 1 significa zero matches
- DEVE especificar `-l/--language`
- DEVE usar `$NAME` para um nó e `$$$ARGS` para múltiplos
- DEVE usar `-p/--pattern` e `-r/--rewrite` juntos no modo single-rule
- Flags — `--rules <PATH>`, `--inline-rules <JSON>`, `--verify-parse`, `--include`, `--exclude`, `--dry-run`, opts de backup
- DEVE preferir transform sobre replace quando o padrão é sintático
- FÓRMULA `atomwrite --workspace . transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## scope
- Exit 1 significa zero matches
- DEVE especificar linguagem com `--language` (alias `--lang`)
- DEVE usar `--query` para categorias preparadas ou `--pattern` para AST customizado
- DEVE usar `--delete` para remover conteúdo casado
- DEVE usar `--action upper|lower|titlecase|squeeze|symbols|normalize`
- DEVE usar `--replace-with "texto"` para substituição customizada
- Queries úteis — comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- NUNCA aplique `--delete` sem dry-run
- FÓRMULA `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`


## batch
- Entrada NDJSON via stdin com campo `op` obrigatório — write, replace, delete, edit, move, copy, hash
- move/copy no manifesto exigem `"force":true` para sobrescrever
- DEVE usar `--transaction` para atomicidade total com rollback automático
- DEVE usar `--file <PATH>` para manifesto em disco
- Flags — `--dry-run`, `--batch-size <N>`, `--input-schema`, opts de backup e `--retention`
- FÓRMULA `echo '{"op":"write","target":"a.txt","content":"olá"}' | atomwrite --workspace . batch --transaction`


## sparse recipe semantic-merge agent-surface watch codemod semantic-search
- DEVE usar `sparse list` com `--max-files` e `--max-bytes` para orçamento em monorepo
- DEVE usar `sparse list --include` e `--exclude` para filtrar
- DEVE usar `sparse read --paths-file <FILE> --head N --max-files N` para leitura orçada
- DEVE usar `sparse outline` para outline AST real sob orçamento (emite `outline_item` com `kind` real)
- DEVE usar `recipe list` para listar receitas embutidas
- DEVE usar `recipe run --name search-replace-verify --pattern OLD --replacement NEW --path . --dry-run --fuzzy auto`
- Recipe search/replace/hash EXCLUI `*.bak.*` por padrão
- Receitas embutidas — `search-replace-verify` e `edit-loop-syntax-check`
- `recipe run` aceita `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- DEVE usar `semantic-merge --base A --ours B --theirs C --output OUT` para merge three-way multi-agente
- semantic-merge é line-based (NÃO AST e NÃO embedding)
- Flags de merge — `--fail-on-conflict`, `--write-conflict-markers`, `--expect-checksum`, opts de backup
- DEVE usar `agent-surface --format json` para manifesto de tools da CLI
- DEVE usar `watch [PATH] --debounce-ms N --max-events N --checksum --gitignore true|false`
- DEVE encapsular `watch` com `--max-events` ou sinal externo (evita hang infinito)
- `watch` exige feature `watch` no binário
- DEVE usar `codemod --rules rules.yaml --dry-run` antes de aplicar campanha multi-regra
- DEVE usar `semantic-search "query" PATH --k 20 --min-score 0.05`
- `semantic-search` é Jaccard de tokens offline — NUNCA depende de embedding de rede
- DEVE usar `--index-dir .atomwrite/semantic-index` para índice invertido local opcional
- Exit 1 em `semantic-search` com zero resultados NÃO é erro de sistema
- FÓRMULA sparse list `atomwrite --workspace . sparse list src/ --max-files 50 --max-bytes 524288 --include '*.rs'`
- FÓRMULA sparse read `atomwrite --workspace . sparse read --paths-file paths.txt --head 30 --max-files 10`
- FÓRMULA sparse outline `atomwrite --workspace . sparse outline src/ --max-files 40 --include '*.rs'`
- FÓRMULA recipe `atomwrite --workspace . recipe run --name search-replace-verify --pattern old_api --replacement new_api --path src --dry-run --fuzzy auto`
- FÓRMULA merge `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output out.rs --fail-on-conflict`
- FÓRMULA agent `atomwrite --workspace . agent-surface --format json`
- FÓRMULA watch `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 100 --checksum`
- FÓRMULA codemod `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- FÓRMULA semantic-search `atomwrite --workspace . semantic-search "optimistic lock checksum" src/ --k 15 --min-score 0.1 --index-dir .atomwrite/semantic-index`


## hash e verify
- DEVE usar `hash` para checksums BLAKE3 de um ou mais arquivos
- Campo de saída é `checksum` (NÃO `value`)
- Flags de hash — `--verify <BLAKE3>`, `--stdin`, `-r/--recursive`, `--exclude <GLOB>`
- DEVE usar `verify <PATH> <EXPECTED_HASH>` para checagem posicional
- Exit 0 quando casa; exit 81 quando NÃO casa
- FÓRMULA `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FÓRMULA recursive `atomwrite --workspace . hash -r src/ --exclude '*.bak.*'`
- FÓRMULA `atomwrite --workspace . verify src/main.rs abc123def456`


## delete
- Backup fica LIGADO por padrão e o `.bak` de deleção é PRESERVADO no sucesso
- DEVE usar `--no-backup` apenas quando a remoção for descartável
- DEVE usar `-y/--yes` para pular confirmação em automação
- Flags — `-r/--recursive`, `--include`, `--exclude`, `--dry-run`, `--confirm`, `--older-than <DURATION>`, `--retention`
- `--keep-backup` em delete é redundante e emite `warnings`
- FÓRMULA `atomwrite --workspace . delete --older-than 7d --yes tmp/`


## diff move copy list count extract
- diff — `--unified`, `--stat`, `-C/--context N`, `--algorithm myers|patience|lcs`
- move/copy — sobrescrita exige `--force` OU `--backup` explícito OU env `ATOMWRITE_BACKUP`
- copy — `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`
- move — `--preserve-hardlinks`, `--retention`
- list — `-g/--include`, `--exclude`, `--long`, `--depth N`, `--count-by-ext`, `--all`
- count — `--by-extension`, `--by-size` com `--top N`
- extract — campos posicionais (`path`, `line_number`), `--delimiter`, `--stdin`
- FÓRMULA diff `atomwrite --workspace . diff src/old.rs src/new.rs --unified`
- FÓRMULA move `atomwrite --workspace . move src/old.rs src/new.rs`
- FÓRMULA copy `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`
- FÓRMULA list `atomwrite --workspace . list --long --depth 2 src/`
- FÓRMULA count `atomwrite --workspace . count --by-size --top 10 src/`
- FÓRMULA extract `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`


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
- Backup usa `reflink_or_copy` (NUNCA hardlink do arquivo vivo)
- DEVE usar `rollback --latest --verify` para restaurar e validar
- Flags de rollback — `--timestamp YYYYMMDD_HHMMSS`, `--latest`, `--verify`, `--dry-run`, `--backup` OPT-IN
- NUNCA faça rollback sem `--verify` em arquivo crítico
- FÓRMULA `atomwrite --workspace . backup src/main.rs --retention 3`
- FÓRMULA `atomwrite --workspace . rollback src/config.toml --latest --verify`


## apply set get del
- apply detecta formato — unified, SEARCH/REPLACE, markdown-fenced, full file
- DEVE usar `--format auto|unified|search-replace|full|markdown` quando o auto-detect for ambíguo
- set/get/del operam SOMENTE em TOML ou JSON via dotted path
- NUNCA use set/get/del em texto puro
- get retorna exit 65 se a chave estiver ausente
- del aceita `--force-missing` para suceder se a chave não existir
- FÓRMULA apply `echo "conteúdo" | atomwrite --workspace . apply src/file.txt --format full`
- FÓRMULA set `atomwrite --workspace . set Cargo.toml package.name meu-crate`
- FÓRMULA get `atomwrite --workspace . get config.toml database.pool.max`
- FÓRMULA del `atomwrite --workspace . del --force-missing config.toml features.experimental`


## case query outline
- case converte identificadores — snake, camel, pascal, kebab, screaming-snake
- DEVE passar `--subvert OLD NEW` (sem isso exit 65)
- DEVE rodar case com `--dry-run` em codebase grande
- query inspeciona AST tree-sitter — `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- outline extrai estrutura — `--kind` (repetível), `--positions`, `--language`
- FÓRMULA case `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`
- FÓRMULA query `atomwrite --workspace . query --kinds src/main.rs`
- FÓRMULA outline `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## wal edit-loop prune completions
- `wal-stats` inspeciona journals (consultivo; NÃO modifica)
- `wal-heal` remove journals terminais órfãos — `--threshold-secs` (padrão 3600), `--max-duration-ms` (padrão 100)
- `edit-loop` aplica N pares `{old,new}` em uma escrita (JSON array ou NDJSON)
- edit-loop aceita `--line-ending`, `--syntax-check`, `--allow-sequential-drift`, opts de backup e herda fuzzy do config
- `prune-backups` limpa legados — `--max-age-secs` (NÃO `--max-age`), `--max-count`, `--dry-run`
- `completions` gera shell completions — `bash|zsh|fish|elvish|powershell` com `--install`
- FÓRMULA edit-loop `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FÓRMULA prune `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`
- FÓRMULA completions `atomwrite completions bash --install`


## Erros
- DEVE verificar exit code ANTES de parsear stdout
- DEVE parsear envelope JSON quando `error` for true
- Campos — error, code, exit, message, path, error_class, retryable, suggestion, workspace, best_candidate
- Estratégia por `error_class` — permanent (NUNCA retentar), transient (backoff), conflict (relê estado), precondition_failed (corrige pré-condição)
- DEVE retentar SOMENTE quando `retryable` for true
- DEVE usar `suggestion` para remediação
- NUNCA ignore exit não-zero exceto zero matches documentado
- NUNCA parseie stderr para dados de erro


## Exit codes
- 0 sucesso
- 1 zero matches em search/replace/transform/scope/semantic-search (NÃO é erro de sistema)
- 2 argumento inválido / conflito de flags
- 4 não encontrado
- 13 permissão negada
- 28 disco cheio
- 30 cota excedida
- 65 entrada inválida (pattern vazio, fuzzy off, chave ausente, best_candidate em match falho)
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
- Edit com match_count — `atomwrite --workspace . edit src/f.rs --old "x" --new "y" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- Fuzzy replace com progresso — `atomwrite --workspace . replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run 'old_api' 'new_api' src/`
- Best candidate — após exit 65 execute `jaq '.best_candidate'` no envelope
- Recipe search-replace-verify — `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- Sparse monorepo — `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- Sparse outline AST — `atomwrite --workspace . sparse outline src/ --max-files 50 --include '*.rs' | jaq 'select(.type=="outline_item") | {kind, name, path}'`
- Sparse read — `atomwrite --workspace . sparse read --paths-file paths.txt --head 40 --max-files 15`
- Semantic merge line-based — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- Semantic search offline — `atomwrite --workspace . semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- Codemod dry-run — `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- Agent surface — `atomwrite --workspace . agent-surface --format json`
- Batch transaction — `atomwrite --workspace . batch --file ops.ndjson --transaction --dry-run`
- Transform dry-run — `atomwrite --workspace . transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- Rollback verify — `atomwrite --workspace . rollback src/config.toml --latest --verify`
- Hash sem bak — `atomwrite --workspace . hash -r . --exclude '*.bak.*' | jaq -r '.checksum'`
- Watch limitado — `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 50 --checksum --gitignore true`
- Stat rápido — `atomwrite --workspace . stat src/main.rs`
- Scope comments — `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`
- Cancel awareness — em exit 143 após SIGTERM trate como cancel limpo e NÃO reutilize tempfile
