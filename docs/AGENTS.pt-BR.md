# atomwrite -- Contrato de Integração com Agentes


[Read in English](AGENTS.md)


## O Que Há de Novo na v0.1.25

- 49 gaps resolvidos (GAP-071 a GAP-134) em 6 rodadas de auditoria e2e (~505 cenários)
- NOVO SUBCOMANDO: `verify <PATH> --checksum <BLAKE3>` — verificação dedicada de checksum (33 subcomandos no total)
- NOVO: arquivo de configuração `.atomwrite.toml` — hierarquia: CLI > env > `.atomwrite.toml` local > XDG `~/.config/atomwrite/config.toml` > defaults
- NOVAS FLAGS: `delete --older-than <DURATION>` (s/m/h/d/w), `delete --confirm` (modo preview), `replace --preserve-case` (adaptação UPPER/lower/Title), `search --pcre2`, `edit --fuzzy-threshold <FLOAT>`, `scope --action symbols|normalize`
- NOVAS FLAGS: `copy --no-reflink`, `copy --preserve-xattr`, `move --preserve-hardlinks`
- FUZZY MATCHING: Jaro-Winkler (`context_aware_jw`) adicionado na cascata de 9 estratégias; campo `diff_preview` nas respostas quando fuzzy match utilizado
- CORREÇÕES CRÍTICAS: `write --backup` não reporta mais `backup_path` fantasma (GAP-101), `set` não redireciona mais chave em scalar TOML (GAP-102), overflow de `size_delta_pct` corrigido (GAP-120)
- CORREÇÕES ALTAS: `copy --preserve` agora preserva permissões E mtime (GAP-103/133), `copy --backup`/`replace --backup` agora retêm arquivos `.bak` (GAP-104/105), erros de I/O agora emitem envelope NDJSON (GAP-098)
- CORREÇÕES MÉDIAS: campo `hash` renomeado `value` → `checksum` (GAP-107), `list --long` datas ISO 8601 (GAP-116), `outline --positions` emite byte offsets (GAP-109), `get`/`del` chave ausente retorna exit 65 INVALID_INPUT (GAP-111), `scope --query comments` captura block comments (GAP-123)
- MUDANÇA DE COMPORTAMENTO: `case` sem `--subvert` retorna exit 65 com mensagem explicativa (GAP-127)
- Validação de conflitos de flags: `append+prepend`, `fixed+regex`, `literal+regex` rejeitados no parse (GAP-093)
- 631 testes passando (10 novos), 0 clippy warnings, 0 fmt diffs


## O Que Há de Novo na v0.1.24

- 52 bugs corrigidos (GAP-2026-019 a GAP-2026-070) em auditoria e2e abrangente
- TODOS os erros agora emitem JSON estruturado no stdout com exit codes tipados
- 20 chamadas `anyhow::bail!()` convertidas para variantes `AtomwriteError`
- `delete --recursive` AGORA FUNCIONA (era no-op para diretórios)
- `hash --recursive` AGORA FUNCIONA (era aceito mas nunca percorria diretórios)
- `search --multiline` AGORA FUNCIONA (flag não era propagada)
- `replace` REJEITA padrão vazio (antes destruía arquivos silenciosamente)
- 621 testes passando (12 novos)


## O Que Há de Novo na v0.1.23

- GAP-2026-015: `allow_hyphen_values = true` em 15 campos CLI — valores com `-` aceitos como dados
- GAP-2026-016: backup habilitado por padrão em 9 structs que mutam conteúdo
- GAP-2026-017: guarda de shrink bloqueia writes >50% menores quando `--expect-checksum` ativo
- GAP-2026-018: `--old-file`/`--new-file` no edit para contornar ARG_MAX
- 609+ testes passando (31 novos)


## O Que Há de Novo na v0.1.22

