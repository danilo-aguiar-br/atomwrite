[Leia em Portugues](CHANGELOG.pt-BR.md)


# Changelog

- All notable changes to this project are documented in this file
- Format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/)
- Versioning follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)


## [Unreleased]

### Fixed
- `search --include`/`--exclude` now correctly filter files via OverrideBuilder (was silently ignored)
- `replace --include`/`--exclude` now correctly filter files via OverrideBuilder
- `transform --include`/`--exclude` now correctly filter files via OverrideBuilder
- `search --context` now emits context lines via custom SearchSink
- `search --max-count` now limits matches per file via SearcherBuilder.max_matches()
- `search --invert` now shows non-matching lines via SearcherBuilder.invert_match()
- `search --sort` now sorts results by file path
- `transform` now processes files in parallel via WalkParallel + crossbeam channel
- `read` modified timestamp now returns ISO 8601 format instead of epoch seconds
- `batch delete` backup now uses atomic create_backup() with fsync
- `create_backup` now uses `fs::copy` instead of `fs::hard_link` to prevent backup corruption when original is overwritten in-place
- Exit codes in `output.rs`, `read.rs`, `batch.rs`, and `hash.rs` moved from magic numbers to named constants in `constants.rs`
- `DETECTION_SIZE` in `binary_detect.rs` centralized to `BINARY_DETECT_SIZE` in `constants.rs`
- Six `unwrap()` calls in `edit.rs` multi-edit mode replaced with `ok_or_else` for safer error handling
- `scope.rs` thread join changed from `unwrap()` to `let _ = join()` to prevent panic propagation
- `rollback.rs` `unwrap()` replaced with descriptive `expect` message
- `# Errors` documentation added to three `output.rs` public functions returning `Result`
- Portuguese test data in `file_io.rs` replaced with English

### Added
- `batch` supports 7 operations: write, replace, delete, edit, hash, move, copy
- `batch --transaction` flag for all-or-nothing execution with automatic rollback
- `edit --fuzzy` flag with cascade of 7 matching strategies (exact, line_trimmed, whitespace_normalized, indent_flexible, escape_normalized, trimmed_boundary, block_anchor)
- `edit --multi` flag for applying multiple NDJSON edit operations in a single atomic write
- `edit` NDJSON output includes fuzzy, strategy, strategies_tried, similarity fields when fuzzy matching is used
- `scope` subcommand for grammatical scoping: select AST categories (comments, functions, strings, etc.) and apply actions (delete, upper, lower, titlecase, squeeze, replace)
- `scope` supports Rust (30 prepared queries), Python (13), JavaScript/TypeScript (11), Go (8), and custom AST patterns via `--pattern`
- `backup` subcommand for creating timestamped file backups with BLAKE3 checksums and configurable retention
- `rollback` subcommand for restoring files from previous backups with optional BLAKE3 verification
- `apply` subcommand for applying patches from stdin with auto-detection of format (unified diff, SEARCH/REPLACE blocks, markdown-fenced diff, full file replacement)
- `--line-ending lf|crlf|cr|auto` flag on `write` and `edit` for line ending normalization
- FIFO and device file detection in path validation (exit codes 85 and 86)
- Hardlink detection before atomic rename with `tracing::warn` when nlink > 1
- Same-file detection in `copy` and `move` to prevent source=destination data loss
- Line ending detection and normalization module (`line_endings.rs`)
- 60 unit tests across 8 modules (was 5 tests in 1 module)
- Integration tests for `backup`, `rollback`, `apply`, and `scope`
- `deny.toml` for cargo-deny license and advisory auditing
- `--lang` global flag for locale override (en, pt-BR) with `ATOMWRITE_LANG` environment variable
- i18n support via `rust-i18n` and `sys-locale`: automatic OS locale detection with en and pt-BR translations
- SPDX license headers (`MIT OR Apache-2.0`) in all 64 `.rs` source files
- Module-level `//!` documentation in all 38 source modules
- Executable doctests for `is_binary`, `detect`, and `normalize` functions
- `ARCHITECTURE.md` bilingual documentation (en + pt-BR) describing module map, data flow, and key decisions
- `[package.metadata.docs.rs]` configuration for docs.rs builds
- `documentation` field and `[badges.maintenance]` in `Cargo.toml`
- Rustdoc lints: `broken_intra_doc_links`, `private_intra_doc_links`, `clippy::doc_markdown`
- `doc(html_root_url)` for docs.rs cross-linking


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


[Unreleased]: https://github.com/comandoaguiar/atomwrite/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/comandoaguiar/atomwrite/releases/tag/v0.1.0
