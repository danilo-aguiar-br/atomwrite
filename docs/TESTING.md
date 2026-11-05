# atomwrite Testing Guide


[Leia em Português](TESTING.pt-BR.md)


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


## Current Release Line: v0.1.38
- The full suite exited 0 on 2026-11-04 with `cargo test --no-fail-fast`, and the run printed 134 `test result: ok` blocks
- The failure envelope schema requires `ok`, `code` and `exit`, so a suite that asserted `error: true` reads `.ok == false` plus `.code` instead
- No test was added for the 0.1.38 fixes and the existing suites were updated in place, because a real run on the installed binary is the proof and a new mock is not
- The 0.1.38 binary was executed on 2026-11-04 and answered `agent-surface` with 47 tools, 42 exit codes and 34 distinct numbers, and `config list` with 43 keys
- The 47 examples of the smoke test below were run against the 0.1.38 binary on 2026-11-04, and 46 exited 0 while `watch` exited 78


## What's New in v0.1.37
- Full CLI surface under test policy: 47 subcommands (`CANONICAL_SUBCOMMAND_COUNT`): `agent-surface`, `apply`, `backup`, `batch`, `calc`, `case`, `codemod`, `commands`, `completions`, `config`, `copy`, `count`, `del`, `delete`, `diff`, `doctor`, `edit`, `edit-loop`, `extract`, `extract-artifact`, `get`, `hash`, `list`, `locale`, `move`, `outline`, `prune-backups`, `query`, `read`, `recipe`, `regex`, `replace`, `rollback`, `scope`, `scratch`, `search`, `semantic-merge`, `semantic-search`, `set`, `sparse`, `stat`, `transform`, `verify`, `wal-heal`, `wal-stats`, `watch`, `write`
- Documentation suites: `cargo test --test docs_coverage --test docs_consistency --test docs_exit_codes --test docs_schemas_sync`
- `docs_coverage` demands every subcommand and every local flag in each primary doc, and `docs_consistency` checks every quoted total against the real binary and proves its own reader rejects a stale count
- `docs_exit_codes` compares the README exit tables with `agent-surface`, and `docs_schemas_sync` keeps the schema files and the published names in step
- `config` verb: `cargo test --test cli_v0137_config_verb` (set/get/unset round trip, scope required, invalid values write nothing, layering key by key, atomic 0600 write)
- Terminal event and failure report: `cargo test --test cli_v0137_terminal_event --test cli_v0137_terminal_pipe --test cli_v0137_terminal_schema` (one terminal event, `MATCH_FAILED` for a missing pair, `files_failed` and `errors[]`, exit 141 with a closed reader, the six summary schemas)
- Retry, cancel and limits: `cargo test --test cli_v0137_retry_deadline --test cli_v0137_retry_guards --test cli_v0137_cancel --test cli_v0137_search_rss --test cli_v0137_wal_startup_heal`
- Source guards (they fail the build): `cargo test --test cli_v0137_hardcode_guard --test cli_v0137_let_underscore_guard`
- `cli_v0137_hardcode_guard` scans the production code under `src/` (it skips `constants*` files, test files, `#[cfg(test)]` items and comments). It forbids duration literals in `Duration::from_*`, the `.bak.` infix, sidecar and probe names and time-unit numbers outside `constants*`, and it forbids `env::var`, `env::var_os`, `env::vars` and `Command::new` anywhere in production code. It also checks that the repository ships no `.sh` file and no remote-automation directory. The `ALLOWED` list of exceptions is empty
- `cli_v0137_let_underscore_guard` forbids `let _ =` in production code, and forbids `.unwrap()`, `.expect(`, `unreachable!(`, `panic!(`, `todo!(` and `unimplemented!(` there. Both allowance lists are empty
- Hermetic harness: every test that runs the binary goes through `common::hermetic(...)` in `tests/common/mod.rs`. The child gets an empty `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `XDG_STATE_HOME` under the cargo target tmp directory, `LANG` and `LC_ALL` fixed to `en_US.UTF-8`, and `LANGUAGE` and `LC_MESSAGES` removed
- `XDG_CACHE_HOME` stays on the real cache of the machine on purpose: the `ast` feature downloads the tree-sitter parsers there on first use, and an empty cache would send the `outline`, `query` and `--syntax-check` tests to the network
- A test that needs its own config redirects `HOME` and `XDG_*` after it calls `hermetic`, because the later `env` call wins
- Version and slim build: `cargo test --test cli_v0137_version`; `cargo xtask check-features` checks the `core` and `core,watch` feature sets
- The supported test run uses the default features: `cargo test --locked`. Do not use `cargo test --no-default-features --features core` as a gate (see Local test profiles)
- Local DoD (local gates only, no remote automation): `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`
- Pin docs/agents to `^0.1.38`


## Smoke Test of the 47 Subcommands
- One example per subcommand, each run against `atomwrite 0.1.38` (the command, its one-line summary and the example)
- Setup: `WS` is the absolute path of an empty workspace that holds `src/a.rs` (an `unwrap()` and a comment), `src/b.rs` (the word `beta` once), `src/c.rs` (with `user_id`), `src/tmp.rs`, `config.toml` (a `[package]` table with `name` and a `[features]` table with `x`), `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` and `rules.yaml` (a YAML list with `id`, `language`, `pattern` and `rewrite`)
- `HASH_A` is the `checksum` that `hash src/a.rs` prints
- Run them with `HOME` and the `XDG_*` variables pointing inside the workspace, as `common::hermetic` does for the test suite, so that no real config, locale preference or completion file is touched
- The examples run in the order below: `rollback` needs the `backup` before it and `edit-loop` needs the `edit` before it
- Result: 46 of the 47 exit 0, and `watch` exits 78 because the default build has no `watch` feature. A build with `--features watch` was not run

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


## What's New in v0.1.36
- Historical (v0.1.36): residual agent-contract CAPA, always-emit errors under `--filter`; extract `--field`; `CANONICAL_SUBCOMMAND_COUNT` was 46
- Historical (v0.1.36): full CLI surface under test policy was 46 subcommands (`CANONICAL_SUBCOMMAND_COUNT`): `agent-surface`, `apply`, `backup`, `batch`, `calc`, `case`, `codemod`, `commands`, `completions`, `config`, `copy`, `count`, `del`, `delete`, `diff`, `doctor`, `edit`, `edit-loop`, `extract`, `get`, `hash`, `list`, `locale`, `move`, `outline`, `prune-backups`, `query`, `read`, `recipe`, `regex`, `replace`, `rollback`, `scope`, `scratch`, `search`, `semantic-merge`, `semantic-search`, `set`, `sparse`, `stat`, `transform`, `verify`, `wal-heal`, `wal-stats`, `watch`, `write`
- Suite `tests/cli_v0136_gaps.rs` covers GAP-019…033 contract points (workspace jail / scratch staging, error `ok:false` + `match_count`, fuzzy honesty with normalized strategies emitting `similarity: null` (never fake 1.0), dual-gate / soft-floor policy, shape flags, extract `--field` not `--select`, staging formulas)
- Local SSOT gate: the gap inventory is a local unversioned note, so the current status of every gap lives in the ADR series and review keeps both aligned
- Run: `cargo test --test cli_v0136_gaps`
- Local DoD (local gates only): `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`
- Historical pin `^0.1.36`; ADR-0055


## What's New in v0.1.35
- Residual A-* closed: suite green without `delete --yes`; large overwrite default-deny (`--ack-overwrite`); `watch` always emits `watch_summary`; semantic-merge conflict markers default ON; monólitos `include!` splits
- Local DoD (local gates only): `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`, `cargo check --target x86_64-pc-windows-gnu`, `cargo install --path . --force`
- Contract suite: `cargo test --test cli_e2e_v0135`
- See ADR-0055 for the A-* closeout matrix


## What's New in v0.1.34
- Suite `tests/cli_v0133_oneshot_fuzzy.rs` locks the one-shot fuzzy + timeout contract (code fix logged as 0.1.33; docs-complete publish 0.1.34)
- Covers: expand-section hang when NEW embeds OLD finishes in under 2 seconds; embeds force single apply via `apply_fuzzy_one_pass`; hard ceiling 10_000 applies; global timeout default 120 / exit 124; cooperative cancel mid-cascade / exit 143
- Explicit contract under test: `apply_fuzzy_one_pass` one-pass L→R (never re-scans inserted text); NEW embedding OLD must not hang
- Run: `cargo test --test cli_v0133_oneshot_fuzzy`
- Keep older suites listed below
- See [ADR-0054](decisions/0054-v0-1-34-oneshot-fuzzy-timeout.md)


## What's New in v0.1.30
- Suite `tests/cli_v0130_agent_contract.rs` covers residual contract: match_count, indent_adjusted, fuzzy modes, recipe bak skip, sparse outline kind, semantic-merge help line-based, replace-all uniqueness. Note (v0.1.35): `--fuzzy off` = exact-only (G-010), not rejected
- Suite `tests/cli_v0129_fuzzy_replace.rs` covers the 0.1.29 surface: fuzzy replace, `best_candidate`, durability, recipe, sparse, semantic-merge, stat, agent-surface, semantic-search, `platform.rename_method`
- `replace --fuzzy auto|aggressive|off` (`off` = exact-only, G-010; default `auto`) with optional `--fuzzy-threshold` and bulk `--progress-every`
- Match-failure envelopes may include `best_candidate` (schema `best-candidate.schema.json`)
- Cooperative cancel emits `cancelled` NDJSON events (schema `cancelled-event.schema.json`, exit 143)
- Write NDJSON reports `platform.rename_method` (`renameat2` or `rename`)
- `write --durability full|fast|auto` trade-off for fsync cost
- Historical (at v0.1.29): new subcommands: `semantic-merge`, `sparse`, `recipe`, `stat`, `agent-surface`, `watch` (feature), `codemod`, `semantic-search` — 41 subcommands total
- Feature matrix: slim `core` (26,560,352 bytes = 25.33 MiB measured on 2026-11-04, above the local size-gate of 15 MiB) vs default AST (71,084,440 bytes = 67.79 MiB measured on 2026-11-04)
- Local gates (manual; no remote automation): size-gate slim ≤15 MiB, whose measured slim build on 2026-11-04 is 25.33 MiB and above it; core-test; schema-diff on `docs/schemas`
- Run the 0.1.30 contract suite: `cargo test --test cli_v0130_agent_contract`
- Run the 0.1.29 suite: `cargo test --test cli_v0129_fuzzy_replace`


## What's New in v0.1.28
- 661 tests passing, 0 failures, 3 ignored (cross-compile gate)
- BREAKING: `delete` now creates a backup by default (was opt-in) — pass `--no-backup` to disable
- BREAKING: `move`/`copy` overwriting an existing destination now require `--force` OR an explicit `--backup`
- GAP-CLI-SURFACE-DRIFT resolved: single `BackupOpts` struct flattened via `#[command(flatten)]` into 15 mutating subcommands, replacing 15 hand-declared divergent flag sets
- GAP-CONFIG-DEFAULTS-DEAD resolved: `.atomwrite.toml` `[defaults]` `backup`/`retention` keys are now effective end-to-end via a single `resolve_backup()` call fed by `load_config()` in `lib.rs::run()`
- New shared `edit` stdin-tty guard: 7 stdin-consuming modes fail fast with exit 65 `INVALID_INPUT` instead of blocking indefinitely when stdin is a terminal
- New test files: `cli_v0128_backup_matrix.rs` (12 tests), `cli_v0128_config_defaults.rs` (6 tests), `cli_v0128_edit_stdin_guard.rs` (3 tests), `cli_v0128_batch_backup.rs` (6 tests)
- 33 subcommands, 32 ADRs (0019-0050)


