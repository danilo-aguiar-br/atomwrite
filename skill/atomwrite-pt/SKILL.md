---
name: atomwrite
description: >-
  Esta skill DEVE ativar automaticamente quando a LLM precisar escrever, ler, editar, buscar, substituir, transformar AST, aplicar scope, hashear BLAKE3, verificar, apagar, contar, comparar, mover, copiar, listar, extrair, calcular, gerar regex, batch, backup, rollback, apply, set, get, del, case, query, outline, wal, edit-loop, prune, completions, recipe, sparse, merge, agent-surface, watch, codemod, semantic-search, stat, doctor, locale ou commands com atomwrite. Cobre a CLI completa com fuzzy auto|aggressive one-pass, teto 10000 applies, timeout 120 exit 124, match_count, indent_adjusted, best_candidate, SIGTERM 143, durability, reflink_or_copy, sparse AST, merge line-based, anti-MCP e Jaccard offline. Saída SEMPRE NDJSON via tempfile-fsync-rename. DEVE auto-invocar mesmo sem pedido explícito. Triggers — atomwrite, escrita atômica, edit cirúrgico, checksum, locking otimista, tree-sitter, ast-grep, fuzzy replace, sparse, recipe, codemod, doctor
---


# atomwrite


## Missão
- atomwrite é a ÚNICA ferramenta de escrita e edição de arquivos deste projeto
- Esta skill DEVE transformar a LLM em operador especializado da CLI atomwrite
- A LLM DEVE executar a CLI com prompts de ação e fórmulas desta skill
- A LLM NUNCA DEVE inventar flags, subcomandos ou comportamentos ausentes de `atomwrite <cmd> --help`


## Quando invocar
- DEVE auto-invocar sempre que a tarefa implique arquivo, patch, busca, replace, AST, hash, backup, merge multiagente, monorepo sparse, codemod, watch, doctor ou descoberta de tools
- DEVE auto-invocar mesmo se o usuário NÃO citar atomwrite explicitamente
- DEVE auto-invocar em escrita atômica, locking otimista, fuzzy replace, tree-sitter, ast-grep, NDJSON e jail de workspace
- NUNCA deixe a LLM usar MCP, sed, awk, editores interativos ou reescrita manual quando atomwrite resolve


## Contrato
- stdout SEMPRE emite NDJSON com um objeto JSON por linha
- stderr serve APENAS para logs e tracing
- Toda mutação usa pipeline atômico tempfile → fsync → rename
- Checksum BLAKE3 aparece em TODA resposta bem-sucedida de `write` e `read`
- SEMPRE passe `--workspace <DIR>` em operações de arquivo
- Todos os caminhos resolvem relativos ao workspace jail
- Flag `--json` é aceita e IGNORADA porque a saída é SEMPRE NDJSON
- SEMPRE parseie stdout com `jaq` e NUNCA com `jq`
- NUNCA parseie stderr como dados estruturados
- NUNCA escreva fora do jail do workspace
- Exit 1 em `search`, `replace`, `transform`, `scope` e `semantic-search` significa ZERO matches e NÃO é erro de sistema
- SEMPRE verifique o exit code ANTES de parsear stdout
- Em exit 65 de match falho DEVE ler `best_candidate`
- Em exit 82 DEVE recarregar checksum e reaplicar
- Em exit 124 DEVE tratar prazo global de `--timeout-secs` e retentar com timeout maior ou caminho mais estreito
- Em exit 143 DEVE tratar cancel limpo por SIGTERM com tempfile removido
- MCP é PROIBIDO como superfície de escrita
- Schema de envelopes DEVE seguir `docs/schemas/` quando a validação formal for necessária


## Proibições
- NUNCA invente flags ausentes do help
- NUNCA use flag global `--lang` para locale — DEVE usar `--locale`
- NUNCA passe `--fuzzy off` — a CLI rejeita com exit 65
- NUNCA use `--replace-all` em `replace` — essa flag existe SOMENTE em `edit`
- NUNCA espere re-scan do texto recém-inserido no mesmo replace fuzzy
- NUNCA defina `--timeout-secs 0` em monorepo fuzzy salvo job intencionalmente sem prazo
- NUNCA pule `--dry-run` em replace, transform, scope ou codemod em árvore grande
- NUNCA reescreva arquivo inteiro com `write` se `edit`, `replace` ou `transform` resolve
- NUNCA ignore exit não-zero exceto zero-match documentado
- NUNCA use set, get ou del em texto puro
- NUNCA faça rollback crítico sem `--verify`
- NUNCA rode `watch` sem `--max-events` e sem timeout global


