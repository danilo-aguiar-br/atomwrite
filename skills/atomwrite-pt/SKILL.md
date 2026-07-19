---
name: atomwrite
description: >-
 Esta skill DEVE ativar automaticamente sempre que a LLM precisar do atomwrite para write, read, edit, search, replace, transform AST, scope, hash BLAKE3, verify, delete, count, diff, move, copy, list, extract, calc, regex, batch, backup, rollback, apply, set, get, del, case, query, outline, wal, edit-loop, prune, completions, recipe, sparse, semantic-merge, agent-surface, watch, codemod, semantic-search, stat, doctor, locale ou commands. Esta skill DEVE ensinar fuzzy auto|aggressive|off exact-only, one-pass, teto 10000, timeout 120 exit 124, MATCH_FAILED best_candidate, ack-overwrite write grande, watch_summary, delete --plan, SIGTERM 143, durability, reflink_or_copy, sparse, merge line-based, anti-MCP, Jaccard offline. Saída DEVE ser SEMPRE NDJSON via tempfile-fsync-rename. Esta skill DEVE auto-invocar sem pedido explícito. Triggers incluem atomwrite, escrita atômica, edit cirúrgico, checksum, locking otimista, tree-sitter, ast-grep, fuzzy replace, sparse, recipe, codemod, doctor
---

# atomwrite

## Missão
### OBRIGATÓRIO
- A LLM DEVE tratar atomwrite como a ÚNICA superfície permitida de escrita e edição de arquivos deste projeto
- A LLM DEVE executar esta skill como runbook de ação e DEVE rodar a CLI com as fórmulas abaixo
- A LLM DEVE rodar `atomwrite <cmd> --help` antes de inventar qualquer flag e DEVE recusar flags desconhecidas
- A LLM DEVE parsear stdout com `jaq` e DEVE checar o exit code ANTES de parsear
### PROIBIDO
- NUNCA use MCP, sed, awk, editores interativos ou reescrita manual quando atomwrite resolve a tarefa
- NUNCA invente subcomandos, flags ou comportamentos ausentes do help
- NUNCA invente knobs de produto `ATOMWRITE_*`
- NUNCA parseie stderr como dados estruturados


## Auto-invocação
### OBRIGATÓRIO
- Esta skill DEVE auto-invocar sempre que a tarefa implique arquivos, patches, busca, replace, AST, hash, backup, merge multiagente, monorepo sparse, codemod, watch, doctor ou descoberta de tools
- Esta skill DEVE auto-invocar mesmo quando o usuário NÃO citar atomwrite
### PROIBIDO
- NUNCA espere pedido explícito de skill quando uma operação de arquivo for necessária


## Contrato
### OBRIGATÓRIO
- stdout DEVE emitir SEMPRE NDJSON com um objeto JSON por linha
- stderr DEVE servir APENAS para logs e tracing
- Toda mutação DEVE usar tempfile depois fsync depois rename
- Toda escrita e leitura bem-sucedida DEVE expor `checksum` BLAKE3
- Ops de arquivo DEVEM passar `--workspace <DIR>` e DEVEM resolver caminhos dentro do jail do workspace
- `--json` é aceita e DEVE ser tratada como ignorada porque a saída é SEMPRE NDJSON
- Schemas de envelope DEVEM seguir `docs/schemas/` quando a validação formal for necessária
- Precedência de config DEVE ser flags CLI depois `.atomwrite.toml` ou XDG depois defaults embutidos
### PROIBIDO
- NUNCA escreva fora do jail do workspace
- NUNCA dependa de knobs de env de produto para comportamento em runtime
- NUNCA trate exit 1 em `search`, `replace`, `transform`, `scope` ou `semantic-search` como crash de sistema quando zero matches for o resultado documentado `NO_MATCHES`
### Padrão Correto
- FÓRMULA global — `atomwrite --workspace . --config .atomwrite.toml --timeout-secs 120 --max-concurrency 4 --locale pt-BR --no-progress <cmd>`


