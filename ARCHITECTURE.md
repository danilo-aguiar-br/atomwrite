[Leia em Portugues](ARCHITECTURE.pt-BR.md)


# Architecture


## Overview
- atomwrite is a single Rust binary CLI for atomic file operations
- Designed for LLM agents that need safe, structured file manipulation
- Every write is atomic: tempfile, fsync, rename, fsync directory
- Every response is NDJSON on stdout with BLAKE3 checksums
- All diagnostic logs go to stderr via `tracing` (`init_telemetry` in `src/runtime.rs`): non-blocking writer, `EnvFilter` (`ATOMWRITE_LOG` > `RUST_LOG` > `-v`/`-q`), optional `ATOMWRITE_LOG_DIR` tee with `Rotation::NEVER`, JSON via `ATOMWRITE_LOG_FORMAT`. Agent data plane remains NDJSON on stdout.


## Module Map

### Entry Point
- `src/main.rs` — binary entry: signal setup, tracing init, dispatch
- `src/lib.rs` — library root: module declarations and `run()` dispatcher. Wires the G119 L3 startup `wal-heal` pass via `auto_heal_on_startup` before any subcommand dispatch
- `src/cli.rs` — clap `#[derive(Parser)]` with global flags
- `src/cli_args.rs` — per-subcommand argument structs and value enums

### Core Pipeline
- `src/atomic.rs` — atomic write pipeline: tempfile + fsync + rename + fsync dir
- `src/checksum.rs` — BLAKE3 hash computation for files and byte slices (uses memmap2 for large files)
- `src/file_io.rs` — smart file reading with automatic memmap2 above 1 MiB threshold
- `src/platform.rs` — platform durability and rename: `Durability` full|fast|auto; Linux `renameat2` with rename fallback; macOS `F_FULLFSYNC` on full; write NDJSON reports `platform.durability` and `platform.rename_method`
- `src/fuzzy.rs` — shared 9-strategy match cascade (exact, whitespace, indent, Jaro-Winkler, unicode_normalized, …) used by edit, replace, batch, edit-loop; **one-pass** multi-apply (`apply_fuzzy_one_pass`) on original content (never re-scans inserted text); embeds of pattern inside replacement force a single apply; default max applies = 1; hard ceiling 10_000; cooperative cancel polled mid-cascade; caps (pattern 64 KiB, lev 8192 chars, windows 4096, growth max(4×, +16 MiB)); exposes `match_count` and `indent_adjusted` into EditOutput (v0.1.30 residual; one-shot contract v0.1.33/0.1.34)

