# Migration Guide: v0.1.20 to v0.1.21

- **Target audience**: operators and CI scripts that call `atomwrite` from shell, Make, or other automation layers
- **Scope**: 3 GAP-2026 closures (012, 013 Problem C, 014 v2) and 1 new ADR (0038)
- **Reading time**: 5 minutes
- **Action required**: review the Breaking Changes section; opt-in to `--keep-backup` if your script depends on backups surviving after success

## Breaking Changes

### Backups are deleted after successful `--backup` operations

- **Before (v0.1.20)**: `atomwrite --workspace . write --backup config.toml < new.toml` created a `.bak.<timestamp>` sibling that persisted indefinitely
- **After (v0.1.21)**: the same command creates the backup, performs the write, and DELETES the backup on success
- **Migration**: add `--keep-backup` to any script that depends on the backup surviving past the operation

```bash
# v0.1.20 — backup persists
echo "new" | atomwrite --workspace . write --backup config.toml

# v0.1.21 — same behavior requires the new flag
echo "new" | atomwrite --workspace . write --backup --keep-backup config.toml
```

### `edit` and `rollback` now accept `--backup`

- **Before (v0.1.20)**: `edit` and `rollback` hardcoded `backup: false`; passing `--backup` was silently ignored
- **After (v0.1.21)**: the flag is honored; success path is governed by `--keep-backup` (default: deleted)
- **Migration**: no change required for scripts that never passed `--backup`; scripts that did were silently no-op before and are now active

```bash
# v0.1.20 — --backup silently ignored
echo "new" | atomwrite --workspace . edit --backup src/main.rs --old foo --new bar

# v0.1.21 — creates backup, deletes after success (no --keep-backup)
echo "new" | atomwrite --workspace . edit --backup src/main.rs --old foo --new bar
# v0.1.21 — creates backup, preserves after success
echo "new" | atomwrite --workspace . edit --backup --keep-backup src/main.rs --old foo --new bar
```

## Novidades (Non-Breaking)

### `--allow-sequential-drift` for sequential `edit` pipelines

- Chain multiple `edit` calls without re-capturing `checksum_after` between invocations
- Two valid patterns: re-capture checksum (Pattern A) or pass the new flag (Pattern B)
- Default behavior is unchanged: `STATE_DRIFT` (exit 82) still fires on checksum mismatch when the flag is absent

```bash
# Pattern A — re-capture after each edit
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
echo "line 2" | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --append
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
printf 'line 1\nline 2\n' | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --append

# Pattern B — let each call's pre-state differ from the original
echo "line 2" | atomwrite --workspace . edit src/main.rs --append
printf 'line 1\nline 2\n' | atomwrite --workspace . edit --allow-sequential-drift src/main.rs --append
```

### `--keep-backup` parity on 6 subcommands

- `write`, `edit`, `replace`, `rollback`, `apply`, `batch` all accept `--keep-backup`
- `batch` propagates `--keep-backup` to every `write`/`edit`/`replace` op in the NDJSON manifest
- Per-op `keep_backup: true` in the manifest overrides the batch-level default

## Verification

- After upgrading, run the smoke test in a scratch directory to confirm the new behavior matches your expectations

```bash
# Setup — create a target file
echo "original" > /tmp/migration-test.txt

# v0.1.21 default — backup is deleted after success
echo "updated" | atomwrite --workspace /tmp write --backup migration-test.txt
fd '*.bak.*' /tmp | wc -l
# Expected: 0 (no backup survives)

# v0.1.21 with --keep-backup — backup persists
echo "updated" | atomwrite --workspace /tmp write --backup --keep-backup migration-test.txt
fd '*.bak.*' /tmp | wc -l
# Expected: 1 (one backup survives)

# Failure path — backup is always preserved
echo "updated" | atomwrite --workspace /tmp write --backup /nonexistent-dir/migration-test.txt
fd '*.bak.*' /tmp | wc -l
# Expected: 1 (the failed write preserved its backup)
```

- For CI scripts that audit backup counts, update the assertion to expect `0` after success and add `--keep-backup` where the audit requires persistence

## Field Reference

- `AtomicWriteOptions.keep_backup: bool` — internal field, default `false`
- `EditArgs.backup: bool`, `retention: u8`, `keep_backup: bool` — new in v0.1.21
- `RollbackArgs.backup: bool`, `keep_backup: bool` — new in v0.1.21
- `ReplaceArgs.keep_backup: bool`, `ApplyArgs.keep_backup: bool`, `BatchArgs.keep_backup: bool` — new in v0.1.21
- `EditArgs.allow_sequential_drift: bool` — new in v0.1.21, opt-in

## See Also

- `docs/decisions/0038-backup-cumprido-deleta.md` — ADR-0038 full rationale and alternatives
- `CHANGELOG.md` — v0.1.21 release notes with all GAP-2026 closures
- `SKILL.md` — Padrão Correto for sequential edits and backup patterns
- `docs/HOW_TO_USE.md` — usage examples for `--backup`, `--keep-backup`, and `--allow-sequential-drift`
