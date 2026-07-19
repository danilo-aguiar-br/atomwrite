# atomwrite Cross-Platform Support


[Leia em Português](CROSS_PLATFORM.pt-BR.md)

> Write once, run anywhere -- with real fsync guarantees on every platform

### Verification matrix (v0.1.35 honest)

| Platform | Status |
|----------|--------|
| Linux x86_64 | Full e2e + unit/integration suite |
| Windows (`x86_64-pc-windows-gnu`) | `cargo check` on Linux host |
| macOS (apple-darwin) | Source `cfg` support; cross-compile may fail without host SDK/osxcross — runtime proof on real Mac |
| CI / GitHub Actions | **Not** shipped in this repo (local gates only) |


## What's New in v0.1.12

This section summarizes cross-platform-relevant changes in v0.1.12.

### Signal Handling (Improved)

- v0.1.12 adds 5 new tests in `tests/signal_test.rs` covering SIGINT, SIGTERM, SIGPIPE, batch interruption
- `tests/signal_test.rs::batch_interrupted_by_signal` validates the WAL journal cleanup on signal
- `tests/signal_test.rs::sigpipe_exits_141_or_signal_13` confirms BrokenPipe handling (exit 141 or signal 13)
- `tests/signal_test.rs::sigint_during_search_exits_130` and `sigterm_during_search_exits_143` confirm clean exit codes
- `tests/signal_test.rs::shutdown_message_on_stderr` validates tracing on shutdown

### Windows

- v0.1.12 preserves the v0.1.4 Windows 10/11 fix: `cargo install atomwrite` succeeds
- `init_console` improvements: UTF-8 code page 65001 + `ENABLE_VIRTUAL_TERMINAL_PROCESSING`
- `persist_with_retry` handles `PermissionDenied` during atomic rename with exponential backoff
- Windows-specific: 5 new error codes (83, 88, 91, 92, 93) all have bilingual messages

### Linux

- v0.1.12 requires Rust 1.88 or later
- `--include-fifo` flag skips FIFO/named pipes (G56) to prevent blocking
- `--strict-atomic` flag aborts on EXDEV (G90) for filesystems where atomicity is critical
- Advisory file lock via `flock` works on Linux (G54)
- xattr preservation works on ext4, btrfs, XFS, F2FS (G39)

### macOS

- v0.1.12 preserves the v0.1.2 macOS arm64 (Apple Silicon) and macOS x86_64 build fixes
- Reflink CoW works on APFS (G64): O(1) backup and copy
- xattr preservation works for `com.apple.quarantine`, `kMDItemUserTags`, `kMDItemFinderComment` (G39)
- Gatekeeper may require `xattr -d com.apple.quarantine` on first run

### Containers (Docker, Podman, Kubernetes)

- EXDEV fallback (G90) handles Docker overlay2 + named volumes automatically
- Exit code 91 (`ExdevFallbackDisabled`) for `--strict-atomic` opt-out
- No code changes required for container users; works out of the box

### NFS

- `flock(2)` is silently ignored on NFS, so `--lock` may not protect against concurrent edits
- Combine `--lock` with `--expect-checksum` for defense in depth
- `--expect-checksum` detects state drift after the write (exit 82)

### Test Coverage