## What's New in v0.1.27
- 631+ tests passing, 0 failures, 3 ignored (cross-compile gate)
- 10 bugs fixed: BUG-SEC-001 (CRITICAL symlink-dir escape), BUG-SCOPE-004 (CRITICAL scope --delete code destruction), BUG-GET, BUG-001, BUG-002, BUG-005, BUG-008, BUG-SCOPE-002, BUG-SCOPE-003, BUG-SCOPE-005
- 3 known limitations documented: GAP-01 (test-fn), GAP-02 (doc-comment), GAP-03 (JS export)
- `tests/cli_get_del.rs` updated: `get` missing key expects exit 65 INVALID_INPUT (was exit 4)
- `scope.rs` unit test `lookup_rust_queries_known` updated for pub-variant patterns
- `scope.rs` unit test `lookup_go_queries_known` updated for var-without-type pattern
- 33 subcommands, 29 ADRs (0019-0047)


## What's New in v0.1.25
- 631 tests passing, 0 failures, 3 ignored (cross-compile gate)
- 49 additional gaps resolved in 6 e2e audit rounds (GAP-071 through GAP-134)
- Property-based fuzzy tests via proptest (5 properties in `tests/proptest_fuzzy.rs`)
- ~505 e2e scenarios executed against real binary


## What's New in v0.1.24
- 621 tests passing, 0 failures, 3 ignored (cross-compile gate)
- 52 gaps resolved (GAP-2026-019 through GAP-2026-070)
- 20 `anyhow::bail!()` converted to typed `AtomwriteError` (inline unit tests added per command)
- 3 new ADRs (0045-0047)

