---
name: atomwrite
description: >-
  Esta skill ensina a utilizar atomwrite, CLI Rust de operacoes atomicas de arquivo com 33 subcomandos (read, write, edit, search, replace, hash, verify, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, backup, rollback, apply, batch, completions, set, get, del, case, query, outline, wal-stats, wal-heal, edit-loop, prune-backups). DEVE ser ativada quando a LLM precisar escrever, ler, editar, buscar, substituir, refatorar AST, gerar regex, calcular, verificar BLAKE3, lotes transacionais, fuzzy Jaro-Winkler, scoping gramatical, backup ou rollback. Saida SEMPRE NDJSON. Pipeline atomico tempfile-fsync-rename
---


# atomwrite


## Identidade Principal
- stdout SEMPRE emite NDJSON (um objeto JSON por linha)
- stderr serve APENAS para logs e tracing
- Toda escrita passa pelo pipeline atomico tempfile, fsync, rename
- Checksum BLAKE3 presente em TODA resposta de write e read
- SEMPRE passar `--workspace <DIR>` para definir raiz do jail
- Todos os caminhos resolvem relativos ao workspace
- Flag `--json` aceita mas ignorada (saida SEMPRE NDJSON)
- NUNCA parsear stderr como dados estruturados
- NUNCA assumir que exit 1 e erro (search, replace, transform, scope usam exit 1 para zero matches)
- NUNCA escrever arquivos fora do jail do workspace
- `--backup` e `true` por padrao em 9 structs (write, edit, replace, apply, batch, set, del, case, transform)
- USAR `--no-backup` para desabilitar backup quando performance for prioridade
- Arquivo `.atomwrite.toml` define defaults por projeto com hierarquia CLI > env > local > XDG > defaults


## Operacoes de Escrita (write)
- SEMPRE enviar conteudo via stdin
- USAR `--backup --retention N` para sobrescritas destrutivas
- USAR `--no-backup` para desabilitar backup
- USAR `--keep-backup` para preservar backup apos sucesso
- USAR `--expect-checksum <BLAKE3>` para locking otimista
- USAR `--allow-shrink` para permitir truncamento quando `--expect-checksum` ativo (shrink-guard bloqueia reducao maior que 50%)
- USAR `--allow-empty-stdin` quando stdin estiver vazio (guard intencional)
- USAR `--dry-run` antes de escritas destrutivas
- USAR `--append` para anexar ao final
- USAR `--prepend` para inserir no inicio
- USAR `--max-size <BYTES>` para limitar tamanho do stdin
- USAR `--line-ending lf|crlf|cr|auto` para normalizar quebras de linha
- USAR `--preserve-timestamps` para manter mtime original
- USAR `--require-backup` para ABORTAR se backup nao estiver ativo e alvo existir
- USAR `--auto-rotate` para forcar backup quando alvo modificado nas ultimas 24h
- USAR `--confirm` para prompt interativo em arquivos maiores que 100KB
- USAR `--risk-threshold <PERCENT>` para telemetria de risco (padrao 255 = desabilitado)
- USAR `--syntax-check` para validar sintaxe via tree-sitter antes de escrever (exit 88 em falha)
- USAR `--wal-policy auto|always|never` para politica WAL
- FORMULA escrita `echo "conteudo" | atomwrite --workspace . write alvo.rs`
- FORMULA locking otimista `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" arq`
- FORMULA append `echo "linha" | atomwrite --workspace . write --append arq`
- NUNCA escrever sem `--workspace`
- NUNCA passar conteudo como argumento CLI


## Operacoes de Leitura (read)
- USAR `read` para conteudo com metadados (checksum, size, lines, mode)
- USAR `--stat` para metadados sem corpo
- USAR `--lines 1:50` para intervalo de linhas
- USAR `--line N` com `--context N` para linha unica com contexto
- USAR `--head N` para primeiras N linhas
- USAR `--tail N` para ultimas N linhas
- USAR `--format raw` para conteudo puro sem envelope JSON
- USAR `--grep <REGEX>` para filtrar linhas por regex
- USAR `--verify-checksum <BLAKE3>` para verificacao de integridade (exit 81 em mismatch)
- Campo `mode` indica variante usada (full, head, tail, line, lines, grep, stat)
- FORMULA leitura parcial `atomwrite --workspace . read --head 20 src/main.rs`
- FORMULA metadados `atomwrite --workspace . read --stat src/main.rs`


