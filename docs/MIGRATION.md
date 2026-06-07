# atomwrite Migration Guide


[Leia em Portugues](MIGRATION.pt-BR.md)


## What's New in v0.1.12

This section summarizes the migration-relevant changes in v0.1.12. See the [v0.1.11 to v0.1.12 (Current)](#v0111-to-v0112-current) section below for the full migration guide with code examples.

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
- See the [v0.1.11 to v0.1.12 (Current)](#v0111-to-v0112-current) section for detailed migration steps.

### Test Coverage

- 445 tests passing (was 320 baseline, +125 new in v0.1.11+v0.1.12)
- 7 new ADRs in `docs/decisions/` (0019-0025)
- 7 new JSON schemas in `docs/schemas/`
- See [docs/decisions/README.md](README.md) for architectural decisions

## Current Version
- atomwrite is at v0.1.12
- This document covers migration from v0.1.0 through v0.1.12, with detailed sections for v0.1.11 to v0.1.12 and earlier major transitions
- See the sections below for additive changes and breaking changes in each version


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

### v0.1.3 (Current)

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
- NDJSON output schemas for all 28 subcommands
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


## v0.1.11 to v0.1.12 (Current)
### v0.1.12 (Current)

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
### v0.1.12 (Current)
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
