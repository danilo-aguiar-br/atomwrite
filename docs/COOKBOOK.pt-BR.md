# Livro de Receitas do atomwrite


[Read in English](COOKBOOK.md)

- Receitas práticas que você pode copiar e colar nos seus workflows de agente


## Objetivos
- Gastar menos tokens: um objeto NDJSON por chamada substitui o encadeamento de shell, e o `commands --brief` responde em 5.020 bytes onde a árvore completa custa 67.492
- Reduzir o custo financeiro, porque a conta acompanha os tokens e as idas e voltas, e uma chamada substitui as várias chamadas de shell que uma tarefa de arquivo pede
- Reduzir os erros com escrita atômica e checksum BLAKE3 em toda resposta, então um resultado velho ou parcial aparece em vez de ser presumido
- Reduzir as alucinações com discriminador `type`, schema fechado por verbo e código de saída que nomeia a falha
- Reduzir o tempo gasto em cada tarefa, porque a leitura, a busca e a escrita rodam num processo só em vez de numa corrente de ferramentas de shell
- Reduzir a latência com execução local e sem rede no caminho do arquivo, medida nas medianas de 5,6 ms no `read` e 6,7 ms no `write` de arquivo pequeno
- Multiplicar a velocidade das operações com `batch`, `edit-loop`, `replace`, `case`, `transform` e `codemod` resolvendo muitos arquivos numa chamada
- Reduzir o lead time entre a solicitação e o resultado verificado, porque a resposta já traz o checksum, o `target_resolved` e o próximo passo
- Tornar o agente mais rápido, mais veloz, mais eficaz e mais assertivo, já que o código de saída, o `code` e o `suggestion` dizem o que fazer sem tentar às cegas


## O Que Há de Novo na v0.1.38 (2026-11-04)
- O envelope de falha carrega `ok`, `code` e `exit`, e o `error: true` saiu porque duplicava `ok: false` mais `code`, custava 13 bytes em toda falha e nenhum consumidor decidia por ele sozinho
- O `read --format raw` para em 24.000 bytes e imprime uma linha no stderr com o `--lines` que lê o resto, e honra `--max-filesize` com exit 65 `FILE_TOO_LARGE`
- Uma imagem chega como uma linha `data:`, e `read doc.pdf` devolve o texto de cada página, com `--ocr` reconhecendo uma imagem ou uma página renderizada em Rust puro
- Um arquivo cuja assinatura de imagem está corrompida falha no `read` com exit 65 `INVALID_INPUT` em vez de devolver os bytes como texto
- O `outline` lê um título Markdown como uma `section` que vai até o fim da própria seção, e o `read --symbol Part` devolve essa seção inteira com as subseções
- O `search --format text` escreve uma linha `caminho:linha:coluna:texto` por match, e uma árvore em que todo arquivo era binário sai 89 `ALL_SKIPPED_BINARY` em vez de `NO_MATCHES`
- O `apply` casa linhas inteiras, aplica vários arquivos numa invocação e publica as linhas mudadas com `--show`
- O `delete -r <dir>` domina a árvore nomeada inteira, incluindo entradas ocultas, ignoradas, links, fifo, socket e dispositivo
- O `target_resolved` é sempre uma lista na ordem de invocação, porque um caminho pode conter a vírgula que antes unia vários alvos
- O lock mira o inode, e o `delete` toma o mesmo lock antes de calcular o hash e antes do unlink
- O `output.profile = "lean"` tira os digests, as contagens de bytes e o tempo de um registro de mutação bem-sucedida, o que levou um `apply` de três linhas de 433 bytes para 196
- O `config` lista 43 chaves, e o `atomwrite commands --brief` imprime uma linha por subcomando
- Fixe os agentes em `^0.1.38`. 47 subcomandos (SSOT via `agent-surface`)


## O Que Há de Novo na v0.1.37 (2026-10-31)
- Pin `^0.1.37`. 47 subcomandos (SSOT via `agent-surface`); o novo é `config`
- Um evento terminal por execução: o resumo que falha carrega `ok:false`, `code` e `exit` nele mesmo, e todo resumo que publica `files_failed` acrescenta `partial: true` e `errors[]` ao lado
- `edit-loop` com um par sem match sai 65 com `MATCH_FAILED`

### Novas Receitas (v0.1.37)
- Como Ler e Mudar uma Chave de Ajuste com config
  - `atomwrite --workspace "$WS" config get fuzzy.max_content_bytes` mostra o valor efetivo e a origem (`default`, `global`, `workspace` ou `explicit`)
  - `atomwrite --workspace "$WS" config set fuzzy.max_content_bytes 134217728 --scope workspace` grava a chave no `.atomwrite.toml` de forma atômica (`--scope global|workspace` é obrigatório)
  - `atomwrite --workspace "$WS" config unset fuzzy.max_content_bytes --scope workspace` a remove de novo
- Como Ler os Arquivos que a Varredura Não Alcançou
  - `atomwrite --workspace "$WS" search TODO src/ | jaq -c 'select(.type=="summary") | {files_failed, errors}'` lista cada arquivo com falha com `path`, `code` e `message` (no máximo `summary.errors_max_entries` entradas (50 por padrão); `files_failed` guarda a contagem total; as duas chaves não aparecem quando todos os arquivos foram alcançados)
- Como Ramificar pelo Exit Code
  - 65: leia `best_candidate`; 78: problema de config ou verbo ausente num build slim; 83: lock não obtido; 124: prazo, aumente `--timeout-secs` ou estreite o path; 141: o leitor fechou o pipe; 143: SIGTERM
- Receitas novas para todo comando que não tinha exemplo -- as seções `Como Gerenciar Chaves de Ajuste e Escopos com config` até `Como Moldar um Stream Sem jq`, abaixo de `Como Observar Progresso de Batch`
  - `config`, `set`, `get`, `del`, `copy`, `move`, `count`, `case`, `locale`, `query`, `outline` e `extract-artifact`
  - `edit-loop`, `backup`, `rollback`, `prune-backups`, `wal-stats`, `wal-heal`, `doctor`, `--print-schema`, o formato do `rules.yaml`, a ramificação por exit code e o evento terminal único