## Fluxo de execução
- DEVE localizar o alvo com `search` ou `sparse` antes de mutar
- DEVE capturar checksum com `read` quando houver concorrência
- DEVE escolher o nível mais preciso da hierarquia de edição
- DEVE rodar `--dry-run` antes de mutação destrutiva em massa
- DEVE aplicar mutação com `--expect-checksum` em sobrescrita concorrente
- DEVE validar envelope NDJSON com checksum, match_count, indent_adjusted e platform
- DEVE tratar falhas por `error_class` e `suggestion`
- DEVE retentar SOMENTE quando `retryable` for true


## Flags globais
- DEVE usar `--workspace <DIR>` como raiz do jail em ops de arquivo
- DEVE usar `--config <PATH>` para forçar `.atomwrite.toml` explícito
- DEVE usar `--timeout-secs <N>` ou alias `--timeout <N>` com padrão 120
- DEVE tratar prazo estourado com exit 124
- DEVE passar 0 em `--timeout-secs` SOMENTE para desligar o prazo
- DEVE usar `-j N`, `--threads N` ou alias `--max-concurrency N` para paralelismo
- DEVE tratar omitido ou 0 em threads como todos os cores com teto de RAM
- DEVE usar `--max-filesize <BYTES>` com padrão global 1 GiB
- DEVE usar `--locale <TAG>` para mensagens e suggestions
- DEVE usar `--json-schema` para emitir schema do subcomando e sair
- DEVE usar `--no-auto-heal` para pular wal-heal na inicialização
- DEVE usar `--no-progress` para desligar heartbeats NDJSON de progresso
- DEVE usar `--no-gitignore`, `--hidden` e `--follow-symlinks` quando o escopo exigir
- DEVE usar `--color auto|always|never` ou `--no-color`
- DEVE usar `-v`, `-vv`, `-vvv` ou `-q`, `-qq` para verbosidade
- FÓRMULA global — `atomwrite --workspace . --config .atomwrite.toml --timeout-secs 120 --max-concurrency 4 --locale pt-BR --no-progress <cmd>`


## Fuzzy obrigatório
- Fuzzy é OBRIGATÓRIO em `edit` e `replace` com padrão `auto`
- Valores permitidos são SOMENTE `--fuzzy auto` e `--fuzzy aggressive`
- DEVE usar `--fuzzy-threshold <FLOAT>` entre 0.0 e 1.0
- Com `--regex` no replace o fuzzy é IGNORADO
- Multi-apply fuzzy é SEMPRE one-pass da esquerda para a direita no conteúdo original
- NUNCA espere rebusca do texto inserido pelo mesmo replace
- Quando NEW embute OLD a aplicação é forçada a uma ocorrência e isso é CORRETO e SEGURO
- Sem `--max-replacements` o padrão fuzzy multi é 1 apply
- Teto duro de applies é 10000 e a CLI corta acima desse teto
- Caps reais — pattern 64 KiB, Levenshtein 8192 chars, janelas 4096, crescimento max de 4x ou mais 16 MiB
- Em falha de match DEVE ler `best_candidate` no envelope exit 65
- DEVE usar `--dry-run` antes de fuzzy replace em massa
- Em exit 124 DEVE aumentar `--timeout-secs` ou estreitar o caminho
- FÓRMULA fuzzy seguro — `atomwrite --workspace . replace --dry-run --fuzzy auto --max-replacements 1 'OLD' 'NEW' src/`


## Hierarquia de edição
- Ordem OBRIGATÓRIA do mais preciso ao mais bruto — search → transform → scope → replace → edit → write
- DEVE usar `search` para localizar e inspecionar
- DEVE usar `transform` para reescrita estrutural por AST
- DEVE usar `scope` para ações gramaticais
- DEVE usar `replace` para substituição textual multiarquivo
- DEVE usar `edit` para edição cirúrgica em um arquivo
- DEVE usar `write` SOMENTE para criar arquivo ou sobrescrita total intencional


