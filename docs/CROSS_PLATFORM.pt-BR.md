# atomwrite Suporte Multiplataforma


[Read in English](CROSS_PLATFORM.md)

> Escreva uma vez, execute em qualquer lugar -- com garantias reais de fsync em cada plataforma


## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças relevantes para cross-platform em v0.1.12.

### Tratamento de Sinais (Melhorado)

- v0.1.12 adiciona 5 novos testes em `tests/signal_test.rs` cobrindo SIGINT, SIGTERM, SIGPIPE, interrupção de batch
- `tests/signal_test.rs::batch_interrupted_by_signal` valida cleanup do journal WAL em sinal
- `tests/signal_test.rs::sigpipe_exits_141_or_signal_13` confirma tratamento de BrokenPipe (exit 141 ou signal 13)
- `tests/signal_test.rs::sigint_during_search_exits_130` e `sigterm_during_search_exits_143` confirmam exit codes limpos
- `tests/signal_test.rs::shutdown_message_on_stderr` valida tracing em shutdown

### Windows

- v0.1.12 preserva o fix do Windows 10/11 da v0.1.4: `cargo install atomwrite` funciona
- Melhorias no `init_console`: UTF-8 code page 65001 + `ENABLE_VIRTUAL_TERMINAL_PROCESSING`
- `persist_with_retry` lida com `PermissionDenied` durante rename atômico com backoff exponencial
- Específico do Windows: 5 novos códigos de erro (83, 88, 91, 92, 93) todos com mensagens bilíngues

### Linux

- v0.1.12 requer Rust 1.88 ou posterior
- Flag `--include-fifo` pula FIFO/named pipes (G56) para prevenir travamento
- Flag `--strict-atomic` aborta em EXDEV (G90) para filesystems onde atomicidade é crítica
- Lock advisory de arquivo via `flock` funciona no Linux (G54)
- Preservação de xattr funciona em ext4, btrfs, XFS, F2FS (G39)

### macOS

- v0.1.12 preserva os fixes de build do macOS arm64 (Apple Silicon) e macOS x86_64 da v0.1.2
- Reflink CoW funciona em APFS (G64): backup e copy O(1)
- Preservação de xattr funciona para `com.apple.quarantine`, `kMDItemUserTags`, `kMDItemFinderComment` (G39)
- Gatekeeper pode exigir `xattr -d com.apple.quarantine` no primeiro uso

### Containers (Docker, Podman, Kubernetes)

- Fallback EXDEV (G90) lida com Docker overlay2 + named volumes automaticamente
- Exit code 91 (`ExdevFallbackDisabled`) para opt-out via `--strict-atomic`
- Sem mudanças de código necessárias para usuários de container; funciona out of the box

### NFS

- `flock(2)` é silenciosamente ignorado em NFS, então `--lock` pode não proteger contra edições concorrentes
- Combine `--lock` com `--expect-checksum` para defesa em profundidade
- `--expect-checksum` detecta desvio de estado após escrita (exit 82)

### Cobertura de Testes

- 502 testes passando (445 na v0.1.12 + 2 na v0.1.14 + 8 G117 + 6 G118 na v0.1.15)
- Gate de cross-compile: `cargo test --test cross_compile_check -- --ignored` valida targets Windows GNU/MSVC
- 5 testes de sinal em `tests/signal_test.rs` cobrem SIGINT/SIGTERM/SIGPIPE/batch/shutdown
- Veja [docs/decisions/README.md](README.md) para decisões arquiteturais

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
- Rename atômico: `rename(2)` dentro do mesmo filesystem
- Cross-device: fallback automático para cópia-depois-deleta
- Testado em x86_64 e aarch64

### macOS (Suporte Completo)
- Fsync de arquivo: `F_FULLFSYNC` via `fcntl` para durabilidade real
- Fsync de diretório: `sync_all()` no diretório pai
- O `fsync` padrão no macOS NÃO garante escrita no disco
- atomwrite usa `F_FULLFSYNC` automaticamente
- Testado em Apple Silicon e Intel

