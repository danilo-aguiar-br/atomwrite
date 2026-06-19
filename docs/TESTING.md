# atomwrite Testing Guide


[Leia em Portugues](TESTING.pt-BR.md)


## What's New in v0.1.12

This section summarizes the test-relevant changes in v0.1.12. The release added 96 new tests (was 320 baseline) for a total of **445 tests in 43 test suites**. v0.1.15 raises the total to **542 tests** (+2 unit tests in v0.1.14, +8 G117 tests in `tests/cli_edit.rs`, +6 G118 tests in `tests/cli_write.rs`).

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

## What's New in v0.1.23 (Current)

- 609 tests passing (575 baseline v0.1.22 + 12 GAP-2026-015 + 7 GAP-2026-016 + 4 GAP-2026-017 + 8 GAP-2026-018 + 3 from [Unreleased] Windows CI)
- 32 subcommands (unchanged from v0.1.22)
- 26 ADRs in docs/decisions/ (0019-0044)
- GAP-2026-015 (allow_hyphen_values) closed; 15 CLI fields across 8 structs now accept values starting with `-`
- GAP-2026-016 (backup-by-default) closed; backup enabled by default in 9 content-mutating structs
- GAP-2026-017 (shrink guard) closed; writes shrinking >50% blocked when --expect-checksum active
- GAP-2026-018 (--old-file/--new-file) closed; edit reads match/replacement from files, bypassing ARG_MAX
- New tests: `tests/cli_v0123_hyphen_values.rs` (12), `tests/cli_v0123_backup_default.rs` (7), `tests/cli_v0123_shrink_guard.rs` (4), `tests/cli_v0123_old_file.rs` (8)

## What's New in v0.1.22

- 575+ tests passing (542 baseline v0.1.18 + 16 GAP-2026-012 tests v0.1.21 + 12 GAP-2026-014 v2 backup-delete tests v0.1.21 + 5 GAP-2026-013 edit-backup tests v0.1.21)
- 32 subcommands (30 baseline v0.1.18 + `edit-loop` + `prune-backups` from v0.1.22)
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


### How to Run

```bash
# Run all 609+ tests
cargo test

# Run only the v0.1.12 regression suite
cargo test --test cli_v012_regressions
cargo test --test cli_v012_audit_regressions
cargo test --test cli_v012_batch4_regressions

# Run with output visible for debugging
cargo test --test cli_v012_syntax_check -- --nocapture

# Run the cross-compile gate
cargo test --test cross_compile_check -- --ignored
```

### Coverage

- 20.19% line coverage via `cargo tarpaulin` (935/4631 lines covered)
- Lower than ideal because tarpaulin only counts unit tests, not CLI integration tests
- The integration test suite is the primary coverage metric (575+ tests across 47+ suites)

### Dependencies Added

- `tree-sitter-language-pack = "1.8"` -- 305 languages for query/outline/syntax-check
- All test dependencies are already in the dev-dependencies section

### ADRs and Schemas

- 22 new ADRs in `docs/decisions/` (0019-0040) explain the architectural decisions behind the v0.1.12 to v0.1.22 features
- 29 JSON schemas in `docs/schemas/` (full index in `docs/schemas/README.md`); v0.1.22 added `edit-loop-output` and `prune-backups-output`
- See [docs/decisions/README.md](README.md) for the full list of ADRs

## Why Categorized Tests
- Each test category validates a different layer of the system
- Unit tests verify pure logic in isolation
- Integration tests verify CLI behavior end-to-end
- Snapshot tests lock down JSON output structure
- Property-based tests discover edge cases humans miss
- Running all categories together gives confidence that atomwrite works correctly


## Current Stats
- 70+ Rust files across `src/` and `tests/`
- **575+ tests total across 47+ test suites** (unit + integration + snapshot + property-based + signal + tracing + NDJSON + regression + cross-compile + concurrency)
- **96 new tests added in v0.1.11+v0.1.12**:
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
  - 3 cross-compile gate tests in `tests/cross_compile_check.rs`
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
- `tests/cross_compile_check.rs` -- 3 tests guarding Windows-only compilation
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
- Located in `tests/signal_test.rs` -- 1 test
- Verify graceful shutdown behavior on signal reception

### NDJSON Validation Tests
- Located in `tests/ndjson_valid_test.rs` -- 17 tests
- Validate NDJSON output structure for 20 of 21 commands
- Include `jaq` interop tests verifying piped JSON parsing
- Include i18n test confirming `--lang` does not alter JSON output

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


## CI Profiles
### Fast (Development)
- Run `cargo test` with default settings
- Skip long-running property tests with `PROPTEST_CASES=10`
- Suitable for pre-commit hooks
- Total time: under 30 seconds

### Full (CI Pipeline)
- Run `cargo test` with all features
- Run `cargo clippy -- -D warnings` for linting
- Run `cargo fmt -- --check` for formatting
- Run `cargo insta test` for snapshot verification
- Run property tests with default case count
- Suitable for pull request validation
- Total time: under 2 minutes

### Release (Pre-Publish)
- Run `cargo test --release` for optimized build testing
- Run `cargo test --test proptest_checksum` with `PROPTEST_CASES=1000`
- Run `cargo test --test proptest_backup` with `PROPTEST_CASES=1000`
- Verify all snapshots are up to date with `cargo insta test`


## Environment Variables
- `PROPTEST_CASES` -- number of property test cases (default: 256)
- `RUST_LOG` -- tracing level for debug output during tests
- `ATOMWRITE_LOG` -- set tracing level for test debugging (e.g. `debug`, `trace`)
- `INSTA_UPDATE` -- set to `always` to auto-update snapshots
- `RUST_TEST_THREADS` -- limit parallel test execution (useful for I/O-bound tests)
- `CARGO_TARGET_DIR` -- override target directory for test builds


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

### Cross-Compile Gate Fails
- Confirm the Windows target is installed: `rustup target list --installed`
- Install missing targets: `rustup target add x86_64-pc-windows-gnu i686-pc-windows-gnu x86_64-pc-windows-msvc`
- On Linux, install the required toolchain: `mingw64-gcc` (Fedora) or `mingw-w64` (Ubuntu)
- For 32-bit Windows: install `mingw32-gcc` separately
- For MSVC: install Visual Studio 2019+ Build Tools with the C++ workload (lib.exe must be on PATH)
- Re-run after fixing: `cargo test --test cross_compile_check -- --ignored`
- The gate reports a missing linker with a clear stderr snippet like "lib.exe" or "i686-w64-mingw32-gcc"; match that string to the missing toolchain


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

# Or via ruplacer
ruplacer --subvert --lang --locale
```
