# Livro de Receitas do atomwrite


[Read in English](COOKBOOK.md)

> Receitas práticas que você pode copiar e colar nos seus workflows de agente


## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças relevantes para receitas em v0.1.12. A release v0.1.12 adiciona novas receitas ao cookbook para os 6 novos subcomandos, as novas flags, e o workflow de recuperação de crash.

### Novas Receitas (Adicionadas em v0.1.12)

- **Como Editar um Arquivo de Config Sem Reescrevê-lo** -- use `set`/`get`/`del` em vez de `read`+`edit` para caminhos dotted em TOML/JSON
- **Como Renomear um Identificador em um Módulo** -- use `case --subvert OLD NEW --to <style>` para 5 estilos de case
- **Como Caminhar um AST de Código** -- use `query --kinds` para listar kinds, `query --query KIND` para filtrar
- **Como Extrair um Mapa de Código** -- use `outline --positions` para funções/structs/enums/traits
- **Como Detectar Erros de Sintaxe Antes do Commit** -- use `write --syntax-check` para validação tree-sitter
- **Como Recuperar de uma Escrita com Crash** -- use `recover_orphan_journals` para inspecionar sidecars WAL
- **Como Compor read com sed/awk** -- use `read --format raw` para composabilidade Unix
- **Como Limitar Search a Arquivos Pequenos** -- use `search --max-filesize` e `--max-columns`
- **Como Substituir Strings Literais** -- use `replace --literal` para desabilitar regex
- **Como Aplicar Refactor Multi-Rule** -- use `transform --rules <file.yaml>` para regras em cascata
- **Como Adquirir Lock Advisory de Arquivo** -- use `write --lock` e `--lock-timeout` para segurança multi-agente
- **Como Fazer Backup com CoW** -- use `backup` e `copy` com reflink padrão em APFS/btrfs/XFS

### Receitas Atualizadas (mudanças v0.1.12 refletidas)

- **Como Escrever um Arquivo Atomicamente** -- agora menciona `--syntax-check`, `--lock`, `--include-fifo`
- **Como Editar um Arquivo** -- agora menciona `--after-line`, `--before-line`, `--range`, `--delete-range`, `--between`
- **Como Ler um Arquivo** -- agora menciona `--format raw`, `--head N`, `--tail N`, `--line N`, `--grep`
- **Como Buscar** -- agora menciona `--max-filesize`, `--max-columns`
- **Como Substituir Texto em Massa** -- agora menciona `--literal`, `--fuzzy`
- **Como Transformar Código** -- agora menciona `--rules`, `--inline-rules`
- **Como Operar em Batch** -- agora menciona `--batch-size`, `--file`
- **Como Fazer Backup** -- agora menciona `--no-reflink`, `--output-dir`

### Novos Subcomandos Disponíveis

- `set` -- escreve um valor em um caminho dotted em TOML/JSON
- `get` -- lê um valor em um caminho dotted
- `del` -- remove uma chave em um caminho dotted
- `case` -- renomeia identificadores em 5 estilos de case via `heck`
- `query` -- caminha um AST tree-sitter
- `outline` -- extrai estrutura de alto nível

### Novas Flags para Comandos Existentes

- `read --format raw` (G81)
- `write --syntax-check` (G72)
- `write --lock` e `--lock-timeout` (G54)
- `search --max-filesize`, `--max-columns` (G68)
- `replace --literal` (G66)
- `transform --rules`, `--inline-rules` (G44)
- `batch --batch-size` (G77)
- `backup/copy --no-reflink` (G64)

### Cobertura de Testes

- 542 testes passando (461 baseline v0.1.15 + 8 G117 edge cases v0.1.18 + 2 G118 replace pre-validation v0.1.18 + 16 incrementos cross-platform/WAL/auditoria v0.1.16-v0.1.18)
- 9 ADRs em `docs/decisions/` (0019-0027)
- 7 novos JSON schemas em `docs/schemas/`
- Veja [docs/decisions/README.md](README.md) para decisões arquiteturais

## Nota de Latência
- Todas as operações executam localmente com overhead sub-milissegundo
- A sequência de escrita atômica adiciona ~1ms para o ciclo fsync-rename-fsync
- O paralelismo de busca escala com os cores de CPU disponíveis
- O modo batch amortiza o custo de startup entre N operações