### New Test Files (v0.1.24)
- `tests/cli_v0124_clap_suggestion.rs` — 5 tests for GAP-019 (actionable clap error suggestions)
- `tests/cli_gap020_diff_workspace.rs` — 3 tests for GAP-020 (diff resolve-first)
- `tests/cli_gap021_scope_readonly.rs` — 4 tests for GAP-021 (scope read-only mode)

### Bug Fixes Validated by Existing Test Suites
- GAP-022 to GAP-070: fixes validated by existing integration tests in `cli_*.rs` files
- Each `bail!` → `AtomwriteError` conversion includes inline `#[cfg(test)]` assertions
- `cargo test` confirms 621 total: 609 (v0.1.23) + 5 (GAP-019) + 3 (GAP-020) + 4 (GAP-021)


## What's New in v0.1.23
- 609 tests passing (575 baseline v0.1.22 + 12 GAP-2026-015 + 7 GAP-2026-016 + 4 GAP-2026-017 + 8 GAP-2026-018 + 3 from [Unreleased] Windows)
- 33 subcommands (32 from v0.1.22 + `verify` from v0.1.25)
- 26 ADRs in docs/decisions/ (0019-0044)
- GAP-2026-015 (allow_hyphen_values) closed; 12 CLI fields across 8 structs accept values starting with `-`, and the three positionals require `--` since 0.1.37
- GAP-2026-016 (backup-by-default) closed; backup enabled by default in 9 content-mutating structs
- GAP-2026-017 (shrink guard) closed; writes shrinking >50% blocked when --expect-checksum active
- GAP-2026-018 (--old-file/--new-file) closed; edit reads match/replacement from files, bypassing ARG_MAX
- New tests: `tests/cli_v0123_hyphen_values.rs` (12), `tests/cli_v0123_backup_default.rs` (7), `tests/cli_v0123_shrink_guard.rs` (4), `tests/cli_v0123_old_file.rs` (8)


