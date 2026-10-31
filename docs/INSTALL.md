# Installation Guide


[Leia em Português](INSTALL.pt-BR.md)

- Complete instructions for installing atomwrite on Linux, macOS, and Windows
- Current target version: v0.1.37 (risk assessment on every writing verb; typed exit 69 `GRAMMAR_UNAVAILABLE`; `write.content_risk_patterns` reachable by `config`; workspace `[log]` honoured; 47 subcommands; slim/full feature builds)
- Sections ordered by platform, with prerequisites and troubleshooting


## What's New in v0.1.37
- Risk assessment: `edit`, `edit-loop`, `replace`, `case`, `transform` and `codemod` publish `risk_assessment` when the payload matches a content-risk pattern, the same field `write` already had
- New exit status `69` `GRAMMAR_UNAVAILABLE` with class `precondition_failed`: a grammar that is missing and cannot be downloaded names the language instead of reporting exit 255 `INTERNAL_ERROR`
- `write.content_risk_patterns` is a list value in `config get` / `config set`, so the risk patterns are XDG configuration; the table holds 37 keys
- `log.file` and `log.rotation` are accepted in the workspace scope too, and the `[log]` section of `.atomwrite.toml` now starts the log sink
- `batch --transaction` publishes one verdict: the `rollback` event carries `ok: false` with the `code` and `exit` of the failure that caused it
- `atomwrite doctor` publishes the check `grammar_cache` with how many of the 23 grammars the AST verbs can need are already cached


## What's New in v0.1.37: Agent Contract
- Agent-contract residual: `scratch init`, content-source jail, global `--select`/`--fields`, `--filter`, `--limit` (long-only; no global `-n`)
- Error envelopes always emit (`ok:false`); `--filter` does not silence failures
- SSOT surface: 47 subcommands — `atomwrite agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`
- New `config` verb (`path|list|get|set|unset`, 37 keys, `--scope global|workspace` required on `path`, `set` and `unset`); `cargo xtask check-features` checks the slim feature sets
- `atomwrite --version` prints `atomwrite 0.1.37 (no-git) <target triple>` at that release (the source is `no-git` when no git repository tracks this crate, otherwise the short commit id). Since v0.1.37 the string no longer carries a `-dirty` suffix, so an untracked file cannot change it
- Pin agents to `^0.1.37`. Suites: `cargo test --test cli_v0137_config_verb --test cli_v0137_terminal_event --test cli_v0137_version`. Decisions: ADR-0058 onward (index in `docs/decisions/README.md`)
- Minimum Rust is 1.90 (`rust-version` in `Cargo.toml`, edition 2024). The repository `rust-toolchain.toml` pins the 1.94 channel for contributors
- The `watch` subcommand is not in the default build: add `--features watch` to `cargo install` (the default binary exits 78 with a rebuild hint)
- The `ast` feature downloads the tree-sitter parsers at run time on first use (see Parsers Downloaded at Run Time)
- Release sizes measured on 2026-10-31: slim 10.40 MiB and default 54.45 MiB (see Slim vs Full Feature Builds)
- New in this guide: Files and Directories (XDG), Shell Completions, Man Pages and Verification Status (what was not validated)


## 47 Subcommands (SSOT)

After install, discover the full surface with:

```bash
atomwrite agent-surface
atomwrite commands --include-globals
atomwrite doctor --strict
```

Compact inventory of all 47 names (`CANONICAL_SUBCOMMAND_COUNT`), each with a one-line summary and an example that was run against `atomwrite 0.1.37`:

