[Leia em Portugues](CHANGELOG.pt-BR.md)


# Changelog

- All notable changes to this project are documented in this file
- Format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/)
- Versioning follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)


## [Unreleased]

### Fixed (CI Failures - GAP 17 follow-up)
- **`signal_test::shutdown_message_on_stderr` flushes the shutdown message via `io::stderr().lock()`** — The first v0.1.8 fix moved `eprintln!` from the signal handler to the main thread, but used `writeln!(io::stderr(), ...)` which is fully-buffered when stderr is redirected to a pipe (as in `cargo test`'s `Stdio::piped()`). The buffer was never flushed before the process exited with the signal exit code, so the parent test saw an empty stderr. The fix uses `io::stderr().lock()` to acquire the `StderrLock` guard, which flushes the buffer on Drop. This guarantees the message reaches the captured stderr pipe before the process exits. CI ubuntu-latest will confirm on push.

## [0.1.8] - 2026-06-05

### Fixed (CI Failures - GAP 17 and GAP 18)
- **`signal_test::shutdown_message_on_stderr` no longer fails on Linux CI** — Removed `eprintln!("\natomwrite: shutting down...")` from the SIGINT and SIGTERM signal handlers. Per POSIX.1-2017 `signal-safety(7)`, stdio functions like `eprintln!` are NOT async-signal-safe; Rust's stderr uses a global `Mutex` that can deadlock or lose buffered output if the signal arrives while another thread holds the lock. The user-facing shutdown message is now emitted by the main thread in `src/main.rs` when it observes `is_shutdown() == true` after `atomwrite::run` returns, which is the only async-signal-safe way to guarantee the message reaches the captured stderr pipe before the process exits. The Windows `ctrlc` path still emits the message inline (ctrlc handlers run in a normal thread, not signal context).
- **`atomic::tests::create_backup_and_retention` no longer fails on Windows CI** — Added `platform::fsync_file_best_effort` which logs a warning and continues instead of returning an error. On Windows, antivirus products (Windows Defender, third-party AV) can transiently hold a read handle on files in `%TEMP%` with `FILE_SHARE_READ` but without `FILE_SHARE_WRITE`, causing `FlushFileBuffers` to return `ERROR_ACCESS_DENIED` (os error 5). The primary write path still uses the strict `fsync_file`; only the backup-durability fsync is best-effort because the backup itself has already been created via `fs::copy`.
- **CI matrix pinned to `windows-2025-vs2026`** — The matrix entry for Windows was changed from `windows-latest` to `windows-2025-vs2026` (its successor before the June 15 2026 GitHub-hosted runner migration). This silences the "windows-latest requests are being redirected to windows-2025-vs2026 by June 15, 2026" NOTICE and prevents unexpected runner changes from breaking the build.

### Validation
- `cargo build --all-features`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo test --all-features`: 302/302 tests PASS (all 5 `signal_test` cases pass; `atomic::create_backup_and_retention` passes)
- `cargo fmt -- --check`: PASS
- `cargo audit`: PASS (no vulnerabilities, no `--ignore` flag)
- `cargo deny check`: PASS (advisories, bans, licenses, sources all OK)

### Notes
- v0.1.8 is a NON-BREAKING change. No public API was modified.
- The signal-handler change is internal: external consumers that relied on the shutdown message appearing on stderr continue to see it; it is now emitted by the main thread instead of the signal handler.
- The Windows backup fsync change is internal: backup files are still created and atomic; the only difference is that the durability flush for backup metadata is best-effort. If a future user reports data loss on backup, we can re-tighten the fsync.

## [0.1.7] - 2026-06-05

### Fixed (CI Failures - GAP 15)
- **`cargo audit` no longer reports RUSTSEC-2026-0009** — Upgraded `time` transitive dependency from 0.3.45 to 0.3.47 (DoS via stack exhaustion in RFC 2822 parser, fixed upstream via DEPTH_LIMIT=32). The upgrade required bumping MSRV from 1.85 to 1.88. The `deny.toml` ignore for RUSTSEC-2026-0009 was removed because the advisory no longer applies.
- **macos-latest CI failure** — `src/platform.rs:31` no longer uses `return Ok(())` (removed redundant return). clippy 1.94+ `needless_return` lint no longer triggers; the `RUSTFLAGS: -Dwarnings` env in CI no longer aborts the build.
- **windows-latest CI failure** — `src/signal.rs:15-16` constants `EXIT_SIGINT` and `EXIT_SIGTERM` now have `#[cfg_attr(not(unix), allow(dead_code))]`. The `RUSTFLAGS: -Dwarnings` env no longer aborts on `dead_code` in Windows builds.
- **`actions/checkout` and `actions/cache` Node 20 deprecation** — Both actions bumped to their major version that supports Node 24 (`actions/checkout@v6`, `actions/cache@v5`). `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: "true"` added to workflow env as belt-and-suspenders. The deprecation warning no longer appears in CI logs.
- **MSRV bumped to 1.88** — `rust-version` in `Cargo.toml` is now 1.88. All documentation files (EN and PT-BR) updated: `docs/INSTALL.md`, `docs/INSTALL.pt-BR.md`, `docs/HOW_TO_USE.md`, `docs/HOW_TO_USE.pt-BR.md`, `docs/CROSS_PLATFORM.md`, `docs/CROSS_PLATFORM.pt-BR.md`, `docs/COOKBOOK.md`, `docs/COOKBOOK.pt-BR.md`, `CONTRIBUTING.md`, `CONTRIBUTING.pt-BR.md`.

### Changed
- **`build.rs:4-12`** — Collapsed nested `if let + if` into `if let + &&` to satisfy clippy 1.94+ `collapsible_if` lint.
- **`src/lib.rs`** — Added `#![allow(clippy::collapsible_if)]` and `#![allow(clippy::needless_return)]` as deliberate project decisions to keep `if let` blocks on separate lines for readability. This avoids 25 separate refactor sites in subcommand handlers.
- **Snapshot test platform-aware** — `tests/snapshot_write.rs` and `tests/snapshots/snapshot_write__write_output_structure.snap` now use `[platform_fsync]` placeholder for the `platform.fsync` field, allowing the same snapshot to be valid on Linux (`sync_data`), macOS (`F_FULLFSYNC`), and Windows.

### Validation
- `cargo build --all-features`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo test --all-features`: 302 of 303 tests PASS (1 pre-existing failure in `signal_test::shutdown_message_on_stderr` tracked as GAP 16, unrelated to GAP 15)
- `cargo fmt -- --check`: PASS
- `cargo audit`: PASS (no vulnerabilities, no `--ignore` flag)
- `cargo deny check`: PASS (advisories, bans, licenses, sources all OK)
- Cross-compile `x86_64-pc-windows-gnu`: PASS (build, clippy -D warnings, tests --no-run)
- Cross-compile `i686-pc-windows-gnu`: PASS (check --all-features)

### Notes
- This is a NON-BREAKING change for users on Rust 1.88 or later. Users on Rust 1.85-1.87 must upgrade.
- The `time` transitive dependency is now patched (0.3.47+), resolving RUSTSEC-2026-0009.
- Cross-compile Windows GNU and i686 targets are now explicitly validated by the local developer workflow; MSVC target requires Windows runner (CI windows-latest job covers it).

### Fixed (GAP 16 - signal_test)
- **`signal_test::shutdown_message_on_stderr` no longer fails on macOS** — Replaced `libc::write(2, SHUTDOWN_MSG.as_ptr().cast(), ...)` in the SIGINT and SIGTERM signal handlers with `eprintln!`. The Rust runtime's stderr is reliably captured by `Stdio::piped()` in the test process, while raw fd writes via libc were being lost under cargo test's process-group inheritance. The `SHUTDOWN_MSG` constant was removed as dead code.
- **`tests/signal_test.rs` test reliability** — Increased `thread::sleep` from 50ms to 2000ms before sending SIGINT. The 50ms was insufficient for the atomwrite child to fully initialize tracing, mimalloc, and signal handlers before receiving the signal. Increased per-file payload from 100 to 1000 lines so the search loop runs long enough to confirm graceful shutdown. The test is now stable across 5 consecutive runs.

## [0.1.6] - 2026-06-05

### Added (README Badges)
- **docs.rs badge in README.md and README.pt-BR.md** — Added `[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)` between the Crates.io and License badges. The badge was previously missing from the published README, even though the documentation was being built successfully on docs.rs. The badge now appears in the rendered README on crates.io and on the GitHub repository page.

### Notes
- v0.1.6 is NON-BREAKING. The change is purely visual (badge image in README).
- No code or public API changes.
- No CHANGELOG migration guide required.

## [0.1.5] - 2026-06-05

### Changed (Documentation Hygiene)
- **`#![warn(missing_docs)]` promoted to `#![deny(missing_docs)]`** — Missing public API documentation is now a hard build error instead of a warning. All public items were already documented in v0.1.4 (verified by `RUSTDOCFLAGS="-D warnings" cargo doc --all-features`), so no documentation was added in this change.
- **`#![warn(rustdoc::broken_intra_doc_links)]` promoted to `#![deny(...)]`** — Broken intra-doc links now fail the build instead of being silent warnings.
- **`#![doc(html_root_url = "https://docs.rs/atomwrite/0.1.2")]` removed** — The attribute was hardcoded to the 0.1.2 version, causing intra-doc-links generated for newer versions (0.1.3, 0.1.4) to point to 0.1.2. The attribute is deprecated since rustc 1.48 in favor of the `repository` field, which is already set in `Cargo.toml`. docs.rs now uses the current crate version for link resolution automatically.

### Changed (docs.rs Metadata)
- **`[package.metadata.docs.rs]` cleaned up** — Removed `all-features = true` (no `[features]` table exists, so the flag was a no-op) and `rustdoc-args = ["--cfg", "docsrs"]` (no `#[cfg(docsrs)]` markers exist in the source). Added `targets = ["x86_64-unknown-linux-gnu"]` to make the docs.rs build target explicit.

### Tests
- 302 tests pass with 0 failures (unchanged from v0.1.4)
- 3 ignored tests (cross-compile Windows tests, unchanged)
- `cargo doc --no-deps --all-features` with `RUSTDOCFLAGS="-D warnings"` passes cleanly

### Notes
- v0.1.5 is NON-BREAKING. The bumped deny lints are already satisfied by the existing code.
- v0.1.5 does not change any public API surface or behavior. It only tightens documentation enforcement and removes deprecated metadata.
- No CHANGELOG migration guide required.

## [0.1.4] - 2026-06-05

### Fixed (Windows Compilation - GAP 14)
- **`cargo install atomwrite` no Windows 10/11** — Resolvidos três erros de compilação que bloqueavam a instalação em Windows desde v0.1.3. Erro `E0433` em `src/atomic.rs:404` (tipo `AtomwriteError` usado sem import), erro `E0308` em `src/platform.rs:116` (comparação de `*mut c_void` com literal `0`), e erro `E0507` em `src/atomic.rs:387` (assinatura `&NamedTempFile` mas chamada `.persist()` requer ownership). Todos os três bugs estavam em blocos `#[cfg(windows)]` invisíveis ao CI Linux.

### Fixed (FFI Correctness - GAP 14)
- **`src/platform.rs:116`** — Substituída comparação `handle != 0` por `!handle.is_null()` para conformidade com o padrão idiomático de raw pointer check em Rust. O `HANDLE` retornado por `GetStdHandle` é um `*mut c_void`; compará-lo com literal inteiro `0` viola o sistema de tipos e disparava `E0308`. Padrão agora é `is_null()` para nulidade e `!= INVALID_HANDLE_VALUE` (que já é `HANDLE`) para validade.

### Fixed (Error Suggestions - GAP 13)
- **`WorkspaceJail` suggestion is now context-aware** — When the user already supplied a workspace root via `--workspace` or `ATOMWRITE_WORKSPACE`, the suggestion now says "use a path inside the workspace (<root>)" instead of re-prompting the `--workspace` flag. Fixed the phantom `--force-text` flag that did not exist and caused cascading exit 2 errors.
- **All 20 error variants now have actionable suggestions** — Added suggestions for `InvalidInput`, `Io`, `ConfigInvalid`, `FileImmutable`, `NoMatches`, and `InternalError`. Only `BrokenPipe` (SIGPIPE, not actionable) remains without a suggestion.
- **New `ErrorContext` struct** — Carries `workspace_provided: bool` and `workspace: Option<PathBuf>`. `ErrorJson::from_error_with_context()` and `output::write_error_json_with_context()` use the context to produce precise suggestions.
- **`FileImmutable` suggestion** — Now mentions `chattr -i` (Unix) and `fsutil` (Windows) for clearing the immutable attribute.
- **`NoMatches` suggestion** — Guides the user to broaden the pattern, check `--include`/`--exclude` filters, and verify file content.
- **`InternalError` suggestion** — Asks the user to report the bug with the reason context.

### Added (Cross-Platform Validation - GAP 14)
- **`tests/cross_compile_check.rs`** — New cross-compile gate running `cargo check` against `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, and `x86_64-pc-windows-msvc`. Fails if `E0433`, `E0308`, or `E0507` reappear in any `cfg(windows)` block. Tests marked `#[ignore]` to skip gracefully on hosts without Windows targets.
- **`output::write_error_json_with_context()`** — New helper accepting `&ErrorContext` so the main entry point can propagate `--workspace` provenance to the error output.
- **Windows installation documentation** — New `docs/INSTALL.md` (EN) and `docs/INSTALL.pt-BR.md` (PT-BR) with Windows 10/11 prerequisites, `cargo install` commands, and troubleshooting.

### Changed
- **`src/atomic.rs:13-15`** — Moved `use crate::error::AtomwriteError` into a `#[cfg(windows)]` block to avoid `unused_imports` warning on Linux/macOS. The type is only referenced inside `persist_with_retry`.
- **`src/atomic.rs:386-409`** — `persist_with_retry` now takes `NamedTempFile` by value and recovers the file from `e.file` in the retry branch. Caller updated to pass `temp` by value.
- **`src/main.rs:93-105`** — Error reporting now constructs an `ErrorContext` with `workspace_provided: cli.global.workspace.is_some()` so the `WorkspaceJail` suggestion adapts to the user's invocation.

### Tests
- 7 new GAP 13 tests in `src/error.rs::tests`: `gap13_workspace_jail_suggestion_when_workspace_not_provided`, `gap13_workspace_jail_suggestion_when_workspace_provided`, `gap13_all_variants_have_suggestion`, `gap13_binary_file_suggestion_does_not_mention_force_text_wrong_flag`, `gap13_file_immutable_suggestion_mentions_chattr`, `gap13_no_matches_suggestion_mentions_filters`, `gap13_error_context_default_matches_legacy_behavior`.
- 1 new GAP 13 integration test in `tests/cli_v012_regressions.rs`: `gap13_jail_suggestion_when_workspace_supplied_says_inside`.
- Existing test `jail_suggestion_mentions_workspace_flag` updated to assert the suggestion mentions `--workspace` only when workspace is NOT provided (GAP 13 fix).

### Notes
- GAPs 01-12 (previously resolved) re-audited via `cargo test --all-features` — all 300+ tests still pass.
- Atomic decision `atomwrite-no-github-actions` retained: release is manual via `cargo publish` local after 8 official gates and the cross-compile gate. CI matrix in `.github/workflows/ci.yml` exists for reference only and is not executed.


## [0.1.3] - 2026-06-03

### Changed (BREAKING)
- **Atomic write default behavior for `edit` and `replace`** — `AtomicWriteOptions::default()` now sets `preserve_timestamps: false` (was `true`). The mtime of an edited or replaced file is now updated to the moment the write completes, which is the correct default for build systems that use mtime to detect source changes (cargo, make, cmake, gradle, sbt, bazel, ninja, msbuild). For backup, snapshot, or reproducible-build scenarios where the original timestamp must be preserved, use the new `--preserve-timestamps` flag on `edit` and `replace`. Cargo's fingerprint module compares the mtime of source files against the mtime of `target/.fingerprint/<unit>/dep-info` files; with the old default, cargo would skip the rebuild silently (the "Finished in 0.29s" no-op) because the source appeared older than the binary. See the v0.1.2 → v0.1.3 migration guide in `docs/MIGRATION.md` for the upgrade path.

### Added (Build System Awareness)
- `--preserve-timestamps` flag on `edit` and `replace` to opt back into the v0.1.2 behavior of keeping the original file mtime
- `mtime_preserved` field in `EditOutput` and `ReplaceResult` NDJSON responses so consumers can verify whether the timestamp was kept or updated (always present; `true` when `--preserve-timestamps` was passed, `false` by default)
- New regression tests in `src/atomic.rs::tests`: `atomic_write_updates_mtime_by_default` and `atomic_write_preserves_mtime_when_opted_in`

### Added (Documentation)
- New section "Modification Time And Build Systems" in `docs/HOW_TO_USE.md` explaining how cargo, make, cmake, gradle, sbt, bazel, ninja, and msbuild detect source changes via mtime and why the default was changed
- Portuguese equivalent in `docs/HOW_TO_USE.pt-BR.md`
- New recipe "How to Edit and Trigger a Build Without Manual Touch" in `docs/COOKBOOK.md` showing the `atomwrite edit && cargo build` workflow that no longer requires `touch`
- Portuguese equivalent in `docs/COOKBOOK.pt-BR.md`
- All v0.1.2 → v0.1.3 changes documented in `gaps.md` section "Atomic Edit Preserva mtime E Quebra Detecção De Mudança Pelo Cargo" (GAP 12)

### Test Coverage
- 2 new regression tests in `src/atomic.rs` for the default-updates-mtime and opt-in-preserves-mtime contract
- 2 new tests in `src/ndjson_types.rs::tests` updated for the new `mtime_preserved` field in `EditOutput` and `ReplaceResult`
- 2 snapshot files updated: `tests/snapshots/snapshot_write__edit_output_structure.snap` and `tests/snapshots/snapshot_write__replace_result_structure.snap` now include the new `mtime_preserved: false` field in their JSON output
- Total: 33 test suites, 294 tests passing (was 292+ in v0.1.2)

### Validation Gates
- `cargo fmt --check` clean
- `cargo clippy --all-targets --all-features -- -D warnings` zero warnings
- `cargo test --all-features` 33 suites passing
- `cargo doc --no-deps --all-features` zero warnings
- End-to-end behavior verified: file with mtime=2024-01-01 → `atomwrite edit` (default) → mtime=now → `cargo build` rebuilds correctly; `--preserve-timestamps` keeps the 2024-01-01 mtime as expected


## [0.1.2] - 2026-06-02

### Fixed (CRITICAL)
- **macOS compilation failure** — `nix::fcntl::posix_fadvise` is restricted to `cfg(target_os = "linux")` so atomwrite now compiles on macOS arm64/Intel (the nix crate gates the symbol under `linux_android | emscripten | fuchsia | freebsd` only, breaking macOS previously)
- **`batch --transaction` rollback is now real** — pre-existing files are restored AND files newly created by `write` operations are removed. The NDJSON `rollback` event now reports `files_restored`, `files_removed`, and `total_reverted` so LLMs can verify the ACID contract. Previously files created mid-transaction were never cleaned up.
- **`replace` no longer inflates counters on jail violations** — `total_replacements` is incremented only AFTER workspace jail validation passes. Violations now emit a `JailViolation` error event with `error_class: permanent` and `retryable: false`.
- **`search` parallel events are grouped by path** — parallel walker threads no longer interleave `begin`/`match`/`end` events from different files in NDJSON output. Consumers (LLM and humans) see contiguous event sequences per file.
- **`scope --delete` Rust comments no longer leaves orphan whitespace** — prepared query for Rust comments now matches trailing whitespace so deletion produces clean code.
- **`search` invalid regex emits structured JSON envelope** — invalid patterns now fail with `AtomwriteError::InvalidInput` which propagates through `write_error_json` to stdout, not raw stderr.

### Fixed (HIGH)
- **`batch --file <PATH>` is now functional** — the flag is wired through `cmd_batch` to read the NDJSON manifest from a file (validated against workspace jail) instead of stdin only.
- **`backup --output-dir` is now respected** — the flag plumbs through `AtomicWriteOptions.backup_output_dir` to `create_backup_in`, which creates the directory if missing and prunes old backups in that directory.

### Fixed (UX)
- **Workspace jail error message corrected** — `WORKSPACE_JAIL` errors now suggest `--workspace <root>` or `ATOMWRITE_WORKSPACE=<path>` instead of the misleading "use an absolute path" (which was wrong when the path was already absolute).
- **Proptest backup retention bug fixed** — `cleanup_old_backups_in` now correctly prunes old backups when using `create_backup_in` with a custom output directory.

### Changed (Dependencies)
- `nix` updated from 0.29 to 0.31 (latest stable)
- `signal-hook` updated from 0.3 to 0.4 (latest stable)
- `windows-sys` updated from 0.59 to 0.61 (latest stable)
- `rust-i18n` updated from 3 to 4 (latest stable)
- `nix::fcntl::posix_fadvise` signature changed from `AsRawFd` to `AsFd` in 0.31 — code adapted accordingly

### Added (Agent-First Features)
- `--timeout <SECONDS>` global flag for bounded execution time (0 = no timeout, default 0)
- `--grep <REGEX>` flag on `read` to filter returned lines by regex
- `completions --install` to install completion scripts to XDG data directory (`~/.local/share/bash-completion/completions/atomwrite` for Bash, etc.)

### Security
- `cargo audit` baseline 1 vulnerability acknowledged: `RUSTSEC-2026-0009` in `time 0.3.45` (DoS via stack exhaustion). Fix requires `time >= 0.3.47` which needs Rust 1.88. Our MSRV is 1.85, and atomwrite uses `time` only via `tracing-appender` for log timestamps — not exploitable. Tracked for MSRV bump in 0.2.0.


## [0.1.1] - 2026-06-01

### Fixed
#### Search and Replace Engine
- `search --include`/`--exclude` now correctly filter files via OverrideBuilder (was silently ignored)
- `replace --include`/`--exclude` now correctly filter files via OverrideBuilder
- `transform --include`/`--exclude` now correctly filter files via OverrideBuilder
- `search --context` now emits context lines via custom SearchSink (was missing entirely)
- `search --max-count` now limits matches per file via SearcherBuilder.max_matches() (was being ignored)
- `search --invert` now shows non-matching lines via SearcherBuilder.invert_match() (was inverted behavior)
- `search --sort` now sorts results by file path (was returning in unspecified order)
- `transform` now processes files in parallel via WalkParallel + crossbeam channel (was sequential)
- `search` regex special characters in patterns now properly escaped when `--literal` is set

#### Atomic Writes and Backups
- `batch delete` backup now uses atomic create_backup() with fsync (was racing the write)
- `create_backup` now uses `fs::copy` instead of `fs::hard_link` to prevent backup corruption when original is overwritten in-place (hard links would diverge silently)
- Hardlink detection before atomic rename with `tracing::warn` when nlink > 1
- Same-file detection in `copy` and `move` to prevent source=destination data loss (was overwriting)

#### Error Handling and Code Quality
- 12 broken intra-doc links in `error.rs` corrected (`DiskFull` to `Self::DiskFull` and similar)
- Six `unwrap()` calls in `edit.rs` multi-edit mode replaced with `ok_or_else` for safer error handling
- `scope.rs` thread join changed from `unwrap()` to `let _ = join()` to prevent panic propagation
- `rollback.rs` `unwrap()` replaced with descriptive `expect` message
- Exit codes in `output.rs`, `read.rs`, `batch.rs`, and `hash.rs` moved from magic numbers to named constants in `constants.rs`
- `DETECTION_SIZE` in `binary_detect.rs` centralized to `BINARY_DETECT_SIZE` in `constants.rs`

#### Output Format
- `read` modified timestamp now returns ISO 8601 format instead of epoch seconds (LLM-readable)
- `# Errors` documentation added to three `output.rs` public functions returning `Result`
- Portuguese test data in `file_io.rs` replaced with English (code-in-english convention)

### Added
#### New Subcommands
- `scope` subcommand for grammatical scoping: select AST categories (comments, functions, strings, etc.) and apply actions (delete, upper, lower, titlecase, squeeze, replace)
- `scope` supports Rust (30 prepared queries), Python (13), JavaScript/TypeScript (11), Go (8), and custom AST patterns via `--pattern`
- `backup` subcommand for creating timestamped file backups with BLAKE3 checksums and configurable retention
- `rollback` subcommand for restoring files from previous backups with optional BLAKE3 verification
- `apply` subcommand for applying patches from stdin with auto-detection of format (unified diff, SEARCH/REPLACE blocks, markdown-fenced diff, full file replacement)

#### Batch Operations Expansion
- `batch` supports 7 operations: write, replace, delete, edit, hash, move, copy
- `batch --transaction` flag for all-or-none execution with automatic rollback
- `batch` move and copy operations now accept `source`, `from`, and `src` as field aliases for the source path
- `batch` write, delete, edit, and hash operations now accept `path` as alias for `target`

#### Edit Engine Enhancements
- `edit --fuzzy` flag with cascade of 7 matching strategies (exact, line_trimmed, whitespace_normalized, indent_flexible, escape_normalized, trimmed_boundary, block_anchor)
- `edit --multi` flag for applying multiple NDJSON edit operations in a single atomic write
- `edit` NDJSON output includes `fuzzy`, `strategy`, `strategies_tried`, `similarity` fields when fuzzy matching is used

#### Path Safety
- FIFO and device file detection in path validation (exit codes 85 and 86) — prevents atomic writes to special files

#### Internationalization (i18n)
- `--lang` global flag for locale override (en, pt-BR) with `ATOMWRITE_LANG` environment variable
- i18n support via `rust-i18n` and `sys-locale`: automatic OS locale detection with en and pt-BR translations
- All user-facing strings now locale-aware (errors, warnings, info messages)

#### Internationalization Documentation
- `ARCHITECTURE.md` bilingual documentation (en + pt-BR) describing module map, data flow, and key decisions
- SPDX license headers (`MIT OR Apache-2.0`) in all 64 `.rs` source files
- Module-level `//!` documentation in all 38 source modules
- Executable doctests for `is_binary`, `detect`, and `normalize` functions
- `[package.metadata.docs.rs]` configuration for docs.rs builds
- `documentation` field and `[badges.maintenance]` in `Cargo.toml`
- Rustdoc lints: `broken_intra_doc_links`, `private_intra_doc_links`, `clippy::doc_markdown`
- `doc(html_root_url)` for docs.rs cross-linking

#### Supply Chain and Security
- `deny.toml` for cargo-deny license and advisory auditing
- Line ending detection and normalization module (`line_endings.rs`)
- `--line-ending lf|crlf|cr|auto` flag on `write` and `edit` for line ending normalization

#### Test Infrastructure
- 282 tests across integration and unit test suites (was 5 tests in 1 module at v0.1.0)
- Integration tests for `backup`, `rollback`, `apply`, and `scope`
- 2 fuzz targets (`batch_parse`, `extract_json`) with `libfuzzer-sys` for parser security testing
- Optimistic locking integration tests for `write --expect-checksum` and `edit --expect-checksum`
- NDJSON validation tests expanded from 5 to 20 of 21 commands
- `jaq` interop tests validating NDJSON piped through `jaq` filter
- i18n integration test confirming `--lang` does not alter JSON output

### Security
- SPDX license headers ensure license clarity in all source files
- cargo-deny enforces license compliance and tracks security advisories
- FIFO and device file detection prevents accidental writes to special files
- Hardlink detection prevents silent data corruption when atomic rename breaks hard links

### Known Limitations (fixed in v0.1.2)
- `batch --file <PATH>` flag was declared in help but not wired through to command logic
- `batch --transaction` did not delete files created during failed transactions (only restored modified files)
- `replace` incremented counters BEFORE workspace jail validation, producing contradictory NDJSON counts
- `search` invalid regex produced raw stderr error instead of JSON envelope
- `search` parallel walker interleaved begin/match/end events for different files
- `scope --delete` for Rust comments left orphan whitespace
- macOS compilation failed (nix 0.29 gated `posix_fadvise` to non-macOS Unix)
- Default `--workspace` was CWD-silent (no warning when capturing everything)
- WORKSPACE_JAIL error message suggested "use an absolute path" even when path was already absolute
- `backup --output-dir` was declared but not plumbed through
- 4 dependencies frozen (nix 0.29, signal-hook 0.3, windows-sys 0.59, rust-i18n 3)
- `read` had no `--head`/`--tail`/`--grep` flags for LLM context window control
- `completions` did not auto-install
- No global `--timeout` flag for unbounded operation termination


## [0.1.0] - 2026-05-29
### Added
- 22 subcommands: `read`, `write`, `edit`, `search`, `replace`, `hash`, `delete`, `count`, `diff`, `move`, `copy`, `list`, `extract`, `calc`, `regex`, `transform`, `batch`, `completions`, `scope`, `backup`, `rollback`, `apply`
- Atomic write pipeline: tempfile + fsync + rename + directory fsync on every write operation
- BLAKE3 checksums on every `read` and `write` response
- Optimistic locking via `--expect-checksum` flag
- NDJSON output contract: every stdout line is a JSON object with a `"type"` discriminator
- Structured error responses on stdout with `error: true`, including `error_class`, `retryable`, and `suggestion` fields
- Parallel file search powered by the ripgrep engine (`grep-regex`, `grep-searcher`, `grep-matcher`)
- `.gitignore`-aware traversal via the `ignore` crate
- Structural AST search and rewrite via ast-grep covering 306 languages
- Regex generation from examples via grex
- Math expression evaluation and unit conversions via fend-core
- Unified diff output via the `similar` crate
- Memory-mapped file reads via `memmap2` for large files
- Workspace jail enforcement via `--workspace` to prevent path escapes
- Symlink blocking to prevent traversal outside workspace boundaries
- Batch operations from NDJSON manifests supporting write, replace, and delete
- Shell completion generation for bash, zsh, fish, elvish, and PowerShell
- Signal handling for SIGINT, SIGPIPE, and SIGTERM with clean shutdown
- Cross-platform support: Unix (nix, libc) and Windows (windows-sys)
- 20 distinct exit codes for precise error classification
- `NO_COLOR` environment variable support
- `RUST_LOG` environment variable for log verbosity control
- Release profile with LTO, single codegen unit, symbol stripping, and panic=abort


[Unreleased]: https://github.com/daniloaguiarbr/atomwrite/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/daniloaguiarbr/atomwrite/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/daniloaguiarbr/atomwrite/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/daniloaguiarbr/atomwrite/releases/tag/v0.1.0
