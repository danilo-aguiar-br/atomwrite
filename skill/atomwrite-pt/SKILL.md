---
name: atomwrite
description: |
  Use atomwrite para TODAS as operações de arquivo: read, write, edit, search, replace, hash, verify, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, backup, rollback, apply, batch, completions, set, get, del, case, query, outline, wal-stats, wal-heal, edit-loop, prune-backups (33 subcomandos, v0.1.25).
  Auto-invocar: escrever arquivos, buscar código, substituir texto, refatorar AST, gerar regex, calcular, lote, checksums, verificar integridade, listar estrutura, scoping, backup, rollback, patches, config .atomwrite.toml, fuzzy Jaro-Winkler, scope symbols/normalize, delete por idade, preserve-case.
  Palavras-chave: escrita atômica, NDJSON, BLAKE3, verify, .atomwrite.toml, config-file, fuzzy-threshold, Jaro-Winkler, context_aware_jw, diff_preview, pcre2, preserve-case, older-than, scope-symbols, scope-normalize, typed-errors, backup-by-default, shrink-guard, --no-backup, --allow-shrink.
---


# atomwrite
## TL;DR — v0.1.25 (2026-06-22)
### OBRIGATÓRIO
- v0.1.25 fecha 49 GAPs (071-134): config file, verify subcommand, fuzzy Jaro-Winkler, scope symbols/normalize, 3 correções críticas
- 631 testes passam em 63+ suítes (de 621 em v0.1.24; +10 novos)
- 9 ADRs em `docs/decisions/` (0019-0027), 7 novos JSON schemas
- NOVAS FEATURES: `.atomwrite.toml` config file (GAP-072), `verify` subcomando (GAP-079), fuzzy Jaro-Winkler `context_aware_jw` (GAP-085/087/088), `scope --action symbols|normalize` (GAP-081)
- NOVAS FLAGS: `delete --older-than`, `delete --confirm`, `replace --preserve-case`, `search --pcre2`, `edit --fuzzy-threshold`, `scope --action symbols|normalize`
- CORREÇÕES CRÍTICAS: write --backup phantom backup_path (GAP-101), set misroute em TOML escalar (GAP-102), case no-op silencioso (GAP-127)
- MUDANÇAS COMPORTAMENTAIS: hash output `value`→`checksum` (GAP-107), batch move/copy requer `force:true` (GAP-108), get/del missing key→exit 65 (GAP-111)
- BACKUP FIXES: copy/replace --backup retém .bak em disco (GAP-104/105), write --require-backup força retenção (GAP-106)
- v0.1.24 PREVIAMENTE fechou 52 GAPs (019-070)
- v0.1.23 PREVIAMENTE fechou 4 GAPs (015-018)
- v0.1.22 PREVIAMENTE fechou GAP-2026-012 Frente 3 e GAP-2026-013
- v0.1.20 PREVIAMENTE fechou 11 GAP-2026 (001-011)


## v0.1.25 (2026-06-22) — Config, Verify e 49 Correções

### Novas Features
- GAP-072: `.atomwrite.toml` arquivo de configuração com hierarquia: CLI > env > local > XDG > defaults
- GAP-079: subcomando `verify` (delega para `hash --verify` para verificação BLAKE3)
- GAP-085: matching fuzzy Jaro-Winkler (`context_aware_jw`) para strings curtas no edit
- GAP-086: testes property-based para fuzzy matching via proptest (5 propriedades)
- GAP-087: `edit --fuzzy-threshold <FLOAT>` para sensibilidade configurável de matching
- GAP-088: highlighting de diff fuzzy via `similar::TextDiff` em `FuzzyInfo.diff_preview`
- GAP-081: `scope --action symbols|normalize` (ASCII→Unicode e NFC)

### Correções Críticas
- GAP-101: `write --backup` não mais reporta `backup_path` phantom para backups auto-deletados
- GAP-102: `set` não mais roteava chave errada ao descer em valores TOML escalares (agora exit 65)
- GAP-127: `case` sem `--subvert` retorna exit 65 com mensagem clara em vez de no-op silencioso

### Novas Flags
- `delete --older-than <DURATION>` com duração legível (s/m/h/d/w) — GAP-075
- `delete --confirm` como modo preview (mesmo que `--dry-run`) — GAP-076
- `replace --preserve-case` com `adapt_case()` (UPPER/lower/Title) — GAP-077
- `search --pcre2` retorna exit 65 quando feature PCRE2 não está habilitada — GAP-078
- `edit --fuzzy-threshold <FLOAT>` para sensibilidade configurável de matching — GAP-087
- `scope --action symbols` converte operadores ASCII→Unicode — GAP-081
- `scope --action normalize` normaliza texto para NFC — GAP-081

### Mudanças de Comportamento
- GAP-107: campo de saída do `hash` renomeado de `value` para `checksum` (casa com schema)
- GAP-108: `batch` move/copy agora requer `"force":true` para sobrescrever alvos existentes
- GAP-111: `get`/`del` em chave faltante retorna INVALID_INPUT (exit 65) em vez de FILE_NOT_FOUND (exit 4)
- GAP-113: telemetria de risco default mudou para 255 (desabilitado) — usuário faz opt-in via `--risk-threshold`

### Correções de Backup
- GAP-104: `copy --backup` retém arquivo `.bak` em disco (era auto-deletado)
- GAP-105: `replace --backup` retém arquivo `.bak` em disco (era auto-deletado)
- GAP-106: `write --require-backup` força retenção de backup em disco
- GAP-119: `write --auto-rotate` retém backup em disco

### Estatísticas
- 631 testes passando (621 na v0.1.24 + 10 novos)
- 64 gaps auditados: 49 resolvidos, 5 pré-existentes, 10 reclassificados, 0 pendentes
- 9 ADRs em `docs/decisions/` (0019-0027)
- 7 novos JSON schemas

### Padrão Correto — Verificar Checksum (v0.1.25)
```bash
atomwrite --workspace . verify src/main.rs abc123def456
# Exit 0: checksum válido; Exit 81: checksum inválido
```

### Padrão Correto — Arquivo de Configuração (v0.1.25)
```bash
# Criar config local por projeto
echo '[defaults]
backup = true
retention = 3
line_ending = "lf"' | atomwrite --workspace . write .atomwrite.toml
# Hierarquia: CLI > env > .atomwrite.toml local > XDG global > defaults
```

### Padrão Correto — Delete por Idade (v0.1.25)
```bash
atomwrite --workspace . delete --older-than 7d --dry-run logs/
atomwrite --workspace . delete --older-than 1h --yes tmp/
```

### Padrão Correto — Replace Preservando Case (v0.1.25)
```bash
atomwrite --workspace . replace --preserve-case 'error' 'warning' src/
# ERROR→WARNING, error→warning, Error→Warning
```

### Padrão Correto — Fuzzy Threshold (v0.1.25)
```bash
atomwrite --workspace . edit --fuzzy-threshold 0.8 src/main.rs --old "texto aprox" --new "texto exato"
```

### Padrão Correto — Scope Symbols e Normalize (v0.1.25)
```bash
atomwrite --workspace . scope src/ --lang rust --action symbols --dry-run
atomwrite --workspace . scope src/ --lang rust --action normalize --dry-run
```


## v0.1.24 (2026-06-21) — Erros Tipados e 52 Correções

### Overhaul de Tratamento de Erros (GAP-051 a GAP-070)
- 20 chamadas `anyhow::bail!()` convertidas para variantes tipadas `AtomwriteError`
- TODOS os erros retornam envelope NDJSON estruturado com exit codes corretos
- Erros `NotFound` retornam exit 4 (era exit 1): set/del/get arquivo inexistente, query/outline arquivo inexistente, hash arquivo inexistente
- Erros `InvalidInput` retornam exit 65 (era exit 1): pattern vazio, range inválido, formato não suportado, modo ausente, count mismatch, confirm abortado
- Comandos afetados: set, del, get, query, outline, edit, batch, write, prune-backups, extract, replace
- ADR-0045 documenta injeção de sugestão em erros clap para orientação --old-file/--new-file

### Correções Críticas de Bugs
- `delete --recursive` TRAVERSA diretórios via WalkBuilder (GAP-027) — antes pulava conteúdo
- `delete --recursive` remove subdiretórios vazios em ordem contents_first (GAP-038)
- `search --multiline` propaga flag para AMBOS matcher E SearcherBuilder (GAP-037) — antes apenas matcher recebia
- `batch --transaction` rollback cobre operações `move` e `copy` (GAP-046) — antes apenas `write` era revertido
- `read --line`/`--lines` bounds check previne panic em índices fora do range (GAP-035)
- `replace` rejeita pattern vazio com `InvalidInput` exit 65 (GAP-050) — antes casava zero-width entre cada caractere, DESTRUINDO arquivos
- `read --lines`/`--head`/`--tail` com range vazio não mais retorna `"\n"` espúrio (GAP-041)

### Resolução de Paths Contra Workspace (GAP-020, GAP-022)
- `diff` resolve ambos os caminhos contra --workspace (ADR-0046)
- `scope`, `count`, `transform` resolvem walk roots contra --workspace (GAP-022)
- TODOS os comandos que aceitam paths usam a convenção resolve-first (ADR-0027 universal)

### Melhorias de Backup (GAP-023)
- Formato de timestamp alterado para `YYYYMMDD_HHMMSS_mmm` (resolução em milissegundos)
- Previne colisão quando backup-by-default e backup explícito no mesmo segundo
- `rollback --timestamp` aceita PREFIX match — `20260621_120000` casa com `file.bak.20260621_120000_042`
- RETROCOMPATÍVEL: backups antigos sem milissegundos continuam funcionando

### Correções de Quoting de Valores (GAP-039, GAP-042, GAP-043, GAP-045)
- `get` JSON não mais duplica aspas em strings (era `"\"hello\""`, agora `"hello"`)
- `get` TOML não mais inclui aspas circundantes
- `set`/`del` campos `old_value`/`removed_value` corrigidos identicamente
- `set` TOML chave aninhada `old_value` agora resolve (era sempre null)

### Modo Scope Read-Only (GAP-021, GAP-026, ADR-0047)
- `scope` sem `--delete`/`--action`/`--replace-with` reporta `files_matched` corretamente
- `files_modified` é `null` em modo read-only (antes mostrava contagem de matches incorretamente)
- Matching de nós reescrito com `Node::find_all` em vez de DFS manual