## Referência de Valores Padrão
- `--threads` padrão é o número de cores de CPU disponíveis
- `--max-filesize` padrão é 1 GiB (1.073.741.824 bytes)
- `--color` padrão é `auto` (detecta terminal)
- `--workspace` padrão é o diretório de trabalho atual
- `diff --context` padrão é 3 linhas
- `diff --algorithm` padrão é `patience`


## Como Escrever um Arquivo Atomicamente
- Envie conteúdo via stdin para criar ou sobrescrever um arquivo
- A escrita sobrevive a falhas de energia e crashes de processo

```bash
echo "fn main() { println!(\"hello\"); }" | atomwrite write src/main.rs
```

- Crie um backup antes de sobrescrever:

```bash
cat updated_config.toml | atomwrite write --backup config.toml
```

- Escreva com restrição de workspace:

```bash
echo "data" | atomwrite write --workspace /home/user/project src/data.txt
```


## Como Normalizar Line Endings
- Force line endings LF ao escrever:

```bash
echo "line1\r\nline2\r\n" | atomwrite write --line-ending lf src/file.txt
```

- Force CRLF para compatibilidade Windows:

```bash
cat unix_file.txt | atomwrite write --line-ending crlf src/windows_file.txt
```

- Preserve line endings originais (padrão):

```bash
cat source.txt | atomwrite write --line-ending auto src/output.txt
```

- Normalize line endings durante edição:

```bash
atomwrite edit --line-ending lf src/mixed.rs --old "old_text" --new "new_text"
```


## Como Buscar em Todo o Projeto
- Busque um padrão em todos os arquivos de um diretório:

```bash
atomwrite search 'TODO' src/
```

- Busque com regex e linhas de contexto:

```bash
atomwrite search --regex 'fn\s+test_\w+' --context 2 src/
```

- Obtenha apenas caminhos de arquivos com matches:

```bash
atomwrite search --files 'deprecated' src/
```

- Obtenha contagem de matches por arquivo:

```bash
atomwrite search --count 'unwrap()' src/
```

- Combine com extract para obter campos específicos:

```bash
atomwrite search 'TODO' src/ | atomwrite extract path line_number lines
```


## Como Substituir Texto em Massa
- Substitua uma string em todos os arquivos de um diretório:

```bash
atomwrite replace 'old_function' 'new_function' src/
```

- Visualize substituições sem modificar arquivos:

```bash
atomwrite replace --dry-run 'before' 'after' src/
```

- Substitua com regex:

```bash
atomwrite replace --regex 'v\d+\.\d+\.\d+' 'v2.0.0' src/
```

- Substitua com restrição de workspace:

```bash
atomwrite replace --workspace /home/user/project 'old' 'new' src/
```


## Como Escopar Código por Categoria Gramatical
- Delete todos os comentários de um arquivo Rust:

```bash
atomwrite scope --query comments --delete src/main.rs
```

- Coloque em maiúsculas todos os nomes de função em Python:

```bash
atomwrite scope --query def --action upper src/app.py
```

- Comprima espaços em branco em strings:

```bash
atomwrite scope --query strings --action squeeze src/lib.rs
```

- Substitua comentários por um cabeçalho padrão:

```bash
atomwrite scope --query comments --replace-with "// TODO: revisar" src/main.rs
```

- Use padrão AST customizado para titlecase:

```bash
atomwrite scope --pattern 'fn $NAME($$$ARGS)' --action titlecase -l rust src/
```


## Como Criar e Restaurar Backups
- Crie um backup com timestamp e checksum BLAKE3:

```bash
atomwrite backup src/main.rs src/lib.rs
```

- Visualize a criação de backup sem escrever:

```bash
atomwrite backup --dry-run src/main.rs
```

- Defina retenção de backup para 30 dias:

```bash
atomwrite backup --retention 30 src/config.toml
```

- Restaure o backup mais recente:

```bash
atomwrite rollback src/main.rs --latest
```

- Restaure um backup específico por timestamp:

```bash
atomwrite rollback src/main.rs --timestamp 2026-05-29T12-00-00
```

- Verifique checksum antes de restaurar:

```bash
atomwrite rollback --verify src/main.rs --latest
```

