# atomwrite Cross-Platform Support


[Leia em Português](CROSS_PLATFORM.pt-BR.md)

- One CLI contract on Linux, macOS and Windows; only Linux x86_64 is validated (see the Verification Matrix)


## Objectives
- Spend fewer tokens: one NDJSON object per call replaces the shell pipeline, and `commands --brief` answers in 5 020 bytes where the full tree costs 67 492
- Lower the financial cost, because the bill follows the tokens and the round trips, and one call replaces the several shell calls a file task needs
- Reduce errors with an atomic write and a BLAKE3 checksum on every response, so a stale or partial result is caught instead of assumed
- Reduce hallucinations with a `type` discriminator, a closed schema per verb and an exit code that names the failure
- Reduce the time spent on each task, because the read, the search and the write run in one process instead of a chain of shell tools
- Reduce latency with local execution and no network on the file path, measured at medians of 5.6 ms on `read` and 6.7 ms on `write` for small files
- Multiply the speed of operations with `batch`, `edit-loop`, `replace`, `case`, `transform` and `codemod` settling many files in one call
- Cut the lead time between the request and the verified result, because the response already carries the checksum, the `target_resolved` and the next step
- Make the agent faster, more effective and more assertive, since the exit code, the `code` and the `suggestion` say what to do next without a retry guess


## Verification Matrix (v0.1.38)

| Target | Status |
|--------|--------|
| `x86_64-unknown-linux-gnu` | Validated: built and tested end to end (131 test targets, 1486 tests passed, 0 failed, 9 ignored, as recorded in `CHANGELOG.md`) |
| `x86_64-unknown-linux-musl` | Compiles: `cross_compile_linux_musl_x64_succeeds` passed on 2026-10-31 with `cargo check --lib`; no binary was run |
| `aarch64-unknown-linux-gnu` | Compiles: `cross_compile_linux_aarch64_gnu_succeeds` passed on 2026-10-31 with `cargo check --lib`; no binary was run |
| `aarch64-unknown-linux-musl` | Compiles: `cross_compile_linux_aarch64_musl_succeeds` passed on 2026-10-31 with `cargo check --lib`; no binary was run |
| `x86_64-apple-darwin` | NOT validated: the Apple target is installed but the C toolchain is missing, and `cc-rs` fails while building `libmimalloc-sys` and `zstd-sys` with `-arch arm64` |
| `aarch64-apple-darwin` | NOT validated: the Apple target is installed but the C toolchain is missing, and `cc-rs` fails while building `libmimalloc-sys` and `zstd-sys` with `-arch arm64` |
| `x86_64-pc-windows-msvc` | `cross_compile_windows_msvc_succeeds` exists; the target is not installed on this host, so NOT validated |
| `aarch64-pc-windows-msvc` | `cross_compile_windows_arm64_msvc_succeeds` exists; the target is not installed on this host, so NOT validated |
| `x86_64-pc-windows-gnu` | Compiles: `cross_compile_windows_gnu_x64_succeeds` passed on 2026-10-31, which also proves the `cfg(windows)` paths compile; no binary was run |
| `i686-pc-windows-gnu` | `cross_compile_windows_gnu_i686_succeeds` exists; the target is not installed on this host, so NOT validated |
| Remote automation | Not shipped in this repository (local gates only) |

- Four of the nine `cross_compile_*` tests passed on 2026-10-31 (linux musl x64, linux aarch64 gnu, linux aarch64 musl and windows gnu x64), the two Apple ones stopped at the C toolchain, and the three MSVC or i686 ones need targets this host does not have
- Binary size, measured on release builds for `x86_64-unknown-linux-gnu` on 2026-11-04: slim 25.33 MiB and default 67.79 MiB (see `docs/INSTALL.md`). Other targets were not measured
- Startup time per target was not measured
- Not validated either: the advisory write lock on filesystems without `flock` (NFS) and on Windows, where the lock now targets the file handle and a sharing violation may block the rename, the Windows rename retry of `persist_retry.rs`, and every Windows and macOS path in this guide
- The `ast` feature downloads the tree-sitter parsers at run time on first use, on every platform (23,031,807 bytes measured on Linux x86_64, which is 21,97 MiB; see `docs/INSTALL.md`)
- `read --ocr` downloads the two recognition models on first use, 2.5 MiB for detection and 9.7 MiB for recognition measured on Linux x86_64, and caches them under the platform cache directory, so a network-free run needs one warm call first
- The OCR, PDF render and picture paths were exercised on Linux x86_64 only, and no Windows or macOS run has measured them

Note for all platforms: the workspace jail for content-source paths (`--old-file`/`--new-file`) is OS-agnostic. Stage under `$WS/.atomwrite/scratch/` via `scratch init`, and never rely on the OS `/tmp` outside `--workspace`. The agent-native stdout shape (`--select`/`--filter`/`--limit`) and the error envelopes (`ok:false`) are identical across platforms by design, and were exercised on Linux only.