### Correções Menores
- `hash --stdin` não mais exige argumento PATHS (GAP-032)
- `hash --recursive` implementa travessia de diretório via WalkBuilder (GAP-048)
- `hash` de arquivo inexistente retorna exit 4 em vez de skip silencioso (GAP-044)
- `edit --multi` aceita JSON `{old,new}` sem campo `op` (GAP-031, GAP-036)
- `regex` remove comportamento greedy de `allow_hyphen_values` (GAP-025, GAP-034) — usar separador `--` para exemplos com hífen
- `case --subvert` aceita exatamente 2 args por ocorrência (GAP-047) — era greedy, consumia path
- `risk_assessment` pulado para operações append/prepend (GAP-024) — previne falso positivo
- `--require-backup` guard verifica estado efetivo do backup (GAP-033)
- `read --format raw` pula heurística binária (GAP-030)
- `wal-stats`/`wal-heal` NDJSON inclui campo `type` (GAP-029)
- `scope --query comments --delete` captura nó `line_comment` completo (GAP-028)
- `prune-backups --max-count` ordena lexicograficamente (GAP-049)
- `get` de chave inexistente retorna exit 4 (GAP-040)


### Padrão Correto — Tratamento de Erros Tipados (v0.1.24)
```bash
# Todos os erros retornam NDJSON estruturado — sem mais exit 1 genérico
atomwrite --workspace . get config.toml chave.inexistente
# Exit 4: {"error":true,"code":"FILE_NOT_FOUND","exit":4,"message":"key not found","suggestion":"check key path"}

# Replace rejeita pattern vazio (era destrutivo silencioso antes da v0.1.24)
atomwrite --workspace . replace '' 'X' src/
# Exit 65: {"error":true,"code":"INVALID_INPUT","exit":65,"message":"empty pattern rejected"}
```

### Padrão Correto — Timestamps de Backup com Milissegundos (v0.1.24)
```bash
# Timestamps de backup agora incluem milissegundos para prevenir colisão
atomwrite --workspace . backup src/main.rs
# Cria: src/main.rs.bak.20260621_120000_042

# Rollback aceita prefix match (retrocompatível com formato antigo)
atomwrite --workspace . rollback src/main.rs --timestamp 20260621_120000
# Casa com formato antigo e novo sufixo _mmm
```

### Padrão Correto — Delete Recursivo (v0.1.24, GAP-027)
```bash
# delete --recursive agora realmente traversa diretórios
atomwrite --workspace . delete --recursive --yes logs/
# Remove todos os arquivos E subdiretórios vazios (ordem contents_first)
```

### Padrão Correto — Busca Multilinha (v0.1.24, GAP-037)
```bash
# --multiline agora propaga corretamente para matcher E searcher
atomwrite --workspace . search --multiline 'fn main\(\).*\{[^}]*\}' src/ --include '*.rs'
```


## v0.1.23 (2026-06-19) — Segurança de Dados por Padrão
### OBRIGATÓRIO
- v0.1.23 fecha 4 GAPs (015-018): `allow_hyphen_values` em 15 campos, backup-by-default, shrink-guard, `--old-file`/`--new-file`
- 609 testes passam em 60+ suítes (de 575+ em v0.1.22; +31 novos)
- 4 ADRs adicionados: ADR-0041 (allow-hyphen-values), ADR-0042 (backup-by-default), ADR-0043 (shrink-guard), ADR-0044 (--old-file/--new-file)
- MUDANÇA COMPORTAMENTAL: `--backup` agora é `true` por padrão em 9 structs (write, edit, replace, apply, batch, set, del, case, transform). USAR `--no-backup` para opt-out explícito
- MUDANÇA COMPORTAMENTAL: `write --expect-checksum` agora BLOQUEIA quando stdin tem menos de 50% do tamanho original. USAR `--allow-shrink` para sobrescrever
- v0.1.22 PREVIAMENTE fechou GAP-2026-012 Frente 3 (edit-loop) e GAP-2026-013 (prune-backups)
- v0.1.21 PREVIAMENTE fechou GAP-2026-012 Frentes 1+2, GAP-2026-013 Problema C, GAP-2026-014 v2
- v0.1.20 PREVIAMENTE fechou 11 GAP-2026 (001-011)
- Todas as adições de v0.1.23 são retrocompatíveis EXCETO backup-by-default (quebrante para scripts que dependem de `backup: false` como default)

## v0.1.23 (2026-06-19) — Segurança de Dados por Padrão
### GAP-2026-015 — allow_hyphen_values em 15 campos CLI
- 15 campos em 8 structs agora aceitam valores que começam com `-`
- Resolve falhas em `edit --old "- item"`, `search "-deprecated"`, `calc "-5 + 3"`
- 12 testes de regressão adicionados
### GAP-2026-016 — backup-by-default em 9 structs
- `--backup` agora é `true` por padrão em write, edit, replace, apply, batch, set, del, case, transform
- USAR `--no-backup` para desabilitar backup explicitamente
- Backup é criado ANTES da sobrescrita e DELETADO após sucesso (comportamento `keep_backup: false` inalterado)
- 7 testes de regressão adicionados
### GAP-2026-017 — shrink-guard com --expect-checksum
- `write --expect-checksum` agora BLOQUEIA quando stdin < 50% do tamanho original (exit 65)
- USAR `--allow-shrink` para permitir truncamento intencional
- Sem `--expect-checksum`, comportamento inalterado
- 4 testes de regressão adicionados
### GAP-2026-018 — --old-file/--new-file para edit
- `edit --old-file <PATH> --new-file <PATH>` lê conteúdo de arquivos em vez do argv
- Elimina limite ARG_MAX (~131 KB) para edições com conteúdo grande
- `conflicts_with` impede mistura de `--old` com `--old-file`
- Validação de cross-mixing: `--old` + `--new-file` retorna exit 65
- Campo `source: "arg"|"file"` adicionado em `pair_results`
- 8 testes de regressão adicionados


## v0.1.22 (2026-06-17) — Sub-comandos prune-backups e edit-loop
- `prune-backups [PATHS]...` — limpa backups `.bak.YYYYMMDD_HHMMSS` legados por idade ou quantidade
- `edit-loop [PATH]` — aplica N pares `{old, new}` via NDJSON no stdin em 1 invocação
- 2 ADRs adicionados (0039, 0040)



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
- SABER que desde a v0.1.15 append/prepend, detecção automática de line ending e `--expect-checksum` resolvem o alvo contra o `--workspace` (G118); na v0.1.14 e anteriores SEMPRE manter CWD = workspace como workaround, ou alvos relativos truncam no append e pulam a verificação de checksum
- USAR `--prepend` para inserir conteúdo no início do arquivo existente
- USAR `--max-size <BYTES>` para limitar tamanho do stdin aceito
- USAR `--line-ending lf|crlf|cr|auto` para normalizar quebras de linha (padrão: auto)
- Resposta inclui `checksum` (BLAKE3) e `bytes_written`
- SABER que desde a v0.1.23 `--backup` é `true` por padrão — backup é criado ANTES da sobrescrita e DELETADO após sucesso
- USAR `--no-backup` para desabilitar backup quando performance for prioridade
- SABER que desde a v0.1.23 `--expect-checksum` BLOQUEIA writes que reduzem o arquivo em mais de 50% (exit 65 com `shrink_blocked: true`)
- USAR `--allow-shrink` para permitir truncamento intencional quando `--expect-checksum` está ativo
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
- USAR `--max-filesize <BYTES>` para pular arquivos maiores que o cap (sobrescreve `--max-filesize` global)
- USAR `--max-columns <N>` para truncar linhas de saída mais largas que N colunas (G68)
- USAR `--include-fifo` para atravessar FIFO/named pipes (G56) — desabilitado por padrão por segurança
- USAR `--pcre2` para habilitar engine PCRE2 (retorna exit 65 se feature não compilada) (v0.1.25, GAP-078)
- Resposta é NDJSON com um objeto por match
### PROIBIDO
- NUNCA tratar exit code 1 como falha em search
- NUNCA usar `--include-fifo` em diretórios não confiáveis (pode travar em pipes lentos)
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
### Padrão Correto — Busca Com Truncamento de Coluna
```bash
atomwrite --workspace . search 'error' src/ --max-columns 120
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
- USAR `--preserve-case` para preservar case do original (UPPER/lower/Title) via `adapt_case()` (v0.1.25, GAP-077)
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
- SABER que desde a v0.1.15 o multi-par `--old`/`--new` roda a cascata fuzzy completa de 9 estratégias por par (G117 corrigido); respostas de sucesso incluem `pairs_total` e `pair_results` (`index` 1-based, `matched`, `strategy`, `similarity`)
- SABER que um par falho aborta o lote inteiro por padrão (all-or-nothing, sem escrita) e o envelope de erro carrega `failed_pair_index`, `pairs_total` e `pair_results`; pares após a falha nunca foram tentados e ficam ausentes
- USAR `--partial` (v0.1.15) para aplicar os pares que casam e relatar os demais com `matched: false`; zero pares aplicados sai com 1 (`NO_MATCHES`) sem escrever
- NUNCA fazer pipe de `edit` para `jaq` sem verificação: o envelope de erro vai para o stdout, então `| jaq '.edits'` mascara o exit 65 como `{"edits": null}` — use `jaq -e '.edits'` ou verifique `${PIPESTATUS[0]}`
- USAR `--after-line N` para inserir conteúdo após uma linha específica
- USAR `--before-line N` para inserir conteúdo antes de uma linha específica
- USAR `--range N:M` para substituir um intervalo de linhas
- USAR `--delete-range N:M` para deletar um intervalo de linhas
- USAR `--after-match "texto"` para inserir conteúdo após primeiro match do texto
- USAR `--before-match "texto"` para inserir conteúdo antes do primeiro match
- USAR `--between "inicio" "fim"` para substituir conteúdo entre dois marcadores
- USAR `--fuzzy auto|off|aggressive` para controlar correspondência aproximada de texto
- USAR `--fuzzy-threshold <FLOAT>` para ajustar sensibilidade do matching fuzzy (0.0-1.0, padrão auto) (v0.1.25, GAP-087)
- SABER que v0.1.25 adiciona estratégia `context_aware_jw` (Jaro-Winkler) para strings curtas (GAP-085)
- SABER que v0.1.25 adiciona campo `diff_preview` em `FuzzyInfo` com highlighting de diferenças (GAP-088)
- USAR `--multi` para aplicar múltiplas edições de uma vez (lê NDJSON do stdin)
- USAR `--expect-checksum <BLAKE3>` para locking otimista
- USAR `--line-ending lf|crlf|cr|auto` para normalizar quebras de linha
- USAR `--preserve-timestamps` para manter o mtime original do arquivo (padrão: mtime é atualizado para refletir a edição). Adicione ao integrar com sistemas de build (cargo, make, cmake) que precisam de timestamps estáveis
- Enviar novo conteúdo via stdin ao usar `--range`, `--after-line` ou `--before-line`
- Nota: `edit` e `replace` agora atualizam o mtime do arquivo por padrão (v0.1.3+). Este é o comportamento correto para cargo/make/cmake detectarem a mudança. Para backup ou builds reproduzíveis, passe `--preserve-timestamps` para manter o timestamp original
- USAR `--old-file <PATH>` para ler conteúdo de match de arquivo em disco (alternativa a `--old` para conteúdo grande)
- USAR `--new-file <PATH>` para ler conteúdo de substituição de arquivo (alternativa a `--new` para conteúdo grande)
- SABER que `--old-file` e `--old` são mutuamente exclusivos via `conflicts_with` — clap emite exit 2 automaticamente
- SABER que cross-mixing (`--old` + `--new-file` ou `--old-file` + `--new`) retorna exit 65 (`INVALID_INPUT`)
- SABER que conteúdo lido de arquivo tem trailing newline removido via `strip_file_trailing_newline()` para paridade com argv
- SABER que a resposta inclui `source: "arg"|"file"` em `pair_results` para rastreabilidade
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
### Padrão Correto — Multi-Par Com Verificação Por Par (v0.1.15)
```bash
# pair_results é o ground truth por item; jaq -e falha o pipe em envelopes de erro
atomwrite --workspace . edit src/main.rs --old "foo" --new "bar" --old "baz" --new "qux" \
  | jaq -e '.pair_results'