- **GAP-2026-012 Frente 3 fechado** — novo sub-comando `edit-loop [PATH]` aplica N pares `{old, new}` em 1 invocação via NDJSON no stdin. Reduz 5 chamadas `edit` sequenciais (5 spawns de subprocess, 5 recapturas de checksum) para uma única escrita atômica. Suporta `--partial`, `--backup`, `--keep-backup`, `--line-ending`, `--preserve-timestamps`, `--fuzzy`, `--expect-checksum`. Veja `tests/cli_v0121_edit_loop.rs` e ADR-0039.
- **GAP-2026-013 Frente 2 fechado** — novo sub-comando `prune-backups [PATHS]...` oferece limpeza manual de arquivos `.bak.YYYYMMDD_HHMMSS` legados da v0.1.20 e anteriores. Flags: `--max-age <SECONDS>`, `--max-count <N>`, `--dry-run` (default true para segurança). Reusa `cleanup_old_backups_in` de `src/atomic.rs`. Veja `tests/cli_v0121_prune_backups.rs` e ADR-0040.
- 2 novos schemas NDJSON: `edit-loop-output.schema.json` (com `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`) e `prune-backups-output.schema.json` (com `action`, `path`, `reason`, `total`, `elapsed_ms`).
- 32 subcomandos no total (adicionados `edit-loop` e `prune-backups` aos 30 anteriores).

## O Que Há de Novo na v0.1.21

- **GAP-2026-012 fechado** — nova flag `--allow-sequential-drift` em `edit` aceita drift de checksum entre edits sequenciais no mesmo arquivo, eliminando o `STATE_DRIFT` (exit 82) falso-positivo quando o mesmo agente é dono do arquivo entre iterações. Comportamento padrão (sem a flag) inalterado: re-capturar checksum entre edits.
- **GAP-2026-013 Frente 4 fechado** — `edit` e `rollback` agora expõem flags `--backup` e `--retention` para paridade com `write` e `replace`. Default `backup: false` preserva comportamento existente.
- **GAP-2026-014 v2 fechado** — default de backup mudou para **deletar após sucesso**. Anteriormente, backups acumulavam com a política `retention: u8` (padrão 5). Agora, `--backup` cria um `.bak.{timestamp}` que é removido inline no sucesso. Nova flag opt-in `--keep-backup` preserva o backup. Operações que falham sempre preservam o backup para inspeção. Veja ADR-0038 e `tests/cli_v0121_backup_keep_flag.rs`.
- Novo padrão documentado: capturar checksum no topo de cada iteração de loop via `atomwrite read --json | jaq -r '.checksum'`, depois passar para `--expect-checksum`. Elimina STATE_DRIFT para edits sequenciais pelo mesmo agente.

## O Que Há de Novo na v0.1.15 (estendido em v0.1.18)

- G117: o `edit` multi-par `--old/--new` agora roda a mesma cascata fuzzy de 9 estratégias do caminho single, por par. Envelopes de sucesso ganham `pairs_total` e `pair_results[{index, matched, strategy, similarity}]`; falhas ganham `failed_pair_index` (exit 65, arquivo intacto). Novo opt-in `--partial` aplica os pares que casam e relata os demais.
- G118: o `write` resolve o alvo contra `--workspace` ANTES de append/prepend, detecção automática de line ending e `--expect-checksum`. Com CWD divergente, drift de checksum agora retorna exit 82 (`STATE_DRIFT`) em vez de sobrescrever silenciosamente, e alvos fora do jail retornam exit 126 cedo.
- 542 testes passando; os ADRs 0031-0037 documentam o fechamento de sete gaps entre v0.1.19-v0.1.20.

## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças v0.1.12 mais relevantes para agentes de IA que usam atomwrite como ferramenta. Todos os 13 gaps da auditoria PRD fechados em v0.1.11+v0.1.12 estão listados abaixo.

### Subcomandos Adicionados (v14 Tier 3)

- `set <PATH> <KEY_PATH> <VALUE>` — escreve um valor em um caminho dotted em um arquivo TOML ou JSON, preservando comentários e ordem das chaves via `toml_edit`. Use isto em vez de reescrever o arquivo de config inteiro (economiza tokens, preserva formatação).
- `get <PATH> <KEY_PATH>` — lê um valor em um caminho dotted. NDJSON: `{"type":"get","key_path","value","found","format"}`. Use isto em vez de ler o arquivo de config inteiro.
- `del <PATH> <KEY_PATH>` — remove uma chave. Flag `--force-missing` trata chaves ausentes como no-op success. Use isto para scripts de cleanup idempotentes.
- `case <PATHS...> --subvert OLD NEW --to <style>` — renomeia identificadores em múltiplos arquivos via `heck`. Estilos: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`.
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` — caminha um AST tree-sitter e emite nós como NDJSON. 305 linguagens via `tree-sitter-language-pack`.
- `outline <PATH> [--kind <KIND>] [--positions]` — extrai estrutura de alto nível (funções, classes, structs, enums, traits, módulos) como NDJSON.

