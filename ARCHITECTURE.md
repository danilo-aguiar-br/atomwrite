[Leia em Português](ARCHITECTURE.pt-BR.md)


# Architecture


## Overview
- atomwrite is a single Rust binary CLI for atomic file operations
- Designed for LLM agents that need safe, structured file manipulation
- Every write is atomic: tempfile, fsync, rename, fsync directory
- Every response is NDJSON on stdout with BLAKE3 checksums
- All diagnostic logs go to stderr via `tracing` (`init_tracing` in `src/runtime.rs`, no product telemetry): non-blocking writer, filter from CLI `-v`/`-q`, optional XDG state log file (key `log.file`), no product `ATOMWRITE_*` / `RUST_LOG` knobs in v0.1.35+
- The agent data plane remains NDJSON on stdout


## Module Map

### Entry Point
- `src/main.rs` — binary entry: signal setup, tracing init, dispatch
- `src/lib.rs` — library root: module declarations and the `run()` dispatcher
- `src/lib_dispatch.inc.rs` — command dispatch and `startup_wal_heal`, the startup `wal-heal` pass that runs before dispatch, only for verbs that mutate and only over the directories named in argv
- `src/runtime.rs` (+ `runtime_cli.inc.rs`) — binary startup helpers: tracing, locale, clap error enrichment, schema prescan
- `src/cli.rs` (+ `cli_global.inc.rs`) — clap `#[derive(Parser)]` with the global flags
- `src/cli_args/` — per-subcommand argument structs and value enums, split in `mod.rs` and `part*.inc.rs`

