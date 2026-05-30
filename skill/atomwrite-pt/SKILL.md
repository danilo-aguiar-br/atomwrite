---
name: atomwrite
description: |
  Use atomwrite para TODAS as operações de arquivo: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, backup, rollback, apply, batch, completions.
  Auto-invocar quando o usuário pedir: escrever arquivos, buscar código, substituir texto, refatorar AST, gerar regex, calcular expressões, operações em lote, verificar checksums, listar estrutura, escopo gramatical, backup de arquivos, rollback, aplicar patches.
  Palavras-chave: escrita atômica, operação de arquivo, NDJSON, BLAKE3, checksum, refatorar, ast-grep, lote, busca paralela, scoping, backup, rollback, aplicar patch.
---


## Identidade Principal
### OBRIGATÓRIO
- stdout é SEMPRE NDJSON (um objeto JSON por linha)
- stderr é apenas para logs e tracing
- Toda escrita passa pelo pipeline atômico: tempfile, fsync, rename
- Checksum BLAKE3 presente em toda resposta de write e read
- Passar `--workspace <DIR>` para definir a raiz do jail em todas as operações de caminho
- Todos os caminhos são resolvidos relativos à raiz do workspace
### PROIBIDO
- NUNCA parsear stderr como dados estruturados
- NUNCA assumir que exit 1 é erro (search usa exit 1 para zero resultados)
- NUNCA escrever arquivos fora do jail do workspace


## Operações de Escrita
### OBRIGATÓRIO — Escrita Atômica
- SEMPRE passar a flag `--workspace` para definir a raiz do jail
- SEMPRE enviar conteúdo via stdin
- USAR `--backup --retention N` para sobrescritas destrutivas
- USAR `--expect-checksum <BLAKE3>` para locking otimista (detecção de state drift)
- USAR `--dry-run` antes de escritas destrutivas para pré-visualizar a operação
- Resposta inclui `checksum` (BLAKE3) e `bytes_written`
### PROIBIDO
- NUNCA escrever sem `--workspace`
- NUNCA passar conteúdo de arquivo como argumento CLI
### Padrão Correto — Escrita
```bash
echo "content" | atomwrite --workspace . write target.rs
```
### Padrão Correto — Escrita com Backup
```bash
cat new_config.toml | atomwrite --workspace . write --backup --retention 3 config.toml
```
### Padrão Correto — Locking Otimista
```bash
echo "updated" | atomwrite --workspace . write --expect-checksum abc123 config.toml
```


## Operações de Leitura
### OBRIGATÓRIO
- USAR `read` para conteúdo de arquivo com metadados
- USAR `read --stat` para metadados apenas (sem corpo)
- USAR `read --lines 1:50` para leituras parciais por intervalo de linhas
- USAR `read --verify-checksum <BLAKE3>` para verificação de integridade
- Resposta inclui `checksum`, `size`, `lines`
### Padrão Correto — Leitura
```bash
atomwrite --workspace . read src/main.rs
```
### Padrão Correto — Leitura Parcial
```bash
atomwrite --workspace . read --lines 1:50 src/main.rs
```
### Padrão Correto — Apenas Metadados
```bash
atomwrite --workspace . read --stat src/main.rs
```


## Operações de Busca
### OBRIGATÓRIO
- USAR `search` para busca paralela via ripgrep em arquivos
- Exit code 1 significa zero resultados encontrados (NÃO é um erro)
- USAR `--include '*.rs'` para filtrar por extensão de arquivo
- USAR `--context N` para linhas de contexto ao redor de cada match
- USAR `--fixed` (`-F`) para busca literal (sem regex)
- USAR `--count` (`-c`) para contar matches por arquivo em vez de listar
- USAR `--files` (`-l`) para listar apenas nomes de arquivos com matches
- USAR `--max-count N` (`-m`) para limitar matches por arquivo
- USAR `--invert` para retornar linhas que NÃO casam com o padrão
- USAR `--sort path` para ordenar resultados por caminho de arquivo
- Resposta é NDJSON com um objeto por match
### PROIBIDO
- NUNCA tratar exit code 1 como falha em search
### Padrão Correto — Busca
```bash
atomwrite --workspace . search 'TODO|FIXME' src/ --include '*.rs'
```
### Padrão Correto — Busca com Contexto
```bash
atomwrite --workspace . search 'unsafe' src/ --context 3
```