### Flags Adicionadas (Críticas para Agentes)

- `--format raw` (alias `--raw`) em `read` — emite bytes crus para composabilidade Unix com `sed`, `awk`, `diff`, `patch`. G81.
- `--syntax-check` em `write` — invoca o parser tree-sitter (24 linguagens) para validar código. Exit 88 em erro de sintaxe. G72.
- `--max-filesize <BYTES>` em `search` — pula arquivos maiores que o limite (padrão 10 MiB). G68.
- `--max-columns <N>` em `search` — trunca matches com >N colunas (padrão 500). G68.
- `--literal` (alias `-F`) em `replace` — desabilita interpretação de regex. G66.
- `--rules <file.yaml>` e `--inline-rules <YAML>` em `transform` — multi-rule YAML para refactors em cascata. G44.
- `--batch-size <N>` em `batch` — controla pico de memória (padrão 100). G77.
- `--no-reflink` em `backup`/`copy` — desabilita CoW para filesystems sem suporte. G64.
- `--include-fifo` em `write` — permite escrita em named pipes. G56.
- `--strict-atomic` em `write` — aborta em EXDEV em vez de copy fallback. G90.
- `--lock` e `--lock-timeout <ms>` em `write`/`edit` — lock advisory via `flock`. G54.

### Códigos de Erro Adicionados (5 Novos)

- 83 `LockTimeout` (G54 lock advisory via flock excedido)
- 88 `SyntaxError` (G72 `--syntax-check` via parser tree-sitter)
- 91 `ExdevFallbackDisabled` (G90 `--strict-atomic` opt out do fallback Docker/NFS)
- 92 `CopyBackBlake3Failed` (G114 escrita in-place perdeu integridade de checksum)
- 93 `OrphanJournal` (G114 sidecar WAL deixado por crash)
- Veja OBRIGATÓRIO -- Exit Codes abaixo para a tabela completa com todos os 25 códigos.

### Recuperação de Crash (G114)

- `atomic_write` escreve `.atomwrite.journal.<target>.atomwrite.journal.json` com entradas `Started`/`Committed`.
- `recover_orphan_journals(dir)` é consultivo (sem auto-replay, sem auto-delete).
- O agente recebe eventos `{"type":"wal_recovery","orphan_journals":[...]}` e decide.

### Gaps Fechados (13 dos Top 20 do PRD)

G39 xattr, G41 binary detect (content_inspector), G54 advisory lock, G56 FIFO skip, G58 line endings, G64 reflink CoW, G66 --literal, G68 --max-filesize, G72 syntax check, G74 --threads, G76 --diff-algorithm, G77 --batch-size, G80 SIGPIPE, G81 --format raw, G90 EXDEV fallback, G116 fuzzy match, v14 Tier 3 (set/get/del/case/query/outline).

### Dependências Adicionadas

- `tree-sitter-language-pack = "1.8"` (305 linguagens, download + dynamic-loading, ~5-10MB footprint)
- `toml_edit` (preserva formatação TOML)
- `heck = "0.5"` (conversão de case)
- `reflink-copy = "0.1"` (backup CoW)
- `content_inspector = "0.2"` (detecção UTF-16)
- `xattr = "1"` (extended attributes)

### Cobertura de Testes

- **542 testes passando** (461 baseline v0.1.15 + 8 G117 edge cases v0.1.18 + 2 G118 replace pre-validation v0.1.18 + 16 incrementos cross-platform/WAL/auditoria v0.1.16-v0.1.18)
- 9 ADRs em `docs/decisions/` (0019-0027)
- 7 novos JSON schemas em `docs/schemas/` (set, get, del, case, query, outline, wal-recovery)
- Veja [docs/decisions/README.md](README.md) para decisões arquiteturais

## Por Que atomwrite
- Seu agente faz dezenas de chamadas de ferramenta para ler, escrever, buscar e substituir arquivos
- Cada chamada custa tokens, latência e espaço na janela de contexto
- atomwrite substitui tudo isso com uma CLI que lida com todas as operações de arquivo
- Toda escrita é atômica: tempfile, fsync, rename, fsync-dir
- Toda saída é NDJSON: um objeto JSON por linha no stdout
- Toda resposta inclui um checksum BLAKE3
- O checksum na resposta elimina leituras de verificação