## Operacoes de Edicao (edit)
- USAR `--old "texto" --new "texto"` para substituicao exata (repetivel para multiplos pares)
- Multi-par roda cascata fuzzy de 9 estrategias por par incluindo Jaro-Winkler (context_aware_jw)
- Resposta inclui pairs_total e pair_results (index, matched, strategy, similarity, diff_preview)
- Par falho aborta lote inteiro por padrao (all-or-nothing)
- USAR `--partial` para aplicar pares que casam e relatar os demais
- USAR `--fuzzy auto|off|aggressive` para controlar matching aproximado
- USAR `--fuzzy-threshold <FLOAT>` para sensibilidade configuravel (0.0 a 1.0)
- USAR `--after-line N` para inserir apos linha
- USAR `--before-line N` para inserir antes da linha
- USAR `--range N:M` para substituir intervalo
- USAR `--delete-range N:M` para deletar intervalo
- USAR `--after-match "texto"` para inserir apos match
- USAR `--before-match "texto"` para inserir antes do match
- USAR `--between "inicio" "fim"` para substituir entre marcadores
- USAR `--multi` para multiplas edicoes via NDJSON no stdin
- USAR `--expect-checksum <BLAKE3>` para locking otimista
- USAR `--allow-sequential-drift` para pipeline sequencial sem re-captura de checksum
- USAR `--line-ending lf|crlf|cr|auto` para normalizar quebras
- USAR `--preserve-timestamps` para manter mtime
- USAR `--backup`, `--no-backup`, `--keep-backup`, `--retention N`
- USAR `--old-file <PATH>` e `--new-file <PATH>` para conteudo grande (evita ARG_MAX)
- `--old-file` e `--old` sao mutuamente exclusivos (clap emite exit 2)
- Cross-mixing (`--old` + `--new-file`) retorna exit 65
- USAR `--wal-policy auto|always|never`
- NUNCA fazer pipe de edit para jaq sem verificacao (usar `jaq -e` ou `${PIPESTATUS[0]}`)
- FORMULA edicao `atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo"`
- FORMULA multi-par `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --old "c" --new "d" | jaq -e '.pair_results'`
- FORMULA via arquivo `atomwrite --workspace . edit src/main.rs --old-file old.txt --new-file new.txt`
- FORMULA inserir apos linha `echo "nova" | atomwrite --workspace . edit src/main.rs --after-line 10`


## Operacoes de Busca (search)
- Exit code 1 significa zero resultados (NAO e erro)
- USAR `--include '*.rs'` e `--exclude '*.log'` para filtrar
- USAR `--context N` para linhas de contexto
- USAR `--fixed` (`-F`) para busca literal
- USAR `--regex` (`-e`) para forcar modo regex
- USAR `--word` (`-w`) para limite de palavra
- USAR `--case-insensitive` (`-i`), `--smart-case` (`-S`)
- USAR `--count` (`-c`) para contagem por arquivo
- USAR `--files` (`-l`) para listar nomes de arquivo
- USAR `--max-count N` (`-m`) para limitar matches por arquivo
- USAR `--multiline` (`-U`) para matching multilinha
- USAR `--invert` para linhas que NAO casam
- USAR `--sort path|modified|created|none` para ordenar (path garante ordenacao global deterministica)
- USAR `--max-filesize <BYTES>` para pular arquivos grandes
- USAR `--max-columns <N>` para truncar linhas largas
- USAR `--no-begin-end` para suprimir eventos begin/end em arquivos sem matches
- USAR `--pcre2` para engine PCRE2 (exit 65 se feature nao compilada)
- USAR `--include-fifo` para FIFO/named pipes (NUNCA em diretorios nao confiaveis)
- FORMULA busca `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs'`
- FORMULA contagem `atomwrite --workspace . search 'unwrap' src/ --count --sort path`


## Operacoes de Substituicao (replace)
- Exit code 1 para zero matches
- SEMPRE usar `--dry-run` primeiro
- USAR `--regex`, `--word`, `--literal` (`-F`)
- USAR `--include`, `--exclude` para filtrar
- USAR `--preview` para diff sem escrita
- USAR `--max-replacements N` (`-n`) para limitar por arquivo
- USAR `--expect-checksum <BLAKE3>` para locking
- USAR `--backup`, `--no-backup`, `--keep-backup`
- USAR `--preserve-timestamps` para manter mtime
- USAR `--preserve-case` para preservar case (UPPER, lower, Title)
- FORMULA `atomwrite --workspace . replace --dry-run 'antigo' 'novo' src/`
- FORMULA regex `atomwrite --workspace . replace --regex 'v\d+\.\d+' 'v2.0' src/ --include '*.toml'`


