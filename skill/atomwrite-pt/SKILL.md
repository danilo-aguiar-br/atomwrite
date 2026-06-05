---
name: atomwrite
description: |
  Use atomwrite para TODAS as operações de arquivo: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, backup, rollback, apply, batch, completions.
  Auto-invocar quando o usuário pedir: escrever arquivos, buscar código, substituir texto, refatorar AST, gerar regex, calcular expressões, operações em lote, verificar checksums, listar estrutura, escopo gramatical, backup de arquivos, rollback, aplicar patches, editar e disparar build do cargo, preservar timestamps de arquivos.
  Palavras-chave: escrita atômica, operação de arquivo, NDJSON, BLAKE3, checksum, refatorar, ast-grep, lote, busca paralela, scoping, backup, rollback, aplicar patch, timeout, grep, instalar completions, mtime, preservar timestamps, preservação de timestamp, consciência de sistema de build, cargo build, make, cmake.
---


# atomwrite
## Identidade Principal
### OBRIGATÓRIO
- stdout é SEMPRE NDJSON (um objeto JSON por linha)
- stderr é apenas para logs e tracing
- Toda escrita passa pelo pipeline atômico: tempfile, fsync, rename
- Checksum BLAKE3 presente em toda resposta de write e read
- Passar `--workspace <DIR>` para definir a raiz do jail em todas as operações de caminho
- Todos os caminhos são resolvidos relativos à raiz do workspace
- A flag `--json` é aceita mas ignorada (saída é SEMPRE NDJSON por design)
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
- USAR `--append` para anexar conteúdo ao final do arquivo existente
- USAR `--prepend` para inserir conteúdo no início do arquivo existente
- USAR `--max-size <BYTES>` para limitar tamanho do stdin aceito
- USAR `--line-ending lf|crlf|cr|auto` para normalizar quebras de linha (padrão: auto)
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
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
echo "updated" | atomwrite --workspace . write --expect-checksum "$CS" src/main.rs
```
### Padrão Correto — Append e Prepend
```bash
echo "// nova linha" | atomwrite --workspace . write --append src/main.rs
echo "// header" | atomwrite --workspace . write --prepend src/main.rs
```


## Operações de Leitura
### OBRIGATÓRIO
- USAR `read` para conteúdo de arquivo com metadados
- USAR `read --stat` para metadados apenas (sem corpo)
- USAR `read --lines 1:50` para leituras parciais por intervalo de linhas
- USAR `read --line N` para ler uma única linha com contexto opcional via `--context N`
- USAR `read --head N` para ler as primeiras N linhas
- USAR `read --tail N` para ler as últimas N linhas
- USAR `read --format raw` para conteúdo puro sem envelope JSON
- USAR `read --grep <REGEX>` para filtrar linhas retornadas às que casam com regex (v0.1.2+)
- USAR `read --verify-checksum <BLAKE3>` para verificação de integridade
- Resposta inclui `checksum`, `size`, `lines`
### Padrão Correto — Leitura
```bash
atomwrite --workspace . read src/main.rs
```
### Padrão Correto — Leitura Parcial
```bash
atomwrite --workspace . read --lines 1:50 src/main.rs
atomwrite --workspace . read --head 20 src/main.rs
atomwrite --workspace . read --tail 10 src/main.rs
```
### Padrão Correto — Linha com Contexto
```bash
atomwrite --workspace . read --line 42 --context 5 src/main.rs
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
- USAR `--exclude '*.log'` para excluir arquivos por padrão glob
- USAR `--context N` para linhas de contexto ao redor de cada match
- USAR `--fixed` (`-F`) para busca literal (sem regex)
- USAR `--regex` (`-e`) para forçar modo regex explicitamente
- USAR `--word` (`-w`) para correspondência por limite de palavra
- USAR `--case-insensitive` (`-i`) para busca sem distinção de maiúsculas
- USAR `--smart-case` (`-S`) para insensitive quando padrão é minúsculo
- USAR `--count` (`-c`) para contar matches por arquivo em vez de listar
- USAR `--files` (`-l`) para listar apenas nomes de arquivos com matches
- USAR `--max-count N` (`-m`) para limitar matches por arquivo
- USAR `--multiline` (`-U`) para habilitar correspondência multilinha
- USAR `--invert` para retornar linhas que NÃO casam com o padrão
- USAR `--sort path|modified|created|none` para ordenar resultados
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
### Padrão Correto — Contagem por Arquivo
```bash
atomwrite --workspace . search 'unwrap' src/ --count --sort path
```