## Backup e configuração
- Backup default é true nos mutadores via `--backup`, `--no-backup`, `--keep-backup` e `--retention <N>`
- `--backup` e `--no-backup` são mutuamente exclusivos com exit 2
- Backup padrão cria `.bak.<timestamp>` adjacente e AUTO-REMOVE no sucesso
- Em falha o `.bak` permanece para rollback
- EXCEÇÃO — `delete` e `replace` PRESERVAM o `.bak` no sucesso
- EXCEÇÃO — `rollback` exige `--backup` explícito para backup opt-in
- EXCEÇÃO — `move` e `copy` exigem `--force` OU `--backup` explícito OU env `ATOMWRITE_BACKUP` para sobrescrever destino
- TOML `[defaults] backup = true` sozinho NÃO autoriza sobrescrita em move e copy
- Backup usa `reflink_or_copy` e NUNCA hardlink do arquivo vivo
- Precedência — env `ATOMWRITE_BACKUP` com valor exato 0 desliga, depois flags CLI, depois `.atomwrite.toml`, depois default embutido
- DEVE ler `platform.backup_method`, `platform.durability` e `platform.rename_method` quando emitidos
- `.atomwrite.toml` governa `[defaults]` e `[fuzzy]`
- `mode` fuzzy VÁLIDO é SOMENTE `auto` ou `aggressive`
- NUNCA configure `mode = "off"`
- Flags CLI vencem o TOML quando explicitadas
- FÓRMULA config — defina `[fuzzy]` com `mode = "auto"` e `threshold = 0.85` em `.atomwrite.toml`


## Mutação de arquivos
### write
- DEVE enviar conteúdo via stdin e NUNCA como argumento CLI
- DEVE usar `--expect-checksum <BLAKE3>` para locking otimista
- DEVE usar `--durability full|fast|auto` com padrão `auto`
- DEVE usar `--allow-shrink` se a escrita encolher mais de 50 por cento com checksum
- DEVE usar `--allow-empty-stdin` e `--no-checksum-when-empty` quando stdin vazio for intencional
- DEVE usar `--syntax-check` para validar tree-sitter com exit 88 em falha
- Flags — `--append`, `--prepend`, `--max-size`, `--line-ending`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--confirm`, `--risk-threshold`, `--wal-policy`, `--dry-run`, opts de backup
- SIGTERM durante write limpa tempfile e retorna exit 143
- FÓRMULA — `echo "conteúdo" | atomwrite --workspace . write --durability full alvo.rs`
- FÓRMULA lock — `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto arq`

### edit
- DEVE usar `--old` e `--new` para pares exatos repetíveis
- Multi-par roda cascata fuzzy e emite `pairs_total` com `pair_results`
- Par falho aborta o lote por padrão e `--partial` aplica só os que casam
- Multi-ocorrência em edit EXIGE `--replace-all`
- Sucesso em edit com `--replace-all` emite `match_count`
- Indent realinhado emite `indent_adjusted: true`
- Flags de posição — `--after-line`, `--before-line`, `--range`, `--delete-range`, `--after-match`, `--before-match`, `--between`, `--multi`
- DEVE usar `--old-file` e `--new-file` para payloads grandes
- DEVE usar `--expect-checksum` e `--allow-sequential-drift` só em pipeline controlado
- Stdin de terminal nos modos de inserção falha com exit 65 e NÃO trava
- NUNCA pipe edit para `jaq` sem checar `${PIPESTATUS[0]}`
- FÓRMULA — `atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo" --fuzzy auto`
- FÓRMULA multi — `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- FÓRMULA insert — `echo "linha" | atomwrite --workspace . edit src/main.rs --after-line 10`

### replace
- Exit 1 significa zero matches e NÃO é erro de sistema
- DEVE SEMPRE rodar `--dry-run` ou `--preview` antes de aplicar
- DEVE usar `--fuzzy auto|aggressive` e `--fuzzy-threshold`
- DEVE usar `--progress-every <N>` com padrão 50 e 0 para desligar
- Multi-ocorrência fuzzy usa `--max-replacements` e NUNCA `--replace-all`
- Padrão fuzzy multi é 1 apply e teto duro é 10000
- Quando NEW embute OLD o fuzzy força um apply mesmo com max alto
- Flags — `--regex`, `-w`, `-F`, `-g/--include`, `--exclude`, `-n/--max-replacements`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case`, opts de backup
- `replace` PRESERVA `.bak` no sucesso
- FÓRMULA dry-run — `atomwrite --workspace . replace --dry-run --fuzzy auto --progress-every 20 'antigo' 'novo' src/`
- FÓRMULA agressivo — `atomwrite --workspace . replace --fuzzy aggressive --fuzzy-threshold 0.8 --max-replacements 3 'API' 'Api' src/ --include '*.rs'`
- FÓRMULA expandir seção — `atomwrite --workspace . replace --fuzzy auto $'section {\n  old\n}' $'section {\n  old\n  new_line\n}' path.rs`

