---
name: atomwrite
description: >-
  Esta skill DEVE ativar quando a LLM precisar de escrita atômica, leitura, edição, busca, substituição, refatoração AST, scoping gramatical, hash BLAKE3, lotes transacionais, fuzzy Jaro-Winkler, cálculo, regex, backup ou rollback de arquivos. Cobre os 33 subcomandos da CLI Rust atomwrite (read, write, edit, search, replace, hash, verify, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, batch, backup, rollback, apply, set, get, del, case, query, outline, wal-stats, wal-heal, edit-loop, prune-backups, completions). Saída SEMPRE NDJSON. Pipeline atômico tempfile-fsync-rename. Triggers — atomwrite, escrita atômica, edição cirúrgica, BLAKE3, checksum, locking otimista, WAL journal, tree-sitter, ast-grep, scoping gramatical
---


# atomwrite


## Identidade Principal
- stdout SEMPRE emite NDJSON (um objeto JSON por linha)
- stderr serve APENAS para logs e tracing
- Toda escrita passa pelo pipeline atômico tempfile, fsync, rename
- Checksum BLAKE3 presente em TODA resposta de write e read
- SEMPRE passar `--workspace <DIR>` para definir raiz do jail
- Todos os caminhos resolvem relativos ao workspace
- Flag `--json` aceita mas ignorada (saída SEMPRE NDJSON)
- NUNCA parsear stderr como dados estruturados
- NUNCA assumir que exit 1 é erro (search, replace, transform, scope usam exit 1 para zero matches)
- NUNCA escrever arquivos fora do jail do workspace
- `--backup` é `true` por padrão em 9 structs (write, edit, replace, apply, batch, set, del, case, transform)
- USAR `--no-backup` para desabilitar backup quando performance for prioridade
- Arquivo `.atomwrite.toml` define defaults por projeto com hierarquia CLI > env > local > XDG > defaults


## Operações de Escrita (write)
- SEMPRE enviar conteúdo via stdin
- USAR `--backup --retention N` para sobrescritas destrutivas
- USAR `--no-backup` para desabilitar backup
- USAR `--keep-backup` para preservar backup após sucesso
- USAR `--expect-checksum <BLAKE3>` para locking otimista
- USAR `--allow-shrink` para permitir truncamento quando `--expect-checksum` ativo (shrink-guard bloqueia redução maior que 50%)
- USAR `--allow-empty-stdin` quando stdin estiver vazio (guard intencional)
- USAR `--no-checksum-when-empty` para pular `--expect-checksum` quando stdin vazio
- USAR `--dry-run` antes de escritas destrutivas
- USAR `--append` para anexar ao final
- USAR `--prepend` para inserir no início
- USAR `--max-size <BYTES>` para limitar tamanho do stdin
- USAR `--line-ending lf|crlf|cr|auto` para normalizar quebras de linha
- USAR `--preserve-timestamps` para manter mtime original
- USAR `--require-backup` para ABORTAR se backup não estiver ativo e alvo existir
- USAR `--auto-rotate` para forçar backup quando alvo modificado nas últimas 24h
- USAR `--confirm` para prompt interativo em arquivos maiores que 100KB
- USAR `--risk-threshold <PERCENT>` para telemetria de risco (padrão 255 = desabilitado)
- USAR `--syntax-check` para validar sintaxe via tree-sitter antes de escrever (exit 88 em falha)
- USAR `--wal-policy auto|always|never` para política WAL
- FÓRMULA escrita `echo "conteúdo" | atomwrite --workspace . write alvo.rs`
- FÓRMULA locking otimista `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" arq`
- FÓRMULA append `echo "linha" | atomwrite --workspace . write --append arq`
- NUNCA escrever sem `--workspace`
- NUNCA passar conteúdo como argumento CLI