## Economia
### Economia de Tokens
- Cada subcomando custa ~50-200 tokens de saída
- Um batch de 100 escritas custa 1 chamada bash em vez de 100 chamadas de ferramenta
- O checksum nas respostas de escrita economiza uma leitura por escrita
- Uma sessão típica de refatoração economiza 500+ chamadas de ferramenta

### Janela de Contexto
- Saída NDJSON é compacta e estruturada
- Sem formatação verbosa para humanos para interpretar
- Agentes consomem a saída diretamente sem etapas de extração


## Soberania
- atomwrite é um binário Rust standalone com zero dependências de runtime
- Sem serviço cloud, sem API key, sem acesso à rede necessário
- Todas as operações executam localmente com latência sub-milissegundo
- O agente controla todos os aspectos das operações de arquivo
- Sem vendor lock-in a qualquer framework de agente ou servidor MCP


## Agentes Compatíveis
- Claude Code (Anthropic)
- Cursor (Anysphere)
- Windsurf (Codeium)
- Aider
- OpenAI Codex CLI
- Qualquer agente que invoque comandos bash e interprete JSON


## Quickstart

```bash
cargo install atomwrite
echo "hello" | atomwrite write src/hello.txt
atomwrite read src/hello.txt
atomwrite search 'hello' src/
atomwrite replace 'hello' 'world' src/
atomwrite calc "2 horas + 30 minutos para segundos"
```


