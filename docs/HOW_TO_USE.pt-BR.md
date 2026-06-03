# Como Usar o atomwrite


[Read in English](HOW_TO_USE.md)

> Uma CLI substitui dezenas de chamadas de ferramenta que seu agente faz hoje


## Pré-requisitos
- Toolchain Rust 1.85 ou superior
- Instale via `cargo install atomwrite`
- Verifique com `atomwrite --version`
- Funciona em Linux, macOS e Windows


## Primeiro Comando em 60 Segundos
- Escreva um arquivo atomicamente a partir do stdin:

```bash
echo "hello world" | atomwrite write src/hello.txt
```

- Leia de volta com metadados e checksum:

```bash
atomwrite read src/hello.txt
```

- Você recebe NDJSON no stdout com path, checksum, bytes e tempo
- Toda escrita sobrevive a falhas de energia e crashes


## Comandos Principais
### write
- Cria ou sobrescreve arquivos atomicamente via stdin
- A escrita segue a sequência tempfile, fsync, rename, fsync-dir
- Seus dados chegam ao disco ou a operação falha de forma limpa

```bash
echo "fn main() {}" | atomwrite write src/main.rs
cat config.toml | atomwrite write --backup config.toml
echo "data" | atomwrite write --expect-checksum abc123 src/file.txt
```

- Use `--backup` para criar backup com timestamp antes de sobrescrever
- Use `--expect-checksum` para locking otimista em edições concorrentes
- Use `--line-ending lf|crlf|cr|auto` para normalizar line endings (padrão: auto preserva o original)
- Use `--dry-run` para visualizar a operação sem escrever

### read
- Lê arquivos com metadados, checksum e conteúdo opcional
- Retorna contagem de linhas, bytes, permissões e data de modificação

```bash
atomwrite read src/main.rs
atomwrite read --stat src/main.rs
atomwrite read --lines 1:50 src/main.rs
atomwrite read --verify-checksum abc123 src/main.rs
```

- Use `--stat` para obter metadados sem conteúdo do arquivo
- Use `--lines 1:50` para ler um intervalo específico de linhas
- Arquivos binários são detectados e o conteúdo é omitido automaticamente

### edit
- Edita arquivos cirurgicamente por número de linha, marcador de texto ou match exato
- A edição é atômica: tempfile, fsync, rename

```bash
echo "new line" | atomwrite edit src/main.rs --after-line 5
echo "replacement block" | atomwrite edit src/main.rs --range 10:20
atomwrite edit src/main.rs --old "old_text" --new "new_text"
```

- Use `--fuzzy auto|off|aggressive` para matching fuzzy quando match exato falhar
- Use `--multi` para aplicar múltiplas edições NDJSON em uma escrita atômica via stdin
- Use `--line-ending lf|crlf|cr|auto` para normalizar line endings (padrão: auto preserva o original)
- Use `--preserve-timestamps` para manter o mtime original do arquivo (padrão: mtime é atualizado para refletir a edição)
- Retorna checksums antes e depois para verificação
- Retorna contagem de linhas antes e depois para auditoria
- Retorna flag `mtime_preserved` na resposta NDJSON

### search
- Busca conteúdo de arquivos em paralelo usando o engine do ripgrep
- Retorna matches como NDJSON com números de linha e offsets de bytes

```bash
atomwrite search 'TODO' src/
atomwrite search --regex 'fn\s+\w+' src/
atomwrite search --count 'error' logs/
atomwrite search --files 'deprecated' src/
```

- Use `--regex` para padrões de expressão regular
- Use `--fixed` (`-F`) para busca literal de string (sem regex)
- Use `--word` (`-w`) para corresponder apenas palavras inteiras
- Use `--case-insensitive` (`-i`) para busca sem distinção de maiúsculas
- Use `--context N` (`-C`) para linhas de contexto ao redor de matches
- Use `--max-count N` (`-m`) para limitar matches por arquivo
- Use `--invert` para mostrar linhas que NÃO correspondem
- Use `--sort path` para ordenar resultados por caminho de arquivo
- Use `--count` (`-c`) para contagem de matches por arquivo
- Use `--files` (`-l`) para apenas caminhos de arquivo
- Use `--include` (`-g`) e `--exclude` para filtragem de arquivo por glob
- Exit code 1 significa zero matches (não é um erro)

### replace
- Substitui texto em múltiplos arquivos com escritas atômicas
- Cada arquivo modificado passa pela sequência atômica completa

```bash
atomwrite replace 'old_name' 'new_name' src/
atomwrite replace --regex 'v\d+\.\d+' 'v2.0' src/
atomwrite replace --dry-run 'before' 'after' src/
```

- Use `--dry-run` para visualizar substituições sem modificar arquivos
- Use `--preserve-timestamps` para manter o mtime original dos arquivos modificados (padrão: mtime é atualizado para refletir a mudança)
- Retorna NDJSON por arquivo com contagem de substituições e checksums
- Emite uma linha de resumo com total de arquivos e substituições