## What's New in v0.1.22: Test Totals
- 631+ tests passing (542 baseline v0.1.18 + 33 in v0.1.21-v0.1.22 + 46 in v0.1.23-v0.1.24 + 10 in v0.1.25)
- 33 subcommands (30 baseline v0.1.18 + `edit-loop` + `prune-backups` from v0.1.22 + `verify` from v0.1.25)
- 22 ADRs in docs/decisions/ (0019-0040)
- 2 new NDJSON schemas: `edit-loop-output.schema.json`, `prune-backups-output.schema.json`


## What's New in v0.1.21
- GAP-2026-012 (--allow-sequential-drift flag) closed; sequential edits no longer require re-capturing checksum between iterations
- GAP-2026-013 Front 4 (--backup exposed in `edit` and `rollback`) closed; parity with `write` and `replace`
- GAP-2026-014 v2 (backup default-deletes after success) closed; `--keep-backup` opt-in escape hatch added
- ADR-0038 created documenting the backup-default-deletes paradigm
- New tests: `tests/cli_v0121_sequential_drift.rs` (4), `tests/cli_v0121_backup_keep_flag.rs` (3), `tests/cli_v0121_edit_backup.rs` (3), `tests/cli_v0121_rollback_backup.rs` (2), `tests/cli_v0121_apply_keep.rs` (2), `tests/cli_v0121_batch_keep.rs` (2), `tests/proptest_v0121_backup_delete.rs` (2 property tests)


## What's New in v0.1.22
- GAP-2026-012 Front 3 (edit-loop helper subcommand) closed; new `edit-loop` applies N pairs {old, new} in 1 invocation via NDJSON on stdin
- GAP-2026-013 Front 2 (prune-backups subcommand) closed; new `prune-backups` provides manual cleanup of legacy `.bak.YYYYMMDD_HHMMSS` files
- ADR-0039 created documenting the `edit-loop` design
- ADR-0040 created documenting the `prune-backups` design
- New tests: `tests/cli_v0121_edit_loop.rs` (4), `tests/cli_v0121_prune_backups.rs` (3)


## What's New in v0.1.12 (historical)

This section summarizes the test-relevant changes in v0.1.12. The release added 96 new tests (was 320 baseline) for a total of 445 tests in 43 test suites. v0.1.15 raises the total to 542 tests (+2 unit tests in v0.1.14, +8 G117 tests in `tests/cli_edit.rs`, +6 G118 tests in `tests/cli_write.rs`).

### New Test Files (10 Added in v0.1.11+v0.1.12)
- `tests/cli_v012_regressions.rs` -- 11 tests for v0.1.2 through v0.1.4 regressions
- `tests/cli_v012_audit_regressions.rs` -- 27 tests for the v0.1.12 G72/G114 audit
- `tests/cli_v012_batch4_regressions.rs` -- 23 tests for the v0.1.12 final batch
- `tests/cli_v012_syntax_check.rs` -- 5 tests for G72 tree-sitter validation
- `tests/cli_v012_wal.rs` -- 8 tests for G114 WAL sidecar
- `tests/cli_v012_xattr_reflink.rs` -- 3 tests for G39 xattr + G64 reflink
- `tests/cli_set.rs` -- 6 tests for v14 Tier 3 set subcommand
- `tests/cli_get_del.rs` -- 5 tests for v14 Tier 3 get/del subcommands
- `tests/cli_query.rs` -- 5 tests for v14 Tier 3 query subcommand
- `tests/cli_outline.rs` -- 5 tests for v14 Tier 3 outline subcommand
- `tests/cli_case.rs` -- 3 tests for v14 Tier 3 case subcommand

### New Tests in Existing Files
- 12 tests in `src/binary_detect.rs::tests` (G41 content_inspector)
- 16 tests in `src/syntax_check.rs::tests` (G72 tree-sitter)
- 8 tests in `src/wal.rs::tests` (G114 WAL)
- 3 tests in `src/xattr_restore.rs::tests` (G39 xattr)
- 3 tests in `src/lock.rs::tests` (G54 advisory lock)
- 3 tests in `src/atomic.rs::tests` (WriteStrategy: rename/inplace/copyback)

### Test Suite Organization
- Total: 43 test suites (was 34 in v0.1.10)
- Unit tests in `src/`
- Integration tests in `tests/cli_*.rs`
- Property-based tests in `tests/proptest_*.rs`
- Snapshot tests via `insta`
- Signal tests (SIGINT, SIGTERM, SIGPIPE)


### How to Run

```bash
# Run all tests
cargo test

# Run the v0.1.33/v0.1.34 one-shot fuzzy + timeout suite
cargo test --test cli_v0133_oneshot_fuzzy
# Run the v0.1.30 residual contract suite
cargo test --test cli_v0130_agent_contract
# Run the v0.1.29 suite (fuzzy replace + new surface)
cargo test --test cli_v0129_fuzzy_replace

# Run only the v0.1.12 regression suite
cargo test --test cli_v012_regressions
cargo test --test cli_v012_audit_regressions
cargo test --test cli_v012_batch4_regressions

# Supported run: the full suite with the default features
cargo test --locked

# Slim build: only prove that it compiles (xtask exists in a source checkout only)
cargo xtask check-features

# Run with output visible for debugging
cargo test --test cli_v012_syntax_check -- --nocapture

# Run the cross-compile gate
cargo test --test cross_compile_check -- --ignored
```

