# atomwrite Migration Guide


[Leia em Portugues](MIGRATION.pt-BR.md)


## Current Version
- atomwrite is at v0.1.2
- This document covers migration from v0.1.1 to v0.1.2
- See the section below for additive changes in v0.1.2


## v0.1.1 to v0.1.2

### v0.1.2 (Current)

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
- NDJSON output schemas for all 22 subcommands
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

### Additive Changes
- `batch` supports 7 operations: write, replace, delete, edit, hash, move, copy (was write, replace, delete)
- `batch --transaction` flag for all-or-nothing execution with rollback
- `batch` move and copy accept `source`, `from`, `src` as field aliases
- `batch` write, delete, edit, hash accept `path` as alias for `target`
- `edit --fuzzy` flag with 7-strategy cascade for approximate text matching
- `edit --multi` flag for multiple NDJSON edits in one atomic write
- `scope` subcommand for grammatical scoping with AST-based actions
- `backup` subcommand for timestamped backups with BLAKE3 checksums
- `rollback` subcommand for restoring from backups
- `apply` subcommand for patch application with auto-format detection
- `--line-ending lf|crlf|cr|auto` flag on `write` and `edit`
- `--lang <LOCALE>` global flag for locale override (en, pt-BR)
- i18n support via `rust-i18n` with automatic OS locale detection
- 282 tests (was 5 in v0.1.0)

### JSON Output Changes
- `edit` output includes new optional fields: `fuzzy`, `strategy`, `strategies_tried`, `similarity`
- `read` timestamp changed from epoch seconds to ISO 8601 format
- New output types added for `scope`, `backup`, `rollback`, `apply`
- All existing fields remain unchanged

### Migration Action
- No action required
- Existing `jaq` filters and JSON parsing code continue to work
- New fields are additive and safe to ignore


## Compatibility Notes
### v0.1.1 (Current)
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
