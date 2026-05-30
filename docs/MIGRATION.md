# atomwrite Migration Guide


[Leia em Portugues](MIGRATION.pt-BR.md)


## Current Version
- atomwrite is at v0.1.0
- This is the first public release
- This document establishes the migration template for future versions
- No migrations are required at this time


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


## Compatibility Notes
### v0.1.0 (Current)
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