### Coverage
- Historical (at v0.1.12): 20.19% line coverage via `cargo tarpaulin` (935/4631 lines covered), measured at that release and not repeated since
- It was low because tarpaulin only counts unit tests, not CLI integration tests
- The integration test suite is the primary coverage metric (see Current Stats)

### Dependencies Added
- `tree-sitter-language-pack = "1.8"` -- 305 languages for query/outline/syntax-check at that version, and the tree now pins 1.22.1 with 372
- All test dependencies are already in the dev-dependencies section

### ADRs and Schemas
- 29 ADRs in `docs/decisions/` (0019-0047) explain the architectural decisions behind the v0.1.12 to v0.1.27 features
- Historical (at v0.1.27): 29 JSON schemas in `docs/schemas/`; the current count is 54 (full index in `docs/schemas/README.md`)
- See [docs/decisions/README.md](decisions/README.md) for the full list of ADRs


## Why Categorized Tests
- Each test category validates a different layer of the system
- Unit tests verify pure logic in isolation
- Integration tests verify CLI behavior end-to-end
- Snapshot tests lock down JSON output structure
- Property-based tests discover edge cases humans miss
- Running all categories together gives confidence that atomwrite works correctly


## Current Stats
- Final run of v0.1.38 recorded in `CHANGELOG.md`: 131 test targets, 1486 tests passed, 0 failed and 9 ignored (the `cross_compile_*` tests)
- Inventory measured with `fd` and `rg` on the source tree: 131 integration test files in `tests/` (plus `tests/common/mod.rs`, `doc_scan.rs` and `schema_check.rs`), 994 `#[test]` functions in `tests/` and 485 in `src/` (DERIVED: 994 + 485 = 1479), plus 9 snapshot files, 4 proptest files and 2 fuzz targets
- Measure it again with `rg -c '#\[test\]' tests src` and `cargo test --lib --tests`. A macro-generated test (proptest) is not counted by that `rg`, so the number of tests that run can differ from the number of `#[test]` attributes
- Suites by theme: unit + integration + snapshot + property-based + signal + tracing + NDJSON + regression + cross-compile + concurrency
- Historical (v0.1.11 and v0.1.12): 96 new tests were added then:
  - 11 tests in `tests/cli_v012_regressions.rs` (GAP 13, GAP 14, GAP 18 fixes)
  - 27 tests in `tests/cli_v012_audit_regressions.rs` (v0.1.12 G72/G114 audit)
  - 23 tests in `tests/cli_v012_batch4_regressions.rs` (v0.1.12 final batch)
  - 5 tests in `tests/cli_v012_syntax_check.rs` (G72 tree-sitter)
  - 8 tests in `tests/cli_v012_wal.rs` (G114 WAL sidecar)
  - 3 tests in `tests/cli_v012_xattr_reflink.rs` (G39 xattr + G64 reflink)
  - 6 tests in `tests/cli_set.rs` (v14 Tier 3)
  - 5 tests in `tests/cli_get_del.rs` (v14 Tier 3)
  - 5 tests in `tests/cli_query.rs` (v14 Tier 3)
  - 5 tests in `tests/cli_outline.rs` (v14 Tier 3)
  - 3 tests in `tests/cli_case.rs` (v14 Tier 3)
  - 3 mtime regression tests in `src/atomic.rs::tests`
  - 7 GAP 13 error-suggestion tests in `src/error.rs::tests`
  - 3 cross-compile gate tests in `tests/cross_compile_check.rs` (v0.1.4; there are 9 now)
  - 16 tests in `src/syntax_check.rs::tests` (G72 tree-sitter)
  - 8 tests in `src/wal.rs::tests` (G114 WAL)
- 12 tests in `src/binary_detect.rs::tests` (G41 content_inspector)
- 3 tests in `src/xattr_restore.rs::tests` (G39)
- 3 tests in `src/lock.rs::tests` (G54 advisory lock)
- 9 snapshot files in `tests/snapshots/`
- 2 proptest regression files
- 2 fuzz targets in `fuzz/fuzz_targets/`


## Test Categories
### Unit Tests (src/)
- Located inside source files via `#[cfg(test)]` modules
- Test pure functions without filesystem or CLI interaction
- `path_safety.rs` -- 5 tests for path traversal prevention and workspace jail
- `checksum.rs` -- BLAKE3 hashing correctness