### Safety and Validation
- `src/path_safety.rs` — workspace jail: path traversal prevention, symlink validation, FIFO/device detection, Windows reserved names (COM0–9/LPT0–9), NFC, long-path `\\?\` helper
- `src/storage.rs` — config/data/cache/state via `directories::ProjectDirs` or `ATOMWRITE_HOME` override
- `src/env_detect.rs` — WSL/container/K8s/CI/Termux/Flatpak/Snap/sudo autodetection for `doctor`
- `src/signal.rs` — SIGINT/SIGTERM handling via signal-hook with graceful shutdown coordination
- `src/error.rs` — domain error enum with exit codes, error classification, and retryable flag
- `src/lock.rs` — advisory file locking via flock(2) on `.<target>.atomwrite.lock` sidecar

### Crash Recovery (v0.1.12, G114 + v0.1.15-v0.1.18, G119)
- `src/wal.rs` — WAL sidecar writer: appends `Started` and `Committed` entries to `.atomwrite.journal.<target>.atomwrite.journal.json`. Provides `recover_orphan_journals(dir)` consultative recovery. 8 unit tests. Hosts the `WalPolicy` enum and `JournalGuard` RAII from G119 L1/L2
- `src/wal/heuristics.rs` — G119 L4 HeuristicsEngine: 5 composable functions (`h1_ttl`, `h2_lru_within_cap`, `h3_rate_limit`, `h4_sentinel`, `h5_archive`) aggregated via `heuristics_should_preserve`
- `src/commands/wal_stats.rs` — G119 L5 telemetry subcommand: total_journals, by_state, oldest_age, total_size, by_directory, auto_heal_recommended, estimated_reclaim_bytes

### Syntax Check (v0.1.12, G72)
- `src/syntax_check.rs` — REAL tree-sitter syntax check via `tree-sitter-language-pack`. Replaces the v0.1.11 bracket-balance heuristic. Supports 24 languages out-of-the-box. Falls back to legacy heuristic for unknown extensions. 16 unit tests.

### Output
- `src/output.rs` — NDJSON writer with broken-pipe handling (SIGPIPE-safe)
- `src/ndjson_types.rs` — output type definitions with schemars JSON Schema derivation
- `src/constants.rs` — named constants for buffer sizes, thresholds, and exit codes

### Utilities
- `src/binary_detect.rs` — null-byte heuristic for binary content detection
- `src/line_endings.rs` — LF/CRLF/CR detection and normalization
- `src/lang_utils.rs` — programming-language extension maps for AST commands
- `src/locale.rs` — UI locale: `Idioma`, sys-locale detect, unic-langid parse, fluent-langneg negotiate, OnceLock, XDG preference
- `src/xattr_restore.rs` — save and restore xattrs (macOS quarantine, Linux selinux/capabilities)
- `src/reflink.rs` — reflink (copy-on-write) helper via `reflink-copy`

### Subcommand Handlers
- `src/commands/` — 41 subcommand implementations (handlers + aliases), each in its own module where applicable
- Each handler receives parsed args, global config, an NDJSON writer, and shutdown signal
- All handlers follow the same signature: `fn cmd_*(args, global, writer, shutdown) -> Result<()>`
- **v0.1.11 baseline (22)**: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, batch, backup, rollback, apply, completions
- **v0.1.12 added (6)**: set, get, del, case, query, outline
- **v0.1.15 added (2)**: wal-heal (G119 L3), wal-stats (G119 L5)
- **v0.1.22 added (2)**: edit-loop, prune-backups
- **v0.1.25 added (1)**: verify
- **v0.1.29 added (8)**: recipe, sparse, semantic-merge, agent-surface, watch, codemod, semantic-search, stat (alias of `read --stat`)
- **v0.1.29 command modules**: `recipe.rs`, `sparse.rs`, `semantic_merge.rs`, `agent_surface.rs`, `watch.rs`, `codemod.rs`, `semantic_search.rs`

### Cargo Features (v0.1.29+)
- `core` — slim binary without AST/watch/semantic (PRD 5–8 MiB target; measured ~7.7 MiB release)
- `ast` — ast-grep + tree-sitter + language-pack + serde_yaml (default with lang-rust/ts/py)
- `lang-rust` / `lang-ts` / `lang-py` / `lang-go` / `lang-full` — language surface for scope/transform
- `watch` — filesystem watch via notify (`watch` subcommand)
- `semantic` — offline token/index ranking (`semantic-search`)
- `full` — default + lang-full + watch + semantic
- Default features: `core`, `ast`, `lang-rust`, `lang-ts`, `lang-py`
- Slim build: `cargo build --release --no-default-features --features core`


## Data Flow

```
stdin ──> content bytes
             │
             ├── [write/edit/apply] ──> atomic_write() ──> tempfile
             │                              │                 │
             │                              │              fsync(fd)
             │                              │                 │
             │                              │           rename(temp, target)
             │                              │                 │
             │                              │           fsync(dir)
             │                              │                 │
             │                              └──> BLAKE3 checksum
             │
             ├── [search/replace] ──> WalkParallel ──> ripgrep engine
             │                              │
             │                       crossbeam channel
             │                              │
             │                         NDJSON events
             │
             └── [read/hash/list] ──> direct fs ops ──> NDJSON events
                                                            │
                                                       stdout (NDJSON)