- Visualize restauração sem aplicar:

```bash
atomwrite rollback --dry-run src/main.rs --latest
```


## Como Aplicar Patches a Partir do Stdin
- Aplique um patch de diff unificado:

```bash
cat fix.patch | atomwrite apply src/main.rs
```

- Aplique um patch em formato markdown-fenced:

```bash
cat changes.md | atomwrite apply --format markdown src/main.rs
```

- Aplique blocos SEARCH/REPLACE de um agente:

```bash
cat agent_output.txt | atomwrite apply --format search-replace src/main.rs
```

- Aplique com backup automático antes do patching:

```bash
cat fix.patch | atomwrite apply --backup src/main.rs
```

- Visualize aplicação do patch sem modificar:

```bash
cat fix.patch | atomwrite apply --dry-run src/main.rs
```

- Aplique substituição completa de arquivo:

```bash
cat new_version.rs | atomwrite apply --format full src/main.rs
```


## Como Refatorar Com Padrões AST
- Renomeie uma função em toda a codebase Rust:

```bash
atomwrite transform -p 'old_fn($$$ARGS)' -r 'new_fn($$$ARGS)' -l rust src/
```

- Migre de println para tracing:

```bash
atomwrite transform -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/
```

- Substitua todas as chamadas unwrap pelo operador `?`:

```bash
atomwrite transform -p '$EXPR.unwrap()' -r '$EXPR?' -l rust src/
```

- Migre JavaScript console.log:

```bash
atomwrite transform -p 'console.log($$$ARGS)' -r 'logger.info($$$ARGS)' -l js src/
```

- Visualize transform AST sem aplicar:

```bash
atomwrite transform --dry-run -p 'old_api($$$ARGS)' -r 'new_api($$$ARGS)' -l python src/
```


## Como Gerar Regex a Partir de Exemplos
- Gere um padrão regex de data:

```bash
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"
```

- Gere com generalização de dígitos e palavras:

```bash
atomwrite regex --digits --words "user_123" "admin_456" "guest_789"
```

- Use a regex gerada em uma busca:

```bash
PATTERN=$(atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" | atomwrite extract regex)
atomwrite search --regex "$PATTERN" src/
```


## Como Calcular Conversões de Unidades
- Converta unidades de tempo:

```bash
atomwrite calc "2 hours + 30 minutes to seconds"
```

- Converta tamanhos de dados:

```bash
atomwrite calc "10 GiB to MB"
```

- Avalie expressões matemáticas:

```bash
atomwrite calc "sqrt(144) + 3^2"
```

- Calcule porcentagens:

```bash
atomwrite calc "15% of 200"
```


## Como Executar Batch de Múltiplas Operações
- Batch suporta 7 operações: write, replace, delete, edit, hash, move, copy
- Crie um manifesto NDJSON com múltiplas operações:

```bash
cat <<'EOF' > manifest.ndjson
{"op":"write","path":"src/a.txt","content":"hello"}
{"op":"write","path":"src/b.txt","content":"world"}
{"op":"delete","path":"src/old.txt"}
{"op":"edit","path":"src/a.txt","old":"hello","new":"hello world"}
{"op":"hash","path":"src/b.txt"}
{"op":"move","source":"src/a.txt","target":"src/renamed.txt"}
{"op":"copy","source":"src/b.txt","target":"src/b_copy.txt"}
EOF
cat manifest.ndjson | atomwrite batch
```

- Visualize o batch sem executar:

```bash
cat manifest.ndjson | atomwrite batch --dry-run
```

- Execute como transação tudo-ou-nada com rollback automático em falha:

```bash
cat manifest.ndjson | atomwrite batch --transaction
```

- Gere um manifesto a partir de resultados de busca:

```bash
atomwrite search --files 'deprecated' src/ | \
  atomwrite extract path | \
  while read -r p; do echo "{\"op\":\"delete\",\"path\":\"$p\"}"; done | \
  atomwrite batch --dry-run
```


## Como Verificar Integridade de Arquivos
- Calcule o hash de um arquivo e armazene o checksum:

```bash
atomwrite hash src/main.rs
```

- Verifique um arquivo contra um checksum conhecido:

```bash
atomwrite hash --verify abc123def456 src/main.rs
```