- Setup of the examples: `WS` is the absolute path of an empty workspace that holds `src/a.rs` (an `unwrap()` and a comment), `src/b.rs` (the word `beta` once), `src/c.rs` (with `user_id`), `src/tmp.rs`, `config.toml` (a `[package]` table with `name` and a `[features]` table with `x`), `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` and `rules.yaml` (a YAML list with `id`, `language`, `pattern` and `rewrite`)
- `HASH_A` is the `checksum` that `hash src/a.rs` prints
- The examples run in the order below: `rollback` needs the `backup` before it and `edit-loop` needs the `edit` before it
- Result: 46 of the 47 exit 0, and `watch` exits 78 because the default build has no `watch` feature

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
- `prune-backups` — Remove old .bak files by age or count. Example: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run .`
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
- `config` — Read and write persistent tuning keys (37 keys, scopes global and workspace). Example: `atomwrite --workspace "$WS" config get fuzzy.threshold`


## What's New in v0.1.35
- Residual A-* gaps; large overwrite `--ack-overwrite`; watch_summary; semantic-merge markers default ON
- Historical pin `^0.1.35` (superseded by 0.1.36). One-shot fuzzy / timeout 120 / exit 124 from 0.1.33–0.1.34 remain


## What's New in v0.1.12

This section summarizes installation-relevant changes in v0.1.12.

### Installation (Linux/macOS/Windows)

The v0.1.12 release is a drop-in replacement for v0.1.4. Install with:

```bash
cargo install atomwrite --locked --version "^0.1.12"
```

The Windows 10/11 fix from v0.1.4 is preserved (cargo install now succeeds). v0.1.12 adds:

- 6 new subcommands (`set`, `get`, `del`, `case`, `query`, `outline`) via Tier 3 v14 implementation
- G72 REAL syntax check via tree-sitter (24 languages via `tree-sitter-language-pack`)
- G114 WAL sidecar for crash recovery (consultive, no auto-replay)
- 5 new error codes (83 LockTimeout, 88 SyntaxError, 91 ExdevFallbackDisabled, 92 CopyBackBlake3Failed, 93 OrphanJournal)

### New Dependency
- `tree-sitter-language-pack = "1.22.1"` with `download` + `dynamic-loading` features
- Parsers download on first use and are not bundled; the measured download is 23,031,807 bytes on Linux x86_64, which is 21,97 MiB (see Parsers Downloaded at Run Time)
- No manual `cargo build` or source compilation required for language support

### Windows-Specific Notes
- v0.1.12 preserves the v0.1.4 Windows fix: `cargo install atomwrite` succeeds on Windows 10/11.
- New `init_console` improvements (from v0.1.4) ensure UTF-8 and ANSI sequences work in Windows Terminal and PowerShell 7+.
- `persist_with_retry` handles `PermissionDenied` during atomic rename with exponential backoff (Windows Defender compensation).

### Linux-Specific Notes
- v0.1.12 required Rust 1.88 or later (historical). The current minimum is Rust 1.90 (`rust-version` in `Cargo.toml`).
- `cargo install atomwrite` from crates.io is the recommended installation path.
- No system-level dependencies required beyond the standard Rust toolchain.

### macOS-Specific Notes
- v0.1.12 preserves the v0.1.2 macOS arm64 (Apple Silicon) and macOS x86_64 build fixes.
- Gatekeeper may require `xattr -d com.apple.quarantine $(which atomwrite)` on first run.
- `posix_fadvise` is correctly gated to `cfg(target_os = "linux")` only (no-op on macOS).

### Test Coverage
- Current (v0.1.37): 131 test targets ran 1486 tests with 0 failures and 9 ignored (the `cross_compile_*` tests), as recorded in `CHANGELOG.md`
- ADRs in `docs/decisions/` (0058 onward are the v0.1.37 decisions)
- The JSON schemas in `docs/schemas/`, where 54 files describe the 55 names `atomwrite --print-schema` lists
- See [docs/decisions/README.md](decisions/README.md) for architectural decisions


## Linux

### Quick Install (Ubuntu/Debian)

```bash
# Install Rust 1.90 or later via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install atomwrite v0.1.37 from crates.io
cargo install atomwrite --locked --version "^0.1.37"
# Or from a local clone:
# cargo install --path . --locked --force

# Verify
atomwrite --version
```

### Quick Install (Fedora/RHEL)

```bash
# Install build tools (the distribution Rust must be 1.90 or newer; check `rustc --version`, otherwise use rustup as above)
sudo dnf install rust cargo gcc