### delete move copy apply set get del case
- `delete` mantém backup ligado e PRESERVA `.bak` no sucesso
- DEVE usar `-y/--yes` em automação e `--older-than` para limpeza por idade
- `move` e `copy` exigem `--force` ou `--backup` ou env para sobrescrever destino
- `copy` aceita `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`
- `apply` detecta unified, SEARCH/REPLACE, markdown e full file
- DEVE usar `--format auto|unified|search-replace|full|markdown` quando o auto-detect for ambíguo
- set, get e del operam SOMENTE em TOML ou JSON via dotted path
- get retorna exit 65 se a chave estiver ausente
- del aceita `--force-missing`
- case exige `--subvert OLD NEW` e DEVE rodar com `--dry-run` em codebase grande
- FÓRMULA delete — `atomwrite --workspace . delete --older-than 7d --yes tmp/`
- FÓRMULA move — `atomwrite --workspace . move src/old.rs src/new.rs`
- FÓRMULA copy — `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`
- FÓRMULA apply — `echo "conteúdo" | atomwrite --workspace . apply src/file.txt --format full`
- FÓRMULA set — `atomwrite --workspace . set Cargo.toml package.name meu-crate`
- FÓRMULA get — `atomwrite --workspace . get config.toml database.pool.max`
- FÓRMULA del — `atomwrite --workspace . del --force-missing config.toml features.experimental`
- FÓRMULA case — `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`


## Leitura e busca
### read e stat
- DEVE usar `read` para conteúdo com checksum, size, lines e mode
- DEVE usar `stat PATH` como alias de `read --stat`
- Flags — `--lines`, `--line`, `-C/--context`, `--head`, `--tail`, `--format raw|ndjson`, `--grep`, `--verify-checksum`
- Exit 81 em mismatch de `--verify-checksum`
- FÓRMULA — `atomwrite --workspace . read --head 20 src/main.rs`
- FÓRMULA — `atomwrite --workspace . stat src/main.rs`

### search
- Exit 1 significa zero resultados e NÃO é erro de sistema
- DEVE usar search para localizar alvos antes de mutar
- Flags — `-g/--include`, `--exclude`, `-C`, `-F`, `-e/--regex`, `-w`, `-i`, `-S`, `-c`, `-l`, `-m`, `-U`, `-P`, `--invert`, `--sort`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Default local de `--max-filesize` no search é 10 MiB
- FÓRMULA — `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs' --sort path`

### list count extract diff
- list — `--include`, `--exclude`, `--long`, `--depth`, `--count-by-ext`, `--all`
- count — `--by-extension`, `--by-size`, `--top`
- extract — campos posicionais e `--delimiter`, `--stdin`
- diff — `--unified`, `--stat`, `-C`, `--algorithm myers|patience|lcs`
- FÓRMULA list — `atomwrite --workspace . list --long --depth 2 src/`
- FÓRMULA count — `atomwrite --workspace . count --by-size --top 10 src/`
- FÓRMULA extract — `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`
- FÓRMULA diff — `atomwrite --workspace . diff src/old.rs src/new.rs --unified`


## AST e estrutura
### transform
- Exit 1 significa zero matches
- DEVE especificar `-l/--language`
- DEVE usar `$NAME` para um nó e `$$$ARGS` para múltiplos
- DEVE usar `-p/--pattern` e `-r/--rewrite` juntos no modo single-rule
- Flags — `--rules`, `--inline-rules`, `--verify-parse`, `--include`, `--exclude`, `--dry-run`, opts de backup
- DEVE preferir transform sobre replace quando o padrão é sintático
- FÓRMULA — `atomwrite --workspace . transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`

### scope
- Exit 1 significa zero matches
- DEVE especificar `--language` com alias `--lang` no subcomando
- DEVE usar `--query` ou `--pattern`
- DEVE usar `--delete`, `--action` ou `--replace-with`
- Queries úteis — comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- NUNCA aplique `--delete` sem dry-run
- FÓRMULA — `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`