```
### Padrão Correto — Aplicação Parcial (v0.1.15)
```bash
# Aplica os pares que casam e relata os ausentes com matched:false
atomwrite --workspace . edit --partial src/main.rs --old "foo" --new "bar" --old "talvez" --new "x" \
  | jaq -e '{edits, pairs_total, ausentes: [.pair_results[] | select(.matched | not) | .index]}'
```
### Antipadrão — Pipe Sem Verificação Mascara Falhas do Edit (G117)
```bash
# PROIBIDO: o exit 65 morre no pipe e o jaq imprime {"edits": null} com exit 0
atomwrite --workspace . edit src/main.rs --old "ausente" --new "x" | jaq '{edits: .edits}'
# OBRIGATÓRIO: jaq -e converte campo ausente em exit 1, ou verifique ${PIPESTATUS[0]}
atomwrite --workspace . edit src/main.rs --old "ausente" --new "x" | jaq -e '.edits'
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
### Padrão Correto — Edição via Arquivo (v0.1.23, GAP-018)
```bash
# Ler conteúdo de match e substituição de arquivos em disco
atomwrite --workspace . edit src/main.rs --old-file old.txt --new-file new.txt
# Múltiplos pares via arquivo
atomwrite --workspace . edit src/main.rs --old-file a.txt --new-file a2.txt --old-file b.txt --new-file b2.txt
# Conteúdo grande (200+ KB) sem limite ARG_MAX
echo "conteúdo grande..." | atomwrite --workspace . write /tmp/old.txt
echo "novo conteúdo..." | atomwrite --workspace . write /tmp/new.txt
atomwrite --workspace . edit target.rs --old-file /tmp/old.txt --new-file /tmp/new.txt
```
### Padrão Correto — Shrink Guard (v0.1.23, GAP-017)
```bash
# Write com expect-checksum BLOQUEIA shrink > 50% por padrão
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
echo "pequeno" | atomwrite --workspace . write --expect-checksum "$CS" src/main.rs
# Exit 65: stdin is 95% smaller than target; pass --allow-shrink to confirm

# Override explícito para truncamento intencional
echo "pequeno" | atomwrite --workspace . write --expect-checksum "$CS" --allow-shrink src/main.rs
```
### Padrão Correto — Backup por Padrão (v0.1.23, GAP-016)
```bash
# v0.1.23: backup é criado automaticamente (e deletado após sucesso)
echo "novo" | atomwrite --workspace . write target.txt
# Desabilitar backup quando performance for prioridade
echo "novo" | atomwrite --workspace . write --no-backup target.txt
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
- USAR `--rules <PATH>` (G44) para carregar múltiplas regras de um arquivo YAML/JSON
- USAR `--inline-rules <JSON>` (G44) para aplicar múltiplas regras de uma string JSON inline
- Ambos `--pattern` e `--rewrite` são OBRIGATÓRIOS no modo single-rule (sem modo somente busca)
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
- USAR `--action symbols` para converter operadores ASCII para Unicode (v0.1.25, GAP-081)
- USAR `--action normalize` para normalizar texto para forma NFC (v0.1.25, GAP-081)
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
- USAR `--transaction` para garantir atomicidade do lote inteiro (falha em uma op reverte todas)
- USAR `--dry-run` para pré-visualizar o lote inteiro
- USAR `--input-schema` para obter o JSON Schema do formato de entrada
- USAR `--batch-size <N>` (G77) para controlar tamanho do chunk para manifestos grandes — útil para streaming com restrição de memória
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
- SABER que desde a v0.1.25 o campo de saída foi renomeado de `value` para `checksum` (GAP-107)
### Padrão Correto — Hash
```bash
atomwrite --workspace . hash src/main.rs
atomwrite --workspace . hash src/*.rs
atomwrite --workspace . hash --verify abc123 src/main.rs
echo "content" | atomwrite hash --stdin
```


## Operações de Verificação (v0.1.25)
### OBRIGATÓRIO
- USAR `verify` para verificar checksum BLAKE3 de um arquivo contra um hash esperado
- Delega internamente para `hash --verify` — wrapper semântico para clareza
- Exit 0 quando checksum casa; exit 81 quando não casa
- Aceita `<PATH> <EXPECTED_HASH>` como argumentos posicionais
### Padrão Correto — Verificação
```bash
atomwrite --workspace . verify src/main.rs abc123def456
# Exit 0: checksum válido
# Exit 81: checksum inválido
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
- USAR `--older-than <DURATION>` para filtrar por idade (s/m/h/d/w) — ex: `7d` para 7 dias (v0.1.25, GAP-075)
- USAR `--confirm` como alias de `--dry-run` para modo preview (v0.1.25, GAP-076)
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
- USAR `--no-reflink` (G64) para desabilitar otimização de reflink (copy-on-write) — força cópia byte a byte completa
- USAR `--preserve-xattr` (G39) para manter extended attributes em copy/move
- USAR `--preserve-hardlinks` (G55) em `move` para manter contagem de hardlinks intacta
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


## Operações Set (v14 Tier 3 — v0.1.12)
### OBRIGATÓRIO
- USAR `set` para escrever um único valor em um arquivo de config TOML ou JSON
- ACEITAR `<PATH> <KEY_PATH> <VALUE>` como argumentos posicionais (auto-detecta TOML vs JSON pela extensão)
- USAR notação dotted path para chaves aninhadas: `package.version`, `database.pool.max`
- USAR `--backup` para criar backup com timestamp antes da modificação
- USAR `--preserve-timestamps` para preservar mtime/atime original do arquivo
- VALUE é auto-coercido: `true`/`false` para bool, strings numéricas para int/float, o resto permanece string
- Resposta é NDJSON com `type: "result"`, `path`, `key_path`, `checksum`, `action: "set"`
### PROIBIDO
- NUNCA usar `set` em texto puro ou formatos não suportados (apenas TOML e JSON)
- NUNCA usar `set` sem especificar o dotted path completo (sem escopo implícito atual)
### Padrão Correto — Set Valor Top-Level
```bash
atomwrite --workspace . set Cargo.toml package.version 0.2.0
```
### Padrão Correto — Set Valor Aninhado Com Backup
```bash
atomwrite --workspace . set --backup config.toml database.pool.max 20
```
### Padrão Correto — Set Boolean JSON
```bash
atomwrite --workspace . set package.json scripts.test true
```


## Operações Get (v14 Tier 3 — v0.1.12)
### OBRIGATÓRIO
- USAR `get` para ler um único valor de um arquivo de config TOML ou JSON
- ACEITAR `<PATH> <KEY_PATH>` como argumentos posicionais
- USAR notação dotted path para chaves aninhadas
- Resposta é NDJSON com `type: "result"`, `value` (auto-parseado), `key_path`
- Retorna `FILE_NOT_FOUND` (exit 4) se a chave não existe
### Padrão Correto — Get Valor Top-Level
```bash
atomwrite --workspace . get Cargo.toml package.version
# Retorna: {"type":"result","key_path":"package.version","value":"0.1.12",...}
```
### Padrão Correto — Get Valor Aninhado
```bash
atomwrite --workspace . get config.toml database.pool.max
```


## Operações Del (v14 Tier 3 — v0.1.12)
### OBRIGATÓRIO
- USAR `del` para remover uma chave de um arquivo de config TOML ou JSON
- ACEITAR `<PATH> <KEY_PATH>` como argumentos posicionais
- USAR notação dotted path para chaves aninhadas
- USAR `--force-missing` para suceder silenciosamente se a chave já estiver ausente (idempotente)
- USAR `--backup` para criar backup com timestamp antes da deleção
- USAR `--preserve-timestamps` para preservar mtime/atime original
- Resposta é NDJSON com `type: "result"`, `action: "deleted"` ou `"already_missing"`
### Padrão Correto — Deletar Chave
```bash
atomwrite --workspace . del config.toml dependencies.deprecated
```
### Padrão Correto — Deleção Idempotente
```bash
atomwrite --workspace . del --force-missing config.toml features.experimental
# Retorna: {"type":"result","action":"already_missing",...} se a chave já estava ausente
```


## Operações Case (v14 Tier 3 — v0.1.12)
### OBRIGATÓRIO
- USAR `case` para converter case de identificadores em arquivos fonte (refatorar convenção de naming)
- ACEITAR um ou mais `[PATHS]` como argumentos posicionais
- USAR `--to <STYLE>` para definir alvo: `snake` (padrão), `camel`, `pascal`, `kebab`, `screaming-snake`
- USAR `--subvert OLD NEW` (repetível) para renomear identificadores específicos que não devem seguir a regra global
- USAR `--backup` para criar backups com timestamp antes da modificação
- Resposta é NDJSON com `type: "result"`, `files_modified`, `identifiers_renamed`
### PROIBIDO
- NUNCA rodar `case` sem `--dry-run` primeiro em uma base de código grande
- NUNCA usar `case` em arquivos gerados (ex. `target/`, `dist/`)
### Padrão Correto — Snake Case (Padrão)
```bash
atomwrite --workspace . case --to snake --dry-run src/
atomwrite --workspace . case --to snake src/
```
### Padrão Correto — Camel Case Com Exceções
```bash
# Converter snake_case para camelCase, mas manter constantes SCREAMING_SNAKE
atomwrite --workspace . case --to camel --subvert MAX_POOL MAX_POOL src/
```


## Operações Query (v14 Tier 3 — v0.1.12)
### OBRIGATÓRIO
- USAR `query` para inspecionar a estrutura AST de um único arquivo fonte via tree-sitter
- ACEITAR `<PATH>` como argumento posicional
- USAR `--kinds` para listar todos os node kinds nomeados no arquivo (com contagens de ocorrência)
- USAR `--tree` para imprimir a árvore de parse completa
- USAR `--query <PATTERN>` (curto `-Q`) para rodar uma query S-expression tree-sitter
- USAR `--positions` para incluir byte offsets e posições de início para cada match
- USAR `--language <LANG>` para sobrescrever auto-detecção por extensão
- Auto-detecta linguagem pela extensão do arquivo; suporta 24 linguagens via `tree-sitter-language-pack`
- Resposta é NDJSON com `type: "kinds" | "tree" | "matches"` dependendo do modo
### PROIBIDO
- NUNCA usar `--query` (S-expression) em arquivos de linguagens não suportadas (retorna resultado vazio silenciosamente)
- NUNCA passar arquivos grandes (acima de `--max-filesize`) por `query` sem escopo
### Padrão Correto — Listar Node Kinds
```bash
atomwrite --workspace . query --kinds src/main.rs
# Retorna: {"type":"kinds","kinds":[{"name":"function_item","count":42},...]}
```
### Padrão Correto — Imprimir Árvore Completa
```bash
atomwrite --workspace . query --tree src/main.rs
```
### Padrão Correto — Query Com Posições
```bash
atomwrite --workspace . query -Q '(function_item name: (identifier) @name)' --positions src/main.rs
```


## Operações Outline (v14 Tier 3 — v0.1.12)
### OBRIGATÓRIO
- USAR `outline` para extrair estrutura de alto nível (funções, classes, structs, enums) de um arquivo fonte
- ACEITAR `<PATH>` como argumento posicional
- USAR `--kind <KIND>` (repetível) para filtrar por node kind: `function_item`, `struct_item`, `enum_item`, `impl_item`, `class_definition`, `function_definition`, etc.
- USAR `--positions` para incluir byte offsets e posições de início/fim
- USAR `--language <LANG>` para sobrescrever auto-detecção por extensão
- Resposta é NDJSON com `type: "result"`, `items: [{kind, name, range, ...}]`
### PROIBIDO
- NUNCA usar `outline` em arquivos binários (use `read --stat` em vez disso)
- NUNCA encadear `outline` para `replace` sem revisar o output primeiro
### Padrão Correto — Outline Completo
```bash
atomwrite --workspace . outline src/main.rs
# Retorna: {"type":"result","items":[{"kind":"function_item","name":"main","range":[...]},...]}
```
### Padrão Correto — Filtrar por Kind
```bash
atomwrite --workspace . outline --kind function_item --kind struct_item src/lib.rs
```
### Padrão Correto — Outline Com Posições
```bash
atomwrite --workspace . outline --kind function_item --positions src/main.rs | jaq '.items[] | {name, start: .range.start}'
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
### Padrão Correto — v0.1.12 Editor de Config TOML Com Locking Otimista
```bash
CS=$(atomwrite --workspace . read --stat config.toml | jaq -r '.checksum')
atomwrite --workspace . set --backup --preserve-timestamps config.toml database.pool.max 20
# Ou verifique antes de escrever:
atomwrite --workspace . get config.toml database.pool.max  # confirma valor atual
atomwrite --workspace . set config.toml database.pool.max 20
```
### Padrão Correto — v0.1.12 Query AST e Extrair Posições
```bash
# Listar todas as definições de função em um arquivo Rust com suas posições
atomwrite --workspace . query -Q '(function_item name: (identifier) @name)' --positions src/main.rs \\
  | jaq -c '{name: .matches[].captures.name.text, line: .matches[].range.start.line}'
# Contar funções por arquivo
for f in src/*.rs; do
  count=$(atomwrite --workspace . query --kinds "$f" | jaq '.kinds[] | select(.name=="function_item") | .count')
  echo "$f: $count funções"
done
```
### Padrão Correto — v0.1.12 Outline Com Filtro de Kind
```bash
# Obter todos os structs e enums em lib.rs
atomwrite --workspace . outline --kind struct_item --kind enum_item src/lib.rs
# Encontrar a função mais longa em main.rs
atomwrite --workspace . outline --kind function_item --positions src/main.rs \\
  | jaq -c '.items[] | {name, length: (.range.end.byte - .range.start.byte)}' \\
  | sort -t: -k2 -rn | head -1
```
### Padrão Correto — v0.1.12 Recovery WAL Consultivo
```bash
# Detectar journals órfãos antes de retomar trabalho
ls -la .atomwrite.journal.*.json 2>/dev/null | head
# Use a API Rust para controle total:
# let report = atomwrite::wal::recover_orphan_journals(Path::new("src/"))?;
# println!("{}", report.to_json()?);
# Decisão do agente: replay committed, abort in-progress, ou skip stale
```
### Padrão Correto — v0.1.12 Renomeação de Case Com Auditoria
```bash
# Dry-run primeiro, depois aplicar
atomwrite --workspace . case --to kebab --dry-run src/
# Capturar a contagem de arquivos que MUDARIAM
atomwrite --workspace . case --to kebab --dry-run src/ | jaq -s 'map(select(.type=="result") | .files_modified) | add'
# Se aceitável, aplicar
atomwrite --workspace . case --to kebab --backup src/
```
### Padrão Correto — v0.1.12 Verificação de Sintaxe Pre-Commit
```bash
# Verificar sintaxe de arquivo Rust antes do commit
atomwrite --workspace . write --syntax-check src/lib.rs < new_lib.rs
# Exit 88 (SyntaxError) se tree-sitter detectar sintaxe inválida
# Use em hooks pre-commit ou CI linting
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


## Suporte ao Windows 10/11 (v0.1.12)
### OBRIGATÓRIO
- VERIFICAR que Visual Studio 2019+ Build Tools com workload C++ está instalado antes de `cargo install atomwrite`
- VERIFICAR que Rust 1.88 ou posterior está instalado
- USAR Windows Terminal ou PowerShell 7+ para output UTF-8 e sequências ANSI adequadas
- CONFIAR que `init_console` define code page 65001 e `ENABLE_VIRTUAL_TERMINAL_PROCESSING` automaticamente
- ESTAR CIENTE de que `tree-sitter-language-pack` 1.8 com feature `download` requer acesso de rede no primeiro build — o script postinstall baixa parsers do GitHub
- ESPERAR que o primeiro `cargo install atomwrite` no Windows pode levar 5-10 minutos devido aos downloads de parsers
- CONFIAR que os 5 novos códigos de erro (83, 88, 91, 92, 93) funcionam no Windows — são testados nos gates de cross-compile
### PROIBIDO
- NUNCA usar console legado `cmd.exe` para output (mojibake esperado)
- NUNCA depender de `cargo install atomwrite` funcionando na v0.1.3 (quebrado no Windows 10/11; fix está na v0.1.4)
- NUNCA usar `query` no Windows sem antes garantir que os parsers foram baixados (use `--language` para sobrescrever se auto-detect falhar)
### Padrão Correto — Instalação Windows (v0.1.12)
```powershell
rustup default stable
rustup target add x86_64-pc-windows-msvc
cargo install atomwrite --locked --version '^0.1.12'
atomwrite --version  # Saída NDJSON
# Primeira execução pode levar alguns segundos para inicializar parsers tree-sitter
```


## Validação Cross-Compile (v0.1.12)
### OBRIGATÓRIO
- EXECUTAR `cargo test --test cross_compile_check -- --ignored` antes de qualquer release que toque código `#[cfg(windows)]`
- INSTALAR targets Windows: `rustup target add x86_64-pc-windows-gnu` e `i686-pc-windows-gnu`
- NO Linux, INSTALAR mingw-w64: `mingw64-gcc` (Fedora) ou `mingw-w64` (Ubuntu) e `mingw32-gcc` para 32-bit
- CONFIAR que o gate falha em qualquer regressão de `E0433`, `E0308`, ou `E0507` em código Windows-only
- VERIFICAR que os 10 novos arquivos de teste da v0.1.12 compilam em todos os 3 targets de cross-compile — `cli_set`, `cli_case`, `cli_query`, `cli_outline`, `cli_get_del`, `cli_v012_syntax_check`, `cli_v012_wal`, `cli_v012_audit_regressions`, `cli_v012_xattr_reflink`, `cli_v012_batch4_regressions`
- ESTAR CIENTE de que `tree-sitter-language-pack` é baixado em build time, então cross-compile offline requer pré-baixar os parsers
### Padrão Correto — Gate de Cross-Compile (v0.1.12)
```bash
rustup target add x86_64-pc-windows-gnu i686-pc-windows-gnu x86_64-pc-windows-msvc
cargo test --test cross_compile_check -- --ignored
# Verificar que os 10 arquivos de teste da v0.1.12 compilam em todos os 3 targets Windows
cargo check --target x86_64-pc-windows-gnu --tests
cargo check --target i686-pc-windows-gnu --tests
cargo check --target x86_64-pc-windows-msvc --tests
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
- `83` — timeout de lock (v0.1.12+)
- `85` — FIFO detectado (named pipe não pode ser escrito atomicamente)
- `86` — arquivo de dispositivo detectado (bloco ou caractere)
- `88` — erro de sintaxe detectado (v0.1.12+, verificação G72 tree-sitter falhou)
- `91` — fallback EXDEV desabilitado (v0.1.12+, --strict-atomic proíbe copy-fallback)
- `92` — verificação BLAKE3 do copy-back falhou (v0.1.12+)
- `93` — journal órfão detectado (v0.1.12+, recuperação consultiva G114)
- `126` — violação do jail do workspace (caminho escapa à raiz do workspace)
- `127` — symlink bloqueado (alvo do symlink fora do workspace)
- `128` — imutável (arquivo marcado como imutável)
- `130` — SIGINT (interrompido pelo usuário)
- `141` — SIGPIPE (pipe quebrado)
- `143` — SIGTERM (terminado por sinal)
- `255` — erro interno (falha inesperada)


### Notas de Drift v0.1.19 — Consolidação de Exit Code da Fase D
- DRIFT 1 — `STATE_DRIFT` (82) absorve `CHECKSUM_VERIFY_FAILED` (81) para `--verify-checksum` em reads e writes. Ambos são classe conflict, retentáveis. O code 81 é agora histórico, preservado apenas para o mismatch BLAKE3 do caminho `read` no conteúdo do arquivo. O code 82 cobre a falha de locking otimista incluindo o mismatch de `--expect-checksum` em writes e edits, e o mismatch de `--verify-checksum` em reads.
- DRIFT 2 — `--syntax-check` retorna `SYNTAX_ERROR_DETECTED`, NÃO `SYNTAX_ERROR`. O rename aconteceu no rollout do G72 tree-sitter da v0.1.12 mas a documentação não foi atualizada. O nome histórico `SYNTAX_ERROR` é preservado apenas em prosa para grep-ability.
- DRIFT 3 — `ORPHAN_JOURNAL` (93) é consultivo, NÃO autodetectado. O portão é `ATOMWRITE_WAL=1` OU `--strict-atomic`. O `write` padrão (v0.1.16 G119 `WalPolicy::Auto`) não escreve sidecar e portanto não pode detectar órfãos. Invocações padrão nunca veem este code.
- DRIFT 4 — `BROKEN_PIPE` (141) exige propagação real de SIGPIPE. Um pipe simples `head -1` NÃO o dispara. A restauração de SIGPIPE da v0.1.4+ recoloca a disposição default, então o sinal só é levantado quando o consumidor downstream fecha ativamente o pipe no meio do stream.
- DRIFT 5 — Leituras de arquivo binário retornam exit 0 com metadados `kind=binary`, NÃO exit 65. A heurística `BINARY_FILE` da v0.1.4 foi alterada para emitir envelope estruturado e exit 0. O caminho do code 65 agora só dispara para `read` sem `--format raw` E com a heurística binária bypassada.
- DRIFT 6 — Argumento posicional ausente retorna `ARGUMENT_PARSE_ERROR` (exit 2), NÃO `INVALID_INPUT` (65). Erros de argumento no nível clap são reportados como exit 2. O code 65 é reservado para validação de conteúdo em runtime (TOML malformado, regex inválida, stdin vazio padrão).
- DRIFT 7 — Falta de `--workspace` cai para CWD, NÃO é erro. `--workspace` é uma flag com default CWD, não um argumento obrigatório. `WORKSPACE_JAIL` (126) só dispara quando um caminho absoluto resolve fora do jail efetivo.
- Veja `docs/decisions/0033-v0-1-19-exit-code-naming-drift-consolidation.md` para a justificativa completa e as consequências de aceitar o comportamento do binário como canônico.
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
### OBRIGATÓRIO — Lista Completa de Códigos de Erro (25 codes a partir da v0.1.12)
- `WORKSPACE_JAIL` (exit 126, precondition_failed, não retentável)
- `SYMLINK_BLOCKED` (exit 127, precondition_failed, não retentável)
- `FILE_NOT_FOUND` (exit 4, permanent, não retentável)
- `PERMISSION_DENIED` (exit 13, transient, retentável via `persist_with_retry` no Windows)
- `CHECKSUM_VERIFY_FAILED` (exit 81, conflict, não retentável)
- `STATE_DRIFT` (exit 82, conflict, não retentável)
- `LOCK_TIMEOUT` (exit 83, transient, retentável com backoff — v0.1.12+, contenção de arquivo de lock G54)
- `FIFO_DETECTED` (exit 85, precondition_failed, não retentável)
- `DEVICE_FILE` (exit 86, precondition_failed, não retentável)
- `SYNTAX_ERROR` (exit 88, permanent, não retentável — v0.1.12+, validação tree-sitter G72 falhou)
- `EXDEV_FALLBACK_DISABLED` (exit 91, precondition_failed, não retentável — v0.1.12+, modo atômico estrito G90 proíbe fallback de cópia cross-device)
- `COPY_BACK_BLAKE3_FAILED` (exit 92, conflict, retentável após reler — v0.1.12+, verificação de checksum de copy-back cross-device G114 falhou)
- `ORPHAN_JOURNAL` (exit 93, precondition_failed, não retentável — v0.1.12+, sidecar WAL órfão G114 detectado; chame `recover_orphan_journals` consultivamente)
- `DISK_FULL` (exit 28, transient, retentável)
- `QUOTA_EXCEEDED` (exit 30, transient, retentável)
- `CROSS_DEVICE` (exit 73, permanent, não retentável)
- `IO_ERROR` (exit 74, transient, retentável)
- `CONFIG_INVALID` (exit 78, permanent, não retentável)
- `FILE_IMMUTABLE` (exit 128, precondition_failed, não retentável)
- `BINARY_FILE` (exit 65, permanent, não retentável — use `read --format raw` para ignorar envelope JSON)
- `FILE_TOO_LARGE` (exit 65, permanent, não retentável — arquivo excede limite `--max-filesize`)
- `NO_MATCHES` (exit 1, permanent, não retentável — por design, não é um erro)
- `INVALID_INPUT` (exit 65, permanent, não retentável)
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


## Arquivo de Configuração (v0.1.25)
### OBRIGATÓRIO
- SABER que v0.1.25 introduz `.atomwrite.toml` como arquivo de configuração (GAP-072)
- Hierarquia de precedência: flags CLI > variáveis de ambiente > `.atomwrite.toml` local > `$XDG_CONFIG_HOME/atomwrite/config.toml` > defaults
- USAR `.atomwrite.toml` na raiz do workspace para definir defaults por projeto
- VERIFICAR configuração ativa com `atomwrite --workspace . get .atomwrite.toml <key_path>`
### Padrão Correto — Config por Projeto
```bash
echo '[defaults]
backup = true
retention = 3
line_ending = "lf"' | atomwrite --workspace . write .atomwrite.toml
```


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


## Schemas Versionados (v0.1.12)
### OBRIGATÓRIO
- SABER que schemas JSON estáveis estão commitados em `docs/schemas/`
- SABER que `error-output.schema.json` é o contrato para todos os envelopes de erro
- SABER que o campo `workspace` (string, opcional) foi adicionado em v0.1.4
- USAR o schema versionado para validar respostas no pipeline do agente
- NÃO inventar suas próprias regras de parsing; confiar no schema versionado como fonte de verdade
### Obrigatório — Índice de Schemas (29 schemas a partir da v0.1.12)
- `error-output.schema.json` — envelope para todas as respostas `error: true` (v0.1.4)
- `write-output.schema.json` — resposta do comando `write`
- `read-output.schema.json` — resposta do comando `read` com metadados
- `search-output.schema.json` — matches NDJSON do comando `search`
- `replace-output.schema.json` — resposta batch do comando `replace`
- `edit-output.schema.json` — resposta do comando `edit` com `mtime_preserved`
- `transform-output.schema.json` — resposta de refator AST do `transform`
- `scope-output.schema.json` — resposta de scoping gramatical do `scope`
- `batch-output.schema.json` — resultado transacional do `batch`
- `hash-output.schema.json` — resposta de checksum BLAKE3 do `hash`
- `delete-output.schema.json` — confirmação de remoção do `delete`
- `diff-output.schema.json` — hunks de diff estruturado do `diff`
- `move-output.schema.json` — confirmação de renomeação do `move`
- `copy-output.schema.json` — resposta de verificação do `copy`
- `list-output.schema.json` — listagem de diretório do `list`
- `count-output.schema.json` — contagem de arquivos e linhas do `count`
- `extract-output.schema.json` — extração de campos do `extract`
- `calc-output.schema.json` — cálculo matemático e conversão de unidades do `calc`
- `regex-output.schema.json` — padrão gerado pelo `regex`
- `backup-output.schema.json` — backup com timestamp do `backup`
- `rollback-output.schema.json` — restauração do `rollback`
- `apply-output.schema.json` — aplicação de patch do `apply`
- `set-result.schema.json` — v14 Tier 3 do `set` (v0.1.12, NOVO)
- `get-result.schema.json` — v14 Tier 3 do `get` (v0.1.12, NOVO)
- `del-result.schema.json` — v14 Tier 3 do `del` (v0.1.12, NOVO)
- `case-result.schema.json` — v14 Tier 3 do `case` (v0.1.12, NOVO)
- `query-output.schema.json` — v14 Tier 3 do `query` (oneOf 3: kinds/tree/matches, v0.1.12, NOVO)
- `outline-output.schema.json` — v14 Tier 3 do `outline` (oneOf 2: items/empty, v0.1.12, NOVO)
- `wal-recovery.schema.json` — relatório de recovery WAL (v0.1.12, NOVO)
### Obrigatório — Exemplo de Validação Programática
```bash
# Validar resposta NDJSON contra seu schema usando ajv-cli
echo '{"type":"result","checksum":"abc...","bytes_written":42}' | \\
  ajv validate -s docs/schemas/write-output.schema.json -d /dev/stdin
# Ou com Python jsonschema:
python3 -c "import json, jsonschema; \\
  s = json.load(open('docs/schemas/write-output.schema.json')); \\
  d = json.loads('{\"type\":\"result\",\"checksum\":\"abc\",\"bytes_written\":42}'); \\
  jsonschema.validate(d, s); print('OK')"
```


## Testes e Gates de Qualidade (v0.1.12)
### OBRIGATÓRIO — Postura de Qualidade
- 631 testes em 63+ suítes de teste passam com zero regressões a partir da v0.1.25
- Decomposição da contagem de testes: 320 baseline (v0.1.10) + +29 (v0.1.11) + +96 (v0.1.12) + +2 (v0.1.14) + +14 (v0.1.15: 8 G117 + 6 G118) = 461 (v0.1.15) + +41 (v0.1.18: G118 + G119 + G120 + 2 ADRs) = 502 total
- Decomposição v0.1.21 a v0.1.25: +13 (v0.1.21: drift + backup parity) + +16 (v0.1.22: edit-loop + prune-backups) + +31 (v0.1.23: 12 hyphen + 7 backup + 4 shrink + 8 old-file) + +12 (v0.1.24: typed-errors + bug-fixes + path-resolution) + +10 (v0.1.25: config + verify + fuzzy-proptest + scope-actions + bug-fixes) = 631 total
- Novos arquivos de teste v0.1.23 (4): `cli_v0123_hyphen_values`, `cli_v0123_backup_default`, `cli_v0123_shrink_guard`, `cli_v0123_old_file`
- Novos arquivos de teste v0.1.12 (10): `cli_set`, `cli_case`, `cli_query`, `cli_outline`, `cli_get_del`, `cli_v012_syntax_check`, `cli_v012_wal`, `cli_v012_audit_regressions` (27 testes), `cli_v012_xattr_reflink`, `cli_v012_batch4_regressions` (23 testes)
- Cobertura de teste v0.1.12 por categoria: G72 syntax check (16 testes), G114 WAL (8 testes), v14 query/outline (10 testes), TOML dotted path (6 testes), set/get/del/case (15 testes), regressões de auditoria (50 testes)
- 8 gates oficiais passam em cada commit: `fmt`, `clippy`, `build`, `test`, `doc`, `deny`, `audit`, `msrv`
- 3 targets de cross-compile passam: `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, `x86_64-pc-windows-msvc`
- Cargo deny e cargo audit reportam zero vulnerabilidades (time 0.3.47+ resolveu RUSTSEC-2026-0009 via DEPTH_LIMIT=32)
- MSRV é Rust 1.88 stable
- Cobertura via `cargo tarpaulin`: 20.19% cobertura de linha (935/4631 linhas) — cobertura é pesada em testes de integração
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
## Migração v0.1.12 Referência Rápida
### OBRIGATÓRIO — Saiba O Que Mudou Desde v0.1.11
- **6 novos subcomandos ADITIVOS**: `set`, `get`, `del`, `case`, `query`, `outline` (editores estruturados v14 Tier 3 + ferramentas AST tree-sitter). Nenhum subcomando existente foi renomeado ou removido
- **5 novas variantes de erro ADITIVAS**: `LockTimeout` (83), `SyntaxError` (88), `ExdevFallbackDisabled` (91), `CopyBackBlake3Failed` (92), `OrphanJournal` (93). Todas bilíngues com sugestões acionáveis
- **`atomwrite write --syntax-check` é OPT-IN**: comportamento padrão de `write` não mudou. Verificação de sintaxe G72 REAL via tree-sitter (24 linguagens)
- **Sidecar WAL é apenas consultivo**: `atomic_write` escreve `.atomwrite.journal.<target>.atomwrite.journal.json` apenas quando `ATOMWRITE_WAL=1` está definido OU `--strict-atomic` é passado. `write` padrão NÃO escreve o sidecar. `recover_orphan_journals(dir)` é consultivo
- **542 testes passam em 47 suítes** (eram 320 em v0.1.10). Cobertura completa entre todos os 30 subcomandos
- **7 ADRs adicionados** em `docs/decisions/` (0019-0025) e 7 novos JSON Schemas em `docs/schemas/`
- **Nova dependência**: `tree-sitter-language-pack = "1.8"` com features `download` + `dynamic-loading`. Footprint da instalação fica em torno de 5-10 MB


## Subcomandos WAL (v0.1.18)
### OBRIGATÓRIO — wal-stats
- USAR `wal-stats` para inspecionar o estado do journal WAL para telemetria e debug
- SABER que `wal-stats` é consultivo: escaneia o workspace e reporta snapshot dos arquivos de journal sem modificar
- SEMPRE combinar `wal-stats` com `--workspace <DIR>` para delimitar o escopo do scan
- USAR `--dry-run` para pré-visualizar o que o scan encontraria sem fazer a varredura
- Resposta é NDJSON com `type: "result"`, contagens de estado terminal, tamanho total, distribuição de idade e breakdown por diretório
- JSON response: `{action: "scanned", terminal_committed, terminal_aborted, terminal_started, total_bytes, oldest_age_secs, breakdown_by_dir}`
- USAR para debug de ops quando há suspeita de journals órfãos ou crescimento inesperado de sidecars

### OBRIGATÓRIO — wal-heal
- USAR `wal-heal` para remover journals terminais órfãos mais antigos que um threshold
- Threshold padrão é 3600 segundos (1 hora) via `--threshold-secs <N>`
- Budget padrão de wall-clock é 100ms via `--max-duration-ms <N>`
- USAR `--threshold-secs` e `--max-duration-ms` para ajustar ao seu ambiente
- USAR quando o workspace acumula journals terminais de processos interrompidos ou crashados
- Equivalente em auto-pass roda no startup com threshold de 3600s e budget de 100ms; pular via flag global `--no-auto-heal` ou env `ATOMWRITE_WAL_NO_AUTO_HEAL=1`

### Padrão Correto — Inspecionar Estado WAL
```bash
# Snapshot do estado WAL atual do projeto
atomwrite --workspace . wal-stats
# Output: {"type":"result","action":"scanned","terminal_committed":42,...}
```

### Padrão Correto — Curar Journals Órfãos
```bash
# Remove journals terminais mais antigos que 1 hora
atomwrite --workspace . wal-heal --threshold-secs 3600
# Threshold e budget customizados
atomwrite --workspace . wal-heal --threshold-secs 7200 --max-duration-ms 500
```


## v0.1.21 — Padrão de Edits Sequenciais com Re-captura de Checksum
### OBRIGATÓRIO — Padrão do Gap-2026-012
- SABER que encadear múltiplas chamadas `edit` no mesmo arquivo sem re-capturar `checksum_after` produz `STATE_DRIFT` (exit 82) em toda chamada após a primeira
- DOIS padrões válidos para pipelines sequenciais — escolha um por pipeline
- PADRÃO A — re-capturar `checksum_after` após cada `edit` e passar para a próxima chamada. Reduz risco de drift a zero mas dobra as invocações CLI (um `read` por `edit`)
- PADRÃO B — passar `--allow-sequential-drift` para cada chamada `edit`. Mesmo número de invocações CLI do método naive; a flag suprime `STATE_DRIFT` e emite `tracing::warn!` nomeando o drift
- NUNCA usar `--allow-sequential-drift` em cenário verdadeiramente paralelo. A flag existe para o caso sequencial de agente único; agentes concorrentes devem usar o Padrão A com reads frescos

### Padrão A — Re-captura de Checksum Após Cada Edit
```bash
# Seed inicial do alvo
echo "line 1" > /tmp/seq.txt
# Ler o checksum inicial
CS=$(atomwrite --workspace /tmp read seq.txt | jaq -r '.checksum')
# Edit 1 — passa o checksum capturado
echo "line 2" | atomwrite --workspace /tmp edit --expect-checksum "$CS" seq.txt --append
# Re-capturar o checksum pós-edit
CS=$(atomwrite --workspace /tmp read seq.txt | jaq -r '.checksum')
# Edit 2 — usa o novo checksum
printf 'line 1\nline 2\n' | atomwrite --workspace /tmp edit --expect-checksum "$CS" seq.txt --append
```

### Padrão B — Permitir Drift Sequencial
```bash
# Seed inicial do alvo
echo "line 1" > /tmp/seq.txt
# Ler o checksum inicial
CS=$(atomwrite --workspace /tmp read seq.txt | jaq -r '.checksum')
# Edit 1 — passa o checksum capturado (drift dispararia aqui sem a flag no edit 2+)
echo "line 2" | atomwrite --workspace /tmp edit --expect-checksum "$CS" seq.txt --append
# Edit 2 — drift é permitido, o pré-estado difere de CS mas a flag suprime STATE_DRIFT
printf 'line 1\nline 2\n' | atomwrite --workspace /tmp edit --allow-sequential-drift seq.txt --append
```

### PROIBIDO
- NUNCA usar `--allow-sequential-drift` para bypassar drift causado por escritor CONCORRENTE — isso é race condition real, não pipeline sequencial, e o warning existe para expor isso
- NUNCA passar `--allow-sequential-drift` para um pipeline que roda múltiplas invocações de `edit` em paralelo contra o mesmo arquivo

## v0.1.21 — Deleção de Backup Após Sucesso
### OBRIGATÓRIO — Comportamento do Gap-2026-014 v2
- SABER que `write --backup`, `replace --backup` e `edit --backup` DELETAM o arquivo de backup por default após a escrita ser bem-sucedida
- USAR `--keep-backup` em `write`, `edit`, `replace`, `rollback`, `apply` ou `batch` para preservar o backup após sucesso
- SABER que backups são SEMPRE preservados quando a escrita FALHA. A flag `--keep-backup` afeta apenas o caminho de sucesso
- USAR `--keep-backup` em scripts de CI que precisam do backup como evidência forense após a escrita completar
- NUNCA assumir que backups persistem após uma operação `--backup` bem-sucedida. O comportamento pré-v0.1.21 de backup-vive-para-sempre foi removido

### Padrão — Preservar Backup para Auditoria
```bash
# Backup sobrevive após sucesso quando --keep-backup é passado
echo "new" | atomwrite --workspace /tmp write --backup --keep-backup config.toml
# Comportamento default: backup é deletado após sucesso
echo "new" | atomwrite --workspace /tmp write --backup config.toml
# Verifica o estado pós-sucesso
test -f /tmp/config.toml.bak.* && echo "backup presente" || echo "backup deletado"
```

## Notas da v0.1.20
### OBRIGATÓRIO — Rename Global de --lang para --locale
- Flag GLOBAL `--lang` foi RENOMEADA para `--locale` (mudança quebrante na v0.1.20)
- Variável de ambiente `ATOMWRITE_LANG` permanece INALTERADA
- Nome do campo Rust `lang` no Cli struct permanece INALTERADO
- Veja ADR-0037 para a justificativa completa do rename e notas de migração
- ATUALIZE invocações existentes de `--lang pt-BR` para `--locale pt-BR`
- NÃO confundir com `transform --lang` ou `scope --lang` (flags de linguagem de subcomando permanecem)

### OBRIGATÓRIO — Flags de Guarda de Intenção de Write (v0.1.20)
- SABER que a v0.1.20 introduz quatro novas flags de segurança de escrita
- USAR `--require-backup` para ABORTAR se `--backup` não estiver setado E o alvo existir
- USAR `--confirm` para disparar prompt interativo S/N para arquivos maiores que 100KB
- USAR `--auto-rotate` para FORÇAR backup quando o alvo foi modificado nas últimas 24 horas
- USAR `--risk-threshold <PERCENT>` para emitir telemetria `risk_assessment` quando o delta de tamanho exceder o threshold (padrão: 50)
- Comportamento padrão do `write` permanece INALTERADO — estas flags são aditivas
- USAR `--require-backup` em pipelines de CI para prevenir sobrescritas destrutivas sem backup

### Padrão Correto — Exigir Backup Antes de Sobrescrita
```bash
# v0.1.20+: aborta se --backup não estiver setado e alvo existir
atomwrite --workspace . write --require-backup src/main.rs < new_main.rs
# Exit 1 (Validation) se --backup também não foi setado
```

### Padrão Correto — Confirmar Escrita em Arquivos Grandes
```bash
# v0.1.20+: prompt interativo para arquivos > 100KB
atomwrite --workspace . write --confirm big_dataset.csv < new_data.csv
# Pergunta s/N antes de aplicar
```

### Padrão Correto — Auto-Rotacionar Alvos Recentes
```bash
# v0.1.20+: força backup quando alvo modificado nas últimas 24h
atomwrite --workspace . write --auto-rotate src/frequently_changed.rs < new.rs
# Backup auto-criado se mtime dentro de 24h
```

### Padrão Correto — Telemetria de Threshold de Risco
```bash
# v0.1.20+: emite risk_assessment quando delta de tamanho > 50%
atomwrite --workspace . write --risk-threshold 30 src/data.json < new.json
# Resposta NDJSON inclui bloco risk_assessment
```

### OBRIGATÓRIO — Modo Count --by-size (v0.1.20)
- SABER que `--by-size` produz saída NDJSON estruturada (v0.1.20+)
- Resposta inclui `mode: "by_size"`, array `items[path, bytes]`, ordenado DECRESCENTE por tamanho
- USAR `--top N` para truncar a lista de items (padrão: 10)
- Substitui a antiga saída em tabela de texto por um contrato parseável
- CONSUMIR via `jaq '.items[] | {path, bytes}'` para pipelines downstream

### Padrão Correto — Maiores Arquivos
```bash
# v0.1.20+: top 10 maiores arquivos com saída estruturada
atomwrite --workspace . count --by-size --top 10
# Output: {"type":"result","mode":"by_size","items":[{"path":"...","bytes":N},...]}
```

### OBRIGATÓRIO — Discriminador Read --mode (v0.1.20)
- SABER que o campo `mode` na saída do `read` agora é POPULADO
- Valor de `mode` é um de: `full`, `head`, `tail`, `line`, `lines`, `grep`, `stat`
- USAR isto para desambiguar qual variante de read produziu a resposta
- Anteriormente o campo estava sempre ausente ou null

### Padrão Correto — Inspecionar Modo do Read
```bash
# v0.1.20+: read reporta qual modo foi usado
atomwrite --workspace . read --head 20 src/main.rs | jaq '.mode'
# Output: "head"

atomwrite --workspace . read --grep 'TODO' src/main.rs | jaq '.mode'
# Output: "grep"

atomwrite --workspace . read --stat src/main.rs | jaq '.mode'
# Output: "stat"
```

### OBRIGATÓRIO — Search --no-begin-end (v0.1.20)
- USAR `--no-begin-end` para suprimir eventos `begin`/`end` por arquivo quando arquivos não têm matches
- Útil para pipelines streaming que só se importam com conteúdo de match
- Comportamento padrão INALTERADO — eventos `begin`/`end` continuam emitidos a menos que suprimidos
- Combinar com `--count` para contagens compactas por arquivo

### Padrão Correto — Suprimir Marcadores de Arquivo Vazio
```bash
# v0.1.20+: silencia eventos begin/end para arquivos sem matches
atomwrite --workspace . search --no-begin-end 'TODO' src/ --include '*.rs'
# Output: apenas arquivos com matches emitem begin/end; zero-match ficam silentes
```

### OBRIGATÓRIO — Write --preserve-timestamps (v0.1.20)
- USAR `--preserve-timestamps` no `write` para manter o mtime original do alvo
- Comportamento padrão INALTERADO — mtime é atualizado para refletir a escrita por padrão
- Útil para workflows de backup, snapshot e builds reproduzíveis
- Espelho da flag `--preserve-timestamps` existente em `edit` e `replace`

### Padrão Correto — Preservar mtime no Write
```bash
# v0.1.20+: mantém mtime original no write
atomwrite --workspace . write --preserve-timestamps src/snapshot.rs < new.rs
# mtime do alvo inalterado após a escrita atômica
```

### OBRIGATÓRIO — Alias Scope --lang (v0.1.20)
- Após o rename global de `--lang` para `--locale`, `scope --lang <LANG>` agora é um alias funcional de `--language`
- USAR `scope --lang rust` como atalho para `scope --language rust`
- Ambas as formas são aceitas — `--lang` no `scope` é o seletor de linguagem local do subcomando
- Isso evita colisão com a flag global de locale que foi renomeada para `--locale`


## GAP-2026 v0.1.20 — Cobertura Completa
### OBRIGATÓRIO — Os 11 Gaps Fechados em v0.1.20
- GAP-2026-001: `count --by-size` finalmente implementa a flag da help (teste de regressão `count_by_size_top_n_returns_sorted`)
- GAP-2026-002: `write --preserve-timestamps` (paridade com edit/replace)
- GAP-2026-003: alias `scope --lang` após rename global `--lang` → `--locale` (ADR-0037)
- GAP-2026-004: `write --line-ending crlf` aceita ambas as formas `crlf` e `cr-lf` (4 variantes com `value` + `alias`)
- GAP-2026-005b: semântica de `edit --partial` (single-pair retorna NO_MATCHES exit 1; multi-pair aplica matched e relata unmatched) — ADR-0036
- GAP-2026-006: testes de regressão de `diff --algorithm` para myers/patience/lcs
- GAP-2026-007: `count --by-extension` filtra sufixos timestamp de backup via `BACKUP_RE` `\.bak\.\d{8}_\d{6}$`
- GAP-2026-008: `read --head/--line/--lines` reporta contagem de linhas FILTRADA (novo campo `lines_total` preserva o original)
- GAP-2026-009: `read` emite discriminador `mode` (`full|head|tail|line|lines|grep|stat`)
- GAP-2026-010: `search --no-begin-end` para output mais limpo de walks com zero matches
- GAP-2026-011: guardas de intenção de `write` (defesa em profundidade após incidente c24-framework34.html de 2026-06-15) — 6 camadas L1-L6 (telemetria, --require-backup, --confirm, --preview, --auto-rotate, risk_assessment no envelope) — ADR-0035

### OBRIGATÓRIO — Origem das Guardas de Intenção de Write (c24-framework34.html)
- Em 2026-06-15, `atomwrite write` sem `--append` truncou c24-framework34.html (491.827 bytes) para poucos bytes
- Aproximadamente 127 linhas (~9 KB) de trabalho de 2026-06-15 foram perdidas
- Um backup manual `cp` de 2026-06-14 23:49 existia mas não cobria o trabalho de 2026-06-15
- Sem `--backup`, sem prompt de confirmação, sem telemetria de delta de tamanho, o caminho entre a intenção do operador e o conteúdo no disco era um syscall direto
- v0.1.20 adiciona 6 camadas de defesa em profundidade (L1-L6) para prevenir recorrência — veja ADR-0035
- L1 telemetria (size_delta_pct) é INFORMATIVA (default off, opt-in via `--risk-threshold`)
- L2 `--require-backup` ABORTA com `InvalidInput` (exit 65) se `--backup` não foi setado
- L3 `--confirm` pergunta `Overwrite <path> (<bytes> bytes)? [y/N]` para alvos > 100 KB
- L4 `--preview` emite diff estrutural antes do atomic write
- L5 `--auto-rotate` força backup quando alvo foi modificado em 24h
- L6 campo `risk_assessment` no envelope (somente quando uma guarda dispara)

### OBRIGATÓRIO — v0.1.19 (release predecessora) — 3 ADRs Adicionados
- ADR-0031 — G121 path resolution helper: `search` e `replace` resolvem root paths contra o workspace via helper compartilhado (CWE-367)
- ADR-0032 — query S-expr real implementation: `query` aceita S-expressions tree-sitter via `Query::new` (docs v0.1.12 prometiam mas código nunca implementou)
- ADR-0033 — v0.1.19 exit code drift consolidation: 7 drifts de exit code entre docs publicadas e binário (STATE_DRIFT, SYNTAX_ERROR_DETECTED, ORPHAN_JOURNAL, BROKEN_PIPE, binary read, ARGUMENT_PARSE_ERROR, missing --workspace)

### OBRIGATÓRIO — Anti-Pattern Help-Driven Testing (ADR-0034)
- 5 dos 11 GAP-2026 (001, 003, 004, 005b, 006) tinham clap `--help` declarando flag antes da implementação existir
- Regra v0.1.21+: `cargo test --doc` deve parsear cada bloco de help e validar que cada flag está wired a um teste de regressão cujo nome inclui a flag

## Fluxo de Recovery WAL (v0.1.12)
### OBRIGATÓRIO
- SABER que `atomic_write` só escreve um sidecar WAL quando a env var `ATOMWRITE_WAL=1` está definida OU a flag CLI `--strict-atomic` é passada
- SABER que o caminho do sidecar é `.atomwrite.journal.<target_basename>.atomwrite.journal.json`
- SABER que `recover_orphan_journals(dir)` é CONSULTIVO — NÃO faz replay nem delete automático
- SABER que cada sidecar contém `JournalEntry::{Started, Committed, Aborted}` com `op_id` e `pid`
### Obrigatório — Árvore de Decisão de Recovery
1. **Detectar órfãos**: escanear diretório por arquivos `*.atomwrite.journal.json`
2. **Ler entradas**: parsear cada sidecar para determinar quais operações foram `Started` mas não `Committed`/`Aborted`
3. **Decidir por entrada**:
   - `Committed` → seguro deletar sidecar (operação completou com sucesso)
   - `Aborted` → seguro deletar sidecar (operação foi revertida)
   - `Started` sem `Committed`/`Aborted` → AMBÍGUO: consulte o usuário ou verifique o inode do arquivo alvo
4. **Ação atômica**: aplicar decisão via API Rust `recover_orphan_journals`
### Obrigatório — Padrão da API Rust
```rust
use atomwrite::wal::{recover_orphan_journals, OrphanJournalReport};
use std::path::Path;

let report: OrphanJournalReport = recover_orphan_journals(Path::new("src/"))?;
// Inspecionar report.entries: Vec<JournalEntry>
// Aplicar sua lógica de decisão por entrada
// Usar atomwrite delete com --force para limpar sidecars reconciliados
```
### PROIBIDO
- NUNCA deletar sidecars automaticamente sem confirmação do usuário
- NUNCA fazer replay de entradas WAL sem verificar o estado atual do arquivo alvo
- NUNCA tratar WAL como única fonte de verdade para atomicidade (a syscall rename é a primitiva atômica real; WAL é apenas para forense de crash)


## Gaps Fechados na v0.1.12
### OBRIGATÓRIO — Saber Quais Eram os 20 Gaps
- A release v0.1.12 fecha 20 gaps técnicos nomeados de `gaps.md`. Cada gap tem um ADR em `docs/decisions/0019-0025` e um teste em `tests/`
- **G72 — Verificação de sintaxe REAL via tree-sitter**: `atomwrite write --syntax-check` valida conteúdo contra 24 linguagens via `tree_sitter_language_pack`. Substitui verificação heurística de balanceamento de colchetes. Retorna `SyntaxError` (88) em falha
- **G90 — Fallback de cópia EXDEV controlado**: modo `--strict-atomic` proíbe fallback de cópia em moves cross-device. Retorna `ExdevFallbackDisabled` (91) quando acionado
- **G114 — Sidecar WAL consultivo**: `ATOMWRITE_WAL=1` ou `--strict-atomic` escreve `.atomwrite.journal.<target>.json`. `recover_orphan_journals` é a API de recovery consultivo
- **G114 — Verificação BLAKE3 do copy-back**: copy-back cross-device verifica o checksum do destino antes de deletar a origem. Retorna `CopyBackBlake3Failed` (92) em mismatch
- **G54 — Arquivo de lock com timeout**: cada write adquire lock de arquivo com 30s de timeout. Retorna `LockTimeout` (83) em contenção
- **G44 — Transform multirule**: `transform --rules <PATH>` e `--inline-rules <JSON>` aceitam múltiplas regras
- **G66 — Search/replace literal**: `--literal` (`-F`) trata pattern como string literal, sem escape de regex
- **G64 — Detecção de reflink**: `--no-reflink` em `copy`/`move` desabilita otimização de reflink (copy-on-write)
- **G68 — max-filesize e max-columns**: `--max-filesize <BYTES>` cap global; `--max-columns <N>` limita largura de saída do `search`
- **G56 — Inclusão de FIFO**: `--include-fifo` em `search` atravessa FIFO/named pipes
- **G39 — Preservação de xattr**: `--preserve-xattr` em `copy`/`move` mantém extended attributes
- **G41 — Tratamento binário**: `read --format raw` emite bytes crus sem envelope JSON, evita `BinaryFile` (65) para conteúdo conhecido como binário
- **G58 — Normalização de line ending**: `--line-ending lf|crlf|cr|auto` em `write` e `edit`
- **G76 — Escolha de algoritmo de diff**: `diff --algorithm myers|patience|lcs` seleciona algoritmo
- **G74 — Threads paralelas**: `--threads <N>` / `-j <N>` flag global controla pool Rayon
- **G80 — Restauração de SIGPIPE**: SIGPIPE é restaurado para disposição default em Unix para que pipes para `head`/`wc`/`jaq` terminem limpos
- **G55 — Preservação de hardlink**: `--preserve-hardlinks` em `move` mantém contagem de hardlinks
- **G77 — Tamanho de stream de batch**: `--batch-size <N>` controla tamanho de chunk do `batch` para manifestos grandes
- **G81 — Formato raw de read**: `read --format raw` emite conteúdo cru, pula parsing JSON
- **v14 Tier 3 — 6 novos subcomandos**: `set`, `get`, `del`, `case`, `query`, `outline` (esta release)


## Notas sobre Tree-sitter-language-pack (v0.1.12)
### OBRIGATÓRIO
- SABER que `tree-sitter-language-pack = "1.8"` é a única nova dependência de runtime
- SABER que a feature `download` puxa parsers do GitHub no primeiro uso
- SABER que a feature `dynamic-loading` carrega parsers como bibliotecas compartilhadas (.so/.dll/.dylib) em runtime
- SABER que 24 linguagens têm cobertura de parser built-in: bash, c, cpp, css, elixir, go, html, java, javascript, json, kotlin, lua, markdown, ocaml, php, python, ql, ruby, rust, scala, sql, swift, toml, typescript, yaml
- SABER que 305+ linguagens adicionais estão disponíveis via dynamic-loading
- SABER que no Windows, o passo de download requer acesso de rede durante o primeiro `cargo install` ou `cargo build`
- SABER que no Linux, parsers são cacheados em `~/.cache/tree-sitter-language-pack/` (ou `$XDG_CACHE_HOME`)
- SABER que no macOS, o dynamic loader procura em `/usr/local/lib/` e `DYLD_LIBRARY_PATH`
### PROIBIDO
- NUNCA depender de parsers tree-sitter estarem disponíveis offline a menos que você os tenha pré-baixado
- NUNCA chamar `query` em arquivo com extensão não mapeada para uma linguagem (retornará erro)


## Resumo de Changelog v0.1.5-v0.1.14
### OBRIGATÓRIO — O Que Mudou Em Releases Intermediárias
- Esta seção consolida mudanças das releases v0.1.5 até v0.1.14 que a skill pulava anteriormente. Para detalhes completos, veja `CHANGELOG.md`
- **v0.1.5**: Adicionada flag global `--color auto|always|never`; corrigido bug de fall-through de locale em mensagens de erro
- **v0.1.6**: Adicionado `--follow-symlinks` aos comandos de travessia; allowlist de licenças do `cargo deny` expandida
- **v0.1.7**: Corrigido `RUSTSEC-2026-0009` via `time = "0.3.47+" DEPTH_LIMIT=32`; adicionado `--invert` ao `search`
- **v0.1.8**: Adicionado `--sort` ao `search` e `count --by-size`; semântica de `--max-count` melhorada
- **v0.1.9**: Adicionada flag global `--max-filesize`; `transform` reescrito com contexto de erro adequado
- **v0.1.10**: Adicionado `--batch-size` ao `batch`; adicionado gate miri no CI (apenas nightly); baseline de 320 testes
- **v0.1.11**: Adicionado esqueleto de `set`, `get`, `del` (incompleto — completado na v0.1.12); `--preserve-timestamps` ao `edit`; +29 testes
- **v0.1.12**: +96 testes, 5 novos códigos de erro, 6 novos subcomandos, sidecar WAL, tree-sitter, 7 ADRs, 7 schemas
- **v0.1.13/v0.1.14**: correções de CI Windows (E0433 do libc; `write --line-ending auto` determinístico em arquivos novos); +2 testes unitários
- **v0.1.15**: Esta release. G117 (edit multi-par com paridade fuzzy + `pair_results` + `--partial`), G118 (`write` resolve o alvo via `validate_path` antes dos pré-passos), GAP 18 (snapshot `dir_fsync` redigido), MSRV do CI 1.85→1.88; 461 testes, ADRs 0026-0027
- **v0.1.18**: G118 estendido para replace (G118+R), G119 limpeza inteligente de WAL (subcomando wal-heal), G120 guarda de stdin vazio para read/hash/edit/apply, follow-up do GAP 18; 502 testes (44 suítes, 0 falharam, 3 ignorados), ADRs 0028-0030, 30 subcomandos totais


## Padrões Agent-First v0.1.12
### Obrigatório — Padrões Específicos v0.1.12
- USAR `set`/`get`/`del` em vez de parsear TOML/JSON manualmente no código do agente
- USAR `query --kinds` primeiro para descobrir node kinds antes de rodar queries S-expression custosas
- USAR `outline --kind` para extrair assinaturas de função sem parsear código fonte
- USAR `case --dry-run` antes de qualquer renomeação em massa, depois capturar a contagem de arquivos do output do dry-run
- USAR `--syntax-check` em `write` ao modificar arquivos fonte, para falhar rápido em erros de sintaxe
- USAR `recover_orphan_journals` consultivamente — nunca fazer replay ou delete automático
- USAR os novos exit codes 83, 88, 91, 92, 93 na lógica de retry: LockTimeout é retentável, SyntaxError não é, OrphanJournal requer decisão do usuário
- USAR download offline do `tree-sitter-language-pack` como pre-flight em CI: `cargo install --locked atomwrite` baixará parsers no primeiro uso

### Obrigatório — Padrão: Pre-Flight de Verificação de Sintaxe
```bash
# Validar fonte Rust antes do commit
atomwrite --workspace . write --syntax-check src/lib.rs < new_lib.rs
# Exit 0 em sucesso, exit 88 (SyntaxError) em falha
```

### Obrigatório — Padrão: Batch de Update de Config Com Locking
```bash
# Atualizar múltiplas chaves TOML atomicamente com locking otimista
{
  echo '{"op":"set","target":"config.toml","key_path":"database.pool.max","value":"20"}'
  echo '{"op":"set","target":"config.toml","key_path":"features.experimental","value":"true"}'
} | atomwrite --workspace . batch --transaction --dry-run
```

### Obrigatório — Padrão: Busca AST-Aware
```bash
# Encontrar todas as funções nomeadas "main" na base de código
atomwrite --workspace . query -Q '(function_item name: (identifier) @name (#eq? @name "main"))' src/
```

### Obrigatório — Padrão: Revisão de Código Baseada em Outline
```bash
# Obter um mapa rápido de todos os itens top-level em um arquivo
atomwrite --workspace . outline src/lib.rs | jaq '.items[] | "\(.kind): \(.name)"'
```

## v0.1.22 (2026-06-17) — Padrões de Edits Sequenciais com Re-captura e edit-loop

### Padrão Correto — Edits Sequenciais com Re-captura de Checksum

Quando você encadeia múltiplos `edit` no mesmo arquivo, cada `edit` muda o checksum BLAKE3. Sem re-capturar o checksum antes de cada `--expect-checksum`, você recebe `STATE_DRIFT` (exit 82) espúrio.

**Padrão A — re-captura explícita**:

```bash
CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
echo "novo" | atomwrite --workspace . edit --after-line 10 \
  --expect-checksum "$CS" src/foo.rs

# Re-capturar antes do próximo edit
CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
echo "outro" | atomwrite --workspace . edit --after-line 20 \
  --expect-chksum "$CS" src/foo.rs
```

**Padrão B — flag `--allow-sequential-drift`** (opt-in):

```bash
CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
echo "novo" | atomwrite --workspace . edit --allow-sequential-drift \
  --after-line 10 --expect-checksum "$CS" src/foo.rs
echo "outro" | atomwrite --workspace . edit --allow-sequential-drift \
  --after-line 20 --expect-checksum "$CS" src/foo.rs
```

**Padrão C — sub-comando `edit-loop`** (N edições em 1 invocação):

```bash
echo '[{"old":"foo","new":"bar"},{"old":"baz","new":"qux"}]' \
  | atomwrite --workspace . edit-loop --backup --keep-backup src/foo.rs
```

### Padrão Correto — Prune-Backups

```bash
# Listar backups que seriam removidos (sem deletar)
atomwrite --workspace . prune-backups --max-age 86400 --dry-run /path/

# Remover backups com mais de 24 horas
atomwrite --workspace . prune-backups --max-age 86400 /path/

# Manter apenas os 3 backups mais recentes
atomwrite --workspace . prune-backups --max-count 3 /path/
```