# Install atomwrite v0.1.37 from crates.io
cargo install atomwrite --locked --version "^0.1.37"
# Or from a local clone:
# cargo install --path . --locked --force
```

### Quick Install (Arch)

```bash
# The distribution Rust must be 1.90 or newer; check `rustc --version`, otherwise use rustup as above
sudo pacman -S rust

# Install atomwrite v0.1.37 from crates.io
cargo install atomwrite --locked --version "^0.1.37"
# Or from a local clone:
# cargo install --path . --locked --force
```


## macOS

### Quick Install

```bash
# Install Rust 1.90+ via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install atomwrite v0.1.37 from crates.io
cargo install atomwrite --locked --version "^0.1.37"
# Or from a local clone:
# cargo install --path . --locked --force

# Allow in Gatekeeper if prompted
xattr -d com.apple.quarantine $(which atomwrite) 2>/dev/null || true
```


## Windows 10 / Windows 11

### Prerequisites

1. Rust 1.90 or later — install via [rustup.rs](https://rustup.rs)
2. Visual Studio Build Tools 2019 or later with the "Desktop development with C++" workload — required for linking. Download from [visualstudio.microsoft.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
3. Windows Terminal or PowerShell 7+ (Windows Terminal recommended for UTF-8 rendering)
4. Git for Windows if installing from source

### Quick Install (from source — preferred)

```powershell
git clone https://github.com/danilo-aguiar-br/atomwrite.git
cd atomwrite
cargo install --path . --locked --force
```

### Quick Install (from crates.io)

```powershell
# Open PowerShell 7+ or Windows Terminal
rustup default stable
rustup target add x86_64-pc-windows-msvc

# Install atomwrite v0.1.37 from crates.io
cargo install atomwrite --locked --version "^0.1.37"

# Verify (expect one text line: atomwrite 0.1.37 (no-git) x86_64-pc-windows-msvc)
atomwrite --version
```

### Troubleshooting

### "error: linker 'link.exe' not found"

Install Visual Studio Build Tools with the C++ workload. The installer is at
<https://visualstudio.microsoft.com/visual-cpp-build-tools/>. After installation,
restart your terminal so the updated PATH is picked up.

### "error[E0433]: failed to resolve: use of undeclared type `AtomwriteError`"

This bug was fixed in v0.1.4. v0.1.12 is a drop-in replacement that also includes 6 new subcommands, G72 real syntax check, G114 WAL sidecar, and 5 new error codes. Make sure you install
v0.1.12 or later:

```powershell
cargo install atomwrite --locked --version "^0.1.12"
```

If you cannot upgrade, compile from source with the fix applied.

### "error: linking with `link.exe` failed: exit code: 1125"

Antivirus (Windows Defender, McAfee, etc.) is blocking the linker. Add an
exclusion for `%USERPROFILE%\.cargo` and `%USERPROFILE%\.rustup`, then retry.

### Mojibake in console output

Windows legacy console (`cmd.exe`) does not support UTF-8 by default. Use
Windows Terminal (which auto-enables UTF-8) or run:

```powershell
chcp 65001
```

before invoking atomwrite. The `init_console` function in atomwrite also
enables UTF-8 and ANSI escape sequences on Windows console handles when run
from a properly configured terminal.

### "operation not permitted" during atomic write

The atomic write pipeline retries `PermissionDenied` with full jitter (base 50 ms, ceiling 400 ms, the `retry.*` keys of `atomwrite config`) until the retry budget or `--timeout-secs` ends. If
a file is held open by another process (e.g., editor without proper file
release, antivirus scanner, indexed search), the retry may exhaust. Close
the file in the locking application and retry.


## Installing Specific Versions

```bash
# Recommended: crates.io pin for v0.1.37
cargo install atomwrite --locked --version "^0.1.37"

# From a local clone / working tree
cargo install --path . --locked --force

# Historical pin example (0.1.28): a version with no operator installs exactly that version
cargo install atomwrite --locked --version 0.1.28

# Force reinstall of the crates.io pin
cargo install atomwrite --locked --version "^0.1.37" --force