- Calcule hash a partir do stdin:

```bash
echo "data" | atomwrite hash --stdin
```

- Compare dois arquivos para verificar diferenças:

```bash
atomwrite diff --stat src/old.rs src/new.rs
```


## Como Usar Locking Otimista
- Desde a v0.1.15 o alvo é resolvido contra o `--workspace` antes da verificação (G118); na v0.1.14 e anteriores, execute com CWD = workspace ou um alvo relativo pula silenciosamente a checagem de checksum
- Leia um arquivo e capture o checksum:

```bash
CHECKSUM=$(atomwrite read --stat src/config.toml | atomwrite extract checksum)
```

- Escreva com o checksum esperado:

```bash
echo "updated content" | atomwrite write --expect-checksum "$CHECKSUM" src/config.toml
```

- Trate desvio de estado (exit code 82):

```bash
echo "updated content" | atomwrite write --expect-checksum "$CHECKSUM" src/config.toml
if [ $? -eq 82 ]; then
  echo "File changed by another process, re-reading..."
  CHECKSUM=$(atomwrite read --stat src/config.toml | atomwrite extract checksum)
  echo "updated content" | atomwrite write --expect-checksum "$CHECKSUM" src/config.toml
fi
```


## Como Editar E Disparar Build Sem Touch Manual
- Edite um arquivo fonte em um projeto Rust e dispare o cargo sem rodar `touch` manualmente:

```bash
atomwrite edit src/main.rs --old "old_text" --new "new_text"
cargo build
```

- Isso funciona porque o `edit` atualiza o mtime por padrão, então o cargo vê o fonte como mais novo que o arquivo dep-info e recompila.
- Se você desativar a atualização de mtime com `--preserve-timestamps`, o cargo pode pular o rebuild silenciosamente (o famoso no-op `Finished in 0.29s`):

```bash
atomwrite edit --preserve-timestamps src/main.rs --old "old_text" --new "new_text"
cargo build  # pode ser um no-op silencioso, forçando você a tocar o arquivo manualmente
```

- Verifique se o mtime foi preservado lendo o campo `mtime_preserved` na resposta NDJSON:

```bash
atomwrite edit src/main.rs --old "old" --new "new" | atomwrite extract mtime_preserved
```

- Use `--preserve-timestamps` apenas para cenários de backup, snapshot ou builds reproduzíveis. Para desenvolvimento interativo, mantenha o padrão para que sistemas de build detectem suas mudanças.


## Como Criar Backups Com Retenção
- Escreva um arquivo com backup automático:

```bash
echo "new content" | atomwrite write --backup src/config.toml
```

- Delete um arquivo com backup:

```bash
atomwrite delete --backup src/old_module.rs
```

- Defina período de retenção para backups:

```bash
atomwrite delete --backup --retention 30 src/old_module.rs
```

- Liste arquivos de backup em um diretório:

```bash
atomwrite list --long .atomwrite-backups/
```


## Como Extrair Campos de Pipeline NDJSON
- Use extract para extrair campos específicos da saída do atomwrite
- Use nomes de campo para chaves JSON ou índices posicionais para colunas de texto

```bash
atomwrite search 'TODO' src/ | atomwrite extract path line_number lines
```

- Extraia apenas paths de resultados de busca:

```bash
atomwrite search --files 'error' src/ | atomwrite extract path
```

- Extraia checksums de resultados de escrita:

```bash
echo "data" | atomwrite write src/file.txt | atomwrite extract checksum
```

- Extraia colunas de texto por índice:

```bash
echo "a b c d" | atomwrite extract 0 2
```


## Como Listar Estrutura do Projeto
- Liste arquivos com saída NDJSON:

```bash
atomwrite list src/
```

- Formato longo com tamanho, permissões e data de modificação:

```bash
atomwrite list --long src/
```

- Conte arquivos agrupados por extensão:

```bash
atomwrite list --count-by-ext src/
```

- Combine com extract para visões customizadas:

```bash
atomwrite list --long src/ | atomwrite extract path bytes
```


## Operações de Escopo
### Deletar todos os comentários de arquivos Rust

```bash
atomwrite --workspace . scope src/ --lang rust --query comments --delete
```

### Converter nomes de função para maiúsculas (prévia)

