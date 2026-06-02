# Guia de Migração do atomwrite


[Read in English](MIGRATION.md)


## Versão Atual
- atomwrite está na v0.1.2
- Este documento cobre migração da v0.1.1 para v0.1.2
- Veja a seção abaixo para mudanças aditivas na v0.1.2


## v0.1.1 para v0.1.2

### v0.1.2 (Atual)

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
- Schemas de saída NDJSON para todos os 22 subcomandos
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

### Mudanças Aditivas
- `batch` suporta 7 operações: write, replace, delete, edit, hash, move, copy (era write, replace, delete)
- `batch --transaction` flag para execução tudo-ou-nada com rollback
- `batch` move e copy aceitam `source`, `from`, `src` como aliases de campo
- `batch` write, delete, edit, hash aceitam `path` como alias de `target`
- `edit --fuzzy` flag com cascata de 7 estratégias para matching aproximado
- `edit --multi` flag para múltiplas edições NDJSON em uma escrita atômica
- Subcomando `scope` para escopo gramatical com ações baseadas em AST
- Subcomando `backup` para backups com timestamp e checksums BLAKE3
- Subcomando `rollback` para restauração a partir de backups
- Subcomando `apply` para aplicação de patches com detecção automática de formato
- Flag `--line-ending lf|crlf|cr|auto` em `write` e `edit`
- Flag global `--lang <LOCALE>` para override de locale (en, pt-BR)
- Suporte a i18n via `rust-i18n` com detecção automática de locale do SO
- 282 testes (eram 5 na v0.1.0)

### Mudanças na Saída JSON
- Saída de `edit` inclui novos campos opcionais: `fuzzy`, `strategy`, `strategies_tried`, `similarity`
- Timestamp de `read` mudou de epoch seconds para formato ISO 8601
- Novos tipos de saída adicionados para `scope`, `backup`, `rollback`, `apply`
- Todos os campos existentes permanecem inalterados

### Ação de Migração
- Nenhuma ação necessária
- Filtros `jaq` e código de parsing JSON existentes continuam funcionando
- Novos campos são aditivos e seguros para ignorar


## Notas de Compatibilidade
### v0.1.1 (Atual)
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