## O Que Há de Novo na v0.1.36 (2026-07-31)
- Histórico (v0.1.36): 46 subcomandos (SSOT via `agent-surface`)
- Shape agent-native: `--select`/`--fields`, `--filter`, `--limit` (sem `-n` global). Erros sempre emitem, exceto um pipe quebrado e uma divergência de checksum já publicada como um evento `hash_verify`
- Staging: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch` e `--old-file`/`--new-file` sob `.atomwrite/scratch/`
- Loops finos sem jq: `atomwrite --select ok,error,code --filter 'ok=false' --limit 1 <cmd>`
- GAP-019…033 fechados/mitigados (jail de content-source + erros `ok:false` + fuzzy honesto)

### Novas Receitas (v0.1.36)
- Como Estagiar um Patch com scratch init -- `--old-file`/`--new-file` dentro do jail
- Como Projetar NDJSON com --select/--filter/--limit -- shape agent-native sem jq
- Como Passar `$VAR` Literal com Heredoc entre Aspas Simples -- conteúdo seguro no shell
- Como Descobrir Prontidão do Host com doctor + commands -- bootstrap de agente
- Como Fazer Shrink Intencional com --allow-shrink -- override da guarda de shrink
- Como Usar Replace Aggressive Só Depois de --dry-run -- edições em massa fail-closed


## 47 Subcomandos (SSOT)

As receitas abaixo cobrem caminhos comuns de agente; a superfície completa são 47 ferramentas (`CANONICAL_SUBCOMMAND_COUNT` / `atomwrite agent-surface`). Todo comando desta lista traz a superfície local completa de flags em [docs/AGENTS.md](AGENTS.md) e [docs/HOW_TO_USE.md](HOW_TO_USE.md).
Descoberta:

```bash
atomwrite agent-surface
atomwrite commands --include-globals
```

- `agent-surface` — inventário de ferramentas derivado do clap (integração por subprocesso, CLI com saída NDJSON)
- `apply` — aplica patches da stdin (unified, search/replace, full, markdown)
- `backup` — cria backups com timestamp e checksums BLAKE3
- `batch` — lote multi-op via NDJSON (transação opcional)
- `calc` — expressões matemáticas e conversões de unidade
- `case` — renomeia identificadores em arquivos (`--subvert` + estilo `--to`), por exemplo `atomwrite case --subvert old_name new_name src/main.rs`
- `codemod` — campanha codemod multi-regra em YAML (delega ao transform), por exemplo `atomwrite codemod --rules rules.yaml src/`
- `commands` — emite a árvore clap completa em JSON para descoberta do agente
- `completions` — gera completions de shell (bash, zsh, fish, elvish, powershell)
- `config` — chaves persistentes de ajuste (`path|list|get|set|unset`, `--scope global|workspace`), por exemplo `atomwrite config set fuzzy.threshold 0.9 --scope workspace`
- `copy` — copia arquivos com verificação de checksum, por exemplo `atomwrite copy a.txt b.txt`
- `count` — conta linhas, matches ou arquivos por extensão, por exemplo `atomwrite count src/`
- `del` — remove uma chave de arquivo de config estruturado (TOML/JSON), por exemplo `atomwrite del Cargo.toml workspace.members`
- `delete` — apaga arquivos ou árvores, dono da árvore nomeada inteira no `-r`, com backup opcional (`--plan` só planeja)
- `diff` — compara dois arquivos ou um arquivo contra o stdin (unified ou stat)
- `doctor` — diagnostica ambiente e prontidão do host
- `edit` — edita cirurgicamente por linha, marcador ou match exato
- `edit-loop` — aplica N pares `old`/`new` da stdin NDJSON em um write atômico, por exemplo `atomwrite edit-loop src/main.rs < pairs.ndjson`
- `extract` — extrai campos de NDJSON via pipe (`--field` local; nunca `--select` local)
- `extract-artifact` — sinal delimitado de cabeçalhos de PDF, OOXML e imagem (nunca os bytes brutos do artefato), por exemplo `atomwrite extract-artifact report.pdf`
- `get` — lê um valor em caminho pontilhado em TOML/JSON, por exemplo `atomwrite get Cargo.toml package.version`
- `hash` — calcula checksums BLAKE3
- `list` — lista a estrutura de arquivos do projeto com metadados (um registro NDJSON por entrada)
- `locale` — mostra / define / limpa preferência de locale da UI (XDG)
- `move` — move ou renomeia arquivos atomicamente (fallback de cópia em EXDEV), por exemplo `atomwrite move a.txt b.txt`
- `outline` — extrai estrutura AST de alto nível (funções, classes, …), por exemplo `atomwrite outline src/main.rs`
- `prune-backups` — poda backups `.bak.*` por idade ou quantidade, ex. `atomwrite prune-backups --max-count 5 --recursive .`
- `query` — percorre AST tree-sitter e emite nós como NDJSON, por exemplo `atomwrite query --kinds src/main.rs`
- `read` — lê arquivos com metadados, checksum e conteúdo opcional
- `recipe` — receitas multi-passo nomeadas (`list` / `run`), por exemplo `atomwrite recipe list`
- `regex` — gera regex a partir de exemplos
- `replace` — substitui texto em arquivos com writes atômicos
- `rollback` — restaura um arquivo a partir de um backup anterior
- `scope` — escopo gramatical (apagar comentários, transformar case, …)
- `scratch` — staging sob `.atomwrite/scratch/`; nunca `/tmp` do SO fora do workspace (exit 126), por exemplo `atomwrite scratch init .atomwrite/scratch`
- `search` — busca conteúdo de arquivos em paralelo (engine ripgrep)
- `semantic-merge` — merge de 3 vias line-based (não AST/embedding), por exemplo `atomwrite semantic-merge --base b.txt --ours o.txt --theirs t.txt --output m.txt`
- `semantic-search` — busca offline por Jaccard de tokens (sem rede), por exemplo `atomwrite semantic-search 'atomic write' src/`
- `set` — grava um valor em caminho pontilhado em TOML/JSON, por exemplo `atomwrite set Cargo.toml package.name demo`
- `sparse` — list/read esparso orçado para janelas de contexto de agente, por exemplo `atomwrite sparse list src/ --max-files 20`
- `stat` — alias de metadados de `read --stat` (sem corpo), por exemplo `atomwrite stat src/main.rs`
- `transform` — refatoração AST via ast-grep
- `verify` — verifica integridade do arquivo por checksum BLAKE3, por exemplo `atomwrite verify a.txt <checksum>`
- `wal-heal` — remove journals WAL terminais obsoletos (seguro, idempotente), por exemplo `atomwrite wal-heal .`
- `wal-stats` — snapshot de diagnóstico local do WAL (contagens, idade, estimativa de reclaim), por exemplo `atomwrite wal-stats`
- `watch` — observa eventos do sistema de arquivos (feature `watch`), por exemplo `atomwrite watch src/`
- `write` — cria ou sobrescreve arquivos atomicamente via stdin


## O Que Há de Novo na v0.1.35 (2026-07-19)
- Gaps residuais A-* fechados (ADR-0055): suite verde sem `delete --yes`; sobrescrita grande default-deny (`--ack-overwrite`); `watch` sempre emite `watch_summary`; markers de conflito do semantic-merge default ON; monólitos fatiados com `include!` (SRP)
- Write em arquivos grandes: alvos existentes acima de XDG `[write].confirm_large_bytes` (padrão 100 KiB) exigem `--ack-overwrite` (agente one-shot; sem Y/N interativo). `--confirm` é flag legada de awareness; `--require-large-ack` é independente (sem colisão de alias)
- Delete: use `--plan` ou `--dry-run` para plan-only; `delete --confirm` e `delete --yes`/`-y` são rejeitados (fail-closed). Execute de novo sem essas flags para apagar
- Watch: NDJSON final `type:watch_summary` em todo caminho de saída; idle padrão 500 ms (`--idle-exit-ms` / XDG `[watch].idle_exit_ms`); debounce via CLI ou XDG `[watch].debounce_ms`
- Semantic-merge: markers de conflito gravados por padrão; opt-out `--no-conflict-markers`
- DoD local (executado à mão na estação de trabalho): `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`, `cargo check --target x86_64-pc-windows-gnu`, `cargo install --path . --force`
- Suite de contrato: `cargo test --test cli_e2e_v0135`
- Pin histórico `^0.1.35` (substituído por `^0.1.36`). Configuração somente CLI + XDG / `.atomwrite.toml` (sem knobs `ATOMWRITE_*` de produto; sem telemetria de produto)


## O Que Há de Novo na v0.1.34

A v0.1.34 (2026-07-19) é a publicação docs-complete do runtime one-shot da v0.1.33: multi-apply fuzzy é one-pass E→D (`apply_fuzzy_one_pass`); máximo de applies padrão 1; embeds forçam apply único; global `--timeout-secs` / `--timeout` padrão 120 (`0` desabilita, prazo esgotado exit 124). Histórico (na v0.1.34): 41 subcomandos.

### Novas Receitas (v0.1.34)
- Como Expandir uma Seção com Fuzzy Replace com Segurança -- NEW contém OLD; one-pass, sem hang
- Timeout global padrão 120s — jobs longos em monorepo usam `--timeout-secs 0` ou um valor maior

### Novas Receitas (superfície v0.1.29 + residual v0.1.30)
- Como Fazer Replace Fuzzy em um Monorepo -- `replace --fuzzy auto` com `--progress-every` opcional
- Como Rodar search-replace-verify -- `recipe run --name search-replace-verify`
- Como Orçar um Sparse List -- `sparse list --max-files` / `--max-bytes`
- Como Fazer Semantic Merge de 3 Vias (line-based, não AST) -- `semantic-merge --base --ours --theirs`
- Como Escolher Durability no write -- `--durability full|fast|auto`
- Como Instalar Slim vs Full -- `--no-default-features --features core` vs default
- Como Recuperar Match Falho com best_candidate -- inspecione o near-miss no envelope de erro e retente
- Como Inspecionar rename_method e durability -- `platform.*` via `jaq` (Linux `renameat2`)
- Como Ler o Método de Backup Honesto -- `platform.backup_method` é `reflink_or_copy` (nunca hardlink do arquivo vivo)
- Como Inventariar a Superfície do Agente -- `agent-surface`
- Como Fazer Stat Sem Ler o Corpo -- `stat` (alias de `read --stat`)
- Como Observar com Debounce -- `watch` (feature `watch`)
- Como Fazer Semantic-Search Offline (Jaccard) -- `semantic-search` (sempre compilado), `--index-dir`
- Como Rodar uma Campanha Codemod -- `codemod --dry-run`, `by_rule_id`
- Como Observar Progresso de Batch -- heartbeats de batch + `--progress-every` no replace


## Como Expandir uma Seção com Fuzzy Replace com Segurança
- Edições de expand-section de agentes frequentemente produzem NEW que contém OLD (o bloco original mais linhas)
- Antes da v0.1.33, reescanear texto inserido podia hangar por dias; one-pass E→D nunca reescaneia texto inserido
- Quando o replacement contém o pattern (string fixa), atomwrite força apply único mesmo com `--max-replacements` enorme
- Máximo de applies fuzzy padrão é 1 quando `--max-replacements` é omitido (teto rígido 10_000)
- Global `--timeout-secs` tem padrão 120 (exit 124 no prazo); passe `0` só se quiser runtime ilimitado de propósito

```bash
# NEW embute OLD — seguro, termina em um passe (sem hang)
atomwrite --workspace . replace --fuzzy auto \
  $'## Setup\n\nInstall deps.' \
  $'## Setup\n\nInstall deps.\n\n### Extra\nAlso run migrations.' \
  docs/README.md

# Multi-hit explícito ainda é one-pass no conteúdo original
atomwrite --workspace . replace --fuzzy auto --max-replacements 3 \
  'TODO' 'DONE' src/
```


## Como Fazer Replace Fuzzy em um Monorepo
- Use replace fuzzy quando indentação ou whitespace diverge entre pacotes
- Prefira `--fuzzy auto` (cascata padrão para agentes); `--fuzzy off` = exact-only, não rejeitado
- Emita progresso NDJSON em árvores grandes com `--progress-every`
- Lembre: timeout global padrão é 120s (exit 124); use `--timeout-secs 0` para varreduras ilimitadas de monorepo
- Teto rígido de applies fuzzy é 10_000; o padrão permanece 1 quando `--max-replacements` é omitido

```bash
# Prévia primeiro
atomwrite --workspace . replace --dry-run --fuzzy auto \
  'legacy_api(' 'new_api(' packages/ --include '*.rs'

# Aplicar com progresso a cada 50 arquivos
atomwrite --workspace . replace --fuzzy auto --progress-every 50 \
  'legacy_api(' 'new_api(' packages/ --include '*.rs'
```


## Como Rodar search-replace-verify
- Recipe built-in busca, substitui e depois hasheia para integridade
- Exige `--pattern` e `--replacement`

```bash
atomwrite --workspace . recipe list
atomwrite --workspace . recipe run --name search-replace-verify \
  --path src --pattern OLD_TOKEN --replacement NEW_TOKEN --fuzzy auto
```


## Como Orçar um Sparse List
- Limite contagem de arquivos e orçamento de bytes de path para janelas de contexto de agente
- O arquivo de paths do `sparse read` precisa estar dentro do workspace: um caminho fora sai 126

```bash
atomwrite --workspace . sparse list --max-files 100 --max-bytes 1048576 src/
atomwrite --workspace . sparse list --max-files 20 --include '*.rs' .
# Alimente paths no sparse read (o arquivo de paths fica dentro do workspace)
atomwrite --workspace . scratch init .atomwrite/scratch
atomwrite --workspace . sparse list --max-files 20 src/ \
  | jaq -r 'select(.type=="sparse_entry") | .path' > .atomwrite/scratch/paths.txt
atomwrite --workspace . sparse read --paths-file .atomwrite/scratch/paths.txt --head 30 --max-files 20
```


## Como Fazer Semantic Merge de 3 Vias (line-based, não AST)
- Una edições concorrentes de agentes com uma base comum
- Base, ours, theirs e saída precisam estar dentro do workspace: um caminho fora sai 126
- Por padrão o conflito é gravado como markers `<<<<<<<` na saída, o evento diz `status: "conflict"` e a execução sai 0
- Adicione `--fail-on-conflict` para exit 84 `MERGE_CONFLICT` em conflitos não resolvidos (o arquivo de saída não é gravado), ou `--no-conflict-markers` para manter o ours sem markers

```bash
atomwrite --workspace . scratch init .atomwrite/scratch
# .atomwrite/scratch/{base,ours,theirs}.rs guardam as três versões
atomwrite --workspace . semantic-merge \
  --base .atomwrite/scratch/base.rs \
  --ours .atomwrite/scratch/ours.rs \
  --theirs .atomwrite/scratch/theirs.rs \
  --output src/merged.rs --fail-on-conflict