## 32 Subcomandos
- `read` -- lê arquivos com metadados, checksum, conteúdo opcional; `--format raw` (alias `--raw`) emite bytes crus para composabilidade Unix (G81); `--grep <REGEX>` filtra linhas retornadas
- `write` -- cria ou sobrescreve arquivos atomicamente via stdin; `--syntax-check` valida com tree-sitter após escrita (G72, exit 88)
- `edit` -- edita cirurgicamente por número de linha, marcador de texto ou match exato; `--fuzzy auto|off|aggressive` para matching fuzzy; `--multi` para multi-edit NDJSON
- `search` -- busca conteúdo de arquivos em paralelo (engine ripgrep); suporta `--context N`, `--max-count N`, `--invert`, `--sort path`, `--fixed`, `--word`, `--case-insensitive`, `--include`, `--exclude`
- `replace` -- substitui texto em múltiplos arquivos com escritas atômicas
- `hash` -- calcula checksums BLAKE3
- `delete` -- deleta arquivos com backup opcional
- `count` -- conta linhas, arquivos por extensão
- `diff` -- compara dois arquivos (unificado, estatística ou mudanças)
- `move` -- move ou renomeia arquivos atomicamente
- `copy` -- copia arquivos com verificação de checksum
- `list` -- lista estrutura de arquivos do projeto com metadados
- `extract` -- extrai campos de NDJSON ou colunas de texto
- `calc` -- avalia expressões matemáticas e conversões de unidades (engine fend)
- `regex` -- gera regex a partir de exemplos (engine grex)
- `transform` -- busca e reescrita estrutural por AST (ast-grep, 306 linguagens)
- `scope` -- escopo gramatical sobre categorias de código; `--delete` para remover matches; `--action upper|lower|titlecase|squeeze` para transformações de texto; `--replace-with "texto"` para substituição customizada; `--query` para consultas preparadas (comments, fn, strings, struct, etc); `--pattern` para padrões AST customizados; suporta Rust (30 queries), Python (13), JS/TS (11), Go (8)
- `backup` -- cria backups com timestamp e checksums BLAKE3; `--retention` para período de retenção, `--dry-run` para preview
- `rollback` -- restaura a partir de backup; `--timestamp` ou `--latest` para selecionar backup, `--verify` para validação de checksum, `--dry-run` para preview
- `apply` -- aplica patches do stdin com detecção automática de formato (unified diff, blocos SEARCH/REPLACE, markdown-fenced, arquivo completo); `--format` para forçar formato, `--backup` para segurança, `--dry-run` para preview
- `batch` -- executa múltiplas operações a partir de manifesto NDJSON (write, replace, delete, edit, hash, move, copy); suporta `--transaction` para tudo-ou-nada
- `completions` -- gera completions de shell
- `set` -- (v0.1.12, v14 Tier 3) escreve um valor em um caminho dotted em um arquivo TOML ou JSON via `toml_edit`; auto-coage int/bool/float/string
- `get` -- (v0.1.12, v14 Tier 3) lê um valor em um caminho dotted; NDJSON: `{"type":"get","key_path","value","found","format"}`
- `del` -- (v0.1.12, v14 Tier 3) remove uma chave; flag `--force-missing` trata chaves ausentes como no-op success
- `case` -- (v0.1.12, v14 Tier 3) renomeia identificadores em múltiplos arquivos via `heck`; estilos: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`
- `query` -- (v0.1.12, v14 Tier 3, G72) caminha um AST tree-sitter e emite nós como NDJSON; 305 linguagens via `tree-sitter-language-pack`; modos: `--kinds`, `--query <KIND>`, `-Q <KIND>`, `--tree`, `--positions`
- `outline` -- (v0.1.12, v14 Tier 3) extrai estrutura de alto nível (funções, classes, structs, enums, traits, módulos) como NDJSON
- `wal-stats` -- (v0.1.18) inspeciona estado do journal WAL para telemetria e debug; escopo via `--workspace <DIR>`; relatório NDJSON com `terminal_committed`, `terminal_aborted`, `total_bytes`, `oldest_age_secs`
- `wal-heal` -- (v0.1.18) remove journals terminais órfãos mais antigos que `--threshold-secs` (padrão 3600s); budget de wall-clock via `--max-duration-ms` (padrão 100ms)
- `edit-loop` -- (v0.1.22) aplica N pares `{old, new}` em 1 invocação via NDJSON no stdin; suporta `--partial`, `--backup`, `--keep-backup`, `--line-ending`, `--preserve-timestamps`, `--fuzzy`, `--expect-checksum`
- `prune-backups` -- (v0.1.22) limpeza manual de arquivos `.bak.YYYYMMDD_HHMMSS` legados (v0.1.20 e anteriores); flags `--max-age <SECONDS>`, `--max-count <N>`, `--dry-run` (default `true` para segurança); saída NDJSON com `path`, `reason`, `action`, `total`


## OBRIGATÓRIO -- Contrato de Saída
- stdout: SEMPRE NDJSON estruturado (um objeto JSON por linha)
- stderr: apenas logs (formato tracing, somente com `--verbose`)
- Todo objeto tem um campo discriminador `"type"`
- Flush após cada linha
- NUNCA interprete stderr como dados estruturados
- SEMPRE interprete stdout linha por linha como JSON


## OBRIGATÓRIO -- Contrato CRUD
### Create (write)
- Envie conteúdo via stdin
- Receba path, bytes_written, checksum, info de plataforma
- Use `--backup` para preservar versão anterior
- Use `--expect-checksum` para locking otimista

### Read (read)
- Receba path, content, lines, bytes, checksum, permissions, modified, kind
- Use `--stat` para pular conteúdo (apenas metadados)
- Use `--lines START:END` para leituras parciais (1-based inclusivo)
- Use `--head N` para primeiras N linhas, `--tail N` para últimas N linhas
- Use `--grep <REGEX>` para filtrar linhas retornadas às que casam com regex
- Arquivos binários são auto-detectados e conteúdo é omitido

### Update (edit, replace, transform)
- `edit` -- cirúrgico: por número de linha, marcador de texto ou match exato
- `replace` -- em massa: em múltiplos arquivos com suporte a regex
- `transform` -- estrutural: reescrita por AST em codebases
- Todos os três retornam checksums antes e depois da modificação
- Todos os três suportam `--dry-run` para preview
- `edit` e `replace` suportam `--preserve-timestamps` para dispensar a atualização de mtime (padrão: mtime é atualizado para refletir a mudança, então sistemas de build como cargo/make/cmake detectam a mudança do fonte sem `touch` manual)
- A saída NDJSON de `edit` e `replace` inclui o campo `mtime_preserved: bool` para verificar qual caminho foi tomado

### Delete (delete)
- Receba path, bytes, checksum_before
- Use `--backup` para deleção reversível
- Use `--recursive` para diretórios
- Use `--dry-run` para preview


## OBRIGATÓRIO -- Formato de Saída JSON
### Resposta de Write

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc...","elapsed_ms":1,"platform":{"fsync":"sync_data","dir_fsync":"sync_all"}}
```

### Resposta de Read

```json
{"type":"read","path":"/abs/path","content":"...","lines":10,"bytes":42,"checksum":"abc...","permissions":"rw-r--r--","modified":"2026-01-01T00:00:00Z","kind":"file","binary":false}
```

### Resposta de Edit

