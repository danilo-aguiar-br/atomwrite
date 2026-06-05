[Read in English](README.md)


# atomwrite

> Operações atômicas de arquivo para agentes LLM -- um CLI, zero corrupção

[![Crates.io](https://img.shields.io/crates/v/atomwrite)](https://crates.io/crates/atomwrite)
[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)
[![License](https://img.shields.io/crates/l/atomwrite)](LICENSE)
[![CI](https://github.com/daniloaguiarbr/atomwrite/actions/workflows/ci.yml/badge.svg)](https://github.com/daniloaguiarbr/atomwrite/actions)


## O Que É
- Um único binário Rust que resolve toda operação de arquivo que um agente LLM precisa
- Ler, escrever, editar, buscar, substituir, diff, copiar, mover, deletar, transformar, scoping, backup, rollback, apply -- tudo em uma ferramenta
- Toda escrita é atômica: tempfile, fsync, rename, fsync do diretório
- Toda resposta é NDJSON: um objeto JSON por linha, legível por máquina
- Todo arquivo recebe checksum BLAKE3: detecta drift, verifica integridade, habilita locking otimista


## O Que Há De Novo Na v0.1.8 (Release Pendente)
- **`signal_test::shutdown_message_on_stderr` não falha mais no CI Linux (ubuntu-latest)** — Removido `eprintln!` dos handlers de SIGINT e SIGTERM porque POSIX.1-2017 `signal-safety(7)` proíbe explicitamente funções stdio em signal handlers. O `std::io::stderr()` do Rust usa um `Mutex` global que pode causar deadlock ou perder output bufferizado quando o sinal chega enquanto outra thread segura o lock. A mensagem de shutdown agora é emitida pela main thread em `src/main.rs` quando observa `is_shutdown() == true` após `atomwrite::run` retornar, que é a única forma async-signal-safe de garantir que a mensagem chegue ao pipe de stderr capturado antes do processo terminar. O caminho Windows `ctrlc` ainda emite a mensagem inline porque handlers ctrlc rodam em thread normal.
- **`atomic::tests::create_backup_and_retention` não falha mais no CI Windows (windows-latest)** — Adicionado `platform::fsync_file_best_effort` que registra warning e continua. No Windows, produtos antivírus (Windows Defender, AVs terceiros) seguram transientemente um handle de leitura em arquivos em `%TEMP%` com `FILE_SHARE_READ` mas sem `FILE_SHARE_WRITE`, o que faz `FlushFileBuffers` retornar `ERROR_ACCESS_DENIED` (os error 5). Apenas o fsync de durabilidade do backup é best-effort; o caminho de escrita principal ainda usa o `fsync_file` estrito porque o backup em si já foi criado via `fs::copy` quando o fsync executa.
- **Matriz CI fixada em `windows-2025-vs2026`** — Substituído `windows-latest` por `windows-2025-vs2026` (seu sucessor antes da migração de runners hospedados no GitHub em 15 de junho de 2026). Silencia o NOTICE "windows-latest requests are being redirected" e previne mudanças inesperadas de runner que possam quebrar o build.

Veja o guia de migração v0.1.7 → v0.1.8 abaixo para o caminho de upgrade. v0.1.7 é a release anterior.

## O Que Houve De Novo Na v0.1.7
- **CI do GitHub Actions 100% verde** — Todos os 6 jobs de CI (check matrix x3, deny, doc, msrv, security) passam após corrigir 4 falhas distintas. macos-latest não aborta mais em `clippy::needless_return` (removido `return` redundante em `src/platform.rs:31`); windows-latest não aborta mais em `dead_code` (adicionado `#[cfg_attr(not(unix), allow(dead_code))]` em `EXIT_SIGINT` e `EXIT_SIGTERM` em `src/signal.rs:15-18`); `cargo audit` não reporta mais RUSTSEC-2026-0009 (atualizado `time` 0.3.45 → 0.3.47 com `DEPTH_LIMIT=32`); deny.toml não precisa mais da entrada `ignore` para RUSTSEC-2026-0009. Tanto o `ignore` do deny.toml quanto a flag `cargo audit --ignore` foram removidos pois a advisory não se aplica mais.
- **MSRV bumped para 1.88** — `rust-version` no `Cargo.toml` atualizado de 1.85 para 1.88. Necessário para permitir `time` 0.3.47 que precisa de features edition2024 introduzidas em 1.85+ e um bump interno de layout. Todos os 12 arquivos bilíngues de documentação atualizados (INSTALL, HOW_TO_USE, CROSS_PLATFORM, COOKBOOK, CONTRIBUTING, mais variantes PT-BR).
- **GitHub Actions pronto para Node 24** — `actions/checkout` bumped de `@v4` para `@v6` e `actions/cache` de `@v4` para `@v5` em ambos `ci.yml` e `bench.yml`. `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: "true"` adicionado ao env do workflow como cinto-e-suspensórios. Sem mais warnings de deprecação de Node 20 nos logs de CI.
- **Cross-compile Windows validado localmente em macOS** — x86_64-pc-windows-gnu e i686-pc-windows-gnu passam em `cargo check`, `cargo build`, `cargo clippy --all-targets --all-features -- -D warnings`, e `cargo test --no-run`. Binário PE32+ de 203MB gerado em macOS via MinGW-w64 14.0.0. Documentação completa na memory `atomwrite-windows-cross-compile-validated-2026-06-05`.
- **Snapshot test platform-aware** — `tests/snapshot_write.rs` e o arquivo `.snap` correspondente usam placeholder `[platform_fsync]` para o campo `platform.fsync`, permitindo que o mesmo snapshot valide em Linux (`sync_data`), macOS (`F_FULLFSYNC`), e Windows.
- **`signal_test::shutdown_message_on_stderr` não falha mais em macOS** — Substituída a chamada `libc::write(2, SHUTDOWN_MSG.as_ptr().cast(), ...)` nos handlers de SIGINT e SIGTERM por `eprintln!`. O stderr do runtime Rust é capturado de forma confiável pelo `Stdio::piped()` no processo de teste, enquanto writes brutos via libc eram perdidos na herança de process group do cargo test. A constante `SHUTDOWN_MSG` foi removida por ser dead code. O teste estava anteriormente marcado `#[ignore]` (GAP 16) e agora está totalmente ativo.

## O Que Há De Novo Na v0.1.4
- **`cargo install atomwrite` funciona no Windows 10/11** — Três erros de compilação em blocos `#[cfg(windows)]` que quebravam a release v0.1.3 no Windows estão corrigidos: E0433 em `src/atomic.rs:404` (falta de import de `AtomwriteError`), E0507 em `src/atomic.rs:387` (`persist_with_retry` agora recebe `NamedTempFile` por valor), e E0308 em `src/platform.rs:116` (raw pointer `handle` agora comparado com `!handle.is_null()` em vez de literal `0`).
- **Sugestões de erro context-aware** — A sugestão de `WorkspaceJail` agora se adapta: quando o usuário já forneceu `--workspace` (ou `ATOMWRITE_WORKSPACE`), a sugestão diz "use a path inside the workspace" em vez de re-pedir a flag. Todas as 20 variants de erro agora carregam texto `suggestion` acionável (anteriormente 6 não tinham sugestão). A referência phantom à flag `--force-text` foi removida.
- **Novo struct `ErrorContext`** — `ErrorJson::from_error_with_context()` e `output::write_error_json_with_context()` propagam proveniência de workspace do parser CLI até o output NDJSON para que sugestões permaneçam precisas. A versão legacy `from_error()` é preservada para compatibilidade.
- **Gate de cross-compile** — Novo `tests/cross_compile_check.rs` executa `cargo check` contra `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, e `x86_64-pc-windows-msvc`. O gate falha em qualquer regressão de `E0433`, `E0308`, ou `E0507` em blocos `cfg(windows)`. Testes são `#[ignore]` por padrão; execute com `cargo test --test cross_compile_check -- --ignored` antes de qualquer release que toque código Windows-only.
- **Guia de instalação Windows** — Novos `docs/INSTALL.md` (inglês) e `docs/INSTALL.pt-BR.md` (português) cobrem pré-requisitos do Windows 10/11 (Visual Studio Build Tools, Rust 1.88+, Windows Terminal), comandos `cargo install`, e troubleshooting.

Veja o guia de migração v0.1.3 → v0.1.4 em `docs/MIGRATION.pt-BR.md` para o caminho de upgrade. v0.1.3 foi a release anterior.

## O Que Houve De Novo Na v0.1.3
- Flag `--preserve-timestamps` em `edit` e `replace` para controlar o mtime do arquivo (padrão: mtime é atualizado para refletir a mudança)
- Campo `mtime_preserved` nas respostas NDJSON de `EditOutput` e `ReplaceResult` para visibilidade diagnóstica
- BREAKING: escrita atômica não preserva mais o mtime original do arquivo por padrão. Isso corrige um no-op silencioso em `cargo build` / `make` / `cmake` / `gradle` que ocorria quando o arquivo fonte parecia mais antigo que o binário. Veja o guia de migração v0.1.2 → v0.1.3 em `docs/MIGRATION.pt-BR.md`


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

### Scoping Gramatical
- Selecione categorias AST como comentários, funções, classes e strings
- Aplique ações: delete, uppercase, lowercase, titlecase, squeeze ou replace
- Cobre Rust, Python, JavaScript, TypeScript e Go com queries preparadas
- Use `--pattern` para padrões AST customizados além das queries embutidas

### Operações em Lote
- Execute operações de write, replace, delete, edit, hash, move e copy a partir de um manifesto NDJSON
- Use `--transaction` para execução tudo-ou-nada com rollback automático
- Todas as operações em um lote compartilham as mesmas garantias atômicas
- Use `backup` e `rollback` para fluxos manuais de snapshot e restauração
- Uma chamada CLI substitui centenas de invocações individuais


## Início Rápido
```bash
cargo install atomwrite

# Escrever arquivo atomicamente via stdin
echo "hello world" | atomwrite write src/hello.txt

# Ler com checksum
atomwrite read src/hello.txt

# Buscar em um diretório
atomwrite search 'hello' src/

# Substituir texto com escritas atômicas
atomwrite replace 'hello' 'world' src/

# Avaliar expressões matemáticas e conversões de unidade
atomwrite calc "2 hours + 30 minutes to seconds"
```


## Instalação
### Pelo crates.io
```bash
cargo install atomwrite
```

### A partir do código-fonte
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
- Use `--dry-run` antes de operações destrutivas
- Use `--expect-checksum <HASH>` para locking otimista
- Use `--lang <LOCALE>` para substituir o idioma de exibição (en, pt-BR)
- Pipe stdin para os comandos `write` e `batch`


## Comandos

### read
- Lê um ou mais arquivos com metadados, tamanho, permissões e checksum BLAKE3
- Use `--stat` para retornar apenas metadados sem conteúdo
```bash
atomwrite read src/main.rs
```

### write
- Cria ou sobrescreve um arquivo atomicamente a partir do stdin
- Retorna o checksum BLAKE3 do conteúdo escrito
- Use `--line-ending lf|crlf|cr|auto` para normalizar line endings (padrão: auto preserva o original)
```bash
echo "fn main() {}" | atomwrite write src/main.rs
```

### edit
- Edita cirurgicamente um arquivo por número de linha, marcador de texto ou match exato
- Suporta operações de inserção, substituição e deleção
- Use `--expect-checksum` para prevenir conflitos de edição concorrente
- Use `--fuzzy auto|off|aggressive` para matching fuzzy de texto
- Use `--line-ending lf|crlf|cr|auto` para normalizar line endings
```bash
echo "new content" | atomwrite edit src/main.rs --after-line 5
```

### search
- Busca conteúdo de arquivos em paralelo usando o motor ripgrep
- Retorna matches estruturados com arquivo, linha, coluna e contexto
- Sai com código 1 quando zero matches são encontrados (não é um erro)
```bash
atomwrite search 'TODO' src/ --include '*.rs'
```

### replace
- Substitui texto em arquivos com escritas atômicas
- Suporta padrões regex e strings literais
- Use `--dry-run` para pré-visualizar mudanças
```bash
atomwrite replace 'old_name' 'new_name' src/ --include '*.rs'
```

### hash
- Calcula checksums BLAKE3 para um ou mais arquivos
```bash
atomwrite hash src/main.rs src/lib.rs
```

### delete
- Deleta arquivos com backup opcional antes da remoção
- Use `--backup` para criar uma cópia `.bak` antes
```bash
atomwrite delete src/temp.rs --backup
```

### count
- Conta linhas em arquivos ou conta arquivos por extensão em um diretório
```bash
atomwrite count src/ --by-extension
```

### diff
- Compara dois arquivos com saída unified, stat ou apenas mudanças
```bash
atomwrite diff src/old.rs src/new.rs --unified
```

### move
- Move ou renomeia arquivos atomicamente
- Faz fallback para copy+delete em movimentações entre dispositivos
```bash
atomwrite move src/old.rs src/new.rs
```

### copy
- Copia arquivos com verificação de checksum BLAKE3 após a cópia
```bash
atomwrite copy src/template.rs src/new_module.rs
```

### list
- Lista estrutura de arquivos do projeto com metadados
- Respeita `.gitignore` por padrão
```bash
atomwrite list src/ --depth 2
```

### extract
- Extrai campos de entrada NDJSON ou colunas de texto do stdin
```bash
atomwrite search 'TODO' src/ | atomwrite extract path line
```

### calc
- Avalia expressões matemáticas e conversões de unidade
- Usa fend para aritmética de precisão arbitrária
```bash
atomwrite calc "2 GiB to bytes"
atomwrite calc "sqrt(144) + 3^2"
```

### regex
- Gera padrões regex a partir de strings de exemplo
- Usa grex para inferência automática
```bash
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"
```

### transform
- Busca e reescrita estrutural por AST com ast-grep
- Cobre 306 linguagens de programação
- Ambos `--pattern` e `--rewrite` são obrigatórios
```bash
atomwrite transform -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/
atomwrite transform -p 'console.log($$$ARGS)' -r 'logger.info($$$ARGS)' -l js src/
```

### scope
- Scoping gramatical: seleciona categorias AST e aplica ações
- Use `--query` para queries preparadas (fn, comments, strings, struct, etc.)
- Use `--pattern` para padrões AST customizados
- Use `--delete` para remover conteúdo ou `--action upper|lower|titlecase|squeeze`
- Cobre Rust, Python, JavaScript, TypeScript e Go
```bash
atomwrite scope src/ --lang rust --query comments --delete
atomwrite scope src/ --lang rust --query fn --action upper --dry-run
```

### backup
- Cria backups com timestamp de arquivos com checksums BLAKE3
- Use `--retention N` para controlar quantos backups manter
```bash
atomwrite backup src/config.toml
atomwrite backup src/main.rs src/lib.rs --retention 3
```

### rollback
- Restaura um arquivo a partir de um backup anterior
- Use `--verify` para verificar checksum BLAKE3 após restauração
```bash
atomwrite rollback src/config.toml
atomwrite rollback src/config.toml --timestamp 20260530_120000
```

### apply
- Aplica um patch do stdin (unified diff, blocos SEARCH/REPLACE ou substituição completa)
- Detecta formato automaticamente ou use `--format` para especificar
```bash
echo "novo conteudo" | atomwrite apply src/file.txt --format full
git diff src/file.txt | atomwrite apply src/file.txt --format unified
```

### batch
- Executa múltiplas operações a partir de um manifesto NDJSON no stdin
- Suporta operações de write, replace, delete, edit, hash, move e copy
- Use `--transaction` para execução tudo-ou-nada com rollback automático
```bash
cat manifest.ndjson | atomwrite batch
cat manifest.ndjson | atomwrite batch --transaction
```

### completions
- Gera scripts de completion de shell para bash, zsh, fish, elvish ou PowerShell
```bash
atomwrite completions bash
```


## Variáveis de Ambiente
- `NO_COLOR`: desabilita saída colorida quando definida com qualquer valor
- `RUST_LOG`: controla verbosidade dos logs (ex: `RUST_LOG=debug`)
- `ATOMWRITE_LANG`: substitui o locale para mensagens traduzidas (ex: `en`, `pt-BR`)
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
- `82`: drift de estado (checksum não confere, lock otimista falhou)
- `85`: FIFO detectado (named pipe não pode ser escrito atomicamente)
- `86`: arquivo de dispositivo detectado (bloco ou caractere)
- `126`: jail de workspace violada (caminho escapa do workspace)
- `127`: symlink bloqueado (alvo do symlink fora do workspace)
- `128`: arquivo imutável (não pode modificar)
- `130`: interrompido por SIGINT
- `141`: pipe quebrado (SIGPIPE)
- `143`: terminado por SIGTERM
- `255`: erro interno


## Tratamento de Erros
- Todos os erros emitem um objeto JSON no stdout com `error: true`
- Campos do erro: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`
- Classes de erro: `permanent`, `transient`, `conflict`, `precondition_failed`
- Erros transient e conflict definem `retryable: true`
- O campo `suggestion` fornece orientação de recuperação acionável para agentes


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
- Exemplo: `echo "content" | atomwrite write file.txt`

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


## Arquitetura
- Veja [ARCHITECTURE.pt-BR.md](ARCHITECTURE.pt-BR.md) para mapa de módulos, fluxo de dados e decisões de projeto


## Contribuindo
- Veja [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) para setup de desenvolvimento e diretrizes


## Segurança
- Veja [SECURITY.pt-BR.md](SECURITY.pt-BR.md) para reporte de vulnerabilidades


## Changelog
- Veja [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) para histórico de releases


## Licença
- Licenciado sob MIT OR Apache-2.0
- Veja [LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE) para detalhes