## Contratos de segurança do agente
### OBRIGATÓRIO
- Sobrescrita grande de arquivo existente DEVE passar `--ack-overwrite` quando o tamanho exceder XDG `[write].confirm_large_bytes` padrão 100 KiB
- Falta de `--ack-overwrite` em sobrescrita grande DEVE ser tratada como exit 65
- Delete plan-only DEVE usar `delete --plan`
- Delete de execução DEVE omitir `--plan` e DEVE omitir flags de confirm rejeitadas
- `watch` DEVE sempre terminar com NDJSON `type:watch_summary` mesmo em idle ou zero eventos
- Idle padrão de watch é cerca de 500 ms via XDG ou built-in salvo override
- `semantic-merge` DEVE gravar markers de conflito por padrão
- Prefer-ours sem markers DEVE passar `--no-conflict-markers` de forma intencional
- Em falha de match exit 65 DEVE parsear o envelope e DEVE ler `best_candidate`
- Em exit 82 DEVE recarregar checksum e reaplicar
- Em exit 124 DEVE aumentar `--timeout-secs` ou estreitar o caminho
- Em exit 143 DEVE tratar cancel limpo por SIGTERM com tempfile removido
### PROIBIDO
- NUNCA passe `delete --confirm`, `delete --yes` ou `-y` porque são rejeitados
- NUNCA sobrescreva arquivos grandes existentes sem `--ack-overwrite`
- NUNCA use prompts interativos Y/N para confirmação de write
- NUNCA trate o legado write `--confirm` como substituto de `--ack-overwrite`
- NUNCA assuma stdout vazio de `watch`
- NUNCA desligue markers de merge por acidente
- NUNCA assuma telemetria de produto e NUNCA embuta GitHub Actions dentro desta CLI de produto
### Padrão Correto
- FÓRMULA write grande — `printf 'payload' | atomwrite --workspace . write --ack-overwrite big.txt`
- FÓRMULA delete plan — `atomwrite --workspace . delete --plan tmp/`
- FÓRMULA delete exec — `atomwrite --workspace . delete tmp/stale.txt`
- FÓRMULA watch — `atomwrite --workspace . watch src/ --debounce-ms 200 --max-events 100 --idle-exit-ms 500 --checksum`
- FÓRMULA merge — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output out.rs --fail-on-conflict`


## Flags globais
### OBRIGATÓRIO
- DEVE usar `--workspace <DIR>` como raiz do jail
- DEVE usar `--config <PATH>` para forçar config explícita
- DEVE usar `--timeout-secs <N>` ou alias `--timeout <N>` padrão 120 e DEVE tratar prazo estourado como exit 124
- DEVE passar `0` para desligar o prazo SOMENTE quando for intencional
- DEVE usar `-j N`, `--threads N` ou alias `--max-concurrency N` para paralelismo
- DEVE tratar threads omitidas ou zero como todos os cores com teto de RAM
- DEVE usar `--max-filesize <BYTES>` com padrão global 1 GiB
- DEVE usar `--locale <TAG>` para mensagens e suggestions e NUNCA usar global `--lang` para locale
- DEVE usar `--json-schema` para emitir o schema do subcomando e sair
- DEVE usar `--no-auto-heal` para pular wal-heal na inicialização
- DEVE usar `--no-progress` para desligar heartbeats NDJSON de progresso
- DEVE usar `--no-gitignore`, `--hidden` e `--follow-symlinks` quando o escopo exigir
- DEVE usar `--color auto|always|never` ou `--no-color`
- DEVE usar `-v`, `-vv`, `-vvv` ou `-q`, `-qq` para verbosidade
### PROIBIDO
- NUNCA invente flags globais ausentes de `atomwrite --help`


## Fuzzy
### OBRIGATÓRIO
- Fuzzy é OBRIGATÓRIO em `edit` e `replace` com padrão `auto`
- Valores permitidos DEVEM ser somente `--fuzzy auto`, `--fuzzy aggressive` e `--fuzzy off`
- `--fuzzy off` DEVE significar exact-only sem cascata
- Miss exact-only em edit DEVE gerar exit 65 `MATCH_FAILED`
- Miss em árvore no replace com zero hits DEVE frequentemente gerar exit 1 `NO_MATCHES`
- DEVE usar `--fuzzy-threshold <FLOAT>` entre 0.0 e 1.0 ao calibrar
- Com `--regex` no replace o fuzzy DEVE ser tratado como ignorado
- Multi-apply DEVE ser one-pass da esquerda para a direita no conteúdo original
- Quando NEW embute OLD o apply DEVE forçar uma ocorrência
- Sem `--max-replacements` o padrão fuzzy multi DEVE ser 1 apply
- Teto duro de applies DEVE ser 10000 e a CLI corta acima disso
- Caps reais DEVEM ser tratados como pattern 64 KiB, Levenshtein 8192 chars, janelas 4096, crescimento max de 4x ou mais 16 MiB
- Agentes DEVEM usar `--fuzzy auto` ou `--fuzzy aggressive` por padrão salvo exact-only intencional
### PROIBIDO
- NUNCA espere rebusca do texto inserido pelo mesmo replace fuzzy
- NUNCA use `--replace-all` em `replace`
- NUNCA defina `--timeout-secs 0` em monorepo fuzzy salvo job intencionalmente sem prazo
### Padrão Correto
- FÓRMULA fuzzy seguro — `atomwrite --workspace . replace --dry-run --fuzzy auto --max-replacements 1 'OLD' 'NEW' src/`
- FÓRMULA agressivo — `atomwrite --workspace . replace --fuzzy aggressive --fuzzy-threshold 0.8 --max-replacements 3 'API' 'Api' src/ --include '*.rs'`
- FÓRMULA exact-only — `atomwrite --workspace . edit src/f.rs --old "exact" --new "next" --fuzzy off`
- FÓRMULA expandir seção — `atomwrite --workspace . replace --fuzzy auto $'section {\n old\n}' $'section {\n old\n new_line\n}' path.rs`


## Hierarquia de edição
### OBRIGATÓRIO
- DEVE escolher ferramentas do mais preciso ao mais bruto nesta ordem — search depois transform depois scope depois replace depois edit depois write
- DEVE usar `search` para localizar
- DEVE usar `transform` para reescrita estrutural por AST
- DEVE usar `scope` para ações gramaticais
- DEVE usar `replace` para substituição textual multiarquivo
- DEVE usar `edit` para edição cirúrgica em um arquivo
- DEVE usar `write` SOMENTE para criar arquivo ou sobrescrever o arquivo inteiro de forma intencional
### PROIBIDO
- NUNCA reescreva o arquivo inteiro com `write` quando `edit`, `replace` ou `transform` resolve a mudança


## Fluxo de execução
### OBRIGATÓRIO
- DEVE localizar o alvo com `search` ou `sparse` antes de mutar
- DEVE capturar checksum com `read` quando houver concorrência
- DEVE rodar `--dry-run` antes de mutação destrutiva em massa
- DEVE aplicar sobrescrita concorrente com `--expect-checksum`
- DEVE validar campos NDJSON `checksum`, `match_count`, `indent_adjusted` e `platform` quando presentes
- DEVE tratar falhas por `error_class` e `suggestion`
- DEVE retentar SOMENTE quando `retryable` for true
### PROIBIDO
- NUNCA ignore exit não-zero exceto casos documentados de zero-match


## Backup e configuração
### OBRIGATÓRIO
- Backup default é true nos mutadores via `--backup`, `--no-backup`, `--keep-backup` e `--retention <N>`
- Backup padrão DEVE criar `.bak.<timestamp>` adjacente e auto-remover no sucesso
- Em falha o `.bak` DEVE permanecer para rollback
- `delete` e `replace` DEVEM preservar `.bak` no sucesso
- `rollback` DEVE usar `--backup` explícito para backup opt-in do alvo restaurado
- `move` e `copy` DEVEM passar `--force` ou `--backup` explícito para sobrescrever destino
- Backup DEVE usar `reflink_or_copy` e NUNCA hardlink do arquivo vivo
- `.atomwrite.toml` DEVE governar `[defaults]` e `[fuzzy]` com modos `auto`, `aggressive` e `off`
- Flags CLI explícitas DEVEM vencer o TOML
### PROIBIDO
- NUNCA passe `--backup` e `--no-backup` juntos porque isso é exit 2
- NUNCA assuma que TOML `[defaults] backup = true` sozinho autoriza sobrescrita em move ou copy
### Padrão Correto
- FÓRMULA config fuzzy — defina `[fuzzy]` com `mode = "auto"` e `threshold = 0.85` em `.atomwrite.toml`


## Mutação de arquivos
### write OBRIGATÓRIO
- DEVE enviar conteúdo via stdin e NUNCA como argumento CLI
- DEVE usar `--expect-checksum <BLAKE3>` para locking otimista
- DEVE usar `--durability full|fast|auto` com padrão `auto`
- DEVE usar `--allow-shrink` quando a escrita com lock encolher mais de 50 por cento
- DEVE usar `--allow-empty-stdin` e `--no-checksum-when-empty` quando stdin vazio for intencional
- DEVE usar `--syntax-check` para validar tree-sitter com exit 88 em falha
- DEVE passar `--ack-overwrite` em sobrescritas grandes existentes
- DEVE conhecer as flags `--append`, `--prepend`, `--max-size`, `--line-ending`, `--preserve-timestamps`, `--require-backup`, `--auto-rotate`, `--ack-overwrite`, `--require-large-ack`, `--risk-threshold`, `--wal-policy`, `--dry-run` e opts de backup
- Sobrescrita grande default-deny DEVE usar `--ack-overwrite` e NUNCA depender de confirm interativo
- SIGTERM durante write DEVE limpar tempfile e retornar exit 143
### write Padrão Correto
- FÓRMULA write — `echo "conteúdo" | atomwrite --workspace . write --durability full alvo.rs`
- FÓRMULA lock — `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto arq`

### edit OBRIGATÓRIO
- DEVE usar `--old` e `--new` para pares exatos repetíveis
- Multi-par DEVE rodar cascata fuzzy e emitir `pairs_total` com `pair_results`
- Par falho DEVE abortar o lote por padrão e `--partial` DEVE aplicar só os que casam
- Multi-ocorrência em edit DEVE passar `--replace-all`
- Sucesso com `--replace-all` DEVE emitir `match_count`
- Realinhamento de indent DEVE emitir `indent_adjusted: true` quando ajustado
- DEVE usar flags de posição `--after-line`, `--before-line`, `--range`, `--delete-range`, `--after-match`, `--before-match`, `--between`, `--multi`
- DEVE usar `--old-file` e `--new-file` para payloads grandes
- DEVE usar `--expect-checksum` e `--allow-sequential-drift` só em pipeline controlado
- Stdin de terminal nos modos de inserção DEVE falhar com exit 65 e NÃO travar
### edit PROIBIDO
- NUNCA pipe edit para `jaq` sem checar `${PIPESTATUS[0]}`
### edit Padrão Correto
- FÓRMULA edit — `atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo" --fuzzy auto`
- FÓRMULA multi — `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- FÓRMULA insert — `echo "linha" | atomwrite --workspace . edit src/main.rs --after-line 10`

### replace OBRIGATÓRIO
- Exit 1 DEVE significar zero matches e NÃO DEVE ser tratado como crash de sistema
- DEVE SEMPRE rodar `--dry-run` ou `--preview` antes de apply em massa
- DEVE usar `--fuzzy auto|aggressive|off` e `--fuzzy-threshold`
- DEVE usar `--progress-every <N>` padrão 50 e 0 para desligar
- Multi-ocorrência fuzzy DEVE usar somente `--max-replacements`
- DEVE conhecer flags `--regex`, `-w`, `-F`, `-g/--include`, `--exclude`, `-n/--max-replacements`, `--expect-checksum`, `--preserve-timestamps`, `--preserve-case` e opts de backup
### replace PROIBIDO
- NUNCA use `--replace-all` em replace
- NUNCA pule dry-run em árvore grande
### replace Padrão Correto
- FÓRMULA dry-run — `atomwrite --workspace . replace --dry-run --fuzzy auto --progress-every 20 'antigo' 'novo' src/`


## Ops de path e mutação estruturada
### OBRIGATÓRIO
- `delete` DEVE manter backup ligado e DEVE preservar `.bak` no sucesso
- Limpeza por idade em delete DEVE usar `--older-than`
- Delete recursivo DEVE usar `-r/--recursive` e globs com `-g/--include` quando necessário
- `move` e `copy` DEVEM passar `--force` ou `--backup` para sobrescrever destino
- `copy` DEVE usar `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr` quando exigido
- `apply` DEVE detectar unified, SEARCH/REPLACE, markdown e full file
- `apply` DEVE usar `--format auto|unified|search-replace|full|markdown` quando o auto-detect for ambíguo
- `set`, `get` e `del` DEVEM operar SOMENTE em TOML ou JSON via dotted path
- Chave ausente em get DEVE ser exit 65
- `del` DEVE aceitar `--force-missing` quando intencional
- `case` DEVE passar `--subvert OLD NEW` e DEVE dry-run em codebase grande
### PROIBIDO
- NUNCA passe aliases de confirm em delete
- NUNCA use set, get ou del em texto puro
### Padrão Correto
- FÓRMULA move — `atomwrite --workspace . move src/old.rs src/new.rs`
- FÓRMULA copy — `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`
- FÓRMULA apply — `echo "conteúdo" | atomwrite --workspace . apply src/file.txt --format full`
- FÓRMULA set — `atomwrite --workspace . set Cargo.toml package.name meu-crate`
- FÓRMULA get — `atomwrite --workspace . get config.toml database.pool.max`
- FÓRMULA del — `atomwrite --workspace . del --force-missing config.toml features.experimental`
- FÓRMULA case — `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`


## Leitura e busca
### OBRIGATÓRIO
- DEVE usar `read` para conteúdo com checksum, size, lines e mode
- DEVE usar `stat PATH` como alias de `read --stat`
- DEVE usar flags de read `--lines`, `--line`, `-C/--context`, `--head`, `--tail`, `--format raw|ndjson`, `--grep`, `--verify-checksum`
- Exit 81 em mismatch de `--verify-checksum` DEVE ser tratado como falha de integridade
- DEVE usar `search` antes de mutar quando a localização do alvo for desconhecida
- DEVE usar flags de search `-g/--include`, `--exclude`, `-C`, `-F`, `-e/--regex`, `-w`, `-i`, `-S`, `-c`, `-l`, `-m`, `-U`, `-P`, `--invert`, `--sort`, `--max-columns`, `--no-begin-end`, `--include-fifo`
- Default local de `--max-filesize` no search DEVE ser tratado como 10 MiB
- DEVE usar `list` com `--include`, `--exclude`, `--long`, `--depth`, `--count-by-ext`, `--all`
- DEVE usar `count` com `--by-extension`, `--by-size`, `--top`
- DEVE usar `extract` com campos posicionais mais `--delimiter` e `--stdin`
- DEVE usar `diff` com `--unified`, `--stat`, `-C`, `--algorithm myers|patience|lcs`
- Segundo path `-` no diff DEVE significar stdin
### Padrão Correto
- FÓRMULA read — `atomwrite --workspace . read --head 20 src/main.rs`
- FÓRMULA stat — `atomwrite --workspace . stat src/main.rs`
- FÓRMULA search — `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs' --sort path`
- FÓRMULA list — `atomwrite --workspace . list --long --depth 2 src/`
- FÓRMULA count — `atomwrite --workspace . count --by-size --top 10 src/`
- FÓRMULA extract — `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`
- FÓRMULA diff — `atomwrite --workspace . diff src/old.rs src/new.rs --unified`
- FÓRMULA diff stdin — `atomwrite --workspace . diff file.txt - --unified`