```json
{"type":"edit","path":"/abs/path","edits":1,"mode":"old_new","bytes_before":100,"bytes_after":110,"checksum_before":"abc...","checksum_after":"def...","lines_before":10,"lines_after":11,"elapsed_ms":1,"fuzzy":true,"strategy":"exact_whitespace","strategies_tried":2,"similarity":null}
```

### Match de Search

```json
{"type":"match","path":"/abs/path","line_number":5,"lines":"matched line content","byte_offset":120,"submatches":[{"match":"text","start":0,"end":4}]}
```

### Resultado de Replace

```json
{"type":"replace","path":"/abs/path","replacements":3,"bytes_before":100,"bytes_after":105,"checksum_before":"abc...","checksum_after":"def...","elapsed_ms":1}
```

### Envelope de Erro

```json
{"error":true,"code":"FILE_NOT_FOUND","exit":4,"message":"file not found: src/missing.rs","path":"src/missing.rs","error_class":"permanent","retryable":false,"suggestion":"verify the file path exists","workspace":null}
```

- Campo `workspace` aparece apenas em erros `WORKSPACE_JAIL` e reporta a raiz do workspace resolvida (pode ser `null`)
- `suggestion` é context-aware: sugestão de `WORKSPACE_JAIL` muda com base em se `--workspace` foi fornecido
- Veja `docs/schemas/` para definições completas de JSON Schema de todos os tipos de saída (`error-output.schema.json` define todos os 20 códigos de erro e o campo `workspace`)


## OBRIGATÓRIO -- Exit Codes
- 0: sucesso
- 1: sem matches (search/replace não encontrou nada)
- 4: arquivo não encontrado
- 13: permissão negada
- 28: disco cheio
- 30: cota excedida
- 65: entrada inválida, arquivo muito grande, ou arquivo binário
- 73: rename entre devices
- 74: erro de I/O
- 78: configuração inválida
- 81: verificação de checksum falhou (hash --verify não confere)
- 82: desvio de estado (checksum não confere em escrita)
- 83: timeout de lock (G54 lock advisory via flock, `--lock-timeout` excedido)
- 85: FIFO detectado (named pipe não pode ser escrito atomicamente)
- 86: arquivo de dispositivo detectado (bloco ou caractere)
- 88: erro de sintaxe detectado (G72 `--syntax-check` via parser tree-sitter)
- 91: fallback EXDEV desabilitado (`--strict-atomic` opt out do fallback G90 Docker/NFS)
- 92: copy-back BLAKE3 falhou (G114 escrita in-place perdeu integridade de checksum)
- 93: journal órfão recuperado (G114 sidecar WAL deixado por crash)
- 126: violação do workspace jail
- 127: symlink bloqueado
- 128: arquivo imutável
- 130: SIGINT
- 141: SIGPIPE (pipe quebrado)
- 143: SIGTERM
- 255: erro interno


