[Leia em Portugues](CHANGELOG.pt-BR.md)


# Changelog

- All notable changes to this project are documented in this file
- Format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/)
- Versioning follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)


## [Unreleased]


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