v0.1.12 additions:
  write/edit ──> [if --syntax-check] ──> syntax_check.rs (tree-sitter)
                          │
                          └──> SyntaxError (exit 88) if errors found
  write/edit ──> [if WAL enabled] ──> wal.rs (Started entry)
                          │
                          └──> [after rename] ──> wal.rs (Committed entry)
  query/outline ──> tree-sitter parse ──> iterative DFS ──> NDJSON events
  set/get/del/case ──> toml_edit / heck ──> NDJSON events

v0.1.15 additions (G119):
  wal-heal ──> scan workspace for stale .atomwrite.journal.*.json
                  │
                  └──> heuristics_should_preserve ──> reap or skip
  wal-stats ──> aggregate by_state, by_directory, age, size ──> NDJSON

v0.1.18 additions (G118 universal resolve-first):
  write/edit/copy/apply/move/rollback/set/del/case/replace ──> validate_path (jail)
                          │
                          └──> WORKSPACE_JAIL (exit 126) on first violation
```


## Key Decisions

### BLAKE3 over SHA-256
- BLAKE3 is 5-14x faster than SHA-256 for file checksums
- Single-threaded performance matters for CLI latency
- Not used for cryptographic security, only integrity detection

### NDJSON over JSON Array
- Streaming: each result is emitted as soon as computed
- No need to buffer entire output before writing
- Pipe-friendly: downstream tools process line by line
- Errors emit in the same format with `error: true` discriminator

### tempfile + rename over In-Place Write
- Atomic: target file is never left half-written
- Survives power loss, OOM kill, SIGKILL
- Backup of original is a copy (not hardlink) to avoid shared-inode corruption
- **In-place fallback (v0.1.12, G55)**: when `nlink > 1` (Unix) or target is a symlink, atomwrite switches to `ftruncate(0) + write_all + fsync_data` to preserve the inode. NDJSON gains `write_strategy: "rename" | "inplace" | "copyback"`.

### Workspace Jail
- All paths validated against --workspace root
- Prevents path traversal via `..` components
- Blocks symlinks pointing outside the workspace
- Rejects FIFO and device files (non-atomic by nature)

### Signal Handling via signal-hook
- SIGINT and SIGTERM set atomic flag for cooperative shutdown
- Second signal forces immediate exit via `_exit` (Unix) / `process::exit` (Windows)
- SIGPIPE reset to default disposition for standard Unix pipe behavior
- Shared singleton `ShutdownSignal` (`OnceLock`) so polling and main-thread `is_shutdown()` see the same flag
- Cooperative cancel also covers global `--timeout-secs` (default **120**; pass `0` to disable; exit **124** on deadline, GNU `timeout` convention)
- Fuzzy cascade polls the cancel flag mid-file (strategies + sliding windows) so the timeout works during `replace --fuzzy` / edit / batch / edit-loop
- `signal::cancelled_error` stamps the live exit code (130 / 143 / 124) — never hard-code SIGTERM
- Long walks (`search`, `replace`, `count`, `list`, `hash`, `delete`, `batch`, …) poll the flag and stop accepting new work
- Atomic writes cancel mid-chunk without renaming the tempfile (target stays intact)
- One-shot CLI: no Tokio `CancellationToken` / `TaskTracker` / systemd `sd_notify` (N/A)

### G72 REAL tree-sitter syntax check (v0.1.12)
- Replaces v0.1.11 bracket-balance heuristic that had false positives (Python indentation, JS template literals) and false negatives (Python `import` of missing module)
- Uses `tree-sitter-language-pack` with `download` + `dynamic-loading` features
- 24 languages covered out-of-the-box; unknown extensions fall back to legacy heuristic
- Exit 88 with first error line/column/kind/message

### G114 WAL sidecar for crash recovery (v0.1.12)
- Sidecar path: `.atomwrite.journal.<target>.atomwrite.journal.json`
- Appends `Started` (op_id, expected_new_checksum, pid, started_at_unix) and `Committed` (op_id, committed_at_unix) entries
- `recover_orphan_journals(dir)` is **consultative** — reads sidecars and reports orphans without touching the FS
- Caller decides whether to replay, abort, or ignore

### tree-sitter-language-pack with dynamic-loading (v0.1.12, ADR-0019)
- Parsers are NOT bundled (would be 1+ GB)
- Downloaded on first use, cached locally in `~/.cache/tree-sitter-language-pack/parsers/`
- Install footprint stays around 5-10 MB
- Alternative: `tree-sitter` as direct dep, but adds 305 parser crates to compile time

### v14 Tier 3 architecture (v0.1.12)
- `set/get/del` use `toml_edit` (preserves comments and key order) for TOML and `serde_json` (canonical) for JSON
- `get/del` use manual `Table` descent via `get_toml_path` and `remove_toml_path` helpers (ADR-0024) instead of `toml_edit::Document::get` which treats dotted keys as literal
- `case` uses `heck` crate for 5 identifier-case styles
- `query/outline` use iterative DFS via `Vec<Node>` stack to avoid stack overflow on deep files (compared to recursive `TreeCursor` traversal)

### L1 WalPolicy + L4 HeuristicsEngine (v0.1.16, G119, ADR-0028)
- `WalPolicy { Auto, Always, Never }` lets callers tune when the WAL sidecar is written; `Auto` skips it for trivial writes (size under 1 MiB, not Edit/Replace, dir under Git, write under 4 KiB)
- `crate::wal::heuristics` aggregates 5 composable functions via `heuristics_should_preserve(target, committed_at_unix, count, rank)`; env vars `ATOMWRITE_WAL_KEEP_SECS`, `ATOMWRITE_WAL_MAX_COUNT`, `ATOMWRITE_WAL_RATE_LIMIT`, `ATOMWRITE_WAL_ARCHIVE_DAYS` tune each lever
- `wal_policy` field on `WriteOutput` NDJSON exposes the decision per call

### L3 startup auto-heal (v0.1.17, G119, ADR-0028)
- `atomwrite` runs an autonomous `wal-heal` pass on startup via `lib.rs::auto_heal_on_startup`, with 3600s threshold and 100ms budget
- Opt-out via `--skip-startup-wal-heal` (see `src/cli.rs`); logs structured info when reaping, debug when nothing to reap, warn on failure

### 4-layer empty-stdin guard (v0.1.16, G120, ADR-0029)
- L1 rejects 0 bytes from stdin by default in `read_stdin_content` with opt-out `--allow-empty-stdin`
- L2 rejects empty stdin in `handle_append_prepend`
- L3 emits `tracing::info!` warning when `--append` + `--expect-checksum` + empty stdin combine ambiguously; opt-out via `--no-checksum-when-empty`
- L4 always emits `stdin_bytes_read: u64` on `WriteOutput` NDJSON for late agent gating

### G118 universal resolve-first + G117 edge cases (v0.1.18, ADR-0030)
- 10 mutating commands now pre-validate root paths via `validate_path` before constructing any walker or worker: `write`, `edit`, `copy`, `apply`, `move`, `rollback`, `set`, `del`, `case`, `replace`
- `replace /etc/passwd` aborts in microseconds with a single `WORKSPACE_JAIL` envelope instead of walking the entire filesystem
- 3 new G117 edge-case regression tests: Unicode exact-match (UTF-8 diacritics), CRLF line-ending preservation after replace, multi-pair where the same `--old` appears twice
- 1 new G120 L3 integration test: `--append + --expect-checksum + --allow-empty-stdin` cross-flag combination is now covered end-to-end

### Internationalization
- Translations embedded at compile time via rust-i18n (`locales/en.toml`, `locales/pt-BR.toml`)
- Detection: `sys-locale` once at startup (never raw portable `LANG` reads)
- Normalize raw tags with `unic-langid` (underscore → hyphen, strip `.UTF-8`; reject `C`/`POSIX` as English)
- Negotiate against available MVP locales with `fluent-langneg` (`NegotiationStrategy::Lookup`)
- Typed `Idioma` enum (`En` | `PtBr`, `#[non_exhaustive]`) + immutable `OnceLock<LocaleState>`
- Precedence: `--locale` / `ATOMWRITE_LANG` → preference under `storage::config_dir()` (`ATOMWRITE_HOME` or XDG → `…/locale`) → OS → `en`
- Override: global `--locale` (clap `value_parser`) or env `ATOMWRITE_LANG` (field name `lang` kept for API stability; flag renamed from `--lang` in ADR-0037)
- Diagnostics: `atomwrite locale` NDJSON report; `locale --set` / `locale --clear` for XDG preference
- NDJSON **codes** and error **Display** messages stay English (agent machine contract)
- Error **suggestions** and selected human stderr warnings follow the resolved locale via `t!`
- Optional Cargo features `i18n-cjk` / `i18n-rtl` / `i18n-europe` / `i18n-full` are placeholders — default binary ships only complete `en` + `pt-BR`
- `build.rs` declares `cargo:rerun-if-changed` for `locales/`, `locales/en.toml`, and `locales/pt-BR.toml`
- Key parity tests: `tests/locale_parity.rs`