# Roll back to an exact version (a ^0.1.36 requirement would resolve to 0.1.37 again)
cargo install atomwrite --locked --version 0.1.36 --force
```

The `--locked` flag ensures `Cargo.lock` is honored, guaranteeing a reproducible
build that matches what the maintainers tested.


## Verifying Installation

```bash
atomwrite --version
# Expect one text line: atomwrite 0.1.37 (no-git) <target triple>
# The source is no-git when no git repository tracks the crate, otherwise the short commit id
```

If the command is not found, ensure `~/.cargo/bin` (Linux/macOS) or
`%USERPROFILE%\.cargo\bin` (Windows) is in your `PATH`.


## Building from Source (all platforms)

```bash
git clone https://github.com/danilo-aguiar-br/atomwrite.git
cd atomwrite
cargo build --release
./target/release/atomwrite --version
```

The release binary is at `target/release/atomwrite` (or `atomwrite.exe` on
Windows).


## Slim vs Full Feature Builds (v0.1.37)

v0.1.37 exposes Cargo features so you can trade AST size for a slim agent binary.

| Profile | Command | Approx size | Notes |
|---|---|---|---|
| slim (`core` only) | `cargo install atomwrite --locked --version "^0.1.37" --no-default-features --features core` | 10.40 MiB | local size-gate ≤15 MiB |
| default (full AST subset) | `cargo install atomwrite --locked --version "^0.1.37"` | 54.45 MiB | `core` + `ast` + `lang-rust` + `lang-ts` + `lang-py` |
| default plus `watch` | `cargo install atomwrite --locked --version "^0.1.37" --features watch` | not measured | adds the `watch` subcommand to the default build |
| `full` | `cargo install atomwrite --locked --version "^0.1.37" --features full` | not measured (larger than default) | default + `lang-full` + `watch` |

Sizes were measured with `cargo build --release --locked` for v0.1.37 on 2026-10-31: slim 10,908,040 bytes (DERIVADO: 10908040 / 1048576 = 10.40 MiB) and default 57,096,064 bytes (DERIVADO: 57096064 / 1048576 = 54.45 MiB). The slim binary answers `atomwrite 0.1.37` and exits 78 on `outline`. The `watch` and `full` profiles were not measured.

### Feature flags
- `core` — atomic read/write/edit/search/replace and other non-AST commands
- `ast` — tree-sitter / ast-grep stack (`transform`, `scope`, `query`, `outline`, `--syntax-check`)
- `lang-rust` / `lang-ts` / `lang-py` / `lang-go` — language packs for AST work
- `lang-full` — all language packs above
- `watch` — `watch` subcommand (`notify`)
- `full` — `default` + `lang-full` + `watch`
- `default` — `core` + `ast` + `lang-rust` + `lang-ts` + `lang-py` (`watch` is opt-in)
- `semantic-search` has no feature gate: it is always compiled (offline Jaccard). The old `semantic` feature does not exist any more
- `i18n-cjk`, `i18n-rtl`, `i18n-europe` and `i18n-full` are empty placeholders with no translations behind them: do not enable them
- `slow-tests` gates slow tests for contributors and has no effect on the binary

Slim build behaviour:

- `outline`, `sparse outline`, `query`, `scope`, `transform` and `codemod` exit 78 (`CONFIG_INVALID`) with a rebuild hint, never a fake success (measured on a release slim build of this tree, 10,348,424 bytes)
- The default build answers `watch` the same way: exit 78 with `watch requires rebuild: cargo install atomwrite --features watch`
- `write --syntax-check` exited 0 on the slim build for a valid Rust file, and whether the check is skipped there was not examined
- `cargo xtask check-features` runs `cargo check --locked --no-default-features` for the `core` and `core,watch` sets and stops at the first failure. `xtask` is not in the crates.io package, so run it from a source checkout

### Examples

```bash
# Slim agent install (agent-friendly)
cargo install atomwrite --locked --version "^0.1.37" --no-default-features --features core

# Default/full install with AST
cargo install atomwrite --locked --version "^0.1.37"

# Default plus the watch subcommand
cargo install atomwrite --locked --version "^0.1.37" --features watch