## AST e estrutura
### OBRIGATÓRIO
- Exit 1 em `transform` DEVE significar zero matches
- `transform` DEVE definir `-l/--language`
- `transform` DEVE usar `$NAME` para um nó e `$$$ARGS` para múltiplos
- Modo single-rule DEVE passar `-p/--pattern` e `-r/--rewrite` juntos
- DEVE conhecer flags de transform `--rules`, `--inline-rules`, `--verify-parse`, `--include`, `--exclude`, `--dry-run` e opts de backup
- DEVE preferir transform sobre replace quando o padrão for sintático
- `scope` DEVE definir `--language` com alias `--lang` no subcomando
- `scope` DEVE usar `--query` ou `--pattern`
- `scope` DEVE usar `--delete`, `--action` ou `--replace-with`
- Queries de scope DEVEM incluir comments, strings, fn, pub-fn, async-fn, struct, enum, trait, impl, mod, use, class, def, import, export
- `query` DEVE usar `--kinds`, `--tree`, `-Q/--query`, `--positions`, `--language`
- `outline` DEVE usar `--kind` repetível, `--positions` e `--language`
### PROIBIDO
- NUNCA aplique `scope --delete` sem dry-run
### Padrão Correto
- FÓRMULA transform — `atomwrite --workspace . transform --dry-run -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`
- FÓRMULA scope — `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`
- FÓRMULA query — `atomwrite --workspace . query --kinds src/main.rs`
- FÓRMULA outline — `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## Orquestração multi-arquivo
### batch OBRIGATÓRIO
- Entrada DEVE ser NDJSON via stdin com campo `op` obrigatório entre write, replace, delete, edit, move, copy, hash
- move e copy no manifesto DEVEM definir `"force":true` para sobrescrever
- DEVE usar `--transaction` para atomicidade total
- DEVE usar `--file <PATH>` para manifesto em disco
- DEVE conhecer flags `--dry-run`, `--batch-size`, `--input-schema` e opts de backup
### batch Padrão Correto
- FÓRMULA batch — `echo '{"op":"write","target":"a.txt","content":"olá"}' | atomwrite --workspace . batch --transaction`

### sparse recipe merge surface watch codemod semantic-search OBRIGATÓRIO
- DEVE usar `sparse list` com `--max-files` e `--max-bytes`
- DEVE usar `sparse read --paths-file` com `--head` e `--max-files`
- DEVE usar `sparse outline` para outline AST real sob orçamento
- DEVE usar `recipe list` e `recipe run --name search-replace-verify`
- Recipe search, replace e hash DEVEM excluir `*.bak.*` por padrão
- Receitas embutidas DEVEM ser `search-replace-verify` e `edit-loop-syntax-check`
- `recipe run` DEVE aceitar `--fuzzy-threshold`, `--include`, `--exclude`, `--pairs-file`, `--target`, `--syntax-check`
- DEVE usar `semantic-merge --base A --ours B --theirs C --output OUT`
- semantic-merge DEVE ser tratado como line-based e NUNCA como AST ou embedding
- Flags de merge DEVEM incluir `--fail-on-conflict`, `--write-conflict-markers`, `--no-conflict-markers`, `--expect-checksum` e opts de backup
- DEVE usar `agent-surface --format json` para o manifesto de tools
- DEVE usar `watch` com `--debounce-ms`, `--max-events`, `--idle-exit-ms`, `--checksum` e `--gitignore`
- DEVE encapsular watch com `--max-events` ou timeout global
- `watch` exige a feature de binário `watch`
- DEVE usar `codemod --rules rules.yaml --dry-run` antes de aplicar
- DEVE usar `semantic-search "query" PATH --k 20 --min-score 0.05`
- semantic-search DEVE ser Jaccard offline e NUNCA depender de embedding remoto
- DEVE passar `--index-dir` SOMENTE quando índice local for necessário
### PROIBIDO
- NUNCA rode `watch` sem teto de max-events ou timeout
- NUNCA trate zero hits de semantic-search como crash de sistema
### Padrão Correto
- FÓRMULA sparse list — `atomwrite --workspace . sparse list src/ --max-files 50 --max-bytes 524288 --include '*.rs'`
- FÓRMULA sparse read — `atomwrite --workspace . sparse read --paths-file paths.txt --head 30 --max-files 10`
- FÓRMULA sparse outline — `atomwrite --workspace . sparse outline src/ --max-files 40 --include '*.rs'`
- FÓRMULA recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern old_api --replacement new_api --path src --dry-run --fuzzy auto`
- FÓRMULA agent — `atomwrite --workspace . agent-surface --format json`
- FÓRMULA codemod — `atomwrite --workspace . codemod --rules rules.yaml --dry-run src/`
- FÓRMULA semantic-search — `atomwrite --workspace . semantic-search "optimistic lock checksum" src/ --k 15 --min-score 0.1 --index-dir .atomwrite/semantic-index`