```bash
atomwrite --workspace . scope src/ --lang rust --query fn --action upper --dry-run
```

### Remover comentários de scripts Python

```bash
atomwrite --workspace . scope scripts/ --lang python --query comments --delete
```


## Backup e Rollback
### Criar backup antes de edição arriscada

```bash
atomwrite --workspace . backup src/config.rs
echo "new config" | atomwrite --workspace . write src/config.rs
```

### Restaurar do backup mais recente

```bash
atomwrite --workspace . rollback src/config.rs
```

### Restaurar de timestamp específico com verificação

```bash
atomwrite --workspace . rollback src/config.rs --timestamp 20260530_120000 --verify
```


## Aplicação de Patches
### Aplicar substituição completa de arquivo

```bash
echo "new content" | atomwrite --workspace . apply src/file.txt --format full
```

### Aplicar diff unificado do Git

```bash
git diff src/file.rs | atomwrite --workspace . apply src/file.rs
```

### Aplicar blocos SEARCH/REPLACE

```bash
cat <<'EOF' | atomwrite --workspace . apply src/main.rs
<<<< SEARCH
old_function_name
==== REPLACE
new_function_name
>>>> END
EOF
```


## Padrões Agent-First (v0.1.2+)

### Limite de Tempo em Busca Longa

```bash
# Aborta após 60s se a busca não terminar; emite erro NDJSON com error_class=transient
atomwrite --workspace . --timeout 60 search 'TODO' src/
```

### Ler Apenas Linhas que Casam com Regex

```bash
# Útil para extrair logs de arquivos enormes sem esgotar o contexto
atomwrite --workspace . read --grep 'ERROR|WARN' /var/log/app.log
```

### Ler Primeiras N Linhas de Arquivo Enorme

```bash
# Evita carregar o arquivo inteiro no contexto
atomwrite --workspace . read --head 20 huge.log
```

### Batch a partir de Arquivo em vez de stdin

```bash
# Arquivo de manifesto persistido (NDJSON, uma op por linha)
atomwrite --workspace . batch --file ops.ndjson
```

### Backup em Diretório Centralizado

```bash
# Mantém o diretório de origem limpo; centraliza backups
atomwrite --workspace . backup --output-dir /var/backups/atomwrite src/critical.rs
```

### Instalar Completions de Shell no Primeiro Uso

```bash
# Auto-instala em ~/.local/share/bash-completion/completions/atomwrite
atomwrite completions bash --install
```

### Usar Variável de Ambiente para Workspace

```bash
# Para agentes que não passam --workspace explicitamente
export ATOMWRITE_WORKSPACE=/home/usuario/projeto
atomwrite read src/main.rs
```


## Padrões Agent-First (v0.1.3+)

### Editar e Disparar Build do Cargo Sem Touch Manual

```bash
# Novo padrão: edit atualiza o mtime, então o cargo rebuilda automaticamente
atomwrite edit src/main.rs --old "texto_antigo" --new "texto_novo"
cargo build  # rebuilda sem precisar de `touch` antes
```

### Ler mtime_preserved Da Resposta de Edit

```bash
# Parse a resposta NDJSON para verificar se o timestamp foi mantido
atomwrite edit src/main.rs --old "antigo" --new "novo" | atomwrite extract mtime_preserved
```

### Preservar mtime Original Para Workflows de Backup ou Snapshot

```bash
# Voltar ao comportamento v0.1.2 de preservar o mtime original do arquivo
atomwrite edit --preserve-timestamps src/snapshot.rs --old "antigo" --new "novo"
atomwrite replace --preserve-timestamps 'old_api' 'new_api' src/
```


## Como Interpretar Sugestões de Erro (v0.1.4)
- Todo envelope de erro inclui um campo `suggestion` com orientação acionável de recuperação
- A sugestão de `WorkspaceJail` se adapta com base em se `--workspace` foi fornecido
- Use a sugestão para guiar a lógica de retry do agente em vez de parsear o texto da mensagem