### Macros policy (no project-local `macro_rules!` / proc-macro crate)
- **Decision:** atomwrite has **zero** custom declarative or procedural macros in-tree
- Exhaust generics, traits, functions, and `const` asserts before introducing a macro (rules_rust_macros)
- Ecosystem macros only where syntax cannot be a function:
  - **Derive:** `clap`, `serde`, `thiserror`, `schemars` for CLI / NDJSON / errors
  - **Attribute:** `#[tracing::instrument]` on command entry points
  - **Function-like (external):** `rust_i18n::i18n!`, `schemars::schema_for!`, `tracing::{info,debug,…}!`
  - **std built-ins:** `env!` / `option_env!` (version), `matches!`, `vec!`, `format!`, `write!`/`writeln!`, `const _: () = assert!(…)`
- Prefer **typed** `Serialize` + `JsonSchema` structs over ad-hoc `serde_json::json!` for agent NDJSON contracts
- **Zero** `serde_json::json!` in production code paths: all stdout/index events use typed structs; child re-tagging uses `output::ndjson_insert_field` (no free-form macro)
- No `todo!` / `unimplemented!` / `dbg!` in production paths; `panic!` only in tests
- No local `proc-macro = true` crate (would force `syn`/`quote` compile cost for a CLI that does not need a DSL)