### query e outline
- query inspeciona AST com `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- outline extrai estrutura com `--kind` repetível, `--positions` e `--language`
- FÓRMULA query — `atomwrite --workspace . query --kinds src/main.rs`
- FÓRMULA outline — `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## Orquestração multi-arquivo
### batch
- Entrada NDJSON via stdin com campo `op` obrigatório — write, replace, delete, edit, move, copy, hash
- move e copy no manifesto exigem `"force":true` para sobrescrever
- DEVE usar `--transaction` para atomicidade total
- DEVE usar `--file <PATH>` para manifesto em disco
- Flags — `--dry-run`, `--batch-size`, `--input-schema`, opts de backup
- FÓRMULA — `echo '{"op":"write","target":"a.txt","content":"olá"}' | atomwrite --workspace . batch --transaction`

### recipe sparse semantic-merge agent-surface watch codemod semantic-search
- DEVE usar `sparse list` com `--max-files` e `--max-bytes`
- DEVE usar `sparse read --paths-file` com `--head` e `--max-files`
- DEVE usar `sparse outline` para outline AST real sob orçamento
- DEVE usar `recipe list` e `recipe run --name search-replace-verify`
- Recipe search, replace e hash EXCLUI `*.bak.*` por padrão
- Receitas embutidas — `search-replace-verify` e `edit-loop-syntax-check`
- `recipe run` aceita `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- DEVE usar `semantic-merge --base A --ours B --theirs C --output OUT`
- semantic-merge é line-based e NÃO é AST nem embedding
- Flags de merge — `--fail-on-conflict`, `--write-conflict-markers`, `--expect-checksum`, opts de backup
- DEVE usar `agent-surface --format json` para manifesto de tools
- DEVE usar `watch` com `--debounce-ms`, `--max-events`, `--checksum` e `--gitignore`
- DEVE encapsular watch com `--max-events` ou timeout global
- `watch` exige feature `watch` no binário
- DEVE usar `codemod --rules rules.yaml --dry-run` antes de aplicar
- DEVE usar `semantic-search "query" PATH --k 20 --min-score 0.05`
- semantic-search é Jaccard offline e NUNCA depende de embedding de rede
- DEVE passar `--index-dir` SOMENTE quando índice local for necessário
- Exit 1 em semantic-search com zero resultados NÃO é erro de sistema
- FÓRMULA sparse list — `atomwrite --workspace . sparse list src/ --max-files 50 --max-bytes 524288 --include '*.rs'`
- FÓRMULA sparse read — `atomwrite --workspace . sparse read --paths-file paths.txt --head 30 --max-files 10`
- FÓRMULA sparse outline — `atomwrite --workspace . sparse outline src/ --max-files 40 --include '*.rs'`
- FÓRMULA recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern old_api --replacement new_api --path src --dry-run --fuzzy auto`
- FÓRMULA merge — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output out.rs --fail-on-conflict`
- FÓRMULA agent — `atomwrite --workspace . agent-surface --format json`
- FÓRMULA watch — `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 100 --checksum`
- FÓRMULA codemod — `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- FÓRMULA semantic-search — `atomwrite --workspace . semantic-search "optimistic lock checksum" src/ --k 15 --min-score 0.1 --index-dir .atomwrite/semantic-index`


## Descoberta e host
- DEVE usar `doctor` para diagnosticar ambiente e dependências do host
- DEVE usar `doctor --strict` quando qualquer falha DEVE abortar o pipeline
- DEVE usar `locale` para inspecionar locale resolvido
- DEVE usar `locale --set pt-BR` ou `locale --set en` para persistir preferência
- DEVE usar `locale --clear` para limpar preferência persistida
- DEVE usar `commands` para emitir a árvore completa de comandos em JSON
- DEVE usar `commands --include-globals` para incluir flags globais no manifesto
- FÓRMULA doctor — `atomwrite doctor --strict`
- FÓRMULA locale — `atomwrite locale --set pt-BR`
- FÓRMULA commands — `atomwrite commands --include-globals`