```bash
# Quando workspace NÃO é fornecido, a solicitação da flag é sugerida
atomwrite read /etc/passwd 2>/dev/null
# Saída: {"error":true,"code":"WORKSPACE_JAIL","exit":126,...,"suggestion":"set --workspace <root> or export ATOMWRITE_WORKSPACE=<path>",...}

# Quando workspace É fornecido, a sugestão diz "use a path inside"
atomwrite --workspace /home/user/project read /etc/passwd 2>/dev/null
# Saída: {"error":true,"code":"WORKSPACE_JAIL","exit":126,...,"suggestion":"use a path inside the workspace (/home/user/project)",...}
```


## Como Instalar no Windows 10/11 (v0.1.4)
- v0.1.4 finalmente corrige `cargo install atomwrite` no Windows
- Instale Visual Studio 2019+ Build Tools com workload C++
- Instale Rust 1.88+ via rustup
- Execute `cargo install atomwrite --locked`
- Veja [INSTALL.md](INSTALL.md) para o guia completo de troubleshooting Windows

```powershell
# PowerShell 7+ ou Windows Terminal
rustup default stable
rustup target add x86_64-pc-windows-msvc
cargo install atomwrite --locked
atomwrite --version  # Saída NDJSON
```


## Como Descobrir o JSON Schema em Runtime
- Use `--json-schema` para emitir o JSON Schema da saída de qualquer subcomando
- Sem necessidade de ler arquivo de schema estático; o schema faz parte do binário

```bash
# Obter schema da saída do subcomando read
atomwrite --json-schema read
atomwrite --json-schema write
atomwrite --json-schema edit
atomwrite --json-schema search
atomwrite --json-schema replace
atomwrite --json-schema batch
atomwrite --json-schema error  # compartilhado por todos os subcomandos
```

- Conecte o schema ao `jaq` para validar saída ao vivo:

```bash
# 1. Capturar o schema
atomwrite --json-schema error > /tmp/error.schema.json

# 2. Executar o comando real e validar cada linha NDJSON
atomwrite --workspace . read /missing 2>/dev/null \
  | while IFS= read -r line; do
      echo "$line" | jaq -r --validate --slurpfile s /tmp/error.schema.json '.' && echo "OK" || echo "FAIL"
    done
```


## Como Ler NDJSON em Pipeline Shell com jaq
- Toda saída do atomwrite é um objeto JSON por linha
- Combine com `jaq` para filtragem estruturada, mapeamento e agregação

```bash
# Extrair apenas o checksum de uma resposta de read
atomwrite read src/main.rs | jaq -r '.checksum'

# Contar matches de search por arquivo
atomwrite search 'TODO' src/ | jaq -r '.path' | sort | uniq -c | sort -rn

# Somar bytes_written em um batch
atomwrite batch < manifest.ndjson | jaq -s 'map(.bytes_written // 0) | add'

# Filtrar envelopes de erro por classe
atomwrite read /missing 2>/dev/null | jaq 'select(.error_class == "permanent")'
```


## Como Lidar com Erros Persistentes com Lógica de Retry
- Combine `retryable: true` dos envelopes de erro com `set -e` e loop de retry em shell

```bash
#!/usr/bin/env bash
# retry-on-transient.sh
attempt=1
max_attempts=5
delay=1

while [ $attempt -le $max_attempts ]; do
  output=$(atomwrite --workspace . "$@" 2>/dev/null)
  exit_code=$?

  if [ $exit_code -eq 0 ]; then
    echo "$output"
    exit 0
  fi

  # Parsear flag retryable do envelope de erro
  retryable=$(echo "$output" | jaq -r '.retryable // false')

  if [ "$retryable" = "true" ]; then
    echo "Tentativa $attempt: erro transiente, retentando em ${delay}s..." >&2
    sleep $delay
    delay=$((delay * 2))
    attempt=$((attempt + 1))
  else
    echo "$output" >&2
    exit $exit_code
  fi
done

echo "Falhou após $max_attempts tentativas" >&2
exit 1
```


## Como Aplicar Múltiplos Pares de Edit Com Segurança (v0.1.15, G117)


## Gerenciamento de Journals WAL (v0.1.15+)

O esforço G119 introduz três novas camadas para gerenciar os journals sidecar do WAL. Estas receitas mostram o uso prático dos novos subcomandos e flags.

### Como Reap de Journals Stale Antes do Commit

O subcomando `wal-heal` (G119 L3) remove com segurança journals terminais (Committed e Aborted). Integre-o a um hook de pre-commit para manter a working tree limpa.