## Error Strategy
- `AtomwriteError` enum with thiserror derives Display
- Each variant maps to a sysexits-compatible exit code
- Error classification: permanent, transient, conflict, precondition_failed
- Transient and conflict errors are marked retryable for agent retry loops
- All errors serialize to NDJSON on stdout with machine-readable fields
- `suggestion` field in `ErrorJson` provides actionable recovery guidance for each error variant
- `ErrorContext` struct (added in v0.1.4) carries `workspace_provided: bool` and `workspace: Option<PathBuf>` from the CLI parser to the error output
- `ErrorJson::from_error_with_context(err, &ErrorContext)` produces context-aware suggestions
- `WorkspaceJail` suggestion adapts based on whether the user supplied `--workspace` or `ATOMWRITE_WORKSPACE`
- Legacy `ErrorJson::from_error(err)` delegates to `from_error_with_context` with `ErrorContext::default()` (backward compatible)
- 25 error variants total (20 baseline from v0.1.4 + 5 added in v0.1.12: `LockTimeout` 83, `SyntaxError` 88, `ExdevFallbackDisabled` 91, `CopyBackBlake3Failed` 92, `OrphanJournal` 93)
- v0.1.24 typed error audit: ALL user-facing `anyhow::bail!()` converted to `AtomwriteError` variants; no error path returns generic exit 1 without JSON envelope