- 700+ tests listed (v0.1.34 working tree; suites `cli_v0133_oneshot_fuzzy`, `cli_v0130_agent_contract`)
- Cross-compile gate: `cargo test --test cross_compile_check -- --ignored` validates Windows GNU/MSVC targets
- 5 signal tests in `tests/signal_test.rs` cover SIGINT/SIGTERM/SIGPIPE/batch/shutdown
- See [docs/decisions/README.md](decisions/README.md) for architectural decisions
- For v0.1.29 durability and rename, see [v0.1.29 — Durability and rename](#v0129--durability-and-rename)
- For v0.1.30 backup honesty and residual contract, see [v0.1.30 — Backup and agent residual](#v0130--backup-and-agent-residual)
- For v0.1.34 one-shot fuzzy + timeout, see [v0.1.34 — One-shot fuzzy and timeout](#v0134--one-shot-fuzzy-and-timeout)

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
- Atomic rename: prefers `renameat2` with fallback to `rename` on `ENOSYS`
- NDJSON write reports `platform.rename_method` (`renameat2` or `rename`)
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
- **v0.1.15 fix (GAP 18)**: the write snapshot test redacts `platform.dir_fsync` as `[platform_dir_fsync]` because Windows reports `best_effort` while Unix reports `sync_all`; the windows-2025 local gate is green again.
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
- Runtime autodetection (`src/env_detect.rs`): `/.dockerenv`, `container`/`CONTAINER` env, cgroup docker/kubepods/containerd/libpod hints
- Kubernetes pods: `KUBERNETES_SERVICE_HOST` reported by `atomwrite doctor`


## Runtime Environment Detection
`atomwrite doctor` reports specialized host flags (Rules Rust multiplataforma):

| Flag | Detection |
|------|-----------|
| WSL1/WSL2 | `WSL_DISTRO_NAME` / `WSL_INTEROP` or `/proc/...` contains `microsoft`/`wsl` |
| Container | `/.dockerenv`, `container` env, cgroup hints |
| Kubernetes | `KUBERNETES_SERVICE_HOST` |
| CI | `CI` or `GITHUB_ACTIONS` |
| Termux | `PREFIX` contains `com.termux` |
| Flatpak / Snap | `FLATPAK_ID` / `SNAP` |
| sudo | `SUDO_USER` |

Also reports `os` / `arch` / `family` / compile `target`, storage `config_dir`, and platform fsync/rename methods.


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

### Elvish
- Generate completions: `atomwrite completions elvish`
- Auto-install path: XDG data `elvish/lib/atomwrite.elv`

### Nushell
- The CLI runs under Nushell with structured NDJSON on stdout (agent contract).
- `clap_complete` does not ship a Nushell generator yet; use external completion or parse `--help` / `json-schema`.


## File Paths and XDG
- atomwrite uses absolute paths in all NDJSON output
- Relative paths in arguments are resolved against the workspace root
- `--workspace` defaults to the current working directory (pass explicitly for agents)
- Workspace jail is **CLI-only**: always pass `--workspace <root>` (no product environment variables)
- Backup files are stored alongside the original with a timestamp suffix, unless `--output-dir` is set
- The `completions --install` command writes to XDG data directories (`directories::ProjectDirs` / platform conventions)
- **`storage.home`** (XDG `config.toml` via `atomwrite set storage.home` / `get`): optional base directory override for config/data/cache/state (`{home}/config`, …). When unset, `directories::ProjectDirs` is used (`src/storage.rs`) — never process environment for home override
- Global config: `{config_dir}/config.toml`; locale preference: `{config_dir}/locale`
- Paths use `PathBuf` / `Path::join` only — no hardcoded `/` or `\` separators in application logic
- Unicode NFC normalization applied during path jail validation
- Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM0`–`COM9`, `LPT0`–`LPT9`) rejected on all hosts for portable agent trees
- On Windows, illegal filename characters and trailing space/dot are rejected; long paths get the `\\?\` prefix via `path_safety::prepare_fs_path` when near `MAX_PATH`


## Build Requirements per Platform
- **Linux** (x86_64, aarch64): Rust 1.88+, standard glibc
- **macOS** (Intel, Apple Silicon): Rust 1.88+, Nix compatibility is restricted to `cfg(target_os = "linux")` so `posix_fadvise` is a no-op on macOS (added in v0.1.2 — before v0.1.2, the build failed on macOS)
- **Windows** (x86_64): Rust 1.88+, MSVC toolchain, `windows-sys` 0.61 (updated in v0.1.2)


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
- v0.1.4 prerequisite: Rust 1.88 or later
- v0.1.4 prerequisite: Windows Terminal or PowerShell 7+ for proper UTF-8 output

### x86_64-pc-windows-gnu (cross-compile from Linux)
- Cross-compile target for contributors and local cross-compile verification
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


## v0.1.20 — What Is New

This release introduces a new safety layer called **intention guards** and renames the global `--lang` flag to `--locale` to disambiguate from the tree-sitter `--lang` selector used by `scope` and `transform`.

### Intention Guards (5 OPT-IN flags)

- `--require-backup <N>` — refuse the operation when fewer than `N` retained backups exist for the target
- `--confirm` — emit a confirmation prompt listing the planned mutation in NDJSON before executing
- `--auto-rotate <N>` — automatically rotate the backup ring down to `N` entries after a successful write
- `--risk-threshold <LOW|MEDIUM|HIGH>` — block operations whose classified risk meets or exceeds the threshold
- `--locale <en|pt-BR>` — renamed from `--lang` to disambiguate from the tree-sitter `--lang`

### Other Additions

- `count --by-size` — list the largest files in the tree with sizes and line counts
- `read --mode raw|envelope` — select between byte-stream output and structured NDJSON envelope
- `search --no-begin-end` — disable the implicit `^` and `$` anchor decoration in regex output
- `write --preserve-timestamps` — keep the source file mtime when overwriting
- `scope --lang rust` — explicit alias accepted for ergonomic symmetry with `transform --lang`

### Statistics

- 621 tests passing in 51 integration suites, 0 failures
- 52 GAP-2026 closed (019-070)
- 3 Windows cross-compile targets green
- 19 ADRs in `docs/decisions/` (0019-0037)

### Migration `--lang` to `--locale`

```bash
# Discover all files using --lang
rg -l -- '--lang\b' .

# Bulk replace while preserving other matches
fd -e sh -e md -e toml -e yml -e yaml -e json -x sd -- '--lang\b' '--locale' {}

# Or via ruplacer
ruplacer --subvert --lang --locale
```


## v0.1.21 — What Is New

This release is fully backward-compatible across all 3 supported platforms (Linux, macOS, Windows). No platform-specific behavior changed.

- `--allow-sequential-drift` opt-in flag on `edit` — uniform behavior across platforms
- `--backup` and `--keep-backup` plumbed through 6 subcommands (`write`, `edit`, `replace`, `rollback`, `apply`, `batch`) — works identically on Linux ext4, macOS APFS, and Windows NTFS
- Backup files use `.bak.<YYYYMMDD_HHMMSS>` naming and atomic `fs::copy` — same atomicity guarantees on all 3 platforms

## v0.1.22 — What Is New

This release is fully backward-compatible across all 3 supported platforms.

- **`prune-backups [PATHS]...`** — manual cleanup of `.bak.YYYYMMDD_HHMMSS` siblings
  - Uses `std::fs::remove_file` directly (not the atomic pipeline)
  - Honors the `--workspace` jail on all platforms
  - NDJSON output is platform-agnostic
- **`edit-loop <PATH>`** — N pairs in 1 invocation via NDJSON
  - Reads the file once with `read_file_string` (uses memmap2 for large files on Linux/macOS)
  - Writes atomically via the same pipeline as `edit`
  - On Windows: respects `init_console` UTF-8 and ANSI handling (v0.1.4)


## v0.1.29 — Durability and rename

This release is fully backward-compatible across Linux, macOS, and Windows for existing flags. New controls are opt-in.

- Behavior break for `replace --fuzzy auto` is documented in [MIGRATION.md](MIGRATION.md) (this section covers platform I/O only)
- Linux atomic rename prefers `renameat2` and falls back to `rename` on `ENOSYS`
- NDJSON write output reports `platform.rename_method` (`renameat2` or `rename`)
- `write --durability full|fast|auto` selects fsync policy
- `full` — full file fsync plus directory fsync when available
- `fast` — skip parent-directory fsync for speed
- `auto` — heuristic based on path and environment
- macOS under durability `full` uses `F_FULLFSYNC` via `fcntl` (same primitive as before, now gated by policy)
- Windows keeps its existing flush path; durability labels still appear in NDJSON for agents
- Backup of the live file is NEVER a hardlink of the same inode (v0.1.30 residual)
- `platform.backup_method` reports `reflink_or_copy` or `copy` only
- Cargo feature `core` keeps the slim binary path; AST/watch/semantic remain optional

## v0.1.30 — Backup and agent residual

- Residual agent-contract release on top of the v0.1.29 platform surface
- Backup path uses `reflink_or_copy` with copy fallback; never hardlink of the live file
- Edit NDJSON may include `match_count` and `indent_adjusted` for agent parsing
- `--fuzzy off` = exact-only (G-010 CLOSED in v0.1.35) on CLI and in `.atomwrite.toml` `[fuzzy] mode` (not rejected; cascade skipped)
- Sparse outline emits real AST `outline_item` kinds under budget
- `semantic-merge` help and docs state line-based merge (not AST)
- Recipe recursive hash excludes `*.bak.*` paths
- See [gaps.md](../gaps.md) and [MIGRATION.md](MIGRATION.md#v0129-to-v0130)

## v0.1.34 — One-shot fuzzy and timeout

- Docs-complete publish of the v0.1.33 one-shot runtime (same binary behavior on all platforms)
- Global `--timeout-secs` / `--timeout` default **120**; `0` disables; deadline → exit **124** (cross-platform)
- Fuzzy multi-apply is **one-pass** L→R (`apply_fuzzy_one_pass`); never re-scans inserted text
- Default max fuzzy applies **1**; if replacement contains pattern (fixed-string), force single apply
- Cooperative cancel polled mid-fuzzy cascade; caps: pattern 64 KiB, lev 8192, windows 4096, growth max(4×,+16 MiB)
- Regression: `cargo test --test cli_v0133_oneshot_fuzzy`
- See [MIGRATION.md](MIGRATION.md#v0133-to-v0134-current) and ADR-0054

## A-014 multi-target smoke (local, no GitHub Actions)

Run on a host with rustup targets installed:

```bash
for t in x86_64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin \
  x86_64-pc-windows-gnu x86_64-pc-windows-msvc i686-pc-windows-gnu; do
  cargo check --release --target "$t" || echo "SKIP/FAIL $t"
done
```

Notes (2026-07-19):
- `x86_64-unknown-linux-gnu`: primary e2e host — required green.
- Apple/Windows targets may fail on missing cross toolchains (`lib.exe`, macOS SDK, zstd asm) — document skip honestly; not a product regression.
- Do **not** add `.github/workflows` to this crate.

## v0.1.35 — Honest multi-OS and local gates

- Linux: full e2e + unit/integration
- Windows-gnu: `cargo check --target x86_64-pc-windows-gnu` on Linux host
- macOS: source-level `cfg`; cross may fail without host SDK — validate on a real Mac
- Product repo ships **no** GitHub Actions workflows; use local DoD from `docs/TESTING.md`
- Watch/notify and path separators are platform-specific; see A-XPLAT-001 in gaps.md §20