## Comandos Utilitários
### hash
- Calcula checksums BLAKE3 para arquivos ou stdin

```bash
atomwrite hash src/main.rs
echo "data" | atomwrite hash --stdin
atomwrite hash --verify abc123 src/main.rs
```

### delete
- Deleta arquivos com backup e dry-run opcionais

```bash
atomwrite delete src/old_file.rs
atomwrite delete --backup src/old_file.rs
atomwrite delete --dry-run src/old_file.rs
atomwrite delete --recursive tmp/
```

### count
- Conta linhas, linhas em branco e arquivos agrupados por extensão

```bash
atomwrite count src/
atomwrite count --by-extension src/
```

### diff
- Compara dois arquivos usando saída unificada, estatística ou por mudança

```bash
atomwrite diff src/a.rs src/b.rs
atomwrite diff --stat src/a.rs src/b.rs
atomwrite diff --unified --context 5 src/a.rs src/b.rs
atomwrite diff --algorithm patience src/a.rs src/b.rs
```

### move
- Move ou renomeia arquivos atomicamente
- Recorre a cópia-depois-deleta para moves entre devices

```bash
atomwrite move src/old.rs src/new.rs
```

### copy
- Copia arquivos com verificação de checksum BLAKE3 após escrita

```bash
atomwrite copy src/main.rs backup/main.rs
```

### list
- Lista estrutura de arquivos do projeto com metadados opcionais

```bash
atomwrite list src/
atomwrite list --long src/
atomwrite list --count-by-ext src/
```

### extract
- Extrai campos de linhas NDJSON ou colunas de texto

```bash
echo "a b c" | atomwrite extract 0 2
atomwrite search 'TODO' src/ | atomwrite extract path line_number
```

### calc
- Avalia expressões matemáticas e conversões de unidades
- Usa o engine fend com precisão arbitrária

```bash
atomwrite calc "2 hours + 30 minutes to seconds"
atomwrite calc "sqrt(144) + 3^2"
atomwrite calc "10 GiB to MB"
```

- Coloque expressões entre aspas para evitar interpolação do shell

### regex
- Gera expressões regulares a partir de strings de exemplo
- Usa o engine grex

```bash
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"
atomwrite regex --digits --words "user_123" "admin_456"
```


## Comandos Avançados
### scope
- Escopo gramatical com ações baseadas em AST sobre categorias de código
- Suporta Rust, Python, JavaScript/TypeScript e Go
- Use `--delete` para remover conteúdo correspondente
- Use `--action upper|lower|titlecase|squeeze` para transformar texto correspondente
- Use `--replace-with "texto"` para substituir conteúdo correspondente com texto customizado

```bash
atomwrite scope --query comments --delete src/main.rs
atomwrite scope --query fn --action upper src/lib.rs
atomwrite scope --query strings --action lower src/app.ts
atomwrite scope --pattern '($$$ARGS)' --action squeeze -l rust src/
atomwrite scope --query comments --replace-with "// atualizado" src/main.rs
```

- Use `--query` para consultas preparadas (comments, fn, strings, struct, etc)
- Use `--pattern` para padrões AST customizados
- Use `--action` para especificar a transformação

### backup
- Cria backups com timestamp e checksums BLAKE3

```bash
atomwrite backup src/main.rs src/lib.rs
atomwrite backup --retention 30 src/config.toml
atomwrite backup --dry-run src/main.rs
```

- Use `--retention` para definir o período de retenção em dias
- Use `--dry-run` para visualizar sem criar backups

### rollback
- Restaura arquivos a partir de um backup anterior

```bash
atomwrite rollback src/main.rs --latest
atomwrite rollback src/main.rs --timestamp 2026-05-29T12-00-00
atomwrite rollback --verify --dry-run src/main.rs
```

- Use `--latest` para restaurar o backup mais recente
- Use `--timestamp` para restaurar um backup específico
- Use `--verify` para validar checksum BLAKE3 antes de restaurar
- Use `--dry-run` para visualizar sem restaurar

### apply
- Aplica patches a partir do stdin com detecção automática de formato
- Suporta unified diff, blocos SEARCH/REPLACE, markdown-fenced e arquivo completo

```bash
cat fix.patch | atomwrite apply src/main.rs
cat changes.md | atomwrite apply --format markdown src/main.rs
cat fix.patch | atomwrite apply --backup src/main.rs
cat fix.patch | atomwrite apply --dry-run src/main.rs
```

- Use `--format` para forçar um formato específico de patch
- Use `--backup` para criar backup antes de aplicar
- Use `--dry-run` para visualizar sem aplicar

