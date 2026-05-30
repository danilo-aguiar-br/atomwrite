# atomwrite Cross-Platform Support


[Leia em Portugues](CROSS_PLATFORM.pt-BR.md)

> Write once, run anywhere -- with real fsync guarantees on every platform


## The Pain You Already Know
- You write a file on Linux and it reaches disk reliably
- You write the same file on macOS and `fsync` silently lies about durability
- You write on Windows and directory fsync is not even a concept
- Your agent does not know which platform it runs on
- atomwrite handles all of this for you transparently


## Support Matrix
### Linux (Full Support)
- File fsync: `fdatasync` via `sync_data()`
- Directory fsync: `sync_all()` on parent directory
- Atomic rename: `rename(2)` within same filesystem
- Cross-device: automatic copy-then-delete fallback
- Tested on x86_64 and aarch64

### macOS (Full Support)
- File fsync: `F_FULLFSYNC` via `fcntl` for true durability
- Directory fsync: `sync_all()` on parent directory
- Standard `fsync` on macOS does NOT guarantee disk write
- atomwrite uses `F_FULLFSYNC` automatically
- Tested on Apple Silicon and Intel

### Windows (Best-Effort)
- File fsync: `FlushFileBuffers` via `sync_all()`
- Directory fsync: best-effort (Windows has no `fsync` for directories)
- Atomic rename via `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`
- NTFS provides reasonable durability guarantees
- Tested on x86_64


## Signal Handling
### Linux and macOS
- SIGINT (130): graceful shutdown, flush pending writes
- SIGTERM (143): graceful shutdown, flush pending writes
- SIGPIPE (141): stop writing to broken pipe silently

### Windows
- Ctrl+C: handled via SetConsoleCtrlHandler
- SIGPIPE: not applicable on Windows
- Process termination: pending atomic writes are abandoned safely


## Containers
- Docker: works out of the box with standard Linux images
- Podman: identical behavior to Docker
- Overlay filesystems: atomic rename works within overlay layers
- Volume mounts: fsync reaches the host filesystem through the mount
- tmpfs: fsync is a no-op but rename is still atomic
- Cross-container moves: use `--workspace` to prevent escaping the mount


## Shell Support
### bash
- Generate completions: `atomwrite completions bash > ~/.local/share/bash-completion/completions/atomwrite`
- Reload: `source ~/.bashrc`

### zsh
- Generate completions: `atomwrite completions zsh > ~/.zfunc/_atomwrite`
- Add to fpath: `fpath=(~/.zfunc $fpath)` in `~/.zshrc`
- Reload: `source ~/.zshrc`

### fish
- Generate completions: `atomwrite completions fish > ~/.config/fish/completions/atomwrite.fish`
- Available immediately in new shells

### PowerShell
- Generate completions: `atomwrite completions powershell > $HOME\Documents\PowerShell\Scripts\atomwrite.ps1`
- Source: `. $HOME\Documents\PowerShell\Scripts\atomwrite.ps1`


## File Paths and XDG
- atomwrite uses absolute paths in all NDJSON output
- Relative paths in arguments are resolved against the workspace root
- `--workspace` defaults to the current working directory
- No XDG directories are created or used
- No configuration files are read from disk
- Backup files are stored alongside the original with a timestamp suffix


## Performance by Target
### x86_64-unknown-linux-gnu
- Fastest target for all operations
- Full SIMD acceleration for BLAKE3 hashing
- Parallel search scales linearly with core count
- Typical write latency: <1ms for files under 1 MiB

### aarch64-unknown-linux-gnu
- NEON acceleration for BLAKE3 hashing
- Performance comparable to x86_64 on modern ARM cores
- Suitable for ARM servers and Raspberry Pi 4+

### x86_64-apple-darwin / aarch64-apple-darwin
- Apple Silicon provides excellent single-core performance
- `F_FULLFSYNC` adds ~0.5ms overhead per write versus standard fsync
- The overhead is the cost of real durability

### x86_64-pc-windows-msvc
- `FlushFileBuffers` overhead varies by storage driver
- NVMe drives: <1ms per write
- Spinning drives: 5-15ms per write due to physical flush


## Agents Validated per Platform
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
