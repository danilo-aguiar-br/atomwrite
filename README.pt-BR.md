[Read in English](README.md)


# atomwrite

> Operações atômicas de arquivo para agentes LLM -- um CLI, zero corrupção

[![Crates.io](https://img.shields.io/crates/v/atomwrite)](https://crates.io/crates/atomwrite)
[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)
[![License](https://img.shields.io/crates/l/atomwrite)](LICENSE)
[![CI](https://github.com/daniloaguiarbr/atomwrite/actions/workflows/ci.yml/badge.svg)](https://github.com/daniloaguiarbr/atomwrite/actions)


## O Que É
- Um único binário Rust que resolve toda operação de arquivo que um agente LLM precisa
- **28 subcomandos**: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, batch, backup, rollback, apply, set, get, del, case, query, outline, completions
- Toda escrita é atômica: tempfile, fsync, rename, fsync do diretório
- Toda resposta é NDJSON: um objeto JSON por linha, legível por máquina
- Todo arquivo recebe checksum BLAKE3: detecta drift, verifica integridade, habilita locking otimista


## O Que Há De Novo Na v0.1.12 (2026-06-07)
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
- **GAP 20 follow-up**: `signal_test::shutdown_message_on_stderr` faz flush da mensagem via `io::stderr().lock()`. A correção do v0.1.8 moveu `eprintln!` do signal handler para a main thread mas usou `writeln!(io::stderr(), ...)` que é fully buffered quando stderr é redirecionado para um pipe. A correção usa o guard `StderrLock` que faz flush no Drop


## O Que Houve De Novo Na v0.1.8 (2026-06-05)
- **`signal_test::shutdown_message_on_stderr` não falha mais no CI Linux** — `eprintln!` removido dos handlers SIGINT/SIGTERM conforme POSIX.1-2017 `signal-safety(7)`. Mensagem agora emitida pela main thread em `src/main.rs` após `atomwrite::run` retornar
- **`atomic::tests::create_backup_and_retention` não falha mais no CI Windows** — `platform::fsync_file_best_effort` registra warning e continua em `ERROR_ACCESS_DENIED`
- **Matriz de CI fixada em `windows-2025-vs2026`** — Substituído `windows-latest` para silenciar NOTICE de migração


## O Que Houve De Novo Na v0.1.7 (2026-06-05)
- **CI do GitHub Actions 100% verde** — Todos os 6 jobs (check matrix x3, deny, doc, msrv, security) passam após corrigir 4 falhas distintas
- **MSRV bumped para 1.88** — Necessário para permitir `time` 0.3.47 que resolve RUSTSEC-2026-0009
- **GitHub Actions pronto para Node 24** — `actions/checkout@v6`, `actions/cache@v5`, `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true`


## O Que Houve De Novo Na v0.1.4
- **`cargo install atomwrite` funciona no Windows 10/11** — Três erros de compilação em `#[cfg(windows)]` corrigidos (E0433, E0507, E0308)
- **Sugestões de erro context-aware** — Sugestão de `WorkspaceJail` se adapta quando `--workspace` já foi fornecido
- **Gate de cross-compile** — `tests/cross_compile_check.rs` falha em qualquer regressão `E0433`, `E0308` ou `E0507` em blocos `cfg(windows)`


## O Que Houve De Novo Na v0.1.3
- Flag `--preserve-timestamps` em `edit` e `replace` para controlar o mtime do arquivo
- Campo `mtime_preserved` nas respostas NDJSON de `EditOutput` e `ReplaceResult`
- BREAKING: escrita atômica não preserva mais o mtime original por padrão


## Por Que
- Agentes LLM usam dezenas de comandos shell para manipular arquivos
- Uma única falha de energia ou crash no meio da escrita corrompe o arquivo
- Parsear saída não estruturada de CLI desperdiça tokens e causa alucinações
- Agentes precisam de checksums para detectar edições concorrentes mas raramente os calculam
- atomwrite resolve os quatro problemas com um único `cargo install`


## Superpoderes
### Escritas Atômicas
- Usa tempfile + fsync + rename + fsync do diretório em toda escrita
- Garante tudo-ou-nada: o arquivo nunca fica meio escrito
- Sobrevive a queda de energia, OOM kill e SIGKILL
- Verificação de sintaxe G72 REAL via tree-sitter opcional (flag `--syntax-check` em `write`)
- Sidecar WAL G114 opcional para recuperação de crash (`.atomwrite.journal.<target>.atomwrite.journal.json`)

### Saída NDJSON
- stdout é SEMPRE JSON estruturado, um objeto por linha
- Todo objeto carrega um campo discriminador `"type"`
- Agentes parseiam a saída sem regex ou scraping frágil de texto
- Erros também emitem JSON com `error: true` no stdout

### Checksums BLAKE3
- Toda resposta de `read` e `write` inclui um hash BLAKE3
- Use `--expect-checksum` para locking otimista em edições concorrentes
- Detecte drift de estado antes de aplicar mudanças

### Busca Paralela
- Construída sobre o motor do ripgrep para busca em conteúdo de arquivos
- Respeita `.gitignore` automaticamente
- Retorna matches estruturados com arquivo, linha, coluna e contexto

### Transformações por AST
- Busca e reescrita estrutural com ast-grep
- Cobre 306 linguagens de programação
- Refatore código pela árvore sintática, não por regex frágil
- Novos subcomandos `query` e `outline` caminham ASTs tree-sitter (305 linguagens) via `tree-sitter-language-pack`

### Scoping Gramatical
- Selecione categorias AST como comentários, funções, classes e strings
- Aplique ações: delete, uppercase, lowercase, titlecase, squeeze ou replace
- Cobre Rust, Python, JavaScript, TypeScript e Go com queries preparadas
- Use `--pattern` para padrões AST customizados além das queries embutidas

### Operações em Lote
- Execute operações de write, replace, delete, edit, hash, move e copy a partir de um manifesto NDJSON
- Use `--transaction` para execução tudo-ou-nada com rollback automático
- Todas as operações em um lote compartilham as mesmas garantias atômicas

### Edição Estruturada de Configuração (v0.1.12)
- `set` escreve um valor em um caminho dotted em arquivos TOML ou JSON (preserva comentários via `toml_edit`)
- `get` lê um valor em um caminho dotted com formato auto-detectado
- `del` remove uma chave (com `--force-missing` para tratar chaves ausentes como no-op success)
- `case` renomeia identificadores em múltiplos arquivos via `heck` (snake, camel, pascal, kebab, screaming-snake)


## Início Rápido
```bash
cargo install atomwrite

# Escrever arquivo atomicamente a partir do stdin
echo "hello world" | atomwrite --workspace . write src/hello.txt

# Ler de volta com checksum
atomwrite --workspace . read src/hello.txt

# Buscar em um diretório
atomwrite --workspace . search 'hello' src/

# Substituir texto com escritas atômicas
atomwrite --workspace . replace 'hello' 'world' src/

# Avaliar expressões matemáticas e conversões
atomwrite calc "2 hours + 30 minutes to seconds"

# v0.1.12: definir um valor em um arquivo TOML (preserva comentários)
atomwrite --workspace . set Cargo.toml package.version 0.2.0

# v0.1.12: caminhar o AST de um arquivo Rust
atomwrite --workspace . query src/main.rs --kinds

# v0.1.12: extrair outline de um arquivo Python
atomwrite --workspace . outline src/app.py

# v0.1.12: verificação de sintaxe tree-sitter REAL antes de commitar
echo "fn broken(" | atomwrite --workspace . write --syntax-check src/x.rs
```


## Instalação
### Do crates.io
```bash
cargo install atomwrite
```

### Da fonte
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
- Use `--workspace <DIR>` para restringir operações a uma raiz de projeto
- Use `ATOMWRITE_WORKSPACE` para definir a raiz do workspace via variável de ambiente
- Use `--dry-run` antes de operações destrutivas
- Use `--expect-checksum <HASH>` para locking otimista
- Use `--lang <LOCALE>` para substituir o idioma de exibição (en, pt-BR)
- Faça pipe de stdin para os comandos `write` e `batch`


## Comandos (28 no total)

### I/O Principal
- `read` — ler arquivos com metadados, checksum, conteúdo opcional
- `write` — criar ou sobrescrever arquivos atomicamente via stdin
- `edit` — editar cirurgicamente por número de linha, marcador de texto ou match exato
- `delete` — deletar arquivos com backup opcional
- `copy` — copiar arquivos com verificação de checksum
- `move` — mover ou renomear arquivos atomicamente (EXDEV copy-fallback)
- `apply` — aplicar patches do stdin (unified diff, search/replace, full, markdown)

### Busca e substituição
- `search` — buscar conteúdo de arquivos em paralelo (motor ripgrep)
- `replace` — substituir texto entre arquivos com escritas atômicas
- `transform` — refatoração AST via ast-grep (306 linguagens)

### Inspeção
- `hash` — calcular checksums BLAKE3
- `count` — contar linhas, arquivos por extensão
- `diff` — comparar dois arquivos (unified, stat ou mudanças)
- `list` — listar arquivos em uma árvore de diretórios
- `extract` — extrair campos de entrada NDJSON via pipe
- `scope` — scoping gramatical (deletar todos os comentários, etc.)
- `regex` — gerar regex a partir de exemplos
- `calc` — matemática e conversões de unidades
- `completions` — gerar completions de shell (bash, zsh, fish, elvish, powershell)

### Backup e recuperação
- `backup` — criar backups com timestamp com checksums BLAKE3
- `rollback` — restaurar de um backup anterior
- `batch` — operações em lote dirigidas por NDJSON (transacional)

### Editores estruturados de configuração (v0.1.12, v14 Tier 3)
- `set <PATH> <KEY_PATH> <VALUE>` — escrever um valor em um caminho dotted em um arquivo TOML ou JSON. Preserva comentários e ordem das chaves via `toml_edit`. Auto-coage int/bool/float/string.
- `get <PATH> <KEY_PATH>` — ler um valor em um caminho dotted. NDJSON: `{"type":"get","key_path","value","found","format"}`.
- `del <PATH> <KEY_PATH>` — remover uma chave. Flag `--force-missing` trata chaves ausentes como no-op success.
- `case <PATHS...> --subvert OLD NEW --to <style>` — renomear identificadores em múltiplos arquivos via `heck`. Estilos: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`.

### Ferramentas AST (v0.1.12, v14 Tier 3 + G72, via tree-sitter-language-pack)
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` — caminhar um AST tree-sitter e emitir nós como NDJSON. 305 linguagens suportadas.
- `outline <PATH> [--kind <KIND>] [--positions]` — extrair estrutura de alto nível (funções, classes, structs, enums, traits, módulos) como NDJSON.
- `write --syntax-check` — verificação de sintaxe G72 REAL via tree-sitter. 24 linguagens cobertas. Exit 88 com primeira linha/coluna/kind/mensagem de erro.

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
echo "novo conteudo" | atomwrite --workspace . apply src/file.txt --format full

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

# completions
atomwrite completions bash
```


## Variáveis de Ambiente
- `NO_COLOR`: desabilita saída colorida quando definida com qualquer valor
- `RUST_LOG`: controla verbosidade dos logs (ex: `RUST_LOG=debug`)
- `ATOMWRITE_LANG`: substitui o locale para mensagens traduzidas (ex: `en`, `pt-BR`)
- `ATOMWRITE_WORKSPACE`: define a raiz do workspace para validação de path jail (alternativa a `--workspace`)
- `RAYON_NUM_THREADS`: substitui número de threads paralelas para search, replace, transform e scope


## Códigos de Saída
- `0`: sucesso
- `1`: nenhum match encontrado (search, não é um erro)
- `4`: arquivo não encontrado
- `13`: permissão negada
- `28`: disco cheio (sem espaço restante no dispositivo)
- `30`: cota excedida
- `65`: entrada inválida (argumentos incorretos ou dados malformados)
- `73`: rename entre dispositivos (fronteira de filesystem)
- `74`: erro de I/O
- `78`: configuração inválida
- `81`: verificação de checksum falhou
- `82`: drift de estado (checksum não confere, lock otimista falhou)
- `83`: timeout de lock (v0.1.12+)
- `85`: FIFO detectado (named pipe não pode ser escrito atomicamente)
- `86`: arquivo de dispositivo detectado (bloco ou caractere)
- `88`: erro de sintaxe detectado (v0.1.12+, `--syntax-check` falhou)
- `91`: fallback EXDEV desabilitado (v0.1.12+, `--strict-atomic` proíbe copy-fallback entre devices)
- `92`: verificação BLAKE3 do copy-back falhou (v0.1.12+)
- `93`: journal órfão detectado (v0.1.12+, recuperação consultiva G114)
- `126`: jail de workspace violada (caminho escapa do workspace)
- `127`: symlink bloqueado (alvo do symlink fora do workspace)
- `128`: arquivo imutável (não pode modificar)
- `130`: interrompido por SIGINT
- `141`: pipe quebrado (SIGPIPE)
- `143`: terminado por SIGTERM
- `255`: erro interno


## Tratamento de Erros
- Todos os erros emitem um objeto JSON no stdout com `error: true`
- Campos do erro: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`
- Classes de erro: `permanent`, `transient`, `conflict`, `precondition_failed`
- 25 variantes de erro no total (20 baseline de v0.1.4 + 5 adicionadas em v0.1.12)
- Erros transient e conflict definem `retryable: true`
- O campo `suggestion` fornece orientação de recuperação acionável para agentes
- Veja `docs/schemas/error-output.schema.json` para o contrato completo


## Performance
- Binário estático único sem dependências de runtime
- Builds release usam LTO, codegen unit único e stripping de símbolos
- Leitura de arquivos via memory-map com `memmap2` para arquivos grandes
- Busca paralela via rayon e o motor ripgrep
- Latência típica de operação de arquivo: abaixo de 5 ms para arquivos pequenos


## FAQ de Solução de Problemas

### atomwrite write trava sem saída
- Certifique-se de estar fazendo pipe de conteúdo para stdin
- `write` lê do stdin e aguarda EOF
- Exemplo: `echo "content" | atomwrite --workspace . write file.txt`

### search retorna código de saída 1
- Código de saída 1 significa zero matches encontrados
- Este é o comportamento esperado, não um erro
- Verifique o padrão e o caminho alvo

### rename entre dispositivos falha com exit 73
- A origem e o destino estão em filesystems diferentes
- atomwrite faz fallback para copy+delete no `move` entre dispositivos
- Use `copy` seguido de `delete` como alternativa

### checksum não confere com exit 82
- Outro processo modificou o arquivo entre read e write
- Releia o arquivo para obter o checksum atual
- Repita a operação com o `--expect-checksum` atualizado

### jail de workspace violada com exit 126
- O caminho alvo resolve para fora do limite do `--workspace`
- Verifique se o caminho não contém travessias `..` ou symlinks escapando do workspace

### verificação de sintaxe falhou com exit 88 (v0.1.12+)
- A verificação G72 REAL tree-sitter encontrou um erro de sintaxe no arquivo
- Inspecione a primeira linha/coluna/kind/mensagem de erro no envelope JSON de erro
- Corrija a sintaxe e repita, ou remova `--syntax-check` para contornar


## Arquitetura
- Veja [ARCHITECTURE.pt-BR.md](ARCHITECTURE.pt-BR.md) para mapa de módulos, fluxo de dados e decisões de projeto
- Veja [docs/decisions/](docs/decisions/README.md) para 7 ADRs cobrindo a arquitetura v0.1.12 (G72, G114, v14 Tier 3)
- Veja [docs/schemas/](docs/schemas/README.md) para 22 contratos estáveis de JSON Schema para toda saída NDJSON


## Contribuindo
- Veja [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) para setup de desenvolvimento e diretrizes
- Veja [docs/decisions/README.md](docs/decisions/README.md) para o log de decisões


## Segurança
- Veja [SECURITY.pt-BR.md](SECURITY.pt-BR.md) para reporte de vulnerabilidades
- Veja a seção "Known Security Advisories" do SECURITY.pt-BR.md para advisories resolvidas e ativas


## Changelog
- Veja [CHANGELOG.md](CHANGELOG.md) para histórico de releases em inglês
- Veja [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) para histórico de releases em português


## Licença
- Licenciado sob MIT OR Apache-2.0
- Veja [LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE) para detalhes
