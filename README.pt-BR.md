[Read in English](README.md)


# atomwrite


- Operações atômicas de arquivo para agentes LLM: um CLI, escritas tudo ou nada

[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)
[![Crates.io](https://img.shields.io/crates/v/atomwrite)](https://crates.io/crates/atomwrite)
[![License](https://img.shields.io/crates/l/atomwrite)](https://crates.io/crates/atomwrite)
[![MSRV](https://img.shields.io/badge/MSRV-1.90-orange)](https://github.com/danilo-aguiar-br/atomwrite/blob/main/Cargo.toml)
[![Downloads](https://img.shields.io/crates/d/atomwrite)](https://crates.io/crates/atomwrite)
[![Rust](https://img.shields.io/badge/rust-1.90%2B-blue)](https://www.rust-lang.org)

- Resumo legível por agentes: [llms.pt-BR.txt](llms.pt-BR.txt) e [llms-full.pt-BR.txt](llms-full.pt-BR.txt)
- Atalhos: [Instalação](#instalação), [Início Rápido](#início-rápido), [Flags Globais](#flags-globais), [Comandos](#comandos-47-no-total--ssot), [Configuração](#configuração), [Códigos de Saída](#códigos-de-saída)


## O Que É
- Um único binário Rust que resolve toda operação de arquivo que um agente LLM precisa
- 47 subcomandos (SSOT: clap `Commands` / `agent-surface`), todos listados com exemplo em [Comandos](#comandos-47-no-total--ssot)
- Toda escrita é atômica: tempfile, fsync, rename, fsync do diretório
- Toda resposta é NDJSON: um objeto JSON por linha, legível por máquina
- Todo arquivo recebe checksum BLAKE3: detecta drift, verifica integridade, habilita locking otimista


## Instalação
### Do crates.io
```bash
cargo install atomwrite
```

### Builds enxuto e completo
- Features: `core`, `ast`, `lang-rust`, `lang-ts`, `lang-py`, `lang-go`, `lang-full`, `watch`, `full` (default = `core` + `ast` + `lang-rust` + `lang-ts` + `lang-py`; `watch` é opt-in; `semantic-search` é sempre compilado)
- Build enxuto (somente core, 10.908.040 bytes medidos em 2026-10-31, DERIVADO: 10908040 / 1048576 = 10,40 MiB, size-gate local até 15 MiB): `cargo install --path . --locked --force --no-default-features --features core`
- Build completo/default (linguagens AST, 54,4 MiB medidos no build release da v0.1.37 desta árvore): `cargo install --path . --locked --force`
- Build com `watch`: `cargo install --path . --locked --force --features watch` (o binário padrão sai 78 em `watch` com uma dica de rebuild)
- Build enxuto: `outline` e `sparse outline` saem 78 com dica de rebuild (medido), e `query`, `scope`, `transform` e `write --syntax-check` têm stubs para o mesmo caso (lido em `src/commands/*_stub.rs` e `src/syntax_check_stub.rs`, o build enxuto não foi executado para confirmar)
- `cargo xtask check-features` confere os conjuntos `core` e `core,watch`
- As features `i18n-cjk`, `i18n-rtl`, `i18n-europe`, `i18n-full` e `slow-tests` são placeholders ou chaves de teste: não as ative
- A meta de 5-8 MB do plano do produto vale para o perfil core, não para o build default com AST: o core medido (10,40 MiB) fica acima dessa meta e abaixo do gate de 15 MiB

### Do código-fonte
```bash
git clone https://github.com/danilo-aguiar-br/atomwrite.git
cd atomwrite
cargo build --release
```

### Autocompletar do shell
```bash
# Bash
atomwrite completions bash > ~/.local/share/bash-completion/completions/atomwrite

# Zsh
atomwrite completions zsh > ~/.zfunc/_atomwrite

# Fish
atomwrite completions fish > ~/.config/fish/completions/atomwrite.fish
```


## Início Rápido
```bash
# Escrever um arquivo atomicamente via stdin
echo "hello world" | atomwrite --workspace . write src/hello.txt

# Ler de volta com checksum
atomwrite --workspace . read src/hello.txt

# Buscar através de um diretório
atomwrite --workspace . search 'hello' src/

# Substituir texto com escritas atômicas
atomwrite --workspace . replace 'hello' 'world' src/

# Avaliar matemática e conversões de unidades
atomwrite calc "2 hours + 30 minutes to seconds"

# Definir um valor em um arquivo TOML (preserva comentários)
atomwrite --workspace . set Cargo.toml package.version 0.2.0

# Caminhar o AST de um arquivo Rust (baixa o parser no primeiro uso, veja Rede e Privacidade)
atomwrite --workspace . query src/main.rs --kinds

# Extrair o outline de um arquivo Python
atomwrite --workspace . outline src/app.py

# Verificação de sintaxe tree-sitter real antes de gravar (exit 88 em erro de sintaxe)
echo "fn broken(" | atomwrite --workspace . write --syntax-check src/x.rs

# Relatar sidecars WAL obsoletos e estimar o reclaim
atomwrite --workspace . wal-stats

# Colher journals terminais mais antigos que o threshold
atomwrite --workspace . wal-heal . --threshold-secs 3600
```


## Expansão do read na v0.1.37 (2026-10-31)
- O `read` aceita um ou mais caminhos: caminho explícito sai na ordem do argv, e operando com `*`, `?` ou `[` é padrão que o próprio binário expande contra um nível do diretório dele
- A ordem do padrão é a do shell com `LC_ALL=C`, tirada dos bytes codificados de cada nome, então o mesmo código ordena nos três alvos, e só o Linux foi executado, enquanto macOS e Windows seguem não validados
- `--format raw` concatena todo alvo sem cabeçalho, sem separador e sem `\n` próprio, então `atomwrite read --format raw '*.rs' | cmp - <(LC_ALL=C cat *.rs)` sai 0 com diff vazio
- O lote copia em blocos de 64 KiB, então a memória fica plana em vez de somar todos os arquivos, e o `--timeout-secs` ainda corta a cópia com exit 124 porque cada bloco avança a checagem de cancelamento
- Alvo que falha imprime uma linha em stderr e o run segue para o próximo, e o código final é o semântico da primeira falha: 4 para caminho ausente, 13 para permissão, 126 fora do `--workspace`
- Erro de `read --format raw` agora vai para stderr e nunca para stdout, porque aquele canal promete bytes e envelope JSON ali era o defeito
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` e `--verify-checksum` exigem alvo único: nomear vários sai 65 com mensagem que nomeia a flag
- `**` está fora: padrão com `**` ou com mais de um separador sai 65 em vez de casar outra coisa em silêncio
- O formato NDJSON padrão não muda, e um alvo em `--format raw` sai byte a byte igual ao 0.1.37
- `[profile.dev] opt-level = 1` cortou o custo do fuzzy no perfil dev de 18533 ms para 4232 ms no mesmo acervo de 15 MB
- Pin `^0.1.37`. 47 subcomandos (SSOT)


## Novidades na v0.1.37 (2026-10-31)
- A avaliação de risco agora roda em todo verbo que grava: `edit`, `edit-loop`, `replace`, `case`, `transform` e `codemod` publicam `risk_assessment` quando o conteúdo casa um padrão de risco, exatamente como o `write` já fazia. As barreiras de tamanho continuam fora desses verbos de propósito, porque eles reescrevem uma região e um resultado menor é a operação funcionando
- Gramática que esta máquina não consegue fornecer é exit 69 `GRAMMAR_UNAVAILABLE` com classe `precondition_failed`, e nunca 255 `INTERNAL_ERROR`. O `atomwrite doctor` publica o check novo `grammar_cache` com quantas das 23 gramáticas que os verbos de AST podem precisar já estão em cache
- `write.content_risk_patterns` é acessível por `config get` e `config set` como valor de lista, então os padrões de risco são configuração XDG em vez de literal compilado (37 chaves)
- `log.file` e `log.rotation` passam a ser aceitos também no escopo workspace, e a seção `[log]` do `.atomwrite.toml` agora liga o destino de log em vez de ser validada e descartada
- `batch --transaction` publica um veredito só: o evento `rollback` carrega `ok: false` com o `code` e o `exit` da falha que o causou, e a transação trava todos os alvos em ordem de caminho antes do snapshot
- O `--version` não acrescenta mais `-dirty`, então um arquivo não rastreado não consegue mudar a string


## Novidades na v0.1.37 (rascunho de 2026-10-15)
- O histórico completo está no [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md), os passos de atualização estão em [docs/MIGRATION.pt-BR.md](docs/MIGRATION.pt-BR.md)
- Verbo novo `config` (o 47º subcomando): `config path|list|get|set|unset|doctor` lê e grava 37 chaves persistentes de ajuste, veja [Configuração](#configuração)
- Verbo novo `extract-artifact`: sinal delimitado de cabeçalhos de PDF, OOXML e imagem, nunca os bytes brutos
- Shape de stream global: `--sort-by`, `--dedupe`, `--aggregate`, `--join`, `--diff-against`, `--redact`, `--normalize-encoding`, `--truncate-content`, `--max-output-bytes`, veja [Flags Globais](#flags-globais)
- O evento terminal de sucesso carrega `ok: true` (os eventos por item de um stream, como `entry` e `match`, não carregam), e uma execução com falha termina em exatamente um evento terminal com `ok: false`
- Verbos que gravam recebem caminho explícito (exit 2 sem ele), e o evento terminal traz `target_resolved` e `target_source: "argv"`
- Todo evento terminal também traz `workspace_source`: `argv` quando a raiz veio do `--workspace`, e `cwd` quando veio do diretório de trabalho do processo
- `edit-loop` é tudo-ou-nada: par sem match sai com 65 e não grava nada, enquanto `--partial` grava os que casam e sai com 87; o checksum que ele lê sob o lock é a base da escrita, então edição concorrente sai 82 `STATE_DRIFT`
- Escrita atômica toma um lock advisory automático (espera até 5 s, exit 83), e `search`, `write`, `edit` e os demais verbos de stdin obedecem SIGTERM, SIGHUP e o prazo (exit 143, 129, 124)
- O `watch` saiu do conjunto padrão de features, e `--version` imprime `atomwrite 0.1.37 (<sha>) <target triple>` dentro de um checkout rastreado e `atomwrite 0.1.37 (no-git) <target triple>` fora dele
- Mudanças que podem quebrar um consumidor:
  - campos JSON de topo cujo valor era `null` deixaram de sair: teste a chave ausente, não `null`
  - `--max-output-bytes` agora exige `--allow-truncate`, e cada flag sozinha sai com 2
  - `semantic-merge` com conflito sai com 84 (`MERGE_CONFLICT`), e `apply` com diff unificado que não casa com o arquivo sai com 65 e não grava nada
  - regex ou glob inválido sai com 65, não 255, e `get`, `set` e `del` em arquivo TOML ou JSON inválido saem com 65
  - `case`, `transform`, `scope` e `codemod` podem sair com 82 (`STATE_DRIFT`) quando um arquivo mudou durante a execução
  - `sparse list` aplica `--include` e `--exclude`, e `search.max_inflight_bytes` começa em 4 MiB
  - a API Rust da biblioteca mudou, veja [API da Biblioteca](#api-da-biblioteca)
- Pin `^0.1.37`. 47 subcomandos (SSOT). Só CLI + XDG (sem knobs `ATOMWRITE_*`; sem telemetria). Erros sempre emitem, com duas exceções deliberadas: um pipe quebrado e uma divergência de checksum já publicada como um evento `hash_verify`


## Por Quê
- Agentes LLM gerenciam dezenas de comandos shell para manipular arquivos
- Um crash no meio de uma escrita pode corromper um arquivo gravado no próprio lugar
- Parsear saída não estruturada de CLI desperdiça tokens e causa alucinações
- Agentes precisam de checksums para detectar edições concorrentes mas raramente os computam
- atomwrite resolve todos os quatro problemas com um único `cargo install`


## Superpoderes
### Escritas Atômicas
- Usa tempfile + fsync + rename + fsync do diretório em cada escrita
- Um leitor vê o arquivo antigo ou o arquivo novo, nunca um arquivo meio escrito
- A durabilidade é definida por `write --durability full|fast|auto` (padrão `auto`); uma queda de energia não foi simulada nos testes
- Verificação de sintaxe real opcional via tree-sitter (flag `--syntax-check` em `write`, exit 88)
- Sidecar WAL opcional para recuperação de crash (`.atomwrite.journal.<target>.atomwrite.journal.json`)

### Saída NDJSON
- stdout é SEMPRE JSON estruturado, um objeto por linha
- Cada objeto carrega um campo discriminador `"type"`
- Agentes parseiam saída sem regex ou scraping frágil de texto
- Erros também emitem JSON com `error: true` e `ok: false` no stdout

### Checksums BLAKE3
- Cada resposta de `read` e `write` inclui um hash BLAKE3
- Use `--expect-checksum` para locking otimista em edições concorrentes
- Detecte state drift antes de aplicar mudanças

### Busca Paralela
- Construída sobre o engine ripgrep para busca em conteúdo de arquivos
- Respeita `.gitignore` automaticamente
- Retorna matches estruturados com arquivo, linha, coluna e contexto

### Transformações AST-Aware
- `transform` e `codemod` reescrevem pela árvore sintática via ast-grep 0.43, que suporta 28 linguagens (medido: cada um dos 28 nomes é aceito, `zig` e `erlang` saem 65 `unsupported language`)
- `query`, `outline` e `--syntax-check` usam `tree-sitter-language-pack` 1.22.1, cujo manifesto lista 372 linguagens (medido em 2026-10-31 no manifesto dessa versão)
- A detecção automática de linguagem por extensão de arquivo cobre 23 linguagens (`src/syntax_check.rs`); `--language NAME` aceita um parser que já está no cache local
- Refatore código pela árvore sintática, não regex frágil

### Scoping Gramatical
- Selecione categorias AST como comentários, funções, classes e strings
- Aplique ações: delete, uppercase, lowercase, titlecase, squeeze ou replace
- Cobre Rust, Python, JavaScript, TypeScript e Go com queries preparadas
- Use `--pattern` para padrões AST customizados além das queries built-in

### Operações em Lote
- Execute operações write, replace, delete, edit, hash, move e copy a partir de um manifesto NDJSON
- Use `--transaction` para execução all-or-nothing com rollback automático
- Todas as operações em um lote compartilham as mesmas garantias atômicas

### Edição Estruturada de Config
- `set` escreve um valor em um dotted path em arquivos TOML ou JSON (preserva comentários via `toml_edit`)
- `get` lê um valor em um dotted path com formato auto-detectado
- `del` remove uma chave (com `--force-missing` para tratar chaves ausentes como sucesso no-op)
- `case` renomeia identificadores dentro de arquivos via `heck` (snake, camel, pascal, kebab, screaming-snake)


## Uso
- Toda saída vai para stdout como NDJSON
- Os logs vão para stderr: avisos e erros sempre, info e debug com `-v` / `-vv`
- Sempre passe `--workspace <DIR>` para restringir operações à raiz do projeto
- Use `--dry-run` antes de operações destrutivas
- Use `--expect-checksum <HASH>` para locking otimista
- Use `--locale <TAG>` ou `atomwrite locale --set` / `locale --clear` para locale de UI (`en`, `pt-BR`); códigos NDJSON ficam em inglês
- A configuração é somente flags CLI + XDG / `.atomwrite.toml`, veja [Configuração](#configuração)
- Pipe stdin para `write` e `batch`
- Globais agent-native de stdout (todos os subcomandos): `--select` / `--fields` (chaves de topo CSV), `--filter KEY=VAL` (AND repetível), `--limit N` (somente long: não existe `-n` global, e `replace` possui o short `-n`)
- NDJSON de erro sempre tem `ok: false` e geralmente `match_count`: parseie `exit` / `code` primeiro; o shaping de sucesso via `--filter`/`--limit` nunca descarta envelopes de erro


## Flags Globais
- 33 flags são aceitas antes ou depois do subcomando (fonte: `atomwrite commands --include-globals`); um `--json` oculto também é aceito e ignorado, porque a saída é sempre NDJSON
- Aliases: `--max-concurrency` para `-j` / `--threads`, `--timeout` para `--timeout-secs`, `--fields` para `--select`
- Flags com nome igual a um global mas locais a um comando (por exemplo `search --max-filesize`, 10 MiB por padrão) aparecem em `atomwrite <comando> --help`

| Flag | Padrão | O que faz |
|---|---|---|
| `-v`, `--verbose` | desligado | aumenta a verbosidade do log no stderr (`-v` info, `-vv` debug, `-vvv` trace) |
| `-q`, `--quiet` | desligado | diminui (`-q` erro, `-qq` desligado) |
| `--workspace <DIR>` | diretório atual | raiz do jail de caminhos, passe sempre |
| `--config <PATH>` | nenhum | `.atomwrite.toml` explícito, falha se ausente ou malformado; substitui os dois arquivos em `config get` e `config list` |
| `--color <WHEN>` | `auto` | `auto`, `always` ou `never` |
| `--no-color` | desligado | desliga a saída colorida |
| `--ready-file <PATH>` | nenhum | grava o PID depois que os handlers de sinal estão instalados (só harness de teste) |
| `--no-gitignore` | desligado | não respeita arquivos `.gitignore` |
| `--hidden` | desligado | inclui arquivos e diretórios ocultos |
| `--follow-symlinks` | desligado | segue links simbólicos |
| `-j`, `--threads <N>` | automático | limite de workers; omitido ou `0` = o menor entre núcleos lógicos, 16 e o teto de RAM; um `N` explícito vale, também acima de 16 |
| `--max-filesize <BYTES>` | 1 GiB | rejeita arquivo maior (`read`, `edit`, `hash` e `write` saem 65 `FILE_TOO_LARGE`), e no `write` ele também limita o payload do stdin, então a flag nomeia um teto só e o payload e o alvo obedecem ao mesmo; só o `search` pula em silêncio, com limite local próprio de 10 MiB contado em `files_skipped` |
| `--timeout-secs <N>` | 120 | prazo global, `0` desliga, exit 124 com um evento `cancelled` que tem `deadline_exceeded: true` |
| `--no-progress` | desligado | desliga eventos NDJSON de progresso em operações longas |
| `--json-schema` | desligado | emite o JSON Schema da saída do comando e sai |
| `--print-schema [NAME]` | nenhum | lista os 55 nomes de schema, ou emite um schema; nome desconhecido sai 65 |
| `--non-interactive` | desligado | declara execução hospedada por agente: nunca pergunta, nunca lê confirmação do stdin |
| `--confirm-side-effect` | desligado | reconhece um efeito colateral onde o contrato do comando pede; num verbo que não escreve ela é recusada com exit 65 `INVALID_INPUT` em vez de virar no-op silencioso |
| `--allow-truncate` | desligado | autoriza truncar valores, exigido por `--max-output-bytes` |
| `--locale <TAG>` | resolvido | `en` ou `pt-BR`; precedência: esta flag, a preferência salva, o idioma do SO, `en` |
| `--no-auto-heal` | desligado | pula o passe de `wal-heal` na partida (threshold de 3600 s, orçamento de 100 ms) |
| `--select <KEYS>` | todas as chaves | mantém só as chaves de topo listadas (CSV) |
| `--filter <KEY=VALUE>` | nenhum | mantém eventos cuja chave é igual ao valor (`!=` e `==` funcionam; repetível, AND); nunca descarta um envelope de erro |
| `--limit <N>` | nenhum | teto de trabalho: corta a lista antes da ação e para a varredura no registro que atinge o teto, então uma árvore enorme não é lida até o fim; e o resumo terminal traz `walker_stopped_early: true` só quando a varredura foi mesmo cortada antes do fim da árvore |
| `--sort-by <KEY>` | ordem de chegada | ordena eventos por `path-line`, `line-path` ou qualquer chave (acumula até o fim) |
| `--dedupe <KEYS>` | desligado | descarta eventos cuja tupla de chaves (CSV) já foi vista |
| `--aggregate <KEY[:OP[:FIELD]]>` | desligado | agrupa eventos acumulados por `KEY`; `OP` é `count`, `sum`, `min` ou `max` |
| `--join <KEY:SUFFIX>` | desligado | lê a tabela NDJSON do stdin uma vez (até 16 MiB) e anexa as linhas que casam como um array `SUFFIX` em todo evento de dados que tenha `KEY`; eventos sem `KEY` ou sem casamento, eventos de falha e o summary ficam como estavam; roda antes de `--select`, `--filter` e `--limit` (com `--sort-by`, `--dedupe` ou `--aggregate`: depois do sort, antes do dedupe); sai 65 `INVALID_INPUT` para stdin TTY, tabela acima de 16 MiB, linha que não é objeto JSON, formato inválido, `KEY` sem linha com valor string, ou comando que já lê o stdin (`write`, `edit`, `edit-loop`, `apply`, `extract`, `batch` sem `--file`, `hash --stdin`, `calc` ou `regex` lendo o stdin, `diff` com `-`) |
| `--truncate-content <KEY:LEN>` | desligado | corta uma chave string em `LEN` caracteres |
| `--redact` | desligado | troca valores com formato de segredo por `[redacted]` |
| `--normalize-encoding` | desligado | normaliza todo valor string para Unicode NFC |
| `--diff-against <PATH>` | desligado | compara os eventos com um arquivo NDJSON base dentro do jail e emite um evento `stream_diff` |
| `--max-output-bytes <N>` | desligado | corta todo valor string, `checksum` incluído; exige `--allow-truncate` e cada flag sozinha sai com 2 |

```bash
# Exemplos de shape de stream
atomwrite --workspace . --sort-by path-line search 'TODO' src/
atomwrite --workspace . --aggregate type:count list src/ --depth 1
atomwrite --workspace . --dedupe kind list src/ --depth 1
atomwrite --workspace . --truncate-content content:20 read src/main.rs
atomwrite --workspace . --redact read src/main.rs
atomwrite --workspace . --normalize-encoding read src/main.rs
atomwrite --workspace . list src/ --depth 1 > base.ndjson
atomwrite --workspace . --diff-against base.ndjson list src/ --depth 1
atomwrite --workspace . --max-output-bytes 16 --allow-truncate --select path,checksum read src/lib.rs
atomwrite --workspace . --non-interactive read src/lib.rs
printf '{"path":"src/lib.rs","owner":"alice"}\n' | atomwrite --workspace . --join path:meta list src/ --depth 1
```


## Comandos (47 no total — SSOT)

### I/O Central
- `read` — ler arquivos com metadados, checksum, conteúdo opcional (`--format raw` ou `ndjson`, `--head`, `--tail`, `--lines`)
- `write` — criar ou sobrescrever arquivos atomicamente via stdin (alvos existentes grandes exigem `--ack-overwrite`)
- `edit` — editar cirurgicamente por número de linha, marcador de texto ou match exato
- `edit-loop` — aplicar N pares `old`/`new` do NDJSON stdin em uma escrita atômica
- `delete` — deletar arquivos com backup opcional (`--plan` plan-only; `--confirm`/`--yes` rejeitados)
- `copy` — copiar arquivos com verificação de checksum
- `move` — mover ou renomear arquivos atomicamente (fallback copy em EXDEV)
- `apply` — aplicar patches do stdin (unified diff, search/replace, full, markdown)
- `scratch` — criar diretório de staging do agente sob o workspace (nunca OS `/tmp`)
- `verify` — verificar integridade do arquivo comparando checksum BLAKE3

### Buscar e substituir
- `search` — buscar conteúdo de arquivos em paralelo (engine ripgrep)
- `replace` — substituir texto entre arquivos com escritas atômicas
- `transform` — refatoração AST via ast-grep (28 linguagens)
- `codemod` — campanha de codemod multi-regra (YAML rules, delega a transform)
- `semantic-search` — busca offline por Jaccard de tokens (sem rede, sem API de embeddings)
- `semantic-merge` — merge de três vias baseado em linhas (não AST/embedding) para escritas multi-agente

### Inspeção
- `hash` — calcular checksums BLAKE3
- `stat` — alias de metadados de `read --stat` (tamanho, mode, checksum; sem corpo)
- `count` — contar linhas, arquivos por extensão ou por tamanho
- `diff` — comparar dois arquivos (unified, stat ou changes)
- `list` — listar arquivos em uma árvore de diretório
- `sparse` — list/read/outline esparso com orçamento (limita arquivos e bytes para contexto do agente)
- `extract` — extrair campos de entrada NDJSON via pipe (campos posicionais ou `--field` local; nunca `extract --select` local: o `--select` global só projeta chaves de saída)
- `extract-artifact` — sinal delimitado de cabeçalhos de PDF, OOXML e imagem (nunca os bytes brutos do artefato)
- `scope` — scoping gramatical (deletar todos os comentários, etc.)
- `regex` — gerar regex a partir de exemplos
- `calc` — matemática e conversões de unidades
- `doctor` — diagnosticar ambiente e dependências
- `locale` — mostrar locale de UI resolvido / persistir preferência (XDG)
- `commands` — emitir árvore completa de comandos como JSON para descoberta do agente
- `config` — chaves persistentes de ajuste: `config path|list|get|set|unset|doctor`, 37 chaves, `--scope global|workspace` obrigatório em `set` e `unset` (e em `path`)
- `agent-surface` — manifesto da superfície de ferramentas do agente (integração por subprocesso, CLI com saída NDJSON)
- `watch` — observar eventos de filesystem (exige build com `--features watch`; o binário padrão sai 78)
- `completions` — gerar completions de shell (bash, zsh, fish, elvish, powershell)

### Backup e recuperação
- `backup` — criar backups com timestamp e checksums BLAKE3
- `rollback` — restaurar a partir de um backup anterior
- `prune-backups` — podar backups `.bak.YYYYMMDD_HHMMSS` por idade ou contagem
- `batch` — operações em lote dirigidas por NDJSON (transacional)
- `recipe` — receitas nomeadas multi-passo (`list` / `run`)

### Editores de config estruturada
- `set <PATH> <KEY_PATH> <VALUE>` — escrever um valor em um dotted path em um arquivo TOML ou JSON, preservando comentários e ordem de chaves via `toml_edit`
- `get <PATH> <KEY_PATH>` — ler um valor em um dotted path, NDJSON `{"type":"get","key_path","value","found","format"}`
- `del <PATH> <KEY_PATH>` — remover uma chave, `--force-missing` trata chave ausente como sucesso no-op
- `case <FILES...> --subvert OLD NEW --to <style>` — renomear identificadores dentro dos FILES dados via `heck` (estilos: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`); um argumento diretório é expandido pelo walker compartilhado, honra `--include` e `--exclude`, e um diretório que não devolve nenhum arquivo sai 65 em vez de fingir sucesso

### Ferramentas AST (via tree-sitter-language-pack)
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` — caminhar um AST tree-sitter e emitir nós como NDJSON
- `outline <PATH> [--kind <KIND>] [--positions]` — extrair estrutura de alto nível (funções, classes, structs, enums, traits, módulos) como NDJSON; exige um arquivo, não um diretório
- `write --syntax-check` — verificação de sintaxe real via tree-sitter, exit 88 com primeira linha, coluna, kind e mensagem de erro

### Limpeza de WAL
- `wal-stats` — snapshot de telemetria: total de journals, breakdown por estado, idade do mais antigo, tamanho total, breakdown por diretório, recomendação de auto-heal, bytes estimados de reclaim
- `wal-heal <PATH> [--threshold-secs N] [--max-duration-ms N]` — colher journals terminais obsoletos (committed ou aborted) mais antigos que o threshold, e também remover um journal `Started` cujo PID dono não existe mais, reportado como `dead_owner_removed`

### Padrões comuns para cada subcomando
```bash
# read
atomwrite --workspace . read src/main.rs
atomwrite --workspace . read --head 20 src/main.rs
atomwrite --workspace . read --format raw --lines 1:3 src/main.rs

# write
echo "fn main() {}" | atomwrite --workspace . write src/main.rs

# edit
atomwrite --workspace . edit src/main.rs --old "texto antigo" --new "texto novo"

# edit-loop (N pares old/new, uma escrita atômica)
printf '%s\n' '{"old":"foo","new":"bar"}' '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace . edit-loop src/version.rs

# scratch: stage sob .atomwrite/scratch/, nunca OS /tmp fora de --workspace, para --old-file/--new-file
atomwrite --workspace . scratch init .atomwrite/scratch
atomwrite --workspace . edit src/main.rs \
  --old-file .atomwrite/scratch/old.txt \
  --new-file .atomwrite/scratch/new.txt

# verify
atomwrite --workspace . verify src/main.rs abc123def456

# search
atomwrite --workspace . search 'TODO' src/ --include '*.rs'

# replace
atomwrite --workspace . replace 'nome_antigo' 'nome_novo' src/ --include '*.rs'

# hash
atomwrite --workspace . hash src/main.rs src/lib.rs

# delete
atomwrite --workspace . delete src/temp.rs --backup
atomwrite --workspace . delete --plan src/temp.rs

# count
atomwrite --workspace . count src/ --by-extension

# diff
atomwrite --workspace . diff src/old.rs src/new.rs --unified

# move
atomwrite --workspace . move src/old.rs src/new.rs

# copy
atomwrite --workspace . copy src/template.rs src/new_module.rs

# list
atomwrite --workspace . list src/ --depth 2

# extract: campos posicionais ou --field local; NUNCA extract --select local
atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line
atomwrite --workspace . search 'TODO' src/ | atomwrite extract --field path --field line
# shape agent-native (global --select só projeta chaves de saída):
atomwrite --workspace . search 'TODO' src/ --select path,line --limit 20

# extract-artifact (cabeçalho de PDF, OOXML ou imagem; aqui uma imagem)
atomwrite --workspace . extract-artifact logo.png

# calc
atomwrite calc "2 GiB to bytes"
atomwrite calc "sqrt(144) + 3^2"

# regex
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"

# transform
atomwrite --workspace . transform -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/

# scope
atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run

# batch
cat manifest.ndjson | atomwrite --workspace . batch --transaction

# backup
atomwrite --workspace . backup src/config.toml

# rollback
atomwrite --workspace . rollback src/config.toml

# prune-backups
atomwrite --workspace . prune-backups --max-age-secs 86400 src/config.toml
atomwrite --workspace . prune-backups --max-count 3 --dry-run src/config.toml

# apply
echo "novo conteúdo" | atomwrite --workspace . apply src/file.txt --format full

# set
atomwrite --workspace . set Cargo.toml package.version 0.2.0

# get
atomwrite --workspace . get Cargo.toml package.version

# del
atomwrite --workspace . del Cargo.toml package.metadata.boring

# case: um arquivo ou um diretório (o diretório é expandido pelo walker)
atomwrite --workspace . case src/main.rs --subvert user_id UserId --to pascal

# query
atomwrite --workspace . query src/main.rs --kinds
atomwrite --workspace . query src/main.rs -Q function_item --positions

# outline
atomwrite --workspace . outline src/app.py

# wal-stats
atomwrite --workspace . wal-stats

# wal-heal
atomwrite --workspace . wal-heal . --threshold-secs 3600 --max-duration-ms 100

# stat
atomwrite --workspace . stat src/lib.rs

# sparse
atomwrite --workspace . sparse list --max-files 100 --max-bytes 1048576 src/

# semantic-merge (baseado em linhas, não AST; exit 84 quando há conflitos)
atomwrite --workspace . semantic-merge \
  --base base.rs --ours ours.rs --theirs theirs.rs --output src/merged.rs

# recipe
atomwrite --workspace . recipe list
atomwrite --workspace . recipe run --name search-replace-verify \
  --path src --pattern unwrap --replacement expect --fuzzy auto --dry-run

# agent-surface
atomwrite --workspace . agent-surface

# watch (exige build com --features watch; o binário padrão sai 78)
atomwrite --workspace . watch . --debounce-ms 200 --max-events 20

# codemod
atomwrite --workspace . codemod --rules rules.yaml --dry-run src/

# semantic-search
atomwrite --workspace . semantic-search "fn main println" src/ --k 10

# doctor
atomwrite --workspace . doctor
atomwrite --workspace . doctor --strict

# locale
atomwrite locale
atomwrite locale --set pt-BR
atomwrite locale --clear

# commands (árvore clap completa para descoberta do agente)
atomwrite commands
atomwrite commands --include-globals

# completions
atomwrite completions bash

# config (chaves persistentes de ajuste, veja Configuração)
atomwrite --workspace . config list
atomwrite --workspace . config get fuzzy.threshold
atomwrite --workspace . config set fuzzy.threshold 0.9 --scope workspace
atomwrite --workspace . config unset fuzzy.threshold --scope workspace
```
- `codemod --rules` lê uma lista YAML de itens com `language`, `pattern`, `rewrite` e um `id` opcional; a forma estilo ast-grep com `rule:` e `fix:` sai 65
- `prune-backups --dry-run` não recebe valor: omita a flag para aplicar a poda


## Configuração
- Precedência: flags CLI, depois o `.atomwrite.toml` do workspace, depois o `config.toml` XDG global (os dois arquivos são mesclados chave a chave), depois os defaults embutidos
- `config path|list|get|set|unset|doctor` é a única forma de mudar uma chave persistente de ajuste
- `config set CHAVE VALOR --scope global|workspace` e `config unset CHAVE --scope global|workspace` exigem `--scope` (exit 2 sem ele, nada é tocado), e `config path` também o recebe
- `config list` e `config get CHAVE` não recebem `--scope`: mostram `value`, `origin` (`default`, `global`, `workspace` ou `explicit` com `--config PATH`) e `default`
- O escopo `global` é o `config.toml` XDG (modo 0600), o escopo `workspace` é o `.atomwrite.toml` na raiz do workspace
- O `set` preserva comentários, valida o arquivo inteiro e grava por tempfile-fsync-rename sem backup; o evento terminal carrega `target_resolved` e `target_source: "argv"`
- Chave desconhecida ou valor fora da faixa sai 65 e não grava nada; um arquivo que não é TOML válido nunca é reescrito (`set` e `unset` saem 78); o `config` funciona mesmo com um `config.toml` quebrado
- Uma flag ainda vence a configuração (por exemplo `extract-artifact --max-text-chars`)

| Chave | Tipo | Padrão | Aceito | Escopos |
|---|---|---|---|---|
| `fuzzy.mode` | string | `auto` | `auto`, `aggressive`, `off` | global, workspace |
| `fuzzy.threshold` | float | 0.7 | 0.0 a 1.0 | global, workspace |
| `fuzzy.threshold_aggressive` | float | 0.5 | 0.0 a 1.0 | global, workspace |
| `fuzzy.threshold_context` | float | 0.8 | 0.0 a 1.0 | global, workspace |
| `fuzzy.threshold_jw` | float | 0.85 | 0.0 a 1.0 | global, workspace |
| `fuzzy.soft_floor_delta` | float | 0.05 | 0.0 a 1.0 | global, workspace |
| `fuzzy.max_content_bytes` | integer | 268435456 (256 MiB) | 1 ou mais | global, workspace |
| `write.confirm_large_bytes` | integer | 102400 (100 KiB) | 1 ou mais | global, workspace |
| `log.file` | boolean | `false` | `true`, `false` | global, workspace |
| `log.rotation` | string | `daily` | `daily`, `hourly` | global, workspace |
| `storage.home` | string | nenhum | um diretório | só global |
| `locale` | string | nenhum (resolvido) | `en`, `pt-BR` | só global |
| `concurrency.max_threads` | integer | 16 | 1 a 256 | global, workspace |
| `concurrency.ram_fraction_percent` | integer | 50 | 1 a 100 | global, workspace |
| `search.max_inflight_bytes` | integer | 8388608 (8 MiB) | 4194304 (4 MiB) a 1073741824 (1 GiB) | global, workspace |
| `summary.errors_max_entries` | integer | 50 | 1 a 1000 | global, workspace |
| `lock.acquire_timeout_ms` | integer | 5000 | 100 a 3600000 | global, workspace |
| `retry.base_ms` | integer | 50 | 1 a 60000, sem passar de `retry.max_ms` | global, workspace |
| `retry.max_ms` | integer | 400 | 1 a 60000 | global, workspace |
| `retry.max_elapsed_ms` | integer | 2000 | 1 a 3600000 | global, workspace |
| `extract.max_sample_rows` | integer | 10 | 0 a 100000 | global, workspace |
| `extract.max_text_chars` | integer | 400 | 1 a 10000000 | global, workspace |
| `extract.max_sheets` | integer | 32 | 1 a 10000 | global, workspace |
| `extract.max_slides` | integer | 64 | 1 a 10000 | global, workspace |

- Um escopo fora da última coluna sai 65 (por exemplo `config set storage.home /srv --scope workspace`)
- Esses campos são chaves do `config` como qualquer linha acima, e cada um responde a `config get`: `defaults.backup`, `defaults.retention`, `defaults.line_ending`, `defaults.max_filesize`, `write.shrink_block_percent`, `write.auto_rotate_max_age_secs`, `write.content_risk_patterns`, `watch.idle_exit_ms`, `watch.debounce_ms`, `search.context`, `search.smart_case`, `search.max_filesize` e `search.max_columns`; uma seção desconhecida no `.atomwrite.toml` faz o `read` sair 78 (medido)
- Arquivos no Linux: config global `$XDG_CONFIG_HOME/atomwrite/config.toml`, locale salvo `$XDG_CONFIG_HOME/atomwrite/locale`, log `$XDG_STATE_HOME/atomwrite/logs/atomwrite.<data>.jsonl` (só com `log.file = true`, arquivo modo 0600, diretório 0700), parsers `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/`
- macOS e Windows resolvem os mesmos arquivos pelo crate `directories`; esses caminhos foram lidos do código e não executados lá
- `storage.home` move `config` e `state` para `{home}/config` e `{home}/state`

```bash
atomwrite --workspace . config get fuzzy.max_content_bytes
atomwrite --workspace . config set fuzzy.max_content_bytes 1000000 --scope global
```


## Variáveis de Ambiente
- Política de produto: o atomwrite não usa variáveis de ambiente de produto para knobs de runtime (workspace, backup, locale, filtro de log, home override, WAL, etc.)
- Configure via flags CLI e armazenamento XDG / `.atomwrite.toml` (e opcional `--config`)
- O `doctor` não lê nenhuma variável de ambiente: a detecção do host (WSL, contêiner, Kubernetes, Termux, Flatpak, Snap) usa só sondas de arquivo, e isso não é API de configuração
- Dependências leem algumas variáveis por conta própria, nunca como configuração do atomwrite: o crate de idioma do SO (`LANG`, `LC_ALL`, `LC_MESSAGES`, `LANGUAGE`), os crates XDG (`HOME`, `XDG_*`) e o cliente de download de parsers (`ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`); veja o ADR-0057 em [docs/decisions/](docs/decisions/README.md)
- Variáveis de shell usadas pelo harness do agente (PATH, etc.) ficam fora da config de produto do atomwrite


## Rede e Privacidade
- Sem telemetria: o atomwrite não envia nada sobre você nem sobre seus arquivos
- O único tráfego de rede é o download de parsers usado por `query`, `outline` e `write --syntax-check` (o último lido do código-fonte, não executado); `transform`, `scope` e `codemod` usam parsers do ast-grep compilados no binário
- No primeiro uso de uma linguagem, o binário baixa o pacote de parsers da plataforma de `https://github.com/xberg-io/tree-sitter-language-pack/releases/download/v1.22.1/` (no Linux x86_64 o arquivo `parsers-linux-x86_64.tar.zst`, 23031807 bytes no manifesto, cerca de 22 MiB, mais 3,9 MiB de bibliotecas de parser extraídas para as seis linguagens que esta máquina usou até agora); o manifesto traz um `sha256` para cada pacote de plataforma
- O cache fica em `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` (`bundles`, `libs`, `manifest.json`), e o download só acontece enquanto o parser necessário falta ali
- O cliente de download respeita as variáveis de proxy listadas em [Variáveis de Ambiente](#variáveis-de-ambiente)
- Uso offline: preencha o diretório de cache em um host com rede e copie-o para o host offline
- Um panic grava um arquivo de relatório de falha `report-<uuid>.toml` no diretório temporário do SO (pelo crate `human-panic`) e imprime o caminho; nada é transmitido, e você decide se envia
- Detalhes: [PRIVACY.pt-BR.md](PRIVACY.pt-BR.md)


## Códigos de Saída
- Envelopes de erro no stdout são sempre NDJSON com `ok: false` (e geralmente `match_count`); leia `exit` / `code` antes de tratar a linha como sucesso
- A lista abaixo é derivada do enum de erros e conferida contra `agent-surface` e `--help`
<!-- exit-codes:begin -->
- `0` SUCCESS: sucesso
- `1` NO_MATCHES: zero matches encontrados (search, replace, transform, scope; não é falha)
- `1` DOCTOR_CHECKS_FAILED: `doctor --strict` achou checks reprovados ou com aviso (o evento do relatório traz todos os checks)
- `2` ARGUMENT_PARSE_ERROR: linha de comando rejeitada pelo parser
- `2` UNSUPPORTED_COMBINATION: duas flags que este verbo recusa combinar (leia `flag` e `other` no envelope)
- `4` FILE_NOT_FOUND: arquivo não encontrado
- `4` GLOB_NO_MATCHES: padrão não casou nenhum arquivo
- `13` PERMISSION_DENIED: permissão negada
- `28` DISK_FULL: disco cheio (sem espaço no dispositivo)
- `30` QUOTA_EXCEEDED: cota excedida
- `65` INVALID_INPUT: entrada inválida (argumentos ruins ou dados malformados, incluindo stdin vazio sem `--allow-empty-stdin` e arquivo acima de `[fuzzy].max_content_bytes` no fuzzy)
- `65` MATCH_FAILED: padrão não casou, ou um par do `edit-loop` sem match (leia `best_candidate`)
- `65` MATCH_AMBIGUOUS: padrão casou mais de uma vez (leia `best_candidate`)
- `65` BINARY_FILE: conteúdo binário rejeitado
- `65` FILE_TOO_LARGE: arquivo acima do limite de tamanho
- `69` GRAMMAR_UNAVAILABLE: a gramática tree-sitter de um idioma não está nesta máquina (classe `precondition_failed`, retryable `false`)
- `73` CROSS_DEVICE: rename cross-device (limite de filesystem)
- `74` IO_ERROR: erro de I/O
- `78` CONFIG_INVALID: configuração inválida, ou verbo que precisa de uma feature ausente neste build (slim)
- `80` TRANSACTION_ROLLBACK_FAILED: rollback da transação do batch falhou (backups mantidos)
- `81` CHECKSUM_VERIFY_FAILED: verificação de checksum falhou
- `82` STATE_DRIFT: state drift (mismatch de checksum, lock otimista falhou)
- `83` LOCK_TIMEOUT: lock não obtido em `lock.acquire_timeout_ms` (padrão 5000)
- `84` MERGE_CONFLICT: `semantic-merge` encontrou conflitos
- `85` FIFO_DETECTED: FIFO detectado (named pipe não pode ser escrito atomicamente)
- `86` DEVICE_FILE: arquivo de dispositivo detectado (bloco ou caractere)
- `87` EDIT_LOOP_PARTIAL: `edit-loop --partial` gravou os pares que casaram e alguns pares falharam
- `88` SYNTAX_ERROR_DETECTED: erro de sintaxe detectado (`--syntax-check` falhou)
- `91` EXDEV_FALLBACK_DISABLED: fallback EXDEV desabilitado (fallback de cópia cross-device não é permitido)
- `92` COPY_BACK_BLAKE3_FAILED: verificação BLAKE3 do copy-back falhou
- `93` ORPHAN_JOURNAL: journal órfão detectado
- `124` TIMEOUT: prazo de `--timeout-secs` global excedido (default 120; passe `0` para desligar)
- `126` WORKSPACE_JAIL: violação do jail do workspace (caminho escapa do workspace)
- `127` SYMLINK_BLOCKED: symlink bloqueado (alvo fora do workspace)
- `128` IMMUTABLE_FILE: arquivo imutável (não pode modificar)
- `129` SIGHUP: terminal desligado (SIGHUP)
- `130` CANCELLED: execução cancelada antes de terminar (o status segue a causa)
- `130` SIGINT: interrompido por SIGINT
- `141` BROKEN_PIPE: pipe quebrado (SIGPIPE ou erro de escrita `BrokenPipe`)
- `143` SIGTERM: terminado por SIGTERM
- `255` INTERNAL_ERROR: erro interno
<!-- exit-codes:end -->


## Tratamento de Sinal
- Unix: SIGINT (Ctrl+C), SIGTERM e SIGHUP são interceptados para um shutdown cooperativo com exit 130, 143 e 129; a execução termina com um evento `cancelled` que carrega `signal` e `deadline_exceeded`
- Unix: o SIGPIPE tem um handler, não é ignorado nem deixado na ação padrão; o handler marca um shutdown cooperativo com exit 141
- A ação padrão do SIGPIPE matava o processo antes de qualquer destrutor rodar e deixava sidecars de lock, journals e tempfiles das escritas em andamento; com o handler todo `Drop` roda e nada fica para trás
- Uma escrita que falha porque o leitor fechou sai 141 (`BROKEN_PIPE`) sem panic e sem relatório de falha: `completions`, `--print-schema`, `--help` e todo verbo (corrigido na 0.1.37, teste `cli_v0137_broken_pipe_outputs`)
- Uma saída muito curta que cabe no buffer do pipe (por exemplo `--version | head -1`) pode terminar antes de o leitor fechar e então sai 0
- Uma escrita que bate no pipe fechado falha com `BrokenPipe` e também sai 141, e um pai que ignora SIGPIPE recebe o mesmo exit 141
- Um SIGPIPE repetido nunca força a saída; um SIGINT, SIGTERM ou SIGHUP repetido força
- Windows: Ctrl+C interceptado via console handler; não existe SIGPIPE, e o `BrokenPipe` do `std` segue o mesmo caminho (não validado lá)
- Primeiro sinal: define a flag de shutdown e imprime uma mensagem no stderr no locale ativo (`atomwrite: encerrando (shutting down)...` com `--locale pt-BR`)
- Segundo sinal: terminação imediata via `_exit` (Unix) ou `exit` (Windows)
- Threads de walker (search, replace, transform, scope) param entre arquivos
- Operações em lote param entre operações
- Verbos de stdin (`write`, `edit` e os demais) obedecem aos sinais e ao prazo com o stdin ainda aberto: SIGTERM sai 143 (medido)


## Tratamento de Erro
- Todos os erros emitem um objeto JSON no stdout com `error: true` e `ok: false`
- Campos de erro: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`
- Todo resumo que publica `files_failed` acrescenta `partial: true` e `errors[]` ao lado (cada entrada tem `path`, `code` e `message`, `path` é `<walk>` quando a varredura falhou sem nomear um arquivo, no máximo 50), e isso cobre `search`, `transform`, `replace`, `scope`, `count`, `list`, `sparse`, `semantic-search` e `case`
- Classes de erro: `permanent`, `transient`, `conflict`, `precondition_failed`
- 35 variantes de erro em `src/error/variants.inc.rs`
- Erros transient e conflict definem `retryable: true`
- O campo `suggestion` fornece orientação de recuperação acionável para agentes, no locale ativo (`suggestion_lang`)
- Veja `docs/schemas/error-output.schema.json` para o contrato completo


## Performance
- Binário único; no Linux ele liga dinamicamente só com a biblioteca C do sistema (`ldd` no build da v0.1.37 lista `libgcc_s.so.1`, `libm.so.6` e `libc.so.6`)
- Builds de release usam LTO, codegen unit único e stripping de símbolos
- Leituras de arquivo via mmap com `memmap2` para arquivos grandes
- Paralelismo multi-arquivo é o modus operandi padrão: `ignore::WalkParallel` / `collect_files_parallel` (search conteúdo + `--target files`, replace, count, transform, scope, discovery recursiva) + `rayon` (hash, backup, delete/copy multi-path e recursivo, case multi-path, semantic-search, batch não-txn, prune multi-alvo, sparse read, diff dual-read)
- Limite de workers: `--threads` / `-j` / `--max-concurrency` (default = o menor entre os núcleos lógicos, 16 threads e o teto de RAM; um `-j N` explícito vale como pedido; ver `src/concurrency.rs`)
- Latência de arquivo pequeno: medianas de 5,6 ms (`read`) e 6,7 ms (`write`) em uma sessão de 20 execuções, e 9,4 ms e 9,7 ms em outra de 30 execuções, processo inteiro incluído; máquina: Intel Xeon E5-2699 v3 2,30 GHz com 72 CPUs lógicas, Linux, diretório de trabalho em tmpfs, medido em 2026-10-19 enquanto outros jobs rodavam no mesmo host


## Limites Conhecidos
- O `replace --expect-checksum` compara o checksum sob o lock, então um escritor que entra entre a checagem e a escrita é detectado como exit 82 `STATE_DRIFT`; os verbos que leem, mudam e regravam (`edit`, `edit-loop`, `replace`, `set`, `del`, `config set`, `apply`, `write --append`) serializam sob o lock
- Um nome de alvo grande o bastante para passar o sidecar de 255 bytes recebe um digest BLAKE3 curto como nome do sidecar, então um alvo de 240 bytes trava em vez de falhar com `ENAMETOOLONG`; um alvo de 244 bytes foi provado com `replace`
- O diretório do alvo precisa permitir a criação desse sidecar de lock: gravar num arquivo existente dentro de um diretório somente leitura sai 13 mesmo com o arquivo gravável
- O teto de RAM da contagem automática de workers lê `MemAvailable` de `/proc/meminfo` só no Linux e ignora limites de contêiner (cgroup)
- `--limit` é teto de trabalho: a lista de alvos é cortada antes da ação e a varredura sai no registro que atinge o teto, então uma árvore enorme não é lida até o fim; o `walker_stopped_early` reporta a saída do próprio caminhante, então uma árvore pequena o bastante para o walk paralelo terminar de despachar antes reporta `false` e o `files_visited` é a contagem honesta
- Sem `--sort-by`, a ordem dos resultados do `search` varia de uma execução para outra porque os walkers rodam em paralelo
- `case` expande um argumento diretório pelo walker compartilhado e honra `--include` e `--exclude`; diretório que não devolve nenhum arquivo sai 65 em vez de fingir sucesso sem renomear nada
- `query`, `outline` e `write --syntax-check` detectam só 23 linguagens por extensão; `--language NAME` funciona para um parser já extraído no cache local, e sai 65 `unsupported language override` para uma linguagem ainda não usada
- O `extract-artifact` emite um único evento `artifact` e nenhum resumo, então uma execução com sucesso não tem evento com `ok: true`
- `locale --set` e `locale --clear` imprimem o idioma resolvido de antes da mudança; um `atomwrite locale` seguinte mostra o estado novo
- macOS e Windows não foram executados; o código só de Unix tem pares `cfg(windows)` que foram apenas compilados com `cargo check` a partir do Linux


## API da Biblioteca
- A interface suportada do atomwrite é o CLI e seu contrato NDJSON; o crate também publica um alvo de biblioteca Rust que não é coberto por essa promessa
- A API Rust pública da v0.1.37 mudou em relação à 0.1.36 publicada de formas que o SemVer trata como quebra: o `cargo semver-checks` 0.50.0 reportou 10 verificações major reprovadas (removidos `locale::Idioma` e `locale::{negotiate_to_idioma,resolved_idioma}`, removido `LocaleSource::EnvVar`, removido `signal::reset_sigpipe`, quatro constantes de lock removidas, `LockGuard` deixou de ser `Send` e `Sync`, variantes novas em `Commands` e campos públicos novos em structs sem `#[non_exhaustive]`, e `watch` deixou de ser feature padrão)
- Fixe o `atomwrite` como dependência de biblioteca em uma versão exata (`=0.1.37`) e leia o [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) antes de cada atualização


## FAQ de Troubleshooting

### atomwrite write trava sem saída
- Garanta que está pipeando conteúdo para o stdin
- `write` lê do stdin e espera EOF
- Exemplo: `echo "content" | atomwrite --workspace . write file.txt`

### search retorna exit code 1
- Exit code 1 significa zero matches encontrados
- Este é comportamento esperado, não um erro
- Verifique o padrão e o caminho alvo

### search com padrão que começa com hífen
- Padrão que começa com `-` vai depois de `--`, como no `rg` e no `grep`
- `atomwrite --workspace . search -F -- '-deprecated' src/` busca esse texto literal
- Sem o `--` o token é uma flag que o verbo não tem, então a execução sai 2 e imprime a dica `to pass '-Z' as a value, use '-- -Z'`

### search, replace e transform saem 4 mesmo tendo imprimido matches
- Raiz nomeada na invocação que não existe reprova a execução mesmo quando as outras raízes casaram, em `search`, `replace` e `transform`
- Os matches continuam chegando no stdout, e o resumo traz `ok: false`, `code: FILE_NOT_FOUND`, `files_failed` e o caminho em `errors[]`
- Arquivo que falha durante a varredura é o outro caso: `partial: true` entra no resumo, e a execução mantém exit 0 em `search`, `replace`, `transform`, `scope`, `case`, `sparse`, `semantic-search` e `backup`, enquanto `count`, `list`, `hash`, `copy`, `delete` e `prune-backups` saem com a primeira falha porque a resposta deles é um total sobre a árvore; o contrato é o ADR-0077 (`docs/decisions/0077-answer-family-and-walk-honesty.md`)

### rename cross-device falha com exit 73
- A origem e o destino estão em filesystems diferentes
- atomwrite cai no fallback de copy+delete para `move` entre devices
- Use `copy` seguido de `delete` como alternativa

### mismatch de checksum com exit 82
- Outro processo modificou o arquivo entre read e write
- Releia o arquivo para obter o checksum atual
- Retente a operação com o `--expect-checksum` atualizado

### violação do jail do workspace com exit 126
- O caminho alvo resolve fora do limite do `--workspace`
- Verifique que o caminho não contém travessias `..` ou symlinks escapando do workspace

### verificação de sintaxe falhou com exit 88
- A verificação de sintaxe tree-sitter real encontrou um erro de sintaxe no arquivo
- Inspecione a primeira linha/coluna/kind/mensagem de erro no envelope JSON de erro
- Corrija a sintaxe e retente, ou remova `--syntax-check` para bypassar

### stdin vazio rejeitado com exit 65
- O guard rejeitou 0 bytes do stdin como provável falha de pipeline upstream
- Passe `--allow-empty-stdin` para confirmar que a entrada vazia é intencional
- Ou pipe conteúdo real: `echo "x" | atomwrite --workspace . write file`

### edit trava (ou falha com exit 65) em --after-line/--after-match/--between/--multi
- Esses modos leem o conteúdo de substituição do stdin, igual ao `write`
- Um stdin de terminal falha RÁPIDO com exit 65 `INVALID_INPUT` em vez de bloquear para sempre
- Pipe conteúdo: `echo "texto novo" | atomwrite --workspace . edit arquivo.rs --after-line 10`
- Ou use `--new-file <PATH>` quando o modo aceitar
- `--after-match`/`--before-match`/`--between`/`--after-line`/`--before-line`/`--range`/`--delete-range` rejeitam `--old`/`--new`/`--old-file`/`--new-file` no parse (exit 2): esses modos só aceitam conteúdo via stdin

### watch sai 78
- O binário padrão é compilado sem a feature `watch`
- Recompile com `cargo install --path . --locked --force --features watch`


## MSRV e Alvos
- Versão mínima suportada do Rust: 1.90 (edition 2024); o `rust-toolchain.toml` fixa o canal 1.94 para desenvolvimento
- Locales: `en` e `pt-BR`
- Linux x86_64 é exercitado de ponta a ponta
- Windows é conferido só com `cargo check --target x86_64-pc-windows-gnu` a partir de um host Linux
- macOS não foi exercitado em um Mac real


## Documentação
- [docs/INSTALL.pt-BR.md](docs/INSTALL.pt-BR.md): instalação, features e perfis de build
- [docs/HOW_TO_USE.pt-BR.md](docs/HOW_TO_USE.pt-BR.md): guia de uso completo
- [docs/COOKBOOK.pt-BR.md](docs/COOKBOOK.pt-BR.md): receitas por tarefa
- [docs/AGENTS.pt-BR.md](docs/AGENTS.pt-BR.md): contrato para agentes LLM
- [docs/CROSS_PLATFORM.pt-BR.md](docs/CROSS_PLATFORM.pt-BR.md): notas por plataforma
- [docs/MIGRATION.pt-BR.md](docs/MIGRATION.pt-BR.md): caminhos de atualização entre versões
- [docs/TESTING.pt-BR.md](docs/TESTING.pt-BR.md): como a suíte de testes é organizada
- [INTEGRATIONS.pt-BR.md](INTEGRATIONS.pt-BR.md): usar o atomwrite a partir de agentes e scripts
- [PRIVACY.pt-BR.md](PRIVACY.pt-BR.md): o que o binário lê, grava e baixa
- [llms.pt-BR.txt](llms.pt-BR.txt) e [llms-full.pt-BR.txt](llms-full.pt-BR.txt): resumos para janelas de contexto de LLM


## Arquitetura
- Veja [ARCHITECTURE.pt-BR.md](ARCHITECTURE.pt-BR.md) para mapa de módulos, fluxo de dados e decisões de design
- Veja [docs/decisions/](docs/decisions/README.md) para o índice de 60 ADRs (0019 a 0078; mais 3 ficam em `docs/adr/`)
- A verificação é rodar o programa, e não uma suíte de testes: toda afirmação deste README tem por trás uma invocação real do binário, e o [docs/TESTING.pt-BR.md](docs/TESTING.pt-BR.md) descreve o arranjo dos casos que o mantenedor escolhe manter
- Veja [docs/schemas/](docs/schemas/README.md) para os contratos de JSON Schema da saída NDJSON, onde 54 arquivos descrevem os 55 nomes que `atomwrite --print-schema` lista


## Contribuindo
- Veja [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) para setup de desenvolvimento, DoD local, processo de PR e notas de release
- Veja [CONTRIBUTING.md](CONTRIBUTING.md) para o guia em inglês
- Siga o [Código de Conduta](CODE_OF_CONDUCT.pt-BR.md) / [Code of Conduct](CODE_OF_CONDUCT.md)


## Segurança
- Veja [SECURITY.pt-BR.md](SECURITY.pt-BR.md) para relato de vulnerabilidades
- Veja a seção "Known Security Advisories" em SECURITY.pt-BR.md para advisories resolvidos e ativos
- Veja [PRIVACY.pt-BR.md](PRIVACY.pt-BR.md) para o tratamento de dados


## Changelog
- Veja [CHANGELOG.md](CHANGELOG.md) para o histórico de releases em inglês
- Veja [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) para o histórico de releases em português


## Licença
- Licenciado sob MIT OR Apache-2.0
- Veja [LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE) para detalhes