```bash
# .git/hooks/pre-commit
atomwrite --workspace . wal-heal --threshold-secs 0 \
  | jaq -e '.removed_count >= 0' \
  || { echo "wal-heal falhou"; exit 1; }
```

A flag `--threshold-secs 0` remove journals terminais de qualquer idade. `wal-heal` NUNCA toca em entradas Started. Para uma varredura mais conservadora, eleve o threshold para reter journals recentes para análise forense.

### Como Configurar Política de WAL por Carga de Trabalho

A flag `--wal-policy` (G119 L1) em `write` e `edit` controla quando o journal sidecar é escrito. Três valores são aceitos:

- `auto` (padrão) -- política escolhida pelo build, otimizada para uso geral
- `always` -- sempre escreve o journal sidecar (trilha de auditoria forense)
- `never` -- nunca escreve o journal sidecar (caminho mais rápido, sem metadados de recovery)

```bash
# Builds de CI: pula overhead de journal, sidecars não têm consumidor lá
atomwrite --workspace . write --wal-policy never ci-config.toml < config.toml

# Deploys de produção: trilha de auditoria importa, força o sidecar
atomwrite --workspace . write --wal-policy always /etc/myapp/config.toml < prod.toml

# Uso geral por agente: deixa o padrão decidir
atomwrite --workspace . write src/lib.rs < new_lib.rs
```

### Como Inspecionar Saúde dos Journals

O subcomando `wal-stats` (G119 L5) emite telemetria read-only sobre o estado atual do WAL. Combine-o com `jaq` para gatear CI ou scripts pós-build.

```bash
# Telemetria completa como NDJSON
atomwrite --workspace . wal-stats

# Gate em journals com reclaimable zero
atomwrite --workspace . wal-stats | jaq -e '.reclaimable == 0' || { echo "drift"; exit 1; }

# Extrai apenas o stale threshold para diagnóstico
atomwrite --workspace . wal-stats | jaq -r '.stale_threshold_secs'
```


- Pares repetidos `--old`/`--new` rodam a cascata fuzzy completa de 9 estratégias por par
- O padrão é all-or-nothing: um par falho aborta o lote com exit 65, sem escrita, e `failed_pair_index` no envelope de erro
- `--partial` aplica os pares que casam e relata os demais com `matched: false`

```bash
# All-or-nothing com ground truth por par
atomwrite --workspace . edit src/main.rs --old "foo" --new "bar" --old "baz" --new "qux" \
  | jaq -e '.pair_results'

# Aplicação parcial: preserva o trabalho válido e lista os ausentes
atomwrite --workspace . edit --partial src/main.rs --old "foo" --new "bar" --old "talvez" --new "x" \
  | jaq -e '{edits, pairs_total, ausentes: [.pair_results[] | select(.matched | not) | .index]}'

# Anti-mascaramento: pipe sem -e esconde o exit 65 como {"edits": null} com exit 0 no pipeline
atomwrite --workspace . edit src/main.rs --old "ausente" --new "x" | jaq -e '.edits' \
  || echo "edit falhou: ${PIPESTATUS[0]}" >&2
```


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


## v0.1.21 — Receitas

### Backup Deletado Após Sucesso

- **Mudança de padrão** — `write --backup` não deixa mais um sibling `.bak.<timestamp>` em disco após sucesso. O backup é deletado.
- **Opt-in para preservar** — passe `--keep-backup` para qualquer um de `write`, `edit`, `replace`, `rollback`, `apply`, `batch`:

```bash
# Default v0.1.21: backup é criado, escrita sucede, backup é deletado
echo "novo" | atomwrite --workspace . write --backup config.toml

# Opt-in v0.1.21: backup é criado, escrita sucede, backup é preservado
echo "novo" | atomwrite --workspace . write --backup --keep-backup config.toml
```

### Padrão de Edits Sequenciais com Re-captura de Checksum

- **Padrão A — re-captura explícita** é a forma canônica de encadear N chamadas `edit` no mesmo arquivo:

```bash
#!/usr/bin/env bash
set -euo pipefail
for par in "foo:bar" "baz:qux" "alpha:beta"; do
  OLD="${par%:*}"
  NEW="${par#*:}"
  CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
  echo "$NEW" | atomwrite --workspace . edit --old "$OLD" --new "$NEW" \
    --expect-checksum "$CS" src/foo.rs
done
```