## Descoberta e host
### OBRIGATÓRIO
- DEVE usar `doctor` para diagnosticar ambiente e dependências do host
- DEVE usar `doctor --strict` quando qualquer falha DEVE abortar o pipeline
- DEVE usar `locale` para inspecionar locale resolvido
- DEVE usar `locale --set pt-BR` ou `locale --set en` para persistir preferência
- DEVE usar `locale --clear` para limpar preferência persistida
- DEVE usar `commands` para emitir a árvore completa de comandos em JSON
- DEVE usar `commands --include-globals` para incluir flags globais no manifesto
### Padrão Correto
- FÓRMULA doctor — `atomwrite doctor --strict`
- FÓRMULA locale — `atomwrite locale --set pt-BR`
- FÓRMULA commands — `atomwrite commands --include-globals`


## Utilitários
### OBRIGATÓRIO
- DEVE usar `hash` para BLAKE3 e DEVE ler o campo `checksum` e NUNCA `value`
- DEVE conhecer flags de hash `--verify`, `--stdin`, `-r/--recursive`, `--exclude`
- DEVE usar `verify <PATH> <EXPECTED_HASH>` com exit 0 se casa e 81 se NÃO casa
- DEVE usar `backup` com `--retention` e `--output-dir`
- DEVE usar `rollback --latest --verify` em restauração crítica
- Flags de rollback DEVEM incluir `--timestamp`, `--latest`, `--verify`, `--dry-run` e `--backup` opt-in
- DEVE usar `prune-backups` com `--max-age-secs` e NUNCA `--max-age`
- `calc` DEVE avaliar expressões e unidades sem exigir workspace
- `regex` DEVE gerar padrão a partir de 3 ou mais exemplos com flags `-d`, `-w`, `-s`, `-r`, `-i`, `--no-anchors`, `--stdin`
- `wal-stats` DEVE inspecionar journals e NÃO modificar
- `wal-heal` DEVE remover journals órfãos com `--threshold-secs` e `--max-duration-ms`
- `edit-loop` DEVE aplicar N pares old e new em uma escrita
- `completions` DEVE gerar bash, zsh, fish, elvish ou powershell com `--install`
### Padrão Correto
- FÓRMULA hash — `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FÓRMULA hash recursive — `atomwrite --workspace . hash -r src/ --exclude '*.bak.*'`
- FÓRMULA verify — `atomwrite --workspace . verify src/main.rs abc123def456`
- FÓRMULA backup — `atomwrite --workspace . backup src/main.rs --retention 3`
- FÓRMULA rollback — `atomwrite --workspace . rollback src/config.toml --latest --verify`
- FÓRMULA prune — `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .`
- FÓRMULA calc — `atomwrite calc "2 hours + 30 minutes to seconds"`
- FÓRMULA regex — `atomwrite regex "abc-12" "xyz-99" "id-7" --digits`
- FÓRMULA edit-loop — `echo '[{"old":"foo","new":"bar"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FÓRMULA completions — `atomwrite completions bash --install`