## Operações de Substituição
### OBRIGATÓRIO
- USAR `replace` para substituição em massa com escritas atômicas
- SEMPRE usar `--dry-run` primeiro para substituições destrutivas
- USAR `--regex` para padrões baseados em regex
- USAR `--word` para correspondência por limite de palavra
- USAR `--literal` (`-F`) para tratar padrão como string literal
- USAR `--include '*.rs'` para filtrar arquivos por extensão
- USAR `--exclude '*.log'` para excluir arquivos por padrão glob
- USAR `--preview` para mostrar diff sem escrever
- USAR `--max-replacements N` (`-n`) para limitar substituições por arquivo
- USAR `--expect-checksum <BLAKE3>` para locking otimista
- USAR `--backup` para criar backup antes de modificar
- USAR `--preserve-timestamps` para manter o mtime original dos arquivos modificados (padrão: mtime é atualizado para refletir a mudança). Adicione ao integrar com sistemas de build (cargo, make, cmake) que precisam de timestamps estáveis
- Resposta inclui `matches`, `files_modified`, checksums por arquivo e campo `mtime_preserved`
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
### Padrão Correto — Substituição Com mtime Preservado
```bash
# v0.1.3+: manter o mtime original de todos os arquivos substituídos
atomwrite --workspace . replace --preserve-timestamps 'old_api' 'new_api' src/
```


## Operações de Edição
### OBRIGATÓRIO
- USAR `edit` para modificações cirúrgicas por número de linha ou marcador de texto
- USAR `--old "texto" --new "texto"` para substituição exata (repetível para múltiplas)
- USAR `--after-line N` para inserir conteúdo após uma linha específica
- USAR `--before-line N` para inserir conteúdo antes de uma linha específica
- USAR `--range N:M` para substituir um intervalo de linhas
- USAR `--delete-range N:M` para deletar um intervalo de linhas
- USAR `--after-match "texto"` para inserir conteúdo após primeiro match do texto
- USAR `--before-match "texto"` para inserir conteúdo antes do primeiro match
- USAR `--between "inicio" "fim"` para substituir conteúdo entre dois marcadores
- USAR `--fuzzy auto|off|aggressive` para controlar correspondência aproximada de texto
- USAR `--multi` para aplicar múltiplas edições de uma vez (lê NDJSON do stdin)
- USAR `--expect-checksum <BLAKE3>` para locking otimista
- USAR `--line-ending lf|crlf|cr|auto` para normalizar quebras de linha
- USAR `--preserve-timestamps` para manter o mtime original do arquivo (padrão: mtime é atualizado para refletir a edição). Adicione ao integrar com sistemas de build (cargo, make, cmake) que precisam de timestamps estáveis
- Enviar novo conteúdo via stdin ao usar `--range`, `--after-line` ou `--before-line`
- Nota: `edit` e `replace` agora atualizam o mtime do arquivo por padrão (v0.1.3+). Este é o comportamento correto para cargo/make/cmake detectarem a mudança. Para backup ou builds reproduzíveis, passe `--preserve-timestamps` para manter o timestamp original
### Padrão Correto — Edição por Texto
```bash
atomwrite --workspace . edit src/main.rs --old "old_text" --new "new_text"
```
### Padrão Correto — Edição Com mtime Preservado
```bash
# v0.1.3+: manter o mtime original do arquivo (ex: para workflows de backup ou snapshot)
atomwrite --workspace . edit --preserve-timestamps src/main.rs --old "old_text" --new "new_text"
```
### Padrão Correto — Verificar Se mtime Foi Preservado
```bash
# v0.1.3+: ler o campo mtime_preserved da resposta NDJSON
atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo" | jaq -r '.mtime_preserved'
```
### Padrão Correto — Ler Resposta NDJSON Completa de Edit
```bash
# v0.1.3+: o envelope EditOutput inclui mtime_preserved como último campo
atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo" | jaq 'del(.checksum_before, .checksum_after) | {type, mtime_preserved, bytes_after}'
```
### Padrão Correto — Múltiplas Substituições
```bash
atomwrite --workspace . edit src/main.rs --old "foo" --new "bar" --old "baz" --new "qux"
```
### Padrão Correto — Inserir Após Linha
```bash
echo "new_line_content" | atomwrite --workspace . edit src/main.rs --after-line 10
```
### Padrão Correto — Deletar Intervalo
```bash
atomwrite --workspace . edit src/main.rs --delete-range 5:10
```
### Padrão Correto — Substituir Entre Marcadores
```bash
echo "novo bloco" | atomwrite --workspace . edit src/main.rs --between "// START" "// END"
```
### Padrão Correto — Múltiplas Edições via NDJSON
```bash
echo '{"old":"foo","new":"bar"}
{"old":"baz","new":"qux"}' | atomwrite --workspace . edit --multi src/main.rs
```