## Architectural Decision Records (ADRs)
- See `docs/decisions/README.md` for the full ADR index
- 32 ADRs have been added since v0.1.12 (0019-0050), all following the Michael Nygard format (Status, Context, Decision, Consequences, Alternatives, Trigger to revisit)
- 0019 — tree-sitter-language-pack choice
- 0020 — WAL sidecar path and JSONL shape
- 0021 — v14 query/outline accepts only kind names, not S-expressions
- 0022 — G72 tree-sitter replaces heuristic
- 0023 — G114 WAL is consultive, not auto-replay
- 0024 — get/del TOML path uses manual Table descent
- 0025 — positions is opt-in in query/tree only
- 0026 — G117 multi-pair edit: fuzzy parity, per-pair reporting, opt-in --partial (v0.1.15)
- 0027 — G118 write resolves the target before pre-steps (v0.1.15)
- 0028 — G119 5-layer WAL cleanup: L1 WalPolicy, L2 JournalGuard, L3 startup auto-heal, L4 HeuristicsEngine, L5 wal-stats telemetry (v0.1.15-v0.1.17)
- 0029 — G120 4-layer empty-stdin guard: L1 read_stdin_content, L2 handle_append_prepend, L3 cross-validation warning, L4 stdin_bytes_read telemetry (v0.1.16)
- 0030 — v0.1.18 trio: replace pre-validates root paths, G120 L3 cross-flag test, G117 Unicode/CRLF/multi-pair edge cases


- 0031 — Exit code canonicalization: 7 documentation drifts consolidated to match the canonical list (v0.1.19)
- 0032 — Intention guards: a new safety layer of 5 OPT-IN flags (--require-backup, --confirm, --auto-rotate, --risk-threshold, --locale) intercepting destructive mutations before they touch disk (v0.1.20)
- 0033 — scope --lang alias accepted for ergonomic symmetry with transform --lang (v0.1.20)
- 0034 — --locale rename from --lang to disambiguate from the tree-sitter language selector (v0.1.20)
- 0035 — count --by-size: list the largest files in the tree with sizes and line counts (v0.1.20)
- 0036 — read --mode raw|envelope: select between byte-stream output and structured NDJSON envelope (v0.1.20)
- 0037 — search --no-begin-end: disable the implicit ^ and $ anchor decoration in regex output (v0.1.20)
- 0038 — backup cumprido deleta: `keep_backup` default false + `delete_backup_quietly` helper (v0.1.21)
- 0039 — edit-loop: sub-command for N pairs in 1 invocation via NDJSON (v0.1.22)
- 0040 — prune-backups: manual cleanup of legacy `.bak` files (v0.1.22)
- 0041 — allow_hyphen_values for 15 CLI text fields (v0.1.23)
- 0042 — backup-by-default in 9 content-mutating structs (v0.1.23)
- 0043 — shrink guard with --expect-checksum (v0.1.23)
- 0044 — --old-file/--new-file to bypass ARG_MAX (v0.1.23)
- 0045 — actionable suggestion for clap parse errors (v0.1.24)
- 0046 — diff resolve-first retrofit (v0.1.24)
- 0047 — scope read-only mode fix (v0.1.24)
- 0048 — unified BackupOpts: single struct flattened via `#[command(flatten)]` into 15 mutating subcommands, resolved by a single `resolve_backup()` with precedence `ATOMWRITE_BACKUP` env > CLI flags > `.atomwrite.toml` `[defaults]` > built-in default (v0.1.28)
- 0049 — live config plumbing: `load_config` called once in `lib.rs::run()`, `DefaultsSection` propagated to every mutating handler (v0.1.28)
- 0050 — stdin-tty guard: `main.rs` computes `stdin.is_terminal()` (std `IsTerminal`, Rust >= 1.70) and propagates `stdin_is_tty` down to `cmd_edit`; stdin-consuming modes fail fast with exit 65 instead of blocking indefinitely (v0.1.28)
- 0054 — one-shot fuzzy contract: one-pass multi-apply, embeds force single apply, default max applies 1, `--timeout-secs` default 120 / exit 124, cancel polled mid-fuzzy, resource caps (v0.1.33 runtime; v0.1.34 docs-complete)


## Test Architecture
- 621 tests across 51 test suites (152 unit tests inside `src/` + integration suites + doctests)
- Unit tests are colocated with the code under `#[cfg(test)]` modules
- Integration tests live in `tests/` and use `assert_cmd` + `predicates` for shell-out tests
- Property-based tests via `proptest` for checksum and backup
- Cross-compile gate via `tests/cross_compile_check.rs`
- Snapshot tests via `insta` for stable NDJSON output