## Parsing e erros
### OBRIGATÓRIO
- DEVE verificar exit code ANTES de parsear stdout
- DEVE parsear o envelope JSON quando `error` for true
- DEVE ler campos error, code, exit, message, path, error_class, retryable, suggestion, workspace, best_candidate
- `MATCH_FAILED` exit 65 DEVE significar miss de edit ou fuzzy e DEVE expor `best_candidate` para recuperação do agente
- `NO_MATCHES` exit 1 DEVE significar search ou replace com zero matches na árvore
- permanent NUNCA retenta, transient DEVE usar backoff, conflict DEVE reler estado, precondition_failed DEVE corrigir a pré-condição
- DEVE usar `suggestion` para remediação
### PROIBIDO
- NUNCA parseie stderr para dados de erro
- NUNCA retente quando `retryable` for false


## Exit codes
### OBRIGATÓRIO
- 0 sucesso
- 1 zero matches em search, replace, transform, scope e semantic-search como `NO_MATCHES`
- 2 argumento inválido ou conflito de flags
- 4 não encontrado
- 65 entrada inválida, sobrescrita grande sem `--ack-overwrite`, chave ausente ou `MATCH_FAILED`
- 78 configuração inválida ou feature ausente
- 81 checksum mismatch
- 82 state drift
- 88 erro de sintaxe tree-sitter
- 124 timeout global de `--timeout-secs`
- 126 violação do jail do workspace
- 143 SIGTERM com cancel limpo
- DEVE mapear demais códigos pelo envelope `exit` e `error_class` sem inventar semântica


