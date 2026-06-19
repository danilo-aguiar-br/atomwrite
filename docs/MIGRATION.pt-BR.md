# Guia de Migração do atomwrite


[Read in English](MIGRATION.md)


## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças relevantes para migração em v0.1.12. Veja a seção [v0.1.11 para v0.1.12](#v0111-para-v0112) abaixo para o guia de migração da v0.1.12 e a seção [v0.1.12 para v0.1.15 (Atual)](#v0112-para-v0115-atual) para a transição mais recente.

### Novos Subcomandos (6)

- `set` — escreve um valor em um caminho dotted em TOML/JSON. Use em vez de reescrever o arquivo inteiro.
- `get` — lê um valor em um caminho dotted. Use em vez de ler o arquivo inteiro.
- `del` — remove uma chave em um caminho dotted. `--force-missing` para scripts idempotentes.
- `case` — renomeia identificadores em 5 estilos de case via `heck`.
- `query` — caminha um AST tree-sitter. 305 linguagens.
- `outline` — extrai definições top-level. 305 linguagens.

Todos os 6 subcomandos são aditivos. Nenhum código existente é afetado.

### Novas Flags (15 Total)

- `write --syntax-check` (G72)
- `write --include-fifo` (G56)
- `write --strict-atomic` (G90)
- `write --lock` e `--lock-timeout` (G54)
- `read --format raw` e `--raw` (G81)
- `read --head N`, `--tail N`, `--line N`, `--grep <REGEX>`
- `search --max-filesize` e `--max-columns` (G68)
- `replace --literal` e `-F` (G66)
- `transform --rules` e `--inline-rules` (G44)
- `batch --batch-size` (G77)
- `backup/copy --no-reflink` (G64)
- `diff --diff-algorithm patience|myers|lcs` (G76)

Todas as flags são aditivas com valores padrão que preservam o comportamento de v0.1.11.

### Novos Códigos de Erro (5)

- 83 `LockTimeout` (G54)
- 88 `SyntaxError` (G72)
- 91 `ExdevFallbackDisabled` (G90)
- 92 `CopyBackBlake3Failed` (G114)
- 93 `OrphanJournal` (G114)

Total de códigos de erro: 25 (eram 20 em v0.1.4). Todos os novos códigos têm mensagens bilíngues e sugestões acionáveis.

### Dependências Adicionadas

- `tree-sitter-language-pack = "1.8"` — 305 linguagens, dynamic loading
- `toml_edit` — preserva formatação TOML
- `heck = "0.5"` — conversão de case
- `reflink-copy = "0.1"` — backup CoW
- `content_inspector = "0.2"` — detecção UTF-16
- `xattr = "1"` — extended attributes

Todas aditivas. Nenhuma dependência existente removida.

### Mudanças de Comportamento

- Nenhuma. v0.1.12 é totalmente retrocompatível com v0.1.11.
- Novos subcomandos são opt-in: scripts existentes continuam funcionando.
- Valores padrão das novas flags preservam o comportamento de v0.1.11.
- Adições de código de erro não mudam exit codes existentes.

### Ação de Migração

- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.12"`
- Novos subcomandos e flags são opt-in. Nenhuma mudança de código necessária para chamadores existentes.
- Veja a seção [v0.1.12 para v0.1.15 (Atual)](#v0112-para-v0115-atual) para os passos de migração mais recentes.

### Cobertura de Testes

- 542 testes passando (445 na v0.1.12 + 2 na v0.1.14 + 8 G117 + 6 G118 na v0.1.15 + 40 v0.1.16-v0.1.18 cross-platform + 21 v0.1.19 + 20 v0.1.20)
- 9 ADRs em `docs/decisions/` (0019-0027)
- 7 novos JSON schemas em `docs/schemas/`
- Veja [docs/decisions/README.md](README.md) para decisões arquiteturais

## Versão Atual
- atomwrite está na v0.1.23
- Este documento cobre migração de v0.1.0 a v0.1.15, com seções detalhadas para v0.1.12 a v0.1.15, v0.1.11 a v0.1.12 e grandes transições anteriores
- Veja as seções abaixo para mudanças aditivas e breaking changes em cada versão


## v0.1.12 para v0.1.15 (Atual)

### Aditivo (G117)

- O `edit` multi-par `--old`/`--new` roda a cascata fuzzy completa de 9 estratégias por par (antes era somente exato).
- O envelope de sucesso ganha `pairs_total` e `pair_results`; o envelope de erro ganha `failed_pair_index`, `pairs_total`, `pair_results` (continua `INVALID_INPUT`, exit 65, arquivo intacto).
- Novo opt-in `edit --partial` aplica os pares que casam e relata os demais; zero matches mapeia para `NO_MATCHES` (exit 1) sem escrita.

### Correção Comportamental (G118)

- O `write` resolve o alvo via jail do workspace ANTES de append/prepend, detecção automática de line ending e `--expect-checksum`.
- Com CWD fora do workspace: `--append`/`--prepend` não truncam mais, `--line-ending auto` volta a detectar o arquivo existente, e um `--expect-checksum` divergente retorna exit 82 (`STATE_DRIFT`) em vez de sobrescrever silenciosamente. Alvos fora do jail retornam exit 126 cedo.
- Breaking apenas para chamadores que dependiam da sobrescrita silenciosa do bug.

### Ação de Migração (v0.1.15)

- Atualize o pin de versão: `cargo install atomwrite --locked --version "^0.1.15"`
- MSRV inalterada em Rust 1.88. Nenhuma mudança de código necessária para chamadores em conformidade.
- Veja o ADR-0026 e o ADR-0027 em `docs/decisions/` para a justificativa completa.

## v0.1.3 para v0.1.4 (Histórico)

### v0.1.4 (Histórico)

#### Corrigido (Compilação Windows - GAP 14)

Três erros de compilação em blocos `#[cfg(windows)]` impediam `cargo install atomwrite` de funcionar no Windows 10/11 desde v0.1.3:

- `E0433` em `src/atomic.rs:404` — `persist_with_retry` usava `AtomwriteError::PermissionDenied` sem importá-lo. O `use crate::error::AtomwriteError;` agora está gated sob `#[cfg(windows)]` para evitar `unused_imports` em Linux/macOS.
- `E0507` em `src/atomic.rs:387` — `persist_with_retry` recebia `&NamedTempFile` mas chamava `temp.persist()` que requer ownership. Assinatura mudou para `fn persist_with_retry(mut temp: NamedTempFile, target: &Path) -> Result<()>`. O branch de retry agora recupera o arquivo de `e.file` (PersistError expõe o NamedTempFile original em falha).
- `E0308` em `src/platform.rs:116` — `GetStdHandle` retorna `HANDLE` que é `*mut c_void` no windows-sys 0.61. O literal `0` é `usize`; comparar raw pointer com inteiro é erro de tipo. Substituído `handle != 0` por `!handle.is_null()`. A comparação `handle != INVALID_HANDLE_VALUE` permanece inalterada porque `INVALID_HANDLE_VALUE` já é tipada como `HANDLE` (`-1i32 as _`).

Impacto da migração:
- Nenhuma mudança de API ou comportamento para usuários Linux ou macOS
- Usuários Windows: `cargo install atomwrite` agora funciona; sem necessidade de patches manuais ou compilar do código fonte
- Toda a semântica de escrita atômica, exit codes, output NDJSON, e flags CLI permanecem inalteradas

#### Corrigido (Sugestões de Erro - GAP 13)

Sugestões de erro agora são context-aware e acionáveis:

- Sugestão de `WorkspaceJail` se adapta: quando o usuário forneceu `--workspace` (ou `ATOMWRITE_WORKSPACE`), a sugestão agora diz "use a path inside the workspace (<root>)" em vez de re-pedir a flag.
- Todas as 20 variants de erro agora carregam texto `suggestion`. Anteriormente 6 variants (InvalidInput, Io, ConfigInvalid, FileImmutable, NoMatches, InternalError) retornavam `None`. Apenas `BrokenPipe` (SIGPIPE, não-acionável) permanece sem sugestão.
- Referência phantom à flag `--force-text` removida da sugestão de BinaryFile.
- Novo struct `ErrorContext` (`workspace_provided`, `workspace`) e API `ErrorJson::from_error_with_context()`. A versão legacy `from_error()` é preservada.

Novas sugestões:
- `FileImmutable` — menciona `chattr -i` (Unix) e `fsutil` (Windows) para limpar o atributo imutável
- `NoMatches` — orienta o usuário a ampliar o padrão e revisar filtros `--include`/`--exclude`
- `InternalError` — solicita report de bug com o contexto da razão
- `InvalidInput` — pede ao usuário para revisar o input e checar argumentos
- `Io` — aponta para a mensagem de erro de I/O subjacente
- `ConfigInvalid` — aponta para a razão da configuração

Impacto da migração:
- Sem quebra de API: `ErrorJson::from_error()` ainda funciona com o mesmo output
- Se você parseia o campo `suggestion` de envelopes de erro, o texto pode diferir para as variants afetadas. A semântica (dica acionável) é preservada ou melhorada.

#### Adicionado (Validação Cross-Platform - GAP 14)

- `tests/cross_compile_check.rs` — 3 testes de cross-compile gated para `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, e `x86_64-pc-windows-msvc`. Falha em qualquer regressão de `E0433`, `E0308`, ou `E0507` em blocos `cfg(windows)`. Execute com `cargo test --test cross_compile_check -- --ignored` antes de releases que tocam código Windows-only.
- `output::write_error_json_with_context()` — propaga `ErrorContext` do parser CLI até o output NDJSON.
- `docs/INSTALL.md` e `docs/INSTALL.pt-BR.md` — pré-requisitos de instalação Windows 10/11, comandos `cargo install`, e troubleshooting.

#### Referência

Veja as seções "GAP 13" e "GAP 14" em `gaps.md` para a análise completa de causa raiz e rationale de design.


## v0.1.2 para v0.1.3

### v0.1.3 (Anterior)

#### Mudado (BREAKING)

##### Escrita atômica padrão não preserva mais o mtime

Os subcomandos `edit` e `replace` agora atualizam o tempo de modificação do arquivo (mtime) para o momento em que a escrita é concluída, em vez de preservar o mtime original. Este é o padrão correto para sistemas de build que usam mtime para detectar mudanças em código fonte.

Antes da v0.1.3:
- `edit` e `replace` hardcodavam `AtomicWriteOptions::preserve_timestamps = true`
- O mtime do arquivo era restaurado para o valor que tinha ANTES do rename atômico
- Sistemas de build que comparam mtime do fonte com mtime do dep-info (cargo, make, cmake) pulavam o rebuild silenciosamente quando o fonte parecia mais antigo que o binário
- Workaround: agentes tinham que rodar `touch <file>` após `atomwrite edit` para forçar o cargo a detectar a mudança

Depois da v0.1.3:
- `edit` e `replace` usam `AtomicWriteOptions::preserve_timestamps = false` por padrão
- O mtime é definido para "agora" automaticamente, então o cargo detecta a mudança sem intervenção manual
- Agentes não precisam mais de `touch` após editar um arquivo fonte Rust antes de `cargo build`
- Para workflows de backup, snapshot ou builds reproduzíveis que precisam do timestamp original, passe a nova flag `--preserve-timestamps`

Consumidores afetados (agentes LLM):
- Se você constrói código após editar com atomwrite, o novo padrão corrige um no-op silencioso "Finished in 0.29s" onde o cargo pula o rebuild
- Se você depende do comportamento antigo de preservação de mtime, adicione `--preserve-timestamps` às suas invocações de `edit` e `replace`

Campo de diagnóstico:
- A saída NDJSON de `edit` e `replace` agora inclui `mtime_preserved: bool` para que você verifique qual caminho foi tomado (true = timestamp mantido, false = timestamp atualizado)

#### Adicionado (Consciência de Sistema de Build)

- Flag `--preserve-timestamps` em `edit` e `replace` para voltar ao comportamento de preservação de mtime da v0.1.2
- Campo `mtime_preserved` nas respostas NDJSON de `EditOutput` e `ReplaceResult`

#### Referência

Veja `gaps.md` seção "Atomic Edit Preserva mtime E Quebra Detecção De Mudança Pelo Cargo" (GAP 12) para a análise completa de causa raiz e justificativa de design.


## v0.1.1 para v0.1.2

### v0.1.2

#### Correções (Bug Fixes)

##### `batch --transaction` rollback agora é real
Anteriormente, arquivos criados por operações `write` durante uma transação nunca eram removidos no rollback. Agora:
- `RollbackEvent` inclui `files_restored`, `files_removed` e `total_reverted`
- Novos arquivos criados no meio da transação são deletados no rollback
- Arquivos pré-existentes modificados são restaurados do backup

Consumidores afetados (agentes LLM): confiem no evento NDJSON `rollback` — o estado do disco corresponde a ele.

##### `replace` não infla mais contadores em violações de jail
Anteriormente, `total_replacements` era incrementado para arquivos fora do jail do workspace. Agora:
- Validação do jail roda ANTES do incremento do contador
- Violações emitem `ReplaceErrorEvent` com `kind: jail_violation`, `error_class: permanent`, `retryable: false`
- `total_replacements` reflete apenas matches dentro do jail

##### Eventos paralelos do `search` agora são agrupados por path
O walker paralelo não intercala mais eventos `begin`/`match`/`end` de arquivos diferentes. Sequências de eventos para um dado path agora são contíguas na saída NDJSON.

##### `scope --delete` em comentários Rust não deixa mais espaço em branco órfão
A query preparada `comments` para Rust agora casa whitespace trailing, então a deleção produz código limpo.

##### `search` com regex inválido emite envelope JSON estruturado
Padrões inválidos agora falham com `AtomwriteError::InvalidInput` que é serializado como `error.json` no stdout, não stderr cru.

##### `batch --file <PATH>` agora é funcional
A flag agora realmente lê o manifesto NDJSON de um arquivo (validado contra jail do workspace) em vez de ser ignorada.

##### `backup --output-dir <DIR>` agora é respeitado
A flag agora coloca backups no diretório customizado (criado se faltar) e poda backups antigos naquele diretório.

##### Mensagem de erro de WORKSPACE_JAIL corrigida
A sugestão enganosa "use an absolute path" agora é "set --workspace <root> or export ATOMWRITE_WORKSPACE=<path>".

#### Adicionado (Funcionalidades Agent-First)

- Flag global `--timeout <SECONDS>` para execução com limite de tempo (0 = sem timeout, padrão 0)
- `read --grep <REGEX>` filtra para retornar apenas linhas que casam com regex
- `completions --install` para instalar scripts de completions no diretório de dados XDG

#### Mudado (Dependências)

- `nix` 0.29 → 0.31
- `signal-hook` 0.3 → 0.4
- `windows-sys` 0.59 → 0.61
- `rust-i18n` 3 → 4

#### Cross-Platform

atomwrite v0.1.2 agora compila no macOS arm64 (Apple Silicon) e macOS x86_64. A syscall `posix_fadvise` agora está corretamente restrita a `target_os = "linux"` apenas.


## O Que Muda
### Compromisso SemVer
- atomwrite segue Semantic Versioning 2.0.0
- Versão MAJOR: mudanças que quebram flags CLI, exit codes ou schema de saída JSON
- Versão MINOR: novos subcomandos, novas flags, novos campos JSON (apenas aditivos)
- Versão PATCH: correções de bug sem mudanças de API

### O Que Conta Como Quebra
- Remover ou renomear uma flag CLI
- Mudar o significado de um exit code
- Remover um campo da saída JSON
- Mudar o tipo de um campo JSON existente
- Renomear um campo JSON
- Mudar o comportamento padrão de uma flag existente

### O Que NÃO Quebra
- Adicionar um novo subcomando
- Adicionar uma nova flag opcional
- Adicionar um novo campo na saída JSON
- Adicionar um novo exit code
- Melhorar mensagens de erro
- Melhorias de performance

### Estabilizações Planejadas para 1.0
- Schemas de saída NDJSON para todos os 30 subcomandos
- Atribuições de exit codes
- Strings de código de erro (`FILE_NOT_FOUND`, `STATE_DRIFT`, etc)
- Nomes e comportamento de flags globais
- Formato do manifesto de batch

### Potenciais Mudanças Quebrando Antes do 1.0
- Nomes de campos na saída NDJSON podem mudar antes do 1.0
- Novos campos obrigatórios podem ser adicionados aos tipos de saída
- Valores de exit codes podem mudar para alinhar com sysexits
- O formato de saída do `--json-schema` pode evoluir


## Template de Migração Passo a Passo
- Use este template ao migrar entre versões

### Passo 1 -- Leia o Changelog
- Revise o `CHANGELOG.md` para a versão alvo
- Identifique todas as entradas marcadas como BREAKING

### Passo 2 -- Verifique Seus Comandos
- Liste toda invocação de atomwrite no seu agente ou scripts
- Compare cada flag contra as notas de migração

### Passo 3 -- Compare JSON Schemas
- Execute `atomwrite <subcommand> --json-schema` com ambas as versões
- Identifique adições, remoções e mudanças de tipo nos campos

### Passo 4 -- Atualize o Parsing de JSON
- Atualize seus filtros `jaq` ou código de parsing JSON
- Trate novos campos com graciosidade (mudanças aditivas)
- Remova referências a campos deletados

### Passo 5 -- Atualize o Tratamento de Exit Codes
- Revise blocos `case` ou `if` que tratam exit codes
- Adicione tratamento para novos exit codes
- Remova tratamento para exit codes depreciados

### Passo 6 -- Teste em Modo Dry-Run
- Execute toda invocação modificada com `--dry-run` primeiro
- Verifique se a estrutura de saída corresponde ao esperado

### Passo 7 -- Deploy
- Atualize o binário via `cargo install atomwrite`
- Execute sua suite de testes
- Verifique o comportamento do agente em ambiente de staging


## Template de Mudanças de JSON Schema
- Use este formato para documentar mudanças de campo entre versões

### Antes (vX.Y.Z)

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc..."}
```

### Depois (vX.Y.Z)

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc...","new_field":"value"}
```

### Ação de Migração
- Novo campo `new_field` é aditivo e OPCIONAL
- Nenhuma ação necessária para consumidores existentes
- Atualize o parsing para consumir o novo campo se útil


## v0.1.0 para v0.1.1
### Resumo
- ZERO mudanças que quebram compatibilidade
- Todos os comandos, flags e saída JSON da v0.1.0 permanecem inalterados
- Nenhuma ação de migração necessária para consumidores existentes

### Comportamentos Corrigidos (falhas silenciosas corrigidas)
- `search --include` e `search --exclude` agora realmente filtram arquivos (eram silenciosamente ignorados)
- `replace --include` e `replace --exclude` agora realmente filtram arquivos
- `transform --include` e `transform --exclude` agora realmente filtram arquivos
- `search --context N` agora emite linhas de contexto ao redor dos matches
- `search --max-count N` agora limita matches por arquivo
- `search --invert` agora mostra linhas sem correspondência (estava invertido)
- `search --sort path` agora ordena resultados por caminho de arquivo
- `transform` agora processa arquivos em paralelo (era sequencial)
- Timestamp `modified` de `read` agora retorna ISO 8601 em vez de epoch seconds
- Backup de `batch delete` agora usa create_backup() atômico com fsync (estava competindo com a escrita)
- `create_backup` agora usa `fs::copy` em vez de `fs::hard_link` (hard links divergiriam silenciosamente)
- 12 links intra-doc quebrados em `error.rs` corrigidos
- Números mágicos de exit code substituídos por constantes nomeadas em `constants.rs`
- Seis chamadas `unwrap()` em `edit.rs` modo multi-edit substituídas por `ok_or_else`
- `scope.rs` thread join não causa mais panic em falha

### Mudanças Aditivas
#### Novos Subcomandos
- Subcomando `scope` para escopo gramatical com ações baseadas em AST (delete, upper, lower, titlecase, squeeze, replace)
- `scope` suporta Rust (30 queries preparadas), Python (13), JavaScript/TypeScript (11), Go (8)
- Subcomando `backup` para backups com timestamp e checksums BLAKE3 e retenção configurável
- Subcomando `rollback` para restauração a partir de backups com verificação BLAKE3 opcional
- Subcomando `apply` para aplicação de patches com detecção automática de formato (unified diff, SEARCH/REPLACE, markdown-fenced, full file)

#### Novas Flags
- Flag `batch --transaction` para execução tudo-ou-nada com rollback
- Flag `edit --fuzzy` com cascata de 7 estratégias para matching aproximado
- Flag `edit --multi` para múltiplas edições NDJSON em uma escrita atômica
- Flag `--line-ending lf|crlf|cr|auto` em `write` e `edit`
- Flag global `--lang <LOCALE>` para override de locale (en, pt-BR)
- `batch` move e copy aceitam `source`, `from`, `src` como aliases de campo
- `batch` write, delete, edit, hash aceitam `path` como alias de `target`

#### Internacionalização
- Suporte a i18n via `rust-i18n` com detecção automática de locale do SO
- Todas as strings voltadas ao usuário agora cientes de locale (erros, avisos, mensagens informativas)
- Documentação bilíngue (en + pt-BR) para todos os documentos principais

#### Segurança
- Detecção de FIFO e arquivos de dispositivo na validação de caminho (códigos de saída 85 e 86)
- Detecção de hardlink antes do rename atômico com `tracing::warn` quando nlink > 1
- Detecção de mesmo arquivo em `copy` e `move` para prevenir perda de dados quando origem=destino
- Headers de licença SPDX em todos os 64 arquivos `.rs` fonte
- `deny.toml` para auditoria de licenças e advisories via cargo-deny

#### Infraestrutura de Testes
- 282 testes (eram 5 na v0.1.0)
- Testes de integração para `backup`, `rollback`, `apply` e `scope`
- 2 alvos de fuzzing (`batch_parse`, `extract_json`) com `libfuzzer-sys`
- Testes de integração de locking otimista
- Testes de validação NDJSON expandidos de 5 para 20 de 21 comandos
- Testes de interoperabilidade `jaq` validando NDJSON via filtro
- Teste de integração i18n

### Mudanças na Saída JSON
- Saída de `edit` inclui novos campos opcionais: `fuzzy`, `strategy`, `strategies_tried`, `similarity` (apenas quando correspondência fuzzy é usada)
- Timestamp de `read` mudou de epoch seconds para formato ISO 8601 (quebra para consumidores lendo `modified` como número)
- Novos tipos de saída adicionados para `scope`, `backup`, `rollback`, `apply`
- Todos os campos existentes permanecem inalterados

### Exemplo de Mudança de JSON Schema

```json
// Antes (v0.1.0)
{"type":"read","path":"/abs/file","content":"...","modified":1704067200}

// Depois (v0.1.1)
{"type":"read","path":"/abs/file","content":"...","modified":"2024-01-01T00:00:00Z"}
```

### Limitações Conhecidas Corrigidas em v0.1.2
- Flag `batch --file <PATH>` era declarada mas não era conectada (agora lê manifesto de arquivo)
- `batch --transaction` não deletava arquivos criados no meio da transação
- `replace` inflava contadores em violações de jail
- Walker paralelo do `search` intercalava eventos de arquivos diferentes
- `search` com regex inválido produzia erro cru no stderr em vez de envelope JSON
- `scope --delete` em comentários Rust deixava whitespace órfão
- Compilação no macOS falhava (nix 0.29 restringia `posix_fadvise` a Unix não-macOS)
- `backup --output-dir` era declarado mas não era conectado
- Sem flags `--timeout`, `--grep`, `completions --install`

### Ação de Migração
- Nenhuma ação necessária para v0.1.0 a v0.1.1
- Filtros `jaq` e código de parsing JSON existentes continuam funcionando para todos os campos exceto `read.modified` (epoch → ISO 8601)
- Atualize consumidores que leem `read.modified` como valor numérico
- Novos campos são aditivos e seguros para ignorar
- Recomendado: atualize para v0.1.2 em seguida, que corrige 14 issues introduzidas na v0.1.1


## v0.1.11 para v0.1.12
### v0.1.12

A release v0.1.12 fecha 13 dos Top 20 gaps da auditoria PRD v5-v16 (`gaps.md`). É aditiva: todo comportamento de v0.1.11 é preservado.

#### Adicionado (Novos Subcomandos -- v14 Tier 3)
- `set <PATH> <KEY_PATH> <VALUE>` -- escreve um valor em um caminho dotted em arquivo TOML ou JSON, preservando comentários e ordem das chaves via `toml_edit`.
- `get <PATH> <KEY_PATH>` -- lê um valor em um caminho dotted. NDJSON: `{"type":"get","key_path","value","found","format"}`.
- `del <PATH> <KEY_PATH>` -- remove uma chave. Flag `--force-missing` trata chaves ausentes como no-op success.
- `case <PATHS...> --subvert OLD NEW --to <style>` -- renomeia identificadores via `heck`.
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` -- caminha um AST tree-sitter.
- `outline <PATH> [--kind <KIND>] [--positions]` -- extrai estrutura de alto nível.

#### Adicionado (G72 verificação de sintaxe REAL)
- `atomwrite write --syntax-check` invoca tree-sitter (24 linguagens). Exit 88.

#### Adicionado (G114 sidecar WAL)
- `.atomwrite.journal.<target>.atomwrite.journal.json` com `Started`/`Committed`.
- `recover_orphan_journals(dir)` é consultivo.

#### Adicionado (Outros Gaps)
- **G54 `--lock` e `--lock-timeout <ms>`** -- flock advisory. Exit 83.
- **G39 xattr** -- macOS quarantine, Linux SELinux, capabilities POSIX preservados.
- **G41 content_inspector** -- UTF-8, UTF-16LE, UTF-16BE, Binário corretamente detectados.
- **G64 reflink CoW** -- `reflink_or_copy` em APFS/btrfs/XFS.
- **G90 fallback EXDEV** -- copy fallback para Docker/NFS. `--strict-atomic` para opt out (exit 91).
- **G44 transform multi-rule** -- `--rules <file.yaml>` e `--inline-rules <YAML>`.
- **G68 `--max-filesize` e `--max-columns`** -- search skip/truncate.
- **G80 SIGPIPE** -- broken pipe → exit 0.
- **G81 `--format raw` e `--raw`** -- read emite bytes crus para composabilidade Unix.

#### Adicionado (5 Novos Códigos de Erro)
- 83 `LockTimeout`
- 88 `SyntaxError`
- 91 `ExdevFallbackDisabled`
- 92 `CopyBackBlake3Failed`
- 93 `OrphanJournal`

#### Ação de Migração
- Nenhuma mudança de código necessária
- Novos subcomandos são opt-in
- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.12"`

## Notas de Compatibilidade
### v0.1.15 (Atual)
- G117: o `edit` multi-par ganha paridade fuzzy, `pair_results`, `failed_pair_index` e o opt-in `--partial` -- os campos de envelope são aditivos
- G118: o `write` resolve o alvo contra o workspace antes de append/prepend, detecção automática de line ending e `--expect-checksum` -- exits 82/126 agora disparam onde havia sobrescrita silenciosa
- Nenhum código de erro novo; MSRV permanece em Rust 1.88
### v0.1.12
- 6 novos subcomandos: `set`, `get`, `del`, `case`, `query`, `outline` (v14 Tier 3)
- G72 verificação de sintaxe REAL via tree-sitter (`atomwrite write --syntax-check`, exit 88)
- G114 sidecar WAL para recuperação de crash (`.atomwrite.journal.<target>.atomwrite.journal.json`)
- G54 lock advisory via `flock` (exit 83 em timeout)
- G39 preservação de xattr (quarantine macOS, SELinux Linux, capabilities POSIX)
- G41 content_inspector para detecção UTF-16/BOM/binário
- G64 reflink CoW para backup/copy em APFS/btrfs/XFS
- G90 fallback EXDEV para Docker/NFS (exit 91 com `--strict-atomic`)
- G44 transform multi-rule YAML (`--rules` / `--inline-rules`)
- G68 `--max-filesize` e `--max-columns` para search
- G80 tratamento de SIGPIPE (broken pipe → exit 0)
- G81 `--format raw` para read (composabilidade Unix)
- 5 novos códigos de erro: 83, 88, 91, 92, 93
- Todo comportamento de v0.1.11 preservado
- 445 testes (era 320 baseline, +125 novos)

### v0.1.3 (Histórico)
- BREAKING: `edit` e `replace` não preservam mais o mtime original do arquivo por padrão
- Nova flag `--preserve-timestamps` em `edit` e `replace` restaura o comportamento da v0.1.2
- Novo campo `mtime_preserved` nas respostas NDJSON de `EditOutput` e `ReplaceResult`
- Todo comportamento da v0.1.2 preservado caso contrário (correção de build no macOS, correção de transação do batch, agrupamento de eventos do search, etc)

### v0.1.2 (Anterior)
- Todo comportamento da v0.1.1 preservado
- 6 correções críticas de bugs incluindo build no macOS, transação do batch, contador do replace
- 2 correções de alta prioridade (batch --file, backup --output-dir)
- 3 flags agent-first (--timeout, --grep, completions --install)
- 4 atualizações de dependências (nix 0.31, signal-hook 0.4, windows-sys 0.61, rust-i18n 4)

### v0.1.1
- Todo comportamento da v0.1.0 preservado
- Novos subcomandos e flags são apenas aditivos
- Exit codes inalterados da v0.1.0

### v0.1.0
- Primeira versão pública
- Todos os JSON schemas estão definidos em `docs/schemas/`
- Use `--json-schema` em qualquer subcomando para introspecção em runtime
- Exit codes seguem convenções sysexits
- Releases pré-1.0 não garantem estabilidade de saída
- Releases pós-1.0 manterão compatibilidade retroativa dentro de versões maiores


## Plano de Rollback
- Mantenha o binário da versão anterior disponível antes de atualizar
- Use `cargo install atomwrite@0.x.y` para fixar uma versão específica
- Verifique o rollback executando `atomwrite --version`
- Teste a nova versão em ambiente de staging antes de produção
- Monitore exit codes e saída NDJSON para mudanças inesperadas
- Reverta para a versão anterior se testes do agente falharem
- Reverta a configuração do agente para corresponder a versão anterior da CLI


## v0.1.20 — Novidades

Esta release introduz uma nova camada de segurança chamada **intention guards** e renomeia a flag global `--lang` para `--locale` para desambiguar do seletor tree-sitter `--lang` usado por `scope` e `transform`.

### Intention Guards (5 flags OPT-IN)

- `--require-backup <N>` — recusa a operação quando menos de `N` backups retidos existem para o alvo
- `--confirm` — emite um prompt de confirmação listando a mutação planejada em NDJSON antes de executar
- `--auto-rotate <N>` — rotaciona automaticamente o anel de backups para `N` entradas após uma escrita bem-sucedida
- `--risk-threshold <LOW|MEDIUM|HIGH>` — bloqueia operações cujo risco classificado atinge ou excede o threshold
- `--locale <en|pt-BR>` — renomeado de `--lang` para desambiguar do `--lang` tree-sitter

### Outras Adições

- `count --by-size` — lista os maiores arquivos da árvore com tamanhos e contagem de linhas
- `read --mode raw|envelope` — seleciona entre saída byte-stream e envelope NDJSON estruturado
- `search --no-begin-end` — desabilita a decoração implícita de âncoras `^` e `$` na saída regex
- `write --preserve-timestamps` — preserva o mtime do arquivo fonte ao sobrescrever
- `scope --lang rust` — alias explícito aceito para simetria ergonômica com `transform --lang`

### Estatísticas

- 542 testes passando em 47 suites de integração, 0 falhas
- 11 GAP-2026 fechados
- 3 targets de cross-compile Windows verdes
- 19 ADRs em `docs/decisions/` (0019-0037)

### Migração `--lang` para `--locale`

```bash
# Descobrir todos os arquivos com --lang
rg -l -- '--lang\b' .

# Substituir em massa preservando outros matches
fd -e sh -e md -e toml -e yml -e yaml -e json -x sd -- '--lang\b' '--locale' {}

# Ou via ruplacer
ruplacer --subvert --lang --locale
```

## v0.1.21 — Novidades

Esta release fecha 3 GAP-2026 items (012, 013 Problema C, 014 v2) e adiciona 1 ADR (0038 backup cumprido deleta). A mudança mais visível é que operações `--backup` agora DELETAM o backup após sucesso por padrão; adicione `--keep-backup` para preservá-lo. A segunda mudança visível é que `edit` e `rollback` agora aceitam `--backup`, fechando o buraco de paridade de API da v0.1.20. A terceira mudança é `--allow-sequential-drift` em `edit` para pipelines sequenciais.

### Operações de Backup

- `write --backup` e `replace --backup` DELETAM o backup após sucesso por padrão
- `edit --backup` e `rollback --backup` são NOVIDADES em v0.1.21; a flag é honrada em todos os 4 subcomandos mutantes
- `--keep-backup` é a flag OPT-IN para preservar o backup após sucesso em `write`, `edit`, `replace`, `rollback`, `apply` e `batch`
- `apply --keep-backup` e `batch --keep-backup` são NOVIDADES em v0.1.21 para paridade
- Backups são SEMPRE preservados no caminho de FALHA, independentemente de `--keep-backup`

### Padrão de Edits Sequenciais

- Encadear múltiplas chamadas `edit` no mesmo arquivo sem re-capturar `checksum_after` produz `STATE_DRIFT` (exit 82) em toda chamada após a primeira
- Dois padrões válidos: re-capturar checksum (Padrão A) ou passar `--allow-sequential-drift` (Padrão B)
- Comportamento padrão inalterado: `STATE_DRIFT` ainda dispara em mismatch de checksum quando a flag está ausente

#### Exemplo — Padrão A

```bash
# Checksum inicial
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')

# Edit 1 — passa o checksum capturado
echo "linha 2" | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --append

# Re-captura o checksum pós-edição
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')

# Edit 2 — usa o novo checksum
printf 'linha 1\nlinha 2\n' | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --append
```

#### Exemplo — Padrão B

```bash
# Edit 1 — checksum inicial
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
echo "linha 2" | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --append

# Edit 2 — drift permitido, o pré-estado difere de CS
printf 'linha 1\nlinha 2\n' | atomwrite --workspace . edit --allow-sequential-drift src/main.rs --append
```

### Migration Notes

- **Breaking change** — `write --backup` e `replace --backup` não deixam mais um sibling `.bak` em disco após uma escrita bem-sucedida. O comportamento pré-v0.1.21 de backup vive para sempre acabou. Adicione `--keep-backup` a qualquer script que dependa do backup persistindo através da operação, ou reescreva para ler o backup antes da escrita completar.
- **Breaking change** — `edit` e `rollback` agora aceitam `--backup` mas o ignoram sem reclamação se as pré-condições da camada atômica rejeitarem. O novo opt-in é a flag explícita `--backup`; scripts antigos que chamavam `edit` com a suposição de sem backup ainda recebem sem backup por padrão.
- **Não-breaking** — `apply --keep-backup` e `batch --keep-backup` são aditivos. Comportamento padrão (sem backup) permanece inalterado.

### Validation

- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 555+ testes passando (542 baseline v0.1.20 + 13 novos)
- 1 novo ADR: 0038 (backup cumprido deleta)
- 3 novos GAP-2026 fechados (012, 013 Problema C, 014 v2)
- Cross-compile verificado em 3 targets Windows
- Smoke test de migração: `fd '*.bak.*' . | wc -l` reporta 0 em uma execução pós-sucesso; reporta 1 quando `--keep-backup` está setado


## v0.1.22 — Novidades

Esta release adiciona 2 novos sub-comandos para fechar o último GAP-2026-012 e dar aos operadores ferramentas para gerenciar backups legados.

### Sub-comandos Adicionados

- **`prune-backups [PATHS]...`** — limpeza manual de backups `.bak.YYYYMMDD_HHMMSS` legados
  - Flags: `--max-age <SECONDS>` (deleta backups mais antigos que N segundos), `--max-count <N>` (mantém N mais recentes), `--dry-run` (lista sem deletar; default `true` para segurança)
  - Saída NDJSON com `scanned`, `deleted`, `kept`, `elapsed_ms`, `dry_run`
  - Exit 0 em scan completo, 1 se nenhum backup encontrado, 65 em falha de precondição
  - Veja `docs/decisions/0040-prune-backups-subcommand.md`
- **`edit-loop <PATH>`** — aplica N pares `{old, new}` via NDJSON no stdin em 1 invocação
  - Aceita flags globais (`--workspace`, `--expect-checksum`, `--partial`, `--fuzzy`, `--line-ending`, `--preserve-timestamps`)
  - Saída NDJSON: 1 linha `pair_result` por par + linha `summary` final com `pairs_total`, `pairs_matched`, `pairs_unmatched`
  - Exit 0 se todos os pares casaram, 1 se zero matches (NO_MATCHES), 65 em falha de precondição
  - Veja `docs/decisions/0039-edit-loop-helper.md`

### Validation

- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 575+ testes passando (555+ baseline v0.1.21 + 16+ novos de regressão e property tests)
- 2 novos ADRs: 0039 (edit-loop helper), 0040 (prune-backups subcommand)
- 2 novos schemas NDJSON: `edit-loop-output.schema.json`, `prune-backups-output.schema.json`
- Cross-compile verificado em 3 targets Windows


## v0.1.22 para v0.1.23

### Mudanças Comportamentais (Ação Necessária)

- backup-by-default: todos os 9 comandos que mutam conteúdo (`write`, `edit`, `edit-loop`, `replace`, `transform`, `apply`, `set`, `del`, `case`) agora criam backup ANTES de escrever por padrão. O backup é auto-deletado após sucesso (default existente `keep_backup: false` inalterado). Se seu pipeline depende de NENHUM arquivo de backup ser criado (ex.: verifica ausência de arquivos `.bak.*`), adicione `--no-backup` ao comando ou defina `ATOMWRITE_BACKUP=0` globalmente
- guarda de shrink: `write --expect-checksum` agora BLOQUEIA writes que reduzem o arquivo em mais de 50%. Se seu pipeline legitimamente trunca arquivos usando `--expect-checksum`, adicione `--allow-shrink` ao comando. Sem `--expect-checksum`, o comportamento é inalterado

### Mudanças Aditivas (Sem Ação Necessária)

- `allow_hyphen_values`: 15 campos CLI em 8 structs agora aceitam valores iniciando com `-`. Anteriormente causavam exit 2 (ARGUMENT_PARSE_ERROR). Nenhuma migração necessária — isso corrige um bug
- `edit --old-file <PATH> --new-file <PATH>`: novas flags que leem conteúdo de match/substituição de arquivos em vez de argumentos CLI. Contorna o ARG_MAX do kernel (~131 KB). Cross-mixing de `--old` com `--new-file` (ou vice-versa) retorna exit 65. Nenhuma migração necessária — são flags novas opt-in

### Checklist de Migração

- Se usa `write` sem `--backup`: nenhuma ação necessária (backup auto-deleta após sucesso)
- Se verifica ausência de arquivos `.bak.*` em CI: adicione `--no-backup` ou defina `ATOMWRITE_BACKUP=0`
- Se usa `write --expect-checksum` para truncar arquivos legitimamente: adicione `--allow-shrink`
- Se passa valores iniciando com `-` para `edit --old`, `search`, `replace`, `calc`, `regex`, `transform`, `read --grep`, `query --query`: a correção é automática, nenhuma migração necessária