## Operações de Substituição
### OBRIGATÓRIO
- USAR `replace` para substituição em massa com escritas atômicas
- SEMPRE usar `--dry-run` primeiro para substituições destrutivas
- USAR `--regex` para padrões baseados em regex
- USAR `--word` para correspondência por limite de palavra
- Resposta inclui `matches`, `files_modified`, checksums por arquivo
### PROIBIDO
- NUNCA executar replace sem `--dry-run` primeiro
### Padrão Correto — Substituição
```bash
atomwrite --workspace . replace --dry-run 'old_api' 'new_api' src/
atomwrite --workspace . replace 'old_api' 'new_api' src/
```
### Padrão Correto — Substituição com Regex
```bash
atomwrite --workspace . replace --regex 'v\d+\.\d+' 'v2.0' src/ --include '*.toml'
```


## Operações de Edição
### OBRIGATÓRIO
- USAR `edit` para modificações cirúrgicas por número de linha ou marcador de texto
- USAR `--old "texto" --new "texto"` para substituição exata dentro de um arquivo
- USAR `--after-line N` para inserir conteúdo após uma linha específica
- USAR `--before-line N` para inserir conteúdo antes de uma linha específica
- USAR `--range N:M` para substituir um intervalo de linhas
- USAR `--fuzzy auto|off|aggressive` para controlar correspondência aproximada de texto
- USAR `--multi` para aplicar múltiplas edições em uma única chamada
- Enviar novo conteúdo via stdin ao usar `--range`, `--after-line` ou `--before-line`
### Padrão Correto — Edição por Texto
```bash
atomwrite --workspace . edit src/main.rs --old "old_text" --new "new_text"
```
### Padrão Correto — Inserir Após Linha
```bash
echo "new_line_content" | atomwrite --workspace . edit src/main.rs --after-line 10
```


## Operações de Transformação (AST)
### OBRIGATÓRIO
- USAR `transform` para refatoração estrutural via ast-grep
- SEMPRE especificar `--lang` (`-l`) para a linguagem alvo
- USAR `$NAME` para capturas de nó AST único
- USAR `$$$ARGS` para capturas de múltiplos nós AST (variádico)
- 306 linguagens suportadas via ast-grep
- USAR `--dry-run` para pré-visualizar transformações
### Padrão Correto — Transformação
```bash
atomwrite --workspace . transform -p 'console.log($$$A)' -r 'logger.info($$$A)' -l js src/
```
### Padrão Correto — Refatoração Rust
```bash
atomwrite --workspace . transform -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/
```
### Padrão Correto — Dry Run
```bash
atomwrite --workspace . transform --dry-run -p 'old_fn($$$A)' -r 'new_fn($$$A)' -l rust src/
```


## Operações em Lote
### OBRIGATÓRIO
- USAR `batch` para múltiplas operações em uma única chamada
- Entrada é NDJSON via stdin (um objeto JSON por linha)
- Cada linha requer um campo `op`: `write`, `replace`, `delete`, `edit`, `move`, `copy`, `hash`
- USAR `--dry-run` para pré-visualizar o lote inteiro
- USAR `--transaction` para garantir atomicidade do lote inteiro (falha em uma op reverte todas)
- Resposta é NDJSON com um resultado por operação
### Padrão Correto — Lote
```bash
echo '{"op":"write","target":"a.txt","content":"hello"}
{"op":"replace","path":"b.txt","pattern":"old","replacement":"new"}
{"op":"delete","target":"tmp.log"}' | atomwrite --workspace . batch
```
### Padrão Correto — Lote Dry Run
```bash
cat ops.ndjson | atomwrite --workspace . batch --dry-run
```


## Operações de Hash
### OBRIGATÓRIO
- USAR `hash` para checksums BLAKE3 independentes
- Aceita um ou mais caminhos de arquivo
- Resposta inclui `path` e `checksum` por arquivo
### Padrão Correto — Hash
```bash
atomwrite --workspace . hash src/main.rs
atomwrite --workspace . hash src/*.rs
```


## Operações de Remoção
### OBRIGATÓRIO
- USAR `delete` para remoção atômica de arquivos
- USAR `--backup --retention N` para manter backups antes da remoção
- USAR `--dry-run` para pré-visualizar
### Padrão Correto — Remoção
```bash
atomwrite --workspace . delete --backup --retention 1 tmp/scratch.rs
```


## Operações de Diff
### OBRIGATÓRIO
- USAR `diff` para comparar dois arquivos ou um arquivo contra stdin
- Resposta inclui hunks de diff estruturados em NDJSON
### Padrão Correto — Diff
```bash
atomwrite --workspace . diff src/old.rs src/new.rs
```


