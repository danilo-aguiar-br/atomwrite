# Como Usar o atomwrite


[Read in English](HOW_TO_USE.md)

> Uma CLI substitui dezenas de chamadas de ferramenta que seu agente faz hoje


## O Que Há de Novo na v0.1.12

## Início Rápido: Limpeza de WAL (G119)

A v0.1.15 entrega um sistema de gerenciamento de WAL em três camadas (G119). O novo subcomando `wal-stats` é telemetria read-only. O novo subcomando `wal-heal` é reap escopado. A nova flag `--wal-policy` controla a criação do sidecar por escrita. Use a flag global `--no-auto-heal` para desabilitar a healing automática durante workflows sensíveis.

### Inspecionar com `wal-stats`

```bash
atomwrite --workspace . wal-stats
# {"type":"result","journals_total":0,...,"reclaimable":0,...}
```

### Heal com `wal-heal`

```bash
# Remove todos os journals terminais (ignora entradas Started)
atomwrite --workspace . wal-heal --threshold-secs 0
```

### Escolher um `--wal-policy`

```bash
# Padrão: deixa o build decidir
atomwrite --workspace . write src/lib.rs < new.rs

# Proibir qualquer criação de sidecar (higiene de CI)
atomwrite --workspace . write --wal-policy never ci-output.txt < data.txt
```

### Desabilitar Auto-Heal Globalmente

Ao rodar workflows em batch via scripts ou capturas forenses, desabilite a healing automática para que o script controle quando o reap acontece.

```bash
atomwrite --workspace . --no-auto-heal wal-stats
```

### Tabela de Decisão de Política

| Carga de Trabalho | `--wal-policy` | Justificativa |
|---|---|---|
| Dev local / uso interativo | `auto` (padrão) | Otimizado para uso geral; trade-off balanceado |
| Builds de CI e jobs efêmeros | `never` | Sidecars não têm consumidor; pula o overhead |
| Deploys de produção / trilha de auditoria | `always` | Metadados forenses exigidos para postmortem |
| Migrações em massa e jobs em batch | `never` + `--no-auto-heal` | Velocidade e controle explícito do reap |
| Análise forense / debugging | `always` + `wal-heal` manual | Mantém todos os sidecars; reap só quando você decide |


Esta seção resume as mudanças relevantes para uso em v0.1.12.

### Novos Subcomandos (Tier 3)

6 novos subcomandos para operações estruturadas de config e código:

- `set <PATH> <KEY_PATH> <VALUE>` -- escreve um valor em um caminho dotted em TOML/JSON
- `get <PATH> <KEY_PATH>` -- lê um valor em um caminho dotted
- `del <PATH> <KEY_PATH>` -- remove uma chave (`--force-missing` para scripts idempotentes)
- `case <PATHS...> --subvert OLD NEW --to <style>` -- renomeia identificadores em 5 estilos de case
- `query <PATH> [--kinds|--query <KIND>|--tree] [--positions]` -- caminha um AST tree-sitter
- `outline <PATH> [--kind <KIND>] [--positions]` -- extrai estrutura de alto nível

Veja a seção Comandos Avançados abaixo para documentação detalhada de cada.

### Novas Flags para Comandos Existentes

- `write --syntax-check` -- valida com tree-sitter após escrita (G72, exit 88)
- `write --lock` e `--lock-timeout <ms>` -- lock advisory via flock (G54, exit 83)
- `write --include-fifo` -- permite escrita em named pipes (G56)
- `write --strict-atomic` -- aborta em EXDEV em vez de copy fallback (G90, exit 91)
- `read --format raw` (alias `--raw`) -- emite bytes crus para composabilidade Unix (G81)
- `read --head N`, `--tail N`, `--line N`, `--grep <REGEX>` -- novos modos de read
- `search --max-filesize <BYTES>` -- pula arquivos maiores que o limite (G68, padrão 10 MiB)
- `search --max-columns <N>` -- trunca matches com >N colunas (G68, padrão 500)
- `replace --literal` (alias `-F`) -- desabilita interpretação de regex (G66)
- `transform --rules <file.yaml>` -- multi-rule YAML para refactors em cascata (G44)
- `transform --inline-rules <YAML>` -- multi-rule YAML inline
- `batch --batch-size <N>` -- controla pico de memória (G77, padrão 100)
- `backup/copy --no-reflink` -- desabilita CoW para filesystems sem suporte (G64)