## Operações de Transformação (AST)
### OBRIGATÓRIO
- USAR `transform` para refatoração estrutural via ast-grep
- SEMPRE especificar `--lang` (`-l`) para a linguagem alvo
- USAR `$NAME` para capturas de nó AST único
- USAR `$$$ARGS` para capturas de múltiplos nós AST (variádico)
- 306 linguagens suportadas via ast-grep
- USAR `--dry-run` para pré-visualizar transformações
- USAR `--backup` para criar backup antes de modificar
- USAR `--include` e `--exclude` para filtrar arquivos por extensão
- Ambos `--pattern` e `--rewrite` são OBRIGATÓRIOS (sem modo somente busca)
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


## Operações de Scoping Gramatical
### OBRIGATÓRIO
- USAR `scope` para selecionar categorias AST e aplicar ações no código
- SEMPRE especificar `--lang` para a linguagem alvo
- USAR `--query` para queries preparadas por linguagem (ver lista abaixo)
- USAR `--pattern` para padrões AST customizados
- USAR `--delete` para remover conteúdo correspondente
- USAR `--action upper|lower|titlecase|squeeze` para transformações de texto
- USAR `--replace-with "texto"` para substituição customizada
- USAR `--include '*.rs'` para filtrar arquivos por extensão
- USAR `--exclude '*.log'` para excluir arquivos por padrão glob
- USAR `--backup` para criar backup antes de modificar
- USAR `--dry-run` para pré-visualizar mudanças
### Queries Preparadas — Rust
- `comments`, `doc-comment`, `strings`
- `fn`, `pub-fn`, `async-fn`, `unsafe-fn`, `test-fn`
- `struct`, `pub-struct`, `enum`, `pub-enum`
- `trait`, `impl`, `mod`, `use`
- `closure`, `unsafe`, `attribute`, `derive`
- `return`, `match`, `if-let`, `while-let`
- `for`, `loop`, `const`, `static`
- `type-alias`, `macro-rules`
### Queries Preparadas — Python
- `comments`, `strings`
- `class`, `def`, `async-def`, `lambda`
- `import`, `from-import`
- `with`, `for`, `while`
- `decorator`, `try-except`
### Queries Preparadas — JavaScript e TypeScript
- `comments`, `strings`
- `fn`, `arrow-fn`, `async-fn`
- `class`, `import`, `export`
- `try-catch`, `const`, `let`
### Queries Preparadas — Go
- `fn`, `struct`, `interface`
- `goroutine`, `defer`, `import`
- `const`, `var`
### Padrão Correto — Scoping
```bash
atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run
atomwrite --workspace . scope src/ --lang rust --query fn --action upper --dry-run
atomwrite --workspace . scope src/ --lang python --query def --action lower
```


## Operações em Lote
### OBRIGATÓRIO
- USAR `batch` para múltiplas operações em uma única chamada
- Entrada é NDJSON via stdin (um objeto JSON por linha)
- Cada linha requer um campo `op`: `write`, `replace`, `delete`, `edit`, `move`, `copy`, `hash`
- Para `move` e `copy`: usar campo `source` (origem) e `target` (destino)
- USAR `--file <PATH>` para ler manifesto de arquivo em vez de stdin
- USAR `--dry-run` para pré-visualizar o lote inteiro
- USAR `--transaction` para garantir atomicidade do lote inteiro (falha em uma op reverte todas)
- USAR `--input-schema` para obter o JSON Schema do formato de entrada
- Resposta é NDJSON com um resultado por operação
### Padrão Correto — Lote com Write e Delete
```bash
echo '{"op":"write","target":"a.txt","content":"hello"}
{"op":"delete","target":"tmp.log"}' | atomwrite --workspace . batch
```
### Padrão Correto — Lote com Move e Copy
```bash
echo '{"op":"move","source":"src/old.rs","target":"src/new.rs"}
{"op":"copy","source":"src/template.rs","target":"src/module.rs"}' | atomwrite --workspace . batch
```
### Padrão Correto — Lote Transacional
```bash
cat ops.ndjson | atomwrite --workspace . batch --transaction --dry-run
cat ops.ndjson | atomwrite --workspace . batch --transaction
```
### Padrão Correto — Lote de Arquivo
```bash
atomwrite --workspace . batch --file ops.ndjson --transaction
```