## Operacoes de Transformacao AST (transform)
- Exit code 1 para zero matches
- SEMPRE especificar `--lang` (`-l`) para linguagem alvo
- USAR `$NAME` para captura de no unico e `$$$ARGS` para multiplos
- 306 linguagens suportadas via ast-grep
- USAR `--pattern` e `--rewrite` (AMBOS obrigatorios no modo single-rule)
- USAR `--dry-run`, `--backup`, `--no-backup`
- USAR `--include`, `--exclude` para filtrar
- USAR `--rules <PATH>` para regras de arquivo YAML/JSON
- USAR `--inline-rules <JSON>` para regras inline
- USAR `--verify-parse` para validar parse tree
- FORMULA `atomwrite --workspace . transform -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## Operacoes de Scoping Gramatical (scope)
- Exit code 1 para zero matches
- SEMPRE especificar `--lang` para linguagem alvo
- USAR `--query` para queries preparadas (suporta return types, generics e trait impls)
- USAR `--pattern` para padroes AST customizados
- USAR `--delete` para remover conteudo
- USAR `--action upper|lower|titlecase|squeeze|symbols|normalize`
- `--action symbols` converte operadores ASCII para Unicode
- `--action normalize` normaliza texto para NFC
- USAR `--replace-with "texto"` para substituicao
- USAR `--include`, `--exclude`, `--backup`, `--dry-run`
- Queries Rust: comments, doc-comment, strings, fn, pub-fn, async-fn, unsafe-fn, test-fn, struct, pub-struct, enum, pub-enum, trait, impl, mod, use, closure, unsafe, attribute, derive, return, match, if-let, while-let, for, loop, const, static, type-alias, macro-rules
- Queries Python: comments, strings, class, def, async-def, lambda, import, from-import, with, for, while, decorator, try-except
- Queries JS/TS: comments, strings, fn, arrow-fn, async-fn, class, import, export, try-catch, const, let
- Queries Go: fn, struct, interface, goroutine, defer, import, const, var
- FORMULA `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`


## Operacoes em Lote (batch)
- Entrada NDJSON via stdin (campo `op` obrigatorio: write, replace, delete, edit, move, copy, hash)
- move/copy requerem `"force":true` para sobrescrever
- USAR `--file <PATH>` para ler manifesto de arquivo
- USAR `--transaction` para atomicidade total com rollback automatico
- USAR `--dry-run`, `--keep-backup`, `--batch-size <N>`, `--input-schema`
- FORMULA `echo '{"op":"write","target":"a.txt","content":"ola"}' | atomwrite --workspace . batch --transaction`


## Operacoes de Hash e Verificacao (hash, verify)
- hash calcula checksums BLAKE3 de um ou mais arquivos
- USAR `--verify <BLAKE3>` para verificar contra hash esperado
- USAR `--stdin` para hashear conteudo do stdin
- USAR `--recursive` (`-r`) para hashear diretorios
- Campo de saida e `checksum` (NAO `value`)
- verify aceita `<PATH> <EXPECTED_HASH>` como argumentos posicionais
- verify retorna exit 0 quando casa, exit 81 quando nao casa
- FORMULA hash `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FORMULA verificacao `atomwrite --workspace . verify src/main.rs abc123def456`


## Operacoes de Remocao (delete)
- USAR `--backup --retention N` para manter backups
- USAR `--recursive` (`-r`) para diretorios (traversa via WalkBuilder, remove subdiretorios vazios)
- USAR `--include`, `--exclude` para filtrar
- USAR `--yes` (`-y`) para pular confirmacao
- USAR `--dry-run` ou `--confirm` para pre-visualizar
- USAR `--older-than <DURATION>` para filtrar por idade (s/m/h/d/w)
- FORMULA `atomwrite --workspace . delete --older-than 7d --yes tmp/`


## Operacoes de Diff (diff)
- USAR `--unified` para formato unified
- USAR `--stat` para estatisticas resumidas
- USAR `--context N` (`-C`) para linhas de contexto (padrao 3)
- USAR `--algorithm myers|patience|lcs` (padrao patience)
- FORMULA `atomwrite --workspace . diff src/old.rs src/new.rs --unified`


## Operacoes de Mover e Copiar (move, copy)
- USAR `--force` para sobrescrever destino
- USAR `--dry-run`, `--backup`
- copy aceita `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`
- move aceita `--preserve-hardlinks`, `--retention N`
- FORMULA mover `atomwrite --workspace . move src/old.rs src/new.rs`
- FORMULA copiar `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`


