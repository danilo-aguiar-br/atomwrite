[Read in English](README.md)


# atomwrite

> Operações atômicas de arquivo para agentes LLM -- um CLI, zero corrupção

[![Crates.io](https://img.shields.io/crates/v/atomwrite)](https://crates.io/crates/atomwrite)
[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)
[![License](https://img.shields.io/crates/l/atomwrite)](LICENSE)
[![CI](https://github.com/daniloaguiarbr/atomwrite/actions/workflows/ci.yml/badge.svg)](https://github.com/daniloaguiarbr/atomwrite/actions)


## O Que É
- Um único binário Rust que resolve toda operação de arquivo que um agente LLM precisa
- **30 subcomandos**: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, batch, backup, rollback, apply, set, get, del, case, query, outline, wal-heal, wal-stats, completions
- Toda escrita é atômica: tempfile, fsync, rename, fsync do diretório
- Toda resposta é NDJSON: um objeto JSON por linha, legível por máquina
- Todo arquivo recebe checksum BLAKE3: detecta drift, verifica integridade, habilita locking otimista


## O Que Há De Novo Na v0.1.20 (2026-06-15)
- **542 testes passando** em 47 suítes de teste (502 na v0.1.18 + 21 na v0.1.19 + 19 na v0.1.20), 0 falhas, 0 warnings de clippy
- **11 GAP-2026 fechados** na v0.1.20
- **Intention guards** — nova camada de segurança OPT-IN com 5 flags: `--require-backup N`, `--confirm`, `--auto-rotate N`, `--risk-threshold LOW|MEDIUM|HIGH` e `--locale en|pt-BR`. Interceptam mutações destrutivas antes de tocarem o disco. ADR-0032
- **Renomeação `--locale`** a partir de `--lang` para desambiguar do `--lang` tree-sitter usado por `scope` e `transform`. O antigo `--lang` permanece como alias oculto por uma versão minor. ADR-0034
- **`count --by-size`** — lista os maiores arquivos da árvore com tamanhos e contagem de linhas. ADR-0035
- **`read --mode raw|envelope`** — seleciona entre saída byte-stream e envelope NDJSON estruturado. ADR-0036
- **`search --no-begin-end`** — desabilita a decoração implícita de âncoras `^` e `$` na saída regex. ADR-0037
- **`scope --lang rust`** — alias explícito aceito para simetria ergonômica com `transform --lang`. ADR-0033
- **`write --preserve-timestamps`** — preserva o mtime do arquivo fonte ao sobrescrever
- **19 ADRs** em `docs/decisions/` (0019-0037). 3 targets de cross-compile Windows verdes (x86_64-gnu, i686-gnu, x86_64-msvc)


## O Que Houve De Novo Na v0.1.19 (2026-06-14)
- **G121 helper de resolução de caminho** que unifica a resolução do jail em todos os subcomandos de escrita. Fonte única de verdade para a checagem do limite do workspace
- **Modo `query` S-expression real** via `tree-sitter` — agentes agora podem rodar queries AST estruturadas contra arquivos fonte
- **7 derivas de documentação de exit codes** consolidadas para casar com a lista canônica em `docs/exit-codes.md` (ADR-0031)
- **21 novos testes** em 3 suítes de integração. Targets de cross-compile permanecem verdes
## O Que Houve De Novo Na v0.1.17 (2026-06-13)
- **L3 auto-heal no startup**: `atomwrite` executa um passe autônomo de `wal-heal` no startup com threshold de 3600s e budget de 100ms. O passe é opt-out via `--skip-startup-wal-heal` (ver `src/cli.rs`). O subcomando explícito `wal-heal` aterrissou na v0.1.15; v0.1.17 conecta a mesma lógica em `lib.rs::run` antes do despacho de qualquer subcomando


## O Que Houve De Novo Na v0.1.16 (2026-06-13)
- **L1 WalPolicy**: `enum WalPolicy { Auto, Always, Never }` em `src/wal.rs`, exposto via flag `--wal-policy` em `write` e `edit`. Default `Auto` pula o sidecar WAL para escritas triviais (tamanho sob 1 MiB, não-Edit/Replace, diretório sob Git, escrita sob 4 KiB)
- **L4 HeuristicsEngine**: submódulo `crate::wal::heuristics` com 5 funções componíveis (`h1_ttl`, `h2_lru_within_cap`, `h3_rate_limit`, `h4_sentinel`, `h5_archive`). Env vars: `ATOMWRITE_WAL_KEEP_SECS`, `ATOMWRITE_WAL_MAX_COUNT`, `ATOMWRITE_WAL_RATE_LIMIT`, `ATOMWRITE_WAL_ARCHIVE_DAYS`. Campo de telemetria `wal_policy` em `WriteOutput` NDJSON


## O Que Houve De Novo Na v0.1.15 (2026-06-11)
- **G117 corrigido — `edit` multi-par alcança paridade fuzzy com o caminho de par único**: pares repetidos `--old`/`--new` agora rodam a cascata fuzzy completa de 9 estratégias por par, em vez de busca exata apenas
- **Relato por par**: envelopes de sucesso ganham `pairs_total` e `pair_results` (`index` 1-based, `matched`, `strategy`, `similarity`); envelopes de erro ganham `failed_pair_index`, `pairs_total` e `pair_results` — sem mais bisseção manual de lotes falhos
- **Nova flag `--partial` (opt-in)**: aplica os pares que casam e relata os demais com `matched: false`; zero pares aplicados sai com 1 (`NO_MATCHES`) sem escrever; o padrão continua all-or-nothing
- **Receita anti-mascaramento**: `edit ... | jaq '.edits'` esconde o exit 65 como `{"edits": null}` — use sempre `jaq -e '.edits'` ou verifique `${PIPESTATUS[0]}`
- **G118 corrigido — `write` resolve o alvo contra o workspace antes de cada pré-passo**: com CWD fora do workspace, `--append`/`--prepend` truncava o arquivo, `--expect-checksum` era silenciosamente pulado e `--line-ending auto` perdia a detecção (dupla identidade de caminho, CWE-367); divergência de checksum agora sai com 82 (`STATE_DRIFT`) e alvo fora do jail falha cedo com exit 126 (ADR-0027)
- **GAP 18 corrigido — CI Windows verde de novo**: o snapshot do write agora redige `dir_fsync` como `[platform_dir_fsync]` (Windows emite `best_effort`, Unix `sync_all`)
- **Job de MSRV alinhado**: o CI agora testa o MSRV documentado 1.88 (o job estava pinado em 1.85)
- **461 testes passando** (445 na v0.1.12 + 2 na v0.1.14 + 8 G117 + 6 G118). 43 suítes de teste, 0 falhas

## O Que Houve De Novo Na v0.1.12 (2026-06-07)
- **6 novos subcomandos (v14 Tier 3)**: `set`, `get`, `del`, `case`, `query`, `outline` para edição estruturada de configuração e análise AST
- **G72 verificação de sintaxe REAL** -- `atomwrite write --syntax-check` invoca o parser `tree-sitter` real (24 linguagens cobertas) no lugar da heurística de balanceamento de colchetes. Código de saída 88 no primeiro erro de sintaxe
- **G114 sidecar WAL para recuperação de crash** -- `atomic_write` escreve `.atomwrite.journal.<target>.atomwrite.journal.json` com entradas `Started` e `Committed`. `recover_orphan_journals(dir)` é consultivo: relata órfãos sem auto-replay
- **5 novas variantes de erro**: `LockTimeout` (83), `SyntaxError` (88), `ExdevFallbackDisabled` (91), `CopyBackBlake3Failed` (92), `OrphanJournal` (93). Todas bilíngues EN/PT-BR com sugestões acionáveis em `ErrorContext`
- **Dependência adicionada**: `tree-sitter-language-pack = "1.8"` com features `download` + `dynamic-loading`. Parsers baixam no primeiro uso, footprint da instalação fica em torno de 5-10 MB em vez de 1+ GB
- **445 testes passando** (eram 320 baseline em v0.1.10, +125 novos em v0.1.11 e v0.1.12). 43 suites de testes de integração, 0 falhas
- **7 ADRs** em `docs/decisions/` documentando todas as decisões arquiteturais para v0.1.12 (escolha de tree-sitter-language-pack, design do WAL sidecar, query/outline apenas kind-name, etc.)
- **7 novos JSON Schemas** em `docs/schemas/` (set, get, del, case, query, outline, wal-recovery)

Veja `docs/HOW_TO_USE.pt-BR.md` para o quickstart completo da v0.1.12 e `docs/MIGRATION.pt-BR.md` para o caminho de upgrade v0.1.11 → v0.1.12. v0.1.11 é a release anterior.


## O Que Houve De Novo Na v0.1.11 (2026-06-05)
- **`signal_test::shutdown_message_on_stderr` não falha mais no CI Windows (windows-2025-vs2026)** — `libc::write(STDERR_FILENO, ...)` foi movido de `src/main.rs` para `src/signal.rs` e gated por `#[cfg(unix)]`. O caminho Windows `ctrlc` foi mantido como estava. Também foi adicionado um loop de retry `EAGAIN` e `EINTR` para tornar o write robusto contra syscalls interrompidos e limites apertados de buffer de pipe em sandboxes de CI
- **Detecção de readiness sem race no `signal_test`** — O teste agora define `ATOMWRITE_READY_FILE` para um caminho sob o tempdir, e o atomwrite escreve seu PID lá assim que `install_handlers_early` retorna. O teste faz poll do arquivo com deadline de 10 s antes de enviar SIGINT, eliminando a janela de microssegundos onde SIGINT poderia competir com `posix_spawn` e chegar antes do `sigaction` do kernel ser configurado
- **Instalação idempotente de signal-handler** — `install_handlers_early` e `install_handlers` agora compartilham um único `Arc<ShutdownSignal>` via `OnceCell`. Anteriormente cada função criava sua própria instância, e apenas a primeira era flipada pela cadeia signal-hook, então a checagem `is_shutdown()` da main thread ficava `false` e a banner nunca era escrita


## O Que Houve De Novo Na v0.1.10 (2026-06-05)
- **GAP 20 follow-up**: `signal_test::shutdown_message_on_stderr` faz flush da mensagem de shutdown via `io::stderr().lock()`. A correção v0.1.8 moveu `eprintln!` do signal handler para a main thread mas usava `writeln!(io::stderr(), ...)` que é totalmente bufferizado quando stderr é redirecionado para um pipe. A correção usa o guard `StderrLock` que faz flush no Drop


## O Que Houve De Novo Na v0.1.8 (2026-06-05)
- **`signal_test::shutdown_message_on_stderr` não falha mais no CI Linux** — `eprintln!` removido dos signal handlers SIGINT/SIGTERM conforme POSIX.1-2017 `signal-safety(7)`. Mensagem agora emitida pela main thread em `src/main.rs` após `atomwrite::run` retornar
- **`atomic::tests::create_backup_and_retention` não falha mais no CI Windows** — `platform::fsync_file_best_effort` registra um warning e continua em `ERROR_ACCESS_DENIED`
- **Matriz CI pinada em `windows-2025-vs2026`** — Substituiu `windows-latest` para silenciar NOTICE de migração


## O Que Houve De Novo Na v0.1.7 (2026-06-05)
- **CI GitHub Actions totalmente verde** — Todos os 6 jobs (check matrix x3, deny, doc, msrv, security) passam após corrigir 4 falhas distintas
- **MSRV elevado para 1.88** — Necessário para permitir `time` 0.3.47 que resolve RUSTSEC-2026-0009
- **GitHub Actions Node 24 pronto** — `actions/checkout@v6`, `actions/cache@v5`, `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true`
- **Cross-compile Windows validado localmente no macOS** — Ambos `x86_64-pc-windows-gnu` e `i686-pc-windows-gnu` passam `cargo check`, `cargo build`, `cargo clippy --all-targets --all-features -- -D warnings` e `cargo test --no-run`


## O Que Houve De Novo Na v0.1.4
- **`cargo install atomwrite` funciona no Windows 10/11** — Três erros de compilação em blocos `#[cfg(windows)]` foram corrigidos (E0433, E0507, E0308)
- **Sugestões de erro context-aware** — Sugestão de `WorkspaceJail` se adapta quando `--workspace` já é fornecido
- **Gate de cross-compile** — `tests/cross_compile_check.rs` falha em qualquer regressão `E0433`, `E0308` ou `E0507` em blocos `cfg(windows)`


## O Que Houve De Novo Na v0.1.3
- Flag `--preserve-timestamps` em `edit` e `replace` para controlar mtime do arquivo (padrão: mtime é atualizado para refletir a mudança)
- Campo `mtime_preserved` nas respostas NDJSON `EditOutput` e `ReplaceResult`
- BREAKING: escrita atômica não preserva mais o mtime original do arquivo por padrão. Corrige um no-op silencioso em `cargo build` / `make` / `cmake` / `gradle`


## Por Quê
- Agentes LLM gerenciam dezenas de comandos shell para manipular arquivos
- Uma única falha de energia ou crash no meio de uma escrita corrompe o arquivo
- Parsear saída não estruturada de CLI desperdiça tokens e causa alucinações
- Agentes precisam de checksums para detectar edições concorrentes mas raramente os computam
- atomwrite resolve todos os quatro problemas com um único `cargo install`


## Superpoderes
### Escritas Atômicas
- Usa tempfile + fsync + rename + fsync do diretório em cada escrita
- Garante all-or-nothing: o arquivo nunca fica meio escrito
- Sobrevive a perda de energia, OOM kills e SIGKILL
- Verificação de sintaxe G72 REAL opcional via tree-sitter (flag `--syntax-check` em `write`)
- Sidecar WAL G114 opcional para recuperação de crash (`.atomwrite.journal.<target>.atomwrite.journal.json`)

### Saída NDJSON
- stdout é SEMPRE JSON estruturado, um objeto por linha
- Cada objeto carrega um campo discriminador `"type"`
- Agentes parseiam saída sem regex ou scraping frágil de texto
- Erros também emitem JSON com `error: true` no stdout

### Checksums BLAKE3
- Cada resposta de `read` e `write` inclui um hash BLAKE3
- Use `--expect-checksum` para locking otimista em edições concorrentes
- Detecte state drift antes de aplicar mudanças

### Busca Paralela
- Construída sobre o engine ripgrep para busca em conteúdo de arquivos
- Respeita `.gitignore` automaticamente
- Retorna matches estruturados com arquivo, linha, coluna e contexto

### Transformações AST-Aware
- Busca estrutural e reescrita powered by ast-grep
- Cobre 306 linguagens de programação
- Refatore código pela árvore sintática, não regex frágil
- Novos subcomandos `query` e `outline` caminham ASTs tree-sitter (305 linguagens) via `tree-sitter-language-pack`

### Scoping Gramatical
- Selecione categorias AST como comentários, funções, classes e strings
- Aplique ações: delete, uppercase, lowercase, titlecase, squeeze ou replace
- Cobre Rust, Python, JavaScript, TypeScript e Go com queries preparadas
- Use `--pattern` para padrões AST customizados além das queries built-in

### Operações em Lote
- Execute operações write, replace, delete, edit, hash, move e copy a partir de um manifesto NDJSON
- Use `--transaction` para execução all-or-nothing com rollback automático
- Todas as operações em um lote compartilham as mesmas garantias atômicas

### Edição Estruturada de Config (v0.1.12)
- `set` escreve um valor em um dotted path em arquivos TOML ou JSON (preserva comentários via `toml_edit`)
- `get` lê um valor em um dotted path com formato auto-detectado
- `del` remove uma chave (com `--force-missing` para tratar chaves ausentes como sucesso no-op)
- `case` renomeia identificadores em múltiplos arquivos via `heck` (snake, camel, pascal, kebab, screaming-snake)


## Quick Start
```bash
cargo install atomwrite

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

# v0.1.12: definir um valor em um arquivo TOML (preserva comentários)
atomwrite --workspace . set Cargo.toml package.version 0.2.0

# v0.1.12: caminhar o AST de um arquivo Rust
atomwrite --workspace . query src/main.rs --kinds

# v0.1.12: extrair outline de um arquivo Python
atomwrite --workspace . outline src/app.py

# v0.1.12: verificação de sintaxe tree-sitter REAL antes de commitar
echo "fn broken(" | atomwrite --workspace . write --syntax-check src/x.rs

# v0.1.15: relatar sidecars obsoletos e estimar reclaim
atomwrite --workspace . wal-stats

# v0.1.15: colher journals mais antigos que o threshold
atomwrite --workspace . wal-heal --threshold-secs 3600
```


## Instalação
### Do crates.io
```bash
cargo install atomwrite
```

### Do source
```bash
git clone https://github.com/daniloaguiarbr/atomwrite.git
cd atomwrite
cargo build --release
```

### Completions de Shell
```bash
# Bash
atomwrite completions bash > ~/.local/share/bash-completion/completions/atomwrite

# Zsh
atomwrite completions zsh > ~/.zfunc/_atomwrite

# Fish
atomwrite completions fish > ~/.config/fish/completions/atomwrite.fish
```


## Uso
- Toda saída vai para stdout como NDJSON
- Todos os logs vão para stderr (apenas com `--verbose`)
- Use `--workspace <DIR>` para restringir operações à raiz do projeto
- Use `ATOMWRITE_WORKSPACE` para definir a raiz do workspace via env var
- Use `--dry-run` antes de operações destrutivas
- Use `--expect-checksum <HASH>` para locking otimista
- Use `--lang <LOCALE>` para sobrescrever o idioma de exibição (en, pt-BR)
- Pipe stdin para `write` e `batch`


## Comandos (30 no total)

### I/O Central
- `read` — ler arquivos com metadados, checksum, conteúdo opcional
- `write` — criar ou sobrescrever arquivos atomicamente via stdin
- `edit` — editar cirurgicamente por número de linha, marcador de texto ou match exato
- `delete` — deletar arquivos com backup opcional
- `copy` — copiar arquivos com verificação de checksum
- `move` — mover ou renomear arquivos atomicamente (fallback copy em EXDEV)
- `apply` — aplicar patches do stdin (unified diff, search/replace, full, markdown)

### Buscar e substituir
- `search` — buscar conteúdo de arquivos em paralelo (engine ripgrep)
- `replace` — substituir texto entre arquivos com escritas atômicas
- `transform` — refatoração AST via ast-grep (306 linguagens)

### Inspeção
- `hash` — calcular checksums BLAKE3
- `count` — contar linhas, arquivos por extensão
- `diff` — comparar dois arquivos (unified, stat ou changes)
- `list` — listar arquivos em uma árvore de diretório
- `extract` — extrair campos de entrada NDJSON via pipe
- `scope` — scoping gramatical (deletar todos os comentários, etc.)
- `regex` — gerar regex a partir de exemplos
- `calc` — matemática e conversões de unidades
- `completions` — gerar completions de shell (bash, zsh, fish, elvish, powershell)

### Backup e recovery
- `backup` — criar backups com timestamp e checksums BLAKE3
- `rollback` — restaurar a partir de um backup anterior
- `batch` — operações em lote dirigidas por NDJSON (transacional)

### Editores de config estruturada (v0.1.12, v14 Tier 3)
- `set <PATH> <KEY_PATH> <VALUE>` — escrever um valor em um dotted path em um arquivo TOML ou JSON. Preserva comentários e ordem de chaves via `toml_edit`. Auto-coerce int/bool/float/string.
- `get <PATH> <KEY_PATH>` — ler um valor em um dotted path. NDJSON: `{"type":"get","key_path","value","found","format"}`.
- `del <PATH> <KEY_PATH>` — remover uma chave. Flag `--force-missing` trata chaves ausentes como sucesso no-op.
- `case <PATHS...> --subvert OLD NEW --to <style>` — renomear identificadores em múltiplos arquivos via `heck`. Estilos: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`.

### Ferramentas AST (v0.1.12, v14 Tier 3 + G72, via tree-sitter-language-pack)
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` — caminhar um AST tree-sitter e emitir nós como NDJSON. 305 linguagens suportadas.
- `outline <PATH> [--kind <KIND>] [--positions]` — extrair estrutura de alto nível (funções, classes, structs, enums, traits, módulos) como NDJSON.
- `write --syntax-check` — verificação de sintaxe REAL G72 via tree-sitter. 24 linguagens cobertas. Exit 88 com primeira linha/coluna/kind/mensagem de erro.

### Limpeza WAL (v0.1.15, G119)
- `wal-stats` — snapshot de telemetria: total de journals, breakdown por estado, idade do mais antigo, tamanho total, breakdown por diretório, recomendação de auto-heal, bytes estimados de reclaim
- `wal-heal [--threshold-secs N] [--budget-ms N]` — colher journals terminais obsoletos mais antigos que o threshold. Seguro e idempotente

### Padrões comuns para cada subcomando

```bash
# read
atomwrite --workspace . read src/main.rs

# write
echo "fn main() {}" | atomwrite --workspace . write src/main.rs

# edit
atomwrite --workspace . edit src/main.rs --old "texto antigo" --new "texto novo"

# search
atomwrite --workspace . search 'TODO' src/ --include '*.rs'

# replace
atomwrite --workspace . replace 'nome_antigo' 'nome_novo' src/ --include '*.rs'

# hash
atomwrite --workspace . hash src/main.rs src/lib.rs

# delete
atomwrite --workspace . delete src/temp.rs --backup

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

# extract
atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line

# calc
atomwrite calc "2 GiB to bytes"
atomwrite calc "sqrt(144) + 3^2"

# regex
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"

# transform
atomwrite --workspace . transform -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/

# scope
atomwrite --workspace . scope src/ --lang rust --query comments --delete

# batch
cat manifest.ndjson | atomwrite --workspace . batch --transaction

# backup
atomwrite --workspace . backup src/config.toml

# rollback
atomwrite --workspace . rollback src/config.toml

# apply
echo "novo conteúdo" | atomwrite --workspace . apply src/file.txt --format full

# set (v0.1.12)
atomwrite --workspace . set Cargo.toml package.version 0.2.0

# get (v0.1.12)
atomwrite --workspace . get Cargo.toml package.version

# del (v0.1.12)
atomwrite --workspace . del Cargo.toml package.metadata.boring

# case (v0.1.12)
atomwrite --workspace . case src/ --subvert user_id UserId --to pascal

# query (v0.1.12)
atomwrite --workspace . query src/main.rs --kinds
atomwrite --workspace . query src/main.rs -Q function_item --positions

# outline (v0.1.12)
atomwrite --workspace . outline src/app.py

# wal-stats (v0.1.15)
atomwrite --workspace . wal-stats

# wal-heal (v0.1.15)
atomwrite --workspace . wal-heal --threshold-secs 3600 --budget-ms 100

# completions
atomwrite completions bash
```


## Variáveis de Ambiente
- `NO_COLOR`: desabilita saída colorida quando definida com qualquer valor
- `RUST_LOG`: controla verbosidade de log (ex., `RUST_LOG=debug`)
- `ATOMWRITE_LANG`: sobrescreve locale para mensagens traduzidas (ex., `en`, `pt-BR`)
- `ATOMWRITE_WORKSPACE`: define a raiz do workspace para validação de path jail (alternativa a `--workspace`)
- `ATOMWRITE_WAL_KEEP_SECS`, `ATOMWRITE_WAL_MAX_COUNT`, `ATOMWRITE_WAL_RATE_LIMIT`, `ATOMWRITE_WAL_ARCHIVE_DAYS`: knobs do G119 L4 HeuristicsEngine (v0.1.16+)
- `RAYON_NUM_THREADS`: sobrescreve número de threads paralelas para search, replace, transform e scope


## Códigos de Saída
- `0`: sucesso
- `1`: zero matches encontrados (search, não é um erro)
- `4`: arquivo não encontrado
- `13`: permissão negada
- `28`: disco cheio (sem espaço no dispositivo)
- `30`: cota excedida
- `65`: entrada inválida (argumentos ruins ou dados malformados, incluindo stdin vazio sem `--allow-empty-stdin` desde v0.1.16)
- `73`: rename cross-device (limite de filesystem)
- `74`: erro de I/O
- `78`: configuração inválida
- `81`: verificação de checksum falhou
- `82`: state drift (mismatch de checksum, lock otimista falhou)
- `83`: timeout de lock (v0.1.12+)
- `85`: FIFO detectado (named pipe não pode ser escrito atomicamente)
- `86`: arquivo de dispositivo detectado (bloco ou caractere)
- `88`: erro de sintaxe detectado (v0.1.12+, `--syntax-check` falhou)
- `91`: fallback EXDEV desabilitado (v0.1.12+, `--strict-atomic` proíbe fallback de cópia cross-device)
- `92`: verificação BLAKE3 do copy-back falhou (v0.1.12+)
- `93`: journal órfão detectado (v0.1.12+, recovery consultivo G114)
- `126`: violação do jail do workspace (caminho escapa do workspace)
- `127`: symlink bloqueado (alvo do symlink fora do workspace)
- `128`: arquivo imutável (não pode modificar)
- `130`: interrompido por SIGINT
- `141`: pipe quebrado (SIGPIPE)
- `143`: terminado por SIGTERM
- `255`: erro interno


## Tratamento de Sinal
- Unix: SIGINT (Ctrl+C) e SIGTERM interceptados para shutdown gracioso
- Unix: SIGPIPE resetado para SIG_DFL para comportamento padrão de pipe (exit 141)
- Windows: Ctrl+C interceptado via console handler
- Primeiro sinal: define flag de shutdown, imprime "shutting down..." no stderr
- Segundo sinal: terminação imediata via `_exit` (Unix) ou `exit` (Windows)
- Threads de walker (search, replace, transform, scope) param entre arquivos
- Operações em lote param entre operações


## Tratamento de Erro
- Todos os erros emitem um objeto JSON no stdout com `error: true`
- Campos de erro: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`
- Classes de erro: `permanent`, `transient`, `conflict`, `precondition_failed`
- 25 variantes de erro no total (20 baseline de v0.1.4 + 5 adicionadas em v0.1.12)
- Erros transient e conflict definem `retryable: true`
- O campo `suggestion` fornece orientação de recuperação acionável para agentes
- Veja `docs/schemas/error-output.schema.json` para o contrato completo


## Performance
- Binário estático único com zero dependências de runtime
- Builds de release usam LTO, codegen unit único e stripping de símbolos
- Leituras de arquivo via mmap com `memmap2` para arquivos grandes
- Busca paralela via rayon e engine ripgrep
- Latência típica de operação de arquivo: sob 5 ms para arquivos pequenos


## FAQ de Troubleshooting

### atomwrite write trava sem saída
- Garanta que está pipeando conteúdo para o stdin
- `write` lê do stdin e espera EOF
- Exemplo: `echo "content" | atomwrite --workspace . write file.txt`

### search retorna exit code 1
- Exit code 1 significa zero matches encontrados
- Este é comportamento esperado, não um erro
- Verifique o padrão e o caminho alvo

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

### verificação de sintaxe falhou com exit 88 (v0.1.12+)
- A verificação de sintaxe tree-sitter REAL G72 encontrou um erro de sintaxe no arquivo
- Inspecione a primeira linha/coluna/kind/mensagem de erro no envelope JSON de erro
- Corrija a sintaxe e retente, ou remova `--syntax-check` para bypassar

### stdin vazio rejeitado com exit 65 (v0.1.16+)
- O guard G120 L1 rejeitou 0 bytes do stdin como provável falha de pipeline upstream
- Passe `--allow-empty-stdin` para confirmar que a entrada vazia é intencional
- Ou pipe conteúdo real: `echo "x" | atomwrite --workspace . write file`


## Arquitetura
- Veja [ARCHITECTURE.pt-BR.md](ARCHITECTURE.pt-BR.md) para mapa de módulos, fluxo de dados e decisões de design
- Veja [docs/decisions/](docs/decisions/README.md) para 12 ADRs cobrindo arquitetura a partir de v0.1.12 (G72, G114, v14 Tier 3, G117, G118, G119, G120, trio v0.1.18)
- Veja [docs/schemas/](docs/schemas/README.md) para 22 contratos estáveis de JSON Schema para toda saída NDJSON


## Contribuindo
- Veja [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) para setup de desenvolvimento e guidelines
- Veja [docs/decisions/README.md](docs/decisions/README.md) para o log de decisões


## Segurança
- Veja [SECURITY.pt-BR.md](SECURITY.pt-BR.md) para relato de vulnerabilidades
- Veja seção "Known Security Advisories" em SECURITY.pt-BR.md para advisories resolvidos e ativos


## Changelog
- Veja [CHANGELOG.md](CHANGELOG.md) para o histórico de releases em inglês
- Veja [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) para o histórico de releases em português


## Licença
- Licenciado sob MIT OR Apache-2.0
- Veja [LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE) para detalhes