## Operações de Hash
### OBRIGATÓRIO
- USAR `hash` para checksums BLAKE3 independentes
- Aceita um ou mais caminhos de arquivo
- USAR `--verify <BLAKE3>` para verificar checksum contra hash esperado
- USAR `--stdin` para hashear conteúdo do stdin
- USAR `--recursive` (`-r`) para hashear diretórios recursivamente
- Resposta inclui `path` e `checksum` por arquivo
### Padrão Correto — Hash
```bash
atomwrite --workspace . hash src/main.rs
atomwrite --workspace . hash src/*.rs
atomwrite --workspace . hash --verify abc123 src/main.rs
echo "content" | atomwrite hash --stdin
```


## Operações de Remoção
### OBRIGATÓRIO
- USAR `delete` para remoção atômica de arquivos
- USAR `--backup --retention N` para manter backups antes da remoção
- USAR `--recursive` (`-r`) para remover diretórios recursivamente
- USAR `--include '*.log'` para filtrar por extensão
- USAR `--exclude '*.rs'` para excluir por extensão
- USAR `--yes` (`-y`) para pular confirmação
- USAR `--dry-run` para pré-visualizar
### Padrão Correto — Remoção
```bash
atomwrite --workspace . delete --backup --retention 1 tmp/scratch.rs
atomwrite --workspace . delete --recursive --include '*.log' --dry-run logs/
```


## Operações de Diff
### OBRIGATÓRIO
- USAR `diff` para comparar dois arquivos
- USAR `--unified` para formato unified diff
- USAR `--stat` para mostrar apenas estatísticas resumidas
- USAR `--context N` (`-C`) para linhas de contexto no diff (padrão: 3)
- USAR `--algorithm myers|patience|lcs` para escolher algoritmo de diff (padrão: patience)
- Resposta inclui hunks de diff estruturados em NDJSON
### Padrão Correto — Diff
```bash
atomwrite --workspace . diff src/old.rs src/new.rs
atomwrite --workspace . diff --stat src/old.rs src/new.rs
atomwrite --workspace . diff --unified --context 5 src/old.rs src/new.rs
```


## Operações de Mover e Copiar
### OBRIGATÓRIO
- USAR `move` para renomear/mover atomicamente dentro do workspace
- USAR `copy` para cópia atômica com verificação de checksum
- Ambos respeitam o jail do workspace
- USAR `--force` para sobrescrever destino se existir
- USAR `--dry-run` para pré-visualizar
- USAR `--backup` para criar backup do destino se existir
- `copy` aceita `--recursive` para copiar diretórios e `--preserve` para manter timestamps
### Padrão Correto — Mover
```bash
atomwrite --workspace . move src/old.rs src/new.rs
atomwrite --workspace . move --force src/old.rs src/existing.rs
```
### Padrão Correto — Copiar
```bash
atomwrite --workspace . copy src/template.rs src/new_module.rs
atomwrite --workspace . copy --recursive --preserve src/dir/ dest/dir/
```


## Operações de Listagem
### OBRIGATÓRIO
- USAR `list` para listagem de diretórios e arquivos
- USAR `--include '*.rs'` para filtrar por extensão
- USAR `--exclude '*.log'` para excluir por extensão
- USAR `--long` para saída em formato detalhado com metadados
- USAR `--depth N` para limitar profundidade de diretório
- USAR `--count-by-ext` para contagem agrupada por extensão
- USAR `--all` para incluir arquivos ocultos
### Padrão Correto — Listagem
```bash
atomwrite --workspace . list --include '*.rs' src/
atomwrite --workspace . list --long --depth 2 src/
atomwrite --workspace . list --count-by-ext src/
atomwrite --workspace . list --all --long src/
```