## Operacoes de Listagem, Contagem e Extracao (list, count, extract)
- list: `--include`, `--exclude`, `--long`, `--depth N`, `--count-by-ext`, `--all`
- count: `--by-extension`, `--by-size` com `--top N`, `--include`, `--exclude`
- extract: campos posicionais (path, line_number), `--delimiter <SEP>`, `--stdin`
- FORMULA listagem `atomwrite --workspace . list --long --depth 2 src/`
- FORMULA contagem `atomwrite --workspace . count --by-size --top 10 src/`
- FORMULA extracao `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`


## Operacoes de Calculo e Regex (calc, regex)
- calc: expressao entre aspas, `--stdin` para ler do stdin (stateless, sem --workspace)
- regex: gera regex a partir de exemplos (3+ para precisao)
- regex: `--digits` (`-d`), `--words` (`-w`), `--spaces` (`-s`), `--repetitions` (`-r`)
- regex: `--case-insensitive` (`-i`), `--no-anchors`, `--stdin`
- FORMULA calculo `atomwrite calc "2 hours + 30 minutes to seconds"`
- FORMULA regex `atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" --digits`


## Operacoes de Backup e Rollback (backup, rollback)
- backup: `--retention N` (padrao 5), `--output-dir <DIR>`, `--dry-run`
- rollback: `--latest` (padrao), `--timestamp YYYYMMDD_HHMMSS` (aceita prefix match com milissegundos)
- rollback: `--verify` para checksum BLAKE3 apos restauracao
- rollback: `--backup`, `--keep-backup`, `--retention N`, `--dry-run`
- FORMULA backup `atomwrite --workspace . backup src/main.rs --retention 3`
- FORMULA rollback `atomwrite --workspace . rollback src/config.toml --verify`


## Operacoes de Patch (apply)
- Detecta formato: unified diff, SEARCH/REPLACE, markdown-fenced, full file
- USAR `--format auto|unified|search-replace|full|markdown` para forcar formato
- USAR `--backup`, `--no-backup`, `--keep-backup`, `--retention N`, `--dry-run`
- FORMULA `echo "conteudo" | atomwrite --workspace . apply src/file.txt --format full`


## Operacoes de Config (set, get, del)
- set: escreve valor em TOML ou JSON via dotted path (auto-coerce bool/int/float/string)
- get: le valor via dotted path (exit 65 INVALID_INPUT se chave nao existe)
- del: remove chave via dotted path
- USAR `--backup`, `--no-backup`, `--preserve-timestamps` em set e del
- USAR `--force-missing` em del para suceder silenciosamente se chave ausente
- NUNCA usar set/get/del em texto puro (apenas TOML e JSON)
- FORMULA set `atomwrite --workspace . set Cargo.toml package.version 0.2.0`
- FORMULA get `atomwrite --workspace . get config.toml database.pool.max`
- FORMULA del `atomwrite --workspace . del --force-missing config.toml features.experimental`


## Operacoes de Case, Query e Outline (case, query, outline)
- case: converte case de identificadores (snake, camel, pascal, kebab, screaming-snake)
- case: `--subvert OLD NEW` e OBRIGATORIO (sem ele retorna exit 65)
- case: `--to <STYLE>`, `--backup`, `--no-backup`, `--preserve-timestamps`, `--dry-run`
- NUNCA rodar case sem --dry-run em codebase grande
- query: inspeciona AST via tree-sitter (24 linguagens)
- query: `--kinds` lista node kinds (resposta e NDJSON stream, um objeto por kind)
- query: `--tree` para arvore completa
- query: `--query <PATTERN>` (`-Q`) para S-expression
- query: `--positions`, `--language <LANG>`
- outline: extrai estrutura de alto nivel (funcoes, structs, enums)
- outline: `--kind <KIND>` (repetivel), `--positions`, `--language <LANG>`
- FORMULA case `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`
- FORMULA query `atomwrite --workspace . query --kinds src/main.rs`
- FORMULA outline `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## Operacoes WAL e Manutencao (wal-stats, wal-heal, edit-loop, prune-backups, completions)
- wal-stats: inspeciona estado do journal WAL (consultivo, nao modifica)
- wal-heal: remove journals terminais orfaos mais antigos que threshold
- wal-heal: `--threshold-secs <N>` (padrao 3600), `--max-duration-ms <N>` (padrao 100)
- edit-loop: aplica N pares {old, new} em 1 invocacao (aceita JSON array E NDJSON)
- edit-loop: `--backup`, `--no-backup`, `--keep-backup`, `--retention N`, `--line-ending`, `--syntax-check`, `--allow-sequential-drift`
- prune-backups: limpa backups legados por idade ou quantidade
- prune-backups: `--max-age-secs <N>` (NAO --max-age), `--max-count <N>`, `--dry-run`
- completions: gera completions para bash, zsh, fish, elvish, powershell
- completions: `--install` para instalar automaticamente
- FORMULA edit-loop `echo '[{"old":"foo","new":"bar"},{"old":"baz","new":"qux"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FORMULA prune `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run /path/`
- FORMULA completions `atomwrite completions bash --install`