## Utilitários
### hash e verify
- DEVE usar `hash` para checksums BLAKE3
- Campo de saída é `checksum` e NUNCA `value`
- Flags — `--verify`, `--stdin`, `-r/--recursive`, `--exclude`
- DEVE usar `verify <PATH> <EXPECTED_HASH>` com exit 0 se casa e 81 se NÃO casa
- FÓRMULA — `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FÓRMULA recursive — `atomwrite --workspace . hash -r src/ --exclude '*.bak.*'`
- FÓRMULA verify — `atomwrite --workspace . verify src/main.rs abc123def456`

### backup rollback prune
- DEVE usar `backup` com `--retention` e `--output-dir`
- DEVE usar `rollback --latest --verify` em restauração crítica
- Flags de rollback — `--timestamp`, `--latest`, `--verify`, `--dry-run`, `--backup` opt-in
- DEVE usar `prune-backups` com `--max-age-secs` e NUNCA `--max-age`
- FÓRMULA backup — `atomwrite --workspace . backup src/main.rs --retention 3`
- FÓRMULA rollback — `atomwrite --workspace . rollback src/config.toml --latest --verify`
- FÓRMULA prune — `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`

### calc regex wal edit-loop completions
- calc avalia expressão e unidades sem exigir workspace
- regex gera padrão a partir de 3 ou mais exemplos
- Flags regex — `-d`, `-w`, `-s`, `-r`, `-i`, `--no-anchors`, `--stdin`
- `wal-stats` inspeciona journals e NÃO modifica
- `wal-heal` remove journals órfãos com `--threshold-secs` e `--max-duration-ms`
- `edit-loop` aplica N pares old e new em uma escrita
- `completions` gera bash, zsh, fish, elvish ou powershell com `--install`
- FÓRMULA calc — `atomwrite calc "2 hours + 30 minutes to seconds"`
- FÓRMULA regex — `atomwrite regex "abc-12" "xyz-99" "id-7" --digits`
- FÓRMULA edit-loop — `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FÓRMULA completions — `atomwrite completions bash --install`


## Parsing e erros
- DEVE verificar exit code ANTES de parsear stdout
- DEVE parsear envelope JSON quando `error` for true
- Campos — error, code, exit, message, path, error_class, retryable, suggestion, workspace, best_candidate
- Estratégia por `error_class` — permanent NUNCA retenta, transient usa backoff, conflict relê estado, precondition_failed corrige pré-condição
- DEVE retentar SOMENTE quando `retryable` for true
- DEVE usar `suggestion` para remediação
- NUNCA parseie stderr para dados de erro


## Exit codes
- 0 sucesso
- 1 zero matches em search, replace, transform, scope e semantic-search
- 2 argumento inválido ou conflito de flags
- 4 não encontrado
- 13 permissão negada
- 28 disco cheio
- 30 cota excedida
- 65 entrada inválida, fuzzy off, chave ausente ou best_candidate em match falho
- 73 cross-device
- 74 erro de I/O
- 78 configuração inválida
- 81 checksum mismatch
- 82 state drift
- 83 timeout de lock
- 85 FIFO detectado
- 86 arquivo de dispositivo
- 88 erro de sintaxe tree-sitter
- 91 fallback EXDEV desabilitado
- 92 verificação BLAKE3 de copy-back falhou
- 93 journal órfão consultivo
- 124 timeout global de `--timeout-secs`
- 126 violação do jail do workspace
- 127 symlink bloqueado
- 128 arquivo imutável
- 130 SIGINT
- 141 SIGPIPE
- 143 SIGTERM com cancel limpo
- 255 erro interno


## Fórmulas de execução
- DEVE executar locking otimista — `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto arq`
- DEVE executar edit com match_count — `atomwrite --workspace . edit src/f.rs --old "x" --new "y" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- DEVE executar fuzzy replace com progresso — `atomwrite --workspace . replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run 'old_api' 'new_api' src/`
- DEVE executar expansão de seção com NEW embutindo OLD — `atomwrite --workspace . replace --fuzzy auto $'section {\n  old\n}' $'section {\n  old\n  new_line\n}' path.rs`
- DEVE recuperar timeout 124 aumentando `--timeout-secs` ou estreitando o caminho
- DEVE ler best_candidate após exit 65 com `jaq '.best_candidate'`
- DEVE executar recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- DEVE executar sparse monorepo — `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- DEVE executar merge line-based — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- DEVE executar semantic-search offline — `atomwrite --workspace . semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- DEVE executar host readiness — `atomwrite doctor --strict && atomwrite commands --include-globals`
- DEVE tratar exit 143 como cancel limpo e NUNCA reutilizar tempfile
