# Livro de Receitas do atomwrite


[Read in English](COOKBOOK.md)

> Receitas práticas que você pode copiar e colar nos seus workflows de agente


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