## Operações de Leitura (read)
- USAR `read` para conteúdo com metadados (checksum, size, lines, mode)
- USAR `--stat` para metadados sem corpo
- USAR `--lines 1:50` para intervalo de linhas
- USAR `--line N` com `-C/--context N` para linha única com contexto
- USAR `--head N` para primeiras N linhas
- USAR `--tail N` para últimas N linhas
- USAR `--format raw` para conteúdo puro sem envelope JSON
- USAR `--grep <REGEX>` para filtrar linhas por regex
- USAR `--verify-checksum <BLAKE3>` para verificação de integridade (exit 81 em mismatch)
- Campo `mode` indica variante usada (full, head, tail, line, lines, grep, stat)
- FÓRMULA leitura parcial `atomwrite --workspace . read --head 20 src/main.rs`
- FÓRMULA metadados `atomwrite --workspace . read --stat src/main.rs`


## Operações de Edição (edit)
- USAR `--old "texto" --new "texto"` para substituição exata (repetível para múltiplos pares)
- Multi-par roda cascata fuzzy de 9 estratégias por par incluindo Jaro-Winkler (context_aware_jw)
- Resposta inclui pairs_total e pair_results (index, matched, strategy, similarity, diff_preview, old, new)
- Par falho aborta lote inteiro por padrão (all-or-nothing)
- USAR `--partial` para aplicar pares que casam e relatar os demais
- USAR `--fuzzy auto|off|aggressive` para controlar matching aproximado
- USAR `--fuzzy-threshold <FLOAT>` para sensibilidade configurável (0.0 a 1.0)
- USAR `--after-line N` para inserir após linha
- USAR `--before-line N` para inserir antes da linha
- USAR `--range N:M` para substituir intervalo
- USAR `--delete-range N:M` para deletar intervalo
- USAR `--after-match "texto"` para inserir após match
- USAR `--before-match "texto"` para inserir antes do match
- USAR `--between "início" "fim"` para substituir entre marcadores
- USAR `--multi` para múltiplas edições via NDJSON no stdin
- USAR `--expect-checksum <BLAKE3>` para locking otimista
- USAR `--allow-sequential-drift` para pipeline sequencial sem recaptura de checksum
- USAR `--line-ending lf|crlf|cr|auto` para normalizar quebras
- USAR `--preserve-timestamps` para manter mtime
- USAR `--backup`, `--no-backup`, `--keep-backup`, `--retention N`
- USAR `--old-file <PATH>` e `--new-file <PATH>` para conteúdo grande (evita ARG_MAX)
- `--old-file` e `--old` são mutuamente exclusivos (clap emite exit 2)
- Cross-mixing (`--old` + `--new-file`) retorna exit 65
- USAR `--wal-policy auto|always|never`
- NUNCA fazer pipe de edit para jaq sem verificação (usar `jaq -e` ou `${PIPESTATUS[0]}`)
- FÓRMULA edição `atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo"`
- FÓRMULA multi-par `atomwrite --workspace . edit src/main.rs --old "a" --new "b" --old "c" --new "d" | jaq -e '.pair_results'`
- FÓRMULA via arquivo `atomwrite --workspace . edit src/main.rs --old-file old.txt --new-file new.txt`
- FÓRMULA inserir após linha `echo "nova" | atomwrite --workspace . edit src/main.rs --after-line 10`


## Operações de Busca (search)
- Exit code 1 significa zero resultados (NÃO é erro)
- USAR `-g/--include '*.rs'` e `--exclude '*.log'` para filtrar
- USAR `-C/--context N` para linhas de contexto
- USAR `-F/--fixed` para busca literal
- USAR `-e/--regex` para forçar modo regex
- USAR `-w/--word` para limite de palavra
- USAR `-i/--case-insensitive`, `-S/--smart-case`
- USAR `-c/--count` para contagem por arquivo
- USAR `-l/--files` para listar nomes de arquivo
- USAR `-m/--max-count N` para limitar matches por arquivo
- USAR `-U/--multiline` para matching multilinha
- USAR `-P/--pcre2` para engine PCRE2 (exit 65 se feature não compilada)
- USAR `--invert` para linhas que NÃO casam
- USAR `--sort path|modified|created|none` para ordenar (path garante ordenação global determinística)
- USAR `--max-filesize <BYTES>` para pular arquivos grandes
- USAR `--max-columns <N>` para truncar linhas largas
- USAR `--no-begin-end` para suprimir eventos begin/end em arquivos sem matches
- USAR `--include-fifo` para FIFO/named pipes (NUNCA em diretórios não confiáveis)
- FÓRMULA busca `atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs'`
- FÓRMULA contagem `atomwrite --workspace . search 'unwrap' src/ --count --sort path`