### Integration Tests (tests/cli_*.rs)
- Located in `tests/` directory
- Each file tests one subcommand end-to-end via the compiled binary
- Use `assert_cmd` to invoke atomwrite as a subprocess
- Use `tempfile` crate for isolated filesystem fixtures
- `cli_write.rs` -- 6 tests for atomic write operations
- `cli_read.rs` -- 6 tests for read with metadata and checksums
- `cli_edit.rs` -- 5 tests for surgical edit modes
- `cli_search.rs` -- 6 tests for parallel search
- `cli_replace.rs` -- 6 tests for bulk text replacement
- `cli_hash.rs` -- 4 tests for BLAKE3 checksum operations
- `cli_delete.rs` -- 4 tests for file deletion with backup
- `cli_count.rs` -- 2 tests for line and file counting
- `cli_diff.rs` -- 4 tests for file comparison
- `cli_move.rs` -- 5 tests for atomic move and rename
- `cli_copy.rs` -- 4 tests for copy with checksum verification
- `cli_list.rs` -- 4 tests for project structure listing
- `cli_extract.rs` -- 4 tests for NDJSON field extraction
- `cli_calc.rs` -- 5 tests for math and unit conversion
- `cli_regex.rs` -- 5 tests for regex generation from examples
- `cli_transform.rs` -- 5 tests for AST search and rewrite
- `cli_scope.rs` -- scope operations: find functions, delete comments, unknown language error, custom patterns
- `cli_backup.rs` -- backup creation, dry-run, not-found error, multiple files
- `cli_rollback.rs` -- restore from backup, dry-run, no-backup error, verify flag
- `cli_apply.rs` -- full file replacement, SEARCH/REPLACE blocks, unified diff, dry-run
- `cli_batch.rs` -- 13 tests for batch operation execution (write, replace, delete, move, copy, path alias, transaction rollback)

### Snapshot Tests (insta)
- Located in `tests/snapshot_write.rs` -- 9 tests
- Use the `insta` crate for snapshot-based output verification
- Lock down the exact JSON structure of each output type
- Snapshot files stored in `tests/snapshots/`
- Run `cargo insta review` to accept or reject snapshot changes
- Available snapshots:
- `snapshot_write__write_output_structure.snap`
- `snapshot_write__read_output_structure.snap`
- `snapshot_write__edit_output_structure.snap`
- `snapshot_write__search_match_structure.snap`
- `snapshot_write__replace_result_structure.snap`
- `snapshot_write__error_not_found_structure.snap`
- `snapshot_write__batch_summary_structure.snap`
- `snapshot_write__error_invalid_input_structure.snap`
- `snapshot_write__error_workspace_jail_structure.snap`

### Regression Tests
- `tests/cli_v012_regressions.rs` -- 11 tests for v0.1.2 through v0.1.4 regressions
- `jail_suggestion_mentions_workspace_flag` -- updated in v0.1.4 (GAP 13): asserts the suggestion mentions `--workspace` when no workspace is provided
- `gap13_jail_suggestion_when_workspace_supplied_says_inside` -- added in v0.1.4 (GAP 13): asserts the suggestion says "inside the workspace" when `--workspace` IS provided
- Other regressions cover scope, batch path/source aliases, fuzzy edit, search files dedup, edit multi NDJSON, clap JSON errors, json-schema, validate path jail, dead flags, mtime

### Cross-Compile Gate (v0.1.4)
- `tests/cross_compile_check.rs` -- 9 tests (3 guarded Windows-only compilation in v0.1.4, 6 more cover Windows arm64, Linux musl and aarch64, Apple aarch64 and x64)
- They were NOT validated for v0.1.37: the audit host lacks the foreign linkers (for example `x86_64-w64-mingw32-gcc`)
- The 9 tests are `cross_compile_windows_gnu_x64_succeeds`, `cross_compile_windows_gnu_i686_succeeds`, `cross_compile_windows_msvc_succeeds`, `cross_compile_windows_arm64_msvc_succeeds`, `cross_compile_linux_musl_x64_succeeds`, `cross_compile_linux_aarch64_gnu_succeeds`, `cross_compile_linux_aarch64_musl_succeeds`, `cross_compile_apple_aarch64_succeeds` and `cross_compile_apple_x64_succeeds`
- `cross_compile_windows_gnu_x64_succeeds` -- asserts `cargo check --target x86_64-pc-windows-gnu` succeeds and emits no E0433, E0308, or E0507
- `cross_compile_windows_gnu_i686_succeeds` -- asserts the same for `i686-pc-windows-gnu` (32-bit Windows)
- `cross_compile_windows_msvc_succeeds` -- asserts the same for `x86_64-pc-windows-msvc` (Microsoft Visual C++ toolchain)
- All tests are `#[ignore]` by default to skip on hosts without the Windows targets installed
- Run with `cargo test --test cross_compile_check -- --ignored`
- Skips gracefully when the required linker (lib.exe for MSVC, i686-w64-mingw32-gcc for 32-bit) is missing
- Required before any release that touches `#[cfg(windows)]` code (see `docs/INSTALL.md` and `docs/CROSS_PLATFORM.md`)

### Property-Based Tests (proptest)
- Located in `tests/proptest_checksum.rs` -- 2 tests
- Located in `tests/proptest_backup.rs` -- 2 tests
- Use the `proptest` crate to generate random inputs
- Verify that BLAKE3 checksums are deterministic for any input
- Verify that backup creation works for arbitrary file contents
- Regression files stored alongside test files