## Operações de Contagem
### OBRIGATÓRIO
- USAR `count` para contagem de arquivos e linhas
- USAR `--by-extension` para agrupar contagens por extensão de arquivo
- USAR `--by-size` com `--top N` para listar maiores arquivos
- USAR `--include` e `--exclude` para filtrar
- Resposta inclui `files`, `lines`, `bytes`
### Padrão Correto — Contagem
```bash
atomwrite --workspace . count --include '*.rs' src/
atomwrite --workspace . count --by-extension src/
atomwrite --workspace . count --by-size --top 20 src/
```


## Operações de Extração
### OBRIGATÓRIO
- USAR `extract` para extração de campos NDJSON de entrada via pipe
- Passar `path` e `line_number` como argumentos posicionais para selecionar campos específicos
- USAR `--delimiter <SEP>` para modo texto com separador customizado
### Padrão Correto — Extração
```bash
atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line_number
```


## Operações de Cálculo
### OBRIGATÓRIO
- USAR `calc` para expressões matemáticas e conversões de unidade
- SEMPRE colocar a expressão entre aspas
- USAR `--stdin` para ler expressões do stdin (uma por linha)
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
- USAR `--digits` (`-d`) para generalização com `\d`
- USAR `--words` (`-w`) para generalização com `\w`
- USAR `--spaces` (`-s`) para generalização com `\s`
- USAR `--repetitions` (`-r`) para detectar repetições
- USAR `--case-insensitive` (`-i`) para correspondência case-insensitive
- USAR `--no-anchors` para remover `^` e `$` do resultado
- USAR `--stdin` para ler exemplos do stdin (um por linha)
- Sem necessidade de `--workspace` (operação stateless)
### Padrão Correto — Regex
```bash
atomwrite regex "192.168.1.1" "10.0.0.255" --digits
atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" --digits
atomwrite regex -d -w -s -r "exemplo1" "exemplo2"
```


## Operações de Backup
### OBRIGATÓRIO
- USAR `backup` para criar backups com timestamp e checksums BLAKE3
- USAR `--retention N` para controlar quantos backups manter (padrão: 5)
- USAR `--output-dir <DIR>` para direcionar backups a diretório específico
- USAR `--dry-run` para pré-visualizar
- Nota: `backup` usa `fs::copy` diretamente (não o pipeline de escrita atômica), então o arquivo de backup herda o mtime da FONTE, não o momento da criação do backup. Isso é intencional e casa com o comportamento POSIX para cópias de arquivo
### Padrão Correto — Backup
```bash
atomwrite --workspace . backup src/config.toml
atomwrite --workspace . backup src/main.rs src/lib.rs --retention 3
atomwrite --workspace . backup src/main.rs --output-dir /tmp/backups/
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
- Nota (v0.1.3+): `apply` atualiza o mtime do arquivo alvo por padrão (mesmo que `edit` e `replace`). Isso garante que sistemas de build detectem a mudança. Use `--preserve-timestamps` para dispensar (ainda não exposto na CLI para `apply`; se necessário, edite o alvo antes/depois)
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


## Pipelines Comuns
### Padrão Correto — Locking Otimista (Read, Modify, Write)
```bash
CS=$(atomwrite --workspace . read src/config.rs | jaq -r '.checksum')
echo "new content" | atomwrite --workspace . write --expect-checksum "$CS" src/config.rs
```
### Padrão Correto — Buscar e Extrair Campos
```bash
atomwrite --workspace . search 'TODO' src/ --include '*.rs' | atomwrite extract path line_number
```
### Padrão Correto — Hash para Auditoria
```bash
atomwrite --workspace . hash src/main.rs src/lib.rs | jaq -r '.checksum'
```
### Padrão Correto — Diff Estruturado
```bash
atomwrite --workspace . diff src/old.rs src/new.rs | jaq '.type'
```
### Padrão Correto — Lote Transacional com Verificação
```bash
cat ops.ndjson | atomwrite --workspace . batch --transaction --dry-run
cat ops.ndjson | atomwrite --workspace . batch --transaction
```
### Padrão Correto — Verificar Comportamento de mtime do Edit (v0.1.3+)
```bash
# Edita e confirma se o mtime foi preservado ou atualizado (booleano)
atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo" | jaq -r '.mtime_preserved'
```
### Padrão Correto — Editar e Disparar Build Sem Touch Manual (v0.1.3+)
```bash
# Comportamento padrão do edit atualiza o mtime, então cargo/make/cmake detectam a mudança
atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo"
cargo build
```


## Padrões Agent-First (v0.1.3+)

### Editar Arquivo Fonte e Disparar Build Sem Touch Manual

```bash
# Novo padrão: edit atualiza o mtime, então cargo/make/cmake rebuildam automaticamente
atomwrite --workspace . edit src/main.rs --old "texto_antigo" --new "texto_novo"
cargo build  # rebuilda sem precisar de `touch` antes
```

### Ler mtime_preserved Da Resposta de Edit

```bash
# Parse a resposta NDJSON para verificar se o timestamp foi mantido
atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo" | jaq -r '.mtime_preserved'
```

### Preservar mtime Original Para Workflows de Backup ou Snapshot

```bash
# Voltar ao comportamento v0.1.2 de preservar o mtime original do arquivo
atomwrite --workspace . edit --preserve-timestamps src/snapshot.rs --old "antigo" --new "novo"
atomwrite --workspace . replace --preserve-timestamps 'old_api' 'new_api' src/
```

### Verificar Se Edit Não Pulou Silenciosamente um Build

```bash
# Diagnóstico: confirmar que o mtime foi atualizado, não preservado
resultado=$(atomwrite --workspace . edit src/main.rs --old "antigo" --new "novo" | jaq -r '.mtime_preserved')
if [ "$resultado" = "true" ]; then
  echo "AVISO: mtime foi preservado. Sistemas de build podem pular o rebuild. Use --preserve-timestamps=false ou passe explicitamente."