```


## Como Escolher Durability no write
- `full` para configs e estado durável
- `fast` para artefatos efêmeros
- `auto` para resolução de política padrão

```bash
echo critical | atomwrite --workspace . write --durability full config.toml
# Saída efêmera permanece dentro do jail do workspace (nunca /tmp do SO)
atomwrite --workspace . scratch init .atomwrite/scratch
echo scratch | atomwrite --workspace . write --durability fast .atomwrite/scratch/ephemeral-out.txt
# Ou um diretório de staging relativo ao workspace:
mkdir -p ./tmp-agent
echo scratch | atomwrite --workspace . write --durability fast ./tmp-agent/ephemeral-out.txt
```


## Como Estagiar um Patch com scratch init
- Caminhos content-source de `--old-file`/`--new-file` passam pelo jail (GAP-019)
- Nunca estagie sob `/tmp` do SO — exit 126 `WORKSPACE_JAIL`
- Staging canônico: `{workspace}/.atomwrite/scratch/` via `scratch init`

```bash
atomwrite --workspace . scratch init .atomwrite/scratch
SCR=.atomwrite/scratch
printf %s "$OLD" >"$SCR/old.txt"
printf %s "$NEW" >"$SCR/new.txt"
atomwrite --workspace . edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" src/f.rs
```


## Como Projetar NDJSON com --select/--filter/--limit
- Flags globais de shape aplicam-se a todo subcomando (e ao path de erro)
- `--select`/`--fields` projetam chaves top-level; `--filter` é igualdade top-level (AND repetível)
- `--limit N` é long-only (sem `-n` global); erros sempre emitem mesmo sob filter, exceto um pipe quebrado e uma divergência de checksum já publicada como um evento `hash_verify`
- Os hits de search trazem o texto casado em `lines` (não existe chave `text`)

```bash
# Parse compacto de agente sem jq
atomwrite --workspace . --select ok,error,code,match_count,suggestion \
  edit --old missing --new x --fuzzy off src/f.rs

# Só a primeira falha
atomwrite --workspace . --filter ok=false --limit 1 \
  edit --old a --new b src/f.rs

# Projeta campos de sucesso em hits de search
atomwrite --workspace . --select path,line_number,lines --limit 20 \
  search 'TODO' src/
```


## Como Passar `$VAR` Literal com Heredoc entre Aspas Simples
- Aspas duplas e heredocs sem aspas expandem variáveis do shell
- Delimitador de heredoc entre aspas simples preserva `$VAR`, backticks e `//` literalmente
- Prefira `--old-file`/`--new-file` para patches multi-linha (ainda estagie sob o workspace)

```bash
# $VAR literal e metacaracteres do shell — sem expansão
atomwrite --workspace . edit src/profile --old 'export PATH=$PATH' --new 'export PATH=$HOME/bin:$PATH'

# Corpo multi-linha literal via heredoc com aspas simples → arquivo estagiado
atomwrite --workspace . scratch init .atomwrite/scratch
SCR=.atomwrite/scratch
cat >"$SCR/old.txt" <<'EOF'
# install
curl -fsSL https://example.com | sh
export TOKEN=$SECRET
EOF
cat >"$SCR/new.txt" <<'EOF'
# install
curl -fsSL https://example.com | sh
export TOKEN=$SECRET
# pinned
EOF
atomwrite --workspace . edit docs/install.md \
  --old-file "$SCR/old.txt" --new-file "$SCR/new.txt"
```


## Como Descobrir Prontidão do Host com doctor + commands
- `doctor` reporta prontidão de host/workspace/features em um objeto NDJSON
- `commands` emite a árvore completa do clap para descoberta de agente
- Combine com `agent-surface` para o manifesto de tools (integração por subprocesso, CLI com saída NDJSON)

```bash
atomwrite --workspace . doctor --strict
atomwrite --workspace . commands --include-globals
atomwrite --workspace . agent-surface --format json
```


## Como Fazer Shrink Intencional com --allow-shrink
- A guarda de shrink bloqueia qualquer sobrescrita que reduza um arquivo existente em mais de 50% (exit 65), com ou sem `--expect-checksum`
- Passe `--allow-shrink` só quando o truncamento for intencional

```bash
CS=$(atomwrite --workspace . read target.rs | jaq -r '.checksum')
# Bloqueado sem a flag quando stdin é bem menor:
echo "tiny" | atomwrite --workspace . write --expect-checksum "$CS" target.rs
# Rewrite com shrink intencional:
echo "tiny" | atomwrite --workspace . write --expect-checksum "$CS" --allow-shrink target.rs
```


## Como Usar Replace Aggressive Só Depois de --dry-run
- `--fuzzy aggressive` é poderoso e pode over-match; nunca aplique em massa às cegas
- Sempre pré-visualize com `--dry-run` primeiro; inspecione o NDJSON e só então aplique

```bash
# Só prévia
atomwrite --workspace . replace --dry-run --fuzzy aggressive \
  'legacy_api(' 'new_api(' packages/ --include '*.rs'

# Aplique só depois que o envelope do dry-run parecer correto
atomwrite --workspace . replace --fuzzy aggressive --progress-every 50 \
  'legacy_api(' 'new_api(' packages/ --include '*.rs'
```


## Como Instalar Slim vs Full
- Slim (26.560.352 bytes medidos em 2026-11-04, DERIVADO: 26560352 / 1048576 = 25,33 MiB, acima do size-gate local de 15 MiB): só core, sem stack AST
- Default (71.084.440 bytes medidos em 2026-11-04, DERIVADO: 71084440 / 1048576 = 67,79 MiB): linguagens AST incluídas
- Ambos os números vêm de `cargo build --release --locked` com e sem `--no-default-features --features core`, e o binário slim responde `atomwrite 0.1.38` e sai 78 no `outline`

```bash
# Slim
cargo install --path . --locked --force --no-default-features --features core
# Default com AST
cargo install --path . --locked --force
```


## Como Recuperar Match Falho com best_candidate
- Falhas de match podem emitir `best_candidate` (line, similarity, strategy, text, diff_preview)
- Use o near-miss para corrigir `old` sem reler o arquivo inteiro
- `--fuzzy off` = exact-only, e o padrão é `auto`
- Um miss no `edit` sai 65 `MATCH_FAILED` com `best_candidate`, e um miss no `replace` numa árvore sai 1 `NO_MATCHES`, então use `edit` quando quiser o near-miss

```bash
# Capture o envelope de erro no near-miss (exit 65 / MATCH_FAILED)
atomwrite --workspace . scratch init .atomwrite/scratch
atomwrite --workspace . edit src/main.rs \
  --old 'fn main(){let x=1;}' --new 'fn main() { let x = 2; }' \
  | tee .atomwrite/scratch/miss.json \
  | jaq '{exit, best: .best_candidate, ok, match_count}'

# Retente com o texto do candidato como novo --old
atomwrite --workspace . edit src/main.rs \
  --old "$(jaq -r '.best_candidate.text' < .atomwrite/scratch/miss.json)" \
  --new 'fn main() { // entry point'
```


## Como Inspecionar rename_method e durability
- O NDJSON de write reporta `platform.durability`, `platform.rename_method`, `platform.backup_method`
- No Linux prefere `renameat2` quando disponível, senão `rename`
- Parseie campos platform com `jaq` após cada escrita durável

```bash
echo payload | atomwrite --workspace . write --durability full config.toml \
  | jaq '{durability: .platform.durability, rename_method: .platform.rename_method, backup_method: .platform.backup_method, fsync: .platform.fsync}'

# Caminho rápido para saída efêmera (relativo ao workspace; jail mantido)
atomwrite --workspace . scratch init .atomwrite/scratch
echo scratch | atomwrite --workspace . write --durability fast .atomwrite/scratch/ephemeral-out.txt \
  | jaq -r '.platform | "\(.durability) \(.rename_method)"'
```


## Como Ler o Método de Backup Honesto
- Backup do arquivo vivo NUNCA é hardlink do mesmo inode
- Inspecione `platform.backup_method` (`reflink_or_copy` ou `copy`)
- Prefira essa honestidade em pipelines de agente que corromperiam .bak via inode compartilhado

```bash
# Force sobrescrita com backup e leia o método usado
echo "v2" | atomwrite --workspace . write --keep-backup config.toml \
  | jaq -r '.platform.backup_method // "none"'

# Compare com write simples (backup transacional padrão auto-remove no sucesso)
echo "v3" | atomwrite --workspace . write config.toml \
  | jaq '{backup_method: .platform.backup_method, rename_method: .platform.rename_method}'
```


## Como Inventariar a Superfície do Agente
- `agent-surface` emite manifesto de tools derivado do clap
- Integração por subprocesso, CLI com saída NDJSON; conecte Bash/argv a partir deste inventário
- Use para registrar tools no host sem listas de flags mantidas à mão

```bash
atomwrite --workspace . agent-surface
atomwrite --workspace . agent-surface | jaq '{version, integration: .integration, tools: [.tools[].name]}'
```


## Como Fazer Stat Sem Ler o Corpo
- `stat` é alias fino de `read --stat`
- Retorna metadados e checksum sem bytes do corpo
- Prefira em loops de agente que só precisam de size, mode ou BLAKE3

```bash
atomwrite --workspace . stat src/lib.rs
atomwrite --workspace . stat src/lib.rs | jaq '{path, bytes, checksum, lines, mode}'
# Equivalente:
atomwrite --workspace . read --stat src/lib.rs
```


## Como Observar com Debounce
- Exige build com `--features watch` (ou `--features full`), e o build padrão sai 78 `CONFIG_INVALID` com a dica de rebuild
- Coalesce eventos por path com `--debounce-ms`
- Limite a vida com `--max-events`; `--checksum` opcional para BLAKE3 em arquivos regulares

```bash
# Compile com watch e transmita eventos com debounce
cargo install --path . --locked --force --features watch
atomwrite --workspace . watch . --debounce-ms 200 --max-events 20
atomwrite --workspace . watch src/ --debounce-ms 100 --checksum --max-events 5
```


## Como Fazer Semantic-Search Offline (Jaccard)
- Ranking offline por tokens (Jaccard); não é embedding OpenRouter
- Sempre compilado: sem gate de feature
- Passe `--index-dir` para backend de índice invertido em diretório local

