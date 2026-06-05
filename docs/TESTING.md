# atomwrite Testing Guide


[Leia em Portugues](TESTING.pt-BR.md)


## Why Categorized Tests
- Each test category validates a different layer of the system
- Unit tests verify pure logic in isolation
- Integration tests verify CLI behavior end-to-end
- Snapshot tests lock down JSON output structure
- Property-based tests discover edge cases humans miss
- Running all categories together gives confidence that atomwrite works correctly


## Current Stats
- 65+ Rust files across `src/` and `tests/`
- 300+ tests total across 34 test suites (unit + integration + snapshot + property-based + signal + tracing + NDJSON + regression + cross-compile)
- 10 regression tests in `tests/cli_v012_regressions.rs` (added in v0.1.2, expanded in v0.1.4 with `gap13_jail_suggestion_when_workspace_supplied_says_inside` and updated `jail_suggestion_mentions_workspace_flag`)
- 2 mtime regression tests in `src/atomic.rs::tests` (added in v0.1.3)
- 7 GAP 13 error-suggestion tests in `src/error.rs::tests` (added in v0.1.4)
- 3 cross-compile gate tests in `tests/cross_compile_check.rs` (added in v0.1.4): `cross_compile_windows_gnu_x64_succeeds`, `cross_compile_windows_gnu_i686_succeeds`, `cross_compile_windows_msvc_succeeds`
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