## What's New in v0.1.12

This section summarizes cross-platform-relevant changes in v0.1.12.

### Signal Handling (Improved)
- v0.1.12 adds 5 new tests in `tests/signal_test.rs` covering SIGINT, SIGTERM, SIGPIPE, batch interruption
- `tests/signal_test.rs::batch_interrupted_by_signal` validates the WAL journal cleanup on signal
- `tests/signal_test.rs::sigpipe_handler_ends_the_run_with_exit_141` confirms that a closed pipe ends the run with exit 141 (the test was named `sigpipe_exits_141_or_signal_13` in v0.1.12)
- `tests/signal_test.rs::sigint_during_search_exits_130` and `sigterm_during_search_exits_143` confirm clean exit codes
- `tests/signal_test.rs::shutdown_message_on_stderr` validates tracing on shutdown

### Windows
- v0.1.12 preserves the v0.1.4 Windows 10/11 fix: `cargo install atomwrite` succeeds
- `init_console` improvements: UTF-8 code page 65001 + `ENABLE_VIRTUAL_TERMINAL_PROCESSING`
- `persist_with_retry` handles `PermissionDenied` during atomic rename with retries (v0.1.12 used exponential backoff; see the v0.1.37 notes for the current policy)
- v0.1.12 added 5 error codes (83, 88, 91, 92, 93) with bilingual messages, and none of them is Windows-specific

### Linux
- v0.1.12 required Rust 1.88 or later (historical). The current minimum is Rust 1.90
- FIFO/named pipes are skipped by default (G56) to prevent blocking; `search --include-fifo` opens them again, and `write` to a FIFO exits 85 `FIFO_DETECTED`
- The EXDEV copy fallback (G90) is switched off by `--strict-atomic` on a writing verb, which sets the library option `WriteOptions.strict_atomic`; the write then exits 91 instead of copying across filesystems
- Advisory file lock via `flock` works on Linux (G54)
- xattr preservation works on ext4, btrfs, XFS, F2FS (G39)

### macOS
- v0.1.12 preserves the v0.1.2 macOS arm64 (Apple Silicon) and macOS x86_64 build fixes
- Reflink CoW works on APFS (G64): O(1) backup and copy
- xattr preservation works for `com.apple.quarantine`, `kMDItemUserTags`, `kMDItemFinderComment` (G39)
- Gatekeeper may require `xattr -d com.apple.quarantine` on first run

### Containers (Docker, Kubernetes)
- EXDEV fallback (G90) handles Docker overlay2 + named volumes automatically
- Exit code 91 (`ExdevFallbackDisabled`) belongs to the library-only opt-out of the fallback and is not reachable from the command line
- No code changes required for container users; works out of the box

### NFS
- `flock(2)` is silently ignored on NFS, so the automatic write lock may not protect against concurrent edits, and the lock is taken on the target inode, not on a sidecar name (ADR-0079)
- Combine the automatic lock with `--expect-checksum` for defense in depth
- `--expect-checksum` detects state drift after the write (exit 82)

### Test Coverage
- Final run of v0.1.38 recorded in `CHANGELOG.md`: 131 test targets, 1486 tests passed, 0 failed, 9 ignored (the `cross_compile_*` tests)
- Cross-compile gate: `cargo test --test cross_compile_check -- --ignored` runs nine target checks; none was run for v0.1.38 (see the Verification Matrix)
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
- Tested on x86_64 only; the aarch64 target has a cross-compile test that was not run

### macOS (source support, not validated)
- File fsync: `F_FULLFSYNC` via `fcntl` for true durability
- Directory fsync: `sync_all()` on parent directory
- Standard `fsync` on macOS does NOT guarantee disk write
- atomwrite uses `F_FULLFSYNC` automatically
- Status for v0.1.37: not run on macOS, neither on Apple Silicon nor on Intel; the lines above describe the source behaviour

### Windows (source support, not validated)
- File fsync: `FlushFileBuffers` via `sync_all()`
- Directory fsync: best-effort (Windows has no `fsync` for directories)
- v0.1.15 fix (GAP 18): the write snapshot test redacts `platform.dir_fsync` as `[platform_dir_fsync]` because Windows reports `best_effort` while Unix reports `sync_all` (historical, not re-run on Windows)
- Atomic rename via `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`
- NTFS provides reasonable durability guarantees
- v0.1.4 fix (GAP 14): `cargo install atomwrite` succeeded on Windows 10/11 after three compilation errors in `#[cfg(windows)]` blocks were resolved (historical, not re-run for v0.1.37)
- v0.1.4 addition: `init_console` enables UTF-8 (code page 65001) and `ENABLE_VIRTUAL_TERMINAL_PROCESSING` so ANSI escape sequences are interpreted by the Windows Console Host (not re-run for v0.1.37).
- Current retry policy (v0.1.37): the tempfile persist retries `PermissionDenied` and `ERROR_SHARING_VIOLATION` (32) with full jitter (base 50 ms, ceiling 400 ms, the `retry.*` keys of `atomwrite config`) and still obeys `--timeout-secs`. This compensates for antivirus processes that briefly hold the file. In v0.1.4 the delays were fixed at 100 ms, 200 ms and 400 ms
- Status for v0.1.37: not run on Windows (neither x86_64 nor i686); the retry was checked only by compiling a copy of its body on Linux