```bash
atomwrite --workspace . semantic-search "atomic rename tempfile" src/ --k 10
atomwrite --workspace . semantic-search "fsync durability" . --k 20 --min-score 0.1 \
  --index-dir .atomwrite/semantic-index
```


## Como Rodar uma Campanha Codemod
- Campanha AST multi-regra sobre arquivo YAML de rules
- Sempre dry-run primeiro, e o evento final `codemod_summary` carrega `by_rule_id`
- O formato do `rules.yaml` está em `Como Escrever o rules.yaml do codemod e do transform --rules`
- Aplique só depois que o envelope do dry-run estiver correto

```bash
atomwrite --workspace . codemod --rules rules.yaml --dry-run src/
atomwrite --workspace . codemod --rules rules.yaml --dry-run src/ \
  | jaq -c 'select(.type=="codemod_summary") | .by_rule_id'
# Aplicar
atomwrite --workspace . codemod --rules rules.yaml src/
```


## Como Observar Progresso de Batch
- Batch e replace longos checam cancel cooperativo entre unidades (SIGINT/SIGTERM → cancelled / exit 143)
- `replace --progress-every N` emite heartbeats NDJSON (`type: progress`)
- Batch emite heartbeats automáticos a cada cerca de max(1, total/20) ops (teto 50)
- Suprima progresso com `-q` / `-qq` global quando só o summary final importa

```bash
# Replace em massa com progresso a cada 50 arquivos (padrão)
atomwrite --workspace . replace --progress-every 50 'old_api' 'new_api' src/ \
  | jaq -c 'select(.type=="progress" or .type=="summary")'

# Batch transacional com heartbeats embutidos
printf '%s\n' \
  '{"op":"write","target":"a.txt","content":"one"}' \
  '{"op":"write","target":"b.txt","content":"two"}' \
  | atomwrite --workspace . batch --transaction \
  | jaq -c 'select(.type=="progress" or .type=="summary")'
```


## Como Gerenciar Chaves de Ajuste e Escopos com config
- `config` é a única forma suportada de mudar uma chave de ajuste persistente: ele valida o valor, preserva comentários e grava por tempfile, fsync e rename
- `config list` e `config get CHAVE` não aceitam `--scope`: mostram `value`, `origin` (`default`, `global`, `workspace` ou `explicit` quando `--config CAMINHO` substitui os dois arquivos) e `default`
- `config set` e `config unset` exigem `--scope global|workspace` e não têm escopo padrão: sem ele o verbo sai 2 e não toca em nada
- `global` é o `config.toml` do XDG, `workspace` é o `.atomwrite.toml` na raiz do workspace
- `log.file`, `log.rotation`, `storage.home` e `locale` são só globais (escopo workspace sai 65), e chave desconhecida ou valor fora da faixa sai 65 sem gravar
- O evento terminal carrega `target_resolved` e `target_source: "argv"`: confira se o caminho é o arquivo do escopo que você quis
- As receitas abaixo reutilizam a variável `WS` definida no primeiro bloco

```bash
WS="$PWD"   # caminho absoluto da raiz do workspace, definido uma vez
atomwrite --workspace "$WS" config path --scope workspace
atomwrite --workspace "$WS" config list | jaq -r '.keys[] | "\(.key)=\(.value) (\(.origin))"'
atomwrite --workspace "$WS" config get fuzzy.threshold --select key,value,origin,default
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace --select ok,changed,target_resolved
atomwrite --workspace "$WS" config set write.confirm_large_bytes 204800 --scope global
atomwrite --workspace "$WS" config unset fuzzy.threshold --scope workspace --select ok,changed,old_value
atomwrite --workspace "$WS" config unset write.confirm_large_bytes --scope global

# Recusado, nada é gravado
atomwrite --workspace "$WS" config set locale en --scope workspace    # exit 65, chave só global
atomwrite --workspace "$WS" config set fuzzy.threshold 2 --scope workspace  # exit 65, faixa 0..=1
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9            # exit 2, --scope é obrigatório
```


## Como Editar Chaves TOML ou JSON com set, get e del
- Use `set`, `get` e `del` em um caminho pontilhado em vez de ler e reescrever o arquivo inteiro
- `set` infere o tipo pelo valor (`9` vira inteiro, `true` vira booleano)
- `get` e `del` numa chave que não existe saem 65, e `del --force-missing` sai 0 com `existed: false`
- Cada um deles sai 65 num arquivo que não é TOML ou JSON válido

```bash
printf '[database.pool]\nmax = 5\n' | atomwrite --workspace "$WS" write --no-backup settings.toml
atomwrite --workspace "$WS" set settings.toml database.pool.max 9
atomwrite --workspace "$WS" get settings.toml database.pool.max --select key_path,value,found,format
atomwrite --workspace "$WS" del settings.toml database.pool.max
atomwrite --workspace "$WS" del settings.toml database.pool.max                 # exit 65, chave não encontrada
atomwrite --workspace "$WS" del --force-missing settings.toml database.pool.max   # exit 0, existed false
```


## Como Copiar e Mover Arquivos
- `copy` e `move` recusam um alvo existente com exit 65, e `--force` (ou `--backup`) sobrescreve
- `copy --recursive --preserve` copia uma árvore de diretórios e mantém modos e datas
- `move` recorre a copiar e apagar quando o alvo está em outro dispositivo (EXDEV)

```bash
atomwrite --workspace "$WS" copy src/lib.rs src/lib_copy.rs
atomwrite --workspace "$WS" move src/lib_copy.rs src/lib_moved.rs
atomwrite --workspace "$WS" copy src/lib.rs src/lib_moved.rs           # exit 65, alvo existe
atomwrite --workspace "$WS" copy --force src/lib.rs src/lib_moved.rs
atomwrite --workspace "$WS" copy --recursive --preserve src snapshot/
```


## Como Contar Linhas por Extensão ou Tamanho
- `count --by-extension` devolve um mapa com `files`, `lines`, `blank` e `bytes` por extensão
- `count --by-size --top N` lista os N maiores arquivos

```bash
atomwrite --workspace "$WS" count --by-extension src/
atomwrite --workspace "$WS" count --by-size --top 3 src/ | jaq -c '.items[] | {path, bytes}'
```


## Como Renomear um Identificador com case
- `case` aceita só arquivos: um diretório sai 0 e não muda nada (`identifiers_total: 0`), então liste os arquivos
- `--subvert ANTIGO NOVO` nomeia o par e `--to` escolhe o estilo (`snake` por padrão, `camel`, `pascal`, `kebab`, `screaming-snake`)
- Visualize com `--dry-run` primeiro: o resumo mostra `identifiers_total` e `files_modified: 0`
- Um arquivo que mudou entre a leitura e a gravação falha com `STATE_DRIFT` (82): rode o comando de novo

```bash
atomwrite --workspace "$WS" case --to kebab --subvert setupThing setup-thing --dry-run src/names.rs
atomwrite --workspace "$WS" case --to kebab --subvert setupThing setup-thing src/names.rs src/other.rs
```


## Como Ver o Idioma Resolvido
- `locale` informa o idioma usado em mensagens e sugestões: `resolved`, `source` (`system`, `persisted`) e `available`
- `locale --set en` grava uma preferência no diretório de config do XDG e `locale --clear` a remove
- Um comando pode sobrescrever uma vez com o `--locale en` global, e o campo `suggestion_lang` nomeia o idioma de `suggestion`

```bash
atomwrite locale --select type,resolved,source,available
atomwrite locale --set en
atomwrite locale --clear
```


## Como Listar Nós AST e um Outline do Código
- `query --kinds` conta cada tipo de nó de um arquivo e `query -Q TIPO` emite os nós de um tipo com o texto
- `outline` lista funções, structs e outros itens, `--kind` filtra (repetível) e `--positions` acrescenta a faixa de bytes
- Os dois precisam da gramática da linguagem: o build padrão baixa o parser no primeiro uso e o guarda em cache, e um build slim sai 78

```bash
atomwrite --workspace "$WS" query --kinds src/main.rs --limit 5
atomwrite --workspace "$WS" query -Q function_item src/main.rs --select kind,text --limit 2
atomwrite --workspace "$WS" outline --kind function_item --positions src/main.rs --select name,start_line,end_line
```


## Como Extrair um Sinal Delimitado de PDF, OOXML ou Imagem
- `extract-artifact` lê só cabeçalhos e uma amostra delimitada de um PDF, de um pacote OOXML ou de uma imagem, nunca os bytes brutos: para uma imagem devolve `kind`, `format`, `width`, `height` e `megapixels`
- Os limites vêm das chaves `extract.*` do config, e uma flag (`--max-text-chars`, `--max-sample-rows`) vence a chave
- Um arquivo que não é PDF, pacote OOXML nem imagem sai 65

```bash
atomwrite --workspace "$WS" extract-artifact img.png --select type,kind,format,width,height
atomwrite --workspace "$WS" extract-artifact img.png --max-text-chars 200
atomwrite --workspace "$WS" extract-artifact src/main.rs        # exit 65, sem magic bytes suportados
```


## Como Aplicar N Pares com edit-loop
- `edit-loop` lê pares `{"old": ..., "new": ...}` (um por linha, ou um array JSON) e grava o arquivo uma vez
- O padrão é tudo-ou-nada: um par sem match sai 65 `MATCH_FAILED` com `failed_pair_index` e nada é gravado
- Um par cujo texto `old` aparece mais de uma vez também é recusado com exit 65: alongue o `old` até ficar único
- `--partial` grava os pares que casam e sai 87 `EDIT_LOOP_PARTIAL` com `pairs_applied` e `pairs_unmatched`
- Leia `best_candidate` depois de um miss quando ele existir: mostra o texto mais próximo no arquivo

```bash
printf '%s\n' '{"old":"alpha","new":"ALPHA"}' '{"old":"gamma","new":"GAMMA"}' \
  | atomwrite --workspace "$WS" edit-loop src/el.txt --select ok,pairs_total,pairs_applied,checksum_after

# Um par não casa: exit 65, nada é gravado
printf '%s\n' '{"old":"ALPHA","new":"a"}' '{"old":"zzz","new":"y"}' \
  | atomwrite --workspace "$WS" edit-loop src/el.txt --select ok,code,exit,failed_pair_index,best_candidate

# Mantém os pares que casam: exit 87
printf '%s\n' '{"old":"ALPHA","new":"a"}' '{"old":"zzz","new":"y"}' \
  | atomwrite --workspace "$WS" edit-loop --partial src/el.txt --select ok,code,exit,pairs_applied,pairs_unmatched
```