## Operações de Substituição (replace)
- Exit code 1 para zero matches
- SEMPRE usar `--dry-run` primeiro
- USAR `--regex`, `-w/--word`, `-F/--literal`
- USAR `-g/--include`, `--exclude` para filtrar
- USAR `--preview` para diff sem escrita
- USAR `-n/--max-replacements N` para limitar por arquivo
- USAR `--expect-checksum <BLAKE3>` para locking
- USAR `--backup`, `--no-backup`, `--keep-backup`
- USAR `--preserve-timestamps` para manter mtime
- USAR `--preserve-case` para preservar case (UPPER, lower, Title)
- FÓRMULA `atomwrite --workspace . replace --dry-run 'antigo' 'novo' src/`
- FÓRMULA regex `atomwrite --workspace . replace --regex 'v\d+\.\d+' 'v2.0' src/ --include '*.toml'`


## Operações de Transformação AST (transform)
- Exit code 1 para zero matches
- SEMPRE especificar `-l/--lang` para linguagem alvo
- USAR `$NAME` para captura de nó único e `$$$ARGS` para múltiplos
- 306 linguagens suportadas via ast-grep
- USAR `-p/--pattern` e `-r/--rewrite` (AMBOS obrigatórios no modo single-rule)
- USAR `--dry-run`, `--backup`, `--no-backup`
- USAR `--include`, `--exclude` para filtrar
- USAR `--rules <PATH>` para regras de arquivo YAML/JSON
- USAR `--inline-rules <JSON>` para regras inline
- USAR `--verify-parse` para validar parse tree
- FÓRMULA `atomwrite --workspace . transform -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/`


## Operações de Scoping Gramatical (scope)
- Exit code 1 para zero matches
- SEMPRE especificar `-l/--language` para linguagem alvo (aceita `--lang` como alias)
- USAR `--query` para queries preparadas
- USAR `--pattern` para padrões AST customizados
- USAR `--delete` para remover conteúdo casado
- USAR `--action upper|lower|titlecase|squeeze|symbols|normalize`
- `--action symbols` converte operadores ASCII para Unicode
- `--action normalize` normaliza texto para NFC
- USAR `--replace-with "texto"` para substituição customizada
- USAR `-g/--include`, `--exclude`, `--backup`, `--dry-run`
- Queries Rust: comments, strings, fn, pub-fn, async-fn, unsafe-fn, struct, pub-struct, enum, pub-enum, trait, impl, mod, use, closure, unsafe, attribute, derive, return, match, if-let, while-let, for, loop, const, static, type-alias, macro-rules
- Queries Python: comments, strings, class, def, async-def, lambda, import, from-import, with, for, while, decorator, try-except
- Queries JS/TS: comments, strings, fn, arrow-fn, async-fn, class, import, export, try-catch, const, let
- Queries Go: fn, struct, interface, goroutine, defer, import, const, var
- Limitações conhecidas: `test-fn` e `doc-comment` estão DESABILITADAS (retornam InvalidInput)
- FÓRMULA `atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run`


## Operações em Lote (batch)
- Entrada NDJSON via stdin (campo `op` obrigatório: write, replace, delete, edit, move, copy, hash)
- move/copy requerem `"force":true` para sobrescrever
- USAR `--file <PATH>` para ler manifesto de arquivo
- USAR `--transaction` para atomicidade total com rollback automático
- USAR `--dry-run`, `--keep-backup`, `--batch-size <N>`, `--input-schema`
- FÓRMULA `echo '{"op":"write","target":"a.txt","content":"olá"}' | atomwrite --workspace . batch --transaction`