### transform
- Busca e reescrita estrutural por AST usando ast-grep
- Suporta 306 linguagens de programação
- Entende a sintaxe do código, não apenas padrões de texto

```bash
atomwrite transform -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/
atomwrite transform -p 'console.log($$$ARGS)' -r 'logger.info($$$ARGS)' -l js src/
atomwrite transform -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/
```

- Use `$VAR` para captura de um único nó AST
- Use `$$$VAR` para captura de múltiplos nós AST
- Ambos `--pattern` e `--rewrite` são obrigatórios

### batch
- Executa múltiplas operações a partir de um manifesto NDJSON
- Suporta operações write, replace, delete, edit, hash, move e copy
- Use `--transaction` para execução all-or-nothing com rollback automático

```bash
cat manifest.ndjson | atomwrite batch
cat manifest.ndjson | atomwrite batch --dry-run
```

- Cada linha no manifesto é uma operação
- Retorna resultados por operação mais um resumo agregado
- Use `--dry-run` para validar o manifesto sem executar


## Flags Globais
- `--workspace <PATH>` -- restringe todas as operações a este diretório raiz
- `--verbose` / `-v` -- habilita saída de tracing no stderr
- `--quiet` / `-q` -- suprime saída não essencial
- `--color <auto|always|never>` -- controla saída colorida
- `--no-color` -- desabilita saída colorida (equivalente a `--color never`)
- `--no-gitignore` -- não respeita regras do .gitignore
- `--hidden` -- inclui arquivos e diretórios ocultos
- `--follow-symlinks` -- segue links simbólicos
- `--threads <N>` / `-j <N>` -- número de threads paralelas (0 = todos os cores)
- `--max-filesize <BYTES>` -- ignora arquivos maiores que este limite
- `--json-schema` -- emite JSON schema da saída do subcomando
- `--lang <LOCALE>` -- substitui o locale de exibição (en, pt-BR)
- `--timeout <SECONDS>` -- timeout global de operação (0 = sem timeout)
- `--grep <REGEX>` em `read` para filtrar linhas retornadas às que casam com regex


## Configuração
- atomwrite não requer arquivos de configuração
- Todo comportamento é controlado via flags de linha de comando
- Use `--workspace` para definir o limite do diretório do projeto
- Use `--json-schema` para inspecionar o formato de saída em tempo de execução
- Gere completions de shell com `atomwrite completions bash` ou auto-instale com `atomwrite completions bash --install` (escreve no diretório XDG)
- `ATOMWRITE_LANG`: substitui o locale para mensagens traduzidas
- `ATOMWRITE_WORKSPACE`: define a raiz do workspace para validação do jail de caminho
- `NO_COLOR`: desabilita saída colorida quando definida (veja https://no-color.org)
- `RAYON_NUM_THREADS`: sobrescreve número de threads paralelas


## Tempo De Modificação E Sistemas De Build

Por padrão, `edit` e `replace` atualizam o tempo de modificação do arquivo (`mtime`) para o momento em que a escrita é concluída. Este é o comportamento correto para sistemas de build que usam `mtime` para detectar mudanças em código fonte (cargo, make, cmake, gradle, sbt, bazel, ninja, msbuild).

O que acontece se você desativar a atualização de mtime:
- O cargo compara o mtime de cada arquivo fonte contra o arquivo `dep-info` em `target/.fingerprint/`
- Quando o mtime do fonte é mais antigo que o mtime do dep-info, o cargo assume que nada mudou e pula a recompilação
- Isso produz um no-op silencioso (`Finished in 0.29s`) onde o binário está stale mas o cargo reporta sucesso

Quando preservar o mtime com `--preserve-timestamps`:
- Você está criando um backup ou snapshot do arquivo e quer manter seu timestamp original
- Você está implementando uma operação de controle de versão que reflete estado histórico
- Você está gerando um artefato de build reproduzível onde os timestamps do fonte devem casar com metadados registrados
- Você está escrevendo em um arquivo fora de qualquer contexto de sistema de build

Para workflows interativos de agentes, o padrão seguro é deixar o `atomwrite` atualizar o mtime. O campo `mtime_preserved` na resposta NDJSON informa se o timestamp foi preservado ou atualizado, o que é crítico para diagnosticar rebuilds perdidos em sistemas de build.


## Integração Com Agentes de IA
- Todo subcomando produz NDJSON determinístico no stdout
- Toda escrita inclui um checksum BLAKE3 na resposta
- O checksum elimina a necessidade de leituras de verificação
- Use `--expect-checksum` para locking otimista em workflows concorrentes
- Use `--workspace` para isolar agentes dentro da raiz de um projeto
- Use `--dry-run` antes de operações destrutivas
- Modo batch substitui centenas de chamadas individuais de ferramenta
- Exit codes seguem convenções sysexits para tratamento programático
- Veja [AGENTS.md](AGENTS.md) para o contrato completo de integração com agentes