## Operações de Mover e Copiar
### OBRIGATÓRIO
- USAR `move` para renomear/mover atomicamente dentro do workspace
- USAR `copy` para cópia atômica com verificação de checksum
- Ambos respeitam o jail do workspace
### Padrão Correto — Mover
```bash
atomwrite --workspace . move src/old.rs src/new.rs
```
### Padrão Correto — Copiar
```bash
atomwrite --workspace . copy src/template.rs src/new_module.rs
```


## Operações de Listagem
### OBRIGATÓRIO
- USAR `list` para listagem de diretórios e arquivos
- USAR `--include '*.rs'` para filtrar por extensão
- USAR `--long` para saída em formato detalhado com metadados
- USAR `--depth N` para limitar profundidade de diretório
### Padrão Correto — Listagem
```bash
atomwrite --workspace . list --include '*.rs' src/
atomwrite --workspace . list --long --depth 2 src/
```


## Operações de Contagem
### OBRIGATÓRIO
- USAR `count` para contagem de arquivos e linhas
- Resposta inclui `files`, `lines`, `bytes`
### Padrão Correto — Contagem
```bash
atomwrite --workspace . count --include '*.rs' src/
```


## Operações de Extração
### OBRIGATÓRIO
- USAR `extract` para extração de campos NDJSON de entrada via pipe
- Passar `path` e `line_number` como argumentos posicionais para selecionar campos específicos
### Padrão Correto — Extração
```bash
atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number
```


## Operações de Cálculo
### OBRIGATÓRIO
- USAR `calc` para expressões matemáticas e conversões de unidade
- SEMPRE colocar a expressão entre aspas
- Sem necessidade de `--workspace` (operação stateless)
### Padrão Correto — Cálculo
```bash
atomwrite calc "2 hours + 30 minutes to seconds"
atomwrite calc "1.5 GiB to bytes"
atomwrite calc "sqrt(144) + 2^10"
```


## Operações de Regex
### OBRIGATÓRIO
- USAR `regex` para gerar regex a partir de exemplos
- Passar 3+ exemplos para padrões mais precisos
- USAR `--digits` para generalização com `\d`
- USAR `--words` para generalização com `\w`
- Sem necessidade de `--workspace` (operação stateless)
### Padrão Correto — Regex
```bash
atomwrite regex "192.168.1.1" "10.0.0.255" --digits
atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" --digits
```


## Operações de Scoping Gramatical
### OBRIGATÓRIO
- USAR `scope` para selecionar categorias AST e aplicar ações no código
- SEMPRE especificar `--lang` para a linguagem alvo (rust, python, js, ts, go)
- USAR `--query` para queries preparadas (comments, fn, class, struct, etc.)
- USAR `--pattern` para padrões AST customizados
- USAR `--delete` para remover conteúdo correspondente
- USAR `--action upper|lower|titlecase|squeeze` para transformações de texto
- USAR `--replace-with "texto"` para substituição customizada
- USAR `--dry-run` para pré-visualizar mudanças
### Padrão Correto — Scoping
```bash
atomwrite --workspace . scope src/ --lang rust --query comments --delete
atomwrite --workspace . scope src/ --lang rust --query fn --action upper --dry-run
atomwrite --workspace . scope src/ --lang python --query def --action lower
```

## Operações de Backup
### OBRIGATÓRIO
- USAR `backup` para criar backups com timestamp e checksums BLAKE3
- USAR `--retention N` para controlar quantos backups manter (padrão: 5)
- USAR `--dry-run` para pré-visualizar
### Padrão Correto — Backup
```bash
atomwrite --workspace . backup src/config.toml
atomwrite --workspace . backup src/main.rs src/lib.rs --retention 3
```

## Operações de Rollback
### OBRIGATÓRIO
- USAR `rollback` para restaurar um arquivo a partir de backup anterior
- USAR `--latest` para restaurar o backup mais recente (padrão)
- USAR `--timestamp YYYYMMDD_HHMMSS` para restaurar um backup específico
- USAR `--verify` para verificar checksum BLAKE3 após restauração
- USAR `--dry-run` para pré-visualizar
### Padrão Correto — Rollback
```bash
atomwrite --workspace . rollback src/config.toml
atomwrite --workspace . rollback src/config.toml --timestamp 20260530_120000 --verify
```