### Signal Tests
- Located in `tests/signal_test.rs` -- 5 tests
- `sigpipe_handler_ends_the_run_with_exit_141` proves that a closed pipe ends the run with exit 141 (SIGPIPE has a cooperative handler, not `SIG_DFL`)
- `sigint_during_search_exits_130` and `sigterm_during_search_exits_143` prove the exit codes of the signals
- `batch_interrupted_by_signal` and `shutdown_message_on_stderr` prove the cleanup and the shutdown message
- Verify graceful shutdown behavior on signal reception

### NDJSON Validation Tests
- Located in `tests/ndjson_valid_test.rs` -- 21 tests (counted with `rg -c '#\[test\]'`)
- 19 tests validate the NDJSON output of 19 of the 47 commands: `search`, `hash`, `list`, `diff`, `write`, `read`, `edit`, `replace`, `count`, `backup`, `copy`, `move`, `delete`, `batch`, `calc`, `regex`, `rollback` (not-found case), `apply` and `extract`
- 2 `jaq` interop tests (`search` and `read`) verify piped JSON parsing, and skip when `jaq` is not in `PATH`

### Concurrency Tests
- Located in `tests/cli_concurrency.rs`
- Test parallel file operations and race conditions

### Max Filesize Tests
- Located in `tests/cli_max_filesize.rs`
- Test enforcement of file size limits

### Tracing Tests
- Located in `tests/tracing_test.rs`
- Verify tracing/logging infrastructure behavior

### Fuzzing Targets
- Located in `fuzz/fuzz_targets/batch_parse.rs` -- batch NDJSON parser fuzzing
- Located in `fuzz/fuzz_targets/extract_json.rs` -- extract JSON parser fuzzing
- Require `cargo +nightly fuzz run <target>`
- Run with `--max_total_time=30` for quick validation


## How to Run
### Run All Tests

```bash
cargo test
```

### Run a Specific Test File

```bash
cargo test --test cli_write
cargo test --test cli_search
cargo test --test snapshot_write
cargo test --test proptest_checksum
```

### Run a Specific Test by Name

```bash
cargo test --test cli_write test_write_atomic
cargo test --test cli_edit test_edit_old_new
```

### Run Only Unit Tests

```bash
cargo test --lib
```

### Run Only Integration Tests

```bash
cargo test --tests
```

### Run With Output Visible

```bash
cargo test -- --nocapture
```

### Run Snapshot Review

```bash
cargo insta review
```

### Update All Snapshots

```bash
cargo insta test
cargo insta review
```


## Local test profiles
### Fast (Development)
- Run `cargo test` with default settings
- Skip long-running property tests with `PROPTEST_CASES=10`
- Suitable for pre-commit hooks
- Total time: an old estimate of under 30 seconds, not measured for v0.1.37

### Full (local pre-release v0.1.36+)

Required local release gate (local gates only):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
cargo test --tests
cargo test --test cli_v0136_gaps
```

### Slim (compile check only)
- `cargo test --no-default-features --features core` compiles but is NOT green: 44 tests in 11 targets (`cli_scope`, `cli_transform`, `cli_query`, `cli_outline`, `cli_concurrency`, `cli_e2e_v0135` and others) need the `ast` feature and carry no `#[cfg(feature = "ast")]` (`gaps.md` W19, `CHANGELOG.md` Notes)
- Use `cargo xtask check-features` to prove that the slim build compiles: it runs `cargo check --locked` for the `core` and `core,watch` sets and stops at the first failure
- Run the test suite with the default features

### Full (local pre-release)
- Run `cargo test` with all features
- Run `cargo clippy -- -D warnings` for linting
- Run `cargo fmt -- --check` for formatting
- Run `cargo insta test` for snapshot verification
- Run property tests with default case count
- Suitable for pull request validation
- Total time: an old estimate of under 2 minutes, not measured for v0.1.37

### Release (Pre-Publish)
- Run `cargo test --release` for optimized build testing
- Run `cargo test --test proptest_checksum` with `PROPTEST_CASES=1000`
- Run `cargo test --test proptest_backup` with `PROPTEST_CASES=1000`
- Verify all snapshots are up to date with `cargo insta test`

### Performance (local only)
Rules Rust eficiência + latência: measure before optimizing; never compare
debug vs release; prefer tail percentiles (P50/P99/P999/P9999) over means.

```bash
# Criterion microbenches (hash + read/mmap). Profile inherits release (fat LTO, CGU=1).
# Each bench also prints `latency_hist … P50=…ns P99=…ns P999=…ns P9999=…ns` on stderr.
cargo bench --bench hash_benchmark
cargo bench --bench read_benchmark

# Optional size-optimized binary (same LTO/strip as release, opt-level = "z")
cargo build --profile min-size

# Host-CPU binary for local profiling only (do not publish):
RUSTFLAGS='-C target-cpu=native' cargo build --release

# Hot-path profile (install samply or cargo-flamegraph separately):
# samply record ./target/release/atomwrite search PATTERN --path .
# cargo flamegraph --bin atomwrite -- search PATTERN --path .
```

