# atomwrite Suporte Multiplataforma


[Read in English](CROSS_PLATFORM.md)

- Um só contrato de CLI em Linux, macOS e Windows; só o Linux x86_64 está validado (veja a Matriz de Verificação)


## Matriz de Verificação (v0.1.37)

| Target | Status |
|--------|--------|
| `x86_64-unknown-linux-gnu` | Validado: compilado e testado de ponta a ponta (131 alvos de teste, 1486 testes passaram, 0 falharam, 9 ignorados, conforme registrado no `CHANGELOG.md`) |
| `x86_64-unknown-linux-musl` | Compila: `cross_compile_linux_musl_x64_succeeds` passou em 2026-10-31 com `cargo check --lib`; nenhum binário foi executado |
| `aarch64-unknown-linux-gnu` | Compila: `cross_compile_linux_aarch64_gnu_succeeds` passou em 2026-10-31 com `cargo check --lib`; nenhum binário foi executado |
| `aarch64-unknown-linux-musl` | Compila: `cross_compile_linux_aarch64_musl_succeeds` passou em 2026-10-31 com `cargo check --lib`; nenhum binário foi executado |
| `x86_64-apple-darwin` | NÃO validado: o alvo Apple está instalado mas falta a toolchain C, e o `cc-rs` falha ao compilar `libmimalloc-sys` e `zstd-sys` com `-arch arm64` |
| `aarch64-apple-darwin` | NÃO validado: o alvo Apple está instalado mas falta a toolchain C, e o `cc-rs` falha ao compilar `libmimalloc-sys` e `zstd-sys` com `-arch arm64` |
| `x86_64-pc-windows-msvc` | `cross_compile_windows_msvc_succeeds` existe; o alvo não está instalado neste host, então NÃO validado |
| `aarch64-pc-windows-msvc` | `cross_compile_windows_arm64_msvc_succeeds` existe; o alvo não está instalado neste host, então NÃO validado |
| `x86_64-pc-windows-gnu` | Compila: `cross_compile_windows_gnu_x64_succeeds` passou em 2026-10-31, o que também prova que os caminhos `cfg(windows)` compilam; nenhum binário foi executado |
| `i686-pc-windows-gnu` | `cross_compile_windows_gnu_i686_succeeds` existe; o alvo não está instalado neste host, então NÃO validado |
| Automação remota | Não embarcada neste repositório (somente gates locais) |

- Quatro dos nove testes `cross_compile_*` passaram em 2026-10-31 (linux musl x64, linux aarch64 gnu, linux aarch64 musl e windows gnu x64), os dois da Apple pararam na toolchain C, e os três MSVC ou i686 exigem alvos que este host não tem
- Tamanho do binário, medido em builds release de `x86_64-unknown-linux-gnu` em 2026-10-31: slim 10,40 MiB e default 54,45 MiB (veja `docs/INSTALL.pt-BR.md`). Os outros targets não foram medidos
- O tempo de inicialização por target não foi medido
- Também não validados: o lock advisory de escrita em filesystems sem `flock` (NFS) e no Windows, onde o código agora compila mas nunca rodou, o retry de rename do `persist_retry.rs` no Windows, e todo caminho de Windows e macOS deste guia
- A feature `ast` baixa os parsers tree-sitter em tempo de execução no primeiro uso, em toda plataforma (23.031.807 bytes medidos em Linux x86_64, o que dá 21,97 MiB; veja `docs/INSTALL.pt-BR.md`)

Nota para todas as plataformas: o jail do workspace para caminhos de content-source (`--old-file`/`--new-file`) é agnóstico de SO. Faça staging em `$WS/.atomwrite/scratch/` via `scratch init`, e nunca confie no `/tmp` do SO fora do `--workspace`. O shape agent-native de stdout (`--select`/`--filter`/`--limit`) e os envelopes de erro (`ok:false`) são idênticos entre plataformas por projeto, e foram exercitados só em Linux.


## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças relevantes para cross-platform em v0.1.12.

### Tratamento de Sinais (Melhorado)
- v0.1.12 adiciona 5 novos testes em `tests/signal_test.rs` cobrindo SIGINT, SIGTERM, SIGPIPE, interrupção de batch
- `tests/signal_test.rs::batch_interrupted_by_signal` valida cleanup do journal WAL em sinal
- `tests/signal_test.rs::sigpipe_handler_ends_the_run_with_exit_141` confirma que um pipe fechado encerra a execução com exit 141 (o teste se chamava `sigpipe_exits_141_or_signal_13` na v0.1.12)
- `tests/signal_test.rs::sigint_during_search_exits_130` e `sigterm_during_search_exits_143` confirmam exit codes limpos
- `tests/signal_test.rs::shutdown_message_on_stderr` valida tracing em shutdown

### Windows
- v0.1.12 preserva o fix do Windows 10/11 da v0.1.4: `cargo install atomwrite` funciona
- Melhorias no `init_console`: UTF-8 code page 65001 + `ENABLE_VIRTUAL_TERMINAL_PROCESSING`
- `persist_with_retry` lida com `PermissionDenied` durante rename atômico com retries (a v0.1.12 usava backoff exponencial; veja as notas da v0.1.37 para a política atual)
- A v0.1.12 acrescentou 5 códigos de erro (83, 88, 91, 92, 93) com mensagens bilíngues, e nenhum deles é exclusivo do Windows