### Core Pipeline
- `src/atomic/` — atomic write pipeline: `write.rs` orchestrates tempfile + fsync + rename + fsync dir, `rename_path.rs` is the tempfile path, `inplace.rs` holds the in-place write, the EXDEV fallback and timestamped backups, `persist_retry.rs` retries the Windows tempfile persist, `types.rs` and `time_fmt.rs` are helpers
- `src/checksum.rs` — BLAKE3 hash computation for files and byte slices (uses memmap2 for large files)
- `src/file_io.rs` — smart file reading: one open, one `fstat` on the handle, one read into an exact-size buffer (`try_reserve_exact`), hashing uses `checksum::hash_file`
- `src/platform.rs` — platform durability and rename: `Durability` full|fast|auto, Linux `renameat2` with rename fallback, macOS `F_FULLFSYNC` on full, and the write NDJSON reports `platform.durability` and `platform.rename_method`
- `src/fuzzy/` — shared match cascade used by edit, replace, batch and edit-loop: exact, `line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `trimmed_boundary`, `escape_normalized`, `block_anchor`, `unicode_normalized` and `context_aware` (with a Jaro-Winkler pass)
- `src/fuzzy/one_pass.inc.rs` — one-pass multi-apply (`apply_fuzzy_one_pass`) on the original content, never re-scanning inserted text, a pattern embedded in its replacement forces a single apply, default max applies = 1, hard ceiling 10_000, cooperative cancel polled mid-cascade, caps (pattern 64 KiB, Levenshtein 8192 chars, windows 4096, growth max(4x, +16 MiB))
- The fuzzy output carries `match_count` and `indent_adjusted` in `EditOutput`, normalized strategies emit `similarity: null` (not a fake 1.0), `block_anchor` dual-gates the middle ratio, and `context_aware` applies a soft floor via XDG `[fuzzy].soft_floor_delta` (default 0.05)
- Reflink (copy-on-write) lives in `src/atomic/write.rs` and `src/atomic/inplace.rs`, through the `reflink-copy` crate

### Safety and Validation
- `src/path_safety.rs` — workspace jail: path traversal prevention, symlink validation, FIFO/device detection, Windows reserved names (COM0–9/LPT0–9), NFC, long-path `\\?\` helper
- `src/storage.rs` — config/data/cache/state via `directories::ProjectDirs` (XDG), no product `ATOMWRITE_HOME` runtime knob in v0.1.35+
- `src/env_detect.rs` — WSL/container/K8s/Termux/Flatpak/Snap autodetection for `doctor`, from file probes only (no environment variable is read)
- `src/signal.rs` (+ `signal_handlers.inc.rs`, `signal_cancel.inc.rs`) — SIGINT/SIGTERM/SIGHUP and the SIGPIPE handler via signal-hook with graceful shutdown coordination, and `CancelCheck` for the cooperative cancel
- `src/stdin_cancel.rs` — cancellable stdin: a reader thread that gives up when the run is cancelled or the deadline passes
- `src/error/` — domain error enum with exit codes, error classification and the retryable flag, split in `variants.inc.rs`, `class.inc.rs`, `impl_methods.inc.rs`, `error_json.inc.rs` and `suggest.inc.rs`
- `src/lock.rs` — advisory file locking via `std::fs::File::try_lock`, which is `flock` on Unix and `LockFileEx` on Windows, on the target's own inode; the `.<target>.atomwrite.lock` sidecar is taken after it, best effort, so older releases stay mutually excluded; `LockGuard::target_unchanged` revalidates the inode with `same_file` immediately before the swap, `delete` takes the same lock before it hashes and unlinks, and the sidecar name becomes a short BLAKE3 digest when the literal name would pass 255 bytes (ADR-0079)
- `src/walk_globs.rs` — the one place that compiles the `--include` / `--exclude` globs of every verb, a glob that does not compile is `INVALID_INPUT` (exit 65)

### Crash Recovery (v0.1.12, G114 + v0.1.15-v0.1.18, G119)
- `src/wal/` — the WAL sidecar, one module per concern
- `src/wal/journal.rs` — sidecar writer: appends `Started` and `Committed` entries to `.atomwrite.journal.<target>.atomwrite.journal.json`, and the `JournalGuard` RAII from G119 L2
- `src/wal/policy.rs` — `WalPolicy` (`Auto`, `Always`, `Never`) and the creation heuristics, including `creation_throttled`, the H3 rate limit applied only when a journal is about to be created
- `src/wal/heuristics.rs` — G119 L4 HeuristicsEngine: `h1_ttl`, `h2_lru_within_cap`, `h3_rate_limit`, `h4_sentinel`, `h5_archive`, where `heuristics_should_preserve` aggregates the retention votes for a committed journal and does not consult `h3_rate_limit`
- `src/wal/heal.rs` — `recover_orphan_journals(dir)`, `auto_heal_scoped` and the `wal-heal` pass, which removes `Committed` and `Aborted` sidecars, and removes a `Started` sidecar only when the owning PID no longer exists, reported as `dead_owner_removed`
- `src/wal/stats.rs` and `src/commands/wal_stats.rs` — G119 L5 `wal-stats` subcommand: total_journals, by_state, oldest_age, total_size, by_directory, auto_heal_recommended, estimated_reclaim_bytes

### Syntax Check (v0.1.12, G72)
- `src/syntax_check.rs` — REAL tree-sitter syntax check via `tree-sitter-language-pack`, replacing the v0.1.11 bracket-balance heuristic, with 23 languages mapped by file extension
- Unknown extensions fall back to the legacy heuristic in `src/atomic/syntax_heuristic.rs`
- `src/syntax_check_stub.rs` — the stub used when the `ast` feature is off

### Output
- `src/output.rs` (+ `output_lines.inc.rs`, `output_verdict.inc.rs`) — NDJSON writer with broken-pipe handling, and the success/failure stamp of the terminal event
- `src/output_shape.inc.rs` and `src/output_shape/` — the global runtime shape applied at emission (`--select`/`--fields`, `--filter`, `--limit`, `--sort-by`, `--dedupe`, `--aggregate`, `--join`, `--diff-against`, `--redact`), these are not schema fields
- `src/ndjson/` — output type definitions with schemars JSON Schema derivation
- `src/schema_dispatch.inc.rs` and `src/schema_terminal.inc.rs` — the `--print-schema` listing and the closed terminal schemas
- `src/error/` — the error envelope always emits `ok: false` (required) plus `match_count` on match failures (0 on most failures), with an optional `best_candidate` on miss/ambiguity
- `src/constants.rs` — named constants for buffer sizes, thresholds and exit codes

### Utilities
- `src/binary_detect.rs` — null-byte heuristic for binary content detection
- `src/line_endings.rs` — LF/CRLF/CR detection and normalization
- `src/lang_utils.rs` — programming-language extension maps for AST commands
- `src/locale.rs` (+ `locale_persist.inc.rs`) — UI locale: `Language`, sys-locale detect, unic-langid parse, fluent-langneg negotiate, OnceLock, XDG preference file
- `src/xattr_restore.rs` — save and restore xattrs (macOS quarantine, Linux selinux/capabilities)
- `src/concurrency.rs` (+ `concurrency_walk.inc.rs`) — bounded concurrency and multi-core fan-out, `WalkParallel` collection
- `src/config.rs` — loading of `.atomwrite.toml` and the XDG `config.toml` (flags > workspace > global > defaults)

### Reliability, Tuning and Failure Reporting (v0.1.37)
- `src/retry.rs` — retry with exponential backoff, full jitter and a hard deadline (`RetryPolicy`, `Backoff`, `retry()`), used by the advisory lock and the Windows tempfile persist, sleeping in short slices and obeying the global `--timeout-secs` deadline and the shutdown flag
- `src/best_effort.rs` — the sanctioned way to ignore a failure on purpose: `best_effort` logs at `debug` and continues, while a failure that changes the result is propagated with `?`
- `src/config_tuning.inc.rs` — tuning sections (`concurrency`, `search`, `lock`, `retry`, `extract`), their validation and the process-wide `tuning()` accessor
- `src/commands/config_cmd.rs` (+ `config_cmd_keys.inc.rs`, `config_cmd_write.inc.rs`) — the `config` verb: one typed table of 43 keys, layered read (flag, workspace, global, built-in), validated atomic write that keeps comments
- `src/constants.rs` — includes `constants_concurrency.inc.rs` (retry, thread ceiling, search budget), `constants_config.inc.rs` (config file names, key minimums) and `constants_literals.inc.rs` (named literals: sidecar names, time units, sizes)
- `src/commands/search_budget.inc.rs` — one global byte budget for the search events in flight (default 8 MiB, key `search.max_inflight_bytes`), and the stream being drained never waits, so it cannot deadlock
- `src/ndjson/part3.inc.rs` — `FileFailure`, `FailureSummary`, `FailureTally` and `partial_of`: `files_failed`, `partial` and `errors[]` (at most 50) on the terminal summary of every verb that walks a tree, with `partial` derived from the same count
- `src/output_verdict.inc.rs` — success/failure stamping of the terminal event and the `VerdictPublished` marker, so a failing summary is the only error line of the run
- `workspace_source` — every terminal envelope publishes `argv` or `cwd`, so a run that took the root from the process working directory says so instead of hiding the ambient authority
- `replace --expect-checksum` — the comparison runs under the lock and a drift exits 82 `STATE_DRIFT`, closing the window between the check and the write
- `batch --transaction` — every target lock is taken in path order before the snapshot and held until commit or rollback, so two batches wait instead of interleaving
- `edit-loop` — the checksum read under the lock is the write base, so a concurrent edit exits 82 instead of a lost update
- Typed errors — the edge paths of `cli.rs`, `rename_path.rs` and the fsync helpers raise `AtomwriteError`, so a denied directory exits 13 `PERMISSION_DENIED` and never 255 `INTERNAL_ERROR`
- `src/commands/replace_file.inc.rs` and `src/commands/replace_finish.inc.rs` — the per-file step and the closing step of `replace`, split out of `replace_run.inc.rs`
- `src/walk_globs.rs`, `src/stdin_cancel.rs` and `src/signal_cancel.inc.rs` — the shared glob compiler, the cancellable stdin and the cancel helpers listed above

### Subcommand Handlers
- `src/commands/` — 47 subcommand implementations (handlers + aliases, SSOT `CANONICAL_SUBCOMMAND_COUNT` / `agent-surface`), each in its own module where applicable, and `batch/`, `edit/`, `extract_artifact/` and `scope/` are folders
- Each handler receives parsed args, global config, an NDJSON writer, and the shutdown signal
- All handlers follow the same signature: `fn cmd_*(args, global, writer, shutdown) -> Result<()>`
- v0.1.11 baseline (22): read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, batch, backup, rollback, apply, completions
- v0.1.12 added (6): set, get, del, case, query, outline
- v0.1.15 added (2): wal-heal (G119 L3), wal-stats (G119 L5)
- v0.1.22 added (2): edit-loop, prune-backups
- v0.1.25 added (1): verify
- v0.1.29 added (8): recipe, sparse, semantic-merge, agent-surface, watch, codemod, semantic-search, stat (alias of `read --stat`)
- v0.1.29 command modules: `recipe.rs`, `sparse.rs`, `semantic_merge.rs`, `agent_surface.rs`, `watch.rs`, `codemod.rs`, `semantic_search.rs`
- Host / discovery (complete 41 → 46): `doctor` (host env diagnostics), `locale` (resolved locale + XDG preference), `commands` (full command-tree manifest), `scratch` (`scratch init` staging under `.atomwrite/scratch/`), and `extract-artifact`
- v0.1.37 added (1): `config` (persistent tuning keys), the 47th subcommand
- Historical note: 41 was the documented surface through v0.1.35, and the current SSOT is 47

### Cargo Features (v0.1.29+)
- `core` — slim binary without AST/watch (PRD 5–8 MiB target, 26,560,352 bytes measured on 2026-11-04, DERIVADO 26560352 / 1048576 = 25.33 MiB, and ~7.7 MiB at v0.1.29)
- `ast` — ast-grep + tree-sitter + language-pack + serde-saphyr (default with lang-rust/ts/py)
- `lang-rust` / `lang-ts` / `lang-py` / `lang-go` / `lang-full` — language surface for scope/transform
- `watch` — filesystem watch via notify (`watch` subcommand), opt-in, and the default build answers `watch` with exit 78 and a rebuild hint
- `full` — default + lang-full + watch
- `semantic-search` has no feature gate: it is always compiled (offline Jaccard)
- Default features: `core`, `ast`, `lang-rust`, `lang-ts`, `lang-py`
- Slim build: `cargo build --release --no-default-features --features core`, and in it `outline`, `query`, `scope`, `transform` and `sparse outline` exit 78 with a rebuild hint (`CONFIG_INVALID`), read from `src/commands/*_stub.rs` and `src/commands/sparse.rs`, the slim build was not executed for this document
- `cargo xtask check-features` runs `cargo check --locked --no-default-features` for the `core` and `core,watch` sets and stops at the first failure (`xtask/src/main.rs`)


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
  write/edit ──> [if WAL enabled] ──> wal/journal.rs (Started entry)
                          │
                          └──> [after rename] ──> wal/journal.rs (Committed entry)
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
- BLAKE3 is designed to be faster than SHA-256 (upstream benchmarks, not measured here)
- Single-threaded performance matters for CLI latency
- Not used for cryptographic security, only integrity detection

### NDJSON over JSON Array
- Streaming: each result is emitted as soon as computed
- No need to buffer entire output before writing
- Pipe-friendly: downstream tools process line by line
- Errors emit in the same format with `ok: false` plus `code` and `exit`, and the `error` discriminator of 0.1.37 and earlier is gone

### tempfile + rename over In-Place Write
- Atomic: the target file is never left half-written by a rename-based write
- Behavior after SIGKILL or power loss depends on the filesystem and on `--durability`
- Backup of original is a copy (not hardlink) to avoid shared-inode corruption
- In-place fallback (v0.1.12, G55): when `nlink > 1` (Unix) or target is a symlink, atomwrite switches to `ftruncate(0) + write_all + fsync_data` to preserve the inode, and NDJSON gains `write_strategy: "rename" | "inplace" | "copyback"`

### Workspace Jail
- All paths validated against --workspace root
- Prevents path traversal via `..` components
- Blocks symlinks pointing outside the workspace
- Rejects FIFO and device files (non-atomic by nature)

### Signal Handling via signal-hook
- SIGINT and SIGTERM set an atomic flag for cooperative shutdown
- A second signal forces immediate exit via `_exit` (Unix) / `process::exit` (Windows)
- SIGPIPE is neither ignored nor left at `SIG_DFL`: a cooperative handler (`register_broken_pipe` in `src/signal_handlers.inc.rs`) marks the shutdown and the run exits 141 after it has released locks, journals and tempfiles
- This is a conscious deviation from the plain reset-to-default convention, recorded in ADR-0063, because a default-disposition kill left locks, journals and tempfiles behind
- Shared singleton `ShutdownSignal` (`OnceLock`) so polling and main-thread `is_shutdown()` see the same flag
- Cooperative cancel also covers global `--timeout-secs` (default 120, pass `0` to disable, exit 124 on deadline, GNU `timeout` convention)
- Fuzzy cascade polls the cancel flag mid-file (strategies + sliding windows) so the timeout works during `replace --fuzzy` / edit / batch / edit-loop
- `signal::cancelled_error` stamps the live exit code (130 / 143 / 124), never hard-code SIGTERM
- Long walks (`search`, `replace`, `count`, `list`, `hash`, `delete`, `batch`, …) poll the flag and stop accepting new work
- Atomic writes cancel mid-chunk without renaming the tempfile (target stays intact)
- One-shot CLI: no Tokio `CancellationToken` / `TaskTracker` / systemd `sd_notify` (N/A)

### G72 REAL tree-sitter syntax check (v0.1.12)
- Replaces the v0.1.11 bracket-balance heuristic that had false positives (Python indentation, JS template literals) and false negatives (Python `import` of missing module)
- Uses `tree-sitter-language-pack` with `download` + `dynamic-loading` features
- 23 languages are mapped by extension, and unknown extensions fall back to the legacy heuristic
- Exit 88 with first error line/column/kind/message

### G114 WAL sidecar for crash recovery (v0.1.12)
- Sidecar path: `.atomwrite.journal.<target>.atomwrite.journal.json`
- Appends `Started` (op_id, expected_new_checksum, pid, started_at_unix) and `Committed` (op_id, committed_at_unix) entries
- `recover_orphan_journals(dir)` is consultative: it reads sidecars and reports orphans without touching the FS
- The caller decides whether to replay, abort, or ignore

### tree-sitter-language-pack with dynamic-loading (v0.1.12, ADR-0019, ADR-0057)
- Parsers are NOT bundled
- They are downloaded on first use of a language and cached under `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` (`bundles`, `libs`, `manifest.json`), from the release assets of `github.com/xberg-io/tree-sitter-language-pack`, whose 1.22.1 manifest lists 372 languages
- The manifest of 1.22.1 lists 23,031,807 bytes (about 22 MiB) for the `linux-x86_64` bundle, measured on 2026-10-31, and later runs reuse the cache
- The HTTP client is `ureq`, which reads the proxy variables of the environment, and a host with no network can pre-populate the cache directory
- Alternative: `tree-sitter` as a direct dependency, but it adds hundreds of parser crates to the compile time (ADR-0019)

### v14 Tier 3 architecture (v0.1.12)
- `set/get/del` use `toml_edit` (preserves comments and key order) for TOML and `serde_json` (canonical) for JSON
- `get/del` use manual `Table` descent via the `get_toml_path` and `remove_toml_path` helpers (ADR-0024) instead of `toml_edit::Document::get`, which treats dotted keys as literal
- `case` uses the `heck` crate for 5 identifier-case styles
- `query/outline` use iterative DFS via a `Vec<Node>` stack to avoid stack overflow on deep files (compared to a recursive `TreeCursor` traversal)

### Shared fuzzy cascade for multi-pair edit (v0.1.15, G117, ADR-0026)
- `match_pair(content, old, new, fuzzy_mode)` extracts the cascade, shared by the single-pair and multi-pair paths
- Multi-pair runs the cascade per pair, and the content evolves between pairs (the order is defined behavior)
- Success envelopes gain `pairs_total` + `pair_results`, and failures raise `EditPairFailed` with `failed_pair_index` (reuses `INVALID_INPUT`, exit 65)
- `--partial` (opt-in) applies the pairs that match, and zero applied pairs maps to `NO_MATCHES` (exit 1)

### Write target resolved first (v0.1.15, G118, ADR-0027)
- `cmd_write` resolves the target via `validate_path` once, before append/prepend, line-ending auto-detection and `--expect-checksum`
- It removes the double path identity (CWE-367): with a divergent CWD the append no longer truncates and a checksum drift exits 82
- Convention: every mutating command MUST resolve the target via `validate_path` before any `exists()`/read

### L1 WalPolicy + L4 HeuristicsEngine (v0.1.16, G119, ADR-0028, ADR-0062)
- `WalPolicy { Auto, Always, Never }` lets callers tune when the WAL sidecar is written, and `Auto` creates it for a target above 1 MiB, for `edit`/`replace`, or outside a git directory, and skips a trivial write (4 KiB or less) inside git
- `crate::wal::heuristics` aggregates the retention votes via `heuristics_should_preserve(target, committed_at_unix, count, rank)`, and the levers use constants + XDG/CLI configuration in v0.1.35+ (no product `ATOMWRITE_WAL_*` env knobs)
- The H3 rate limit is a creation-time vote only (`creation_throttled`): after 10 journals opened within 60 s `Auto` stops opening new ones while the writes stay atomic, and a journal that was written is always removed (ADR-0062)
- The `wal_policy` field on the `WriteOutput` NDJSON exposes the decision per call

### L3 startup auto-heal (v0.1.17, G119, ADR-0028, ADR-0062)
- `atomwrite` runs an autonomous `wal-heal` pass before dispatch (`startup_wal_heal` in `src/lib_dispatch.inc.rs`) with a 3600 s threshold and a 100 ms budget
- It runs only for a verb that mutates, and only over the directories named in argv, never the whole workspace
- It removes `Committed` and `Aborted` sidecars and never a `Started` one, which is the orphan that deserves attention after a crash
- Opt-out via the global flag `--no-auto-heal` (`src/cli_global.inc.rs`), and it logs structured info when reaping, debug when nothing to reap, warn on failure

### 4-layer empty-stdin guard (v0.1.16, G120, ADR-0029)
- L1 rejects 0 bytes from stdin by default in `read_stdin_content` with opt-out `--allow-empty-stdin`
- L2 rejects empty stdin in `handle_append_prepend`
- L3 emits a `tracing::info!` warning when `--append` + `--expect-checksum` + empty stdin combine ambiguously, opt-out via `--no-checksum-when-empty`
- L4 always emits `stdin_bytes_read: u64` on the `WriteOutput` NDJSON for late agent gating

### G118 universal resolve-first + G117 edge cases (v0.1.18, ADR-0030)
- 10 mutating commands pre-validate root paths via `validate_path` before constructing any walker or worker: `write`, `edit`, `copy`, `apply`, `move`, `rollback`, `set`, `del`, `case`, `replace`
- `replace /etc/passwd` aborts in microseconds with a single `WORKSPACE_JAIL` envelope instead of walking the entire filesystem
- 3 regression tests for G117 edge cases: Unicode exact-match (UTF-8 diacritics), CRLF line-ending preservation after replace, multi-pair where the same `--old` appears twice
- 1 integration test for G120 L3: the `--append + --expect-checksum + --allow-empty-stdin` cross-flag combination is covered end-to-end

### Internationalization
- Translations embedded at compile time via rust-i18n (`locales/en.toml`, `locales/pt-BR.toml`)
- Detection: `sys-locale` once at startup, and on Linux that crate reads `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` and `LANG`, while `directories` reads `XDG_*` and `HOME` for the config paths
- The product code calls no `env::var` (a test guards it), and the dependency reads are listed in ADR-0057
- Normalize raw tags with `unic-langid` (underscore → hyphen, strip `.UTF-8`), and `C`/`POSIX` yield no language preference
- Negotiate against the available locales with `fluent-langneg` (`NegotiationStrategy::Lookup`)
- Typed `Language` enum (`En` | `PtBr`, `#[non_exhaustive]`) + immutable `OnceLock<LocaleState>`
- Precedence: CLI `--locale` → preference under `storage::config_dir()` (XDG → locale) → OS → `en` (no product env knobs)
- Override: global `--locale` (clap value_parser, field name `lang` kept for API stability), and there is no product `ATOMWRITE_LANG` env knob in v0.1.35+
- Diagnostics: `atomwrite locale` NDJSON report, `locale --set` / `locale --clear` for the XDG preference
- NDJSON codes and error Display messages stay English (agent machine contract)
- Error suggestions and selected human stderr warnings follow the resolved locale via `t!`
- Optional Cargo features `i18n-cjk` / `i18n-rtl` / `i18n-europe` / `i18n-full` are placeholders: the default binary ships only complete `en` + `pt-BR`
- `build.rs` declares `cargo:rerun-if-changed` for `locales/`, `locales/en.toml`, and `locales/pt-BR.toml`
- Key parity tests: `tests/locale_parity.rs`

### Macros policy (one local declarative macro, no proc-macro crate)
- Decision: the production code defines a single project-local macro, `invalid_input!` (`src/error/mod.rs`, `macro_rules!`), which builds an `AtomwriteError::InvalidInput` error from a format string
- The test harness has one more, `impl_hermetic!` in `tests/common/mod.rs`, and there is no local `proc-macro = true` crate
- Exhaust generics, traits, functions, and `const` asserts before introducing a macro
- Ecosystem macros only where syntax cannot be a function:
  - Derive: `clap`, `serde`, `thiserror`, `schemars` for CLI / NDJSON / errors
  - Attribute: `#[tracing::instrument]` on command entry points
  - Function-like (external): `rust_i18n::i18n!`, `schemars::schema_for!`, `tracing::{info,debug,…}!`
  - std built-ins: `env!` / `option_env!` (version), `matches!`, `vec!`, `format!`, `write!`/`writeln!`, `const _: () = assert!(…)`
- Prefer typed `Serialize` + `JsonSchema` structs over ad-hoc `serde_json::json!` for agent NDJSON contracts
- `serde_json::json!` appears only in JSON Schema assembly (`src/schema_dispatch.inc.rs`, `src/schema_terminal.inc.rs`) and in the dynamic key values of the `config` verb (`src/commands/config_cmd*.rs`), and child re-tagging uses `output::ndjson_insert_field`
- No `todo!` / `unimplemented!` / `dbg!` in production paths, and the source guards forbid `panic!` and `.unwrap()` there (see Test Architecture)
- No local `proc-macro = true` crate (it would force the `syn`/`quote` compile cost on a CLI that does not need a DSL)


## Error Strategy
- `AtomwriteError` enum with thiserror derives Display
- Each variant maps to a sysexits-compatible exit code
- Error classification: permanent, transient, conflict, precondition_failed
- Transient and conflict errors are marked retryable for agent retry loops
- All errors serialize to NDJSON on stdout with machine-readable fields
- The `suggestion` field in `ErrorJson` provides actionable recovery guidance for each error variant
- The `ErrorContext` struct (added in v0.1.4) carries `workspace_provided: bool` and `workspace: Option<PathBuf>` from the CLI parser to the error output
- `ErrorJson::from_error_with_context(err, &ErrorContext)` produces context-aware suggestions
- The `WorkspaceJail` suggestion adapts based on whether the user supplied `--workspace` (CLI/XDG only in v0.1.35+)
- Legacy `ErrorJson::from_error(err)` delegates to `from_error_with_context` with `ErrorContext::default()` (backward compatible)
- 38 error variants in total (`rg -c '#\[error' src/error/variants.inc.rs`), and the agent manifest lists 42 exit-code entries with 34 distinct numbers (`atomwrite agent-surface --format json | jaq '.exit_codes'`)
- v0.1.24 typed error audit: ALL user-facing `anyhow::bail!()` converted to `AtomwriteError` variants, and no error path returns a generic exit 1 without a JSON envelope
- A failed run ends with exactly one terminal event with `ok: false` (ADR-0061)


## Architectural Decision Records (ADRs)
- See `docs/decisions/README.md` for the full ADR index, which is the authority for this list
- ADRs 0019 to 0095 live in `docs/decisions/`, each with a pt-BR translation, all following the Michael Nygard format (Status, Context, Decision, Consequences, Alternatives, Trigger to revisit)
- 0019 — tree-sitter-language-pack over direct tree-sitter dependency
- 0020 — WAL sidecar file path and shape
- 0021 — v14 query/outline accept only kind names, not S-expressions
- 0022 — G72 tree-sitter replaces the bracket heuristic
- 0023 — G114 WAL recovery is consultative, not automatic
- 0024 — get/del TOML dotted-path uses manual Table descent
- 0025 — `--positions` is opt-in and limited to the `--query`/`--tree` modes
- 0026 — G117 multi-pair edit: fuzzy parity, per-pair reporting, opt-in `--partial` (v0.1.15)
- 0027 — G118 write resolves the target against the workspace before all pre-steps (v0.1.15)
- 0028 — G119 five-layer autonomous WAL cleanup: L1 WalPolicy, L2 JournalGuard, L3 startup auto-heal, L4 HeuristicsEngine, L5 wal-stats (v0.1.15-v0.1.17)
- 0029 — G120 empty-stdin guard and cross-validation of `--append`, `--expect-checksum` and empty stdin (v0.1.16)
- 0030 — v0.1.18: replace pre-validates root paths, G120 L3 test, G117 edge cases
- 0031 — search and replace resolve root paths against the workspace via a shared helper (G121)
- 0032 — G122: real S-expression matching in `query`
- 0033 — v0.1.19 exit code documentation drift consolidation
- 0034 — help-driven testing: anti-pattern and the v0.1.20 corrective process
- 0035 — six-layer defense-in-depth for the `write` subcommand (v0.1.20)
- 0036 — `--partial` in `edit`: single-pair stays NO_MATCHES, multi-pair applies the matched ones
- 0037 — `GlobalArgs.lang` renamed to `--locale`, freeing the `--lang` namespace (v0.1.20)
- 0038 — a fulfilled backup is deleted: delete after success is the default (v0.1.21)
- 0039 — edit-loop helper: N edits in one invocation (v0.1.22)
- 0040 — prune-backups subcommand: manual cleanup of legacy backups (v0.1.22)
- 0041 — `allow_hyphen_values` for 12 CLI text fields across 8 structs since v0.1.37, because the three positionals require `--` (v0.1.23)
- 0042 — backup by default for content-mutating commands (v0.1.23)
- 0043 — shrink guard with `--expect-checksum` (v0.1.23)
- 0044 — `--old-file`/`--new-file` for the edit command (v0.1.23)
- 0045 — actionable suggestions for clap parse errors (v0.1.24)
- 0046 — diff resolve-first retrofit (v0.1.24)
- 0047 — scope read-only mode fix and `find_all` migration (v0.1.24)
- 0048 — unified `BackupOpts` and a single `resolve_backup()` resolver, with precedence CLI flags > `.atomwrite.toml` `[defaults]` / XDG > built-in default (env knobs removed by v0.1.35)
- 0049 — live config plumbing: `load_config` called once in `lib.rs::run()`, `DefaultsSection` propagated to every mutating handler (v0.1.28)
- 0050 — stdin-tty guard: `main.rs` computes `stdin.is_terminal()` and propagates `stdin_is_tty` down to `cmd_edit`, so stdin-consuming modes fail fast with exit 65 instead of blocking (v0.1.28)
- 0051 — v0.1.29 gap close, agent surface and binary honesty
- 0052 — v0.1.30 residual agent contract
- 0053 — clap rules audit exceptions (v0.1.32)
- 0054 — v0.1.34 one-shot fuzzy contract: one-pass multi-apply, `--timeout-secs` default 120 / exit 124, cancel polled mid-fuzzy, resource caps
- 0055 — v0.1.36 agent-first contract (GAP-2026-019…033): jail on content-source paths + scratch staging, `ok: false` error envelope, `similarity: null`, global `--select`/`--filter`/`--limit` (see `docs/decisions/0055-v0-1-36-agent-contract-gaps.md`)
- 0056 — remove the compile-time `release_max_level_info` cap from `tracing`
- 0057 — dependency environment reads, parser download, MSRV 1.90 and YAML stack
- 0058 — the `config` verb and typed tuning keys
- 0059 — one retry module with full jitter and a global deadline
- 0060 — a byte budget for `search` events in flight
- 0061 — one terminal event per failed run (`publish_failure` and `VerdictPublished`)
- 0062 — WAL rate limit at journal creation and a bounded `startup_wal_heal`
- 0063 — SIGPIPE: a cooperative handler with exit 141 (conscious deviation)
- 0064 — automatic thread ceiling of 16 and a rayon pool built on demand
- 0065 — fuzzy content ceiling and cooperative cancel polls
- 0066 — `DoctorChecksFailed`: `doctor --strict` exits 1 with one terminal line
- 0067 — closed terminal schemas and a faithful `--print-schema`
- 0068 — hermetic tests and source guards
- 0069 — split `constants.rs` and the large modules into `.inc.rs` parts
- 0070 — `watch` becomes an opt-in feature
- 0071 — `extract-artifact`: bounded signal, never raw bytes
- 0072 — agent-native surface: stream shaping, `--print-schema` before clap, side-effect guards
- 0073 — one terminal cancel point and `deadline_exceeded`
- 0074 — strict apply runs every read-modify-write under one lock
- 0075 — grammar pack 1.22, ast-grep 0.51 and MSRV 1.90
- 0076 — `read` accepts several files in the raw format
- 0077 — answer family and walk honesty
- 0078 — one schema per verb, served as a union
- 0079 — the lock is taken on the target inode
- 0080 — `sparse outline` counts the journal under `--hidden`
- 0081 — the verdict fields name the mechanism and the subject
- 0082 — the sparse family walks one policy
- 0083 — `--hidden` follows the flag in every walking verb
- 0084 — every mutating verb publishes the write verdict
- 0085 — the edit family publishes the write verdict
- 0086 — `apply` publishes the backup path and the rollback
- 0087 — the transaction snapshot follows the backup policy
- 0088 — the deletion keeps its undo and the reversal declares what it kept
- 0089 — the target is always an array and the batch names its copies
- 0090 — the transaction owns the copy and a failed write names what it left
- 0091 — the transaction lends its copy to a rewrite
- 0092 — the delete copies only when asked
- 0093 — the backup key distinguishes unset from false
- 0094 — the recursive delete owns the whole named tree
- 0095 — search submatch offsets are bytes


## Test Architecture
- Integration suites: 112 `.rs` files in `tests/` (count them with `fd -e rs -d 1 . tests`), plus unit tests colocated in `src/` and doctests, and the per-test total is not frozen in this document (count `#[test]` with `rg -c '#\[test\]' tests src`)
- Unit tests are colocated with the code under `#[cfg(test)]` modules
- Integration tests live in `tests/` and use `assert_cmd` + `predicates` for shell-out tests
- Every test that runs the binary goes through `common::hermetic(...)` (`tests/common/mod.rs`): an empty `HOME` and `XDG_*`, a fixed `LANG`, and the real XDG cache so the AST parsers are not downloaded again (ADR-0068)
- Source guards fail the build: `tests/cli_v0137_hardcode_guard.rs` (duration literals, `.bak.` infix, sidecar names, time-unit numbers outside `constants*`, `env::var`, `Command::new`, no `.sh` file, no `.github` directory) and `tests/cli_v0137_let_underscore_guard.rs` (`let _ =`, `.unwrap()`, `.expect(`, `panic!`, `todo!`, `unimplemented!` in production code), both with empty allowance lists
- Documentation guards: `tests/docs_consistency.rs` and `tests/docs_exit_codes.rs`
- Property-based tests via `proptest` for checksum, backup and fuzzy
- Cross-compile gate via `tests/cross_compile_check.rs`
- Snapshot tests via `insta` for stable NDJSON output
- Two fuzz targets under `fuzz/fuzz_targets/` (`batch_parse`, `extract_json`)
- v0.1.36 agent contract: `tests/cli_v0136_gaps.rs`


## v0.1.35 residual contracts (architecture notes)
- Large write guard: default-deny when the existing target size exceeds XDG `[write].confirm_large_bytes`, which requires `--ack-overwrite` (one-shot), and `--require-large-ack` is independent (not a clap alias of `--confirm`)
- Delete surface: `--plan` / `--dry-run` plan-only, and `--confirm` and `--yes`/`-y` are rejected fail-closed
- Watch: always emits the terminal NDJSON `type:watch_summary`, idle default 500 ms, debounce from CLI or XDG `[watch].debounce_ms`
- Semantic-merge: conflict markers default ON, and `--no-conflict-markers` opts out
- SRP: secondary monoliths are split via `include!` (ADR-0069)
- Observability: `init_tracing` only, local diagnostics on stderr, no product telemetry
- Config: CLI + XDG / `.atomwrite.toml` only for product knobs, and `doctor` reads no environment variable (host detection uses file probes only)
- Contract language: `message` and `reason` of NDJSON errors and the clap help text are English by contract, and only `suggestion` is localized (en and pt-BR, with `suggestion_lang`)


## v0.1.36 agent contract (ADR-0055)
- Historical (v0.1.36 and early v0.1.37): 46 subcommands (`doctor`, `locale`, `commands`, `scratch` completed the 41 → 45 jump, `extract-artifact` followed in v0.1.37), and the final v0.1.37 SSOT is 47 with `config`
- Error envelope always `ok: false` (required), `match_count` present (0 on most match failures)
- Fuzzy honesty: normalized strategies → `similarity: null`, dual-gate `block_anchor`, soft floor via XDG `[fuzzy].soft_floor_delta` (default 0.05) on `context_aware`
- Content-source paths (`--old-file`/`--new-file`) remain under the workspace jail, and agents stage via `scratch init` → `.atomwrite/scratch/` (policy, not a vulnerability)
- Global shape flags (`--select`/`--fields`, `--filter`, `--limit`) reshape stdout at runtime, they are not NDJSON schema fields, and errors always-emit under `--filter`


## v0.1.37 and v0.1.38 contracts — bounded read, honest patch, inode lock

### Bounded read
- `read` composes the cap (`read.max_output_bytes`, 24,000 bytes), the range windows, `--number`, `--symbol`, `--line N --enclosing` and `--no-comments` in one pass over the same line visitor
- The raw path streams a single target through the same `stream_raw_targets` the multi-target path already used, so the peak memory no longer grows with the file size, and BLAKE3 runs only under `--verify-checksum`
- The cap cuts on a complete line and at the end of the last outline item, the NDJSON event carries `truncated: true` with the `next` hint, and the stderr notice carries the exact `--lines` that reads the rest
- `--no-comments` without `--number` is refused, because a dropped line would point the follow-up `apply` at the wrong one

### Compact shapes
- `search --format text`, `list --format text` and `outline --format tsv` answer in the shape the native agent tools already parse
- `search.format`, `read.number` and `output.profile` are XDG keys, so the cheap shape does not depend on a flag repeated on every call
- `outline` parses raw bytes, publishes `parse_errors` with `first_error_line`, and closes a Markdown section at the line before the next heading of the same or higher level
- `search` closes with `ALL_SKIPPED_BINARY` (exit 89) when every candidate was binary, so "nothing was searched" stops looking like "nothing matched"

### Patch contract
- `apply` reads every file of a multi-file patch, matches whole lines, and refuses an ambiguous or malformed block before the first write
- A write that fails after another file was written restores the files already written, so the call leaves no half-applied patch
- The syntax check compares the parse-error count before and after the swap and refuses only on a rise, so a file that already had an error keeps being editable (ADR-0086)
- `apply --show` and `diff` publish 1-based lines, and `diff` separates `old_line` from `new_line`

### Lock and verdict
- The lock is `std::fs::File::try_lock` on the target inode, and the `.<target>.atomwrite.lock` sidecar is a compatibility file whose failure no longer aborts the write (ADR-0079)
- The failure envelope carries `ok`, `code`, `exit`, `path` and `suggestion`; `message`, `error_class`, `retryable` and `suggestion_lang` appear only under `-v`, and `target_resolved` is always an array (ADR-0089)
- Every mutating verb publishes `write_strategy` and, when a copy was refused, `backup_skipped` (ADR-0084, ADR-0085)
- `delete --recursive` owns the whole tree in one call and publishes `backups_kept`, `backup_bytes` and `dirs_left` when a backup or a leftover exists, so the caller learns what remains without walking again


## Maintenance
- This document and `ARCHITECTURE.pt-BR.md` are maintained together, so a change to one lands in the other in the same delivery
- Counts here (error variants, schemas, test files) come from the commands shown next to them, re-measure before changing a number