Latency budget notes (CLI one-shot, not HFT):

| Path | Dominated by | Notes |
|------|----------------|-------|
| `hash` multi-file | BLAKE3 + FS | Parallel digests (rayon), sorted NDJSON emit |
| `hash`/`read` large file | mmap + page cache | `posix_fadvise(SEQUENTIAL)` on Linux |
| stdout NDJSON | serialization + flush | Flush-per-line is intentional (agent TTFB) |
| PGO/BOLT / isolcpus / mlockall | — | N/A for short-lived multi-host CLI |

Release knobs live in `Cargo.toml` (`[profile.release]`, `[profile.bench]`,
`[profile.min-size]`) and the mold linker in `.cargo/config.toml` (Linux x86_64).
No remote benchmark gates: the benchmarks run on the local machine only (project policy).
## Logging and test harness knobs
- Binary runtime logging is CLI + XDG only (v0.1.35): use `-v`/`-vv`/`-vvv` and `-q`/`-qq`; optional file tee under XDG state `…/atomwrite/logs` via `directories::ProjectDirs` (no product `ATOMWRITE_*` or `RUST_LOG` knobs in the binary).
- Tracing entrypoint: `init_tracing` (local diagnostics only — no product telemetry).
- The `common::hermetic` helper sets `HOME`, `XDG_*`, `LANG` and `LC_ALL` on the child process only, for the tests; it is not a product setting.
- Cargo/test harness only (not product config): `PROPTEST_CASES`, `INSTA_UPDATE`, `RUST_TEST_THREADS`, `CARGO_TARGET_DIR`.
- Do not document or rely on product environment variables for logging, locale, workspace, backup, or home override — removed in v0.1.35 (use CLI + XDG only).
- `doctor` reads no environment variable: host detection uses file probes only, and this configures no product knob.


## Troubleshooting
### Snapshot Mismatch
- Run `cargo insta review` to see the diff
- Accept changes with `cargo insta accept`
- Reject and fix with `cargo insta reject`
- Snapshots live in `tests/snapshots/`

### Proptest Regression
- Check `tests/*.proptest-regressions` files for recorded failures
- These files contain minimal reproducing inputs
- Delete regression files to re-discover (not recommended)
- Fix the root cause instead

### Flaky Tests
- Check for filesystem timing issues in integration tests
- Use `--test-threads 1` to serialize test execution
- Verify `/tmp` has sufficient space for temporary files

### Permission Errors
- Integration tests create temporary directories
- Verify write access to `TMPDIR` or `/tmp`
- Some tests verify permission-denied behavior and need writable parent directories
- Use `tempfile::tempdir()` for all filesystem fixtures

### Test Hangs or Times Out
- Check for infinite loops in the code under test
- Use `RUST_TEST_THREADS=1` to run sequentially
- Check that no test writes to a path another test reads

### Parser Download in Tests
- The `outline`, `query` and `--syntax-check` tests need the tree-sitter parser bundle (23,031,807 bytes on Linux x86_64, which is 21,97 MiB) in the real `XDG_CACHE_HOME`; the first run downloads it, so run it once with network access or fill the cache beforehand
- The tests never read the real `HOME`, config or locale preference: the `common::hermetic` helper isolates them, and only the cache is shared

### A Test Depends on the Machine
- If a test passes on one machine and fails on another, check that it builds its command through `common::hermetic(...)` and does not read a config of the developer
- A test that sets its own config must redirect `HOME` and `XDG_*` after calling `hermetic`

### Slim Tests Fail
- `cargo test --no-default-features --features core` is expected to fail 44 tests in 11 targets that need the `ast` feature; run the suite with the default features instead (see Local test profiles)

### Cross-Compile Gate Fails
- Confirm the Windows target is installed: `rustup target list --installed`
- Install missing targets: `rustup target add x86_64-pc-windows-gnu i686-pc-windows-gnu x86_64-pc-windows-msvc`
- On Linux, install the required toolchain: `mingw64-gcc` (Fedora) or `mingw-w64` (Ubuntu)
- For 32-bit Windows: install `mingw32-gcc` separately
- For MSVC: install Visual Studio 2019+ Build Tools with the C++ workload (lib.exe must be on PATH)
- Re-run after fixing: `cargo test --test cross_compile_check -- --ignored`
- The gate reports a missing linker with a clear stderr snippet like "lib.exe" or "i686-w64-mingw32-gcc"; match that string to the missing toolchain


## v0.1.20 — What Is New

This release introduces a new safety layer called intention guards and renames the global `--lang` flag to `--locale` to disambiguate from the tree-sitter `--lang` selector used by `scope` and `transform`.

### Intention Guards (5 OPT-IN flags)
- `--require-backup` — require `--backup`: abort when it is missing and the target file exists (takes no value)
- `--confirm` — legacy awareness flag on `write` (historical: interactive Y/N on large overwrite). Superseded in v0.1.35: large overwrite is one-shot default-deny via `--ack-overwrite` (no Y/N); `delete --confirm` is rejected (use `--plan`)
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
- 542 tests passing in 47 integration suites, 0 failures
- 11 GAP-2026 closed
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