### 5 Novos Códigos de Erro

- 83 `LockTimeout` (G54)
- 88 `SyntaxError` (G72)
- 91 `ExdevFallbackDisabled` (G90)
- 92 `CopyBackBlake3Failed` (G114)
- 93 `OrphanJournal` (G114)

### G72 Verificação de Sintaxe REAL

`atomwrite write --syntax-check` invoca o parser tree-sitter real (24 linguagens) em vez da heurística de balanceamento de colchetes. Exit 88 com primeira linha/coluna de erro. O parser é baixado no primeiro uso via `tree-sitter-language-pack`.

### G114 Sidecar WAL para Recuperação de Crash

`atomic_write` escreve `.atomwrite.journal.<target>.atomwrite.journal.json` com entradas `Started`/`Committed`. `recover_orphan_journals(dir)` é consultivo (sem auto-replay, sem auto-delete). O agente decide.

### G64 Reflink CoW para Backup/Copy

`backup` e `copy` usam `reflink_or_copy` para backup O(1) em APFS/btrfs/XFS. Fallback para `fs::copy` em filesystems sem suporte a CoW. Use `--no-reflink` para forçar copy.

### Cobertura de Testes

- 502 testes passando (445 na v0.1.12 + 2 na v0.1.14 + 8 G117 + 6 G118 na v0.1.15)
- 9 ADRs em `docs/decisions/` (0019-0027)
- 7 novos JSON schemas em `docs/schemas/`
- Veja [docs/decisions/README.md](README.md) para decisões arquiteturais

## Pré-requisitos
- Toolchain Rust 1.88 ou superior
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
- Desde a v0.1.15 append/prepend, detecção automática de line ending e `--expect-checksum` resolvem o alvo contra o `--workspace` (G118); na v0.1.14 e anteriores mantenha CWD = workspace, ou alvos relativos truncam no append e pulam a verificação de checksum
- Use `--dry-run` para visualizar a operação sem escrever
- Use `--syntax-check` para validar o arquivo com tree-sitter após escrita (G72, exit 88 em erro)
- Use `--preserve-timestamps` para manter o mtime original (padrão: mtime é atualizado para cargo/make/cmake rebuild)
- Use `--include-fifo` para permitir escrita em FIFO/named pipes (padrão: exit 85)
- Use `--strict-atomic` para abortar em EXDEV (G90, padrão: copy fallback para Docker/NFS)
- Use `--lock` para adquirir lock advisory via flock (G54, exit 83 em timeout)
- Use `--no-reflink` para desabilitar backup CoW (G64, padrão: reflink em APFS/btrfs/XFS)

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
- Use `--head N` para ler as primeiras N linhas
- Use `--tail N` para ler as últimas N linhas
- Use `--line N` para ler a linha N com contexto opcional via `--context N`
- Use `--grep <REGEX>` para filtrar as linhas retornadas para as que casam com regex
- Use `--format raw` (ou `--raw`) para emitir bytes crus para composabilidade Unix (G81, quebra o envelope NDJSON)
- Use `--verify-checksum <BLAKE3>` para verificar integridade do arquivo
- Arquivos binários são detectados e o conteúdo é omitido automaticamente

### edit
- Edita arquivos cirurgicamente por número de linha, marcador de texto ou match exato
- A edição é atômica: tempfile, fsync, rename

```bash
echo "new line" | atomwrite edit src/main.rs --after-line 5
echo "replacement block" | atomwrite edit src/main.rs --range 10:20
atomwrite edit src/main.rs --old "old_text" --new "new_text"
```