## Signal Handling
### Linux and macOS
- SIGINT (130): graceful shutdown, flush pending writes
- SIGTERM (143): graceful shutdown, flush pending writes
- SIGHUP (129): a closed terminal ends the run as a cancellation, with `signal: "SIGHUP"` on the `cancelled` envelope
- `--timeout-secs` elapsed (124): the `cancelled` envelope carries `deadline_exceeded: true`
- SIGPIPE (141): SIGPIPE is NOT reset to `SIG_DFL`. The process installs a cooperative handler that marks a shutdown with exit 141 (a deliberate deviation from the literal rule, ADR-0063). The write that hits the closed pipe fails with `BrokenPipe`, every `Drop` runs and no lock, journal or tempfile is left. An in-flight write finishes and commits
- Measured on Linux with the default build: `read big.txt | head -c 5` and `search PATTERN big.txt | head -1` exit 141

### Windows
- Ctrl+C: handled via SetConsoleCtrlHandler
- SIGPIPE: not applicable on Windows. `std` maps `ERROR_BROKEN_PIPE` to `BrokenPipe`, which follows the same path to exit 141 (not validated on Windows)
- Process termination: pending atomic writes are abandoned safely


## Containers
- Docker: works out of the box with standard Linux images
- Overlay filesystems: atomic rename works within overlay layers
- Volume mounts: fsync reaches the host filesystem through the mount
- tmpfs: fsync is a no-op but rename is still atomic
- Cross-container moves: use `--workspace` to prevent escaping the mount
- Status for v0.1.37: no container run was recorded; the bullets above describe the source behaviour
- Runtime autodetection (`src/env_detect.rs`): `/.dockerenv` and cgroup docker/kubepods/containerd/libpod hints, read from files only
- Kubernetes pods: the service-account mount `/var/run/secrets/kubernetes.io/serviceaccount` is reported by `atomwrite doctor`


## Runtime Environment Detection
`atomwrite doctor` reports specialized host flags. Detection reads files only and no environment variable (Rules Rust multiplataforma):

| Flag | Detection |
|------|-----------|
| WSL1/WSL2 | `/proc/sys/kernel/osrelease` or `/proc/version` contains `microsoft` |
| Container | `/.dockerenv` or `/proc/1/cgroup` (fallback `/proc/self/cgroup`) with docker/kubepods/containerd/libpod hints |
| Kubernetes | directory `/var/run/secrets/kubernetes.io/serviceaccount` exists |
| Termux | directory `/data/data/com.termux/files/usr` exists |
| Flatpak / Snap | file `/.flatpak-info` exists / `/proc/self/exe` points under `/snap/` |