## Operações de Apply (Patch)
### OBRIGATÓRIO
- USAR `apply` para aplicar patches do stdin em um arquivo alvo
- Detecta formato automaticamente: unified diff, blocos SEARCH/REPLACE, markdown-fenced, arquivo completo
- USAR `--format auto|unified|search-replace|full|markdown` para forçar formato
- USAR `--backup` para criar backup antes de aplicar patch
- USAR `--dry-run` para pré-visualizar
### Padrão Correto — Apply
```bash
echo "novo conteudo" | atomwrite --workspace . apply src/file.txt --format full
git diff src/file.txt | atomwrite --workspace . apply src/file.txt
```

## Completions
### OBRIGATÓRIO
- USAR `completions` para gerar completions de shell
- Suporta `bash`, `zsh`, `fish`, `elvish`, `powershell`
### Padrão Correto — Completions
```bash
atomwrite completions bash > ~/.local/share/bash-completion/completions/atomwrite
atomwrite completions zsh > ~/.zfunc/_atomwrite
```


## Tratamento de Erros
### OBRIGATÓRIO
- VERIFICAR exit code primeiro antes de parsear stdout
- PARSEAR stdout JSON quando `error: true` para detalhes estruturados do erro
- USAR `error_class` para determinar estratégia de retry
- RETENTAR quando `retryable: true`
- USAR campo `suggestion` para remediação acionável
### PROIBIDO
- NUNCA ignorar exit codes não-zero (exceto exit 1 em search)
- NUNCA parsear stderr para dados de erro
- NUNCA retentar quando `retryable: false`
### Padrão Correto — Tratamento de Erros
```bash
output=$(atomwrite --workspace . read missing.txt 2>/dev/null)
exit_code=$?
if [ $exit_code -ne 0 ]; then
  echo "$output" | jaq '{code: .code, class: .error_class, suggestion: .suggestion}'
fi
```


## Códigos de Saída
### OBRIGATÓRIO — Referência Completa
- `0` — sucesso
- `1` — sem resultados (search/replace encontrou zero matches, NÃO é um erro)
- `4` — não encontrado (arquivo ou diretório não existe)
- `13` — permissão negada
- `28` — disco cheio
- `30` — cota excedida
- `65` — entrada inválida (argumentos ou conteúdo malformado)
- `73` — cross-device (mover entre limites de filesystem)
- `74` — erro de I/O (falha genérica de filesystem)
- `78` — configuração inválida (configuração malformada)
- `82` — state drift (checksum mismatch, locking otimista falhou)
- `126` — violação do jail do workspace (caminho escapa à raiz do workspace)
- `127` — symlink bloqueado (alvo do symlink fora do workspace)
- `85` — FIFO detectado (named pipe não pode ser escrito atomicamente)
- `86` — arquivo de dispositivo detectado (bloco ou caractere)
- `128` — imutável (arquivo marcado como imutável)
- `130` — SIGINT (interrompido pelo usuário)
- `141` — SIGPIPE (pipe quebrado)
- `143` — SIGTERM (terminado por sinal)
- `255` — erro interno (falha inesperada)


## Schema JSON de Erro
### OBRIGATÓRIO — Campos
- `error` (bool) — sempre `true` quando um erro ocorre
- `code` (string) — código de erro legível por máquina
- `exit` (u8) — número do exit code
- `message` (string) — descrição legível por humanos
- `path` (string, opcional) — caminho do arquivo envolvido no erro
- `error_class` (string) — um de: `permanent`, `transient`, `conflict`, `precondition_failed`
- `retryable` (bool) — se a operação pode ser retentada
- `suggestion` (string, opcional) — passo de remediação acionável


## Flags Globais
### OBRIGATÓRIO — Referência
- `--workspace <DIR>` — definir raiz do jail do workspace (OBRIGATÓRIO para operações de arquivo)
- `--json-schema` — imprimir o schema JSON de saída para qualquer subcomando
- `--dry-run` — pré-visualizar operação sem escrever
- `--backup` — criar backup antes de operação destrutiva
- `--retention <N>` — número de backups a reter (usado com `--backup`)
- `--verbose` / `-v` — aumentar verbosidade de log no stderr
- `--quiet` / `-q` — suprimir logs no stderr
- `--lang <LOCALE>` — substituir locale de exibição (en, pt-BR) via env `ATOMWRITE_LANG`


## Introspecção de Schema JSON
### OBRIGATÓRIO
- USAR a flag `--json-schema` para obter o schema de saída de qualquer subcomando
- USAR a saída do schema para validação programática de respostas
### Padrão Correto — Schema
```bash
atomwrite write --json-schema
atomwrite search --json-schema
```