- Use `--fuzzy auto|off|aggressive` para matching fuzzy quando match exato falhar (9 estratégias em cascata, G116)
- Desde a v0.1.15 pares repetidos `--old`/`--new` também rodam a cascata fuzzy por par (G117); respostas incluem `pairs_total` e `pair_results` por par, e falhas relatam `failed_pair_index`
- Use `--partial` (v0.1.15) para aplicar os pares que casam e relatar os demais; zero pares aplicados sai com 1 (`NO_MATCHES`) sem escrever
- Nunca faça pipe de `edit` para `jaq` sem `-e`: o envelope de erro vai para o stdout, então `| jaq '.edits'` mascara o exit 65 como `null` — use `jaq -e '.edits'` ou `${PIPESTATUS[0]}`
- Use `--multi` para aplicar múltiplas edições NDJSON em uma escrita atômica via stdin
- Use `--line-ending lf|crlf|cr|auto` para normalizar line endings (padrão: auto preserva o original)
- Use `--preserve-timestamps` para manter o mtime original do arquivo (padrão: mtime é atualizado para refletir a edição)
- Use `--after-line N` para inserir conteúdo após a linha N
- Use `--before-line N` para inserir conteúdo antes da linha N
- Use `--range N:M` para substituir um intervalo de linhas
- Use `--delete-range N:M` para deletar um intervalo de linhas
- Use `--between START END` para substituir conteúdo entre duas linhas marcadoras
- Retorna checksums antes e depois para verificação
- Retorna contagem de linhas antes e depois para auditoria
- Retorna flag `mtime_preserved` na resposta NDJSON
- Retorna `fuzzy`, `strategy`, `strategies_tried`, `similarity` quando fuzzy matching é usado

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
- Use `--max-filesize <BYTES>` para pular arquivos maiores que o limite (G68, padrão 10 MiB)
- Use `--max-columns <N>` para truncar linhas maiores que N colunas (G68, padrão 500)
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
- Use `--literal` (alias `-F`, `--fixed-strings`) para desabilitar interpretação de regex (G66)
- Use `--regex` para forçar modo regex (padrão)
- Use `--fuzzy auto|off|aggressive` para matching fuzzy (9 estratégias, G116)
- Use `--include` e `--exclude` para filtragem por glob
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
- Use `--no-reflink` para desabilitar backup CoW (G64, padrão: reflink em APFS/btrfs/XFS para O(1) copy)
- Use `--output-dir <DIR>` para escrever backups em um diretório específico

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
- Use `--rules <file.yaml>` para aplicar múltiplas regras de refactor em uma passada (G44)
- Use `--inline-rules <YAML>` para YAML multi-rule inline
- Suporta predicados ast-grep YAML all/any/not/inside/has/follows/precedes

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
- Use `--transaction` para execução tudo-ou-nada com rollback automático em qualquer erro
- Use `--batch-size <N>` para controlar pico de memória (G77, padrão 100, processa em chunks)
- Use `--file <PATH>` para ler o manifesto de um arquivo em vez de stdin


### set
- Escreve um valor em um caminho dotted em um arquivo TOML ou JSON
- Preserva comentários, ordem das chaves e whitespace via `toml_edit`
- Auto-coage o valor para int, float, bool ou string
- Retorna NDJSON com `old_value`, `new_value`, `format`, `comments_preserved`

```bash
atomwrite set Cargo.toml package.version 0.2.0
atomwrite set package.json scripts.build "tsc -b"
```

- Use `--type int|float|bool|string|array` para forçar coerção de tipo
- Use `--type null` para definir uma chave como `null` em JSON
- Use `--force-missing` para criar chaves intermediárias
- Arrays TOML usam notação `key[N]`: `dependencies.serde[0].version = "1.0"`

### get
- Lê um valor em um caminho dotted em um arquivo TOML ou JSON
- Retorna NDJSON com `value`, `found`, `format`

```bash
atomwrite get Cargo.toml package.version
atomwrite get package.json scripts.build
```

- Se a chave estiver ausente, retorna `{"found": false, "value": null}`
- Caminho dotted TOML: `dependencies.serde.features[0]`
- Pointer JSON (RFC 6901): `/dependencies/serde/features/0`
- Exit 0 mesmo quando a chave está ausente; use o campo `found` para detectar

### del
- Remove uma chave em um caminho dotted em um arquivo TOML ou JSON
- Retorna NDJSON com `removed`, `path_was_array_index`, `old_value`

```bash
atomwrite del Cargo.toml package.metadata.deprecated
atomwrite del package.json scripts.build
```

- Use `--force-missing` para tratar chaves ausentes como no-op success (exit 0 em vez de erro)
- Remover um elemento de array desloca os índices subsequentes (TOML) ou usa nulls (JSON)
- Não pode remover uma chave cujo pai não existe; use `--force-missing` para scripts idempotentes

### case
- Renomeia identificadores em múltiplos arquivos usando `heck` para conversão de case
- Renomeia `old_id` → `new_id` e todas as 5 variantes de case: `oldId`, `OLD_ID`, `old-id`, `OldId`, `old_id`

```bash
atomwrite case src/ --subvert user_id account_id --to snake
atomwrite case src/ lib/ --subvert user_id account_id --to camel
```

