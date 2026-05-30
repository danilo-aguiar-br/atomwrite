# atomwrite Suporte Multiplataforma


[Read in English](CROSS_PLATFORM.md)

> Escreva uma vez, execute em qualquer lugar -- com garantias reais de fsync em cada plataforma


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

### Windows (Best-Effort)
- Fsync de arquivo: `FlushFileBuffers` via `sync_all()`
- Fsync de diretório: best-effort (Windows não tem `fsync` para diretórios)
- Rename atômico via `MoveFileExW` com `MOVEFILE_REPLACE_EXISTING`
- NTFS fornece garantias razoáveis de durabilidade
- Testado em x86_64


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
- Nenhum diretório XDG é criado ou usado
- Nenhum arquivo de configuração é lido do disco
- Arquivos de backup são armazenados ao lado do original com sufixo de timestamp


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