## Operações de Hash e Verificação (hash, verify)
- hash calcula checksums BLAKE3 de um ou mais arquivos
- USAR `--verify <BLAKE3>` para verificar contra hash esperado
- USAR `--stdin` para hashear conteúdo do stdin
- USAR `--recursive` (`-r`) para hashear diretórios
- Campo de saída é `checksum` (NÃO `value`)
- verify aceita `<PATH> <EXPECTED_HASH>` como argumentos posicionais
- verify retorna exit 0 quando casa, exit 81 quando não casa
- FÓRMULA hash `atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'`
- FÓRMULA verificação `atomwrite --workspace . verify src/main.rs abc123def456`


## Operações de Remoção (delete)
- USAR `--backup --retention N` para manter backups
- USAR `--recursive` (`-r`) para diretórios (traversa via WalkBuilder, remove subdiretórios vazios)
- USAR `--include`, `--exclude` para filtrar
- USAR `--yes` (`-y`) para pular confirmação
- USAR `--dry-run` ou `--confirm` para pré-visualizar
- USAR `--older-than <DURATION>` para filtrar por idade (s/m/h/d/w)
- FÓRMULA `atomwrite --workspace . delete --older-than 7d --yes tmp/`


## Operações de Diff (diff)
- USAR `--unified` para formato unified
- USAR `--stat` para estatísticas resumidas
- USAR `-C/--context N` para linhas de contexto (padrão 3)
- USAR `--algorithm myers|patience|lcs` (padrão patience)
- FÓRMULA `atomwrite --workspace . diff src/old.rs src/new.rs --unified`


## Operações de Mover e Copiar (move, copy)
- USAR `--force` para sobrescrever destino
- USAR `--dry-run`, `--backup`
- copy aceita `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`
- move aceita `--preserve-hardlinks`, `--retention N`
- FÓRMULA mover `atomwrite --workspace . move src/old.rs src/new.rs`
- FÓRMULA copiar `atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/`


## Operações de Listagem, Contagem e Extração (list, count, extract)
- list: `-g/--include`, `--exclude`, `--long`, `--depth N`, `--count-by-ext`, `--all`
- count: `--by-extension`, `--by-size` com `--top N`, `--include`, `--exclude`
- extract: campos posicionais (path, line_number), `--delimiter <SEP>`, `--stdin`
- FÓRMULA listagem `atomwrite --workspace . list --long --depth 2 src/`
- FÓRMULA contagem `atomwrite --workspace . count --by-size --top 10 src/`
- FÓRMULA extração `atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number`


## Operações de Cálculo e Regex (calc, regex)
- calc: expressão entre aspas, `--stdin` para ler do stdin (stateless, sem --workspace)
- regex: gera regex a partir de exemplos (3+ para precisão)
- regex: `-d/--digits`, `-w/--words`, `-s/--spaces`, `-r/--repetitions`
- regex: `-i/--case-insensitive`, `--no-anchors`, `--stdin`
- FÓRMULA cálculo `atomwrite calc "2 hours + 30 minutes to seconds"`
- FÓRMULA regex `atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" --digits`


## Operações de Backup e Rollback (backup, rollback)
- backup: `--retention N` (padrão 5), `--output-dir <DIR>`, `--dry-run`
- rollback: `--latest` (padrão), `--timestamp YYYYMMDD_HHMMSS` (aceita prefix match com milissegundos)
- rollback: `--verify` para checksum BLAKE3 após restauração
- rollback: `--backup`, `--keep-backup`, `--retention N`, `--dry-run`
- FÓRMULA backup `atomwrite --workspace . backup src/main.rs --retention 3`
- FÓRMULA rollback `atomwrite --workspace . rollback src/config.toml --verify`