## OBRIGATÓRIO -- Tratamento de Erros
- Erros emitem JSON no stdout com `error: true`
- Campos: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`
- Valores de `error_class`: `permanent`, `transient`, `conflict`, `precondition_failed`
- `retryable` é true para classes `transient` e `conflict`
- Campo `workspace` aparece apenas em erros `WORKSPACE_JAIL` e reporta a raiz do workspace resolvida
- Todas as 20 variants de erro carregam texto `suggestion` acionável (adicionado na v0.1.4, GAP 13)
- Sugestão de `WorkspaceJail` é **context-aware**: quando `--workspace` ou `ATOMWRITE_WORKSPACE` já está definido, a sugestão diz "use a path inside the workspace (<root>)" em vez de re-pedir a flag
- Sugestão de `BinaryFile` recomenda `read --stat` para leituras somente de metadados (referência phantom anterior a `--force-text` foi removida)
- Sugestão de `FileImmutable` menciona `chattr -i` (Unix) e `fsutil` (Windows)
- Sugestão de `NoMatches` orienta ampliação do padrão e revisão de filtros `--include`/`--exclude`
- Apenas `BrokenPipe` (SIGPIPE) não tem sugestão porque o erro não é acionável pelo usuário

### API de Sugestão Context-Aware (v0.1.4)
- Nova API Rust: `ErrorJson::from_error_with_context(err, &ErrorContext)` aceita proveniência de workspace
- Struct `ErrorContext` tem `workspace_provided: bool` e `workspace: Option<PathBuf>`
- `ErrorJson::from_error(err)` legacy ainda funciona e produz o mesmo output que a nova API com contexto padrão
- Consumidores programáticos podem chamar `from_error_with_context` para influenciar o texto da sugestão


## OBRIGATÓRIO -- Estratégia de Retry
### Erros Transientes (retryable: true)
- `DISK_FULL` (exit 28) -- aguarde espaço e tente novamente
- `QUOTA_EXCEEDED` (exit 30) -- aguarde reset de cota e tente novamente
- `IO_ERROR` (exit 74) -- tente novamente com backoff exponencial

### Erros de Conflito (retryable: true)
- `STATE_DRIFT` (exit 82) -- releia o arquivo, obtenha novo checksum, tente com `--expect-checksum` atualizado
- `CROSS_DEVICE` (exit 73) -- atomwrite trata internamente via cópia-depois-deleta

### Erros Permanentes (retryable: false)
- `FILE_NOT_FOUND` (exit 4) -- verifique se o path existe antes de tentar novamente
- `PERMISSION_DENIED` (exit 13) -- não tente novamente sem corrigir permissões
- `INVALID_INPUT` (exit 65) -- corrija a entrada e tente novamente
- `CONFIG_INVALID` (exit 78) -- corrija a configuração e tente novamente
- `CHECKSUM_VERIFY_FAILED` (exit 81) -- o hash passado para `--verify` não confere; releia o arquivo
- `FILE_TOO_LARGE` (exit 65) -- aumente `--max-filesize` ou processe um arquivo menor
- `WORKSPACE_JAIL` (exit 126) -- não tente novamente, path está fora do workspace
- `SYMLINK_BLOCKED` (exit 127) -- não tente novamente com symlinks desabilitados
- `IMMUTABLE_FILE` (exit 128) -- não tente novamente sem remover flag de imutabilidade
- `INTERNAL_ERROR` (exit 255) -- reporte como bug; não acionável pelo usuário

### Pré-condição Falhou (retryable: false)
- `BINARY_FILE` (exit 65) -- use modo `--stat` para ler metadados sem conteúdo
- `IMMUTABLE_FILE` (exit 128) -- remova flag de imutabilidade primeiro (chattr -i no Unix, fsutil no Windows)
- `WORKSPACE_JAIL` (exit 126) -- ajuste limite de `--workspace`
- `FIFO_DETECTED` (exit 85) -- pule este arquivo ou use redirecionamento de stdin
- `DEVICE_FILE` (exit 86) -- pule este arquivo ou use redirecionamento de stdin


## OBRIGATÓRIO -- Flags Globais
- `--workspace <PATH>` -- SEMPRE passe para restringir operações à raiz do projeto
- `--verbose` / `-v` -- habilita tracing no stderr
- `--quiet` / `-q` -- suprime saída não essencial
- `--color <auto|always|never>` -- controla saída colorida
- `--no-color` -- desabilita saída colorida (equivalente a `--color never`)
- `--no-gitignore` -- não respeita regras do .gitignore
- `--hidden` -- inclui arquivos e diretórios ocultos
- `--follow-symlinks` -- segue links simbólicos
- `--threads <N>` / `-j <N>` -- threads paralelas (0 = todos os cores)
- `--max-filesize <BYTES>` -- ignora arquivos maiores que o limite
- `--timeout <SECONDS>` -- timeout global de operação (0 = sem timeout, padrão 0). Use para limitar buscas longas, batches e operações replace
- `--json-schema` -- emite JSON schema da saída do subcomando
- `--lang <LOCALE>` -- substitui o locale de exibição (en, pt-BR) via env `ATOMWRITE_LANG`


## PROIBIDO -- Armadilhas Comuns
- NUNCA interprete stderr como dados; contém apenas logs de tracing
- NUNCA assuma que exit code 1 é erro fatal; significa zero matches em search
- NUNCA omita `--workspace` ao executar como agente
- NUNCA omita `--dry-run` antes de operações batch destrutivas
- NUNCA use expressões sem aspas com `calc`; o shell vai interpolar
- NUNCA ignore `checksum_before` e `checksum_after` em respostas de edit/replace
- NUNCA tente novamente erros `permanent` ou `precondition_failed` sem corrigir a causa


## OBRIGATÓRIO -- Orçamento de Tokens
- Cada subcomando: 1 chamada bash, ~50-200 tokens de saída
- Modo batch: 1 chamada bash para N operações
- Checksum na resposta elimina 1 leitura de verificação por escrita
- Uma sessão típica de agente economiza 500+ chamadas versus operações individuais


## OBRIGATÓRIO -- Locking Otimista
- Leia um arquivo e capture seu `checksum` da resposta
- Passe o checksum via `--expect-checksum` na próxima escrita ou edição
- Se o arquivo mudou entre leitura e escrita, atomwrite retorna exit 82 (`STATE_DRIFT`)
- Releia o arquivo para obter o checksum atual e tente novamente
- Isso previne atualizações perdidas em workflows concorrentes de agentes


## Operações de Backup v0.1.21

- Por padrão, backups são DELETADOS após a operação completar com sucesso
- Use `--keep-backup` para preservar o backup após sucesso
- Backups de operações que FALHARAM são sempre preservados para inspeção
- `cleanup_old_backups_in` mantém N backups apenas para casos com `--keep-backup`
- Novo sub-comando `prune-backups` para limpeza manual de backups legados


## Padrão Sequencial v0.1.21

- 5 edits sequenciais SEM re-captura = 4 falham com `STATE_DRIFT` (exit 82)
- 5 edits COM re-captura = todos passam (padrão canônico)
- 5 edits COM `--allow-sequential-drift` = todos passam com warning
- `edit-loop` aplica N pares em 1 invocação (sem STATE_DRIFT interno)


## Subcomandos v0.1.22

Dois novos sub-comandos fecham as frentes rejeitadas em planos anteriores (`gaps.md` linhas 82-83, 201):

### `edit-loop` — N Pares em 1 Invocação

- **Quando usar**: aplicar lote de transformações textuais em um arquivo onde hoje você invocaria `edit` N vezes em loop shell
- **Input**: NDJSON via stdin com um objeto `{old, new}` por linha
- **Comportamento**: lê o arquivo UMA vez, aplica todos os pares em memória, escreve atomicamente UMA vez
- **Vantagem vs shell loop**: 1 invocação CLI em vez de N; 1 read + 1 write em vez de N+N; sem `STATE_DRIFT` interno entre pares

```bash
# Aplicar 3 pares em 1 invocação
printf '%s\n' \
  '{"old":"v0_1_20","new":"v0_1_22"}' \
  '{"old":"foo","new":"bar"}' \
  '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace . edit-loop src/version.rs