## Fórmulas de execução obrigatórias
### OBRIGATÓRIO
- DEVE executar locking otimista — `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" --durability auto arq`
- DEVE executar edit com match_count — `atomwrite --workspace . edit src/f.rs --old "x" --new "y" --replace-all --fuzzy auto | jaq '{match_count, indent_adjusted, checksum}'`
- DEVE executar fuzzy replace com progresso — `atomwrite --workspace . replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run 'old_api' 'new_api' src/`
- DEVE recuperar timeout 124 aumentando `--timeout-secs` ou estreitando o caminho
- DEVE ler best_candidate após exit 65 com `jaq '.best_candidate'`
- DEVE executar recipe — `atomwrite --workspace . recipe run --name search-replace-verify --pattern OLD --replacement NEW --path src --dry-run --fuzzy auto`
- DEVE executar sparse monorepo — `atomwrite --workspace . sparse list . --max-files 100 --max-bytes 1048576 --include '*.rs'`
- DEVE executar merge line-based — `atomwrite --workspace . semantic-merge --base base.rs --ours ours.rs --theirs theirs.rs --output merged.rs --fail-on-conflict`
- DEVE executar semantic-search offline — `atomwrite --workspace . semantic-search "fsync rename atomic" src/ --k 20 --min-score 0.08 --index-dir .atomwrite/semantic-index`
- DEVE executar host readiness — `atomwrite doctor --strict && atomwrite commands --include-globals`
- DEVE tratar exit 143 como cancel limpo e NUNCA reutilizar tempfile