## Operações de Patch (apply)
- Detecta formato: unified diff, SEARCH/REPLACE, markdown-fenced, full file
- USAR `--format auto|unified|search-replace|full|markdown` para forçar formato
- USAR `--backup`, `--no-backup`, `--keep-backup`, `--retention N`, `--dry-run`
- FÓRMULA `echo "conteúdo" | atomwrite --workspace . apply src/file.txt --format full`


## Operações de Config (set, get, del)
- set: escreve valor em TOML ou JSON via dotted path (auto-coerce bool/int/float/string)
- get: lê valor via dotted path (exit 65 INVALID_INPUT se chave não existe)
- del: remove chave via dotted path
- USAR `--backup`, `--no-backup`, `--preserve-timestamps` em set e del
- USAR `--force-missing` em del para suceder silenciosamente se chave ausente
- NUNCA usar set/get/del em texto puro (apenas TOML e JSON)
- FÓRMULA set `atomwrite --workspace . set Cargo.toml package.version 0.2.0`
- FÓRMULA get `atomwrite --workspace . get config.toml database.pool.max`
- FÓRMULA del `atomwrite --workspace . del --force-missing config.toml features.experimental`


## Operações de Case, Query e Outline (case, query, outline)
- case: converte case de identificadores (snake, camel, pascal, kebab, screaming-snake)
- case: `--subvert OLD NEW` é OBRIGATÓRIO (sem ele retorna exit 65)
- case: `--to <STYLE>`, `--backup`, `--no-backup`, `--preserve-timestamps`, `--dry-run`
- NUNCA rodar case sem --dry-run em codebase grande
- query: inspeciona AST via tree-sitter (24 linguagens)
- query: `--kinds` lista node kinds (resposta é NDJSON stream, um objeto por kind)
- query: `--tree` para árvore completa
- query: `-Q/--query <PATTERN>` para S-expression
- query: `--positions`, `--language <LANG>`
- outline: extrai estrutura de alto nível (funções, structs, enums)
- outline: `--kind <KIND>` (repetível), `--positions`, `--language <LANG>`
- FÓRMULA case `atomwrite --workspace . case --to kebab --subvert API API --dry-run src/`
- FÓRMULA query `atomwrite --workspace . query --kinds src/main.rs`
- FÓRMULA outline `atomwrite --workspace . outline --kind function_item --positions src/main.rs`


## Operações WAL e Manutenção (wal-stats, wal-heal, edit-loop, prune-backups, completions)
- wal-stats: inspeciona estado do journal WAL (consultivo, não modifica)
- wal-heal: remove journals terminais órfãos mais antigos que threshold
- wal-heal: `--threshold-secs <N>` (padrão 3600), `--max-duration-ms <N>` (padrão 100)
- edit-loop: aplica N pares {old, new} em 1 invocação (aceita JSON array E NDJSON)
- edit-loop: resposta inclui pair_results com campos old e new para cada par processado
- edit-loop: `--backup`, `--no-backup`, `--keep-backup`, `--retention N`, `--line-ending`, `--syntax-check <LANG>`, `--allow-sequential-drift`
- prune-backups: limpa backups legados por idade ou quantidade
- prune-backups: `--max-age-secs <N>` (NÃO --max-age), `--max-count <N>`, `--dry-run`
- completions: gera completions para bash, zsh, fish, elvish, powershell
- completions: `--install` para instalar automaticamente
- FÓRMULA edit-loop `echo '[{"old":"foo","new":"bar"},{"old":"baz","new":"qux"}]' | atomwrite --workspace . edit-loop src/foo.rs`
- FÓRMULA prune `atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run /path/`
- FÓRMULA completions `atomwrite completions bash --install`


## Tratamento de Erros
- VERIFICAR exit code ANTES de parsear stdout
- PARSEAR stdout JSON quando `error: true` para detalhes estruturados
- Campos do envelope: error, code, exit, message, path, error_class, retryable, suggestion, workspace
- Estratégia por error_class: permanent (NUNCA retentar), transient (backoff exponencial), conflict (reler estado primeiro), precondition_failed (corrigir pré-condição)
- RETENTAR SOMENTE quando `retryable: true`
- USAR campo `suggestion` para remediação acionável (context-aware)
- NUNCA ignorar exit codes não-zero (exceto exit 1 em search/replace/transform/scope)
- NUNCA parsear stderr para dados de erro
- NUNCA retentar quando `retryable: false`