### Linux
- v0.1.12 exigia Rust 1.88 ou posterior (histórico). O mínimo atual é o Rust 1.90
- FIFO/named pipes são pulados por padrão (G56) para prevenir travamento; `search --include-fifo` os abre de novo, e `write` num FIFO sai 85 `FIFO_DETECTED`
- O fallback de cópia em EXDEV (G90) é desligado por `--strict-atomic` num verbo que escreve, que liga a opção de biblioteca `WriteOptions.strict_atomic`; a escrita então sai 91 em vez de copiar entre sistemas de arquivos
- Lock advisory de arquivo via `flock` funciona no Linux (G54)
- Preservação de xattr funciona em ext4, btrfs, XFS, F2FS (G39)

### macOS
- v0.1.12 preserva os fixes de build do macOS arm64 (Apple Silicon) e macOS x86_64 da v0.1.2
- Reflink CoW funciona em APFS (G64): backup e copy O(1)
- Preservação de xattr funciona para `com.apple.quarantine`, `kMDItemUserTags`, `kMDItemFinderComment` (G39)
- Gatekeeper pode exigir `xattr -d com.apple.quarantine` no primeiro uso

### Containers (Docker, Kubernetes)
- Fallback EXDEV (G90) lida com Docker overlay2 + named volumes automaticamente
- O exit code 91 (`ExdevFallbackDisabled`) pertence ao opt-out do fallback que existe só na biblioteca e não é alcançável pela linha de comando
- Sem mudanças de código necessárias para usuários de container; funciona out of the box

### NFS
- `flock(2)` é silenciosamente ignorado em NFS, então o lock automático de escrita pode não proteger contra edições concorrentes
- Combine o lock automático com `--expect-checksum` para defesa em profundidade
- `--expect-checksum` detecta desvio de estado após escrita (exit 82)