### Windows (Suporte Completo a partir da v0.1.4)
- Fsync de arquivo: `FlushFileBuffers` via `sync_all()`
- Fsync de diretório: best-effort (Windows não tem `fsync` para diretórios)
- **Correção v0.1.15 (GAP 18)**: o teste de snapshot do write redige `platform.dir_fsync` como `[platform_dir_fsync]` porque o Windows reporta `best_effort` enquanto o Unix reporta `sync_all`; o job de CI windows-2025 voltou ao verde.
- Rename atômico via `MoveFileExW` com `MOVEFILE_REPLACE_EXISTING`
- NTFS fornece garantias razoáveis de durabilidade
- **Fix v0.1.4 (GAP 14)**: `cargo install atomwrite` agora funciona no Windows 10/11. Três erros de compilação em blocos `#[cfg(windows)]` quebravam a release v0.1.3 no Windows foram resolvidos.
- **Adição v0.1.4**: `init_console` ativa UTF-8 (code page 65001) e `ENABLE_VIRTUAL_TERMINAL_PROCESSING` para que sequências ANSI sejam interpretadas pelo Windows Console Host. Isso faz saída colorida e caracteres Unicode funcionarem corretamente em Windows Terminal e PowerShell 7+.
- **Adição v0.1.4**: `persist_with_retry` lida com erros `PermissionDenied` específicos de Windows durante o rename atômico via retry com backoff exponencial (100ms, 200ms, 400ms) — isso compensa o Windows Defender ou outros processos antivírus que brevemente seguram o arquivo.
- Testado em x86_64 e i686 (i686 requer toolchain mingw 32-bit)


## Tratamento de Sinais
### Linux e macOS
- SIGINT (130): shutdown gracioso, faz flush de escritas pendentes
- SIGTERM (143): shutdown gracioso, faz flush de escritas pendentes
- SIGPIPE (141): para de escrever em pipe quebrado silenciosamente

### Windows
- Ctrl+C: tratado via SetConsoleCtrlHandler
- SIGPIPE: não aplicável no Windows
- Terminação de processo: escritas atômicas pendentes são abandonadas com segurança


## Containers
- Docker: funciona imediatamente com imagens Linux padrão
- Podman: comportamento idêntico ao Docker
- Overlay filesystems: rename atômico funciona dentro das camadas overlay
- Volume mounts: fsync alcança o filesystem do host através do mount
- tmpfs: fsync é no-op mas rename ainda é atômico
- Moves entre containers: use `--workspace` para evitar escapar do mount


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


## Caminhos de Arquivo e XDG
- atomwrite usa caminhos absolutos em toda saída NDJSON
- Caminhos relativos nos argumentos são resolvidos contra a raiz do workspace
- `--workspace` padrão é o diretório de trabalho atual
- `--workspace` é obrigatório quando definido via variável de ambiente `ATOMWRITE_WORKSPACE`
- Arquivos de backup são armazenados ao lado do original com sufixo de timestamp, a menos que `--output-dir` seja definido
- O comando `completions --install` escreve nos diretórios de dados XDG (`$XDG_DATA_HOME` ou `~/.local/share`)


## Requisitos de Build por Plataforma
- **Linux** (x86_64, aarch64): Rust 1.88+, glibc padrão
- **macOS** (Intel, Apple Silicon): Rust 1.88+, a compatibilidade Nix é restrita a `cfg(target_os = "linux")` então `posix_fadvise` é um no-op no macOS (adicionado em v0.1.2 — antes da v0.1.2, o build falhava no macOS)
- **Windows** (x86_64): Rust 1.88+, toolchain MSVC, `windows-sys` 0.61 (atualizado em v0.1.2)


## Performance por Target
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
- Pré-requisito v0.1.4: Visual Studio 2019+ Build Tools com workload C++
- Pré-requisito v0.1.4: Rust 1.88 ou posterior
- Pré-requisito v0.1.4: Windows Terminal ou PowerShell 7+ para UTF-8

### x86_64-pc-windows-gnu (cross-compile do Linux)
- Target de cross-compile para contribuidores
- Requer toolchain mingw-w64 (`mingw64-gcc` Fedora, `mingw-w64` Ubuntu)
- v0.1.4 habilita validação via `cargo test --test cross_compile_check -- --ignored`

### i686-pc-windows-gnu (Windows 32-bit, cross-compile)
- Target de cross-compile para Windows 32-bit
- Requer `mingw32-gcc` no host (separado do mingw 64-bit)
- v0.1.4 habilita validação via `cargo test --test cross_compile_check -- --ignored`
- Drives rotativos: 5-15ms por escrita devido ao flush físico


## Agentes Validados por Plataforma
### Linux
- Claude Code (Anthropic)
- Cursor (Anysphere)
- Aider
- OpenAI Codex CLI

### macOS
- Claude Code (Anthropic)
- Cursor (Anysphere)
- Windsurf (Codeium)
- Aider

### Windows
- Claude Code (Anthropic)
- Cursor (Anysphere)
- Windsurf (Codeium)