- **Padrão B — opt-in `--allow-sequential-drift`** para scripts one-shot que preferem não re-capturar:

```bash
CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
for par in "foo:bar" "baz:qux"; do
  OLD="${par%:*}"; NEW="${par#*:}"
  echo "$NEW" | atomwrite --workspace . edit --allow-sequential-drift \
    --old "$OLD" --new "$NEW" --expect-checksum "$CS" src/foo.rs
done
```


## v0.1.22 — Receitas

### `edit-loop` para N Pares em 1 Invocação

- **Caso de uso**: aplicar um lote de transformações textuais em um arquivo (renomear identificador em 7 lugares, varrer aliases obsoletos) onde hoje você invocaria `edit` N vezes em loop shell.

```bash
# Aplicar 3 pares em 1 invocação
printf '%s\n' \
  '{"old":"v0_1_20","new":"v0_1_22"}' \
  '{"old":"foo","new":"bar"}' \
  '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace . edit-loop src/version.rs

# Com backup preservado para linha do tempo forense
printf '%s\n' '{"old":"foo","new":"bar"}' \
  | atomwrite --workspace . edit-loop --backup --keep-backup src/foo.rs

# Com --partial (best-effort: aplica matched, reporta unmatched)
printf '%s\n' '{"old":"existe","new":"X"}' '{"old":"ausente","new":"Y"}' \
  | atomwrite --workspace . edit-loop --partial src/foo.rs
```

### `prune-backups` para Limpeza Manual de `.bak.*` Legados

- **Caso de uso**: operadores que atualizaram de v0.1.20 herdam siblings `.bak.<timestamp>` que v0.1.21 não cria mais (e portanto não limpa mais automaticamente).

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


## v0.1.24 — Receitas

### Tratamento de Erros Tipados em Pipelines de Agentes

Todos os erros agora emitem JSON estruturado no stdout. Parse exit codes de forma determinística:

```bash
# Pipeline seguro: verificar exit code ANTES de parsear stdout
output=$(atomwrite --workspace . get config.toml database.pool.max 2>/dev/null)
exit_code=$?
case $exit_code in
  0) echo "$output" | jaq -r '.value' ;;
  4) echo "chave não encontrada" ;;
  65) echo "$output" | jaq -r '.suggestion' ;;
  *) echo "inesperado: exit $exit_code" ;;
esac
```

### Delete Recursivo (Agora Funciona)

```bash
# v0.1.24: delete --recursive agora percorre e remove
atomwrite --workspace . delete --recursive --yes logs/

# Dry-run primeiro para pré-visualizar
atomwrite --workspace . delete --recursive --dry-run logs/
```

### Hash Recursivo (Agora Funciona)

```bash
# v0.1.24: hash --recursive caminha diretórios
atomwrite --workspace . hash --recursive src/
```

### Search Multiline (Agora Funciona)

```bash
# v0.1.24: padrões multiline agora casam através de linhas
atomwrite --workspace . search --multiline 'fn main\(\).*\{' src/ --include '*.rs'
```

### Replace Rejeita Padrão Vazio

```bash
# v0.1.24: padrão vazio é rejeitado (antes era destrutivo silenciosamente)
atomwrite --workspace . replace '' 'X' src/
# Exit 65: INVALID_INPUT — padrão vazio casaria em toda posição

# Uso correto: padrão explícito
atomwrite --workspace . replace 'old_api' 'new_api' src/
```

### Timestamp de Backup com Milissegundos

```bash
# v0.1.24: timestamps incluem milissegundos para prevenir colisão
atomwrite --workspace . backup config.toml
# Cria: config.toml.bak.20260621_143022_847

# Rollback aceita match por prefixo (retrocompatível)
atomwrite --workspace . rollback config.toml --timestamp 20260621_143022
```

### Get Valores Sem Aspas Duplas

```bash
# v0.1.24: get retorna valor raw, não string serializada JSON
atomwrite --workspace . get Cargo.toml package.version
# Saída: {"type":"result","value":"0.1.24",...}
# NÃO: {"type":"result","value":"\"0.1.24\"",...}  (comportamento antigo)
```
