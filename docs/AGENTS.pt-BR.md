# atomwrite -- Contrato de Integração com Agentes


[Read in English](AGENTS.md)


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


## 22 Subcomandos
- `read` -- lê arquivos com metadados, checksum, conteúdo opcional
- `write` -- cria ou sobrescreve arquivos atomicamente via stdin
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
- 85: FIFO detectado (named pipe não pode ser escrito atomicamente)
- 86: arquivo de dispositivo detectado (bloco ou caractere)
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