fi
```


## Tratamento de Erros
### OBRIGATÓRIO
- VERIFICAR exit code primeiro antes de parsear stdout
- PARSEAR stdout JSON quando `error: true` para detalhes estruturados do erro
- USAR `error_class` para determinar estratégia de retry
- RETENTAR quando `retryable: true`
- USAR campo `suggestion` para remediação acionável
- ESPERAR que `suggestion` seja context-aware: `WorkspaceJail` difere com base em se `--workspace` foi fornecido
- CONFIAR em `suggestion` para `FileImmutable` (menciona `chattr -i` / `fsutil`), `NoMatches` (ampliar padrão), e `BinaryFile` (usar `read --stat`)
- NOTAR que apenas `BrokenPipe` (SIGPIPE) retorna sem `suggestion` porque não é acionável
### PROIBIDO
- NUNCA ignorar exit codes não-zero (exceto exit 1 em search)
- NUNCA parsear stderr para dados de erro
- NUNCA retentar quando `retryable: false`
- NUNCA inventar sugestões que não estão na resposta (o campo `suggestion` é a fonte única de verdade)
### Padrão Correto — Tratamento de Erros
```bash
output=$(atomwrite --workspace . read missing.txt 2>/dev/null)
exit_code=$?
if [ $exit_code -ne 0 ]; then
  echo "$output" | jaq '{code: .code, class: .error_class, suggestion: .suggestion, workspace: .workspace}'
