# atomwrite Migration Guide


[Leia em Portugues](MIGRATION.pt-BR.md)


## What's New in v0.1.12

This section summarizes the migration-relevant changes in v0.1.12. See the [v0.1.11 to v0.1.12](#v0111-to-v0112) section below for the v0.1.12 migration guide and the [v0.1.12 to v0.1.15 (Current)](#v0112-to-v0115-current) section for the latest transition.

### New Subcommands (6)

- `set` -- write a value at a dotted path in TOML/JSON. Use instead of rewriting entire config files.
- `get` -- read a value at a dotted path. Use instead of reading entire config files.
- `del` -- remove a key at a dotted path. `--force-missing` for idempotent scripts.
- `case` -- rename identifiers in 5 case styles via `heck`.
- `query` -- walk a tree-sitter AST. 305 languages.
- `outline` -- extract top-level definitions. 305 languages.

All 6 subcommands are additive. No existing code is affected.

### New Flags (15 Total)

- `write --syntax-check` (G72)
- `write --include-fifo` (G56)
- `write --strict-atomic` (G90)
- `write --lock` and `--lock-timeout` (G54)
- `read --format raw` and `--raw` (G81)
- `read --head N`, `--tail N`, `--line N`, `--grep <REGEX>`
- `search --max-filesize` and `--max-columns` (G68)
- `replace --literal` and `-F` (G66)
- `transform --rules` and `--inline-rules` (G44)
- `batch --batch-size` (G77)
- `backup/copy --no-reflink` (G64)
- `diff --diff-algorithm patience|myers|lcs` (G76)

All flags are additive with default values that preserve v0.1.11 behavior.

### New Error Codes (5)

- 83 `LockTimeout` (G54)
- 88 `SyntaxError` (G72)
- 91 `ExdevFallbackDisabled` (G90)
- 92 `CopyBackBlake3Failed` (G114)
- 93 `OrphanJournal` (G114)

Total error codes: 25 (was 20 in v0.1.4). All new codes have bilingual messages and actionable suggestions.

### Dependencies Added

- `tree-sitter-language-pack = "1.8"` -- 305 languages, dynamic loading
- `toml_edit` -- preserves TOML formatting
- `heck = "0.5"` -- case conversion
- `reflink-copy = "0.1"` -- CoW backup
- `content_inspector = "0.2"` -- UTF-16 detection
- `xattr = "1"` -- extended attributes

All additive. No existing dependency removed.

### Behavior Changes

- None. v0.1.12 is fully backward-compatible with v0.1.11.
- New subcommands are opt-in: existing scripts keep working.
- Default values for new flags preserve v0.1.11 behavior.
- Error code additions do not change existing exit codes.

### Migration Action

- Update version pin: `cargo install atomwrite --locked --version "^0.1.12"`
- New subcommands and flags are opt-in. No code changes required for existing callers.
- See the [v0.1.12 to v0.1.15 (Current)](#v0112-to-v0115-current) section for the latest migration steps.

### Test Coverage

- 542 tests passing (445 in v0.1.12 + 2 in v0.1.14 + 8 G117 + 6 G118 in v0.1.15 + 40 v0.1.16-v0.1.18 cross-platform + 21 v0.1.19 + 20 v0.1.20)
- 9 ADRs in `docs/decisions/` (0019-0027)
- 7 new JSON schemas in `docs/schemas/`
- See [docs/decisions/README.md](README.md) for architectural decisions

## Current Version
- atomwrite is at v0.1.25
- This document covers migration from v0.1.0 through v0.1.25
- See the sections below for additive changes and breaking changes in each version


## v0.1.24 to v0.1.25 (2026-06-22)

### Additive (Non-Breaking)
- `.atomwrite.toml` configuration file with hierarchy: CLI > env > local > XDG global > defaults
- New `verify` subcommand (delegates to `hash --verify`)
- Jaro-Winkler fuzzy matching strategy for short strings in edit
- `edit --fuzzy-threshold <FLOAT>` for configurable match sensitivity
- `scope` actions `symbols` and `normalize` (NFC)
- `delete --older-than` with human-readable duration
- `delete --confirm` as preview mode
- `replace --preserve-case` with case adaptation
- `search --pcre2` flag (returns exit 65 when feature not enabled)
- `transform --verify-parse` re-validates output via tree-sitter
- `scope --query comments` now captures block comments in Rust
- `copy --preserve` now copies source permissions and timestamps
- `outline --positions` now emits byte offsets and column positions
- Property-based fuzzy tests via proptest

### Bug Fixes (May Affect Behavior)
- `write --backup` no longer reports phantom `backup_path` (critical data integrity fix)
- `set` rejects descent into scalar TOML values (was silently misrouting)
- `case` without `--subvert` now exits 65 (was silent no-op)
- I/O errors via anyhow now emit NDJSON envelopes (7 subcommands affected)
- `hash` output field renamed from `value` to `checksum`
- `batch` move/copy now requires `"force":true` to overwrite existing targets
- `get`/`del` on missing key returns INVALID_INPUT (exit 65) instead of FILE_NOT_FOUND (exit 4)
- `list --long` modified field now emits ISO 8601 format
- `size_delta_pct` in risk_assessment changed from u8 to u32
- Risk telemetry default changed to disabled (255)

### Migration Action
- Update version pin: `cargo install atomwrite --locked --version "^0.1.25"`
- If you parse `hash` output: field `value` is now `checksum`
- If you use `case` without `--subvert`: now exits 65 (add `--subvert` pairs explicitly)
- If you parse `get`/`del` exit codes for missing keys: exit changed from 4 to 65
- If you parse `batch` move/copy: existing targets now require `"force":true`
- MSRV unchanged at Rust 1.88

### Test Coverage
- 631 tests passing (621 in v0.1.24 + 10 new for v0.1.25)
- ~505 e2e scenarios across 6 audit rounds
- See `gaps.md` for the full audit of 64 gaps


## v0.1.23 to v0.1.24 (2026-06-21)

### Behavioral Changes (Breaking)

- Backup timestamp format: `YYYYMMDD_HHMMSS` changed to `YYYYMMDD_HHMMSS_mmm` (millisecond suffix)
- `rollback --timestamp` now accepts PREFIX match (e.g. `--timestamp 20260621_120000` matches `file.bak.20260621_120000_042`)
- `prune-backups --max-count` now sorts by FILENAME (lexicographic) instead of mtime
- `replace` rejects empty pattern with exit 65 (previously destroyed files silently)
- `get` of non-existent key returns exit 4 (was exit 0 with `found: false`)
- `hash` of non-existent file returns exit 4 (was exit 0 silently)
- `regex` removed `allow_hyphen_values` on examples field; use POSIX `--` for hyphen-starting examples

### Error Handling (Breaking for parsers)

- 20 `anyhow::bail!()` calls converted to typed `AtomwriteError` variants
- ALL user-facing errors now emit structured JSON on stdout with correct exit codes
- Exit codes changed: many paths that returned exit 1 now return exit 4 (NotFound) or exit 65 (InvalidInput)
- Commands affected: `set`, `del`, `get`, `query`, `outline`, `edit`, `batch`, `write`, `prune-backups`, `extract`

### Bug Fixes (May Affect Existing Behavior)

- `delete --recursive` NOW WORKS (was a no-op for directories)
- `hash --recursive` NOW WORKS (was accepted but never walked)
- `search --multiline` NOW WORKS (was not propagated to SearcherBuilder)
- `scope --query comments --delete` now removes entire line_comment nodes (was removing only `//`)
- `scope`/`count`/`transform` now resolve walk roots against `--workspace` (was using CWD)
- `diff` now resolves paths against `--workspace`
- `batch --transaction` now reverts `move`/`copy` operations on rollback
- `case --subvert` changed from greedy `num_args=2..` to exact `num_args=2`
- `get` JSON/TOML values no longer doubly-quoted
- `set`/`del` `old_value`/`removed_value` no longer doubly-quoted

### Additive (Non-Breaking)

- `write` risk_assessment skipped for `--append`/`--prepend`
- `scope` summary emits `files_modified: null` in read-only mode
- `wal-stats`/`wal-heal` NDJSON output now includes `type` field
- `hash --stdin` no longer requires PATHS argument
- `edit --multi` field `op` is now optional (inferred as `"exact"`)
- `read --format raw` skips binary heuristic
- `read --line/--lines` no longer panics on out-of-range indices
- `read --lines/--head/--tail` empty range returns empty content instead of `"\n"`
- `set` TOML nested keys now correctly return `old_value`
- `regex` emits warning when examples look like flags
- `--require-backup` now detects `--no-backup` override
- ARGUMENT_PARSE_ERROR (exit 2) gains context-aware `suggestion` field

### ADRs Added

- ADR-0045: Actionable suggestion for clap parse errors
- ADR-0046: diff resolve-first retrofit
- ADR-0047: scope read-only mode fix

### Migration Action

- Update version pin: `cargo install atomwrite --locked --version "^0.1.24"`
- If you parse exit codes: update handlers for paths that changed from exit 1 to exit 4 or exit 65
- If you use `rollback --timestamp`: old format still works (prefix match is backward-compatible)
- If you use `prune-backups --max-count`: results may differ (lexicographic vs mtime ordering)
- If you match backup filenames with regex: update to accept optional `_\d{3}` suffix
- If you pass empty pattern to `replace`: this is now rejected (was a silent data-destruction bug)
- MSRV unchanged at Rust 1.88

### Test Coverage

- 621 tests passing (609 in v0.1.23 + 12 new for v0.1.24)
- 3 ADRs added (0045-0047)
- See `gaps.md` for the full audit of 52 resolved issues


## v0.1.18 to v0.1.20 (2026-06-15)

### Additive (G117)

- `edit` multi-pair `--old`/`--new` runs the full 9-strategy fuzzy cascade per pair (was exact-only).
- The success envelope gains `pairs_total` and `pair_results`; the error envelope gains `failed_pair_index`, `pairs_total`, `pair_results` (still `INVALID_INPUT`, exit 65, file untouched).
- New opt-in `edit --partial` applies the matching pairs and reports the rest; zero matches maps to `NO_MATCHES` (exit 1) with no write.

### Behavioral Fix (G118)

- `write` resolves the target via the workspace jail BEFORE append/prepend, line-ending auto-detection, and `--expect-checksum`.
- With a CWD outside the workspace: `--append`/`--prepend` no longer truncate, `--line-ending auto` detects the existing file again, and a divergent `--expect-checksum` exits 82 (`STATE_DRIFT`) instead of silently overwriting. Out-of-jail targets exit 126 early.
- Breaking only for callers that depended on the buggy silent overwrite.

### Migration Action (v0.1.15)

- Update version pin: `cargo install atomwrite --locked --version "^0.1.15"`
- MSRV unchanged at Rust 1.88. No code changes required for compliant callers.
- See ADR-0026 and ADR-0027 in `docs/decisions/` for the full rationale.

## v0.1.3 to v0.1.4 (Historical)

### v0.1.4 (Historical)

#### Fixed (Windows Compilation - GAP 14)

Three compilation errors in `#[cfg(windows)]` blocks prevented `cargo install atomwrite` from succeeding on Windows 10/11 since v0.1.3:

- `E0433` in `src/atomic.rs:404` — `persist_with_retry` used `AtomwriteError::PermissionDenied` without importing it. The `use crate::error::AtomwriteError;` is now gated under `#[cfg(windows)]` to avoid `unused_imports` on Linux/macOS.
- `E0507` in `src/atomic.rs:387` — `persist_with_retry` took `&NamedTempFile` but called `temp.persist()` which requires ownership. Signature changed to `fn persist_with_retry(mut temp: NamedTempFile, target: &Path) -> Result<()>`. The retry branch now recovers the file from `e.file` (PersistError exposes the original NamedTempFile on failure).
- `E0308` in `src/platform.rs:116` — `GetStdHandle` returns `HANDLE` which is `*mut c_void` in windows-sys 0.61. The literal `0` is a `usize`; comparing a raw pointer to an integer is a type error. Replaced `handle != 0` with `!handle.is_null()`. The `handle != INVALID_HANDLE_VALUE` comparison is unchanged because `INVALID_HANDLE_VALUE` is already typed as `HANDLE` (`-1i32 as _`).

Migration impact:
- No API or behavior change for end users on Linux or macOS
- Windows users: `cargo install atomwrite` now succeeds; no need to apply manual patches or compile from source
- All atomic write semantics, exit codes, NDJSON output, and CLI flags are unchanged

#### Fixed (Error Suggestions - GAP 13)

Error suggestions are now context-aware and actionable:

- `WorkspaceJail` suggestion adapts: when the user has supplied `--workspace` (or `ATOMWRITE_WORKSPACE`), the suggestion now says "use a path inside the workspace (<root>)" instead of re-prompting the flag.
- All 20 error variants now carry `suggestion` text. Previously 6 variants (InvalidInput, Io, ConfigInvalid, FileImmutable, NoMatches, InternalError) returned `None`. Only `BrokenPipe` (SIGPIPE, not actionable) remains without a suggestion.
- Phantom `--force-text` flag reference removed from BinaryFile suggestion.
- New `ErrorContext` struct (`workspace_provided`, `workspace`) and `ErrorJson::from_error_with_context()` API. The legacy `from_error()` is preserved.

New suggestions:
- `FileImmutable` — mentions `chattr -i` (Unix) and `fsutil` (Windows) to clear the immutable attribute
- `NoMatches` — guides the user to broaden the pattern and review `--include`/`--exclude` filters
- `InternalError` — requests a bug report with the reason context
- `InvalidInput` — asks the user to review the input and check arguments
- `Io` — points to the underlying I/O error message
- `ConfigInvalid` — points to the configuration reason

Migration impact:
- No API breakage: `ErrorJson::from_error()` still works with the same output
- If you parse the `suggestion` field of error envelopes, the text may now differ for the affected variants. The semantics (actionable hint) are preserved or improved.

#### Added (Cross-Platform Validation - GAP 14)

- `tests/cross_compile_check.rs` — 3 gated cross-compile tests for `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, and `x86_64-pc-windows-msvc`. Fails on any regression of `E0433`, `E0308`, or `E0507` in `cfg(windows)` blocks. Run with `cargo test --test cross_compile_check -- --ignored` before releases that touch Windows-only code.
- `output::write_error_json_with_context()` — propagates `ErrorContext` from the CLI parser to the NDJSON output.
- `docs/INSTALL.md` and `docs/INSTALL.pt-BR.md` — Windows 10/11 installation prerequisites, `cargo install` commands, and troubleshooting.

#### Reference

See `gaps.md` sections "GAP 13" and "GAP 14" for the full root-cause analysis and design rationale.


## v0.1.2 to v0.1.3

### v0.1.3 (Previous)

#### Changed (BREAKING)

##### Atomic write default no longer preserves mtime

The `edit` and `replace` subcommands now update the file modification time (mtime) to the moment the write completes, instead of preserving the original mtime. This is the correct default for build systems that use mtime to detect source changes.

Before v0.1.3:
- `edit` and `replace` hardcoded `AtomicWriteOptions::preserve_timestamps = true`
- The mtime of the file was restored to the value it had BEFORE the atomic rename
- Build systems that compare source mtime to dep-info mtime (cargo, make, cmake) would skip the rebuild silently when the source appeared older than the binary
- Workaround: agents had to run `touch <file>` after `atomwrite edit` to force cargo to detect the change

After v0.1.3:
- `edit` and `replace` use `AtomicWriteOptions::preserve_timestamps = false` by default
- The mtime is set to "now" automatically, so cargo detects the change without manual intervention
- Agents no longer need `touch` after editing a Rust source file before `cargo build`
- For backup, snapshot, or reproducible-build workflows that need the original timestamp, pass the new `--preserve-timestamps` flag

Affected consumers (LLM agents):
- If you build code after editing with atomwrite, the new default fixes a silent "Finished in 0.29s" no-op where cargo skips the rebuild
- If you depend on the old mtime-preservation behavior, add `--preserve-timestamps` to your `edit` and `replace` invocations

Diagnostic field:
- `edit` and `replace` NDJSON output now include `mtime_preserved: bool` so you can verify which path was taken (true = timestamp kept, false = timestamp updated)

#### Added (Build System Awareness)

- `--preserve-timestamps` flag on `edit` and `replace` to opt back into the v0.1.2 mtime-preservation behavior
- `mtime_preserved` field in `EditOutput` and `ReplaceResult` NDJSON responses

#### Reference

See `gaps.md` section "Atomic Edit Preserva mtime E Quebra Detecção De Mudança Pelo Cargo" (GAP 12) for the full root-cause analysis and design rationale.


## v0.1.1 to v0.1.2

### v0.1.2

#### Fixed (Bug Fixes)

##### `batch --transaction` rollback is now real
Previously, files created by `write` operations during a transaction were never removed on rollback. Now:
- `RollbackEvent` includes `files_restored`, `files_removed`, and `total_reverted`
- New files created mid-transaction are deleted on rollback
- Pre-existing modified files are restored from backup

Affected consumers (LLM agents): trust the NDJSON `rollback` event — disk state matches it.

##### `replace` no longer inflates counters on jail violations
Previously, `total_replacements` was incremented for files outside the workspace jail. Now:
- Jail validation runs BEFORE counter increment
- Violations emit `ReplaceErrorEvent` with `kind: jail_violation`, `error_class: permanent`, `retryable: false`
- `total_replacements` reflects only in-jail matches

##### `search` parallel events are now grouped by path
The parallel walker no longer interleaves `begin`/`match`/`end` events from different files. Event sequences for a given path are now contiguous in NDJSON output.

##### `scope --delete` Rust comments no longer leaves orphan whitespace
The `comments` prepared query for Rust now matches trailing whitespace so deletion produces clean code.

##### `search` invalid regex emits structured JSON envelope
Invalid patterns now fail with `AtomwriteError::InvalidInput` which is serialized as `error.json` on stdout, not raw stderr.

##### `batch --file <PATH>` is now functional
The flag now actually reads the NDJSON manifest from a file (validated against workspace jail) instead of being ignored.

##### `backup --output-dir <DIR>` is now respected
The flag now places backups in the custom directory (created if missing) and prunes old backups in that directory.

##### WORKSPACE_JAIL error message corrected
The misleading "use an absolute path" suggestion is now "set --workspace <root> or export ATOMWRITE_WORKSPACE=<path>".

#### Added (Agent-First Features)

- `--timeout <SECONDS>` global flag for bounded execution (0 = no timeout, default 0)
- `read --grep <REGEX>` filter to return only lines matching a regex
- `completions --install` to install completion scripts to XDG data directory

#### Changed (Dependencies)

- `nix` 0.29 → 0.31
- `signal-hook` 0.3 → 0.4
- `windows-sys` 0.59 → 0.61
- `rust-i18n` 3 → 4

#### Cross-Platform

atomwrite v0.1.2 now compiles on macOS arm64 (Apple Silicon) and macOS x86_64. The `posix_fadvise` syscall is now correctly gated to `target_os = "linux"` only.


## What Changes
### SemVer Commitment
- atomwrite follows Semantic Versioning 2.0.0
- MAJOR version: breaking changes to CLI flags, exit codes or JSON output schema
- MINOR version: new subcommands, new flags, new JSON fields (additive only)
- PATCH version: bug fixes with no API changes

### What Counts as Breaking
- Removing or renaming a CLI flag
- Changing the meaning of an exit code
- Removing a field from JSON output
- Changing the type of an existing JSON field
- Renaming a JSON field
- Changing the default behavior of an existing flag

### What Does NOT Break
- Adding a new subcommand
- Adding a new optional flag
- Adding a new field to JSON output
- Adding a new exit code
- Improving error messages
- Performance improvements

### Planned Stabilizations for 1.0
- NDJSON output schemas for all 30 subcommands
- Exit code assignments
- Error code strings (`FILE_NOT_FOUND`, `STATE_DRIFT`, etc)
- Global flag names and behavior
- Batch manifest format

### Potential Breaking Changes Before 1.0
- Field names in NDJSON output may change before 1.0
- New required fields may be added to output types
- Exit code values may shift to align with sysexits
- The `--json-schema` output format may evolve


## Step-by-Step Migration Template
- Use this template when migrating between versions

### Step 1 -- Read the Changelog
- Review `CHANGELOG.md` for the target version
- Identify all entries marked BREAKING

### Step 2 -- Check Your Commands
- List every atomwrite invocation in your agent or scripts
- Compare each flag against the migration notes

### Step 3 -- Compare JSON Schemas
- Run `atomwrite <subcommand> --json-schema` with both versions
- Identify field additions, removals and type changes

### Step 4 -- Update JSON Parsing
- Update your `jaq` filters or JSON parsing code
- Handle new fields gracefully (additive changes)
- Remove references to deleted fields

### Step 5 -- Update Exit Code Handling
- Review any `case` or `if` blocks that handle exit codes
- Add handling for new exit codes
- Remove handling for deprecated exit codes

### Step 6 -- Test in Dry-Run Mode
- Run every modified invocation with `--dry-run` first
- Verify output structure matches expectations

### Step 7 -- Deploy
- Update the binary via `cargo install atomwrite`
- Run your test suite
- Verify agent behavior in a staging environment


## JSON Schema Changes Template
- Use this format to document field changes between versions

### Before (vX.Y.Z)

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc..."}
```

### After (vX.Y.Z)

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc...","new_field":"value"}
```

### Migration Action
- New field `new_field` is additive and OPTIONAL
- No action required for existing consumers
- Update parsing to consume the new field if useful


## v0.1.0 to v0.1.1
### Summary
- ZERO breaking changes
- All v0.1.0 commands, flags and JSON output remain unchanged
- No migration action required for existing consumers

### Fixed Behaviors (silent failures corrected)
- `search --include` and `search --exclude` now actually filter files (was silently ignored)
- `replace --include` and `replace --exclude` now actually filter files
- `transform --include` and `transform --exclude` now actually filter files
- `search --context N` now emits context lines around matches
- `search --max-count N` now limits matches per file
- `search --invert` now shows non-matching lines (was inverted)
- `search --sort path` now sorts results by file path
- `transform` now processes files in parallel (was sequential)
- `read` modified timestamp now returns ISO 8601 instead of epoch seconds
- `batch delete` backup now uses atomic create_backup() with fsync (was racing the write)
- `create_backup` now uses `fs::copy` instead of `fs::hard_link` (hard links would diverge silently)
- 12 broken intra-doc links in `error.rs` corrected
- Magic exit code numbers replaced with named constants in `constants.rs`
- Six `unwrap()` calls in `edit.rs` multi-edit mode replaced with `ok_or_else`
- `scope.rs` thread join no longer panics on failure

### Additive Changes
#### New Subcommands
- `scope` subcommand for grammatical scoping with AST-based actions (delete, upper, lower, titlecase, squeeze, replace)
- `scope` supports Rust (30 prepared queries), Python (13), JavaScript/TypeScript (11), Go (8)
- `backup` subcommand for timestamped backups with BLAKE3 checksums and configurable retention
- `rollback` subcommand for restoring from backups with optional BLAKE3 verification
- `apply` subcommand for patch application with auto-format detection (unified diff, SEARCH/REPLACE, markdown-fenced, full file)

#### New Flags
- `batch --transaction` flag for all-or-nothing execution with rollback
- `edit --fuzzy` flag with 7-strategy cascade for approximate text matching
- `edit --multi` flag for multiple NDJSON edits in one atomic write
- `--line-ending lf|crlf|cr|auto` flag on `write` and `edit`
- `--lang <LOCALE>` global flag for locale override (en, pt-BR)
- `batch` move and copy accept `source`, `from`, `src` as field aliases
- `batch` write, delete, edit, hash accept `path` as alias for `target`

#### Internationalization
- i18n support via `rust-i18n` with automatic OS locale detection
- All user-facing strings now locale-aware (errors, warnings, info messages)
- Bilingual documentation (en + pt-BR) for all major docs

#### Security
- FIFO and device file detection in path validation (exit codes 85 and 86)
- Hardlink detection before atomic rename with `tracing::warn` when nlink > 1
- Same-file detection in `copy` and `move` to prevent source=destination data loss
- SPDX license headers in all 64 `.rs` source files
- `deny.toml` for cargo-deny license and advisory auditing

#### Test Infrastructure
- 282 tests (was 5 in v0.1.0)
- Integration tests for `backup`, `rollback`, `apply`, and `scope`
- 2 fuzz targets (`batch_parse`, `extract_json`) with `libfuzzer-sys`
- Optimistic locking integration tests
- NDJSON validation tests expanded from 5 to 20 of 21 commands
- `jaq` interop tests validating NDJSON piped through filter
- i18n integration test

### JSON Output Changes
- `edit` output includes new optional fields: `fuzzy`, `strategy`, `strategies_tried`, `similarity` (only when fuzzy matching used)
- `read` timestamp changed from epoch seconds to ISO 8601 format (breaking for consumers reading `modified` as number)
- New output types added for `scope`, `backup`, `rollback`, `apply`
- All existing fields remain unchanged

### JSON Schema Changes Example

```json
// Before (v0.1.0)
{"type":"read","path":"/abs/file","content":"...","modified":1704067200}

// After (v0.1.1)
{"type":"read","path":"/abs/file","content":"...","modified":"2024-01-01T00:00:00Z"}
```

### Known Limitations Fixed in v0.1.2
- `batch --file <PATH>` flag was declared but not wired (now reads manifest from file)
- `batch --transaction` did not delete files created mid-transaction
- `replace` inflated counters on jail violations
- `search` parallel walker interleaved events from different files
- `search` invalid regex produced raw stderr instead of JSON envelope
- `scope --delete` for Rust comments left orphan whitespace
- macOS compilation failed (nix 0.29 gated `posix_fadvise` to non-macOS Unix)
- `backup --output-dir` was declared but not plumbed through
- No `--timeout`, `--grep`, `completions --install` flags

### Migration Action
- No action required for v0.1.0 to v0.1.1
- Existing `jaq` filters and JSON parsing code continue to work for all fields except `read.modified` (epoch → ISO 8601)
- Update consumers that read `read.modified` as a numeric value
- New fields are additive and safe to ignore
- Recommended: upgrade to v0.1.2 next, which fixes 14 issues introduced in v0.1.1


## v0.1.11 to v0.1.12
### v0.1.12

The v0.1.12 release closes 13 of the Top 20 gaps from the PRD v5-v16 audit (`gaps.md`). It is additive: all v0.1.11 behavior is preserved.

#### Added (New Subcommands -- v14 Tier 3)
- `set <PATH> <KEY_PATH> <VALUE>` -- write a value at a dotted path in a TOML or JSON file, preserving comments and key order via `toml_edit`.
- `get <PATH> <KEY_PATH>` -- read a value at a dotted path. NDJSON: `{"type":"get","key_path","value","found","format"}`.
- `del <PATH> <KEY_PATH>` -- remove a key. `--force-missing` flag treats missing keys as a no-op success.
- `case <PATHS...> --subvert OLD NEW --to <style>` -- rename identifiers via `heck`.
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` -- walk a tree-sitter AST.
- `outline <PATH> [--kind <KIND>] [--positions]` -- extract high-level structure.

#### Added (G72 REAL syntax check)
- `atomwrite write --syntax-check` invokes tree-sitter (24 languages). Exit 88.

#### Added (G114 WAL sidecar)
- `.atomwrite.journal.<target>.atomwrite.journal.json` with `Started`/`Committed`.
- `recover_orphan_journals(dir)` is consultive.

#### Added (Other Gaps)
- **G54 `--lock` and `--lock-timeout <ms>`** -- advisory flock. Exit 83.
- **G39 xattr** -- macOS quarantine, Linux SELinux, POSIX capabilities preserved.
- **G41 content_inspector** -- UTF-8, UTF-16LE, UTF-16BE, Binary correctly detected.
- **G64 reflink CoW** -- `reflink_or_copy` in APFS/btrfs/XFS.
- **G90 EXDEV fallback** -- copy fallback for Docker/NFS. `--strict-atomic` to opt out (exit 91).
- **G44 transform multi-rule** -- `--rules <file.yaml>` and `--inline-rules <YAML>`.
- **G68 `--max-filesize` and `--max-columns`** -- search skip/truncate.
- **G80 SIGPIPE** -- broken pipe → exit 0.
- **G81 `--format raw` and `--raw`** -- read emits raw bytes for Unix composability.

#### Added (5 New Error Codes)
- 83 `LockTimeout`
- 88 `SyntaxError`
- 91 `ExdevFallbackDisabled`
- 92 `CopyBackBlake3Failed`
- 93 `OrphanJournal`

#### Migration Action
- No code changes required
- New subcommands are opt-in
- Update version pin: `cargo install atomwrite --locked --version "^0.1.12"`

## Compatibility Notes
### v0.1.15 (Current)
- G117: `edit` multi-pair gains fuzzy parity, `pair_results`, `failed_pair_index`, and opt-in `--partial` -- envelope fields are additive
- G118: `write` resolves the target against the workspace before append/prepend, line-ending auto-detection, and `--expect-checksum` -- exits 82/126 now fire where a silent overwrite happened
- No new error codes; MSRV stays at Rust 1.88
### v0.1.12
- 6 new subcommands: `set`, `get`, `del`, `case`, `query`, `outline` (v14 Tier 3)
- G72 REAL syntax check via tree-sitter (`atomwrite write --syntax-check`, exit 88)
- G114 WAL sidecar for crash recovery (`.atomwrite.journal.<target>.atomwrite.journal.json`)
- G54 Advisory file lock via `flock` (exit 83 on timeout)
- G39 xattr preservation (macOS quarantine, Linux SELinux, POSIX capabilities)
- G41 content_inspector for UTF-16/BOM/binary detection
- G64 reflink CoW for backup/copy in APFS/btrfs/XFS
- G90 EXDEV fallback for Docker/NFS (exit 91 with `--strict-atomic`)
- G44 transform multi-rule YAML (`--rules` / `--inline-rules`)
- G68 `--max-filesize` and `--max-columns` for search
- G80 SIGPIPE handling (broken pipe → exit 0)
- G81 `--format raw` for read (Unix composability)
- 5 new error codes: 83, 88, 91, 92, 93
- All v0.1.11 behavior preserved
- 445 tests (was 320 baseline, +125 new)

### v0.1.3 (Historical)
- BREAKING: `edit` and `replace` no longer preserve the original file mtime by default
- New `--preserve-timestamps` flag on `edit` and `replace` restores the v0.1.2 behavior
- New `mtime_preserved` field in `EditOutput` and `ReplaceResult` NDJSON responses
- All v0.1.2 behavior preserved otherwise (macOS build fix, batch transaction fix, search event grouping, etc)

### v0.1.2 (Previous)
- All v0.1.1 behavior preserved
- 6 critical bug fixes including macOS build, batch transaction, replace counter
- 2 high-priority fixes (batch --file, backup --output-dir)
- 3 agent-first flags (--timeout, --grep, completions --install)
- 4 dependency updates (nix 0.31, signal-hook 0.4, windows-sys 0.61, rust-i18n 4)

### v0.1.1
- All v0.1.0 behavior preserved
- New subcommands and flags are additive only
- Exit codes unchanged from v0.1.0

### v0.1.0
- First public release
- All JSON schemas are defined in `docs/schemas/`
- Use `--json-schema` on any subcommand to introspect at runtime
- Exit codes follow sysexits conventions
- Pre-1.0 releases do not guarantee output stability
- Post-1.0 releases will maintain backward compatibility within major versions


## Rollback Plan
- Keep the previous version binary available before upgrading
- Use `cargo install atomwrite@0.x.y` to pin a specific version
- Verify rollback by running `atomwrite --version`
- Test the new version in a staging environment before production
- Monitor exit codes and NDJSON output for unexpected changes
- Revert to the previous version if agent tests fail
- Revert agent configuration to match the older CLI version


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


## v0.1.21 — What Is New

This release closes 3 GAP-2026 items (012, 013 Problem C, 014 v2) and adds 1 ADR (0038). The most visible change is that `--backup` operations now DELETE the backup after success by default; add `--keep-backup` to preserve it. The second visible change is that `edit` and `rollback` now accept `--backup`, closing the API parity hole from v0.1.20. The third change is `--allow-sequential-drift` on `edit` for sequential pipelines.

For the complete migration guide, see `docs/MIGRATION-v0.1.20-to-v0.1.21.md`.

### Backup Operations

- `write --backup` and `replace --backup` DELETE the backup after success by default
- `edit --backup` and `rollback --backup` are NEW in v0.1.21; the flag is honored on all 4 mutating subcommands
- `--keep-backup` is the OPT-IN flag to preserve the backup after success on `write`, `edit`, `replace`, `rollback`, `apply`, and `batch`
- `apply --keep-backup` and `batch --keep-backup` are NEW in v0.1.21 for parity
- Backups are ALWAYS preserved on the FAILURE path, regardless of `--keep-backup`

### Sequential Edit Pattern

- Chaining multiple `edit` calls on the same file without re-capturing `checksum_after` produces `STATE_DRIFT` (exit 82) on every call after the first
- Two valid patterns: re-capture checksum (Pattern A) or pass `--allow-sequential-drift` (Pattern B)
- Default behavior is unchanged: `STATE_DRIFT` still fires on checksum mismatch when the flag is absent

### Statistics

- 555+ tests passing (542 baseline v0.1.20 + 13 new)
- 1 new ADR (0038)
- 3 Windows cross-compile targets green


## v0.1.22 — What Is New

This release adds 2 new subcommands to address the legacy backup cleanup and the N-edits-in-1-invocation operator pattern. Both are additive: no flag, schema, or default behavior changed for existing commands.

For the complete migration guide, see `docs/MIGRATION-v0.1.21-to-v0.1.22.md`.

### Subcommands Added

- **`prune-backups [PATHS]...`** — manual cleanup of legacy `.bak.YYYYMMDD_HHMMSS` siblings
  - Flags: `--max-age <SECONDS>`, `--max-count <N>`, `--dry-run` (default `true` for safety)
  - NDJSON output with per-backup lines and summary
  - Exit 0 (scan complete), 1 (NO_MATCHES), 65 (precondition failed)
- **`edit-loop <PATH>`** — apply N `{old, new}` substitution pairs in 1 invocation via NDJSON on stdin
  - Flags: `--workspace`, `--expect-checksum`, `--partial`, `--fuzzy`, `--line-ending`, `--preserve-timestamps`, `--backup`, `--keep-backup`, `--retention`
  - NDJSON output with `pair_result` per input line plus a `summary` line

### Statistics

- 575+ tests passing in 56+ integration suites, 0 failures
- 2 new ADRs (0039, 0040)
- 2 new NDJSON schemas (`edit-loop-output.schema.json`, `prune-backups-output.schema.json`)
- 33 subcommands total (32 from v0.1.22 + `verify` from v0.1.25)


## v0.1.22 to v0.1.23

### Behavioral Changes (Action Required)

- backup-by-default: all 9 content-mutating commands (`write`, `edit`, `edit-loop`, `replace`, `transform`, `apply`, `set`, `del`, `case`) now create a backup BEFORE writing by default. The backup is auto-deleted after success (existing `keep_backup: false` default unchanged). If your pipeline depends on NO backup file being created (e.g., checks for `.bak.*` files), add `--no-backup` to the command or set `ATOMWRITE_BACKUP=0` globally
- shrink guard: `write --expect-checksum` now BLOCKS writes that shrink the file by more than 50%. If your pipeline legitimately truncates files while using `--expect-checksum`, add `--allow-shrink` to the command. Without `--expect-checksum`, behavior is unchanged

### Additive Changes (No Action Required)

- `allow_hyphen_values`: 15 CLI fields across 8 structs now accept values starting with `-`. Previously these caused exit 2 (ARGUMENT_PARSE_ERROR). No migration needed — this fixes a bug
- `edit --old-file <PATH> --new-file <PATH>`: new flags that read match/replacement content from files instead of CLI arguments. Bypasses kernel ARG_MAX (~131 KB). Cross-mixing `--old` with `--new-file` (or vice versa) returns exit 65. No migration needed — these are new opt-in flags

### Migration Checklist

- If using `write` without `--backup`: no action needed (backup auto-deletes after success)
- If checking for `.bak.*` file absence in CI: add `--no-backup` or set `ATOMWRITE_BACKUP=0`
- If using `write --expect-checksum` to legitimately truncate files: add `--allow-shrink`
- If passing values starting with `-` to `edit --old`, `search`, `replace`, `calc`, `regex`, `transform`, `read --grep`, `query --query`: the fix is automatic, no migration needed