### Cobertura de Testes
- Execução final da v0.1.37 registrada no `CHANGELOG.md`: 131 alvos de teste, 1486 testes passaram, 0 falharam, 9 ignorados (os testes `cross_compile_*`)
- Gate de cross-compile: `cargo test --test cross_compile_check -- --ignored` roda nove checagens de target; nenhuma foi executada na v0.1.37 (veja a Matriz de Verificação)
- 5 testes de sinal em `tests/signal_test.rs` cobrem SIGINT/SIGTERM/SIGPIPE/batch/shutdown
- Veja [docs/decisions/README.md](decisions/README.md) para decisões arquiteturais
- Para durabilidade e rename na v0.1.29, veja [v0.1.29 — Durabilidade e rename](#v0129--durabilidade-e-rename)
- Para honestidade de backup e residual de agente na v0.1.30, veja [v0.1.30 — Backup e residual de agente](#v0130--backup-e-residual-de-agente)
- Para fuzzy one-shot + timeout na v0.1.34, veja [v0.1.34 — Fuzzy one-shot e timeout](#v0134--fuzzy-one-shot-e-timeout)


## A Dor Que Você Já Conhece
- Você escreve um arquivo no Linux e ele chega ao disco de forma confiável
- Você escreve o mesmo arquivo no macOS e o `fsync` mente silenciosamente sobre durabilidade
- Você escreve no Windows e fsync de diretório nem é um conceito
- Seu agente não sabe em qual plataforma está rodando
- atomwrite cuida de tudo isso para você de forma transparente


## Matriz de Suporte
### Linux (Suporte Completo)
- Fsync de arquivo: `fdatasync` via `sync_data()`
- Fsync de diretório: `sync_all()` no diretório pai
- Rename atômico: prefere `renameat2` com fallback para `rename` em `ENOSYS`
- O NDJSON do write reporta `platform.rename_method` (`renameat2` ou `rename`)
- Cross-device: fallback automático para cópia-depois-deleta
- Testado só em x86_64; o target aarch64 tem um teste de cross-compile que não foi executado

### macOS (suporte no fonte, não validado)
- Fsync de arquivo: `F_FULLFSYNC` via `fcntl` para durabilidade real
- Fsync de diretório: `sync_all()` no diretório pai
- O `fsync` padrão no macOS NÃO garante escrita no disco
- atomwrite usa `F_FULLFSYNC` automaticamente
- Status na v0.1.37: não executado no macOS, nem em Apple Silicon nem em Intel; as linhas acima descrevem o comportamento do fonte

### Windows (suporte no fonte, não validado)
- Fsync de arquivo: `FlushFileBuffers` via `sync_all()`
- Fsync de diretório: best-effort (Windows não tem `fsync` para diretórios)
- Correção v0.1.15 (GAP 18): o teste de snapshot do write redige `platform.dir_fsync` como `[platform_dir_fsync]` porque o Windows reporta `best_effort` enquanto o Unix reporta `sync_all` (histórico, não executado de novo no Windows)
- Rename atômico via `MoveFileExW` com `MOVEFILE_REPLACE_EXISTING`
- NTFS fornece garantias razoáveis de durabilidade
- Fix v0.1.4 (GAP 14): `cargo install atomwrite` passou a funcionar no Windows 10/11 depois que três erros de compilação em blocos `#[cfg(windows)]` foram resolvidos (histórico, não executado de novo na v0.1.37)
- Adição v0.1.4: `init_console` ativa UTF-8 (code page 65001) e `ENABLE_VIRTUAL_TERMINAL_PROCESSING` para que sequências ANSI sejam interpretadas pelo Windows Console Host (não executado de novo na v0.1.37).
- Política de retry atual (v0.1.37): o persist do tempfile repete `PermissionDenied` e `ERROR_SHARING_VIOLATION` (32) com full jitter (base 50 ms, teto 400 ms, as chaves `retry.*` do `atomwrite config`) e continua obedecendo `--timeout-secs`. Isso compensa processos antivírus que seguram o arquivo por pouco tempo. Na v0.1.4 as esperas eram fixas em 100 ms, 200 ms e 400 ms
- Status na v0.1.37: não executado no Windows (nem x86_64 nem i686); o retry só foi conferido compilando uma cópia do corpo dele em Linux


## Tratamento de Sinais
### Linux e macOS
- SIGINT (130): shutdown gracioso, faz flush de escritas pendentes
- SIGTERM (143): shutdown gracioso, faz flush de escritas pendentes
- SIGHUP (129): terminal fechado encerra a execução como cancelamento, com `signal: "SIGHUP"` no envelope `cancelled`
- `--timeout-secs` esgotado (124): o envelope `cancelled` carrega `deadline_exceeded: true`
- SIGPIPE (141): o SIGPIPE NÃO volta a `SIG_DFL`. O processo instala um handler cooperativo que marca um shutdown com exit 141 (desvio consciente da regra literal, ADR-0063). A escrita que acerta o pipe fechado falha com `BrokenPipe`, todo `Drop` roda e nenhum lock, journal ou tempfile fica. Uma escrita em andamento termina e confirma
- Medido em Linux com o build padrão: `read big.txt | head -c 5` e `search PADRAO big.txt | head -1` saem 141

### Windows
- Ctrl+C: tratado via SetConsoleCtrlHandler
- SIGPIPE: não aplicável no Windows. O `std` mapeia `ERROR_BROKEN_PIPE` para `BrokenPipe`, que segue o mesmo caminho até o exit 141 (não validado no Windows)
- Terminação de processo: escritas atômicas pendentes são abandonadas com segurança


## Containers
- Docker: funciona imediatamente com imagens Linux padrão
- Overlay filesystems: rename atômico funciona dentro das camadas overlay
- Volume mounts: fsync alcança o filesystem do host através do mount
- tmpfs: fsync é no-op mas rename ainda é atômico
- Moves entre containers: use `--workspace` para evitar escapar do mount
- Status na v0.1.37: nenhuma execução em container foi registrada; os itens acima descrevem o comportamento do fonte
- Autodetecção de runtime (`src/env_detect.rs`): `/.dockerenv` e hints de cgroup docker/kubepods/containerd/libpod, lidos só de arquivos
- Pods Kubernetes: o mount de service account `/var/run/secrets/kubernetes.io/serviceaccount` é reportado por `atomwrite doctor`


## Detecção de Ambiente em Runtime
`atomwrite doctor` reporta flags especializadas do host. A detecção lê só arquivos e nenhuma variável de ambiente (Rules Rust multiplataforma):

| Flag | Detecção |
|------|----------|
| WSL1/WSL2 | `/proc/sys/kernel/osrelease` ou `/proc/version` contém `microsoft` |
| Container | `/.dockerenv` ou `/proc/1/cgroup` (reserva `/proc/self/cgroup`) com hints docker/kubepods/containerd/libpod |
| Kubernetes | o diretório `/var/run/secrets/kubernetes.io/serviceaccount` existe |
| Termux | o diretório `/data/data/com.termux/files/usr` existe |
| Flatpak / Snap | o arquivo `/.flatpak-info` existe / `/proc/self/exe` aponta para `/snap/` |

No Windows toda sonda de arquivo é pulada. O `doctor` também reporta `os` / `arch` / `family` / `target` de compilação, `config_dir` de storage e métodos de fsync/rename da plataforma.


## Suporte a Shell
### bash
- Gere completions: `atomwrite completions bash > ~/.local/share/bash-completion/completions/atomwrite`
- Auto-instalar (v0.1.2+): `atomwrite completions bash --install` escreve diretamente no diretório XDG
- Recarregue: `source ~/.bashrc`

### zsh
- Gere completions: `atomwrite completions zsh > ~/.zfunc/_atomwrite`
- Adicione ao fpath: `fpath=(~/.zfunc $fpath)` no `~/.zshrc`
- Recarregue: `source ~/.zshrc`

### fish
- Gere completions: `atomwrite completions fish > ~/.config/fish/completions/atomwrite.fish`
- Disponível imediatamente em novos shells

### PowerShell
- Gere completions: `atomwrite completions powershell > $HOME\Documents\PowerShell\Scripts\atomwrite.ps1`
- Carregue: `. $HOME\Documents\PowerShell\Scripts\atomwrite.ps1`

### Elvish
- Gere completions: `atomwrite completions elvish`
- Instalação automática: XDG data `elvish/lib/atomwrite.elv`

### Nushell
- A CLI roda sob Nushell com NDJSON estruturado em stdout (contrato agent-first).
- `clap_complete` ainda não gera completions nativas de Nushell; use completion externa ou `--help` / `json-schema`.


## Caminhos de Arquivo e XDG
- atomwrite usa caminhos absolutos em toda saída NDJSON
- Caminhos relativos nos argumentos são resolvidos contra a raiz do workspace
- `--workspace` padrão é o diretório de trabalho atual
- O jail de workspace é definido só pela CLI: sempre passe `--workspace <raiz>` (sem variáveis de ambiente de produto)
- Arquivos de backup são armazenados ao lado do original com sufixo de timestamp, a menos que `--output-dir` seja definido
- O comando `completions --install` escreve nos diretórios de dados XDG (`$XDG_DATA_HOME` ou `~/.local/share`)
- `storage.home` é uma chave do verbo `config`: `atomwrite config set storage.home DIR --scope global` e `atomwrite config get storage.home`. Ele realoca config, dados, cache e state para `DIR/config`, `DIR/data`, `DIR/cache` e `DIR/state`. Se ausente, usa `directories::ProjectDirs` (`src/storage.rs`), e nenhuma variável de ambiente sobrescreve o home. A tabela de caminhos por plataforma está em `docs/INSTALL.pt-BR.md` (Arquivos e Diretórios)
- Config global: `{config_dir}/config.toml` (modo 0600 em Unix, ACL do perfil do usuário no Windows); preferência de locale: `{config_dir}/locale`
- `completions SHELL --install` escreve sob o pai do diretório de dados (`~/.local/share/` em Linux com a configuração XDG padrão), e sob `{home}` quando `storage.home` está definido
- Paths usam apenas `PathBuf` / `Path::join` — sem separadores `/` ou `\` hardcoded na lógica
- Normalização Unicode NFC na validação de path jail
- Nomes reservados Windows (`CON`, `PRN`, `AUX`, `NUL`, `COM0`–`COM9`, `LPT0`–`LPT9`) rejeitados em todos os hosts para árvores portáveis
- No Windows, caracteres ilegais e espaço/ponto final são rejeitados; paths longos recebem prefixo `\\?\` via `path_safety::prepare_fs_path` perto de `MAX_PATH`


## Requisitos de Build por Plataforma
- Linux (x86_64, aarch64): Rust 1.90+, glibc padrão
- macOS (Intel, Apple Silicon): Rust 1.90+, a compatibilidade Nix é restrita a `cfg(target_os = "linux")` então `posix_fadvise` é um no-op no macOS (adicionado em v0.1.2 — antes da v0.1.2, o build falhava no macOS)
- Windows (x86_64): Rust 1.90+, toolchain MSVC, `windows-sys` 0.61 (atualizado em v0.1.2)
- Só o build Linux x86_64 foi executado na v0.1.37 (veja a Matriz de Verificação)


## Performance por Target
- Os números abaixo são estimativas herdadas de versões anteriores. Nenhum foi medido na v0.1.37, e só o Linux x86_64 foi executado

### x86_64-unknown-linux-gnu
- Target mais rápido para todas as operações
- Aceleração SIMD completa para hashing BLAKE3
- Busca paralela escala linearmente com contagem de cores
- Latência típica de escrita: <1ms para arquivos abaixo de 1 MiB

### aarch64-unknown-linux-gnu
- Aceleração NEON para hashing BLAKE3
- Performance comparável ao x86_64 em cores ARM modernos
- Adequado para servidores ARM e Raspberry Pi 4+

### x86_64-apple-darwin / aarch64-apple-darwin
- Apple Silicon fornece excelente performance single-core
- `F_FULLFSYNC` adiciona ~0.5ms de overhead por escrita versus fsync padrão
- O overhead é o custo da durabilidade real

### x86_64-pc-windows-msvc
- Overhead de `FlushFileBuffers` varia por driver de storage
- Drives NVMe: <1ms por escrita
- Drives rotativos: 5-15ms por escrita devido ao flush físico
- Pré-requisito v0.1.4: Visual Studio 2019+ Build Tools com workload C++
- Pré-requisito v0.1.4: Rust 1.88 ou posterior (histórico; o mínimo atual é o Rust 1.90)
- Pré-requisito v0.1.4: Windows Terminal ou PowerShell 7+ para UTF-8

### x86_64-pc-windows-gnu (cross-compile do Linux)
- Target de cross-compile para contribuidores
- Requer toolchain mingw-w64 (`mingw64-gcc` Fedora, `mingw-w64` Ubuntu)
- v0.1.4 habilita validação via `cargo test --test cross_compile_check -- --ignored`

### i686-pc-windows-gnu (Windows 32-bit, cross-compile)
- Target de cross-compile para Windows 32-bit
- Requer `mingw32-gcc` no host (separado do mingw 64-bit)
- v0.1.4 habilita validação via `cargo test --test cross_compile_check -- --ignored`


## Agentes Validados por Plataforma
- Nenhum teste específico de agente está registrado para nenhuma plataforma, então este guia não cita nenhum agente como validado
- A rota de integração é a CLI mais NDJSON por subprocesso, e a suíte de testes exercita essa rota em Linux x86_64 por meio do `assert_cmd`
- macOS e Windows: nenhuma execução de agente e nenhuma execução da CLI foi registrada na v0.1.37


## v0.1.20 — Novidades

Esta release introduz uma nova camada de segurança chamada intention guards e renomeia a flag global `--lang` para `--locale` para desambiguar do seletor tree-sitter `--lang` usado por `scope` e `transform`.

### Intention Guards (5 flags OPT-IN)
- `--require-backup` — exige `--backup`: aborta quando falta e o arquivo alvo existe (não recebe valor)
- `--confirm` — flag legada de consciência no `write`, sem efeito sozinha: sobrescrita grande exige `--ack-overwrite`, e `delete --confirm` é rejeitado (use `--plan`)
- `--auto-rotate` — força um backup de rotação automática para arquivos modificados há pouco (idade de `[write].auto_rotate_max_age_secs`, padrão 24 h; não recebe valor)
- `--risk-threshold <PERCENT>` — limite de variação de tamanho, em percentual, para o aviso de risco (padrão 255 = desligado; por exemplo `--risk-threshold 50`)
- `--locale <en|pt-BR>` — renomeado de `--lang` para desambiguar do `--lang` tree-sitter

### Outras Adições
- `count --by-size` — lista os maiores arquivos da árvore com tamanhos e contagem de linhas
- `read --format raw|ndjson` — seleciona entre saída byte-stream e envelope NDJSON estruturado (uma grafia antiga com `--mode` nunca existiu)
- `search --no-begin-end` — desabilita a decoração implícita de âncoras `^` e `$` na saída regex
- `write --preserve-timestamps` — preserva o mtime do arquivo fonte ao sobrescrever
- `scope --lang rust` — alias explícito de `scope --language`; o `transform` aceita só `-l`/`--language`

### Estatísticas
- 621 testes passando em 51 suites de integração, 0 falhas
- 52 GAP-2026 fechados (019-070)
- 3 targets de cross-compile Windows verdes
- 19 ADRs em `docs/decisions/` (0019-0037)

### Migração `--lang` para `--locale`

```bash
# Descobrir todos os arquivos com --lang
rg -l -- '--lang\b' .

# Substituir em massa preservando outros matches
fd -e sh -e md -e toml -e yml -e yaml -e json -x sd -- '--lang\b' '--locale' {}

# Ou via ruplacer (só simula até você acrescentar --go)
ruplacer -- '--lang' '--locale' .
```


## v0.1.21 — Novidades

Esta release é totalmente retrocompatível em todas as 3 plataformas suportadas (Linux, macOS, Windows). Nenhum comportamento específico de plataforma foi alterado.

- Flag opt-in `--allow-sequential-drift` em `edit` — comportamento uniforme entre plataformas
- `--backup` e `--keep-backup` propagados em 6 sub-comandos (`write`, `edit`, `replace`, `rollback`, `apply`, `batch`) — funciona identicamente em Linux ext4, macOS APFS e Windows NTFS
- Arquivos de backup usam nomenclatura `.bak.<YYYYMMDD_HHMMSS>` e `fs::copy` atômico — mesmas garantias de atomicidade nas 3 plataformas


## v0.1.22 — Novidades

Esta release é totalmente retrocompatível em todas as 3 plataformas suportadas.

- `prune-backups [PATHS]...` — limpeza manual de siblings `.bak.YYYYMMDD_HHMMSS`
  - Usa `std::fs::remove_file` diretamente (não o pipeline atômico)
  - Honra o jail do `--workspace` em todas as plataformas
  - Saída NDJSON é independente de plataforma
- `edit-loop <PATH>` — N pares em 1 invocação via NDJSON
  - Lê o arquivo uma vez com `read_file_string` (usa memmap2 para arquivos grandes em Linux/macOS)
  - Escreve atomicamente via o mesmo pipeline do `edit`
  - No Windows: respeita `init_console` UTF-8 e tratamento ANSI (v0.1.4)


## v0.1.29 — Durabilidade e rename

Esta release é retrocompatível em Linux, macOS e Windows para flags existentes. Novos controles são opt-in.

- O behavior break de `replace --fuzzy auto` está documentado em [MIGRATION.pt-BR.md](MIGRATION.pt-BR.md) (esta seção cobre I/O de plataforma)
- Linux prefere `renameat2` no rename atômico e faz fallback para `rename` em `ENOSYS`
- Saída NDJSON do write reporta `platform.rename_method` (`renameat2` ou `rename`)
- `write --durability full|fast|auto` seleciona a política de fsync
- `full` — fsync completo do arquivo mais fsync do diretório quando disponível
- `fast` — omite fsync do diretório pai em favor da velocidade
- `auto` — heurística baseada no path e no ambiente
- macOS sob durability `full` usa `F_FULLFSYNC` via `fcntl` (mesmo primitivo de antes, agora sob política)
- Windows mantém o flush existente; rótulos de durability ainda aparecem no NDJSON para agentes
- Backup do arquivo vivo NUNCA é hardlink do mesmo inode (residual v0.1.30)
- `platform.backup_method` reporta apenas `reflink_or_copy` ou `copy`
- Feature Cargo `core` mantém o caminho de binário slim; AST/watch/semantic permanecem opcionais


## v0.1.30 — Backup e residual de agente
- Release residual de contrato de agente sobre a superfície de plataforma da v0.1.29
- Caminho de backup usa `reflink_or_copy` com fallback para copy; nunca hardlink do arquivo vivo
- NDJSON do edit pode incluir `match_count` e `indent_adjusted` para parsing por agentes
- `--fuzzy off` = exact-only (G-010 CLOSED na v0.1.35) na CLI e em `.atomwrite.toml` `[fuzzy] mode` (não rejeitado; cascata pulada)
- Sparse outline emite kinds reais de `outline_item` AST sob orçamento
- Help e docs de `semantic-merge` admitem merge line-based (não AST)
- Hash recursivo de recipe exclui paths `*.bak.*`
- Veja [gaps.md](../gaps.md) e [MIGRATION.pt-BR.md](MIGRATION.pt-BR.md#v0129-para-v0130)


## v0.1.34 — Fuzzy one-shot e timeout
- Publicação docs-complete do runtime one-shot da v0.1.33 (mesmo comportamento do binário em todas as plataformas)
- Global `--timeout-secs` / `--timeout` padrão 120; `0` desabilita; prazo esgotado → exit 124 (cross-platform)
- Multi-apply fuzzy é one-pass E→D (`apply_fuzzy_one_pass`); nunca reescaneia texto inserido
- Máximo de applies fuzzy padrão 1; se o replacement contém o pattern (string fixa), força apply único
- Cancel cooperativo consultado no meio da cascata fuzzy; caps: pattern 64 KiB, lev 8192, windows 4096, growth max(4×,+16 MiB)
- Regressão: `cargo test --test cli_v0133_oneshot_fuzzy`
- Veja [MIGRATION.pt-BR.md](MIGRATION.pt-BR.md#v0133-para-v0134) e ADR-0054


## Smoke multi-target A-014 (local)

Rode num host com os targets do rustup instalados:

```bash
for t in x86_64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin \
  x86_64-pc-windows-gnu x86_64-pc-windows-msvc i686-pc-windows-gnu; do
  cargo check --release --target "$t" || echo "SKIP/FAIL $t"
done
```

Notas (2026-07-19):
- `x86_64-unknown-linux-gnu`: host e2e primário — verde obrigatório.
- Os targets Apple/Windows podem falhar por falta de toolchains cruzadas (`lib.exe`, SDK do macOS, zstd asm) — documente o skip com honestidade; não é regressão do produto.
- O laço não foi executado de novo na v0.1.37 (veja a Matriz de Verificação).


## v0.1.35 — Multi-OS honesto e gates locais
- Linux: e2e completo + unit/integration
- Windows-gnu: `cargo check --target x86_64-pc-windows-gnu` no host Linux
- macOS: `cfg` no fonte; cross pode falhar sem SDK do host — validar em Mac real
- Só gates locais; use o DoD local de `docs/TESTING.pt-BR.md`
- Watch/notify e separadores de path são específicos de plataforma; veja A-XPLAT-001 em ADR-0055


## v0.1.36 — Contrato de agente (neutro de plataforma)
- Histórico (v0.1.36): 46 subcomandos SSOT (`agent-surface` / `commands`); a lista atual de 47 está no catálogo abaixo
- Jail de content-source + `scratch init` (sem allowlist de `/tmp`) — mesma política de path em Linux/macOS/Windows
- NDJSON de erro sempre inclui `ok: false` + `match_count` estável
- Globais `--select`/`--fields`, `--filter`, `--limit` (somente long; sem `-n` global)
- Suite de contrato: `cargo test --test cli_v0136_gaps`; o status vive no ADR que possui o gap
- Veja o ADR-0055 (GAP-2026-019…033)


## v0.1.37 — Notas de plataforma
- Fixe agentes em `^0.1.37`; 47 subcomandos SSOT (`agent-surface` / `commands`), a mesma lista em Linux, macOS e Windows por projeto (exercitada só em Linux)
- Pipe quebrado: exit 141 em toda plataforma. O Unix instala um handler cooperativo de SIGPIPE (o sinal não mata o processo), a escrita que acerta o pipe fechado falha com `BrokenPipe`, todo `Drop` roda e o processo não deixa lock, journal nem tempfile. Um pai que ignora SIGPIPE, ou o Windows (`ERROR_BROKEN_PIPE` mapeado para `BrokenPipe`), recebe o mesmo exit 141. O SIGPIPE não volta a `SIG_DFL`: é um desvio consciente (ADR-0063)
- `config set --scope global` grava o `config.toml` XDG com modo `0600` no Unix; o Windows não tem bits de modo e confia na ACL do perfil do usuário
- O persist de tempfile no Windows repete `PermissionDenied` e `ERROR_SHARING_VIOLATION` (32) com full jitter (base 50 ms, teto 400 ms) e continua obedecendo `--timeout-secs`; não validado no Windows
- O lock advisory (o `std::fs::File::try_lock`, que é `flock` no Unix e `LockFileEx` no Windows) espera até `lock.acquire_timeout_ms` (padrão 5000) com o mesmo backoff; exit 83 quando não é obtido. Não validado em NFS nem no Windows
- Build slim (`--no-default-features --features core`): `outline`, `sparse outline`, `query`, `scope`, `transform` e `codemod` saem 78 (medido em Linux); `cargo xtask check-features` confere os conjuntos `core` e `core,watch`
- O build padrão não tem `watch`: `atomwrite watch` sai 78 até o binário ser compilado com `--features watch`
- Os parsers são baixados em tempo de execução por `outline`, `sparse outline`, `query` e `write --syntax-check` em toda plataforma


## Os 47 Subcomandos em Cada Plataforma
- A lista é a mesma em Linux, macOS e Windows, e cada linha tem um resumo de uma linha e um exemplo que foi executado em Linux x86_64 (`atomwrite 0.1.37`); nenhum foi executado no macOS nem no Windows
- Preparação dos exemplos: `WS` é o caminho absoluto de um workspace vazio que contém `src/a.rs` (um `unwrap()` e um comentário), `src/b.rs` (a palavra `beta` uma vez), `src/c.rs` (com `user_id`), `src/tmp.rs`, `config.toml` (uma tabela `[package]` com `name` e uma tabela `[features]` com `x`), `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` e `rules.yaml` (uma lista YAML com `id`, `language`, `pattern` e `rewrite`)
- `HASH_A` é o `checksum` que `hash src/a.rs` imprime
- Os exemplos usam `printf ... |` e um shell POSIX. No Windows rode os mesmos argumentos de `atomwrite` pelo PowerShell e alimente o stdin com here-string ou com um pipe de `Write-Output`; isso não foi validado
- Os exemplos rodam na ordem abaixo: o `rollback` precisa do `backup` anterior e o `edit-loop` precisa do `edit` anterior
- Resultado em Linux: 46 dos 47 saem 0, e o `watch` sai 78 porque o build padrão não tem a feature `watch`

- `read` — Lê um arquivo com checksum, tamanho e linhas, com opção de só início, fim ou faixa. Exemplo: `atomwrite --workspace "$WS" read --head 2 src/a.rs`
- `write` — Cria ou sobrescreve um arquivo de forma atômica a partir do stdin. Exemplo: `printf 'x\n' | atomwrite --workspace "$WS" write notes/new.txt`
- `edit` — Edita um arquivo por texto exato ou aproximado, número de linha ou marcador. Exemplo: `atomwrite --workspace "$WS" edit src/b.rs --old beta --new gamma`
- `delete` — Apaga arquivos ou árvores com backup, ou só mostra o plano. Exemplo: `atomwrite --workspace "$WS" delete --plan tmp.txt`
- `copy` — Copia um arquivo ou árvore com verificação de checksum. Exemplo: `atomwrite --workspace "$WS" copy src/a.rs src/a_copy.rs`
- `move` — Move ou renomeia um arquivo de forma atômica. Exemplo: `atomwrite --workspace "$WS" move src/tmp.rs src/tmp2.rs`
- `apply` — Aplica um patch (diff unificado, SEARCH/REPLACE, arquivo inteiro) vindo do stdin. Exemplo: `printf 'new body\n' | atomwrite --workspace "$WS" apply --format full notes.txt`
- `search` — Busca conteúdo de arquivos em paralelo com o motor do ripgrep. Exemplo: `atomwrite --workspace "$WS" search alpha src/`
- `replace` — Troca texto em vários arquivos com escrita atômica e casamento aproximado. Exemplo: `atomwrite --workspace "$WS" replace --dry-run alpha omega src/`
- `transform` — Reescreve código por padrão de AST (ast-grep). Exemplo: `atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `hash` — Mostra o checksum BLAKE3 de arquivos. Exemplo: `atomwrite --workspace "$WS" hash src/a.rs`
- `count` — Conta linhas e arquivos por extensão ou por tamanho. Exemplo: `atomwrite --workspace "$WS" count --by-extension src/`
- `diff` — Compara dois arquivos (unificado, estatística ou mudanças). Exemplo: `atomwrite --workspace "$WS" diff src/a.rs src/b.rs --unified`
- `list` — Lista uma árvore de diretórios com metadados. Exemplo: `atomwrite --workspace "$WS" list --long --depth 1 src/`
- `extract` — Extrai campos de NDJSON lido do stdin. Exemplo: `printf '{"a":1}\n' | atomwrite --workspace "$WS" extract --field a`
- `extract-artifact` — Sinal limitado dos cabeçalhos de PDF, OOXML e imagem (nunca os bytes brutos). Exemplo: `atomwrite --workspace "$WS" extract-artifact img.png`
- `scope` — Escopo gramatical: age sobre comentários, strings, funções e outras categorias de código. Exemplo: `atomwrite --workspace "$WS" scope src/ --language rust --query comments --delete --dry-run`
- `regex` — Gera uma regex a partir de exemplos. Exemplo: `atomwrite --workspace "$WS" regex abc-12 xyz-99 id-7 --digits`
- `calc` — Cálculo e conversão de unidades. Exemplo: `atomwrite --workspace "$WS" calc "2 hours to seconds"`
- `completions` — Gera ou instala completions de shell (bash, zsh, fish, elvish, powershell). Exemplo: `atomwrite --workspace "$WS" completions bash`
- `backup` — Cria backups com data e hora e checksum BLAKE3. Exemplo: `atomwrite --workspace "$WS" backup src/a.rs --retention 3`
- `rollback` — Restaura um arquivo a partir de um backup anterior. Exemplo: `atomwrite --workspace "$WS" rollback src/a.rs --latest --verify`
- `batch` — Executa operações NDJSON do stdin, com opção de transação única. Exemplo: `printf '{"op":"write","target":"batch.txt","content":"x"}\n' | atomwrite --workspace "$WS" batch --transaction`
- `edit-loop` — Aplica N pares antigo/novo numa escrita só, tudo ou nada. Exemplo: `printf '[{"old":"gamma","new":"delta"}]\n' | atomwrite --workspace "$WS" edit-loop src/b.rs`
- `prune-backups` — Remove arquivos .bak antigos por idade ou quantidade. Exemplo: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run .`
- `wal-stats` — Mostra o estado dos journals WAL (só leitura). Exemplo: `atomwrite --workspace "$WS" wal-stats`
- `wal-heal` — Remove journals WAL terminais órfãos mais antigos que um limite. Exemplo: `atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .`
- `verify` — Confere um arquivo contra um hash BLAKE3 esperado (exit 0 se igual, 81 se diferente). Exemplo: `atomwrite --workspace "$WS" verify src/a.rs HASH_A`
- `set` — Grava um valor num caminho com pontos em arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" set config.toml package.name demo2`
- `get` — Lê um valor num caminho com pontos em arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" get config.toml package.name`
- `del` — Remove uma chave de um arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" del config.toml features.x`
- `case` — Renomeia identificadores entre estilos de caixa em vários arquivos. Exemplo: `atomwrite --workspace "$WS" case --subvert user_id UserId --to pascal --dry-run src/c.rs`
- `query` — Percorre a AST do tree-sitter e emite os nós. Exemplo: `atomwrite --workspace "$WS" query --kinds src/a.rs`
- `outline` — Emite a estrutura de alto nível de um arquivo de código. Exemplo: `atomwrite --workspace "$WS" outline src/a.rs`
- `semantic-merge` — Merge de três vias por linha com marcadores de conflito (não é AST). Exemplo: `atomwrite --workspace "$WS" semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict`
- `sparse` — Lista, leitura e outline com orçamento para árvores grandes. Exemplo: `atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576`
- `recipe` — Lista e roda receitas embutidas de vários passos. Exemplo: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern alpha --replacement omega --path src --dry-run`
- `stat` — Só metadados, alias de read --stat. Exemplo: `atomwrite --workspace "$WS" stat src/a.rs`
- `agent-surface` — Manifesto de ferramentas para agentes (a fonte única dos subcomandos). Exemplo: `atomwrite --workspace "$WS" agent-surface --format json`
- `watch` — Transmite eventos do sistema de arquivos com debounce e termina com watch_summary (exige build com `--features watch`, o build padrão sai 78). Exemplo: `atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500`
- `codemod` — Aplica uma lista YAML de regras de reescrita por AST numa árvore. Exemplo: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- `semantic-search` — Ranking offline por Jaccard de arquivos para uma consulta. Exemplo: `atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01`
- `doctor` — Diagnóstico de prontidão do host. Exemplo: `atomwrite --workspace "$WS" doctor`
- `scratch` — Cria o diretório de staging .atomwrite/scratch dentro do jail do workspace. Exemplo: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch`
- `locale` — Mostra, define ou limpa o idioma da interface. Exemplo: `atomwrite --workspace "$WS" locale`
- `commands` — Imprime a árvore completa de comandos em JSON. Exemplo: `atomwrite --workspace "$WS" commands --include-globals`
- `config` — Lê e grava chaves de ajuste persistentes (37 chaves, escopos global e workspace). Exemplo: `atomwrite --workspace "$WS" config get fuzzy.threshold`

### Diferenças de Plataforma por Comando
- `write`, `edit`, `edit-loop`, `set`, `del`, `apply`, `config` e as ops `replace` e `edit` do `batch`: serializam no lock advisory, que funciona no Unix e no Windows (NFS: não validado)
- `case`, `transform`, `scope` e `codemod`: arquivo que mudou entre a leitura e a escrita falha com `STATE_DRIFT` (exit 82); igual em toda plataforma
- `move` e `copy`: reflink CoW em APFS, btrfs e XFS, `reflink_or_copy` como método de backup, atributos estendidos preservados em Unix; nada disso foi validado no Windows
- `backup`, `rollback`, `prune-backups`: o nome do backup é `<nome>.bak.<YYYYMMDD_HHMMSS>` em toda plataforma
- `completions`: com `--install` os arquivos vão sob o pai do diretório de dados da plataforma (veja `docs/INSTALL.pt-BR.md`)
- `doctor`: toda sonda de arquivo é pulada no Windows (WSL, container, Kubernetes, Termux, Flatpak e Snap são conceitos de Linux)
- `watch`: exige `--features watch` e usa o crate `notify`, cujo backend depende do sistema operacional; `cargo xtask check-features` só compila o conjunto `core,watch`, e uma execução do `watch` não foi feita nesta auditoria em nenhuma plataforma
- `read`, `write`, `delete`: nomes reservados no Windows (`CON`, `PRN`, `AUX`, `NUL`, `COM0` a `COM9`, `LPT0` a `LPT9`) são rejeitados em todos os hosts, para a árvore de um agente continuar portável
- `locale` e `config`: a preferência de locale e a config global ficam no diretório de config da plataforma