- Estilos: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`
- Multi-arquivo: passe múltiplos caminhos para renomear em um módulo inteiro
- Detecta fronteiras de identificador em 5 estilos; apenas ASCII puro
- Preserva comentários, strings e outras estruturas de código

### query
- Caminha um AST tree-sitter e emite nós como NDJSON
- 305 linguagens via `tree-sitter-language-pack` (parsers baixam no primeiro uso)
- Modos: `--kinds` (lista todos os kinds), `--query <KIND>` (filtra por kind), `-Q <KIND>` (alias), `--tree` (árvore completa), `--positions` (line:column)

```bash
atomwrite query src/main.rs --kinds
atomwrite query src/main.rs --query function_item --positions
atomwrite query src/main.rs --tree
```

- `--positions` adiciona `line` e `column` a cada nó
- `--query` e `--kinds` são mutuamente exclusivos
- Retorna um objeto NDJSON por nó com `kind`, `start_byte`, `end_byte`, `text`
- Suporte a query S-expression está adiado para v0.1.13 (veja ADR-0021)

### outline
- Extrai estrutura de alto nível (funções, classes, structs, enums, traits, módulos) como NDJSON
- 305 linguagens via `tree-sitter-language-pack`
- Retorna um objeto por definição top-level com `kind`, `name`, `line`, `column`

```bash
atomwrite outline src/main.rs
atomwrite outline src/lib.rs --kind function_item
atomwrite outline src/main.rs --positions
```

- `--kind` filtra para um kind tree-sitter específico (ex. `function_item`, `struct_item`, `impl_item`)
- `--positions` adiciona `start_line`, `start_column`, `end_line`, `end_column`
- Retorna 28 kinds de nós estruturais em todas as linguagens
- Mais rápido que `query --kinds` porque pula nós folha

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


## Sugestões de Erro (v0.1.4)
- Todo envelope de erro no stdout inclui um campo `suggestion` com orientação acionável de recuperação
- Todas as 20 variants de erro agora carregam uma `suggestion` (única exceção é `BrokenPipe` porque SIGPIPE não é acionável)
- Sugestões são **context-aware**: a sugestão de `WorkspaceJail` muda dependendo se o usuário já forneceu `--workspace` ou `ATOMWRITE_WORKSPACE`
- Quando workspace É fornecido: `"use a path inside the workspace (<root>)"`
- Quando workspace NÃO é fornecido: `"set --workspace <root> or export ATOMWRITE_WORKSPACE=<path>"`
- `FileImmutable` sugere `chattr -i` (Unix) ou `fsutil` (Windows) para limpar o atributo imutável
- `NoMatches` orienta o usuário a ampliar o padrão e revisar filtros `--include`/`--exclude`
- `BinaryFile` recomenda `read --stat` para leituras somente de metadados (não referencia mais a flag phantom `--force-text` removida na v0.1.4)
- `PermissionDenied` retries são automáticos com backoff exponencial (específico de Windows via `persist_with_retry`)

Exemplo de envelope de erro context-aware (quando workspace NÃO é fornecido):
```json
{"error":true,"code":"WORKSPACE_JAIL","exit":126,"message":"path outside workspace jail: /etc/passwd (workspace: /home/user/project)","path":"/etc/passwd","error_class":"precondition_failed","retryable":false,"suggestion":"set --workspace <root> or export ATOMWRITE_WORKSPACE=<path>","workspace":"/home/user/project"}
```

Exemplo quando workspace É fornecido via `--workspace /home/user/project`:
```json
{"error":true,"code":"WORKSPACE_JAIL","exit":126,"message":"path outside workspace jail: /etc/passwd (workspace: /home/user/project)","path":"/etc/passwd","error_class":"precondition_failed","retryable":false,"suggestion":"use a path inside the workspace (/home/user/project)","workspace":"/home/user/project"}
```


## Instalação no Windows (v0.1.4)
- v0.1.4 finalmente corrige `cargo install atomwrite` no Windows 10/11
- Pré-requisito: Visual Studio 2019+ Build Tools com workload "Desenvolvimento para desktop com C++"
- Pré-requisito: Rust 1.88 ou posterior
- Terminal recomendado: Windows Terminal ou PowerShell 7+ (para output UTF-8 e sequências ANSI)
- Veja [INSTALL.md](INSTALL.md) para o guia completo de instalação Windows 10/11 com troubleshooting