## Tratamento de Erros
- VERIFICAR exit code ANTES de parsear stdout
- PARSEAR stdout JSON quando `error: true` para detalhes estruturados
- Campos do envelope: error, code, exit, message, path, error_class, retryable, suggestion, workspace
- Estrategia por error_class: permanent (NUNCA retentar), transient (backoff exponencial), conflict (reler estado primeiro), precondition_failed (corrigir pre-condicao)
- RETENTAR SOMENTE quando `retryable: true`
- USAR campo `suggestion` para remediacao acionavel (context-aware)
- NUNCA ignorar exit codes nao-zero (exceto exit 1 em search/replace/transform/scope)
- NUNCA parsear stderr para dados de erro
- NUNCA retentar quando `retryable: false`


## Codigos de Saida
- 0 sucesso
- 1 sem resultados (search, replace, transform, scope = zero matches, NAO e erro)
- 4 nao encontrado (arquivo ou diretorio)
- 13 permissao negada
- 28 disco cheio
- 30 cota excedida
- 65 entrada invalida (argumentos malformados, pattern vazio, chave inexistente em get/del)
- 73 cross-device (mover entre filesystems)
- 74 erro de I/O
- 78 configuracao invalida
- 81 verificacao de checksum falhou (BLAKE3 mismatch em read --verify-checksum ou verify)
- 82 state drift (mismatch de checksum em locking otimista)
- 83 timeout de lock
- 85 FIFO detectado
- 86 arquivo de dispositivo detectado
- 88 erro de sintaxe (tree-sitter)
- 91 fallback EXDEV desabilitado
- 92 verificacao BLAKE3 de copy-back falhou
- 93 journal orfao detectado (consultivo)
- 126 violacao do jail do workspace
- 127 symlink bloqueado (alvo fora do workspace)
- 128 arquivo imutavel
- 130 SIGINT
- 141 SIGPIPE
- 143 SIGTERM
- 255 erro interno


## Flags Globais
- `--workspace <DIR>` raiz do jail (OBRIGATORIO para operacoes de arquivo)
- `--max-filesize <BYTES>` tamanho maximo aceito (padrao 1 GiB)
- `--threads <N>` / `-j` threads paralelos (0 = todos os cores)
- `--timeout <SECONDS>` timeout global (padrao 0 = sem timeout)
- `--color auto|always|never`
- `--no-gitignore` nao respeitar .gitignore
- `--hidden` incluir arquivos ocultos
- `--follow-symlinks` seguir links simbolicos
- `-v` info, `-vv` debug, `-vvv` trace
- `-q` error, `-qq` off


## Formulas Prontas de Pipelines
- Locking otimista: `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" arq`
- Buscar e extrair: `atomwrite --workspace . search 'TODO' src/ --include '*.rs' | atomwrite extract path line_number`
- Hash para auditoria: `atomwrite --workspace . hash src/main.rs src/lib.rs | jaq -r '.checksum'`
- Lote transacional: `atomwrite --workspace . batch --file ops.ndjson --transaction`
- Config TOML: `atomwrite --workspace . get config.toml db.pool.max && atomwrite --workspace . set config.toml db.pool.max 20`
- Verificacao pre-commit: `atomwrite --workspace . write --syntax-check src/lib.rs < novo.rs`
- Edits sequenciais: `echo '[{"old":"a","new":"b"},{"old":"c","new":"d"}]' | atomwrite --workspace . edit-loop --backup src/foo.rs`
- Refatoracao AST: `atomwrite --workspace . transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- Substituicao em massa: `atomwrite --workspace . replace --dry-run 'antigo' 'novo' src/`
- Backup e rollback: `atomwrite --workspace . backup src/config.toml && atomwrite --workspace . rollback src/config.toml --verify`