On Windows every file probe is skipped. `doctor` also reports `os` / `arch` / `family` / compile `target`, storage `config_dir`, and platform fsync/rename methods.


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
- The workspace jail is set by the CLI only: always pass `--workspace <root>` (no product environment variables)
- Backup files are stored alongside the original with a timestamp suffix, unless `--output-dir` is set
- The `completions --install` command writes to XDG data directories (`directories::ProjectDirs` / platform conventions)
- `storage.home` is a key of the `config` verb: `atomwrite config set storage.home DIR --scope global` and `atomwrite config get storage.home`. It relocates config, data, cache and state to `DIR/config`, `DIR/data`, `DIR/cache` and `DIR/state`. When unset, `directories::ProjectDirs` is used (`src/storage.rs`), and no process environment variable overrides the home. The table of paths per platform is in `docs/INSTALL.md` (Files and Directories)
- Global config: `{config_dir}/config.toml` (mode 0600 on Unix, ACL of the user profile on Windows); locale preference: `{config_dir}/locale`
- `completions SHELL --install` writes under the parent of the data directory (`~/.local/share/` on Linux with the default XDG setting), and under `{home}` when `storage.home` is set
- Paths use `PathBuf` / `Path::join` only — no hardcoded `/` or `\` separators in application logic
- Unicode NFC normalization applied during path jail validation
- Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM0`–`COM9`, `LPT0`–`LPT9`) rejected on all hosts for portable agent trees
- On Windows, illegal filename characters and trailing space/dot are rejected; long paths get the `\\?\` prefix via `path_safety::prepare_fs_path` when near `MAX_PATH`


## Build Requirements per Platform
- Linux (x86_64, aarch64): Rust 1.90+, standard glibc
- macOS (Intel, Apple Silicon): Rust 1.90+, Nix compatibility is restricted to `cfg(target_os = "linux")` so `posix_fadvise` is a no-op on macOS (added in v0.1.2 — before v0.1.2, the build failed on macOS)
- Windows (x86_64): Rust 1.90+, MSVC toolchain, `windows-sys` 0.61 (updated in v0.1.2)
- Only the Linux x86_64 build was run for v0.1.37 (see the Verification Matrix)


## Performance by Target
- The figures below are estimates carried from earlier releases. None was measured for v0.1.37, and only Linux x86_64 was run at all

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
- v0.1.4 prerequisite: Rust 1.88 or later (historical; the current minimum is Rust 1.90)
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
- No agent-specific test is recorded for any platform, so this guide names no agent as validated
- The integration route is the CLI plus NDJSON through a subprocess, and the test suite exercises that route on Linux x86_64 through `assert_cmd`
- macOS and Windows: no agent run and no CLI run was recorded for v0.1.37


## v0.1.20 — What Is New

This release introduces a new safety layer called intention guards and renames the global `--lang` flag to `--locale` to disambiguate from the tree-sitter `--lang` selector used by `scope` and `transform`.

### Intention Guards (5 OPT-IN flags)
- `--require-backup` — require `--backup`: abort when it is missing and the target file exists (takes no value)
- `--confirm` — legacy awareness flag on `write`, with no effect by itself: a large overwrite needs `--ack-overwrite`, and `delete --confirm` is rejected (use `--plan`)
- `--auto-rotate` — force an automatic rotation backup for recently modified files (age from `[write].auto_rotate_max_age_secs`, default 24 h; takes no value)
- `--risk-threshold <PERCENT>` — size-delta threshold in percent for the risk warning (default 255 = off; for example `--risk-threshold 50`)
- `--locale <en|pt-BR>` — renamed from `--lang` to disambiguate from the tree-sitter `--lang`

### Other Additions
- `count --by-size` — list the largest files in the tree with sizes and line counts
- `read --format raw|ndjson` — select between byte-stream output and structured NDJSON envelope (an old `--mode` spelling never existed)
- `search --no-begin-end` — disable the implicit `^` and `$` anchor decoration in regex output
- `write --preserve-timestamps` — keep the source file mtime when overwriting
- `scope --lang rust` — explicit alias of `scope --language`; `transform` takes `-l`/`--language` only

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

# Or via ruplacer (a dry run until you add --go)
ruplacer -- '--lang' '--locale' .
```


## v0.1.21 — What Is New

This release is fully backward-compatible across all 3 supported platforms (Linux, macOS, Windows). No platform-specific behavior changed.

- `--allow-sequential-drift` opt-in flag on `edit` — uniform behavior across platforms
- `--backup` and `--keep-backup` plumbed through 6 subcommands (`write`, `edit`, `replace`, `rollback`, `apply`, `batch`) — works identically on Linux ext4, macOS APFS, and Windows NTFS
- Backup files use `.bak.<YYYYMMDD_HHMMSS>` naming and atomic `fs::copy` — same atomicity guarantees on all 3 platforms


## v0.1.22 — What Is New

This release is fully backward-compatible across all 3 supported platforms.

- `prune-backups [PATHS]...` — manual cleanup of `.bak.YYYYMMDD_HHMMSS` siblings
  - Uses `std::fs::remove_file` directly (not the atomic pipeline)
  - Honors the `--workspace` jail on all platforms
  - NDJSON output is platform-agnostic