## Como Desfazer uma Mudança com backup e rollback
- `backup` grava `ARQUIVO.bak.YYYYMMDD_HHMMSS_mmm` ao lado do arquivo e devolve `backup_path`
- `rollback ARQUIVO --latest --verify` restaura o backup mais novo e confere o hash BLAKE3, `--timestamp` escolhe um por prefixo e `--dry-run` só planeja
- A guarda de shrink recusa uma sobrescrita que reduz o arquivo em mais de 50% (exit 65), a menos que você passe `--allow-shrink`
- Os verbos que mutam removem o próprio backup no sucesso, `--keep-backup` o mantém, e `delete` mantém o dele porque o arquivo apagado não tem outra cópia
- `prune-backups` recebe os ARQUIVOS ALVO (não um diretório): um diretório como argumento não casa com nada

```bash
atomwrite --workspace "$WS" backup src/lib.rs --select type,ok,backup_path
echo "changed" | atomwrite --workspace "$WS" write --no-backup --allow-shrink src/lib.rs
atomwrite --workspace "$WS" rollback src/lib.rs --latest --dry-run
atomwrite --workspace "$WS" rollback src/lib.rs --latest --verify --select ok,type,restored_from,verified
echo "again" | atomwrite --workspace "$WS" write --keep-backup --allow-shrink src/lib.rs
atomwrite --workspace "$WS" prune-backups --max-count 0 --dry-run src/lib.rs   # lista o que sairia
atomwrite --workspace "$WS" prune-backups --max-count 0 src/lib.rs              # remove todo backup do arquivo
```


## Como Inspecionar e Curar os Journals Write-Ahead
- `wal-stats` só lê: informa `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `auto_heal_recommended` e `estimated_reclaim_bytes`
- `wal-heal CAMINHO` remove journals terminados mais antigos que `--threshold-secs` e nunca toca nos `started`, o `CAMINHO` é obrigatório e `--max-duration-ms` limita a varredura
- Uma execução cancelada por SIGTERM, SIGINT ou prazo não deixa journal, e um crash (`kill -9`) pode deixar um `started`, que o `wal-heal` mantém de propósito, então olhe com `wal-stats` antes de decidir

```bash
atomwrite --workspace "$WS" wal-stats --select total_journals,auto_heal_recommended,estimated_reclaim_bytes
atomwrite --workspace "$WS" wal-heal . --threshold-secs 3600 --max-duration-ms 5000 --select removed,preserved,bytes_reclaimed
atomwrite --workspace "$WS" wal-stats | jaq -e '.total_journals == 0'   # exit 1 enquanto houver journals
```


## Como Barrar um Pipeline com doctor --strict
- `doctor` informa `checks[]` com `name`, `status` e `detail`, e `doctor --strict` sai 1 (`DOCTOR_CHECKS_FAILED`) quando um check não passa
- Rode primeiro e pare o pipeline num exit diferente de zero

```bash
atomwrite doctor --strict --select type,version,ok || exit 1
atomwrite --workspace "$WS" doctor | jaq -r '.checks[] | select(.status != "pass") | "\(.name): \(.detail)"'
```


## Como Descobrir Schemas de Saída com --print-schema
- `--print-schema` sem nome imprime o nome de cada schema que o binário publica, uma string JSON por linha
- `--print-schema NOME` imprime esse JSON Schema e sai 0, e um nome desconhecido sai 65
- Cada verbo serve um schema para o stream inteiro: `search`, `replace`, `list` e `backup` respondem com a união dos seus registros, do resumo terminal e do envelope de erro, e `count-by-ext` e `count-by-size` têm nome próprio
- `--json-schema SUBCOMANDO` é a grafia antiga e continua funcionando

```bash
atomwrite --print-schema | jaq -r '.' | head -n 5
atomwrite --print-schema search | jaq -r '.title'
atomwrite --print-schema config | jaq -r '.title'
atomwrite --print-schema no-such-schema          # exit 65
```


## Como Ramificar pelo Exit Code e Retentar Só Quando for Retryable
- Leia o exit code antes da saída, depois leia `error_class` e `retryable` do evento terminal único, o que exige passar `-v`: o envelope padrão mantém `code`, `exit` e `suggestion` e descarta a prosa de diagnóstico
- Retente só quando `retryable` for `true` (exit 28, 30, 73, 74, 82, 83 e 92), e `permanent`, `precondition_failed` e `conflict` com `retryable: false` pedem outra ação
- 82 `STATE_DRIFT`: leia o arquivo de novo e refaça o comando, e 83 `LOCK_TIMEOUT`: o verbo já esperou `lock.acquire_timeout_ms` (5 s por padrão)
- 65 `MATCH_FAILED` ou `MATCH_AMBIGUOUS`: leia `best_candidate` e `match_count`, 124: aumente `--timeout-secs` ou estreite o path, 141: o leitor fechou o pipe, então a saída está incompleta
- Os verbos já espaçam as próprias tentativas com as chaves `retry.*`, então limite o número de tentativas e deixe a espera para o seu agendador

```bash
attempt=1
while [ "$attempt" -le 3 ]; do
  out=$(atomwrite -v --workspace "$WS" edit src/main.rs --old "let x = 1;" --new "let x = 2;")
  code=$?
  [ "$code" -eq 0 ] && break
  [ "$(printf '%s' "$out" | jaq -r '.retryable // false')" = "true" ] || break
  attempt=$((attempt + 1))
done
printf '%s\n' "$out" | jaq -c '{ok, code, exit, error_class, retryable, suggestion}'
```


## Como Escrever o rules.yaml do codemod e do transform --rules
- O arquivo é uma lista YAML, cada item tem `language`, `pattern` e `rewrite`, e o `id` é opcional
- Uma regra com `id` aparece em `by_rule_id` do evento final `codemod_summary`
- O estilo ast-grep com `rule:` e `fix:` não é aceito: o `codemod` sai 65 `INVALID_INPUT`
- Rode sempre com `--dry-run` primeiro, e `--inline-rules` recebe a mesma lista como string

```bash
cat >rules.yaml <<'EOF'
- id: old-to-new
  language: rust
  pattern: old_fn($$$ARGS)
  rewrite: new_fn($$$ARGS)
- language: rust
  pattern: $E.unwrap()
  rewrite: $E?
EOF
atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/ \
  | jaq -c 'select(.type=="codemod_summary") | .by_rule_id'
atomwrite --workspace "$WS" transform --rules rules.yaml --dry-run src/
atomwrite --workspace "$WS" transform --inline-rules '[{"language":"rust","pattern":"old_fn($$$A)","rewrite":"new_fn($$$A)"}]' --dry-run src/
```


## Como Ler os Arquivos que a Varredura Não Alcançou
- `search`, `replace`, `transform` e `scope` relatam cada arquivo que não conseguiram ler ou gravar como `files_failed` e `errors[]` (as primeiras `summary.errors_max_entries` entradas (50 por padrão)) no resumo
- Raiz nomeada na invocação que não existe reprova a execução mesmo tendo casado, com exit 4 `FILE_NOT_FOUND` e `ok: false` no resumo
- Falha encontrada ao varrer um diretório mantém exit 0, e o resumo traz `partial: true` sempre que `files_failed` aparece, então uma chave só diz que a resposta veio de parte dos alvos
- Zero matches mais uma falha sai com o código do primeiro erro (por exemplo 13 `PERMISSION_DENIED`)
- As três chaves não aparecem quando todos os arquivos foram alcançados

```bash
atomwrite --workspace "$WS" search TODO src/ \
  | jaq -c 'select(.type=="summary") | {ok, partial, files_failed, errors: [.errors[]? | {path, code}]}'
atomwrite --workspace "$WS" replace --dry-run TODO DONE src/ \
  | jaq -c 'select(.type=="summary") | {files_modified, files_failed}'
```


## Como Ler o Evento Terminal Único de uma Execução
- Uma execução com falha termina em exatamente um evento com `ok: false`, e ele carrega `code`, `exit` e `suggestion` nele mesmo: não espere uma segunda linha de erro
- Acrescente `-v` quando estiver investigando a execução, porque `message`, `error_class`, `retryable` e `suggestion_lang` são as quatro chaves que o envelope padrão descarta
- Um resumo que falha (`search`, `replace`, `transform`, `scope`, `batch`) é esse evento terminal, então um filtro `jaq` sobre `ok` basta
- Todo verbo que grava termina com `target_resolved` (o caminho absoluto em que agiu) e `target_source: "argv"`: compare com o caminho que você quis
- `--filter ok=false` não esconde uma falha, porque os erros sempre são emitidos

```bash
atomwrite -v --workspace "$WS" --locale en edit src/main.rs --old zzz_missing --new x \
  | jaq -c '{ok, code, exit, error_class, retryable, suggestion}'