## Códigos de Saída
- 0 sucesso
- 1 sem resultados (search, replace, transform, scope = zero matches, NÃO é erro)
- 4 não encontrado (arquivo ou diretório)
- 13 permissão negada
- 28 disco cheio
- 30 cota excedida
- 65 entrada inválida (argumentos malformados, pattern vazio, chave inexistente em get/del)
- 73 cross-device (mover entre filesystems)
- 74 erro de I/O
- 78 configuração inválida
- 81 verificação de checksum falhou (BLAKE3 mismatch em read --verify-checksum ou verify)
- 82 state drift (mismatch de checksum em locking otimista)
- 83 timeout de lock
- 85 FIFO detectado
- 86 arquivo de dispositivo detectado
- 88 erro de sintaxe (tree-sitter)
- 91 fallback EXDEV desabilitado
- 92 verificação BLAKE3 de copy-back falhou
- 93 journal órfão detectado (consultivo)
- 126 violação do jail do workspace
- 127 symlink bloqueado (alvo fora do workspace)
- 128 arquivo imutável
- 130 SIGINT
- 141 SIGPIPE
- 143 SIGTERM
- 255 erro interno


## Flags Globais
- `--workspace <DIR>` raiz do jail (OBRIGATÓRIO para operações de arquivo)
- `--max-filesize <BYTES>` tamanho máximo aceito (padrão 1 GiB)
- `-j/--threads <N>` threads paralelos (0 = todos os cores)
- `--timeout-secs <SECONDS>` timeout global (padrão 0 = sem timeout)
- `--color auto|always|never`
- `--no-color` desabilitar cores na saída
- `--no-gitignore` não respeitar .gitignore
- `--hidden` incluir arquivos ocultos
- `--follow-symlinks` seguir links simbólicos
- `--locale <LANG>` forçar idioma de mensagens (pt-BR, en)
- `--json-schema` emitir JSON Schema do subcomando e sair
- `--no-auto-heal` pular wal-heal automático na inicialização
- `-v` info, `-vv` debug, `-vvv` trace
- `-q` error, `-qq` off


## Fórmulas Prontas de Pipelines
- Locking otimista: `CS=$(atomwrite --workspace . read arq | jaq -r '.checksum') && echo "novo" | atomwrite --workspace . write --expect-checksum "$CS" arq`
- Buscar e extrair: `atomwrite --workspace . search 'TODO' src/ --include '*.rs' | atomwrite extract path line_number`
- Hash para auditoria: `atomwrite --workspace . hash src/main.rs src/lib.rs | jaq -r '.checksum'`
- Lote transacional: `atomwrite --workspace . batch --file ops.ndjson --transaction`
- Config TOML: `atomwrite --workspace . get config.toml db.pool.max && atomwrite --workspace . set config.toml db.pool.max 20`
- Verificação pré-commit: `atomwrite --workspace . write --syntax-check src/lib.rs < novo.rs`
- Edits sequenciais: `echo '[{"old":"a","new":"b"},{"old":"c","new":"d"}]' | atomwrite --workspace . edit-loop --backup src/foo.rs`
- Refatoração AST: `atomwrite --workspace . transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- Substituição em massa: `atomwrite --workspace . replace --dry-run 'antigo' 'novo' src/`
- Backup e rollback: `atomwrite --workspace . backup src/config.toml && atomwrite --workspace . rollback src/config.toml --verify`
- Verificação de integridade: `atomwrite --workspace . verify src/main.rs $(atomwrite --workspace . hash src/main.rs | jaq -r '.checksum')`
- Escopo gramatical: `atomwrite --workspace . scope src/ --lang rust --query pub-fn --dry-run`