- `edit-loop <PATH>` — N pairs in 1 invocation via NDJSON
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
- Global `--timeout-secs` / `--timeout` default 120; `0` disables; deadline → exit 124 (cross-platform)
- Fuzzy multi-apply is one-pass L→R (`apply_fuzzy_one_pass`); never re-scans inserted text
- Default max fuzzy applies 1; if replacement contains pattern (fixed-string), force single apply
- Cooperative cancel polled mid-fuzzy cascade; caps: pattern 64 KiB, lev 8192, windows 4096, growth max(4×,+16 MiB)
- Regression: `cargo test --test cli_v0133_oneshot_fuzzy`
- See [MIGRATION.md](MIGRATION.md#v0133-to-v0134) and ADR-0054


## A-014 multi-target smoke (local)

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
- The loop was not re-run for v0.1.37 (see the Verification Matrix).


## v0.1.35 — Honest multi-OS and local gates
- Linux: full e2e + unit/integration
- Windows-gnu: `cargo check --target x86_64-pc-windows-gnu` on Linux host
- macOS: source-level `cfg`; cross may fail without host SDK — validate on a real Mac
- Local gates only; use the local DoD from `docs/TESTING.md`
- Watch/notify and path separators are platform-specific; see A-XPLAT-001 in ADR-0055


## v0.1.36 — Agent contract (platform-neutral)
- Historical (v0.1.36): 46 subcommands SSOT (`agent-surface` / `commands`); the current list of 47 is in the catalog below
- Content-source jail + `scratch init` (no `/tmp` allowlist) — same path policy on Linux/macOS/Windows
- Error NDJSON always includes `ok: false` + stable `match_count`
- Global `--select`/`--fields`, `--filter`, `--limit` (long-only; no global `-n`)
- Contract suite: `cargo test --test cli_v0136_gaps`; gap status lives in the ADR that owns the gap
- See ADR-0055 (GAP-2026-019…033)


## v0.1.38 — Platform notes
- Pin agents to `^0.1.38`; 47 subcommands SSOT (`agent-surface` / `commands`), the same list on Linux, macOS and Windows by design (exercised on Linux only)
- `delete` takes the target's inode lock before it hashes and unlinks, by the same `std::fs::File::try_lock` every writer uses, so a deletion and a write to one target do not interleave; a target that does not exist yet still gets the sidecar
- `read --format raw` honors `--max-filesize` on every platform, and the failure envelope carries `ok`, `code` and `exit` with no `error` field
- Windows compiles the crate without a warning, and execution on Windows and macOS was not measured


## v0.1.37 — Platform notes
- Pin agents to `^0.1.37`; 47 subcommands SSOT (`agent-surface` / `commands`), the same list on Linux, macOS and Windows by design (exercised on Linux only)
- Broken pipe: exit 141 on every platform. Unix installs a cooperative SIGPIPE handler (the signal does not kill the process), the write that hits the closed pipe fails with `BrokenPipe`, every `Drop` runs and the process leaves no lock, journal or tempfile. A parent that ignores SIGPIPE, or Windows (`ERROR_BROKEN_PIPE` mapped to `BrokenPipe`), gets the same exit 141. SIGPIPE is not reset to `SIG_DFL`: this is a deliberate deviation (ADR-0063)
- `config set --scope global` writes the XDG `config.toml` with mode `0600` on Unix; Windows has no mode bits and relies on the ACL of the user profile
- Windows tempfile persist retries `PermissionDenied` and `ERROR_SHARING_VIOLATION` (32) with full jitter (base 50 ms, ceiling 400 ms) and still obeys `--timeout-secs`; not validated on Windows
- Advisory lock (`std::fs::File::try_lock`, which is `flock` on Unix and `LockFileEx` on Windows) is taken on the target's own inode and waits up to `lock.acquire_timeout_ms` (default 5000) with the same backoff; exit 83 when it is not acquired. The sidecar `.<target>.atomwrite.lock` is taken beside it for older releases, and it is the only lock a target that does not exist yet can carry. Not validated on NFS or on Windows
- Slim build (`--no-default-features --features core`): `outline`, `sparse outline`, `query`, `scope`, `transform` and `codemod` exit 78 (measured on Linux); `cargo xtask check-features` checks the `core` and `core,watch` sets
- The default build has no `watch`: `atomwrite watch` exits 78 until the binary is built with `--features watch`
- Parsers are downloaded at run time by `outline`, `sparse outline`, `query` and `write --syntax-check` on every platform


## The 47 Subcommands on Every Platform
- The list is the same on Linux, macOS and Windows, and each line has a one-line summary and an example that was run on Linux x86_64 (`atomwrite 0.1.38` on 2026-11-04); none was run on macOS or Windows
- Setup of the examples: `WS` is the absolute path of an empty workspace that holds `src/a.rs` (an `unwrap()` and a comment), `src/b.rs` (the word `beta` once), `src/c.rs` (with `user_id`), `src/tmp.rs`, `config.toml` (a `[package]` table with `name` and a `[features]` table with `x`), `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` and `rules.yaml` (a YAML list with `id`, `language`, `pattern` and `rewrite`)
- `HASH_A` is the `checksum` that `hash src/a.rs` prints
- The examples use `printf ... |` and a POSIX shell. On Windows run the same `atomwrite` arguments from PowerShell and feed stdin with a here-string or a pipe from `Write-Output`; this was not validated
- The examples run in the order below: `rollback` needs the `backup` before it and `edit-loop` needs the `edit` before it
- Result on Linux: 46 of the 47 exit 0, and `watch` exits 78 because the default build has no `watch` feature

- `read` — Read a file with checksum, size and lines, optionally only head/tail/range. Example: `atomwrite --workspace "$WS" read --head 2 src/a.rs`
- `write` — Create or overwrite a file atomically from stdin. Example: `printf 'x\n' | atomwrite --workspace "$WS" write notes/new.txt`
- `edit` — Edit a file by exact or fuzzy text match, line number or marker. Example: `atomwrite --workspace "$WS" edit src/b.rs --old beta --new gamma`
- `delete` — Delete files or trees with backup, or only print the plan. Example: `atomwrite --workspace "$WS" delete --plan tmp.txt`
- `copy` — Copy a file or tree with checksum verification. Example: `atomwrite --workspace "$WS" copy src/a.rs src/a_copy.rs`
- `move` — Move or rename a file atomically. Example: `atomwrite --workspace "$WS" move src/tmp.rs src/tmp2.rs`
- `apply` — Apply a patch (unified diff, SEARCH/REPLACE, full file) from stdin. Example: `printf 'new body\n' | atomwrite --workspace "$WS" apply --format full notes.txt`
- `search` — Search file contents in parallel with the ripgrep engine. Example: `atomwrite --workspace "$WS" search alpha src/`
- `replace` — Replace text across files with atomic writes and fuzzy matching. Example: `atomwrite --workspace "$WS" replace --dry-run alpha omega src/`
- `transform` — Rewrite code by AST pattern (ast-grep). Example: `atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `hash` — Print the BLAKE3 checksum of files. Example: `atomwrite --workspace "$WS" hash src/a.rs`
- `count` — Count lines and files by extension or by size. Example: `atomwrite --workspace "$WS" count --by-extension src/`
- `diff` — Compare two files (unified, stat or changes). Example: `atomwrite --workspace "$WS" diff src/a.rs src/b.rs --unified`
- `list` — List a directory tree with metadata. Example: `atomwrite --workspace "$WS" list --long --depth 1 src/`
- `extract` — Pick fields from NDJSON read on stdin. Example: `printf '{"a":1}\n' | atomwrite --workspace "$WS" extract --field a`
- `extract-artifact` — Bounded signal from PDF, OOXML and image headers (never raw bytes). Example: `atomwrite --workspace "$WS" extract-artifact img.png`
- `scope` — Grammatical scoping: act on comments, strings, functions and other code categories. Example: `atomwrite --workspace "$WS" scope src/ --language rust --query comments --delete --dry-run`
- `regex` — Generate a regex from example strings. Example: `atomwrite --workspace "$WS" regex abc-12 xyz-99 id-7 --digits`
- `calc` — Math and unit conversion. Example: `atomwrite --workspace "$WS" calc "2 hours to seconds"`
- `completions` — Print or install shell completions (bash, zsh, fish, elvish, powershell). Example: `atomwrite --workspace "$WS" completions bash`
- `backup` — Create timestamped backups with BLAKE3 checksums. Example: `atomwrite --workspace "$WS" backup src/a.rs --retention 3`
- `rollback` — Restore a file from a previous backup. Example: `atomwrite --workspace "$WS" rollback src/a.rs --latest --verify`
- `batch` — Run NDJSON operations from stdin, optionally as one transaction. Example: `printf '{"op":"write","target":"batch.txt","content":"x"}\n' | atomwrite --workspace "$WS" batch --transaction`
- `edit-loop` — Apply N old/new pairs in one write, all or nothing. Example: `printf '[{"old":"gamma","new":"delta"}]\n' | atomwrite --workspace "$WS" edit-loop src/b.rs`
- `prune-backups` — Remove old .bak files by age or count. Example: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run --recursive .`
- `wal-stats` — Show the state of WAL journals (read only). Example: `atomwrite --workspace "$WS" wal-stats`
- `wal-heal` — Remove orphan terminal WAL journals older than a threshold. Example: `atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .`
- `verify` — Check a file against an expected BLAKE3 hash (exit 0 match, 81 mismatch). Example: `atomwrite --workspace "$WS" verify src/a.rs HASH_A`
- `set` — Write a value at a dotted path in a TOML or JSON file. Example: `atomwrite --workspace "$WS" set config.toml package.name demo2`
- `get` — Read a value at a dotted path in a TOML or JSON file. Example: `atomwrite --workspace "$WS" get config.toml package.name`
- `del` — Remove a key from a TOML or JSON file. Example: `atomwrite --workspace "$WS" del config.toml features.x`
- `case` — Rename identifiers across files between case styles. Example: `atomwrite --workspace "$WS" case --subvert user_id UserId --to pascal --dry-run src/c.rs`
- `query` — Walk the tree-sitter AST and emit nodes. Example: `atomwrite --workspace "$WS" query --kinds src/a.rs`
- `outline` — Emit the high-level structure of a source file. Example: `atomwrite --workspace "$WS" outline src/a.rs`
- `semantic-merge` — Line-based three-way merge with conflict markers (not AST). Example: `atomwrite --workspace "$WS" semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict`
- `sparse` — Budgeted list, read and outline for large trees. Example: `atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576`
- `recipe` — List and run built-in multi-step recipes. Example: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern alpha --replacement omega --path src --dry-run`
- `stat` — Metadata only, alias of read --stat. Example: `atomwrite --workspace "$WS" stat src/a.rs`
- `agent-surface` — Tool manifest for agents (the SSOT of subcommands). Example: `atomwrite --workspace "$WS" agent-surface --format json`
- `watch` — Stream filesystem events with debounce; ends with watch_summary (needs a build with `--features watch`, the default build exits 78). Example: `atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500`
- `codemod` — Apply a YAML list of AST rewrite rules across a tree. Example: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- `semantic-search` — Offline Jaccard ranking of files by query. Example: `atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01`
- `doctor` — Host readiness diagnostics. Example: `atomwrite --workspace "$WS" doctor`
- `scratch` — Create the staging dir .atomwrite/scratch inside the workspace jail. Example: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch`
- `locale` — Show, set or clear the UI locale. Example: `atomwrite --workspace "$WS" locale`
- `commands` — Print the full command tree as JSON. Example: `atomwrite --workspace "$WS" commands --include-globals`
- `config` — Read and write persistent tuning keys (43 keys, scopes global and workspace). Example: `atomwrite --workspace "$WS" config get fuzzy.threshold`

### Platform Differences per Command
- `write`, `edit`, `edit-loop`, `set`, `del`, `apply`, `config` and the `replace` and `edit` ops of `batch`: they serialise on the advisory lock, which works on Unix and on Windows (NFS: not validated)
- `case`, `transform`, `scope` and `codemod`: a file that changed between the read and the write fails with `STATE_DRIFT` (exit 82); the same on every platform
- `move` and `copy`: reflink CoW on APFS, btrfs and XFS, `reflink_or_copy` as the backup method, extended attributes preserved on Unix; none of this was validated on Windows
- `backup`, `rollback`, `prune-backups`: the backup name is `<name>.bak.<YYYYMMDD_HHMMSS>` on every platform
- `completions`: with `--install` the files go under the parent of the data directory of the platform (see `docs/INSTALL.md`)
- `doctor`: every file probe is skipped on Windows (WSL, container, Kubernetes, Termux, Flatpak and Snap are Linux concepts)
- `watch`: needs `--features watch` and uses the `notify` crate, whose backend depends on the operating system; `cargo xtask check-features` only compiles the `core,watch` set, and a run of `watch` was not done in this audit on any platform
- `read`, `write`, `delete`: names that are reserved on Windows (`CON`, `PRN`, `AUX`, `NUL`, `COM0` to `COM9`, `LPT0` to `LPT9`) are rejected on all hosts, so an agent tree stays portable
- `locale` and `config`: the locale preference and the global config live in the config directory of the platform


## Every local flag, by command
- This list names every local flag of each command, and the global flags stay in `atomwrite --help`.
- `agent-surface`: `--format`
- `apply`: `--ack-overwrite`, `--all`, `--allow-shrink`, `--backup`, `--dry-run`, `--format`, `--keep-backup`, `--no-backup`, `--no-syntax-check`, `--retention`, `--show`, `--strict-atomic`
- `backup`: `--dry-run`, `--output-dir`, `--retention`
- `batch`: `--backup`, `--batch-size`, `--dry-run`, `--file`, `--input-schema`, `--keep-backup`, `--no-backup`, `--retention`, `--strict-atomic`, `--transaction`
- `calc`: `--stdin`
- `case`: `--backup`, `--dry-run`, `--exclude`, `--include`, `--keep-backup`, `--no-backup`, `--preserve-timestamps`, `--retention`, `--strict-atomic`, `--subvert`, `--to`
- `codemod`: `--backup`, `--dry-run`, `--keep-backup`, `--no-backup`, `--retention`, `--rules`, `--strict-atomic`
- `commands`: `--brief`, `--include-globals`
- `completions`: `--install`
- `config`: no local flag of its own, only the global set
- `copy`: `--backup`, `--dry-run`, `--force`, `--keep-backup`, `--no-backup`, `--no-reflink`, `--preserve-xattr`, `--preserve`, `--recursive`, `--retention`, `--strict-atomic`
- `count`: `--by-extension`, `--by-size`, `--exclude`, `--include`, `--top`
- `del`: `--backup`, `--dry-run`, `--force-missing`, `--keep-backup`, `--no-backup`, `--preserve-timestamps`, `--retention`, `--strict-atomic`
- `delete`: `--backup`, `--confirm`, `--dry-run`, `--exclude`, `--include`, `--keep-backup`, `--no-backup`, `--older-than`, `--plan`, `--recursive`, `--retention`, `--yes`
- `diff`: `--algorithm`, `--context`, `--stat`, `--unified`
- `doctor`: `--strict`
- `edit`: `--after-line`, `--after-match`, `--allow-sequential-drift`, `--backup`, `--before-line`, `--before-match`, `--between`, `--delete-range`, `--dry-run`, `--expect-checksum`, `--fuzzy-threshold`, `--fuzzy`, `--keep-backup`, `--line-ending`, `--multi`, `--new-file`, `--new`, `--no-backup`, `--old-file`, `--old`, `--partial`, `--preserve-timestamps`, `--range`, `--replace-all`, `--retention`, `--strict-atomic`, `--wal-policy`
- `edit-loop`: `--allow-sequential-drift`, `--backup`, `--keep-backup`, `--line-ending`, `--no-backup`, `--partial`, `--retention`, `--strict-atomic`, `--syntax-check`
- `extract`: `--delimiter`, `--field`, `--stdin`
- `extract-artifact`: `--max-sample-rows`, `--max-text-chars`
- `get`: no local flag of its own, only the global set
- `hash`: `--exclude`, `--recursive`, `--stdin`, `--verify`
- `list`: `--all`, `--count-by-ext`, `--depth`, `--exclude`, `--format`, `--include`, `--long`
- `locale`: `--clear`, `--set`
- `move`: `--backup`, `--dry-run`, `--force`, `--keep-backup`, `--no-backup`, `--preserve-hardlinks`, `--retention`
- `outline`: `--format`, `--kind`, `--language`, `--positions`
- `prune-backups`: `--dry-run`, `--max-age-secs`, `--max-count`, `--recursive`
- `query`: `--kinds`, `--language`, `--positions`, `--query`, `--tree`
- `read`: `--all`, `--bytes`, `--context`, `--crop`, `--dpi`, `--enclosing`, `--force`, `--format`, `--grep`, `--head`, `--high-res`, `--line`, `--lines`, `--max-tokens`, `--no-comments`, `--number`, `--ocr`, `--page`, `--pages`, `--stat`, `--symbol`, `--tail`, `--verify-checksum`
- `recipe`: no local flag of its own, only the global set
- `regex`: `--case-insensitive`, `--digits`, `--no-anchors`, `--repetitions`, `--spaces`, `--stdin`, `--words`
- `replace`: `--backup`, `--dry-run`, `--exclude`, `--expect-checksum`, `--fuzzy-threshold`, `--fuzzy`, `--include`, `--keep-backup`, `--literal`, `--max-replacements`, `--no-backup`, `--preserve-case`, `--preserve-timestamps`, `--preview`, `--progress-every`, `--regex`, `--retention`, `--strict-atomic`, `--word`
- `rollback`: `--backup`, `--dry-run`, `--keep-backup`, `--latest`, `--no-backup`, `--retention`, `--strict-atomic`, `--timestamp`, `--verify`
- `scope`: `--action`, `--backup`, `--delete`, `--dry-run`, `--exclude`, `--include`, `--keep-backup`, `--language`, `--no-backup`, `--pattern`, `--query`, `--replace-with`, `--retention`, `--strict-atomic`
- `scratch`: no local flag of its own, only the global set
- `search`: `--binary`, `--case-insensitive`, `--context`, `--count`, `--exclude`, `--files`, `--fixed`, `--format`, `--include-fifo`, `--include`, `--invert`, `--limit`, `--max-columns`, `--max-count`, `--max-filesize`, `--multiline`, `--no-begin-end`, `--offset`, `--pcre2`, `--regex`, `--smart-case`, `--sort`, `--target`, `--word`
- `semantic-merge`: `--backup`, `--base`, `--dry-run`, `--expect-checksum`, `--fail-on-conflict`, `--keep-backup`, `--no-backup`, `--no-conflict-markers`, `--ours`, `--output`, `--retention`, `--strict-atomic`, `--theirs`, `--write-conflict-markers`
- `semantic-search`: `--include-backups`, `--index-dir`, `--k`, `--min-score`
- `set`: `--backup`, `--dry-run`, `--keep-backup`, `--no-backup`, `--preserve-timestamps`, `--retention`, `--strict-atomic`
- `sparse`: no local flag of its own, only the global set
- `stat`: `--all`, `--bytes`, `--context`, `--crop`, `--dpi`, `--enclosing`, `--force`, `--format`, `--grep`, `--head`, `--high-res`, `--line`, `--lines`, `--max-tokens`, `--no-comments`, `--number`, `--ocr`, `--page`, `--pages`, `--stat`, `--symbol`, `--tail`, `--verify-checksum`
- `transform`: `--backup`, `--dry-run`, `--exclude`, `--include`, `--inline-rules`, `--keep-backup`, `--language`, `--no-backup`, `--pattern`, `--retention`, `--rewrite`, `--rules`, `--strict-atomic`, `--verify-parse`
- `verify`: no local flag of its own, only the global set
- `wal-heal`: `--dry-run`, `--max-duration-ms`, `--threshold-secs`
- `wal-stats`: `--dry-run`
- `watch`: `--checksum`, `--debounce-ms`, `--gitignore`, `--idle-exit-ms`, `--max-events`
- `write`: `--ack-overwrite`, `--allow-empty-stdin`, `--allow-shrink`, `--append`, `--auto-rotate`, `--backup`, `--confirm`, `--dry-run`, `--durability`, `--expect-checksum`, `--keep-backup`, `--line-ending`, `--max-size`, `--no-backup`, `--no-checksum-when-empty`, `--prepend`, `--preserve-timestamps`, `--require-backup`, `--require-large-ack`, `--retention`, `--risk-threshold`, `--strict-atomic`, `--syntax-check`, `--wal-policy`