atomwrite --workspace "$WS" edit src/main.rs --old "let x = 2;" --new "let x = 3;" --select ok,target_resolved,target_source
```


## Como Moldar um Stream Sem jq
- `--sort-by path-line` ordena os eventos, `--dedupe path` mantém o primeiro evento por path, `--aggregate type:count` conta eventos por tipo e `--truncate-content lines:60` corta um valor string em 60 caracteres
- `--max-output-bytes N` corta todo valor string, `checksum` incluído, e exige `--allow-truncate`: cada flag sozinha sai 2
- Use `--select` e `--limit` primeiro e recorra a estas só quando um pipeline precisar de mais

```bash
atomwrite --workspace "$WS" --sort-by path-line --select path,line_number search TODO src/
atomwrite --workspace "$WS" --dedupe path --select path search old src/
atomwrite --workspace "$WS" --aggregate type:count list src/ --depth 1
atomwrite --workspace "$WS" --truncate-content lines:60 --select path,lines search TODO src/
atomwrite --workspace "$WS" --max-output-bytes 12 --allow-truncate --select checksum hash src/lib.rs
atomwrite --workspace "$WS" --max-output-bytes 12 hash src/lib.rs     # exit 2, exige --allow-truncate
```


## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças relevantes para receitas em v0.1.12. A release v0.1.12 adiciona novas receitas ao cookbook para os 6 novos subcomandos, as novas flags, e o workflow de recuperação de crash.

### Novas Receitas (Adicionadas em v0.1.12)
- Como Editar um Arquivo de Config Sem Reescrevê-lo -- use `set`/`get`/`del` em vez de `read`+`edit` para caminhos dotted em TOML/JSON
- Como Renomear um Identificador em um Módulo -- use `case --subvert OLD NEW --to <style>` para 5 estilos de case
- Como Caminhar um AST de Código -- use `query --kinds` para listar kinds, `query --query KIND` para filtrar
- Como Extrair um Mapa de Código -- use `outline --positions` para funções/structs/enums/traits
- Como Detectar Erros de Sintaxe Antes do Commit -- use `write --syntax-check` para validação tree-sitter
- Como Recuperar de uma Escrita com Crash -- use `wal-stats` e `wal-heal` para inspecionar sidecars WAL
- Como Compor read com Ferramentas Unix -- use `read --format raw` para composabilidade Unix
- Como Limitar Search a Arquivos Pequenos -- use `search --max-filesize` e `--max-columns`
- Como Substituir Strings Literais -- use `replace --literal` para desabilitar regex
- Como Aplicar Refactor Multi-Rule -- use `transform --rules <file.yaml>` para regras em cascata
- Como Adquirir Lock Advisory de Arquivo -- nada a passar: desde a 0.1.37 toda escrita atômica toma o lock sozinha, espera até 5 s e sai com 83 `LOCK_TIMEOUT`
- Como Fazer Backup com CoW -- use `backup` e `copy` com reflink padrão em APFS/btrfs/XFS

### Receitas Atualizadas (mudanças v0.1.12 refletidas)
- Como Escrever um Arquivo Atomicamente -- agora menciona `--syntax-check` e o lock automático (`search --include-fifo` volta a abrir FIFOs, e `write` num FIFO sai 85)
- Como Editar um Arquivo -- agora menciona `--after-line`, `--before-line`, `--range`, `--delete-range`, `--between`
- Como Ler um Arquivo -- agora menciona `--format raw`, `--head N`, `--tail N`, `--line N`, `--grep`
- Como Buscar -- agora menciona `--max-filesize`, `--max-columns`
- Como Substituir Texto em Massa -- agora menciona `--literal`, `--fuzzy`
- Como Transformar Código -- agora menciona `--rules`, `--inline-rules`
- Como Operar em Batch -- agora menciona `--batch-size`, `--file`
- Como Fazer Backup -- agora menciona `--no-reflink`, `--output-dir`

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
- Lock advisory de escrita (G54), automático desde a 0.1.37, sem flag
- `search --max-filesize`, `--max-columns` (G68)
- `replace --literal` (G66)
- `transform --rules`, `--inline-rules` (G44)
- `batch --batch-size` (G77)
- `backup/copy --no-reflink` (G64)

### Cobertura de Testes (Histórico, v0.1.28)
- Histórico (v0.1.28): 661+ testes passando
- 9 ADRs em `docs/decisions/` (0019-0027)
- 7 novos JSON schemas em `docs/schemas/`
- Veja [docs/decisions/README.md](decisions/README.md) para decisões arquiteturais


## Nota de Latência
- Todas as operações executam localmente com overhead sub-milissegundo
- A sequência de escrita atômica adiciona ~1ms para o ciclo fsync-rename-fsync
- O paralelismo de busca escala com os cores de CPU disponíveis
- O modo batch amortiza o custo de startup entre N operações


## Referência de Valores Padrão
- `--threads` omitido ou `0` usa todas as CPUs lógicas até um teto embutido de 16, limitado pela RAM, e um N explícito vale mesmo acima de 16
- `--max-filesize` padrão é 1 GiB (1.073.741.824 bytes); no `search` o padrão é 10 MiB e os arquivos pulados entram em `files_skipped`
- `--color` padrão é `auto` (detecta terminal)
- `--workspace` padrão é o diretório de trabalho atual
- `diff --context` padrão é 3 linhas
- `diff --algorithm` padrão é `patience`
- Global `--timeout-secs` / `--timeout` → 120 (`0` desabilita; prazo exit 124)
- Máximo de applies fuzzy quando `--max-replacements` omitido → 1 (teto rígido 10_000); multi-apply é one-pass no conteúdo original; embeds forçam 1


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
atomwrite scope --query comments --delete -l rust src/main.rs
```

- Coloque em maiúsculas todos os nomes de função em Python:

```bash
atomwrite scope --query def --action upper -l python src/app.py
```

- Comprima espaços em branco em strings:

```bash
atomwrite scope --query strings --action squeeze -l rust src/lib.rs
```

- Toda chamada de `scope` precisa de `--language` (`-l`, alias `--lang`), e sem ele o verbo sai 2

- Substitua comentários por um cabeçalho padrão:

```bash
atomwrite scope --query comments --replace-with "// TODO: revisar" -l rust src/main.rs
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

- Mantenha no máximo 30 cópias de backup:

```bash
atomwrite backup --retention 30 src/config.toml
```

- Restaure o backup mais recente:

```bash
atomwrite rollback src/main.rs --latest
```

- Restaure um backup específico por timestamp:

```bash
atomwrite rollback src/main.rs --timestamp 20260529_120000
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
PATTERN=$(atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" | jaq -r '.regex')
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

- Visualize o batch sem executar (a prévia confere cada operação contra o disco como ele está agora, então uma operação que depende de outra do mesmo manifesto, como um edit num arquivo que o manifesto cria, é relatada como falha e a execução sai 65):

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
  jaq -r 'select(.type=="file") | .path' | \
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
- Leia um arquivo e capture o checksum (`extract checksum` imprime um objeto JSON, então use `jaq -r` para o valor puro):

```bash
CHECKSUM=$(atomwrite read --stat src/config.toml | jaq -r '.checksum')
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
  CHECKSUM=$(atomwrite read --stat src/config.toml | jaq -r '.checksum')
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

- Mantenha no máximo 30 backups do arquivo apagado:

```bash
atomwrite delete --backup --retention 30 src/old_module.rs
```

- Liste arquivos de backup em um diretório:

```bash
atomwrite list --long --all --include '*.bak.*' src/
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
<<<<<<< SEARCH
old_function_name
=======
new_function_name
>>>>>>> REPLACE
EOF
```


## Padrões Agent-First (v0.1.2+)

### Limite de Tempo em Busca Longa

```bash
# Padrão global é 120s (exit 124 no prazo); 0 desabilita
# Sobrescreva para 60s se a busca não terminar; emite erro NDJSON com error_class=transient
atomwrite --workspace . --timeout-secs 60 search 'TODO' src/
```

### Ler Apenas Linhas que Casam com Regex

```bash
# Útil para extrair logs de arquivos enormes sem esgotar o contexto
atomwrite --workspace . read --grep 'ERROR|WARN' logs/app.log
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
# Mantém o diretório de origem limpo; centraliza backups dentro do workspace
atomwrite --workspace . backup --output-dir backups src/critical.rs
```

### Instalar Completions de Shell no Primeiro Uso

```bash
# Auto-instala em ~/.local/share/bash-completion/completions/atomwrite
atomwrite completions bash --install
```

### Sempre passe `--workspace` para agentes

```bash
# Agentes devem passar --workspace explicitamente (sem variáveis de ambiente de produto); use um caminho absoluto
atomwrite --workspace /home/user/project read src/main.rs
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
atomwrite --locale en read /etc/passwd 2>/dev/null
# Saída: {"ok":false,"code":"WORKSPACE_JAIL","exit":126,...,"suggestion":"set --workspace <root> (XDG/config only; no environment variables)",...}

# Quando workspace É fornecido, a sugestão diz "use a path inside"
atomwrite --locale en --workspace /home/user/project read /etc/passwd 2>/dev/null
# Saída: {"ok":false,"code":"WORKSPACE_JAIL","exit":126,...,"suggestion":"use a path inside the workspace (/home/user/project); for --old-file/--new-file stage under /home/user/project/.atomwrite/scratch/ (never OS /tmp outside the jail)",...}
```


## Como Instalar no Windows 10/11 (v0.1.4)
- v0.1.4 finalmente corrige `cargo install atomwrite` no Windows
- Instale Visual Studio 2019+ Build Tools com workload C++
- Instale Rust 1.90+ via rustup
- Execute `cargo install atomwrite --locked`
- Veja [INSTALL.md](INSTALL.md) para o guia completo de troubleshooting Windows

```powershell
# PowerShell 7+ ou Windows Terminal
rustup default stable
rustup target add x86_64-pc-windows-msvc
cargo install atomwrite --locked
atomwrite --version  # texto puro, não NDJSON
```


## Como Descobrir o JSON Schema em Runtime
- Use `--print-schema NOME` para emitir o JSON Schema de uma saída, ou `--print-schema` sozinho para listar os 55 nomes, e `--json-schema SUBCOMANDO` é a grafia antiga e continua funcionando
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

- Confira a saída ao vivo contra as chaves que o schema exige:

```bash
# 1. Capturar o schema (qualquer nome de `atomwrite --print-schema`)
atomwrite --print-schema error > error.schema.json

# 2. Executar o comando real e conferir as chaves obrigatórias do envelope de erro
atomwrite -v --workspace . read /missing 2>/dev/null \
  | jaq -e 'has("error") and has("ok") and has("code") and has("exit") and has("error_class") and has("retryable")'
```


## Como Ler NDJSON em Pipeline Shell com jaq
- A saída dos subcomandos é NDJSON, salvo quando uma flag pede outra coisa, e `read --format raw`, `completions` e `--version` são as exceções
- Combine com `jaq` para filtragem estruturada, mapeamento e agregação

```bash
# Extrair apenas o checksum de uma resposta de read
atomwrite read src/main.rs | jaq -r '.checksum'

# Contar matches de search por arquivo
atomwrite search 'TODO' src/ | jaq -r 'select(.type=="match") | .path' | sort | uniq -c | sort -rn

# Ler os totais do batch
atomwrite batch < manifest.ndjson | jaq -c 'select(.type=="summary") | {operations, succeeded, failed}'

# Filtrar envelopes de erro por classe
atomwrite -v read /missing 2>/dev/null | jaq 'select(.error_class == "permanent")'
```


## Como Lidar com Erros Persistentes com Lógica de Retry
- Retente só quando o evento terminal disser `retryable: true` (exit 28, 30, 73, 74, 82, 83 e 92), e limite o número de tentativas
- Os verbos já espaçam as próprias tentativas com as chaves `retry.*` e `lock.acquire_timeout_ms`, então o loop não precisa de espera própria
- O loop completo, com os exit codes que pedem outra ação, está em `Como Ramificar pelo Exit Code e Retentar Só Quando for Retryable`

```bash
attempt=1
while [ "$attempt" -le 5 ]; do
  output=$(atomwrite --workspace . "$@" 2>/dev/null)
  exit_code=$?
  [ "$exit_code" -eq 0 ] && break
  [ "$(printf '%s' "$output" | jaq -r '.retryable // false')" = "true" ] || break
  attempt=$((attempt + 1))
done
printf '%s\n' "$output"
```


## Gerenciamento de Journals WAL (v0.1.15+)

O esforço G119 introduz três novas camadas para gerenciar os journals sidecar do WAL. Estas receitas mostram o uso prático dos novos subcomandos e flags.

### Como Reap de Journals Stale Antes do Commit

O subcomando `wal-heal` (G119 L3) remove com segurança journals terminais (Committed e Aborted). Integre-o a um hook de pre-commit para manter a working tree limpa.

```bash
# corpo do hook de pre-commit
atomwrite --workspace . wal-heal . --threshold-secs 0 \
  | jaq -e '.removed >= 0' \
  || { echo "wal-heal falhou"; exit 1; }
```

A flag `--threshold-secs 0` remove journals terminais de qualquer idade. `wal-heal` NUNCA toca em entradas Started. Para uma varredura mais conservadora, eleve o threshold para reter journals recentes para análise forense.

### Como Configurar Política de WAL por Carga de Trabalho

A flag `--wal-policy` (G119 L1) em `write` e `edit` controla quando o journal sidecar é escrito. Três valores são aceitos:

- `auto` (padrão) -- política escolhida pelo build, otimizada para uso geral
- `always` -- sempre escreve o journal sidecar (trilha de auditoria forense)
- `never` -- nunca escreve o journal sidecar (caminho mais rápido, sem metadados de recovery)

```bash
# Builds efêmeros: pula overhead de journal, sidecars não têm consumidor lá
atomwrite --workspace . write --wal-policy never ephemeral-config.toml < config.toml

# Deploys de produção: trilha de auditoria importa, força o sidecar
atomwrite --workspace . write --wal-policy always config/prod.toml < prod.toml

# Uso geral por agente: deixa o padrão decidir
atomwrite --workspace . write src/lib.rs < new_lib.rs
```

### Como Inspecionar Saúde dos Journals

O subcomando `wal-stats` (G119 L5) emite diagnósticos locais read-only sobre o estado atual do WAL. Combine-o com `jaq` para gatear scripts locais ou scripts pós-build.

```bash
# Diagnóstico local completo como NDJSON
atomwrite --workspace . wal-stats

# Gate em zero journals
atomwrite --workspace . wal-stats | jaq -e '.total_journals == 0' || { echo "drift"; exit 1; }

# Extrai apenas o tamanho recuperável para diagnóstico
atomwrite --workspace . wal-stats | jaq -r '.estimated_reclaim_bytes'
```


## Como Aplicar Múltiplos Pares de Edit Com Segurança (v0.1.15, G117)
- Pares repetidos `--old`/`--new` rodam a cascata fuzzy por par
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

Esta release introduz uma nova camada de segurança chamada intention guards e renomeia a flag global `--lang` para `--locale` para desambiguar do seletor tree-sitter `--lang` usado por `scope` e `transform`.

### Intention Guards (5 flags OPT-IN)
- `--require-backup` — não recebe valor: aborta quando `--backup` falta e o arquivo alvo existe
- `--confirm` — flag legada sem efeito sozinha, e sobrescritas grandes são guardadas por `--ack-overwrite`
- `--auto-rotate` — não recebe valor: força um backup de auto-rotação para arquivos modificados recentemente
- `--risk-threshold <PERCENT>` — percentual de variação de tamanho que levanta um aviso de risco (padrão desligado, `255`)
- `--locale <en|pt-BR>` — renomeado de `--lang` para desambiguar do `--lang` tree-sitter

### Outras Adições
- `count --by-size` — lista os maiores arquivos da árvore com tamanhos e contagem de linhas
- `read --format raw|ndjson` — seleciona entre saída byte-stream e envelope NDJSON estruturado
- `search --no-begin-end` — desabilita a decoração implícita de âncoras `^` e `$` na saída regex
- `write --preserve-timestamps` — preserva o mtime do arquivo fonte ao sobrescrever
- `scope --lang rust` — alias de `scope --language rust` (o `transform` aceita só `-l` ou `--language`)

### Estatísticas (Histórico, v0.1.20)
- Histórico (v0.1.20): 542 testes passando em 47 suites de integração, 0 falhas
- 11 GAP-2026 fechados
- 3 targets de cross-compile Windows verdes
- 19 ADRs em `docs/decisions/` (0019-0037)

### Migração `--lang` para `--locale`

```bash
# Descobrir todos os arquivos com --lang
rg -l -- '--lang\b' .

# Substituir em massa preservando outros matches
fd -e sh -e md -e toml -e yml -e yaml -e json -x sd -- '--lang\b' '--locale' {}

# Ou via ruplacer (prévia primeiro, acrescente --go para gravar)
ruplacer -- '--lang' '--locale' .
```


## v0.1.21 — Receitas

### Backup Deletado Após Sucesso
- Mudança de padrão — `write --backup` não deixa mais um sibling `.bak.<timestamp>` em disco após sucesso. O backup é deletado.
- Opt-in para preservar — passe `--keep-backup` para qualquer um de `write`, `edit`, `replace`, `rollback`, `apply`, `batch`:

```bash
# Default v0.1.21: backup é criado, escrita sucede, backup é deletado
echo "novo" | atomwrite --workspace . write --backup config.toml

# Opt-in v0.1.21: backup é criado, escrita sucede, backup é preservado
echo "novo" | atomwrite --workspace . write --backup --keep-backup config.toml
```

### Padrão de Edits Sequenciais com Re-captura de Checksum
- Padrão A — re-captura explícita é a forma canônica de encadear N chamadas `edit` no mesmo arquivo:

```bash
set -euo pipefail
for par in "foo:bar" "baz:qux" "alpha:beta"; do
  OLD="${par%:*}"
  NEW="${par#*:}"
  CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
  atomwrite --workspace . edit --old "$OLD" --new "$NEW" \
    --expect-checksum "$CS" src/foo.rs
done
```

- Padrão B — opt-in `--allow-sequential-drift` para scripts one-shot que preferem não re-capturar:

```bash
CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
for par in "foo:bar" "baz:qux"; do
  OLD="${par%:*}"; NEW="${par#*:}"
  atomwrite --workspace . edit --allow-sequential-drift \
    --old "$OLD" --new "$NEW" --expect-checksum "$CS" src/foo.rs
done
```


## v0.1.22 — Receitas

### `edit-loop` para N Pares em 1 Invocação
- Caso de uso: aplicar um lote de transformações textuais em um arquivo (renomear identificador em 7 lugares, varrer aliases obsoletos) onde hoje você invocaria `edit` N vezes em loop shell.

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

# Com --partial (best-effort: aplica matched, reporta unmatched, exit 87)
printf '%s\n' '{"old":"existe","new":"X"}' '{"old":"ausente","new":"Y"}' \
  | atomwrite --workspace . edit-loop --partial src/foo.rs

# Sem --partial um par que não casa sai 65 MATCH_FAILED e nada é gravado
printf '%s\n' '{"old":"existe","new":"X"}' '{"old":"ausente","new":"Y"}' \
  | atomwrite --workspace . edit-loop src/foo.rs
```

### `prune-backups` para Limpeza Manual de `.bak.*` Legados
- Caso de uso: operadores que atualizaram de v0.1.20 herdam siblings `.bak.<timestamp>` que v0.1.21 não cria mais (e portanto não limpa mais automaticamente).
- Alvos: passe os ARQUIVOS cujos backups você quer podar, e um diretório como argumento não casa com nada e ainda sai 0.

```bash
# --dry-run é uma chave sem valor: lista o que SERIA removido
atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run src/foo.rs

# Remove os backups do arquivo que são mais antigos que 24 horas (omita --dry-run para aplicar)
atomwrite --workspace . prune-backups --max-age-secs 86400 src/foo.rs

# Mantém apenas os 3 backups mais recentes do arquivo
atomwrite --workspace . prune-backups --max-count 3 src/foo.rs

# Local gate: afirma zero backups do arquivo após a limpeza
atomwrite --workspace . prune-backups --max-age-secs 0 src/foo.rs \
  && fd -g 'foo.rs.bak.*' src | wc -l | jaq -e '. == 0'
```


## v0.1.25 — Receitas

### Verificar Checksum de Arquivo

```bash
# Subcomando verify dedicado: o hash esperado é o segundo argumento posicional
atomwrite --workspace . verify src/main.rs abc123def456
# Exit 0 em match, exit 81 em mismatch
```

### Deletar Arquivos Antigos por Idade

```bash
# Deletar arquivos mais antigos que 7 dias (sufixos: s/m/h/d/w); um diretório exige --recursive
atomwrite --workspace . delete --recursive --older-than 7d --dry-run logs/
atomwrite --workspace . delete --recursive --older-than 7d logs/

# Visualizar plano de deleção com --plan
atomwrite --workspace . delete --recursive --plan --older-than 1h tmp/
```

- O run estreitado por `--older-than` sai 0 e publica `dirs_left` e `entries_left` enquanto mantém os arquivos mais novos, e o `delete -r` sem filtro remove entrada oculta e ignorada também
- O link dentro da árvore é removido como entrada e nunca seguido, e `--backup` com raiz de diretório é recusado antes de qualquer mudança

### Substituir com Preservação de Case

```bash
# Adapta case automaticamente: Hello→Goodbye, HELLO→GOODBYE, hello→goodbye
atomwrite --workspace . replace --preserve-case 'hello' 'goodbye' src/
```

### Configurar Threshold do Fuzzy

```bash
# Estrito: aceitar apenas matches próximos (0.99)
atomwrite --workspace . edit src/main.rs --old "texto_aproximado" --new "novo_texto" --fuzzy-threshold 0.99

# Solto: aceitar matches distantes (0.5)
atomwrite --workspace . edit src/main.rs --old "texto_aproximado" --new "novo_texto" --fuzzy-threshold 0.5 --fuzzy aggressive
```

### Scope Symbols e Normalize

```bash
# Converter operadores ASCII em strings para símbolos Unicode (=> para ⇒, != para ≠); --query ou --pattern é obrigatório
atomwrite --workspace . scope src/ --lang rust --query strings --action symbols --dry-run

# Aplicar normalização NFC Unicode (exit 1 quando o texto já está normalizado)
atomwrite --workspace . scope src/ --lang rust --query strings --action normalize --dry-run
```

### Usar .atomwrite.toml

```bash
# Criar config de projeto (opcional, flags CLI têm precedência)
cat > .atomwrite.toml << 'EOF'
[defaults]
backup = true

[fuzzy]
mode = "auto"
threshold = 0.85
EOF

# atomwrite carrega .atomwrite.toml automaticamente da raiz do projeto
atomwrite --workspace . edit src/main.rs --old "foo" --new "bar"

# Ou deixe o verbo config gravar as mesmas chaves para você (comentários são mantidos)
atomwrite --workspace . config set fuzzy.threshold 0.85 --scope workspace
```

### Hash Usa Campo checksum

```bash
# v0.1.25: campo é "checksum" (era "value" na v0.1.24)
atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'
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
  4) echo "arquivo não encontrado" ;;
  65) echo "$output" | jaq -r '.message, .suggestion' ;;   # inclui chave ausente
  *) echo "inesperado: exit $exit_code" ;;
esac
```

### Delete Recursivo (Agora Funciona)

```bash
# v0.1.24: delete --recursive agora percorre e remove
atomwrite --workspace . delete --recursive logs/

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
# Saída: {"type":"get","value":"0.1.24",...}
# NÃO: {"type":"get","value":"\"0.1.24\"",...}  (comportamento antigo)
```


## Como Sobrescrever Arquivo Grande com Segurança (v0.1.35)
- Problema: alvo existente acima de XDG `[write].confirm_large_bytes` (padrão 100 KiB)
- Solução: passe `--ack-overwrite` (one-shot; sem Y/N)

```bash
printf 'payload' | atomwrite --workspace . write --ack-overwrite big.txt
```


## Como Pré-visualizar Deletes Sem Mutar (v0.1.35)
- Problema: precisa de plano sem apagar
- Solução: `delete --plan` (não `--confirm` / `--yes`)

```bash
atomwrite --workspace . delete --recursive --plan --older-than 7d logs/
atomwrite --workspace . delete --recursive --older-than 7d logs/
```


## Como Consumir Saída Idle do Watch (v0.1.35)
- Problema: agentes assumiam stdout vazio sem eventos de FS
- Solução: sempre parsear `type:watch_summary` final

```bash
atomwrite --workspace . watch . --idle-exit-ms 500 --max-events 20
# Exige build com --features watch; o build padrão sai 78
```


## Como Fazer Merge Com Markers de Conflito (v0.1.35)

```bash
# b, o e t são arquivos dentro do workspace; um conflito deixa markers e a execução sai 0
atomwrite --workspace . semantic-merge --base b --ours o --theirs t --output out.txt
atomwrite --workspace . semantic-merge --base b --ours o --theirs t --output out.txt --no-conflict-markers
# Exit 84 MERGE_CONFLICT quando há conflito
atomwrite --workspace . semantic-merge --base b --ours o --theirs t --output out.txt --fail-on-conflict
```


## Toda flag local, por comando
- Esta lista nomeia toda flag local de cada comando, e as flags globais ficam em `atomwrite --help`.
- `agent-surface`: `--format`
- `apply`: `--ack-overwrite`, `--all`, `--allow-shrink`, `--backup`, `--dry-run`, `--format`, `--keep-backup`, `--no-backup`, `--no-syntax-check`, `--retention`, `--show`, `--strict-atomic`
- `backup`: `--dry-run`, `--output-dir`, `--retention`
- `batch`: `--backup`, `--batch-size`, `--dry-run`, `--file`, `--input-schema`, `--keep-backup`, `--no-backup`, `--retention`, `--strict-atomic`, `--transaction`
- `calc`: `--stdin`
- `case`: `--backup`, `--dry-run`, `--exclude`, `--include`, `--keep-backup`, `--no-backup`, `--preserve-timestamps`, `--retention`, `--strict-atomic`, `--subvert`, `--to`
- `codemod`: `--backup`, `--dry-run`, `--keep-backup`, `--no-backup`, `--retention`, `--rules`, `--strict-atomic`
- `commands`: `--brief`, `--include-globals`
- `completions`: `--install`
- `config`: não tem flag local própria, só o conjunto global
- `copy`: `--backup`, `--dry-run`, `--force`, `--keep-backup`, `--no-backup`, `--no-reflink`, `--preserve-xattr`, `--preserve`, `--recursive`, `--retention`, `--strict-atomic`
- `count`: `--by-extension`, `--by-size`, `--exclude`, `--include`, `--top`
- `del`: `--backup`, `--dry-run`, `--force-missing`, `--keep-backup`, `--no-backup`, `--preserve-timestamps`, `--retention`, `--strict-atomic`
- `delete`: `--backup`, `--confirm`, `--dry-run`, `--exclude`, `--include`, `--keep-backup`, `--no-backup`, `--older-than`, `--plan`, `--recursive`, `--retention`, `--yes`
- `diff`: `--algorithm`, `--context`, `--stat`, `--unified`
- `doctor`: `--strict`
- `edit`: `--after-line`, `--after-match`, `--allow-sequential-drift`, `--backup`, `--before-line`, `--before-match`, `--between`, `--delete-range`, `--dry-run`, `--expect-checksum`, `--fuzzy-threshold`, `--fuzzy`, `--keep-backup`, `--line-ending`, `--multi`, `--new-file`, `--new`, `--no-backup`, `--old-file`, `--old`, `--partial`, `--preserve-timestamps`, `--range`, `--replace-all`, `--retention`, `--strict-atomic`, `--wal-policy`
- `edit-loop`: `--allow-sequential-drift`, `--backup`, `--keep-backup`, `--line-ending`, `--no-backup`, `--partial`, `--retention`, `--strict-atomic`, `--syntax-check`
- `extract`: `--delimiter`, `--field`, `--stdin`
- `extract-artifact`: `--max-sample-rows`, `--max-text-chars`
- `get`: não tem flag local própria, só o conjunto global
- `hash`: `--exclude`, `--recursive`, `--stdin`, `--verify`
- `list`: `--all`, `--count-by-ext`, `--depth`, `--exclude`, `--format`, `--include`, `--long`
- `locale`: `--clear`, `--set`
- `move`: `--backup`, `--dry-run`, `--force`, `--keep-backup`, `--no-backup`, `--preserve-hardlinks`, `--retention`
- `outline`: `--format`, `--kind`, `--language`, `--positions`
- `prune-backups`: `--dry-run`, `--max-age-secs`, `--max-count`, `--recursive`
- `query`: `--kinds`, `--language`, `--positions`, `--query`, `--tree`
- `read`: `--all`, `--bytes`, `--context`, `--crop`, `--dpi`, `--enclosing`, `--force`, `--format`, `--grep`, `--head`, `--high-res`, `--line`, `--lines`, `--max-tokens`, `--no-comments`, `--number`, `--ocr`, `--page`, `--pages`, `--stat`, `--symbol`, `--tail`, `--verify-checksum`
- `recipe`: não tem flag local própria, só o conjunto global
- `regex`: `--case-insensitive`, `--digits`, `--no-anchors`, `--repetitions`, `--spaces`, `--stdin`, `--words`
- `replace`: `--backup`, `--dry-run`, `--exclude`, `--expect-checksum`, `--fuzzy-threshold`, `--fuzzy`, `--include`, `--keep-backup`, `--literal`, `--max-replacements`, `--no-backup`, `--preserve-case`, `--preserve-timestamps`, `--preview`, `--progress-every`, `--regex`, `--retention`, `--strict-atomic`, `--word`
- `rollback`: `--backup`, `--dry-run`, `--keep-backup`, `--latest`, `--no-backup`, `--retention`, `--strict-atomic`, `--timestamp`, `--verify`
- `scope`: `--action`, `--backup`, `--delete`, `--dry-run`, `--exclude`, `--include`, `--keep-backup`, `--language`, `--no-backup`, `--pattern`, `--query`, `--replace-with`, `--retention`, `--strict-atomic`
- `scratch`: não tem flag local própria, só o conjunto global
- `search`: `--binary`, `--case-insensitive`, `--context`, `--count`, `--exclude`, `--files`, `--fixed`, `--format`, `--include-fifo`, `--include`, `--invert`, `--limit`, `--max-columns`, `--max-count`, `--max-filesize`, `--multiline`, `--no-begin-end`, `--offset`, `--pcre2`, `--regex`, `--smart-case`, `--sort`, `--target`, `--word`
- `semantic-merge`: `--backup`, `--base`, `--dry-run`, `--expect-checksum`, `--fail-on-conflict`, `--keep-backup`, `--no-backup`, `--no-conflict-markers`, `--ours`, `--output`, `--retention`, `--strict-atomic`, `--theirs`, `--write-conflict-markers`
- `semantic-search`: `--include-backups`, `--index-dir`, `--k`, `--min-score`
- `set`: `--backup`, `--dry-run`, `--keep-backup`, `--no-backup`, `--preserve-timestamps`, `--retention`, `--strict-atomic`
- `sparse`: não tem flag local própria, só o conjunto global
- `stat`: `--all`, `--bytes`, `--context`, `--crop`, `--dpi`, `--enclosing`, `--force`, `--format`, `--grep`, `--head`, `--high-res`, `--line`, `--lines`, `--max-tokens`, `--no-comments`, `--number`, `--ocr`, `--page`, `--pages`, `--stat`, `--symbol`, `--tail`, `--verify-checksum`
- `transform`: `--backup`, `--dry-run`, `--exclude`, `--include`, `--inline-rules`, `--keep-backup`, `--language`, `--no-backup`, `--pattern`, `--retention`, `--rewrite`, `--rules`, `--strict-atomic`, `--verify-parse`
- `verify`: não tem flag local própria, só o conjunto global
- `wal-heal`: `--dry-run`, `--max-duration-ms`, `--threshold-secs`
- `wal-stats`: `--dry-run`
- `watch`: `--checksum`, `--debounce-ms`, `--gitignore`, `--idle-exit-ms`, `--max-events`
- `write`: `--ack-overwrite`, `--allow-empty-stdin`, `--allow-shrink`, `--append`, `--auto-rotate`, `--backup`, `--confirm`, `--dry-run`, `--durability`, `--expect-checksum`, `--keep-backup`, `--line-ending`, `--max-size`, `--no-backup`, `--no-checksum-when-empty`, `--prepend`, `--preserve-timestamps`, `--require-backup`, `--require-large-ack`, `--retention`, `--risk-threshold`, `--strict-atomic`, `--syntax-check`, `--wal-policy`