# Com backup preservado (linha do tempo forense)
printf '%s\n' '{"old":"foo","new":"bar"}' \
  | atomwrite --workspace . edit-loop --backup --keep-backup src/foo.rs

# Modo best-effort com --partial
printf '%s\n' '{"old":"existe","new":"X"}' '{"old":"ausente","new":"Y"}' \
  | atomwrite --workspace . edit-loop --partial src/foo.rs
```

- **Schema NDJSON**: `docs/schemas/edit-loop-output.schema.json` — campos `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`
- **ADR**: `docs/decisions/0039-edit-loop-helper.md`

### `prune-backups` — Limpeza Manual de Legados

- **Quando usar**: operadores que atualizaram de v0.1.20 herdam siblings `.bak.YYYYMMDD_HHMMSS` que v0.1.21 não cria mais (e portanto não limpa mais automaticamente)
- **Algoritmo**: varre `[PATHS]...` em busca de arquivos `.bak.YYYYMMDD_HHMMSS`, aplica filtros de idade ou contagem
- **Segurança**: `--dry-run` é DEFAULT `true` para prevenir perda acidental de dados

```bash
# Default --dry-run true: lista o que SERIA removido
atomwrite --workspace . prune-backups --max-age 86400 .

# Remove backups mais antigos que 24 horas
atomwrite --workspace . prune-backups --max-age 86400 --dry-run false .

# Mantém apenas os 3 backups mais recentes por diretório
atomwrite --workspace . prune-backups --max-count 3 --dry-run false .

# Pipeline CI: afirma zero backups órfãos após limpeza
atomwrite --workspace . prune-backups --max-age 0 --dry-run false . \
  && fd '*.bak.*' . | wc -l | jaq -e '. == 0'
```

- **Schema NDJSON**: `docs/schemas/prune-backups-output.schema.json` — campos `path`, `reason`, `action`, `total`, `elapsed_ms`
- **ADR**: `docs/decisions/0040-prune-backups-subcommand.md`
- **Nota operacional**: limpeza retroativa automática foi explicitamente rejeitada (ver ADR-0038 Addendum); operadores devem rodar `prune-backups` sob demanda