fi
```


## Suporte ao Windows 10/11 (v0.1.4)
### OBRIGATÓRIO
- VERIFICAR que Visual Studio 2019+ Build Tools com workload C++ está instalado antes de `cargo install atomwrite`
- VERIFICAR que Rust 1.88 ou posterior está instalado
- USAR Windows Terminal ou PowerShell 7+ para output UTF-8 e sequências ANSI adequadas
- CONFIAR que `init_console` define code page 65001 e `ENABLE_VIRTUAL_TERMINAL_PROCESSING` automaticamente
### PROIBIDO
- NUNCA usar console legado `cmd.exe` para output (mojibake esperado)
- NUNCA depender de `cargo install atomwrite` funcionando na v0.1.3 (quebrado no Windows 10/11; fix está na v0.1.4)
### Padrão Correto — Instalação Windows
```powershell
rustup default stable
rustup target add x86_64-pc-windows-msvc
cargo install atomwrite --locked --version '^0.1.4'
atomwrite --version  # Saída NDJSON
```


## Validação Cross-Compile (v0.1.4)
### OBRIGATÓRIO
- EXECUTAR `cargo test --test cross_compile_check -- --ignored` antes de qualquer release que toque código `#[cfg(windows)]`
- INSTALAR targets Windows: `rustup target add x86_64-pc-windows-gnu` e `i686-pc-windows-gnu`
- NO Linux, INSTALAR mingw-w64: `mingw64-gcc` (Fedora) ou `mingw-w64` (Ubuntu) e `mingw32-gcc` para 32-bit
- CONFIAR que o gate falha em qualquer regressão de `E0433`, `E0308`, ou `E0507` em código Windows-only
### Padrão Correto — Gate de Cross-Compile
```bash
rustup target add x86_64-pc-windows-gnu i686-pc-windows-gnu x86_64-pc-windows-msvc
cargo test --test cross_compile_check -- --ignored
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
- `81` — verificação de checksum falhou (mismatch de hash BLAKE3 em read ou hash)
- `82` — state drift (checksum mismatch, locking otimista falhou)
- `85` — FIFO detectado (named pipe não pode ser escrito atomicamente)
- `86` — arquivo de dispositivo detectado (bloco ou caractere)
- `126` — violação do jail do workspace (caminho escapa à raiz do workspace)
- `127` — symlink bloqueado (alvo do symlink fora do workspace)
- `128` — imutável (arquivo marcado como imutável)
- `130` — SIGINT (interrompido pelo usuário)
- `141` — SIGPIPE (pipe quebrado)
- `143` — SIGTERM (terminado por sinal)
- `255` — erro interno (falha inesperada)


## Schema JSON de Erro
### OBRIGATÓRIO — Campos
- `error` (bool) — sempre `true` quando um erro ocorre
- `code` (string) — código de erro legível por máquina (ver lista completa abaixo)
- `exit` (u8) — número do exit code
- `message` (string) — descrição legível por humanos
- `path` (string, opcional) — caminho do arquivo envolvido no erro
- `error_class` (string) — um de: `permanent`, `transient`, `conflict`, `precondition_failed`
- `retryable` (bool) — se a operação pode ser retentada
- `suggestion` (string, opcional) — passo de remediação acionável (context-aware para `WorkspaceJail`)
- `workspace` (string, opcional) — raiz atual do jail do workspace (v0.1.4+, fix do GAP 13)
### OBRIGATÓRIO — Lista Completa de Códigos de Erro (20 codes)
- `WORKSPACE_JAIL` (exit 126, precondition_failed, não retentável)
- `SYMLINK_BLOCKED` (exit 127, precondition_failed, não retentável)
- `FILE_NOT_FOUND` (exit 4, permanent, não retentável)
- `PERMISSION_DENIED` (exit 13, transient, retentável via `persist_with_retry` no Windows)
- `CHECKSUM_VERIFY_FAILED` (exit 81, conflict, não retentável)
- `STATE_DRIFT` (exit 82, conflict, não retentável)
- `DISK_FULL` (exit 28, transient, retentável)
- `QUOTA_EXCEEDED` (exit 30, transient, retentável)
- `CROSS_DEVICE` (exit 73, permanent, não retentável)
- `IO_ERROR` (exit 74, transient, retentável)
- `FIFO_DETECTED` (exit 85, precondition_failed, não retentável)
- `DEVICE_FILE` (exit 86, precondition_failed, não retentável)
- `FILE_IMMUTABLE` (exit 128, precondition_failed, não retentável)
- `BINARY_FILE` (exit 65, permanent, não retentável)
- `NO_MATCHES` (exit 1, permanent, não retentável — por design, não é um erro)
- `INVALID_INPUT` (exit 65, permanent, não retentável)
- `CONFIG_INVALID` (exit 78, permanent, não retentável)
- `BROKEN_PIPE` (exit 141, transient, não retentável — SIGPIPE não é acionável)
- `INTERNAL_ERROR` (exit 255, permanent, não retentável — reporte um bug)
### OBRIGATÓRIO — Estratégia de Retry por Classe
- `permanent` — NUNCA retentar (bug do chamador ou entrada inválida)
- `transient` — RETENTAR com backoff exponencial (1s, 2s, 4s, 8s, máximo 30s)
- `conflict` — RETENTAR somente após reler o estado (ex: re-fetch checksum)
- `precondition_failed` — NUNCA retentar; corrija a pré-condição (caminho, permissões, tipo)


## Flags Globais
### OBRIGATÓRIO — Referência
- `--workspace <DIR>` — definir raiz do jail do workspace (OBRIGATÓRIO para operações de arquivo)
- `--max-filesize <BYTES>` — tamanho máximo de arquivo aceito em bytes (padrão: 1 GiB)
- `--threads <N>` / `-j` — número de threads paralelos (0 = todos os cores, env: `RAYON_NUM_THREADS`)
- `--timeout <SECONDS>` — timeout global de operação em segundos, 0 significa sem timeout (v0.1.2+, padrão: 0)
- `--json-schema` — imprimir o schema JSON de saída para qualquer subcomando
- `--json` — aceita por compatibilidade mas ignorada (saída é SEMPRE NDJSON)
- `--color auto|always|never` — controlar saída colorida
- `--no-color` — desabilitar saída colorida (equivalente a `--color never`)
- `--no-gitignore` — não respeitar arquivos `.gitignore`
- `--hidden` — incluir arquivos e diretórios ocultos
- `--follow-symlinks` — seguir links simbólicos durante travessia
- `--verbose` / `-v` — aumentar verbosidade de log no stderr (-v info, -vv debug, -vvv trace)
- `--quiet` / `-q` — diminuir verbosidade (-q error, -qq off)
- `--lang <LOCALE>` — substituir locale de exibição (en, pt-BR) via env `ATOMWRITE_LANG`


## Introspecção de Schema JSON
### OBRIGATÓRIO
- USAR a flag `--json-schema` para obter o schema de saída de qualquer subcomando
- USAR a saída do schema para validação programática de respostas
- REFERENCIAR schemas versionados em `docs/schemas/` para contratos estáveis
- NÃO re-parsear a saída de `--json-schema` em cada chamada; cache o schema localmente
### Padrão Correto — Schema
```bash
atomwrite write --json-schema
atomwrite search --json-schema
```


## Schemas Versionados (v0.1.4)
### OBRIGATÓRIO
- SABER que schemas JSON estáveis estão commitados em `docs/schemas/`
- SABER que `error-output.schema.json` é o contrato para todos os envelopes de erro
- SABER que o campo `workspace` (string, opcional) foi adicionado em v0.1.4
- USAR o schema versionado para validar respostas no pipeline do agente
- NÃO inventar suas próprias regras de parsing; confiar no schema versionado como fonte de verdade


## Testes e Gates de Qualidade (v0.1.4)
### OBRIGATÓRIO — Postura de Qualidade
- 300+ testes em 34 suítes de teste passam com zero regressões
- 8 gates oficiais passam em cada commit: `fmt`, `clippy`, `build`, `test`, `doc`, `deny`, `audit`, `msrv`
- 3 targets de cross-compile passam: `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, `x86_64-pc-windows-msvc`
- Cargo deny e cargo audit reportam zero vulnerabilidades (time 0.3.47+ resolveu RUSTSEC-2026-0009 via DEPTH_LIMIT=32)
- MSRV é Rust 1.88 stable
### PROIBIDO
- NUNCA publicar uma release sem todos os 8 gates passando
- NUNCA publicar uma release sem os 3 targets de cross-compile passando
- NUNCA aceitar "funciona no meu Linux" como barra de qualidade de release


## Referência Rápida de Migração v0.1.4
### OBRIGATÓRIO — Saber o Que Mudou Desde v0.1.3
- Fix GAP 14: `cargo install atomwrite` agora funciona no Windows 10/11 (quebrado na v0.1.3)
- Fix GAP 13: sugestões de erro agora são context-aware (sugestão WorkspaceJail muda baseado em se `--workspace` foi fornecido)
- Fix GAP 13: todas as 20 variants de erro agora carregam campos `suggestion` acionáveis
- Fix GAP 13: referência phantom à flag `--force-text` removida das sugestões BinaryFile
- Schema: campo `workspace` adicionado ao envelope de saída de erro
- Novos testes: `tests/cross_compile_check.rs` com 3 testes de cross-compile gated
- Novos testes: 7 testes unitários + 1 teste de integração para contexto de sugestão de erro
- Docs bilíngues: 22 arquivos markdown atualizados em 3 rodadas de auditoria
- NÃO atualizar de v0.1.3 para v0.1.4 se você depende do comportamento phantom `--force-text`