# Default plus watch + all languages
cargo install atomwrite --locked --version "^0.1.37" --features full

# Verify installation (47 subcommands in --help)
atomwrite --version
atomwrite --help
```


## Parsers Downloaded at Run Time
- The default build has the `ast` feature. `outline`, `sparse outline`, `query` and `write --syntax-check` need a tree-sitter parser, and the parsers are NOT bundled in the binary
- With an empty cache, the first of these commands downloads the parser bundle of the platform through `ureq`, which honours the proxy settings of the host. Measured: each of the four filled an empty cache, and `transform`, `codemod` and `scope` did not touch it
- Measured on Linux x86_64 with an empty cache on 2026-10-31: the bundle is 23,031,807 bytes (DERIVED: 23031807 / 1048576 = 21,97 MiB), the six parser libraries fetched for the languages of that run take 3,991,360 bytes (DERIVADO: 3991360 / 1048576 = 3,81 MiB), and the cache directory totals 27,053,221 bytes (DERIVADO: 27053221 / 1048576 = 25,80 MiB)
- Origin: `github.com/xberg-io/tree-sitter-language-pack/releases/download/v1.22.1/parsers-linux-x86_64.tar.zst`, and the `manifest.json` of 1.22.1 lists 372 languages and a `sha256` for each of the 6 platform bundles
- Cache: `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` with `bundles`, `libs` and `manifest.json`. It belongs to the parser crate and sits outside the `atomwrite` directories of the table below
- Later runs read the cache and use no network
- For a host without network access, run `atomwrite --workspace "$WS" outline src/a.rs` once on a connected host and copy the cache directory to the same path on the offline host
- A slim build (`--no-default-features --features core`) has no parser code and downloads nothing
- `human-panic` replaces the default panic output: when the process panics it writes a crash report as a local TOML file in the temporary directory (`report-<id>.toml`) and prints its path. Nothing is sent anywhere


## Files and Directories (XDG)
- atomwrite reads no environment variable of its own. Paths follow the platform conventions of the `directories` crate, and the `directories` and `sys-locale` crates read `HOME`, `XDG_*` and `LANG` themselves (a deliberate deviation recorded in `CHANGELOG.md`)
- Precedence of settings: command-line flags, then the `.atomwrite.toml` at the workspace root, then the global `config.toml`, then the built-in defaults. The two files are merged key by key
- Inspect and change settings only through the verb: `atomwrite --workspace "$WS" config list`, `config get KEY`, and `config set KEY VALUE --scope global|workspace` (37 keys; `--scope` is required on `path`, `set` and `unset`)

| What | Linux (measured) | macOS (read from `src/storage.rs`, not run) | Windows (read from `src/storage.rs`, not run) |
|---|---|---|---|
| Global `config.toml` (mode 0600 on Unix) | `$XDG_CONFIG_HOME/atomwrite/config.toml`, default `~/.config/atomwrite/config.toml` | `~/Library/Application Support/atomwrite/config.toml` | `%APPDATA%\atomwrite\config\config.toml` |
| Locale preference | `locale` in the same directory | same directory | same directory |
| Log file, only with `[log] file = true` | `$XDG_STATE_HOME/atomwrite/logs/atomwrite.<date>.jsonl`, default `~/.local/state/atomwrite/logs` | no state directory: falls back to the data directory | no state directory: falls back to the data directory |
| Completions from `--install` | `$XDG_DATA_HOME/` plus the shell path, default `~/.local/share/` | parent of the data directory, `~/Library/Application Support/` | parent of the data directory, `%APPDATA%\atomwrite\` |
| Parser cache | `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` | owned by the parser crate, not read here | owned by the parser crate, not read here |
| Workspace settings | `.atomwrite.toml` at the workspace root | same | same |

- `storage.home` relocates the directories: `atomwrite config set storage.home DIR --scope global` makes the global config `DIR/config/config.toml`, and data, cache and state go to `DIR/data`, `DIR/cache` and `DIR/state`. The bootstrap `config.toml` that holds `storage.home` stays at the default config path, the old file is not moved, and `config unset storage.home --scope global` undoes it
- The global scope accepts all 37 keys; `log.file`, `log.rotation`, `storage.home` and `locale` are global-only, and the workspace scope rejects them with exit 65
- On macOS and Windows the paths above come from reading the source and the `directories` documentation, and they were not run on those systems


### Config Keys (37)
- `atomwrite config list` prints the 37 keys with `value`, `origin` and `default`; `origin` is `default`, `global`, `workspace` or `explicit` (when `--config PATH` replaces both files)
- `config list` and `config get KEY` take no `--scope`; `config path`, `config set` and `config unset` require `--scope global|workspace` and exit 2 without it
- A value outside the accepted range exits 65 `INVALID_INPUT` and writes nothing; an unknown key exits 65 and lists the valid ones
- Defaults and ranges below were measured on 0.1.37 with `config list` and `config set`

| Key | Default | Accepted values | Scopes |
|---|---|---|---|
| `fuzzy.mode` | `auto` | `auto`, `aggressive`, `off` | global, workspace |
| `fuzzy.threshold` | 0.7 | 0 to 1 | global, workspace |
| `fuzzy.threshold_aggressive` | 0.5 | 0 to 1 | global, workspace |
| `fuzzy.threshold_context` | 0.8 | 0 to 1 | global, workspace |
| `fuzzy.threshold_jw` | 0.85 | 0 to 1 | global, workspace |
| `fuzzy.soft_floor_delta` | 0.05 | 0 to 1 | global, workspace |
| `fuzzy.max_content_bytes` | 268435456 | 1 to 9223372036854775807 | global, workspace |
| `write.confirm_large_bytes` | 102400 | 1 to 9223372036854775807 | global, workspace |
| `log.file` | `false` | `true`, `false` | global |
| `log.rotation` | `daily` | `daily`, `hourly` | global |
| `storage.home` | none | a directory | global |
| `locale` | none | `en`, `pt-BR` | global |
| `defaults.backup` | `true` | `true`, `false` | global, workspace |
| `defaults.line_ending` | `auto` | `auto`, `lf`, `cr-lf`, `cr` | global, workspace |
| `defaults.max_filesize` | 1073741824 | integer | global, workspace |
| `defaults.retention` | 5 | integer | global, workspace |
| `search.context` | 0 | integer | global, workspace |
| `search.max_columns` | 500 | 1 or more | global, workspace |
| `search.max_filesize` | 10485760 | 1 or more | global, workspace |
| `search.smart_case` | `true` | `true`, `false` | global, workspace |
| `watch.debounce_ms` | 200 | integer | global, workspace |
| `watch.idle_exit_ms` | 500 | integer | global, workspace |
| `write.auto_rotate_max_age_secs` | 86400 | 1 or more | global, workspace |
| `write.content_risk_patterns` | the 14 built-in patterns | list of strings | global, workspace |
| `write.shrink_block_percent` | 50 | 1 to 99 | global, workspace |
| `concurrency.max_threads` | 16 | 1 to 256 | global, workspace |
| `concurrency.ram_fraction_percent` | 50 | 1 to 100 | global, workspace |
| `search.max_inflight_bytes` | 8388608 | 4194304 to 1073741824 | global, workspace |
| `summary.errors_max_entries` | 50 | 1 to 1000 | global, workspace |
| `lock.acquire_timeout_ms` | 5000 | 100 to 3600000 | global, workspace |
| `retry.base_ms` | 50 | 1 to 60000 (not above `retry.max_ms`) | global, workspace |
| `retry.max_ms` | 400 | 1 to 60000 | global, workspace |
| `retry.max_elapsed_ms` | 2000 | 1 to 3600000 | global, workspace |
| `extract.max_sample_rows` | 10 | 0 to 100000 | global, workspace |
| `extract.max_text_chars` | 400 | 1 to 10000000 | global, workspace |
| `extract.max_sheets` | 32 | 1 to 10000 | global, workspace |
| `extract.max_slides` | 64 | 1 to 10000 | global, workspace |

- The file may carry more fields than these keys (for example `defaults.line_ending` and `defaults.max_filesize`, which are read but do nothing, and `[watch]` fields); the `config` verb manages only the 37 keys above


## Shell Completions
- `atomwrite completions SHELL` prints the script to stdout for `bash`, `zsh`, `fish`, `powershell` or `elvish`; the stdout is the script itself and not NDJSON
- `atomwrite completions SHELL --install` writes the script under the XDG data directory and prints one NDJSON event, `{"type":"installed","path":"..."}`, with the file it wrote
- Files written on Linux (measured with `XDG_DATA_HOME` set), all under that directory: `bash-completion/completions/atomwrite`, `zsh/site-functions/_atomwrite`, `fish/vendor_completions.d/atomwrite.fish`, `elvish/lib/atomwrite.elv` and `powershell/Completions/_atomwrite.ps1`
- Example: `atomwrite completions bash --install`, then open a new shell (or `source ~/.bashrc`)
- Nushell has no generator in `clap_complete`: the CLI runs under Nushell with NDJSON on stdout, but there is no completion script for it


## Man Pages
- The manual pages come from the clap definitions: `atomwrite.1` plus one page per subcommand (48 files for the 47 subcommands, measured)
- They are NOT part of the crates.io package, because the `xtask` crate is not in the tarball, and `cargo install` does not install them
- Generate them from a source checkout: `cargo xtask man --out-dir target/man` (the default directory is `target/man`)
- Install them on Linux or macOS by copying the files: `mkdir -p ~/.local/share/man/man1 && cp target/man/*.1 ~/.local/share/man/man1/`
- Then run `man atomwrite` or `man atomwrite-edit`; Windows has no `man`, so read `atomwrite --help` and `atomwrite SUBCOMMAND --help` there
- `cargo xtask man` ran on 0.1.37 and exited 0; the copy into `~/.local/share/man/man1` and the `man` lookup were not run


## Health Check (G119)

After installing v0.1.15 or later, run the WAL health commands to confirm the new subcommands are wired correctly. These are read-only or scoped-repair operations that are safe to run in local smoke tests after install.

### Inspect WAL State (read-only)

```bash
atomwrite --workspace . wal-stats
```

Expected NDJSON envelope on an empty workspace (measured on 0.1.37):

```json
{"type":"wal_stats","total_journals":0,"by_state":{"started":0,"committed":0,"aborted":0,"malformed":0},"oldest_journal_age_secs":0,"total_size_bytes":0,"by_directory":[],"auto_heal_recommended":false,"estimated_reclaim_bytes":0,"ok":true}
```

The `auto_heal_recommended` and `estimated_reclaim_bytes` fields point to terminal journals (Committed or Aborted) that `wal-heal` can remove. A journal in the `started` state is never removed by `wal-heal`, because it may belong to a write that crashed: a `kill -9` still leaves one by design.

### Reap Terminal Journals

The G119 L3 layer adds `wal-heal` to remove terminal journals (Committed and Aborted). It never touches Started entries.

```bash
# Remove all terminal journals regardless of age (safe: skips Started)
atomwrite --workspace . wal-heal . --threshold-secs 0
```

Measured output on an empty workspace: `{"type":"wal_heal","removed":0,"preserved":0,"malformed":0,"bytes_reclaimed":0,"threshold_secs":0,"ok":true,...}` with `target_resolved` and `target_source` after it.

Use this command in post-install smoke tests or after a crash recovery sweep.


## Cross-Compile Validation (for contributors)

The project includes a cross-compile gate to detect Windows-only compilation
errors before release. Run:

```bash
# Install Windows targets
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-pc-windows-msvc

# On Linux, install mingw-w64 for the GNU target
sudo dnf install mingw64-gcc mingw64-gcc-c++  # Fedora
sudo apt install mingw-w64                     # Ubuntu

# Run the cross-compile gate
cargo test --test cross_compile_check -- --ignored
```

This is required for any release that touches `#[cfg(windows)]` code paths,
per the GAP 14 fix in v0.1.4 (preserved for historical reference, but you should install v0.1.12 or later).

- `tests/cross_compile_check.rs` has nine `cross_compile_*` tests (Windows GNU x64 and i686, Windows MSVC x64 and arm64, Linux musl x64, Linux aarch64 GNU and musl, Apple aarch64 and x64), all `#[ignore]` by default
- Four of them passed on 2026-10-31 (linux musl x64, linux aarch64 gnu, linux aarch64 musl and windows gnu x64), and `cargo check` needs the rustup target but no foreign linker, so the missing `x86_64-w64-mingw32-gcc` only blocks a full `cargo build`


## Verification Status
- Validated: Linux x86_64 (`x86_64-unknown-linux-gnu`), with the final run of v0.1.37 recorded in `CHANGELOG.md`: 131 test targets, 1486 tests passed, 0 failed and 9 ignored
- Validated by compilation only: `cargo check --lib` passes for `x86_64-pc-windows-gnu`, so the `cfg(windows)` pair (SIGPIPE, `restrict_to_owner`, `persist_retry.rs`) compiles, and no Windows binary was ever run
- Not validated: every Windows and macOS execution path, because no machine of either system was used, and the Apple targets stop at `cc-rs` for want of the C toolchain
- Compiles: four `cross_compile_*` tests passed on 2026-10-31, and the other five need targets this host does not have (see Cross-Compile Validation)
- Not validated: the advisory write lock on filesystems without `flock` (NFS) and on Windows, where the lock is `LockFileEx` through `std::fs::File::try_lock` and was not run on that system
- Not validated: the rename retry of `persist_retry.rs` on Windows, which was only checked by compiling a copy of its body on Linux
- Not validated: a build with `--features watch`, and the slim build beyond the commands listed in Slim vs Full Feature Builds
- Not validated: the install commands for macOS and Windows in this guide, the Gatekeeper command and the paths of the XDG table for those systems


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
- Historical (at v0.1.20): 542 tests passing in 47 integration suites, 0 failures
- 11 GAP-2026 closed
- 3 Windows cross-compile targets green
- Historical (at v0.1.20): 19 ADRs in `docs/decisions/` (0019-0037)

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

This release closes 3 GAP-2026 items and changes default behavior for backup retention:

- `--allow-sequential-drift` on `edit` — opt-in flag for sequential edit pipelines
- `--backup` parity 4/4 — `edit` and `rollback` now accept `--backup` (was hardcoded `false`)
- `--keep-backup` on 6 subcommands — opt-in flag to preserve the backup after success
- Default change — backups are DELETED after successful `--backup` operations; add `--keep-backup` to preserve
- 555+ tests passing, 1 new ADR (0038)


## v0.1.22 — What Is New

This release adds 2 new subcommands for cleanup and N-edits-in-1-invocation patterns:

- `prune-backups [PATHS]...` — manual cleanup of legacy `.bak.YYYYMMDD_HHMMSS` siblings
  - Flags: `--max-age-secs <SECONDS>`, `--max-count <N>`, `--dry-run` (preview only; without it the old backups are removed). One of `--max-age-secs` and `--max-count` is required, and the paths are the target files whose `.bak.*` siblings are considered
  - NDJSON output with per-backup lines and summary
- `edit-loop <PATH>` — apply N `{old, new}` pairs via NDJSON on stdin in 1 invocation
  - Flags: `--workspace`, `--partial`, `--allow-sequential-drift`, `--syntax-check`, `--line-ending`, `--backup`, `--keep-backup`, `--retention` (`edit-loop` does not accept `--expect-checksum`, `--fuzzy` or `--preserve-timestamps`)
  - NDJSON output with per-pair `pair_result` and summary
- 575+ tests passing, 2 new ADRs (0039, 0040), 2 new NDJSON schemas
- 33 subcommands total (32 from v0.1.22 + `verify` from v0.1.25)

### Install v0.1.22

```bash
# From crates.io
cargo install atomwrite --locked --version "^0.1.22"

# Verify
atomwrite --version

# Smoke test the new subcommands
atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .
printf '%s\n' '{"old":"foo","new":"bar"}' \
  | atomwrite --workspace . edit-loop src/foo.rs
```
