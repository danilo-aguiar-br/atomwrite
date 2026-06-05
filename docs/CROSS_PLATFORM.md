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

### Windows (Full Support as of v0.1.4)
- File fsync: `FlushFileBuffers` via `sync_all()`
- Directory fsync: best-effort (Windows has no `fsync` for directories)
- Atomic rename via `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`
- NTFS provides reasonable durability guarantees
- **v0.1.4 fix (GAP 14)**: `cargo install atomwrite` now succeeds on Windows 10/11. Three compilation errors in `#[cfg(windows)]` blocks that broke the v0.1.3 release on Windows are resolved.
- **v0.1.4 addition**: `init_console` enables UTF-8 (code page 65001) and `ENABLE_VIRTUAL_TERMINAL_PROCESSING` so ANSI escape sequences are interpreted by the Windows Console Host. This makes colored output and Unicode characters work correctly in Windows Terminal and PowerShell 7+.
- **v0.1.4 addition**: `persist_with_retry` handles Windows-specific `PermissionDenied` errors during the atomic rename by retrying with exponential backoff (100ms, 200ms, 400ms) — this compensates for Windows Defender or other antivirus processes that briefly hold the file.
- Tested on x86_64 and i686 (i686 requires 32-bit mingw toolchain)


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
- Auto-install (v0.1.2+): `atomwrite completions bash --install` writes directly to XDG data dir
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
- `--workspace` is required when set via the `ATOMWRITE_WORKSPACE` environment variable
- Backup files are stored alongside the original with a timestamp suffix, unless `--output-dir` is set
- The `completions --install` command writes to XDG data directories (`$XDG_DATA_HOME` or `~/.local/share`)


## Build Requirements per Platform
- **Linux** (x86_64, aarch64): Rust 1.85+, standard glibc
- **macOS** (Intel, Apple Silicon): Rust 1.85+, Nix compatibility is restricted to `cfg(target_os = "linux")` so `posix_fadvise` is a no-op on macOS (added in v0.1.2 — before v0.1.2, the build failed on macOS)
- **Windows** (x86_64): Rust 1.85+, MSVC toolchain, `windows-sys` 0.61 (updated in v0.1.2)


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
- v0.1.4 prerequisite: Visual Studio 2019+ Build Tools with "Desktop development with C++" workload
- v0.1.4 prerequisite: Rust 1.85 or later
- v0.1.4 prerequisite: Windows Terminal or PowerShell 7+ for proper UTF-8 output

### x86_64-pc-windows-gnu (cross-compile from Linux)
- Cross-compile target for contributors and CI verification
- Requires mingw-w64 toolchain on the build host (`mingw64-gcc` on Fedora, `mingw-w64` on Ubuntu)
- v0.1.4 enables validation via `cargo test --test cross_compile_check -- --ignored`

### i686-pc-windows-gnu (32-bit Windows, cross-compile)
- Cross-compile target for 32-bit Windows support
- Requires `mingw32-gcc` on the build host (separate from 64-bit mingw)
- v0.1.4 enables validation via `cargo test --test cross_compile_check -- --ignored`


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
